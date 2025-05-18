use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::error::Error;
use crate::schema::Document;
use crate::traits::{EmbeddingModel, VectorStore};
use crate::Result;

use super::similarity::{cosine_similarity, distance_to_similarity, euclidean_distance};

/// Similarity metrics for comparing vectors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimilarityMetric {
    /// Cosine similarity (higher is more similar)
    Cosine,
    /// Euclidean distance (lower is more similar)
    Euclidean,
    /// Dot product (higher is more similar)
    DotProduct,
}

/// Internal document storage with associated embeddings
#[derive(Debug, Clone)]
struct DocumentWithEmbedding {
    /// The document
    document: Document,
    /// The document embedding
    embedding: Vec<f32>,
}

/// An in-memory vector store
pub struct MemoryVectorStore {
    /// Documents with their embeddings
    documents: Arc<RwLock<Vec<DocumentWithEmbedding>>>,
    /// Embedding model to use for queries
    embedding_model: Arc<dyn EmbeddingModel>,
    /// Similarity metric to use
    similarity_metric: SimilarityMetric,
}

impl MemoryVectorStore {
    /// Create a new in-memory vector store
    pub fn new(embedding_model: impl EmbeddingModel + 'static) -> Self {
        Self {
            documents: Arc::new(RwLock::new(Vec::new())),
            embedding_model: Arc::new(embedding_model),
            similarity_metric: SimilarityMetric::Cosine,
        }
    }

    /// Set the similarity metric
    pub fn with_similarity_metric(mut self, metric: SimilarityMetric) -> Self {
        self.similarity_metric = metric;
        self
    }

    /// Calculate similarity between two vectors based on selected metric
    fn calculate_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.similarity_metric {
            SimilarityMetric::Cosine => cosine_similarity(a, b),
            SimilarityMetric::Euclidean => {
                let distance = euclidean_distance(a, b);
                distance_to_similarity(distance)
            }
            SimilarityMetric::DotProduct => super::similarity::dot_product(a, b),
        }
    }
}

#[async_trait]
impl VectorStore for MemoryVectorStore {
    async fn add_documents(&mut self, documents: Vec<Document>) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        // Generate embeddings for the documents
        let embeddings = self
            .embedding_model
            .embed_documents(documents.clone())
            .await?;

        // Add documents with embeddings to storage
        let mut docs_with_embeddings = Vec::with_capacity(documents.len());
        for (doc, embedding) in documents.into_iter().zip(embeddings.into_iter()) {
            docs_with_embeddings.push(DocumentWithEmbedding {
                document: doc,
                embedding,
            });
        }

        // Add to storage
        let mut storage = self.documents.write().map_err(|_| {
            Error::Other("Failed to acquire write lock on vector store".to_string())
        })?;

        storage.extend(docs_with_embeddings);
        Ok(())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Document, f32)>> {
        // Get query embedding
        let query_embedding = self.embedding_model.invoke(query.to_string()).await?;

        // Search by vector
        self.search_by_vector(&query_embedding, limit).await
    }

    async fn search_by_vector(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Document, f32)>> {
        let storage = self
            .documents
            .read()
            .map_err(|_| Error::Other("Failed to acquire read lock on vector store".to_string()))?;

        if storage.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate similarities and create (doc, similarity) pairs
        let mut results: Vec<(Document, f32)> = storage
            .iter()
            .map(|doc_with_embedding| {
                let similarity =
                    self.calculate_similarity(embedding, &doc_with_embedding.embedding);
                (doc_with_embedding.document.clone(), similarity)
            })
            .collect();

        // Sort by similarity (highest first)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top K results
        Ok(results.into_iter().take(limit).collect())
    }

    async fn delete(&mut self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut storage = self.documents.write().map_err(|_| {
            Error::Other("Failed to acquire write lock on vector store".to_string())
        })?;

        // Create a set of IDs to delete
        let id_set: std::collections::HashSet<&String> = ids.iter().collect();

        // Filter out documents with matching IDs
        storage.retain(|doc_with_embedding| {
            if let Some(id) = doc_with_embedding.document.metadata.get("id") {
                if let Some(id_str) = id.as_str() {
                    return !id_set.contains(&id_str.to_string());
                }
            }
            true
        });

        Ok(())
    }
}
