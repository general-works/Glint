use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextParams {
    pub operation: String, // "length", "toUpper", "toLower", "trim", "replace", "split", "substr", "concat", "contains", "startsWith", "endsWith", "regex"
    // Input field settings
    pub input_field: Option<String>,
    #[serde(default = "default_output_field")]
    pub output_field: String,
    // For replace operation
    pub search_value: Option<String>,
    pub replace_value: Option<String>,
    #[serde(default = "default_false")]
    pub case_sensitive: bool,
    #[serde(default = "default_false")]
    pub regex_mode: bool,
    // For split operation
    pub delimiter: Option<String>,
    pub max_split: Option<usize>,
    // For substr operation
    pub start_index: Option<usize>,
    pub length: Option<usize>,
    // For concat operation
    pub concat_values: Option<Vec<String>>,
    pub separator: Option<String>,
    // For contains/startsWith/endsWith operations
    pub search_text: Option<String>,
    // For regex operation
    pub regex_pattern: Option<String>,
    pub regex_flags: Option<String>, // "i", "g", "m", etc.
    // General options
    #[serde(default = "default_true")]
    pub include_input_fields: bool,
    #[serde(default = "default_false")]
    pub handle_arrays: bool,
}

fn default_false() -> bool { false }
fn default_true() -> bool { true }
fn default_output_field() -> String { "result".to_string() }

impl Default for TextParams {
    fn default() -> Self {
        Self {
            operation: "length".to_string(),
            input_field: None,
            output_field: "result".to_string(),
            search_value: None,
            replace_value: None,
            case_sensitive: true,
            regex_mode: false,
            delimiter: None,
            max_split: None,
            start_index: None,
            length: None,
            concat_values: None,
            separator: None,
            search_text: None,
            regex_pattern: None,
            regex_flags: None,
            include_input_fields: true,
            handle_arrays: false,
        }
    }
}

impl INode for TextNode {
    fn new(
        id: NodeId,
        name: String,
        _node_type: String,
        description: Option<String>,
        parameters: Option<Value>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            parameters,
        }
    }

    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> Option<String> {
        self.description.clone()
    }

    fn parameter(&self) -> Option<Value> {
        self.parameters.clone()
    }

    fn execute(&self, input: &NodeOutput) -> Result<Value, Error> {
        let params = if let Some(params_value) = &self.parameters {
            serde_json::from_value::<TextParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            TextParams::default()
        };

        // Process input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        let mut results = Vec::new();

        // If no input data, return empty array
        if input_data.is_empty() {
            return Ok(json!([]));
        }

        // Flatten all items
        let mut all_items = Vec::new();
        for item in input_data {
            if let Value::Array(items) = item {
                all_items.extend(items);
            } else {
                all_items.push(item);
            }
        }

        for item in all_items {
            let result_item = self.process_text_operation(&params, &item)?;
            results.push(result_item);
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<TextParams>(params_value.clone()) {
                let valid_operations = [
                    "length", "toUpper", "toLower", "trim", "replace", "split", 
                    "substr", "concat", "contains", "startsWith", "endsWith", "regex"
                ];
                return valid_operations.contains(&params.operation.as_str());
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl TextNode {
    fn process_text_operation(&self, params: &TextParams, item: &Value) -> Result<Value, Error> {
        let mut result_item = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Get input text
        let input_text = self.get_input_text(params, item)?;
        
        // Apply text operation
        let result_value = match params.operation.as_str() {
            "length" => self.text_length(&input_text),
            "toUpper" => self.text_to_upper(&input_text),
            "toLower" => self.text_to_lower(&input_text),
            "trim" => self.text_trim(&input_text),
            "replace" => self.text_replace(params, &input_text),
            "split" => self.text_split(params, &input_text),
            "substr" => self.text_substr(params, &input_text),
            "concat" => self.text_concat(params, &input_text),
            "contains" => self.text_contains(params, &input_text),
            "startsWith" => self.text_starts_with(params, &input_text),
            "endsWith" => self.text_ends_with(params, &input_text),
            "regex" => self.text_regex(params, &input_text),
            _ => return Err(Error::Validation(format!("Unsupported text operation: {}", params.operation)))
        }?;

        // Add result to output
        if let Value::Object(map) = &mut result_item {
            map.insert(params.output_field.clone(), result_value);
        } else {
            result_item = json!({
                params.output_field.clone(): result_value
            });
        }

        Ok(result_item)
    }

    fn get_input_text(&self, params: &TextParams, item: &Value) -> Result<String, Error> {
        if let Some(input_field) = &params.input_field {
            // Get text from specified field
            let field_value = self.get_field_value(input_field, item)?;
            self.value_to_string(&field_value)
        } else {
            // Try to get text from common fields or convert the whole item
            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                Ok(text.to_string())
            } else if let Some(content) = item.get("content").and_then(|v| v.as_str()) {
                Ok(content.to_string())
            } else if let Some(message) = item.get("message").and_then(|v| v.as_str()) {
                Ok(message.to_string())
            } else {
                self.value_to_string(item)
            }
        }
    }

    fn get_field_value(&self, field_path: &str, data: &Value) -> Result<Value, Error> {
        let parts: Vec<&str> = field_path.split('.').collect();
        let mut current = data;

        for part in parts {
            if let Some(next) = current.get(part) {
                current = next;
            } else {
                return Ok(Value::Null);
            }
        }

        Ok(current.clone())
    }

    fn value_to_string(&self, value: &Value) -> Result<String, Error> {
        match value {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Ok(String::new()),
            _ => Ok(value.to_string())
        }
    }

    fn text_length(&self, text: &str) -> Result<Value, Error> {
        Ok(json!(text.len()))
    }

    fn text_to_upper(&self, text: &str) -> Result<Value, Error> {
        Ok(json!(text.to_uppercase()))
    }

    fn text_to_lower(&self, text: &str) -> Result<Value, Error> {
        Ok(json!(text.to_lowercase()))
    }

    fn text_trim(&self, text: &str) -> Result<Value, Error> {
        Ok(json!(text.trim()))
    }

    fn text_replace(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let search = params.search_value.as_ref()
            .ok_or_else(|| Error::Validation("search_value is required for replace operation".to_string()))?;
        let default_replace = String::new();
        let replace = params.replace_value.as_ref().unwrap_or(&default_replace);

        if params.regex_mode {
            let regex = Regex::new(search)
                .map_err(|e| Error::Validation(format!("Invalid regex pattern: {}", e)))?;
            Ok(json!(regex.replace_all(text, replace).to_string()))
        } else if params.case_sensitive {
            Ok(json!(text.replace(search, replace)))
        } else {
            // Case insensitive replace
            let lower_text = text.to_lowercase();
            let lower_search = search.to_lowercase();
            let mut result = text.to_string();
            let mut start = 0;
            
            while let Some(pos) = lower_text[start..].find(&lower_search) {
                let actual_pos = start + pos;
                result.replace_range(actual_pos..actual_pos + search.len(), replace);
                start = actual_pos + replace.len();
            }
            
            Ok(json!(result))
        }
    }

    fn text_split(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let default_delimiter = ",".to_string();
        let delimiter = params.delimiter.as_ref().unwrap_or(&default_delimiter);
        
        let parts: Vec<&str> = if let Some(max_split) = params.max_split {
            text.splitn(max_split + 1, delimiter).collect()
        } else {
            text.split(delimiter).collect()
        };

        let string_parts: Vec<String> = parts.into_iter().map(|s| s.to_string()).collect();
        Ok(json!(string_parts))
    }

    fn text_substr(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let start = params.start_index.unwrap_or(0);
        let chars: Vec<char> = text.chars().collect();
        
        if start >= chars.len() {
            return Ok(json!(""));
        }

        let end = if let Some(length) = params.length {
            std::cmp::min(start + length, chars.len())
        } else {
            chars.len()
        };

        let result: String = chars[start..end].iter().collect();
        Ok(json!(result))
    }

    fn text_concat(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let default_separator = "".to_string();
        let separator = params.separator.as_ref().unwrap_or(&default_separator);
        let mut parts = vec![text.to_string()];
        
        if let Some(concat_values) = &params.concat_values {
            parts.extend(concat_values.clone());
        }
        
        Ok(json!(parts.join(separator)))
    }

    fn text_contains(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let search = params.search_text.as_ref()
            .ok_or_else(|| Error::Validation("search_text is required for contains operation".to_string()))?;
        
        let result = if params.case_sensitive {
            text.contains(search)
        } else {
            text.to_lowercase().contains(&search.to_lowercase())
        };
        
        Ok(json!(result))
    }

    fn text_starts_with(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let search = params.search_text.as_ref()
            .ok_or_else(|| Error::Validation("search_text is required for startsWith operation".to_string()))?;
        
        let result = if params.case_sensitive {
            text.starts_with(search)
        } else {
            text.to_lowercase().starts_with(&search.to_lowercase())
        };
        
        Ok(json!(result))
    }

    fn text_ends_with(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let search = params.search_text.as_ref()
            .ok_or_else(|| Error::Validation("search_text is required for endsWith operation".to_string()))?;
        
        let result = if params.case_sensitive {
            text.ends_with(search)
        } else {
            text.to_lowercase().ends_with(&search.to_lowercase())
        };
        
        Ok(json!(result))
    }

    fn text_regex(&self, params: &TextParams, text: &str) -> Result<Value, Error> {
        let pattern = params.regex_pattern.as_ref()
            .ok_or_else(|| Error::Validation("regex_pattern is required for regex operation".to_string()))?;
        
        let regex = Regex::new(pattern)
            .map_err(|e| Error::Validation(format!("Invalid regex pattern: {}", e)))?;
        
        // Find all matches
        let matches: Vec<String> = regex.find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect();
        
        Ok(json!(matches))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_text_node_creation() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "operation": "length"
            })),
        );

        assert_eq!(node.name(), "Test Text");
        assert!(node.validate());
    }

    #[test]
    fn test_text_validation() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "invalid_operation"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_text_length() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "length",
                "output_field": "text_length"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"text": "Hello World"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["text_length"], 11);
            assert_eq!(results[0]["text"], "Hello World");
        }
    }

    #[test]
    fn test_text_to_upper() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "toUpper",
                "input_field": "message",
                "output_field": "upper_text"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"message": "hello world"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["upper_text"], "HELLO WORLD");
        }
    }

    #[test]
    fn test_text_replace() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "replace",
                "search_value": "world",
                "replace_value": "Rust",
                "case_sensitive": false,
                "output_field": "replaced_text"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"text": "Hello World!"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["replaced_text"], "Hello Rust!");
        }
    }

    #[test]
    fn test_text_split() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "split",
                "delimiter": ",",
                "output_field": "parts"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"text": "apple,banana,cherry"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["parts"], json!(["apple", "banana", "cherry"]));
        }
    }

    #[test]
    fn test_text_substr() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "substr",
                "start_index": 6,
                "length": 5,
                "output_field": "substring"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"text": "Hello World"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["substring"], "World");
        }
    }

    #[test]
    fn test_text_contains() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "contains",
                "search_text": "world",
                "case_sensitive": false,
                "output_field": "has_world"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"text": "Hello World!"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["has_world"], true);
        }
    }

    #[test]
    fn test_text_regex() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "regex",
                "regex_pattern": r"\b\w+@\w+\.\w+\b",
                "output_field": "emails"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"text": "Contact us at hello@example.com or support@test.org"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["emails"], json!(["hello@example.com", "support@test.org"]));
        }
    }

    #[test]
    fn test_text_concat() {
        let node = TextNode::new(
            uuid::Uuid::new_v4(),
            "Test Text".to_string(),
            "text".to_string(),
            None,
            Some(json!({
                "operation": "concat",
                "concat_values": [" ", "World", "!"],
                "separator": "",
                "output_field": "concatenated"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"text": "Hello"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["concatenated"], "Hello World!");
        }
    }
}