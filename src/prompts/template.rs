use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::error::Error;
use crate::traits::Runnable;
use crate::Result;

/// Trait for prompt templates that can be formatted
#[async_trait]
pub trait PromptTemplate: Send + Sync {
    /// Format the prompt template with the given values
    fn format(&self, values: &HashMap<String, Value>) -> Result<String>;

    /// Get the input variables required by this template
    fn input_variables(&self) -> Vec<String>;
}

/// A simple prompt template using string placeholders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringPromptTemplate {
    /// The template string with {variable} placeholders
    template: String,

    /// The list of input variables
    input_variables: Vec<String>,
}

impl StringPromptTemplate {
    /// Create a new prompt template
    pub fn new(template: impl Into<String>, input_variables: Vec<String>) -> Self {
        Self {
            template: template.into(),
            input_variables,
        }
    }

    /// Create a new prompt template, automatically extracting variables
    pub fn from_template(template: impl Into<String>) -> Self {
        let template_str = template.into();
        let mut input_variables = Vec::new();

        // Simple regex to find variables like {variable}
        let var_regex = regex::Regex::new(r"\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();

        for cap in var_regex.captures_iter(&template_str) {
            if let Some(var_name) = cap.get(1) {
                let var_name = var_name.as_str().to_string();
                if !input_variables.contains(&var_name) {
                    input_variables.push(var_name);
                }
            }
        }

        Self {
            template: template_str,
            input_variables,
        }
    }
}

impl PromptTemplate for StringPromptTemplate {
    fn format(&self, values: &HashMap<String, Value>) -> Result<String> {
        let mut result = self.template.clone();

        for var in &self.input_variables {
            let value = values
                .get(var)
                .ok_or_else(|| Error::PromptTemplate(format!("Variable not found: {}", var)))?;

            let value_str = match value {
                Value::String(s) => s.clone(),
                _ => value.to_string(),
            };

            result = result.replace(&format!("{{{}}}", var), &value_str);
        }

        Ok(result)
    }

    fn input_variables(&self) -> Vec<String> {
        self.input_variables.clone()
    }
}

#[async_trait]
impl Runnable<HashMap<String, Value>, String> for StringPromptTemplate {
    async fn invoke(&self, input: HashMap<String, Value>) -> Result<String> {
        self.format(&input)
    }
}
