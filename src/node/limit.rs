use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_false")]
    pub keep_only_set: bool,
    #[serde(default = "default_false")]
    pub skip_first: bool,
    #[serde(default)]
    pub skip_count: Option<usize>,
}

fn default_limit() -> usize { 1 }
fn default_false() -> bool { false }

impl Default for LimitParams {
    fn default() -> Self {
        Self {
            limit: 1,
            keep_only_set: false,
            skip_first: false,
            skip_count: None,
        }
    }
}

impl INode for LimitNode {
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
            serde_json::from_value::<LimitParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            LimitParams::default()
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

        // Apply limiting logic
        let limited_items = self.apply_limit(&params, all_items)?;

        Ok(Value::Array(limited_items))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<LimitParams>(params_value.clone()) {
                return params.limit > 0;
            }
        }
        true // Default is valid
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl LimitNode {
    fn apply_limit(&self, params: &LimitParams, mut items: Vec<Value>) -> Result<Vec<Value>, Error> {
        let total_items = items.len();
        
        // Calculate skip count
        let skip_count = if params.skip_first {
            params.skip_count.unwrap_or(1)
        } else {
            0
        };

        // Skip items if needed
        if skip_count > 0 {
            if skip_count >= total_items {
                return Ok(vec![]);
            }
            items = items.into_iter().skip(skip_count).collect();
        }

        // Apply limit
        if params.keep_only_set {
            // Keep only the items within the limit range
            items.truncate(params.limit);
        } else {
            // Take the first N items
            items.truncate(params.limit);
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_limit_node_creation() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "limit": 3
            })),
        );

        assert_eq!(node.name(), "Test Limit");
        assert!(node.validate());
    }

    #[test]
    fn test_limit_validation() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 0
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_basic_limit() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 2
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"},
                {"id": 3, "name": "third"},
                {"id": 4, "name": "fourth"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2);
            assert_eq!(results[0]["id"], 1);
            assert_eq!(results[1]["id"], 2);
        }
    }

    #[test]
    fn test_limit_with_skip() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 2,
                "skip_first": true,
                "skip_count": 1
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"},
                {"id": 3, "name": "third"},
                {"id": 4, "name": "fourth"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2);
            assert_eq!(results[0]["id"], 2); // Skipped first item
            assert_eq!(results[1]["id"], 3);
        }
    }

    #[test]
    fn test_limit_larger_than_items() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 10
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2); // Should return all items if limit is larger
            assert_eq!(results[0]["id"], 1);
            assert_eq!(results[1]["id"], 2);
        }
    }

    #[test]
    fn test_skip_all_items() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 2,
                "skip_first": true,
                "skip_count": 5
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 0); // Should return empty array if skip count >= total items
        }
    }

    #[test]
    fn test_multiple_input_arrays() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 3
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"id": 1, "source": "A"},
                {"id": 2, "source": "A"}
            ])
        );
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"id": 3, "source": "B"},
                {"id": 4, "source": "B"}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // Should take first 3 items from combined arrays
        }
    }

    #[test]
    fn test_single_item_input() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 1
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"id": 1, "name": "single"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["id"], 1);
        }
    }

    #[test]
    fn test_empty_input() {
        let node = LimitNode::new(
            uuid::Uuid::new_v4(),
            "Test Limit".to_string(),
            "limit".to_string(),
            None,
            Some(json!({
                "limit": 5
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 0);
        }
    }
}