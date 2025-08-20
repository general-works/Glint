use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeParams {
    pub mode: String, // "append", "merge_by_key", "zip", "combine_all", "wait"
    pub merge_key: Option<String>,
    pub wait_for_all: Option<bool>,
    pub output_format: Option<String>, // "array", "object", "flat"
}

impl Default for MergeParams {
    fn default() -> Self {
        Self {
            mode: "append".to_string(),
            merge_key: None,
            wait_for_all: Some(true),
            output_format: Some("array".to_string()),
        }
    }
}

impl INode for MergeNode {
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
            serde_json::from_value::<MergeParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            MergeParams::default()
        };

        // Collect all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        // If no input data, return empty based on output format
        if input_data.is_empty() {
            return Ok(self.create_empty_output(&params));
        }

        match params.mode.as_str() {
            "append" => self.merge_append(&params, input_data),
            "merge_by_key" => self.merge_by_key(&params, input_data),
            "zip" => self.merge_zip(&params, input_data),
            "combine_all" => self.merge_combine_all(&params, input_data),
            "wait" => self.merge_wait(&params, input_data),
            _ => Err(Error::Validation(format!("Unsupported merge mode: {}", params.mode)))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<MergeParams>(params_value.clone()) {
                match params.mode.as_str() {
                    "merge_by_key" => params.merge_key.is_some(),
                    "append" | "zip" | "combine_all" | "wait" => true,
                    _ => false
                }
            } else {
                false
            }
        } else {
            true // Default params are valid
        }
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl MergeNode {
    fn merge_append(&self, params: &MergeParams, input_data: Vec<Value>) -> Result<Value, Error> {
        let mut result = Vec::new();
        
        for input in input_data {
            match input {
                Value::Array(items) => {
                    result.extend(items);
                }
                _ => {
                    result.push(input);
                }
            }
        }
        
        self.format_output(params, Value::Array(result))
    }

    fn merge_by_key(&self, params: &MergeParams, input_data: Vec<Value>) -> Result<Value, Error> {
        let merge_key = params.merge_key.as_ref()
            .ok_or_else(|| Error::Validation("merge_key is required for merge_by_key mode".to_string()))?;
        
        let mut merged_data = std::collections::HashMap::new();
        
        for input in input_data {
            let items = match input {
                Value::Array(items) => items,
                _ => vec![input],
            };
            
            for item in items {
                if let Some(key_value) = item.get(merge_key) {
                    let key = self.value_to_string(key_value);
                    
                    if let Some(existing) = merged_data.get(&key) {
                        // Merge objects if both are objects
                        if let (Value::Object(existing_obj), Value::Object(new_obj)) = (existing, &item) {
                            let mut merged_obj = existing_obj.clone();
                            for (k, v) in new_obj {
                                merged_obj.insert(k.clone(), v.clone());
                            }
                            merged_data.insert(key, Value::Object(merged_obj));
                        } else {
                            // Otherwise, replace with new value
                            merged_data.insert(key, item);
                        }
                    } else {
                        merged_data.insert(key, item);
                    }
                }
            }
        }
        
        let result: Vec<Value> = merged_data.into_values().collect();
        self.format_output(params, Value::Array(result))
    }

    fn merge_zip(&self, params: &MergeParams, input_data: Vec<Value>) -> Result<Value, Error> {
        // Convert all inputs to arrays
        let arrays: Vec<Vec<Value>> = input_data.into_iter().map(|input| {
            match input {
                Value::Array(items) => items,
                _ => vec![input],
            }
        }).collect();
        
        if arrays.is_empty() {
            return self.format_output(params, Value::Array(vec![]));
        }
        
        // Find the maximum length
        let max_len = arrays.iter().map(|arr| arr.len()).max().unwrap_or(0);
        
        let mut result = Vec::new();
        for i in 0..max_len {
            let mut zipped_item = serde_json::Map::new();
            
            for (input_index, array) in arrays.iter().enumerate() {
                let value = array.get(i).cloned().unwrap_or(Value::Null);
                zipped_item.insert(format!("input_{}", input_index), value);
            }
            
            result.push(Value::Object(zipped_item));
        }
        
        self.format_output(params, Value::Array(result))
    }

    fn merge_combine_all(&self, params: &MergeParams, input_data: Vec<Value>) -> Result<Value, Error> {
        let mut combined = serde_json::Map::new();
        
        for (index, input) in input_data.into_iter().enumerate() {
            combined.insert(format!("input_{}", index), input);
        }
        
        self.format_output(params, Value::Object(combined))
    }

    fn merge_wait(&self, params: &MergeParams, input_data: Vec<Value>) -> Result<Value, Error> {
        // Wait mode simply passes through all inputs as received
        // In a real implementation, this would wait for all inputs before proceeding
        let wait_for_all = params.wait_for_all.unwrap_or(true);
        
        if wait_for_all && input_data.len() < 2 {
            // In a real implementation, this would wait for more inputs
            // For now, we'll just process what we have
        }
        
        let mut result = Vec::new();
        
        for input in input_data {
            match input {
                Value::Array(items) => {
                    result.extend(items);
                }
                _ => {
                    result.push(input);
                }
            }
        }
        
        self.format_output(params, Value::Array(result))
    }

    fn format_output(&self, params: &MergeParams, data: Value) -> Result<Value, Error> {
        let format = params.output_format.as_ref().map(|s| s.as_str()).unwrap_or("array");
        
        match format {
            "array" => {
                match data {
                    Value::Array(_) => Ok(data),
                    _ => Ok(json!([data])),
                }
            }
            "object" => {
                match data {
                    Value::Object(_) => Ok(data),
                    Value::Array(items) => {
                        let mut obj = serde_json::Map::new();
                        for (i, item) in items.into_iter().enumerate() {
                            obj.insert(i.to_string(), item);
                        }
                        Ok(Value::Object(obj))
                    }
                    _ => {
                        let mut obj = serde_json::Map::new();
                        obj.insert("data".to_string(), data);
                        Ok(Value::Object(obj))
                    }
                }
            }
            "flat" => {
                match data {
                    Value::Array(items) => {
                        let mut flat = Vec::new();
                        for item in items {
                            if let Value::Array(nested) = item {
                                flat.extend(nested);
                            } else {
                                flat.push(item);
                            }
                        }
                        Ok(Value::Array(flat))
                    }
                    _ => Ok(data),
                }
            }
            _ => Err(Error::Validation(format!("Unsupported output format: {}", format)))
        }
    }

    fn create_empty_output(&self, params: &MergeParams) -> Value {
        let format = params.output_format.as_ref().map(|s| s.as_str()).unwrap_or("array");
        
        match format {
            "array" => json!([]),
            "object" => json!({}),
            "flat" => json!([]),
            _ => json!([]),
        }
    }

    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => serde_json::to_string(value).unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_merge_node_creation() {
        let node = MergeNode::new(
            Uuid::new_v4(),
            "Test Merge".to_string(),
            "merge".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "mode": "append",
                "output_format": "array"
            })),
        );

        assert_eq!(node.name(), "Test Merge");
        assert!(node.validate());
    }

    #[test]
    fn test_merge_validation_merge_by_key() {
        let node = MergeNode::new(
            Uuid::new_v4(),
            "Test Merge".to_string(),
            "merge".to_string(),
            None,
            Some(json!({
                "mode": "merge_by_key"
                // Missing merge_key
            })),
        );

        assert!(!node.validate());
        
        let valid_node = MergeNode::new(
            Uuid::new_v4(),
            "Test Merge".to_string(),
            "merge".to_string(),
            None,
            Some(json!({
                "mode": "merge_by_key",
                "merge_key": "id"
            })),
        );

        assert!(valid_node.validate());
    }

    #[test]
    fn test_merge_append() {
        let node = MergeNode::new(
            Uuid::new_v4(),
            "Test Merge".to_string(),
            "merge".to_string(),
            None,
            Some(json!({
                "mode": "append",
                "output_format": "array"
            })),
        );

        let mut input = HashMap::new();
        input.insert(Uuid::new_v4(), json!([{"a": 1}, {"a": 2}]));
        input.insert(Uuid::new_v4(), json!([{"b": 3}, {"b": 4}]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(items)) = result {
            assert_eq!(items.len(), 4);
            // Items from both inputs should be present
            assert!(items.iter().any(|item| item.get("a").is_some()));
            assert!(items.iter().any(|item| item.get("b").is_some()));
        }
    }

    #[test]
    fn test_merge_by_key() {
        let node = MergeNode::new(
            Uuid::new_v4(),
            "Test Merge".to_string(),
            "merge".to_string(),
            None,
            Some(json!({
                "mode": "merge_by_key",
                "merge_key": "id",
                "output_format": "array"
            })),
        );

        let mut input = HashMap::new();
        input.insert(Uuid::new_v4(), json!([
            {"id": "1", "name": "item1"},
            {"id": "2", "name": "item2"}
        ]));
        input.insert(Uuid::new_v4(), json!([
            {"id": "1", "value": 100},
            {"id": "3", "name": "item3"}
        ]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(items)) = result {
            assert_eq!(items.len(), 3); // Three unique IDs: 1, 2, 3
            
            // Find item with id "1" - should have both name and value
            let item1 = items.iter().find(|item| item["id"] == "1").unwrap();
            assert_eq!(item1["name"], "item1");
            assert_eq!(item1["value"], 100);
        }
    }

    #[test]
    fn test_merge_zip() {
        let node = MergeNode::new(
            Uuid::new_v4(),
            "Test Merge".to_string(),
            "merge".to_string(),
            None,
            Some(json!({
                "mode": "zip",
                "output_format": "array"
            })),
        );

        let mut input = HashMap::new();
        input.insert(Uuid::new_v4(), json!(["a", "b"]));
        input.insert(Uuid::new_v4(), json!([1, 2]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(items)) = result {
            assert_eq!(items.len(), 2);
            
            // First zipped item should combine first elements
            assert!(items[0]["input_0"] == "a" || items[0]["input_1"] == "a");
            assert!(items[0]["input_0"] == 1 || items[0]["input_1"] == 1);
            
            // Second zipped item should combine second elements  
            assert!(items[1]["input_0"] == "b" || items[1]["input_1"] == "b");
            assert!(items[1]["input_0"] == 2 || items[1]["input_1"] == 2);
        }
    }
}