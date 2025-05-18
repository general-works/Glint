use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::error::Error;
use crate::schema::Document;
use crate::traits::DocumentLoader;
use crate::Result;

/// Loader for text files
pub struct TextLoader {
    file_path: PathBuf,
    encoding: String,
}

impl TextLoader {
    /// Create a new text loader
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            encoding: "utf-8".to_string(),
        }
    }

    /// Set the encoding for the text file
    pub fn with_encoding(mut self, encoding: impl Into<String>) -> Self {
        self.encoding = encoding.into();
        self
    }
}

#[async_trait]
impl DocumentLoader for TextLoader {
    async fn load(&self) -> Result<Vec<Document>> {
        let file_path = self.file_path.clone();
        let metadata = fs::metadata(&file_path).await.map_err(|e| {
            Error::DocumentLoader(format!("Failed to read metadata for file: {}", e))
        })?;

        if !metadata.is_file() {
            return Err(Error::DocumentLoader(format!(
                "Path is not a file: {}",
                file_path.display()
            )));
        }

        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| Error::DocumentLoader(format!("Failed to read file: {}", e)))?;

        let mut doc_metadata = std::collections::HashMap::new();
        doc_metadata.insert(
            "source".to_string(),
            serde_json::Value::String(file_path.to_string_lossy().to_string()),
        );

        Ok(vec![Document::with_metadata(content, doc_metadata)])
    }
}
