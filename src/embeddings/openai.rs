use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::traits::{EmbeddingModel, Runnable};
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbeddingResponse {
    object: String,
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: OpenAIUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

/// OpenAI embeddings model implementation
pub struct OpenAIEmbeddings {
    api_key: String,
    model: String,
    client: reqwest::Client,
    dimension: usize,
}

impl OpenAIEmbeddings {
    /// Create a new OpenAI embeddings model instance
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let model_name = model.into();

        // Set embedding dimension based on model
        let dimension = match model_name.as_str() {
            "text-embedding-ada-002" => 1536,
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            _ => 1536, // Default to 1536 for unknown models
        };

        Self {
            api_key: api_key.into(),
            model: model_name,
            client: reqwest::Client::new(),
            dimension,
        }
    }
}

#[async_trait]
impl Runnable<String, Vec<f32>> for OpenAIEmbeddings {
    async fn invoke(&self, input: String) -> Result<Vec<f32>> {
        let request = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input,
        };

        let res = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .json(&request)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(Error::Request)?;

        // Store status code before consuming the response
        let status = res.status();

        if !status.is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(Error::LLM(format!(
                "OpenAI API error: {} - {}",
                status, error_text
            )));
        }

        let response: OpenAIEmbeddingResponse = res.json().await.map_err(Error::Request)?;

        if response.data.is_empty() {
            return Err(Error::LLM("No embeddings returned".to_string()));
        }

        Ok(response.data[0].embedding.clone())
    }
}

impl EmbeddingModel for OpenAIEmbeddings {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn embedding_dimension(&self) -> usize {
        self.dimension
    }
}
