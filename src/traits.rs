use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::schema::{Document, Message};
use crate::Result;

/// Trait for any component that can be invoked with an input and produces an output asynchronously.
///
/// This is the core abstraction for all runnable components in Glint, including LLMs, embeddings, etc.
#[async_trait]
pub trait Runnable<Input: Send + 'static, Output: 'static> {
    /// Run the component on the given input and return the output.
    async fn invoke(&self, input: Input) -> Result<Output>;

    /// Stream the output of the component (default: wraps invoke in a stream).
    async fn stream(
        &self,
        input: Input,
    ) -> Result<impl futures::Stream<Item = Result<Output>> + Send>
    where
        Self: Sized,
        Output: Send,
    {
        let output = self.invoke(input).await?;
        Ok(futures::stream::once(async move { Ok(output) }))
    }

    /// Run the component on a batch of inputs.
    async fn batch(&self, inputs: Vec<Input>) -> Result<Vec<Result<Output>>>
    where
        Self: Sized,
        Input: Sync,
        Output: Send,
    {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.invoke(input).await);
        }
        Ok(results)
    }
}

/// Trait for language models that can generate text from a string prompt.
#[async_trait]
pub trait LanguageModel: Runnable<String, String> {
    /// Get the model name/identifier.
    fn model_name(&self) -> &str;
    /// Get model parameters as a key-value map.
    fn parameters(&self) -> HashMap<String, Value>;
}

/// Trait for chat models that can handle message exchanges (multi-turn).
#[async_trait]
pub trait ChatModel: Runnable<Vec<Message>, Message> {
    /// Get the model name/identifier.
    fn model_name(&self) -> &str;
    /// Get model parameters as a key-value map.
    fn parameters(&self) -> HashMap<String, Value>;
}

/// Trait for document loaders (e.g. file, web, etc.).
#[async_trait]
pub trait DocumentLoader {
    /// Load documents from a source.
    async fn load(&self) -> Result<Vec<Document>>;
}

/// Trait for text splitters (chunking documents/text).
pub trait TextSplitter {
    /// Split a document into chunks.
    fn split_documents(&self, documents: Vec<Document>) -> Result<Vec<Document>>;
    /// Split text into chunks.
    fn split_text(&self, text: &str) -> Result<Vec<String>>;
}

/// Trait for embedding models (text to vector).
#[async_trait]
pub trait EmbeddingModel: Runnable<String, Vec<f32>> + Send + Sync {
    /// Get the model name/identifier.
    fn model_name(&self) -> &str;
    /// Get the dimension of the embeddings produced by this model.
    fn embedding_dimension(&self) -> usize;
    /// Embed multiple texts in a single batch call (default: loop invoke).
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.invoke(text).await?);
        }
        Ok(embeddings)
    }
    /// Embed documents (using their page_content).
    async fn embed_documents(&self, documents: Vec<Document>) -> Result<Vec<Vec<f32>>> {
        let texts: Vec<String> = documents.into_iter().map(|doc| doc.page_content).collect();
        self.embed_batch(texts).await
    }
}

/// Trait for vector stores (for similarity search, etc.).
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Add documents to the vector store.
    async fn add_documents(&mut self, documents: Vec<Document>) -> Result<()>;
    /// Search for similar documents using a query string.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Document, f32)>>;
    /// Search using a vector directly.
    async fn search_by_vector(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Document, f32)>>;
    /// Delete documents by ID.
    async fn delete(&mut self, ids: &[String]) -> Result<()>;
}
