use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::error::Error;
use crate::traits::{LanguageModel, Runnable};
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAICompletionRequest {
    model: String,
    prompt: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<u32>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIChoice {
    text: String,
    index: u32,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAICompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI LLM implementation
pub struct OpenAI {
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    n: Option<u32>,
    stop: Option<Vec<String>>,
    client: reqwest::Client,
}

impl OpenAI {
    /// Create a new OpenAI LLM instance
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            temperature: 0.7,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            n: None,
            stop: None,
            client: reqwest::Client::new(),
        }
    }

    /// Set the temperature parameter
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the max_tokens parameter
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Create a completion request from the prompt
    fn create_request(&self, prompt: &str) -> OpenAICompletionRequest {
        OpenAICompletionRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            max_tokens: self.max_tokens,
            temperature: Some(self.temperature),
            top_p: self.top_p,
            n: self.n,
            stop: self.stop.clone(),
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
        }
    }
}

#[async_trait]
impl Runnable<String, String> for OpenAI {
    async fn invoke(&self, input: String) -> Result<String> {
        let request = self.create_request(&input);

        let res = self
            .client
            .post("https://api.openai.com/v1/completions")
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

        let completion: OpenAICompletionResponse = res.json().await.map_err(Error::Request)?;

        if completion.choices.is_empty() {
            return Err(Error::LLM("No completions returned".to_string()));
        }

        Ok(completion.choices[0].text.clone())
    }
}

impl LanguageModel for OpenAI {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn parameters(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert("temperature".to_string(), json!(self.temperature));
        if let Some(max_tokens) = self.max_tokens {
            params.insert("max_tokens".to_string(), json!(max_tokens));
        }
        if let Some(top_p) = self.top_p {
            params.insert("top_p".to_string(), json!(top_p));
        }
        if let Some(frequency_penalty) = self.frequency_penalty {
            params.insert("frequency_penalty".to_string(), json!(frequency_penalty));
        }
        if let Some(presence_penalty) = self.presence_penalty {
            params.insert("presence_penalty".to_string(), json!(presence_penalty));
        }
        params
    }
}
