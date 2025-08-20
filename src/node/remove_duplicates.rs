use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveDuplicatesNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareField {
    pub field_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveDuplicatesParams {
    #[serde(default)]
    pub compare: String, // "allFields", "selectedFields", "allFieldsExcept"
    #[serde(default)]
    pub fields_to_compare: Vec<CompareField>,
    #[serde(default)]
    pub fields_to_exclude: Vec<CompareField>,
    #[serde(default = "default_false")]
    pub disable_dot_notation: bool,
}

fn default_false() -> bool { false }

impl Default for RemoveDuplicatesParams {
    fn default() -> Self {
        Self {
            compare: "allFields".to_string(),
            fields_to_compare: vec![],
            fields_to_exclude: vec![],
            disable_dot_notation: false,
        }
    }
}

impl INode for RemoveDuplicatesNode {
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
            serde_json::from_value::<RemoveDuplicatesParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            RemoveDuplicatesParams::default()
        };

        // Process input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        if input_data.is_empty() {
            return Ok(json!([]));
        }

        // Flatten all items into a single array
        let mut all_items = Vec::new();
        for item in input_data {
            if let Value::Array(items) = item {
                all_items.extend(items);
            } else {
                all_items.push(item);
            }
        }

        // Remove duplicates
        let unique_items = self.remove_duplicates(&params, all_items)?;

        Ok(Value::Array(unique_items))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<RemoveDuplicatesParams>(params_value.clone()) {
                match params.compare.as_str() {
                    "allFields" | "allFieldsExcept" => true,
                    "selectedFields" => !params.fields_to_compare.is_empty(),
                    _ => false
                }
            } else {
                false
            }
        } else {
            true // Default is valid
        }
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl RemoveDuplicatesNode {
    fn remove_duplicates(&self, params: &RemoveDuplicatesParams, items: Vec<Value>) -> Result<Vec<Value>, Error> {
        let mut seen_keys = HashSet::new();
        let mut unique_items = Vec::new();

        for item in items {
            let comparison_key = self.generate_comparison_key(params, &item)?;
            
            if !seen_keys.contains(&comparison_key) {
                seen_keys.insert(comparison_key);
                unique_items.push(item);
            }
        }

        Ok(unique_items)
    }

    fn generate_comparison_key(&self, params: &RemoveDuplicatesParams, item: &Value) -> Result<String, Error> {
        match params.compare.as_str() {
            "allFields" => {
                // Use the entire item as comparison key
                Ok(item.to_string())
            },
            "selectedFields" => {
                // Only compare specified fields - create a sorted vector to ensure deterministic comparison
                let mut comparison_fields = Vec::new();
                for field in &params.fields_to_compare {
                    let field_value = self.get_field_value(&field.field_name, item, params.disable_dot_notation);
                    comparison_fields.push((field.field_name.clone(), field_value));
                }
                // Sort by field name to ensure deterministic ordering
                comparison_fields.sort_by_key(|(name, _)| name.clone());
                Ok(serde_json::to_string(&comparison_fields)
                    .map_err(|e| Error::Validation(format!("Failed to serialize comparison data: {}", e)))?)
            },
            "allFieldsExcept" => {
                // Compare all fields except specified ones
                let mut item_copy = item.clone();
                for field in &params.fields_to_exclude {
                    self.remove_field_from_value(&mut item_copy, &field.field_name, params.disable_dot_notation)?;
                }
                Ok(item_copy.to_string())
            },
            _ => {
                Err(Error::Validation(format!("Unsupported compare mode: {}", params.compare)))
            }
        }
    }

    fn get_field_value(&self, field_path: &str, data: &Value, disable_dot_notation: bool) -> Value {
        if disable_dot_notation {
            // Direct field access only
            data.get(field_path).cloned().unwrap_or(Value::Null)
        } else {
            // Support dot notation like "user.name"
            let parts: Vec<&str> = field_path.split('.').collect();
            let mut current = data;

            for part in parts {
                match current.get(part) {
                    Some(next) => current = next,
                    None => return Value::Null,
                }
            }

            current.clone()
        }
    }

    fn remove_field_from_value(&self, value: &mut Value, field_path: &str, disable_dot_notation: bool) -> Result<(), Error> {
        if disable_dot_notation {
            // Direct field removal only
            if let Value::Object(obj) = value {
                obj.remove(field_path);
            }
            return Ok(());
        }

        // Support dot notation removal
        let parts: Vec<&str> = field_path.split('.').collect();
        if parts.is_empty() {
            return Ok(());
        }

        let mut current = value;
        
        // Navigate to the parent of the target field
        for part in &parts[..parts.len()-1] {
            match current {
                Value::Object(obj) => {
                    current = obj.get_mut(*part).ok_or_else(|| 
                        Error::Validation(format!("Field path not found: {}", field_path)))?;
                },
                _ => {
                    return Err(Error::Validation("Cannot navigate to non-object field".to_string()));
                }
            }
        }

        // Remove the final field
        let final_field = parts[parts.len()-1];
        if let Value::Object(obj) = current {
            obj.remove(final_field);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_remove_duplicates_node_creation() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "compare": "allFields"
            })),
        );

        assert_eq!(node.name(), "Test Remove Duplicates");
        assert!(node.validate());
    }

    #[test]
    fn test_remove_duplicates_validation() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "selectedFields",
                "fields_to_compare": []
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_remove_duplicates_all_fields() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "allFields"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "John", "age": 30},
                {"name": "Jane", "age": 25},
                {"name": "John", "age": 30}, // Duplicate
                {"name": "Bob", "age": 35}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // Should remove one duplicate
            
            // Verify that we have unique items
            let mut names = HashSet::new();
            for item in &results {
                let key = format!("{}:{}", item["name"], item["age"]);
                names.insert(key);
            }
            assert_eq!(names.len(), 3);
        }
    }

    #[test]
    fn test_remove_duplicates_selected_fields() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "selectedFields",
                "fields_to_compare": [
                    {"field_name": "email"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "John", "email": "john@example.com", "age": 30},
                {"name": "Johnny", "email": "john@example.com", "age": 31}, // Same email, different other fields
                {"name": "Jane", "email": "jane@example.com", "age": 25}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2); // Should remove duplicate based on email only
            
            // Verify unique emails
            let mut emails = HashSet::new();
            for item in &results {
                emails.insert(item["email"].as_str().unwrap());
            }
            assert_eq!(emails.len(), 2);
            assert!(emails.contains("john@example.com"));
            assert!(emails.contains("jane@example.com"));
        }
    }

    #[test]
    fn test_remove_duplicates_except_fields() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "allFieldsExcept",
                "fields_to_exclude": [
                    {"field_name": "timestamp"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "John", "email": "john@example.com", "timestamp": "2023-01-01"},
                {"name": "John", "email": "john@example.com", "timestamp": "2023-01-02"}, // Same except timestamp
                {"name": "Jane", "email": "jane@example.com", "timestamp": "2023-01-01"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2); // Should remove duplicate when ignoring timestamp
        }
    }

    #[test]
    fn test_remove_duplicates_nested_fields() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "selectedFields",
                "fields_to_compare": [
                    {"field_name": "user.id"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"user": {"id": 1, "name": "John"}, "status": "active"},
                {"user": {"id": 1, "name": "Johnny"}, "status": "inactive"}, // Same user.id
                {"user": {"id": 2, "name": "Jane"}, "status": "active"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2); // Should remove duplicate based on user.id
        }
    }

    #[test]
    fn test_remove_duplicates_no_duplicates() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "allFields"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "John", "age": 30},
                {"name": "Jane", "age": 25},
                {"name": "Bob", "age": 35}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // Should keep all items as no duplicates
        }
    }

    #[test]
    fn test_remove_duplicates_empty_input() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "allFields"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 0);
        }
    }

    #[test]
    fn test_remove_duplicates_multiple_fields() {
        let node = RemoveDuplicatesNode::new(
            uuid::Uuid::new_v4(),
            "Test Remove Duplicates".to_string(),
            "removeDuplicates".to_string(),
            None,
            Some(json!({
                "compare": "selectedFields",
                "fields_to_compare": [
                    {"field_name": "name"},
                    {"field_name": "department"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "John", "department": "Engineering", "salary": 80000},
                {"name": "John", "department": "Engineering", "salary": 85000}, // Same name+dept, different salary
                {"name": "John", "department": "Marketing", "salary": 75000}, // Same name, different dept
                {"name": "Jane", "department": "Engineering", "salary": 82000}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // Should keep 3 unique name+department combinations
        }
    }
}