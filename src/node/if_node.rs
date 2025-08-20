use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfParams {
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: String, // "equals", "notEquals", "contains", "greaterThan", "lessThan"
    pub value: Value,
}

impl Default for IfParams {
    fn default() -> Self {
        Self {
            conditions: vec![],
        }
    }
}

impl INode for IfNode {
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
            serde_json::from_value::<IfParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            IfParams::default()
        };

        // Combine all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        // If no input data, create an empty object
        let data_to_evaluate = if input_data.is_empty() {
            json!({})
        } else {
            input_data[0].clone() // Use first input for evaluation
        };

        // Evaluate conditions
        let condition_result = self.evaluate_conditions(&params.conditions, &data_to_evaluate)?;

        // Return result based on condition evaluation
        Ok(json!({
            "conditionMet": condition_result,
            "data": data_to_evaluate
        }))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<IfParams>(params_value.clone()) {
                return !params.conditions.is_empty();
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl IfNode {
    fn evaluate_conditions(&self, conditions: &[Condition], data: &Value) -> Result<bool, Error> {
        // If no conditions, return true (pass through)
        if conditions.is_empty() {
            return Ok(true);
        }

        // Evaluate all conditions (AND logic)
        for condition in conditions {
            if !self.evaluate_single_condition(condition, data)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn evaluate_single_condition(&self, condition: &Condition, data: &Value) -> Result<bool, Error> {
        // Get the field value from the data
        let field_value = self.get_field_value(&condition.field, data)?;

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
    fn test_if_node_creation() {
        let node = IfNode::new(
            uuid::Uuid::new_v4(),
            "Test If".to_string(),
            "if".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "conditions": [{
                    "field": "status",
                    "operator": "equals",
                    "value": "active"
                }]
            })),
        );

        assert_eq!(node.name(), "Test If");
        assert!(node.validate());
    }

    #[test]
    fn test_if_validation() {
        let node = IfNode::new(
            uuid::Uuid::new_v4(),
            "Test If".to_string(),
            "if".to_string(),
            None,
            Some(json!({
                "conditions": []
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_if_condition_evaluation() {
        let node = IfNode::new(
            uuid::Uuid::new_v4(),
            "Test If".to_string(),
            "if".to_string(),
            None,
            Some(json!({
                "conditions": [{
                    "field": "status",
                    "operator": "equals",
                    "value": "active"
                }]
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({"status": "active", "name": "test"}));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(result_value) = result {
            assert_eq!(result_value["conditionMet"], true);
        }
    }

    #[test]
    fn test_if_condition_false() {
        let node = IfNode::new(
            uuid::Uuid::new_v4(),
            "Test If".to_string(),
            "if".to_string(),
            None,
            Some(json!({
                "conditions": [{
                    "field": "status",
                    "operator": "equals",
                    "value": "inactive"
                }]
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({"status": "active", "name": "test"}));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(result_value) = result {
            assert_eq!(result_value["conditionMet"], false);
        }
    }
}