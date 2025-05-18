use crate::schema::Document;
use crate::traits::TextSplitter;
use crate::Result;

use super::chunk::ChunkSize;

/// Text splitter that splits text based on character delimiters
pub struct CharacterTextSplitter {
    /// Size configuration for chunks
    chunk_size: ChunkSize,
    /// List of delimiter strings, ordered by priority
    separators: Vec<String>,
    /// Whether to keep separators in the chunks
    keep_separator: bool,
}

impl Default for CharacterTextSplitter {
    fn default() -> Self {
        Self {
            chunk_size: ChunkSize::default(),
            separators: vec![
                "\n\n".to_string(),
                "\n".to_string(),
                " ".to_string(),
                "".to_string(),
            ],
            keep_separator: false,
        }
    }
}

impl CharacterTextSplitter {
    /// Create a new text splitter with custom parameters
    pub fn new(chunk_size: ChunkSize, separators: Vec<String>, keep_separator: bool) -> Self {
        Self {
            chunk_size,
            separators,
            keep_separator,
        }
    }

    /// Create a new text splitter with default separators
    pub fn with_chunk_size(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size: ChunkSize::new(chunk_size, chunk_overlap),
            ..Default::default()
        }
    }

    /// Split text on the first available separator
    fn split_text_with_separators(&self, text: &str) -> Vec<String> {
        for separator in &self.separators {
            if separator.is_empty() {
                // If empty separator, split by character
                return text.chars().map(|c| c.to_string()).collect();
            }

            if text.contains(separator) {
                let parts: Vec<String> = if self.keep_separator {
                    text.split(separator)
                        .filter(|s| !s.is_empty())
                        .map(|part| format!("{}{}", part, separator))
                        .collect()
                } else {
                    text.split(separator)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect()
                };

                if !parts.is_empty() {
                    return parts;
                }
            }
        }

        // If no separators match, return the whole text as a single chunk
        vec![text.to_string()]
    }

    /// Merge chunks to respect chunk size requirements
    fn merge_splits(&self, splits: Vec<String>) -> Vec<String> {
        let mut docs: Vec<String> = Vec::new();
        let mut current_doc: Vec<String> = Vec::new();
        let mut current_length = 0;

        for split in splits {
            let split_length = split.len();

            if current_length + split_length > self.chunk_size.chunk_size && !current_doc.is_empty()
            {
                // Add the current document to the list of documents
                docs.push(current_doc.join(""));

                // Check if we need to overlap
                if self.chunk_size.chunk_overlap > 0 {
                    // Find the last few pieces that fit within the overlap
                    let mut overlap_length = 0;
                    let mut overlap_splits = Vec::new();

                    for piece in current_doc.iter().rev() {
                        if overlap_length + piece.len() > self.chunk_size.chunk_overlap {
                            break;
                        }

                        overlap_length += piece.len();
                        overlap_splits.insert(0, piece.clone());
                    }

                    // Start the next document with the overlapping pieces
                    current_doc = overlap_splits;
                    current_length = overlap_length;
                } else {
                    // No overlap, start with an empty document
                    current_doc = Vec::new();
                    current_length = 0;
                }
            }

            // Add the current split to the current document
            current_doc.push(split);
            current_length += split_length;
        }

        // Add the last document
        if !current_doc.is_empty() {
            docs.push(current_doc.join(""));
        }

        docs
    }
}

impl TextSplitter for CharacterTextSplitter {
    fn split_text(&self, text: &str) -> Result<Vec<String>> {
        let splits = self.split_text_with_separators(text);
        Ok(self.merge_splits(splits))
    }

    fn split_documents(&self, documents: Vec<Document>) -> Result<Vec<Document>> {
        let mut result = Vec::new();

        for doc in documents {
            let texts = self.split_text(&doc.page_content)?;

            for text in texts {
                let mut new_doc = Document::new(text);
                new_doc.metadata = doc.metadata.clone();
                result.push(new_doc);
            }
        }

        Ok(result)
    }
}
