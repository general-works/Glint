use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameKeysNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameKeyRule {
    pub current_key: String,
    pub new_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexReplacement {
    pub search_regex: String,
    pub replace_regex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameKeysParams {
    #[serde(default)]
    pub keys: Vec<RenameKeyRule>,
    #[serde(default)]
    pub regex_replacements: Vec<RegexReplacement>,
    // Options
    #[serde(default = "default_false")]
    pub delete_original_key: bool,
    #[serde(default = "default_false")]
    pub regex_case_insensitive: bool,
    #[serde(default = "default_true")]
    pub include_binary: bool,
}

fn default_false() -> bool { false }
fn default_true() -> bool { true }

impl Default for RenameKeysParams {
    fn default() -> Self {
        Self {
            keys: vec![],
            regex_replacements: vec![],
            delete_original_key: false,
            regex_case_insensitive: false,
            include_binary: true,
        }
    }
}

impl INode for RenameKeysNode {
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
            serde_json::from_value::<RenameKeysParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            RenameKeysParams::default()
        };

        // Process input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        let mut results = Vec::new();

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
            let result_item = self.rename_keys_in_item(&params, item)?;
            results.push(result_item);
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<RenameKeysParams>(params_value.clone()) {
                // Check if we have at least one rename rule or regex replacement
                return !params.keys.is_empty() || !params.regex_replacements.is_empty();
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl RenameKeysNode {
    fn rename_keys_in_item(&self, params: &RenameKeysParams, mut item: Value) -> Result<Value, Error> {
        // Apply direct key renames first
        for rule in &params.keys {
            self.apply_key_rename(&mut item, &rule.current_key, &rule.new_key, params.delete_original_key)?;
        }

        // Apply regex replacements
        for regex_rule in &params.regex_replacements {
            self.apply_regex_rename(&mut item, &regex_rule.search_regex, &regex_rule.replace_regex, params)?;
        }

        Ok(item)
    }

    fn apply_key_rename(&self, item: &mut Value, current_key: &str, new_key: &str, delete_original: bool) -> Result<(), Error> {
        if current_key == new_key {
            return Ok(());
        }

        // Get the value from the current key path
        if let Some(value) = self.get_nested_value(item, current_key) {
            let value_copy = value.clone();
            
            // Set the value at the new key path
            self.set_nested_value(item, new_key, value_copy)?;
            
            // Optionally delete the original key
            if delete_original {
                self.delete_nested_key(item, current_key)?;
            }
        }

        Ok(())
    }

    fn apply_regex_rename(&self, item: &mut Value, search_pattern: &str, replace_pattern: &str, params: &RenameKeysParams) -> Result<(), Error> {
        let regex_flags = if params.regex_case_insensitive {
            format!("(?i){}", search_pattern)
        } else {
            search_pattern.to_string()
        };

        let regex = Regex::new(&regex_flags)
            .map_err(|e| Error::Validation(format!("Invalid regex pattern: {}", e)))?;

        // Collect all keys that match the regex
        let mut keys_to_rename = Vec::new();
        self.collect_matching_keys(item, &regex, "", &mut keys_to_rename);

        // Apply renames
        for (full_path, key_name) in keys_to_rename {
            // Use regex replace to transform the key name
            let new_name = regex.replace(&key_name, replace_pattern).to_string();
            if new_name != key_name {
                let new_path = if full_path == key_name {
                    // Top-level key rename
                    new_name
                } else {
                    // Nested key rename - replace the last part of the path
                    let parts: Vec<&str> = full_path.split('.').collect();
                    if parts.len() > 1 {
                        let parent_parts = &parts[..parts.len() - 1];
                        format!("{}.{}", parent_parts.join("."), new_name)
                    } else {
                        new_name
                    }
                };
                
                self.apply_key_rename(item, &full_path, &new_path, true)?;
            }
        }

        Ok(())
    }

    fn collect_matching_keys(&self, value: &Value, regex: &Regex, current_path: &str, matches: &mut Vec<(String, String)>) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    let full_path = if current_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", current_path, key)
                    };

                    if regex.is_match(key) {
                        matches.push((full_path.clone(), key.clone()));
                    }

                    // Recursively check nested objects
                    self.collect_matching_keys(val, regex, &full_path, matches);
                }
            },
            Value::Array(arr) => {
                for (index, val) in arr.iter().enumerate() {
                    let full_path = if current_path.is_empty() {
                        index.to_string()
                    } else {
                        format!("{}.{}", current_path, index)
                    };
                    self.collect_matching_keys(val, regex, &full_path, matches);
                }
            },
            _ => {}
        }
    }


    fn get_nested_value<'a>(&self, value: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            match current {
                Value::Object(obj) => {
                    current = obj.get(part)?;
                },
                Value::Array(arr) => {
                    let index: usize = part.parse().ok()?;
                    current = arr.get(index)?;
                },
                _ => return None,
            }
        }

        Some(current)
    }

    fn set_nested_value(&self, value: &mut Value, path: &str, new_value: Value) -> Result<(), Error> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err(Error::Validation("Empty path not allowed".to_string()));
        }

        let mut current = value;
        
        // Navigate to the parent of the target
        for part in &parts[..parts.len()-1] {
            match current {
                Value::Object(obj) => {
                    if !obj.contains_key(*part) {
                        obj.insert(part.to_string(), json!({}));
                    }
                    current = obj.get_mut(*part).unwrap();
                },
                Value::Array(arr) => {
                    let index: usize = part.parse()
                        .map_err(|_| Error::Validation(format!("Invalid array index: {}", part)))?;
                    
                    // Extend array if necessary
                    while arr.len() <= index {
                        arr.push(json!({}));
                    }
                    current = &mut arr[index];
                },
                _ => {
                    return Err(Error::Validation("Cannot set nested value on non-object/array".to_string()));
                }
            }
        }

        // Set the final value
        let final_key = parts[parts.len()-1];
        match current {
            Value::Object(obj) => {
                obj.insert(final_key.to_string(), new_value);
            },
            Value::Array(arr) => {
                let index: usize = final_key.parse()
                    .map_err(|_| Error::Validation(format!("Invalid array index: {}", final_key)))?;
                
                while arr.len() <= index {
                    arr.push(Value::Null);
                }
                arr[index] = new_value;
            },
            _ => {
                return Err(Error::Validation("Cannot set value on non-object/array".to_string()));
            }
        }

        Ok(())
    }

    fn delete_nested_key(&self, value: &mut Value, path: &str) -> Result<(), Error> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Ok(());
        }

        let mut current = value;
        
        // Navigate to the parent of the target
        for part in &parts[..parts.len()-1] {
            match current {
                Value::Object(obj) => {
                    current = obj.get_mut(*part).ok_or_else(|| 
                        Error::Validation(format!("Path not found: {}", path)))?;
                },
                Value::Array(arr) => {
                    let index: usize = part.parse()
                        .map_err(|_| Error::Validation(format!("Invalid array index: {}", part)))?;
                    current = arr.get_mut(index).ok_or_else(|| 
                        Error::Validation(format!("Array index out of bounds: {}", index)))?;
                },
                _ => {
                    return Err(Error::Validation("Cannot delete from non-object/array".to_string()));
                }
            }
        }

        // Delete the final key
        let final_key = parts[parts.len()-1];
        match current {
            Value::Object(obj) => {
                obj.remove(final_key);
            },
            Value::Array(arr) => {
                let index: usize = final_key.parse()
                    .map_err(|_| Error::Validation(format!("Invalid array index: {}", final_key)))?;
                if index < arr.len() {
                    arr.remove(index);
                }
            },
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_rename_keys_node_creation() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "keys": [
                    {"current_key": "oldName", "new_key": "newName"}
                ]
            })),
        );

        assert_eq!(node.name(), "Test Rename Keys");
        assert!(node.validate());
    }

    #[test]
    fn test_rename_keys_validation() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            None,
            Some(json!({
                "keys": []
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_simple_key_rename() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            None,
            Some(json!({
                "keys": [
                    {"current_key": "firstName", "new_key": "name"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"firstName": "John", "lastName": "Doe", "age": 30})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["name"], "John");
            assert_eq!(results[0]["firstName"], "John"); // Original key still exists unless delete_original_key is true
            assert_eq!(results[0]["lastName"], "Doe");
        }
    }

    #[test]
    fn test_nested_key_rename() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            None,
            Some(json!({
                "keys": [
                    {"current_key": "user.details.fullName", "new_key": "user.info.name"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "user": {
                    "details": {
                        "fullName": "John Doe",
                        "age": 30
                    }
                }
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["user"]["info"]["name"], "John Doe");
            assert_eq!(results[0]["user"]["details"]["age"], 30);
        }
    }

    #[test]
    fn test_key_rename_with_deletion() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            None,
            Some(json!({
                "keys": [
                    {"current_key": "oldField", "new_key": "newField"}
                ],
                "delete_original_key": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"oldField": "value", "otherField": "other"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["newField"], "value");
            assert!(results[0]["oldField"].is_null());
            assert_eq!(results[0]["otherField"], "other");
        }
    }

    #[test]
    fn test_regex_key_rename() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            None,
            Some(json!({
                "regex_replacements": [
                    {"search_regex": "^old(.*)", "replace_regex": "new$1"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"oldName": "John", "oldAge": 30, "status": "active"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["newName"], "John");
            assert_eq!(results[0]["newAge"], 30);
            assert_eq!(results[0]["status"], "active");
        }
    }

    #[test]
    fn test_multiple_key_renames() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            None,
            Some(json!({
                "keys": [
                    {"current_key": "firstName", "new_key": "first_name"},
                    {"current_key": "lastName", "new_key": "last_name"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"firstName": "John", "lastName": "Doe", "age": 30})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["first_name"], "John");
            assert_eq!(results[0]["last_name"], "Doe");
            assert_eq!(results[0]["age"], 30);
        }
    }

    #[test]
    fn test_case_insensitive_regex() {
        let node = RenameKeysNode::new(
            uuid::Uuid::new_v4(),
            "Test Rename Keys".to_string(),
            "renameKeys".to_string(),
            None,
            Some(json!({
                "regex_replacements": [
                    {"search_regex": "name", "replace_regex": "title"}
                ],
                "regex_case_insensitive": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"Name": "John", "USERNAME": "john_doe", "age": 30})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["title"], "John");
            assert_eq!(results[0]["USERtitle"], "john_doe");
            assert_eq!(results[0]["age"], 30);
        }
    }
}