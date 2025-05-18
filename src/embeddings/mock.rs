use async_trait::async_trait;
use std::collections::HashMap;

use crate::traits::{EmbeddingModel, Runnable};
use crate::Result;

/// A mock embeddings model for testing
pub struct MockEmbeddings {
    dimension: usize,
    embeddings: HashMap<String, Vec<f32>>,
}

impl Default for MockEmbeddings {
    fn default() -> Self {
        Self {
            dimension: 4,
            embeddings: HashMap::new(),
        }
    }
}

impl MockEmbeddings {
    /// Create a new mock embeddings model
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            embeddings: HashMap::new(),
        }
    }

    /// Add a pre-defined embedding for a text
    pub fn with_embedding(mut self, text: impl Into<String>, embedding: Vec<f32>) -> Self {
        let text = text.into();
        if embedding.len() != self.dimension {
            panic!(
                "Embedding dimension {} doesn't match expected dimension {}",
                embedding.len(),
                self.dimension
            );
        }
        self.embeddings.insert(text, embedding);
        self
    }

    /// Generate a deterministic embedding from text
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut result = vec![0.0; self.dimension];

        // Simple deterministic algorithm: use character codes
        for (i, c) in text.chars().enumerate() {
            let pos = i % self.dimension;
            // Add a scaled value based on character code
            result[pos] += (c as u32 % 100) as f32 / 100.0;
        }

        // Normalize the vector
        let magnitude: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut result {
                *val /= magnitude;
            }
        }

        result
    }
}

#[async_trait]
impl Runnable<String, Vec<f32>> for MockEmbeddings {
    async fn invoke(&self, input: String) -> Result<Vec<f32>> {
        // Return pre-defined embedding if it exists
        if let Some(embedding) = self.embeddings.get(&input) {
            return Ok(embedding.clone());
        }

        // Otherwise generate a deterministic embedding
        Ok(self.generate_embedding(&input))
    }
}

impl EmbeddingModel for MockEmbeddings {
    fn model_name(&self) -> &str {
        "mock-embeddings"
    }

    fn embedding_dimension(&self) -> usize {
        self.dimension
    }
}
