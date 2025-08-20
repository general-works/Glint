use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitParams {
    pub mode: String, // "batches", "fields", "items"
    // For batch mode
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    // For field split mode
    pub field_to_split: Option<String>,
    #[serde(default = "default_include")]
    pub include: String, // "noOtherFields", "allOtherFields", "selectedFields"
    pub fields_to_include: Option<Vec<String>>,
    pub destination_field: Option<String>,
    // General options
    #[serde(default = "default_true")]
    pub include_binary: bool,
    #[serde(default)]
    pub disable_dot_notation: bool,
}

fn default_batch_size() -> usize {
    10
}

fn default_include() -> String {
    "noOtherFields".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for SplitParams {
    fn default() -> Self {
        Self {
            mode: "items".to_string(),
            batch_size: 10,
            field_to_split: None,
            include: "noOtherFields".to_string(),
            fields_to_include: None,
            destination_field: None,
            include_binary: true,
            disable_dot_notation: false,
        }
    }
}

impl INode for SplitNode {
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
            serde_json::from_value::<SplitParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            SplitParams::default()
        };

        // Combine all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        if input_data.is_empty() {
            return Ok(json!([]));
        }

        match params.mode.as_str() {
            "batches" => self.split_in_batches(&params, &input_data),
            "fields" => self.split_out_fields(&params, &input_data),
            "items" => self.split_items(&params, &input_data),
            _ => Err(Error::Validation(format!("Unsupported split mode: {}", params.mode)))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<SplitParams>(params_value.clone()) {
                return matches!(params.mode.as_str(), "batches" | "fields" | "items") &&
                       (params.mode != "batches" || params.batch_size > 0) &&
                       (params.mode != "fields" || params.field_to_split.is_some());
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl SplitNode {
    fn split_in_batches(&self, params: &SplitParams, input_data: &[Value]) -> Result<Value, Error> {
        let mut all_items = Vec::new();
        
        // Flatten all input data into individual items
        for item in input_data {
            if let Value::Array(items) = item {
                all_items.extend(items.clone());
            } else {
                all_items.push(item.clone());
            }
        }

        // Split into batches
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let total_items = all_items.len();
        
        for (index, item) in all_items.into_iter().enumerate() {
            current_batch.push(item);
            
            // If we've reached the batch size or it's the last item, create a batch
            if current_batch.len() == params.batch_size || index == total_items - 1 {
                batches.push(json!({
                    "items": current_batch,
                    "batch_number": batches.len(),
                    "batch_size": current_batch.len(),
                    "total_batches": (total_items + params.batch_size - 1) / params.batch_size
                }));
                current_batch = Vec::new();
            }
        }

        Ok(Value::Array(batches))
    }

    fn split_out_fields(&self, params: &SplitParams, input_data: &[Value]) -> Result<Value, Error> {
        let field_to_split = params.field_to_split.as_ref()
            .ok_or_else(|| Error::Validation("field_to_split is required for fields mode".to_string()))?;

        let mut results = Vec::new();

        for item in input_data {
            if let Value::Array(items) = item {
                for single_item in items {
                    let split_items = self.split_single_item_fields(&params, single_item, field_to_split)?;
                    results.extend(split_items);
                }
            } else {
                let split_items = self.split_single_item_fields(&params, item, field_to_split)?;
                results.extend(split_items);
            }
        }

        Ok(Value::Array(results))
    }

    fn split_single_item_fields(&self, params: &SplitParams, item: &Value, field_to_split: &str) -> Result<Vec<Value>, Error> {
        let mut results = Vec::new();
        
        // Get the field value to split
        let field_value = self.get_field_value(field_to_split, item, params.disable_dot_notation)?;
        
        // Convert field value to array if it's not already
        let array_to_split = match field_value {
            Value::Array(arr) => arr,
            Value::Object(obj) => obj.values().cloned().collect(),
            Value::Null => Vec::new(),
            other => vec![other]
        };

        // Create new items for each element in the array
        for element in array_to_split {
            let mut new_item = match params.include.as_str() {
                "allOtherFields" => {
                    // Include all fields from original item except the split field
                    let mut item_copy = item.clone();
                    if let Value::Object(ref mut map) = item_copy {
                        map.remove(field_to_split);
                    }
                    item_copy
                }
                "selectedFields" => {
                    // Include only specified fields
                    let mut new_obj = json!({});
                    if let Some(fields_to_include) = &params.fields_to_include {
                        for field in fields_to_include {
                            let field_val = self.get_field_value(field, item, params.disable_dot_notation)?;
                            if let Value::Object(ref mut map) = new_obj {
                                map.insert(field.clone(), field_val);
                            }
                        }
                    }
                    new_obj
                }
                _ => json!({}) // "noOtherFields"
            };

            // Add the split element to the new item
            let destination_field = params.destination_field.as_ref()
                .map_or(field_to_split, |v| v.as_str());
            
            if let Value::Object(ref mut map) = new_item {
                map.insert(destination_field.to_string(), element);
            } else {
                new_item = json!({
                    destination_field: element
                });
            }

            results.push(new_item);
        }

        Ok(results)
    }

    fn split_items(&self, _params: &SplitParams, input_data: &[Value]) -> Result<Value, Error> {
        let mut results = Vec::new();

        // Simple item splitting - convert arrays to individual items
        for item in input_data {
            match item {
                Value::Array(items) => {
                    // Split array into individual items
                    results.extend(items.clone());
                }
                _ => {
                    // Keep single items as-is
                    results.push(item.clone());
                }
            }
        }

        // Add metadata about the split
        let results_with_meta: Vec<Value> = results.into_iter().enumerate().map(|(index, item)| {
            json!({
                "data": item,
                "index": index,
                "total_items": input_data.len()
            })
        }).collect();

        Ok(Value::Array(results_with_meta))
    }

    fn get_field_value(&self, field_path: &str, data: &Value, disable_dot_notation: bool) -> Result<Value, Error> {
        if disable_dot_notation {
            // Direct field access only
            if let Some(value) = data.get(field_path) {
                return Ok(value.clone());
            }
        } else {
            // Support dot notation like "user.name"
            let parts: Vec<&str> = field_path.split('.').collect();
            let mut current = data;

            for part in parts {
                if let Some(next) = current.get(part) {
                    current = next;
                } else {
                    return Ok(Value::Null);
                }
            }

            return Ok(current.clone());
        }

        Ok(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_split_node_creation() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "mode": "items"
            })),
        );

        assert_eq!(node.name(), "Test Split");
        assert!(node.validate());
    }

    #[test]
    fn test_split_validation() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "invalid_mode"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_split_in_batches() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "batches",
                "batch_size": 2
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"id": 1, "name": "item1"},
                {"id": 2, "name": "item2"},
                {"id": 3, "name": "item3"},
                {"id": 4, "name": "item4"},
                {"id": 5, "name": "item5"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(batches)) = result {
            assert_eq!(batches.len(), 3); // Should create 3 batches (2+2+1)
            assert_eq!(batches[0]["batch_size"], 2);
            assert_eq!(batches[1]["batch_size"], 2);
            assert_eq!(batches[2]["batch_size"], 1);
        }
    }

    #[test]
    fn test_split_out_fields() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "fields",
                "field_to_split": "tags",
                "include": "allOtherFields"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "name": "John",
                "age": 30,
                "tags": ["developer", "javascript", "rust"]
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // Should create 3 items (one per tag)
            assert_eq!(results[0]["name"], "John");
            assert_eq!(results[0]["age"], 30);
            assert_eq!(results[0]["tags"], "developer");
            assert_eq!(results[1]["tags"], "javascript");
            assert_eq!(results[2]["tags"], "rust");
        }
    }

    #[test]
    fn test_split_items() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "items"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "item1"},
                {"name": "item2"},
                {"name": "item3"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["data"]["name"], "item1");
            assert_eq!(results[1]["data"]["name"], "item2");
            assert_eq!(results[2]["data"]["name"], "item3");
            assert_eq!(results[0]["index"], 0);
            assert_eq!(results[1]["index"], 1);
            assert_eq!(results[2]["index"], 2);
        }
    }

    #[test]
    fn test_dot_notation_field_access() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "fields",
                "field_to_split": "user.hobbies",
                "include": "noOtherFields"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "user": {
                    "name": "Alice",
                    "hobbies": ["reading", "coding", "gaming"]
                },
                "other_field": "value"
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["user.hobbies"], "reading");
            assert_eq!(results[1]["user.hobbies"], "coding");
            assert_eq!(results[2]["user.hobbies"], "gaming");
        }
    }

    #[test]
    fn test_batch_validation() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "batches",
                "batch_size": 0  // Invalid batch size
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_field_split_validation() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "fields"
                // Missing field_to_split
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_selected_fields_include() {
        let node = SplitNode::new(
            uuid::Uuid::new_v4(),
            "Test Split".to_string(),
            "split".to_string(),
            None,
            Some(json!({
                "mode": "fields",
                "field_to_split": "items",
                "include": "selectedFields",
                "fields_to_include": ["name", "id"]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "name": "Test Data",
                "id": 123,
                "description": "Should not be included",
                "items": ["apple", "banana", "cherry"]
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            // Should include name and id, but not description
            assert_eq!(results[0]["name"], "Test Data");
            assert_eq!(results[0]["id"], 123);
            assert!(results[0]["description"].is_null());
            assert_eq!(results[0]["items"], "apple");
        }
    }
}