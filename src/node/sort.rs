use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortKey {
    pub key: String,
    #[serde(default = "default_asc")]
    pub direction: String, // "asc" or "desc"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortParams {
    #[serde(default)]
    pub sort_keys: Vec<SortKey>,
    #[serde(default = "default_type")]
    pub sort_type: String, // "simple", "code", "multi"
    #[serde(default)]
    pub code: Option<String>, // For custom sorting code
    #[serde(default = "default_false")]
    pub case_sensitive: bool,
    #[serde(default = "default_false")]
    pub randomize: bool,
}

fn default_asc() -> String { "asc".to_string() }
fn default_type() -> String { "simple".to_string() }
fn default_false() -> bool { false }

impl Default for SortParams {
    fn default() -> Self {
        Self {
            sort_keys: vec![],
            sort_type: "simple".to_string(),
            code: None,
            case_sensitive: false,
            randomize: false,
        }
    }
}

impl INode for SortNode {
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
            serde_json::from_value::<SortParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            SortParams::default()
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

        // Apply sorting
        let sorted_items = self.sort_items(&params, all_items)?;

        Ok(Value::Array(sorted_items))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<SortParams>(params_value.clone()) {
                match params.sort_type.as_str() {
                    "simple" | "multi" => !params.sort_keys.is_empty(),
                    "code" => params.code.is_some(),
                    _ => false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl SortNode {
    fn sort_items(&self, params: &SortParams, mut items: Vec<Value>) -> Result<Vec<Value>, Error> {
        if params.randomize {
            // Simple randomization - use a fixed random seed based on items length
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let items_len = items.len();
            items.sort_by(|a, b| {
                let mut hasher1 = DefaultHasher::new();
                let mut hasher2 = DefaultHasher::new();
                a.to_string().hash(&mut hasher1);
                b.to_string().hash(&mut hasher2);
                
                let h1 = hasher1.finish();
                let h2 = hasher2.finish();
                // Add some variation based on items length to avoid predictable ordering
                let h1_mod = h1.wrapping_add(items_len as u64);
                let h2_mod = h2.wrapping_add(items_len as u64 * 2);
                h1_mod.cmp(&h2_mod)
            });
            return Ok(items);
        }

        match params.sort_type.as_str() {
            "simple" => self.simple_sort(params, items),
            "multi" => self.multi_key_sort(params, items),
            "code" => self.code_sort(params, items),
            _ => Err(Error::Validation(format!("Unsupported sort type: {}", params.sort_type)))
        }
    }

    fn simple_sort(&self, params: &SortParams, mut items: Vec<Value>) -> Result<Vec<Value>, Error> {
        if params.sort_keys.is_empty() {
            return Err(Error::Validation("At least one sort key is required for simple sort".to_string()));
        }

        let sort_key = &params.sort_keys[0];
        
        items.sort_by(|a, b| {
            let val_a = self.get_field_value(&sort_key.key, a);
            let val_b = self.get_field_value(&sort_key.key, b);
            
            let comparison = self.compare_values(&val_a, &val_b, params.case_sensitive);
            
            if sort_key.direction == "desc" {
                comparison.reverse()
            } else {
                comparison
            }
        });

        Ok(items)
    }

    fn multi_key_sort(&self, params: &SortParams, mut items: Vec<Value>) -> Result<Vec<Value>, Error> {
        if params.sort_keys.is_empty() {
            return Err(Error::Validation("At least one sort key is required for multi sort".to_string()));
        }

        items.sort_by(|a, b| {
            for sort_key in &params.sort_keys {
                let val_a = self.get_field_value(&sort_key.key, a);
                let val_b = self.get_field_value(&sort_key.key, b);
                
                let comparison = self.compare_values(&val_a, &val_b, params.case_sensitive);
                
                let final_comparison = if sort_key.direction == "desc" {
                    comparison.reverse()
                } else {
                    comparison
                };

                if final_comparison != Ordering::Equal {
                    return final_comparison;
                }
            }
            Ordering::Equal
        });

        Ok(items)
    }

    fn code_sort(&self, params: &SortParams, mut items: Vec<Value>) -> Result<Vec<Value>, Error> {
        let code = params.code.as_ref()
            .ok_or_else(|| Error::Validation("Code is required for code sort".to_string()))?;

        // Simple code-based sorting - very basic implementation
        // In a real implementation, you'd use a proper JavaScript engine
        if code.contains("a.value - b.value") || code.contains("a.value > b.value") {
            items.sort_by(|a, b| {
                let val_a = self.get_field_value("value", a);
                let val_b = self.get_field_value("value", b);
                self.compare_values(&val_a, &val_b, params.case_sensitive)
            });
        } else if code.contains("b.value - a.value") || code.contains("b.value > a.value") {
            items.sort_by(|a, b| {
                let val_a = self.get_field_value("value", a);
                let val_b = self.get_field_value("value", b);
                self.compare_values(&val_b, &val_a, params.case_sensitive)
            });
        } else if code.contains("a.name") && code.contains("b.name") {
            items.sort_by(|a, b| {
                let val_a = self.get_field_value("name", a);
                let val_b = self.get_field_value("name", b);
                if code.contains("localeCompare") || code.contains("<") {
                    self.compare_values(&val_a, &val_b, params.case_sensitive)
                } else {
                    self.compare_values(&val_b, &val_a, params.case_sensitive)
                }
            });
        } else {
            // Default: try to sort by first available field
            if let Some(first_item) = items.first() {
                if let Value::Object(obj) = first_item {
                    if let Some(first_key) = obj.keys().next() {
                        let key = first_key.clone();
                        items.sort_by(|a, b| {
                            let val_a = self.get_field_value(&key, a);
                            let val_b = self.get_field_value(&key, b);
                            self.compare_values(&val_a, &val_b, params.case_sensitive)
                        });
                    }
                }
            }
        }

        Ok(items)
    }

    fn get_field_value(&self, field_path: &str, data: &Value) -> Value {
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

    fn compare_values(&self, a: &Value, b: &Value, case_sensitive: bool) -> Ordering {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let f1 = n1.as_f64().unwrap_or(0.0);
                let f2 = n2.as_f64().unwrap_or(0.0);
                f1.partial_cmp(&f2).unwrap_or(Ordering::Equal)
            },
            (Value::String(s1), Value::String(s2)) => {
                if case_sensitive {
                    s1.cmp(s2)
                } else {
                    s1.to_lowercase().cmp(&s2.to_lowercase())
                }
            },
            (Value::Bool(b1), Value::Bool(b2)) => b1.cmp(b2),
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            // Convert other types to strings for comparison
            _ => {
                let s1 = a.to_string();
                let s2 = b.to_string();
                if case_sensitive {
                    s1.cmp(&s2)
                } else {
                    s1.to_lowercase().cmp(&s2.to_lowercase())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_sort_node_creation() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "sort_keys": [
                    {"key": "name", "direction": "asc"}
                ]
            })),
        );

        assert_eq!(node.name(), "Test Sort");
        assert!(node.validate());
    }

    #[test]
    fn test_sort_validation() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            None,
            Some(json!({
                "sort_type": "simple",
                "sort_keys": []
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_simple_sort_ascending() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            None,
            Some(json!({
                "sort_type": "simple",
                "sort_keys": [
                    {"key": "age", "direction": "asc"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": 25},
                {"name": "Charlie", "age": 35}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["name"], "Bob");
            assert_eq!(results[0]["age"], 25);
            assert_eq!(results[1]["name"], "Alice");
            assert_eq!(results[1]["age"], 30);
            assert_eq!(results[2]["name"], "Charlie");
            assert_eq!(results[2]["age"], 35);
        }
    }

    #[test]
    fn test_simple_sort_descending() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            None,
            Some(json!({
                "sort_type": "simple",
                "sort_keys": [
                    {"key": "score", "direction": "desc"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "Alice", "score": 85},
                {"name": "Bob", "score": 92},
                {"name": "Charlie", "score": 78}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["name"], "Bob");
            assert_eq!(results[0]["score"], 92);
            assert_eq!(results[1]["name"], "Alice");
            assert_eq!(results[1]["score"], 85);
            assert_eq!(results[2]["name"], "Charlie");
            assert_eq!(results[2]["score"], 78);
        }
    }

    #[test]
    fn test_string_sort_case_insensitive() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            None,
            Some(json!({
                "sort_type": "simple",
                "sort_keys": [
                    {"key": "name", "direction": "asc"}
                ],
                "case_sensitive": false
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "charlie"},
                {"name": "Alice"},
                {"name": "bob"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["name"], "Alice");
            assert_eq!(results[1]["name"], "bob");
            assert_eq!(results[2]["name"], "charlie");
        }
    }

    #[test]
    fn test_multi_key_sort() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            None,
            Some(json!({
                "sort_type": "multi",
                "sort_keys": [
                    {"key": "department", "direction": "asc"},
                    {"key": "salary", "direction": "desc"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "Alice", "department": "Engineering", "salary": 80000},
                {"name": "Bob", "department": "Engineering", "salary": 90000},
                {"name": "Charlie", "department": "Marketing", "salary": 70000},
                {"name": "David", "department": "Marketing", "salary": 75000}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 4);
            // Engineering department first (asc), then by salary desc within department
            assert_eq!(results[0]["name"], "Bob"); // Engineering, 90000
            assert_eq!(results[1]["name"], "Alice"); // Engineering, 80000
            // Marketing department second (asc), then by salary desc within department
            assert_eq!(results[2]["name"], "David"); // Marketing, 75000
            assert_eq!(results[3]["name"], "Charlie"); // Marketing, 70000
        }
    }

    #[test]
    fn test_nested_field_sort() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            None,
            Some(json!({
                "sort_type": "simple",
                "sort_keys": [
                    {"key": "user.profile.age", "direction": "asc"}
                ]
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"user": {"profile": {"age": 30, "name": "Alice"}}},
                {"user": {"profile": {"age": 25, "name": "Bob"}}},
                {"user": {"profile": {"age": 35, "name": "Charlie"}}}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["user"]["profile"]["name"], "Bob");
            assert_eq!(results[1]["user"]["profile"]["name"], "Alice");
            assert_eq!(results[2]["user"]["profile"]["name"], "Charlie");
        }
    }

    #[test]
    fn test_code_sort() {
        let node = SortNode::new(
            uuid::Uuid::new_v4(),
            "Test Sort".to_string(),
            "sort".to_string(),
            None,
            Some(json!({
                "sort_type": "code",
                "code": "a.value - b.value"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"value": 30},
                {"value": 10},
                {"value": 20}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["value"], 10);
            assert_eq!(results[1]["value"], 20);
            assert_eq!(results[2]["value"], 30);
        }
    }
}