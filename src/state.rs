use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use crate::error::Error;
use crate::Result;

/// Trait for types that can be used as state in a graph.
///
/// This trait is automatically implemented for common types like:
/// - Primitive types (i32, f64, bool, etc.)
/// - Collections (Vec<T>, HashMap<K,V>, etc.)
/// - Any type that implements Clone + Debug + Send + Sync
pub trait StateValue: Clone + Debug + Send + Sync + 'static {}

// Implement StateValue for common types
impl StateValue for String {}
impl StateValue for i32 {}
impl StateValue for i64 {}
impl StateValue for f32 {}
impl StateValue for f64 {}
impl StateValue for bool {}
impl<T: StateValue> StateValue for Vec<T> {}
impl<K, V> StateValue for HashMap<K, V>
where
    K: Eq + Hash + Clone + Debug + Send + Sync + 'static,
    V: StateValue,
{
}

/// Represents the state that flows through the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State<T: StateValue> {
    /// The current data in the state
    pub data: T,

    /// Metadata associated with the state
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<T: StateValue> State<T> {
    /// Create a new state with the given data
    pub fn new(data: T) -> Self {
        Self {
            data,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the state
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get a value from metadata
    pub fn get_metadata<V: DeserializeOwned>(&self, key: &str) -> Result<Option<V>> {
        match self.metadata.get(key) {
            Some(value) => {
                let deserialized = serde_json::from_value(value.clone())
                    .map_err(|e| Error::State(format!("Failed to deserialize metadata: {}", e)))?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    /// Remove metadata
    pub fn remove_metadata(&mut self, key: &str) -> Option<serde_json::Value> {
        self.metadata.remove(key)
    }

    /// Check if metadata contains a key
    pub fn has_metadata(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }
}

/// Trait for operations that update state
pub trait StateUpdate<S: StateValue>: Send + Sync {
    /// Apply the update to the state
    fn apply(&self, state: State<S>) -> Result<State<S>>;
}

/// A simple state type that holds a map of string keys to values
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapState {
    pub values: HashMap<String, serde_json::Value>,
}

impl StateValue for MapState {}

impl MapState {
    /// Create a new empty map state
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Get a value from the state
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.values.get(key) {
            Some(value) => {
                let deserialized = serde_json::from_value(value.clone())
                    .map_err(|e| Error::State(format!("Failed to deserialize value: {}", e)))?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    /// Set a value in the state
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| Error::State(format!("Failed to serialize value: {}", e)))?;
        self.values.insert(key.into(), json_value);
        Ok(())
    }

    /// Remove a value from the state
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.values.remove(key)
    }

    /// Check if the state contains a key
    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get all keys in the state
    pub fn keys(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    /// Get all values in the state
    pub fn values(&self) -> Vec<&serde_json::Value> {
        self.values.values().collect()
    }

    /// Get the number of key-value pairs
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the state is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
