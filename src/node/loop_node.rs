use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopParams {
    pub mode: String, // "for", "while", "forEach"
    // For "for" mode
    #[serde(default = "default_start")]
    pub start: i64,
    #[serde(default = "default_end")]
    pub end: i64,
    #[serde(default = "default_step")]
    pub step: i64,
    // For "while" mode
    pub condition: Option<String>,
    pub max_iterations: Option<usize>,
    // For "forEach" mode
    pub array_field: Option<String>,
    // General options
    #[serde(default = "default_iteration_field")]
    pub iteration_field: String,
    #[serde(default = "default_value_field")]
    pub value_field: String,
    #[serde(default = "default_true")]
    pub include_original_data: bool,
}

fn default_start() -> i64 { 0 }
fn default_end() -> i64 { 10 }
fn default_step() -> i64 { 1 }
fn default_iteration_field() -> String { "iteration".to_string() }
fn default_value_field() -> String { "value".to_string() }
fn default_true() -> bool { true }

impl Default for LoopParams {
    fn default() -> Self {
        Self {
            mode: "for".to_string(),
            start: 0,
            end: 10,
            step: 1,
            condition: None,
            max_iterations: Some(1000),
            array_field: None,
            iteration_field: "iteration".to_string(),
            value_field: "value".to_string(),
            include_original_data: true,
        }
    }
}

impl INode for LoopNode {
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
            serde_json::from_value::<LoopParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            LoopParams::default()
        };

        // Combine all input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        match params.mode.as_str() {
            "for" => self.execute_for_loop(&params, &input_data),
            "while" => self.execute_while_loop(&params, &input_data),
            "forEach" => self.execute_foreach_loop(&params, &input_data),
            _ => Err(Error::Validation(format!("Unsupported loop mode: {}", params.mode)))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<LoopParams>(params_value.clone()) {
                return matches!(params.mode.as_str(), "for" | "while" | "forEach") &&
                       (params.mode != "for" || params.step != 0) &&
                       (params.mode != "forEach" || params.array_field.is_some());
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl LoopNode {
    fn execute_for_loop(&self, params: &LoopParams, input_data: &[Value]) -> Result<Value, Error> {
        if params.step == 0 {
            return Err(Error::Validation("Loop step cannot be zero".to_string()));
        }

        let mut results = Vec::new();
        let mut current = params.start;
        
        // Determine direction and condition based on step
        let should_continue = if params.step > 0 {
            |curr: i64, end: i64| curr < end
        } else {
            |curr: i64, end: i64| curr > end
        };

        while should_continue(current, params.end) {
            for item in input_data {
                if let Value::Array(items) = item {
                    for single_item in items {
                        let loop_item = self.create_loop_item(params, single_item, current, None)?;
                        results.push(loop_item);
                    }
                } else {
                    let loop_item = self.create_loop_item(params, item, current, None)?;
                    results.push(loop_item);
                }
            }
            current += params.step;
        }

        Ok(Value::Array(results))
    }

    fn execute_while_loop(&self, params: &LoopParams, input_data: &[Value]) -> Result<Value, Error> {
        let condition = params.condition.as_ref()
            .ok_or_else(|| Error::Validation("Condition is required for while loop".to_string()))?;

        let max_iterations = params.max_iterations.unwrap_or(1000);
        let mut results = Vec::new();
        let mut iteration = 0;

        while iteration < max_iterations {
            // Simple condition evaluation (very basic implementation)
            if !self.evaluate_condition(condition, iteration)? {
                break;
            }

            for item in input_data {
                if let Value::Array(items) = item {
                    for single_item in items {
                        let loop_item = self.create_loop_item(params, single_item, iteration as i64, None)?;
                        results.push(loop_item);
                    }
                } else {
                    let loop_item = self.create_loop_item(params, item, iteration as i64, None)?;
                    results.push(loop_item);
                }
            }
            iteration += 1;
        }

        if iteration >= max_iterations {
            return Err(Error::Validation(format!("Loop exceeded maximum iterations: {}", max_iterations)));
        }

        Ok(Value::Array(results))
    }

    fn execute_foreach_loop(&self, params: &LoopParams, input_data: &[Value]) -> Result<Value, Error> {
        let array_field = params.array_field.as_ref()
            .ok_or_else(|| Error::Validation("array_field is required for forEach loop".to_string()))?;

        let mut results = Vec::new();

        for item in input_data {
            if let Value::Array(items) = item {
                for single_item in items {
                    let array_data = self.get_field_value(array_field, single_item)?;
                    let loop_items = self.process_array_for_loop(params, single_item, &array_data)?;
                    results.extend(loop_items);
                }
            } else {
                let array_data = self.get_field_value(array_field, item)?;
                let loop_items = self.process_array_for_loop(params, item, &array_data)?;
                results.extend(loop_items);
            }
        }

        Ok(Value::Array(results))
    }

    fn process_array_for_loop(&self, params: &LoopParams, original_item: &Value, array_data: &Value) -> Result<Vec<Value>, Error> {
        let mut results = Vec::new();
        
        match array_data {
            Value::Array(arr) => {
                for (index, element) in arr.iter().enumerate() {
                    let loop_item = self.create_loop_item(params, original_item, index as i64, Some(element))?;
                    results.push(loop_item);
                }
            }
            Value::Object(obj) => {
                for (index, (_, value)) in obj.iter().enumerate() {
                    let loop_item = self.create_loop_item(params, original_item, index as i64, Some(value))?;
                    results.push(loop_item);
                }
            }
            _ => {
                // Single value, iterate once
                let loop_item = self.create_loop_item(params, original_item, 0, Some(array_data))?;
                results.push(loop_item);
            }
        }

        Ok(results)
    }

    fn create_loop_item(&self, params: &LoopParams, original_item: &Value, iteration: i64, loop_value: Option<&Value>) -> Result<Value, Error> {
        let mut result = if params.include_original_data {
            original_item.clone()
        } else {
            json!({})
        };

        // Add iteration information
        if let Value::Object(ref mut map) = result {
            map.insert(params.iteration_field.clone(), json!(iteration));
            
            if let Some(value) = loop_value {
                map.insert(params.value_field.clone(), value.clone());
            }
        } else {
            let mut new_obj = json!({});
            if let Value::Object(ref mut map) = new_obj {
                map.insert(params.iteration_field.clone(), json!(iteration));
                
                if let Some(value) = loop_value {
                    map.insert(params.value_field.clone(), value.clone());
                }
                
                if params.include_original_data {
                    map.insert("original_data".to_string(), original_item.clone());
                }
            }
            result = new_obj;
        }

        Ok(result)
    }

    fn evaluate_condition(&self, condition: &str, iteration: usize) -> Result<bool, Error> {
        // Very simple condition evaluation
        // In a real implementation, you'd use a proper expression parser
        if condition.contains("iteration < ") {
            if let Some(limit_str) = condition.split("iteration < ").nth(1) {
                if let Ok(limit) = limit_str.trim().parse::<usize>() {
                    return Ok(iteration < limit);
                }
            }
        }
        
        if condition.contains("iteration <= ") {
            if let Some(limit_str) = condition.split("iteration <= ").nth(1) {
                if let Ok(limit) = limit_str.trim().parse::<usize>() {
                    return Ok(iteration <= limit);
                }
            }
        }

        // Default: true for simple cases
        if condition == "true" {
            return Ok(true);
        }
        
        if condition == "false" {
            return Ok(false);
        }

        Err(Error::Validation(format!("Unsupported condition format: {}", condition)))
    }

    fn get_field_value(&self, field_path: &str, data: &Value) -> Result<Value, Error> {
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
    fn test_loop_node_creation() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "mode": "for",
                "start": 0,
                "end": 3,
                "step": 1
            })),
        );

        assert_eq!(node.name(), "Test Loop");
        assert!(node.validate());
    }

    #[test]
    fn test_loop_validation() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            Some(json!({
                "mode": "invalid_mode"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_for_loop_execution() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            Some(json!({
                "mode": "for",
                "start": 0,
                "end": 3,
                "step": 1,
                "include_original_data": false
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"base": "data"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // 0, 1, 2
            assert_eq!(results[0]["iteration"], 0);
            assert_eq!(results[1]["iteration"], 1);
            assert_eq!(results[2]["iteration"], 2);
        }
    }

    #[test]
    fn test_while_loop_execution() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            Some(json!({
                "mode": "while",
                "condition": "iteration < 3",
                "include_original_data": false
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"base": "data"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // 0, 1, 2
            assert_eq!(results[0]["iteration"], 0);
            assert_eq!(results[1]["iteration"], 1);
            assert_eq!(results[2]["iteration"], 2);
        }
    }

    #[test]
    fn test_foreach_loop_execution() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            Some(json!({
                "mode": "forEach",
                "array_field": "items",
                "include_original_data": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "name": "test",
                "items": ["apple", "banana", "cherry"]
            })
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3);
            assert_eq!(results[0]["iteration"], 0);
            assert_eq!(results[0]["value"], "apple");
            assert_eq!(results[0]["name"], "test");
            assert_eq!(results[1]["value"], "banana");
            assert_eq!(results[2]["value"], "cherry");
        }
    }

    #[test]
    fn test_for_loop_with_negative_step() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            Some(json!({
                "mode": "for",
                "start": 5,
                "end": 2,
                "step": -1,
                "include_original_data": false
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"base": "data"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 3); // 5, 4, 3
            assert_eq!(results[0]["iteration"], 5);
            assert_eq!(results[1]["iteration"], 4);
            assert_eq!(results[2]["iteration"], 3);
        }
    }

    #[test]
    fn test_zero_step_validation() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            Some(json!({
                "mode": "for",
                "step": 0
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_foreach_validation() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            Some(json!({
                "mode": "forEach"
                // Missing array_field
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_condition_evaluation() {
        let node = LoopNode::new(
            uuid::Uuid::new_v4(),
            "Test Loop".to_string(),
            "loop".to_string(),
            None,
            None,
        );

        assert!(node.evaluate_condition("iteration < 5", 3).unwrap());
        assert!(!node.evaluate_condition("iteration < 5", 5).unwrap());
        assert!(node.evaluate_condition("iteration <= 5", 5).unwrap());
        assert!(!node.evaluate_condition("iteration <= 5", 6).unwrap());
    }
}