use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::traits::{LanguageModel, Runnable};
use crate::Result;

/// A mock LLM implementation for testing
pub struct MockLLM {
    responses: HashMap<String, String>,
    default_response: String,
}

impl Default for MockLLM {
    fn default() -> Self {
        Self {
            responses: HashMap::new(),
            default_response: "This is a mock response.".to_string(),
        }
    }
}

impl MockLLM {
    /// Create a new mock LLM
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a response mapping
    pub fn with_response(mut self, input: impl Into<String>, response: impl Into<String>) -> Self {
        self.responses.insert(input.into(), response.into());
        self
    }

    /// Set the default response
    pub fn with_default_response(mut self, response: impl Into<String>) -> Self {
        self.default_response = response.into();
        self
    }
}

#[async_trait]
impl Runnable<String, String> for MockLLM {
    async fn invoke(&self, input: String) -> Result<String> {
        Ok(self
            .responses
            .get(&input)
            .cloned()
            .unwrap_or_else(|| self.default_response.clone()))
    }
}

impl LanguageModel for MockLLM {
    fn model_name(&self) -> &str {
        "mock-llm"
    }

    fn parameters(&self) -> HashMap<String, Value> {
        HashMap::new()
    }
}
