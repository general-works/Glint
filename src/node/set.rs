use crate::execution::NodeOutput;
use crate::expression;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetNode {
    pub id: super::NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

impl super::INode for SetNode {
    fn new(
        id: super::NodeId,
        name: String,
        _type: String,
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
        // If no parameters, return empty object
        let Some(params) = &self.parameters else {
            return Ok(Value::Object(serde_json::Map::new()));
        };

        // Process expressions in parameters, return processed parameter copy
        // Note: self.parameters remains unchanged, we use temporary variables to construct return result
        let processed_params = self.process_parameters(params, input)?;

        Ok(processed_params)
    }

    fn validate(&self) -> bool {
        // Validate node name cannot be empty
        if self.name.trim().is_empty() {
            return false;
        }

        // If parameters exist, validate them
        if let Some(params) = &self.parameters {
            if params.is_null() {
                return false;
            }

            // Validate parameter validity
            if !self.validate_parameters(params) {
                return false;
            }
        }

        true
    }

    fn dependencies(&self) -> Vec<super::NodeId> {
        let mut deps = Vec::new();
        if let Some(params) = &self.parameters {
            self.extract_dependencies_from_value(params, &mut deps);
        }
        // Remove duplicates and sort
        deps.sort();
        deps.dedup();
        deps
    }
}

impl SetNode {
    /// Process expressions in parameters, return processed parameter copy
    fn process_parameters(
        &self,
        params: &Value,
        input: &NodeOutput,
    ) -> Result<Value, super::Error> {
        self.process_value(params, input)
    }

    /// Recursively process JSON values, parse and replace expressions within them
    fn process_value(&self, value: &Value, input: &NodeOutput) -> Result<Value, super::Error> {
        match value {
            // If it's a string, check if it's an expression
            Value::String(s) => {
                if expression::is_expression(s) {
                    // Parse expression and extract value from input
                    match expression::extract_value(s, input) {
                        Some(extracted_value) => Ok(extracted_value),
                        None => Err(super::Error::ExpressionEvaluation(format!(
                            "Cannot parse expression or find corresponding value: {}",
                            s
                        ))),
                    }
                } else {
                    // Not an expression, keep original value
                    Ok(value.clone())
                }
            }
            // If it's an array, recursively process each element
            Value::Array(arr) => {
                let mut processed_array = Vec::new();
                for item in arr {
                    let processed_item = self.process_value(item, input)?;
                    processed_array.push(processed_item);
                }
                Ok(Value::Array(processed_array))
            }
            // If it's an object, recursively process each field
            Value::Object(obj) => {
                let mut processed_object = serde_json::Map::new();
                for (key, val) in obj {
                    let processed_val = self.process_value(val, input)?;
                    processed_object.insert(key.clone(), processed_val);
                }
                Ok(Value::Object(processed_object))
            }
            // Other types (numbers, booleans, null) return directly
            _ => Ok(value.clone()),
        }
    }

    /// Validate parameter validity
    fn validate_parameters(&self, params: &Value) -> bool {
        match params {
            // If it's an object, recursively validate each field
            Value::Object(obj) => {
                for (_, value) in obj {
                    if !self.validate_parameter_value(value) {
                        return false;
                    }
                }
                true
            }
            // If not an object, directly validate this value
            _ => self.validate_parameter_value(params),
        }
    }

    /// Validate validity of a single parameter value
    fn validate_parameter_value(&self, value: &Value) -> bool {
        match value {
            // If it's a string, check if it's an expression
            Value::String(s) => {
                if expression::is_expression(s) {
                    // Validate expression syntax
                    expression::validate_expression(s).is_ok()
                } else {
                    // Basic string type, validate not empty
                    !s.is_empty()
                }
            }
            // If it's an array, recursively validate each element
            Value::Array(arr) => {
                for item in arr {
                    if !self.validate_parameter_value(item) {
                        return false;
                    }
                }
                true
            }
            // If it's an object, recursively validate
            Value::Object(obj) => {
                for (_, v) in obj {
                    if !self.validate_parameter_value(v) {
                        return false;
                    }
                }
                true
            }
            // Basic types (numbers, booleans) are all valid
            Value::Number(_) | Value::Bool(_) => true,
            // null values are invalid
            Value::Null => false,
        }
    }

    /// Recursively extract dependent node IDs from JSON values
    fn extract_dependencies_from_value(&self, value: &Value, deps: &mut Vec<super::NodeId>) {
        match value {
            Value::String(s) => {
                if expression::is_expression(s) {
                    if let Some(node_id) = self.extract_node_id_from_expression(s) {
                        deps.push(node_id);
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.extract_dependencies_from_value(item, deps);
                }
            }
            Value::Object(obj) => {
                for (_, val) in obj {
                    self.extract_dependencies_from_value(val, deps);
                }
            }
            _ => {} // Other types have no dependencies
        }
    }

    /// Extract node ID from expression
    fn extract_node_id_from_expression(&self, expression: &str) -> Option<super::NodeId> {
        use crate::expression;

        // Validate expression format
        if expression::validate_expression(expression).is_err() {
            return None;
        }

        // Remove leading = symbol
        let expr = &expression[1..];

        // Check if it starts with ${
        if !expr.starts_with("${") {
            return None;
        }

        // Find position of }
        let close_brace = expr.find('}')?;

        // Extract node ID string
        let node_id_str = &expr[2..close_brace];

        // Parse as UUID
        uuid::Uuid::parse_str(node_id_str).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::INode;
    use serde_json::json;

    #[test]
    fn test_validate_basic_types() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_node".to_string(),
            description: Some("Test node".to_string()),
            parameters: Some(json!({
                "string_value": "hello",
                "number_value": 42,
                "boolean_value": true,
                "array_value": [1, 2, 3],
                "object_value": {
                    "nested": "value"
                }
            })),
        };

        assert!(set_node.validate());
    }

    #[test]
    fn test_validate_valid_expressions() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_node".to_string(),
            description: Some("Test node".to_string()),
            parameters: Some(json!({
                "user": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user",
                "email": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.email",
                "contact": "=${f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90}.contacts[0].value"
            })),
        };

        assert!(set_node.validate());
    }

    #[test]
    fn test_validate_invalid_expressions() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_node".to_string(),
            description: Some("Test node".to_string()),
            parameters: Some(json!({
                "invalid_expr1": "=${invalid-uuid}.user",
                "valid_value": "normal string"
            })),
        };

        assert!(!set_node.validate());
    }

    #[test]
    fn test_validate_null_values() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_node".to_string(),
            description: Some("Test node".to_string()),
            parameters: Some(json!({
                "null_value": null,
                "valid_value": "test"
            })),
        };

        assert!(!set_node.validate());
    }

    #[test]
    fn test_validate_empty_strings() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_node".to_string(),
            description: Some("Test node".to_string()),
            parameters: Some(json!({
                "empty_string": "",
                "valid_value": "test"
            })),
        };

        assert!(!set_node.validate());
    }

    #[test]
    fn test_validate_nested_structure() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_node".to_string(),
            description: Some("Test node".to_string()),
            parameters: Some(json!({
                "nested": {
                    "expression": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.data",
                    "array": [
                        "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.items[0]",
                        "normal string",
                        123
                    ]
                }
            })),
        };

        assert!(set_node.validate());
    }

    #[test]
    fn test_validate_empty_name() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "".to_string(),
            description: Some("Test node".to_string()),
            parameters: Some(json!({"test": "value"})),
        };

        assert!(!set_node.validate());
    }

    #[test]
    fn test_validate_no_parameters() {
        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_node".to_string(),
            description: Some("Test node".to_string()),
            parameters: None,
        };

        assert!(set_node.validate());
    }

    #[test]
    fn test_execute_with_expressions() {
        use std::collections::HashMap;
        use uuid::Uuid;

        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_set_node".to_string(),
            description: Some("Test set node with expressions".to_string()),
            parameters: Some(json!({
                "user_name": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user.name",
                "user_email": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.user.email",
                "static_value": "This is a static value",
                "number_value": 42,
                "contact_phone": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.contacts[0].value"
            })),
        };

        // Create mock input data
        let source_node_id = Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap();
        let mut input = HashMap::new();
        input.insert(
            source_node_id,
            json!({
                "user": {
                    "name": "Zhang San",
                    "email": "zhangsan@example.com"
                },
                "contacts": [
                    {"type": "phone", "value": "138-0000-0000"},
                    {"type": "email", "value": "work@example.com"}
                ]
            }),
        );

        // Execute node
        let result = set_node.execute(&input).unwrap();

        // Verify result
        let expected = json!({
            "user_name": "Zhang San",
            "user_email": "zhangsan@example.com",
            "static_value": "This is a static value",
            "number_value": 42,
            "contact_phone": "138-0000-0000"
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn test_execute_with_nested_parameters() {
        use std::collections::HashMap;
        use uuid::Uuid;

        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_nested_node".to_string(),
            description: Some("Test set node with nested parameters".to_string()),
            parameters: Some(json!({
                "person": {
                    "name": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.name",
                    "age": 25,
                    "contacts": [
                        "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.email",
                        "Fixed contact method"
                    ]
                },
                "metadata": {
                    "processed": true,
                    "source": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.source"
                }
            })),
        };

        // Create mock input data
        let source_node_id = Uuid::parse_str("8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d").unwrap();
        let mut input = HashMap::new();
        input.insert(
            source_node_id,
            json!({
                "name": "Li Si",
                "email": "lisi@example.com",
                "source": "webhook"
            }),
        );

        // Execute node
        let result = set_node.execute(&input).unwrap();

        // Verify result
        let expected = json!({
            "person": {
                "name": "Li Si",
                "age": 25,
                "contacts": [
                    "lisi@example.com",
                    "Fixed contact method"
                ]
            },
            "metadata": {
                "processed": true,
                "source": "webhook"
            }
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn test_execute_no_parameters() {
        use std::collections::HashMap;

        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_empty_node".to_string(),
            description: Some("Test set node with no parameters".to_string()),
            parameters: None,
        };

        let input = HashMap::new();
        let result = set_node.execute(&input).unwrap();

        // Should return empty object when no parameters
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_execute_expression_not_found() {
        use std::collections::HashMap;

        let set_node = SetNode {
            id: uuid::Uuid::new_v4(),
            name: "test_error_node".to_string(),
            description: Some("Test set node with missing expression data".to_string()),
            parameters: Some(json!({
                "missing_data": "=${8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d}.nonexistent.field"
            })),
        };

        // No corresponding node ID in input data
        let input = HashMap::new();
        let result = set_node.execute(&input);

        // Should return error
        assert!(result.is_err());
    }
}
