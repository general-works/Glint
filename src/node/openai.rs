use crate::execution::NodeOutput;
use crate::expression;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use regex::Regex;
use reqwest;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenaiNode {
    pub id: super::NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

impl super::INode for OpenaiNode {
    fn new(
        id: super::NodeId,
        name: String,
        _type: String,
        description: Option<String>,
        parameters: Option<Value>,
    ) -> Self {
        Self { id, name, description, parameters }
    }

    fn id(&self) -> super::NodeId {
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

    fn execute(&self, input: &NodeOutput) -> Result<Value, super::Error> {
        // Use tokio's block_on to run async code
        tokio::runtime::Runtime::new()
            .map_err(|e| super::Error::NodeExecution(format!("Failed to create async runtime: {}", e)))?
            .block_on(self.execute_async(input))
    }

    fn validate(&self) -> bool {
        self.validate_parameters()
    }

    fn dependencies(&self) -> Vec<super::NodeId> {
        self.extract_dependencies_from_parameters()
    }
}

impl OpenaiNode {
    /// Asynchronously execute OpenAI API request
    async fn execute_async(&self, input: &NodeOutput) -> Result<Value, super::Error> {
        // Check if parameters exist
        let Some(params) = &self.parameters else {
            return Err(super::Error::NodeExecution("Missing required parameters".to_string()));
        };

        // Process expressions in parameters, replace content in {{}}
        let processed_params = self.process_parameters(params, input)?;

        // Extract parameters required for OpenAI API
        let model = processed_params.get("model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| super::Error::NodeExecution("Missing model parameter".to_string()))?;

        let messages = processed_params.get("messages")
            .ok_or_else(|| super::Error::NodeExecution("Missing messages parameter".to_string()))?;

        let api_key = processed_params.get("credentials")
            .and_then(|c| c.get("api_key"))
            .and_then(|k| k.as_str())
            .ok_or_else(|| super::Error::NodeExecution("Missing API key".to_string()))?;

        // Build request body
        let request_body = json!({
            "model": model,
            "messages": messages
        });

        // Send HTTP request
        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/chat/completions") // Fixed URL
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| super::Error::NodeExecution(format!("HTTP request failed: {}", e)))?;

        // Check response status
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unable to get error information".to_string());
            return Err(super::Error::NodeExecution(format!(
                "OpenAI API request failed (status code: {}): {}", 
                status, 
                error_text
            )));
        }

        // Parse response
        let response_json: Value = response.json().await
            .map_err(|e| super::Error::NodeExecution(format!("Failed to parse response JSON: {}", e)))?;

        Ok(response_json)
    }

    /// Process expressions in parameters, replace content in {{}} with actual values
    fn process_parameters(&self, params: &Value, input: &NodeOutput) -> Result<Value, super::Error> {
        self.process_value(params, input)
    }

    /// Recursively process JSON values, replace expressions within them
    fn process_value(&self, value: &Value, input: &NodeOutput) -> Result<Value, super::Error> {
        match value {
            Value::String(s) => {
                // Process {{}} expressions in strings
                self.process_string_with_expressions(s, input)
            },
            Value::Array(arr) => {
                let mut processed_array = Vec::new();
                for item in arr {
                    let processed_item = self.process_value(item, input)?;
                    processed_array.push(processed_item);
                }
                Ok(Value::Array(processed_array))
            },
            Value::Object(obj) => {
                let mut processed_object = serde_json::Map::new();
                for (key, val) in obj {
                    let processed_val = self.process_value(val, input)?;
                    processed_object.insert(key.clone(), processed_val);
                }
                Ok(Value::Object(processed_object))
            },
            // Other types return directly
            _ => Ok(value.clone()),
        }
    }

    /// Process {{}} expressions in strings, replace with actual values
    fn process_string_with_expressions(&self, text: &str, input: &NodeOutput) -> Result<Value, super::Error> {
        let re = Regex::new(r"\{\{(.*?)\}\}").unwrap();
        let mut result = text.to_string();

        // Find all expressions and replace them
        for cap in re.captures_iter(text) {
            if let Some(expr_match) = cap.get(0) {
                if let Some(expr) = cap.get(1) {
                    let expression = expr.as_str();

                    // Validate expression and extract value
                    if !expression::is_expression(expression) {
                        return Err(super::Error::ExpressionEvaluation(format!(
                            "Invalid expression: {}", expression
                        )));
                    }

                    match expression::extract_value(expression, input) {
                        Some(extracted_value) => {
                            // Convert extracted value to string
                            let value_str = match extracted_value {
                                Value::String(s) => s,
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "null".to_string(),
                                _ => extracted_value.to_string(),
                            };
                            
                            // Replace expression
                            result = result.replace(expr_match.as_str(), &value_str);
                        },
                        None => {
                            return Err(super::Error::ExpressionEvaluation(format!(
                                "Cannot parse expression or find corresponding value: {}", expression
                            )));
                        }
                    }
                }
            }
        }

        Ok(Value::String(result))
    }

    /// Validate all expressions in parameters are valid
    /// Traverse all content in messages field, validate that content wrapped in {{}} are valid expressions
    fn validate_parameters(&self) -> bool {
        if let Some(params) = &self.parameters {
            self.validate_value(params)
        } else {
            true // If no parameters, consider valid
        }
    }
    
    /// Recursively validate all expressions in JSON values
    fn validate_value(&self, value: &Value) -> bool {
        match value {
            Value::String(s) => {
                self.validate_expressions_in_string(s)
            },
            Value::Array(arr) => {
                arr.iter().all(|item| self.validate_value(item))
            },
            Value::Object(obj) => {
                obj.values().all(|v| self.validate_value(v))
            },
            _ => true // Other types don't need validation
        }
    }
    
    /// Validate all content wrapped in {{}} in strings are valid expressions
    fn validate_expressions_in_string(&self, text: &str) -> bool {
        // Use regex to match {{...}} format expressions
        let re = Regex::new(r"\{\{(.*?)\}\}").unwrap();
        
        for cap in re.captures_iter(text) {
            if let Some(expr) = cap.get(1) {
                let expression = expr.as_str();
                
                // Check if it's a valid expression
                if !expression::is_expression(expression) {
                    return false; // Content in {{}} is not an expression
                }
                
                // Validate if expression format is correct
                if let Err(_) = expression::validate_expression(expression) {
                    return false; // Expression format is invalid
                }
            }
        }
        
        true
    }
    
    /// Extract dependent node IDs from parameters
    /// Traverse all content in messages field, extract node IDs from expressions wrapped in {{}}
    fn extract_dependencies_from_parameters(&self) -> Vec<super::NodeId> {
        let mut dependencies = Vec::new();
        
        if let Some(params) = &self.parameters {
            // Extract all string values from parameters and find expressions
            self.extract_from_value(params, &mut dependencies);
        }
        
        // Remove duplicates
        dependencies.sort();
        dependencies.dedup();
        dependencies
    }
    
    /// Recursively extract all expressions from JSON values
    fn extract_from_value(&self, value: &Value, dependencies: &mut Vec<super::NodeId>) {
        match value {
            Value::String(s) => {
                self.extract_node_ids_from_string(s, dependencies);
            },
            Value::Array(arr) => {
                for item in arr {
                    self.extract_from_value(item, dependencies);
                }
            },
            Value::Object(obj) => {
                for (_, v) in obj {
                    self.extract_from_value(v, dependencies);
                }
            },
            _ => {} // Other types don't need processing
        }
    }
    
    /// Extract node IDs from all expressions wrapped in {{}} in strings
    fn extract_node_ids_from_string(&self, text: &str, dependencies: &mut Vec<super::NodeId>) {
        // Use regex to match {{...}} format expressions
        // Modified regex to correctly match expressions containing } characters
        let re = Regex::new(r"\{\{(.*?)\}\}").unwrap();
        
        for cap in re.captures_iter(text) {
            if let Some(expr) = cap.get(1) {
                let expression = expr.as_str();
                
                // Check if it's an expression (starts with =)
                if expression::is_expression(expression) {
                    // Use expression module's parsing functionality to extract node ID
                    if let Some(node_id) = self.parse_node_id_from_expression(expression) {
                        dependencies.push(node_id);
                    }
                }
            }
        }
    }
    
    /// Parse node ID from expression
    /// Expression format: =${node_id}.path or =${node_id}
    fn parse_node_id_from_expression(&self, expression: &str) -> Option<super::NodeId> {
        // Check if expression starts with =
        if !expression.starts_with('=') {
            return None;
        }
        
        // Remove leading = symbol
        let expr = &expression[1..];
        
        // Check if it starts with ${
        if !expr.starts_with("${") {
            return None;
        }
        
        // Find position of }
        if let Some(close_brace) = expr.find('}') {
            // Extract node ID string
            let node_id_str = &expr[2..close_brace];
            
            // Try to parse as UUID
            if let Ok(node_id) = Uuid::parse_str(node_id_str) {
                return Some(node_id);
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::INode;
    use serde_json::json;

    #[test]
    fn test_extract_dependencies_from_openai_example() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let parameters = json!({
            "model": "gpt-4",
            "credentials": {
                "api_key": "sk-proj-1234567890"
            },
            "messages": [
                {
                    "role": "system",
                    "content": "You are a travel planner, please create a travel plan based on user requirements"
                },
                {
                    "role": "user",
                    "content": "I need to travel to {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}} at {{=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}.date}}"
                }
            ]
        });

        let openai_node = OpenaiNode::new(
            node_id,
            "openai".to_string(),
            "openai".to_string(),
            Some("Create a travel plan".to_string()),
            Some(parameters),
        );

        let dependencies = openai_node.dependencies();
        
        // Should find two dependent node IDs
        assert_eq!(dependencies.len(), 2);
        
        let expected_ids = vec![
            Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap(),
            Uuid::parse_str("f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90").unwrap(),
        ];
        
        for expected_id in expected_ids {
            assert!(dependencies.contains(&expected_id), "Missing dependency: {}", expected_id);
        }
    }

    #[test]
    fn test_extract_node_ids_from_string() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let openai_node = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            None,
        );

        let mut dependencies = Vec::new();
        
        // Test single expression
        let text1 = "I need to travel to {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}}";
        openai_node.extract_node_ids_from_string(text1, &mut dependencies);
        
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0], Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap());
        
        // Test multiple expressions
        dependencies.clear();
        let text2 = "From {{=${node1-uuid-here}.start}} to {{=${node2-uuid-here}.end}}";
        openai_node.extract_node_ids_from_string(text2, &mut dependencies);
        
        // This test should not find any dependencies because UUID format is incorrect
        assert_eq!(dependencies.len(), 0);
        
        // Test multiple expressions with correct format
        dependencies.clear();
        let text3 = "Time {{=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}.date}}, location {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}}";
        openai_node.extract_node_ids_from_string(text3, &mut dependencies);
        
        assert_eq!(dependencies.len(), 2);
    }

    #[test]
    fn test_parse_node_id_from_expression() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let openai_node = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            None,
        );

        // Test valid expressions
        let expr1 = "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination";
        let result1 = openai_node.parse_node_id_from_expression(expr1);
        assert_eq!(result1, Some(Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap()));

        // Test expression with only node ID
        let expr2 = "=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}";
        let result2 = openai_node.parse_node_id_from_expression(expr2);
        assert_eq!(result2, Some(Uuid::parse_str("f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90").unwrap()));

        // Test invalid expression
        let expr3 = "${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination"; // Missing =
        let result3 = openai_node.parse_node_id_from_expression(expr3);
        assert_eq!(result3, None);

        // Test invalid UUID
        let expr4 = "=${invalid-uuid}.destination";
        let result4 = openai_node.parse_node_id_from_expression(expr4);
        assert_eq!(result4, None);
    }

    #[test]
    fn test_validate_openai_example() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let valid_parameters = json!({
            "model": "gpt-4",
            "credentials": {
                "api_key": "sk-proj-1234567890"
            },
            "messages": [
                {
                    "role": "system",
                    "content": "You are a travel planner, please create a travel plan based on user requirements"
                },
                {
                    "role": "user",
                    "content": "I need to travel to {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}} at {{=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}.date}}"
                }
            ]
        });

        let valid_node = OpenaiNode::new(
            node_id,
            "openai".to_string(),
            "openai".to_string(),
            Some("Create a travel plan".to_string()),
            Some(valid_parameters),
        );

        assert!(valid_node.validate(), "Valid OpenAI node should pass validation");
    }

    #[test]
    fn test_validate_invalid_expressions() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        
        // Test invalid expression: content in {{}} is not an expression
        let invalid_parameters1 = json!({
            "messages": [
                {
                    "role": "user",
                    "content": "Hello {{world}}!"  // Not an expression, missing =
                }
            ]
        });

        let invalid_node1 = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            Some(invalid_parameters1),
        );

        assert!(!invalid_node1.validate(), "Node with invalid expression should fail validation");

        // Test invalid expression format
        let invalid_parameters2 = json!({
            "messages": [
                {
                    "role": "user",
                    "content": "Value: {{=${invalid-uuid-format}.value}}"  // Invalid UUID format
                }
            ]
        });

        let invalid_node2 = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            Some(invalid_parameters2),
        );

        assert!(!invalid_node2.validate(), "Node with invalid UUID format should fail validation");
    }

    #[test]
    fn test_validate_expressions_in_string() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let openai_node = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            None,
        );

        // Test valid expressions
        let valid_text1 = "I need to travel to {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}}";
        assert!(openai_node.validate_expressions_in_string(valid_text1));

        let valid_text2 = "Time {{=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}.date}}, location {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}}";
        assert!(openai_node.validate_expressions_in_string(valid_text2));

        // Test invalid expressions
        let invalid_text1 = "Hello {{world}}!"; // Not an expression
        assert!(!openai_node.validate_expressions_in_string(invalid_text1));

        let invalid_text2 = "Value: {{=${invalid-uuid}.value}}"; // Invalid UUID
        assert!(!openai_node.validate_expressions_in_string(invalid_text2));

        // Test text without expressions
        let no_expression_text = "This is a normal text";
        assert!(openai_node.validate_expressions_in_string(no_expression_text));
    }

    #[test]
    fn test_validate_no_parameters() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let node_without_params = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            None, // No parameters
        );

        assert!(node_without_params.validate(), "Node without parameters should be valid");
    }

    #[test]
    fn test_process_string_with_expressions() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let openai_node = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            None,
        );

        // Create test input data
        let mut input = std::collections::HashMap::new();
        let destination_id = Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap();
        let time_id = Uuid::parse_str("f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90").unwrap();
        
        input.insert(destination_id, json!({
            "destination": "Paris"
        }));
        input.insert(time_id, json!({
            "date": "2024-12-25"
        }));

        // Test string expression replacement
        let text = "I need to travel to {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}} at {{=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}.date}}";
        let result = openai_node.process_string_with_expressions(text, &input).unwrap();
        
        if let Value::String(processed_text) = result {
            assert_eq!(processed_text, "I need to travel to Paris at 2024-12-25");
        } else {
            panic!("Expected String result");
        }
    }

    #[test]
    fn test_process_parameters() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let openai_node = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            None,
        );

        // Create test input data
        let mut input = std::collections::HashMap::new();
        let destination_id = Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap();
        let time_id = Uuid::parse_str("f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90").unwrap();
        
        input.insert(destination_id, json!({
            "destination": "Paris"
        }));
        input.insert(time_id, json!({
            "date": "2024-12-25"
        }));

        // Create parameters with expressions
        let params = json!({
            "model": "gpt-4",
            "credentials": {
                "api_key": "sk-proj-1234567890"
            },
            "messages": [
                {
                    "role": "system",
                    "content": "You are a travel planner, please create a travel plan based on user requirements"
                },
                {
                    "role": "user",
                    "content": "I need to travel to {{=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.destination}} at {{=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}.date}}"
                }
            ]
        });

        let processed = openai_node.process_parameters(&params, &input).unwrap();
        
        // Verify expressions are correctly replaced
        let messages = processed.get("messages").unwrap().as_array().unwrap();
        let user_message = &messages[1];
        let content = user_message.get("content").unwrap().as_str().unwrap();
        
        assert_eq!(content, "I need to travel to Paris at 2024-12-25");
    }

    #[test]
    fn test_execute_missing_parameters() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let openai_node = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            None, // No parameters
        );

        let input = std::collections::HashMap::new();
        let result = openai_node.execute(&input);
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Missing required parameters"));
        }
    }

    #[test]
    fn test_execute_missing_model() {
        let node_id = Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
        let params = json!({
            "credentials": {
                "api_key": "sk-proj-1234567890"
            },
            "messages": []
            // Missing model
        });
        
        let openai_node = OpenaiNode::new(
            node_id,
            "test".to_string(),
            "openai".to_string(),
            None,
            Some(params),
        );

        let input = std::collections::HashMap::new();
        let result = openai_node.execute(&input);
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Missing model parameter"));
        }
    }
}