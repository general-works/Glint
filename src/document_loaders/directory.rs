use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::error::Error;
use crate::schema::Document;
use crate::traits::DocumentLoader;
use crate::Result;

use super::text::TextLoader;

/// Loader for directories containing text files
pub struct DirectoryLoader {
    path: PathBuf,
    glob_pattern: Option<String>,
    recursive: bool,
}

impl DirectoryLoader {
    /// Create a new directory loader
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            glob_pattern: None,
            recursive: false,
        }
    }

    /// Set the glob pattern for file matching
    pub fn with_glob_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.glob_pattern = Some(pattern.into());
        self
    }

    /// Enable or disable recursive directory traversal
    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Check if a file matches the glob pattern
    fn matches_pattern(&self, path: &Path) -> bool {
        match &self.glob_pattern {
            Some(pattern) => {
                let glob = glob::Pattern::new(pattern).ok();
                match glob {
                    Some(g) => {
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                return g.matches(file_name_str);
                            }
                        }
                        false
                    }
                    None => true, // Invalid pattern matches everything
                }
            }
            None => true, // No pattern matches everything
        }
    }
}

#[async_trait]
impl DocumentLoader for DirectoryLoader {
    async fn load(&self) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        // Check if the path exists and is a directory
        let metadata = fs::metadata(&self.path).await.map_err(|e| {
            Error::DocumentLoader(format!("Failed to read directory metadata: {}", e))
        })?;

        if !metadata.is_dir() {
            return Err(Error::DocumentLoader(format!(
                "Path is not a directory: {}",
                self.path.display()
            )));
        }

        // Read directory entries
        let mut read_dir = fs::read_dir(&self.path)
            .await
            .map_err(|e| Error::DocumentLoader(format!("Failed to read directory: {}", e)))?;

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let metadata = match fs::metadata(&path).await {
                Ok(meta) => meta,
                Err(_) => continue, // Skip entries we can't read metadata for
            };

            if metadata.is_file() && self.matches_pattern(&path) {
                // Load the file as a document
                let loader = TextLoader::new(&path);
                match loader.load().await {
                    Ok(mut docs) => documents.append(&mut docs),
                    Err(_) => continue, // Skip files that can't be loaded
                }
            } else if metadata.is_dir() && self.recursive {
                // Recursively process subdirectories
                let subdir_loader = DirectoryLoader::new(&path).with_recursive(true);

                let subdir_loader = if let Some(pattern) = &self.glob_pattern {
                    subdir_loader.with_glob_pattern(pattern)
                } else {
                    subdir_loader
                };

                match subdir_loader.load().await {
                    Ok(mut docs) => documents.append(&mut docs),
                    Err(_) => continue, // Skip directories that can't be loaded
                }
            }
        }

        Ok(documents)
    }
}
