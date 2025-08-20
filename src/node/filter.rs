use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterParams {
    pub conditions: Vec<FilterCondition>,
    pub combine: String, // "AND" or "OR"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub field: String,
    pub operator: String, // "equals", "notEquals", "contains", "greaterThan", "lessThan"
    pub value: Value,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            conditions: vec![],
            combine: "AND".to_string(),
        }
    }
}

impl INode for FilterNode {
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
            serde_json::from_value::<FilterParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            FilterParams::default()
        };

        // Combine all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        // If no input data, return empty array
        if input_data.is_empty() {
            return Ok(json!([]));
        }

        // Process each input item
        let mut filtered_results = Vec::new();
        
        for item in input_data {
            if let Value::Array(items) = item {
                // If input is an array, filter each item
                for single_item in items {
                    if self.passes_filter(&params, &single_item)? {
                        filtered_results.push(single_item);
                    }
                }
            } else {
                // If input is a single item, filter it
                if self.passes_filter(&params, &item)? {
                    filtered_results.push(item);
                }
            }
        }

        Ok(Value::Array(filtered_results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<FilterParams>(params_value.clone()) {
                return !params.conditions.is_empty();
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl FilterNode {
    fn passes_filter(&self, params: &FilterParams, item: &Value) -> Result<bool, Error> {
        // If no conditions, let everything pass
        if params.conditions.is_empty() {
            return Ok(true);
        }

        let results: Result<Vec<bool>, Error> = params.conditions
            .iter()
            .map(|condition| self.evaluate_condition(condition, item))
            .collect();

        let condition_results = results?;

        // Combine results based on combine logic
        match params.combine.to_uppercase().as_str() {
            "AND" => Ok(condition_results.iter().all(|&result| result)),
            "OR" => Ok(condition_results.iter().any(|&result| result)),
            _ => Err(Error::Validation(format!("Unsupported combine logic: {}", params.combine)))
        }
    }

    fn evaluate_condition(&self, condition: &FilterCondition, item: &Value) -> Result<bool, Error> {
        // Get the field value from the item
        let field_value = self.get_field_value(&condition.field, item)?;

        // Evaluate based on operator
        match condition.operator.as_str() {
            "equals" => Ok(field_value == condition.value),
            "notEquals" => Ok(field_value != condition.value),
            "contains" => {
                if let (Value::String(haystack), Value::String(needle)) = (&field_value, &condition.value) {
                    Ok(haystack.contains(needle))
                } else {
                    Ok(false)
                }
            }
            "greaterThan" => {
                match (&field_value, &condition.value) {
                    (Value::Number(a), Value::Number(b)) => {
                        if let (Some(a_f64), Some(b_f64)) = (a.as_f64(), b.as_f64()) {
                            Ok(a_f64 > b_f64)
                        } else {
                            Ok(false)
                        }
                    }
                    _ => Ok(false)
                }
            }
            "lessThan" => {
                match (&field_value, &condition.value) {
                    (Value::Number(a), Value::Number(b)) => {
                        if let (Some(a_f64), Some(b_f64)) = (a.as_f64(), b.as_f64()) {
                            Ok(a_f64 < b_f64)
                        } else {
                            Ok(false)
                        }
                    }
                    _ => Ok(false)
                }
            }
            _ => Err(Error::Validation(format!("Unsupported operator: {}", condition.operator)))
        }
    }

    fn get_field_value(&self, field_path: &str, data: &Value) -> Result<Value, Error> {
        // Simple field access - supports dot notation like "user.name"
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_filter_node_creation() {
        let node = FilterNode::new(
            uuid::Uuid::new_v4(),
            "Test Filter".to_string(),
            "filter".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "conditions": [{
                    "field": "status",
                    "operator": "equals",
                    "value": "active"
                }],
                "combine": "AND"
            })),
        );

        assert_eq!(node.name(), "Test Filter");
        assert!(node.validate());
    }

    #[test]
    fn test_filter_validation() {
        let node = FilterNode::new(
            uuid::Uuid::new_v4(),
            "Test Filter".to_string(),
            "filter".to_string(),
            None,
            Some(json!({
                "conditions": [],
                "combine": "AND"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_filter_execution() {
        let node = FilterNode::new(
            uuid::Uuid::new_v4(),
            "Test Filter".to_string(),
            "filter".to_string(),
            None,
            Some(json!({
                "conditions": [{
                    "field": "status",
                    "operator": "equals",
                    "value": "active"
                }],
                "combine": "AND"
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!([
            {"status": "active", "name": "item1"},
            {"status": "inactive", "name": "item2"},
            {"status": "active", "name": "item3"}
        ]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2); // Only active items should pass
            assert_eq!(results[0]["name"], "item1");
            assert_eq!(results[1]["name"], "item3");
        }
    }
}