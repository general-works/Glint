use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::error::Error;
use crate::schema::{Message, MessageRole};
use crate::traits::{ChatModel, Runnable};
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatOpenAIMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatOpenAIRequest {
    model: String,
    messages: Vec<ChatOpenAIMessage>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<u32>,
    stop: Option<Vec<String>>,
    max_tokens: Option<u32>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatOpenAIChoice {
    index: u32,
    message: ChatOpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatOpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<ChatOpenAIChoice>,
    usage: Option<ChatOpenAIUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatOpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI chat model implementation
pub struct ChatOpenAI {
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

impl ChatOpenAI {
    /// Create a new ChatOpenAI instance
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

    /// Convert messages to OpenAI format
    fn convert_messages(&self, messages: &[Message]) -> Vec<ChatOpenAIMessage> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Function => "function",
                }
                .to_string();

                ChatOpenAIMessage {
                    role,
                    content: msg.content.clone(),
                    name: None,
                }
            })
            .collect()
    }
}

#[async_trait]
impl Runnable<Vec<Message>, Message> for ChatOpenAI {
    async fn invoke(&self, input: Vec<Message>) -> Result<Message> {
        if input.is_empty() {
            return Err(Error::LLM("No messages provided".to_string()));
        }

        let openai_messages = self.convert_messages(&input);

        let request = ChatOpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
            temperature: Some(self.temperature),
            top_p: self.top_p,
            n: self.n,
            stop: self.stop.clone(),
            max_tokens: self.max_tokens,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
        };

        let res = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
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

        let response: ChatOpenAIResponse = res.json().await.map_err(Error::Request)?;

        if response.choices.is_empty() {
            return Err(Error::LLM("No chat completions returned".to_string()));
        }

        let choice = &response.choices[0];
        let role = match choice.message.role.as_str() {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "function" => MessageRole::Function,
            _ => MessageRole::Assistant, // Default to assistant for unknown roles
        };

        Ok(Message::new(role, choice.message.content.clone()))
    }
}

impl ChatModel for ChatOpenAI {
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
