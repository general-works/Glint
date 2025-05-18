use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Document represents a piece of text and associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// The document's content
    pub page_content: String,

    /// Metadata associated with the document
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Document {
    /// Create a new document with the given content
    pub fn new(page_content: impl Into<String>) -> Self {
        Self {
            page_content: page_content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new document with content and metadata
    pub fn with_metadata(
        page_content: impl Into<String>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            page_content: page_content.into(),
            metadata,
        }
    }
}

/// Message role types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Function,
}

/// A chat message, containing content and a role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The message role
    pub role: MessageRole,

    /// The message content
    pub content: String,

    /// Optional ID for the message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,

    /// The priority of the message (higher numbers = higher priority)
    #[serde(default)]
    pub priority: u32,
}

impl Message {
    /// Create a new message
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            metadata: HashMap::new(),
            priority: 0,
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    /// Create a function message
    pub fn function(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Function, content)
    }

    /// Add metadata to the message
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set the priority of the message
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// Generation is an individual generated output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    /// The generated text
    pub text: String,

    /// Model-specific generation info
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, serde_json::Value>>,
}

/// LLMResult represents the result of an LLM call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResult {
    /// The generated texts for each prompt
    pub generations: Vec<Vec<Generation>>,

    /// Information about the LLM call
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_output: Option<HashMap<String, serde_json::Value>>,
}
