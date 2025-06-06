use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use uuid::Uuid;

use crate::error::Error;
use crate::state::{State, StateValue};
use crate::Result;

/// Metadata about a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Unique identifier for the checkpoint
    pub id: String,
    /// When the checkpoint was created
    pub created_at: u64,
    /// Name of the node that produced this state
    pub node_name: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A checkpoint storing state at a particular point in graph execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint<S: StateValue> {
    /// Metadata about the checkpoint
    pub metadata: CheckpointMetadata,
    /// The state at this checkpoint
    pub state: State<S>,
}

impl<S: StateValue> Checkpoint<S> {
    /// Create a new checkpoint
    pub fn new(node_name: impl Into<String>, state: State<S>) -> Self {
        Self {
            metadata: CheckpointMetadata {
                id: Uuid::new_v4().to_string(),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                node_name: node_name.into(),
                metadata: HashMap::new(),
            },
            state,
        }
    }

    /// Add metadata to the checkpoint
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Result<Self> {
        let json_value = serde_json::to_value(value).map_err(Error::Serialization)?;
        self.metadata.metadata.insert(key.into(), json_value);
        Ok(self)
    }
}

/// A store for checkpoints that can save and load state
pub trait CheckpointStore<S: StateValue>: Send + Sync {
    /// Save a checkpoint
    fn save(&self, checkpoint: Checkpoint<S>) -> Result<String>;

    /// Load a checkpoint by ID
    fn load(&self, id: &str) -> Result<Checkpoint<S>>;

    /// List all checkpoints
    fn list(&self) -> Result<Vec<CheckpointMetadata>>;

    /// Delete a checkpoint
    fn delete(&self, id: &str) -> Result<()>;

    /// Save multiple checkpoints in a batch
    fn save_batch(&self, checkpoints: Vec<Checkpoint<S>>) -> Result<Vec<String>> {
        let mut ids = Vec::with_capacity(checkpoints.len());
        for checkpoint in checkpoints {
            ids.push(self.save(checkpoint)?);
        }
        Ok(ids)
    }

    /// Load multiple checkpoints by their IDs
    fn load_batch(&self, ids: &[String]) -> Result<Vec<Checkpoint<S>>> {
        let mut checkpoints = Vec::with_capacity(ids.len());
        for id in ids {
            checkpoints.push(self.load(id)?);
        }
        Ok(checkpoints)
    }

    /// Delete multiple checkpoints by their IDs
    fn delete_batch(&self, ids: &[String]) -> Result<()> {
        for id in ids {
            self.delete(id)?;
        }
        Ok(())
    }
}

/// An in-memory checkpoint store
pub struct MemoryCheckpointStore<S: StateValue> {
    checkpoints: Arc<RwLock<HashMap<String, Checkpoint<S>>>>,
}

impl<S: StateValue> Default for MemoryCheckpointStore<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: StateValue> MemoryCheckpointStore<S> {
    /// Create a new in-memory checkpoint store
    pub fn new() -> Self {
        Self {
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<S: StateValue> CheckpointStore<S> for MemoryCheckpointStore<S> {
    fn save(&self, checkpoint: Checkpoint<S>) -> Result<String> {
        let id = checkpoint.metadata.id.clone();
        self.checkpoints
            .write()
            .unwrap()
            .insert(id.clone(), checkpoint);
        Ok(id)
    }

    fn load(&self, id: &str) -> Result<Checkpoint<S>> {
        self.checkpoints
            .read()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::Checkpoint(format!("Checkpoint not found: {}", id)))
    }

    fn list(&self) -> Result<Vec<CheckpointMetadata>> {
        Ok(self
            .checkpoints
            .read()
            .unwrap()
            .values()
            .map(|c| c.metadata.clone())
            .collect())
    }

    fn delete(&self, id: &str) -> Result<()> {
        self.checkpoints
            .write()
            .unwrap()
            .remove(id)
            .ok_or_else(|| Error::Checkpoint(format!("Checkpoint not found: {}", id)))?;
        Ok(())
    }
}

/// A file-based checkpoint store
pub struct FileCheckpointStore<S: StateValue> {
    directory: String,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: StateValue + Serialize + for<'de> Deserialize<'de>> FileCheckpointStore<S> {
    /// Create a new file-based checkpoint store
    pub fn new(directory: impl Into<String>) -> Self {
        Self {
            directory: directory.into(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the file path for a checkpoint ID
    fn get_file_path(&self, id: &str) -> String {
        format!("{}/{}.json", self.directory, id)
    }

    /// Get the metadata file path
    fn get_metadata_file_path(&self) -> String {
        format!("{}/metadata.json", self.directory)
    }

    /// Ensure the directory exists
    async fn ensure_directory(&self) -> Result<()> {
        let path = Path::new(&self.directory);
        if !path.exists() {
            fs::create_dir_all(path).await.map_err(Error::Io)?;
        }
        Ok(())
    }
}

impl<S: StateValue + Serialize + for<'de> Deserialize<'de>> CheckpointStore<S>
    for FileCheckpointStore<S>
{
    fn save(&self, checkpoint: Checkpoint<S>) -> Result<String> {
        let id = checkpoint.metadata.id.clone();
        let file_path = self.get_file_path(&id);

        // Convert to tokio block
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.ensure_directory().await?;

                // Serialize the checkpoint
                let json = serde_json::to_string(&checkpoint).map_err(Error::Serialization)?;

                // Write the checkpoint to a file
                fs::write(&file_path, json).await.map_err(Error::Io)?;

                // Update metadata file
                let metadata_path = self.get_metadata_file_path();
                let metadata = checkpoint.metadata.clone();

                // Read existing metadata if it exists
                let mut all_metadata = if Path::new(&metadata_path).exists() {
                    let content = fs::read_to_string(&metadata_path)
                        .await
                        .map_err(Error::Io)?;
                    serde_json::from_str::<HashMap<String, CheckpointMetadata>>(&content)
                        .map_err(Error::Serialization)?
                } else {
                    HashMap::new()
                };

                // Add new metadata and write back
                all_metadata.insert(id.clone(), metadata);
                let metadata_json =
                    serde_json::to_string(&all_metadata).map_err(Error::Serialization)?;

                fs::write(&metadata_path, metadata_json)
                    .await
                    .map_err(Error::Io)?;

                Ok(id)
            })
        })
    }

    fn load(&self, id: &str) -> Result<Checkpoint<S>> {
        let file_path = self.get_file_path(id);

        // Convert to tokio block
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                // Check if file exists
                if !Path::new(&file_path).exists() {
                    return Err(Error::Other(format!("Checkpoint not found: {}", id)));
                }

                // Read the file
                let content = fs::read_to_string(&file_path).await.map_err(Error::Io)?;

                // Deserialize the checkpoint
                let checkpoint: Checkpoint<S> =
                    serde_json::from_str(&content).map_err(Error::Serialization)?;

                Ok(checkpoint)
            })
        })
    }

    fn list(&self) -> Result<Vec<CheckpointMetadata>> {
        let metadata_path = self.get_metadata_file_path();

        // Convert to tokio block
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                // Check if metadata file exists
                if !Path::new(&metadata_path).exists() {
                    return Ok(Vec::new());
                }

                // Read the file
                let content = fs::read_to_string(&metadata_path)
                    .await
                    .map_err(Error::Io)?;

                // Deserialize the metadata
                let all_metadata: HashMap<String, CheckpointMetadata> =
                    serde_json::from_str(&content).map_err(Error::Serialization)?;

                Ok(all_metadata.values().cloned().collect())
            })
        })
    }

    fn delete(&self, id: &str) -> Result<()> {
        let file_path = self.get_file_path(id);
        let metadata_path = self.get_metadata_file_path();

        // Convert to tokio block
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                // Delete the checkpoint file if it exists
                if Path::new(&file_path).exists() {
                    fs::remove_file(&file_path).await.map_err(Error::Io)?;
                }

                // Update metadata file if it exists
                if Path::new(&metadata_path).exists() {
                    let content = fs::read_to_string(&metadata_path)
                        .await
                        .map_err(Error::Io)?;

                    let mut all_metadata: HashMap<String, CheckpointMetadata> =
                        serde_json::from_str(&content).map_err(Error::Serialization)?;

                    all_metadata.remove(id);

                    let metadata_json =
                        serde_json::to_string(&all_metadata).map_err(Error::Serialization)?;

                    fs::write(&metadata_path, metadata_json)
                        .await
                        .map_err(Error::Io)?;
                }

                Ok(())
            })
        })
    }
}
