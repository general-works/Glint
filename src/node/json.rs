use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonParams {
    pub operation: String, // "parse", "stringify", "validate", "extract", "merge"
    pub json_property: String, // Property containing JSON string to parse
    pub target_property: String, // Property to store result
    #[serde(default = "default_true")]
    pub include_input_fields: bool, // Whether to include original fields in output
    #[serde(default)]
    pub pretty_print: bool, // For stringify operation
    pub extract_path: Option<String>, // For extract operation (JSONPath-like)
}

fn default_true() -> bool {
    true
}

impl Default for JsonParams {
    fn default() -> Self {
        Self {
            operation: "parse".to_string(),
            json_property: "json".to_string(),
            target_property: "data".to_string(),
            include_input_fields: true,
            pretty_print: false,
            extract_path: None,
        }
    }
}

impl INode for JsonNode {
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
            serde_json::from_value::<JsonParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            JsonParams::default()
        };

        // Combine all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        if input_data.is_empty() {
            return Ok(json!([]));
        }

        let mut results = Vec::new();
        
        for item in input_data {
            if let Value::Array(items) = item {
                // If input is an array, process each item
                for single_item in items {
                    let result = self.process_json_operation(&params, &single_item)?;
                    results.push(result);
                }
            } else {
                // If input is a single item, process it
                let result = self.process_json_operation(&params, &item)?;
                results.push(result);
            }
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<JsonParams>(params_value.clone()) {
                return matches!(params.operation.as_str(), "parse" | "stringify" | "validate" | "extract" | "merge") &&
                       !params.json_property.is_empty() &&
                       !params.target_property.is_empty();
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl JsonNode {
    fn process_json_operation(&self, params: &JsonParams, item: &Value) -> Result<Value, Error> {
        match params.operation.as_str() {
            "parse" => self.parse_json(params, item),
            "stringify" => self.stringify_json(params, item),
            "validate" => self.validate_json(params, item),
            "extract" => self.extract_json_path(params, item),
            "merge" => self.merge_json(params, item),
            _ => Err(Error::Validation(format!("Unsupported JSON operation: {}", params.operation)))
        }
    }

    fn parse_json(&self, params: &JsonParams, item: &Value) -> Result<Value, Error> {
        // Get the JSON string from the specified property
        let json_string = if let Some(json_value) = item.get(&params.json_property) {
            if let Some(s) = json_value.as_str() {
                s
            } else {
                return Err(Error::Validation(format!("Property '{}' is not a string", params.json_property)));
            }
        } else {
            return Err(Error::Validation(format!("Property '{}' not found in input", params.json_property)));
        };

        // Parse the JSON string
        let parsed_json: Value = serde_json::from_str(json_string)
            .map_err(|e| Error::Validation(format!("Failed to parse JSON: {}", e)))?;

        // Create the result
        let mut result = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Add the parsed data to the target property
        if let Value::Object(ref mut map) = result {
            map.insert(params.target_property.clone(), parsed_json);
        } else {
            result = json!({
                params.target_property.clone(): parsed_json
            });
        }

        Ok(result)
    }

    fn stringify_json(&self, params: &JsonParams, item: &Value) -> Result<Value, Error> {
        // Get the value from the specified property
        let json_value = if let Some(value) = item.get(&params.json_property) {
            value
        } else {
            return Err(Error::Validation(format!("Property '{}' not found in input", params.json_property)));
        };

        // Convert to JSON string
        let json_string = if params.pretty_print {
            serde_json::to_string_pretty(json_value)
                .map_err(|e| Error::Validation(format!("Failed to stringify JSON: {}", e)))?
        } else {
            serde_json::to_string(json_value)
                .map_err(|e| Error::Validation(format!("Failed to stringify JSON: {}", e)))?
        };

        // Create the result
        let mut result = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Add the stringified data to the target property
        if let Value::Object(ref mut map) = result {
            map.insert(params.target_property.clone(), Value::String(json_string));
        } else {
            result = json!({
                params.target_property.clone(): json_string
            });
        }

        Ok(result)
    }

    fn validate_json(&self, params: &JsonParams, item: &Value) -> Result<Value, Error> {
        // Get the JSON string from the specified property
        let json_string = if let Some(json_value) = item.get(&params.json_property) {
            if let Some(s) = json_value.as_str() {
                s
            } else {
                return Err(Error::Validation(format!("Property '{}' is not a string", params.json_property)));
            }
        } else {
            return Err(Error::Validation(format!("Property '{}' not found in input", params.json_property)));
        };

        // Try to parse the JSON string to validate it
        let validation_result = match serde_json::from_str::<Value>(json_string) {
            Ok(_) => json!({
                "valid": true,
                "error": null
            }),
            Err(e) => json!({
                "valid": false,
                "error": e.to_string()
            })
        };

        // Create the result
        let mut result = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Add the validation result to the target property
        if let Value::Object(ref mut map) = result {
            map.insert(params.target_property.clone(), validation_result);
        } else {
            result = json!({
                params.target_property.clone(): validation_result
            });
        }

        Ok(result)
    }

    fn extract_json_path(&self, params: &JsonParams, item: &Value) -> Result<Value, Error> {
        // Get the JSON value from the specified property
        let json_value = if let Some(value) = item.get(&params.json_property) {
            value
        } else {
            return Err(Error::Validation(format!("Property '{}' not found in input", params.json_property)));
        };

        // Extract value using the path (simple dot notation support)
        let extracted_value = if let Some(path) = &params.extract_path {
            self.extract_by_path(json_value, path)?
        } else {
            json_value.clone()
        };

        // Create the result
        let mut result = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Add the extracted data to the target property
        if let Value::Object(ref mut map) = result {
            map.insert(params.target_property.clone(), extracted_value);
        } else {
            result = json!({
                params.target_property.clone(): extracted_value
            });
        }

        Ok(result)
    }

    fn extract_by_path(&self, value: &Value, path: &str) -> Result<Value, Error> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            // Handle array indices
            if part.starts_with('[') && part.ends_with(']') {
                let index_str = &part[1..part.len()-1];
                if let Ok(index) = index_str.parse::<usize>() {
                    if let Some(array_value) = current.get(index) {
                        current = array_value;
                    } else {
                        return Ok(Value::Null);
                    }
                } else {
                    return Err(Error::Validation(format!("Invalid array index: {}", index_str)));
                }
            } else if let Some(next) = current.get(part) {
                current = next;
            } else {
                return Ok(Value::Null);
            }
        }

        Ok(current.clone())
    }

    fn merge_json(&self, params: &JsonParams, item: &Value) -> Result<Value, Error> {
        // Get the JSON value to merge
        let merge_value = if let Some(value) = item.get(&params.json_property) {
            value
        } else {
            return Err(Error::Validation(format!("Property '{}' not found in input", params.json_property)));
        };

        // Create the result by merging
        let mut result = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Perform deep merge of objects
        if let (Value::Object(result_map), Value::Object(merge_map)) = (&mut result, merge_value) {
            for (key, value) in merge_map {
                result_map.insert(key.clone(), value.clone());
            }
        } else {
            // If not both objects, just set the target property
            if let Value::Object(ref mut map) = result {
                map.insert(params.target_property.clone(), merge_value.clone());
            } else {
                result = json!({
                    params.target_property.clone(): merge_value
                });
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_json_node_creation() {
        let node = JsonNode::new(
            uuid::Uuid::new_v4(),
            "Test JSON".to_string(),
            "json".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "operation": "parse",
                "json_property": "data",
                "target_property": "parsed",
                "include_input_fields": true
            })),
        );

        assert_eq!(node.name(), "Test JSON");
        assert!(node.validate());
    }

    #[test]
    fn test_json_validation() {
        let node = JsonNode::new(
            uuid::Uuid::new_v4(),
            "Test JSON".to_string(),
            "json".to_string(),
            None,
            Some(json!({
                "operation": "invalid_operation",
                "json_property": "data",
                "target_property": "result"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_json_parse_operation() {
        let node = JsonNode::new(
            uuid::Uuid::new_v4(),
            "Test JSON".to_string(),
            "json".to_string(),
            None,
            Some(json!({
                "operation": "parse",
                "json_property": "json_string",
                "target_property": "parsed_data",
                "include_input_fields": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "json_string": "{\"name\": \"test\", \"value\": 123}",
                "other_field": "keep this"
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert!(item["parsed_data"]["name"].as_str().unwrap() == "test");
            assert!(item["parsed_data"]["value"].as_i64().unwrap() == 123);
            assert!(item["other_field"].as_str().unwrap() == "keep this");
        }
    }

    #[test]
    fn test_json_stringify_operation() {
        let node = JsonNode::new(
            uuid::Uuid::new_v4(),
            "Test JSON".to_string(),
            "json".to_string(),
            None,
            Some(json!({
                "operation": "stringify",
                "json_property": "data",
                "target_property": "json_string",
                "include_input_fields": false,
                "pretty_print": false
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "data": {"name": "test", "value": 123},
                "other_field": "remove this"
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            let json_string = item["json_string"].as_str().unwrap();
            assert!(json_string.contains("\"name\":\"test\""));
            assert!(json_string.contains("\"value\":123"));
            assert!(!item.as_object().unwrap().contains_key("other_field"));
        }
    }

    #[test]
    fn test_json_validate_operation() {
        let node = JsonNode::new(
            uuid::Uuid::new_v4(),
            "Test JSON".to_string(),
            "json".to_string(),
            None,
            Some(json!({
                "operation": "validate",
                "json_property": "json_string",
                "target_property": "validation_result",
                "include_input_fields": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "json_string": "{\"valid\": true}",
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert_eq!(item["validation_result"]["valid"], true);
            assert!(item["validation_result"]["error"].is_null());
        }
    }

    #[test]
    fn test_json_extract_operation() {
        let node = JsonNode::new(
            uuid::Uuid::new_v4(),
            "Test JSON".to_string(),
            "json".to_string(),
            None,
            Some(json!({
                "operation": "extract",
                "json_property": "data",
                "target_property": "extracted",
                "include_input_fields": false,
                "extract_path": "user.name"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "data": {
                    "user": {
                        "name": "John Doe",
                        "age": 30
                    },
                    "timestamp": "2023-01-01"
                }
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert_eq!(item["extracted"].as_str().unwrap(), "John Doe");
        }
    }
}