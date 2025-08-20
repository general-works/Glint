use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchParams {
    pub mode: String, // "rules" or "expression"
    pub rules: Option<Vec<SwitchRule>>,
    pub number_outputs: Option<usize>,
    pub output_expression: Option<String>,
    pub fallback_output: Option<String>, // "none", "extra", or output index
    pub all_matching_outputs: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchRule {
    pub conditions: Vec<SwitchCondition>,
    pub combinator: String, // "and" or "or"
    pub output_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCondition {
    pub field: String,
    pub operator: String, // "equals", "notEquals", "contains", "greaterThan", "lessThan"
    pub value: Value,
}

impl Default for SwitchParams {
    fn default() -> Self {
        Self {
            mode: "rules".to_string(),
            rules: Some(vec![]),
            number_outputs: Some(2),
            output_expression: None,
            fallback_output: Some("none".to_string()),
            all_matching_outputs: Some(false),
        }
    }
}

impl INode for SwitchNode {
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
            serde_json::from_value::<SwitchParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            SwitchParams::default()
        };

        // Combine all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        // If no input data, return empty outputs
        if input_data.is_empty() {
            return Ok(self.create_empty_outputs(&params));
        }

        match params.mode.as_str() {
            "rules" => self.execute_rules_mode(&params, &input_data),
            "expression" => self.execute_expression_mode(&params, &input_data),
            _ => Err(Error::Validation(format!("Unsupported mode: {}", params.mode)))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<SwitchParams>(params_value.clone()) {
                match params.mode.as_str() {
                    "rules" => {
                        if let Some(rules) = &params.rules {
                            return !rules.is_empty();
                        }
                        false
                    }
                    "expression" => {
                        params.output_expression.is_some() && 
                        params.number_outputs.unwrap_or(0) > 0
                    }
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

impl SwitchNode {
    fn execute_rules_mode(&self, params: &SwitchParams, input_data: &[Value]) -> Result<Value, Error> {
        let default_rules = vec![];
        let rules = params.rules.as_ref().unwrap_or(&default_rules);
        let all_matching = params.all_matching_outputs.unwrap_or(false);
        let default_fallback = "none".to_string();
        let fallback = params.fallback_output.as_ref().unwrap_or(&default_fallback);
        
        // Initialize outputs
        let mut outputs = vec![Vec::<Value>::new(); rules.len()];
        let mut fallback_output = Vec::<Value>::new();
        
        // Process each input item
        for item in input_data {
            if let Value::Array(items) = item {
                // If input is an array, process each item
                for single_item in items {
                    self.route_item_by_rules(&single_item, rules, &mut outputs, &mut fallback_output, all_matching, fallback)?;
                }
            } else {
                // If input is a single item, process it
                self.route_item_by_rules(item, rules, &mut outputs, &mut fallback_output, all_matching, fallback)?;
            }
        }
        
        // Convert outputs to the expected format
        let mut result_outputs: Vec<Value> = outputs.into_iter()
            .map(|output| Value::Array(output))
            .collect();
        
        // Add fallback output if needed
        if fallback == "extra" {
            result_outputs.push(Value::Array(fallback_output));
        }
        
        Ok(json!({
            "outputs": result_outputs,
            "mode": "rules"
        }))
    }

    fn execute_expression_mode(&self, params: &SwitchParams, input_data: &[Value]) -> Result<Value, Error> {
        let num_outputs = params.number_outputs.unwrap_or(2);
        let expression = params.output_expression.as_ref()
            .ok_or_else(|| Error::Validation("Output expression is required for expression mode".to_string()))?;
        
        // Initialize outputs
        let mut outputs = vec![Vec::<Value>::new(); num_outputs];
        
        // Process each input item
        for item in input_data {
            if let Value::Array(items) = item {
                // If input is an array, process each item
                for single_item in items {
                    let output_index = self.evaluate_expression(expression, &single_item)?;
                    if output_index < num_outputs {
                        outputs[output_index].push(single_item.clone());
                    }
                }
            } else {
                // If input is a single item, process it
                let output_index = self.evaluate_expression(expression, item)?;
                if output_index < num_outputs {
                    outputs[output_index].push(item.clone());
                }
            }
        }
        
        // Convert outputs to the expected format
        let result_outputs: Vec<Value> = outputs.into_iter()
            .map(|output| Value::Array(output))
            .collect();
        
        Ok(json!({
            "outputs": result_outputs,
            "mode": "expression"
        }))
    }

    fn route_item_by_rules(
        &self,
        item: &Value,
        rules: &[SwitchRule],
        outputs: &mut [Vec<Value>],
        fallback_output: &mut Vec<Value>,
        all_matching: bool,
        fallback: &str,
    ) -> Result<(), Error> {
        let mut matched = false;
        
        for (rule_index, rule) in rules.iter().enumerate() {
            if self.evaluate_rule(rule, item)? {
                matched = true;
                if rule_index < outputs.len() {
                    outputs[rule_index].push(item.clone());
                }
                
                if !all_matching {
                    return Ok(()); // Stop at first match if not all_matching
                }
            }
        }
        
        // Handle fallback if no rules matched
        if !matched {
            match fallback {
                "none" => {
                    // Do nothing - item is discarded
                }
                "extra" => {
                    fallback_output.push(item.clone());
                }
                _ => {
                    // Try to parse as output index
                    if let Ok(index) = fallback.parse::<usize>() {
                        if index < outputs.len() {
                            outputs[index].push(item.clone());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn evaluate_rule(&self, rule: &SwitchRule, item: &Value) -> Result<bool, Error> {
        if rule.conditions.is_empty() {
            return Ok(true);
        }
        
        let results: Result<Vec<bool>, Error> = rule.conditions
            .iter()
            .map(|condition| self.evaluate_condition(condition, item))
            .collect();
        
        let condition_results = results?;
        
        // Combine results based on combinator
        match rule.combinator.to_uppercase().as_str() {
            "AND" => Ok(condition_results.iter().all(|&result| result)),
            "OR" => Ok(condition_results.iter().any(|&result| result)),
            _ => Err(Error::Validation(format!("Unsupported combinator: {}", rule.combinator)))
        }
    }

    fn evaluate_condition(&self, condition: &SwitchCondition, item: &Value) -> Result<bool, Error> {
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

    fn evaluate_expression(&self, expression: &str, item: &Value) -> Result<usize, Error> {
        // Very simplified expression evaluation
        // In a real implementation, you would use a proper expression engine
        
        // Handle simple numeric expressions
        if let Ok(index) = expression.parse::<usize>() {
            return Ok(index);
        }
        
        // Handle simple field access like $input.item.priority
        if expression.contains("$input.item.") {
            if let Some(field_name) = expression.strip_prefix("$input.item.") {
                if let Some(field_value) = item.get(field_name) {
                    if let Some(num) = field_value.as_u64() {
                        return Ok(num as usize);
                    }
                }
            }
        }
        
        // Default to output 0
        Ok(0)
    }

    fn create_empty_outputs(&self, params: &SwitchParams) -> Value {
        match params.mode.as_str() {
            "rules" => {
                let num_outputs = params.rules.as_ref().map(|r| r.len()).unwrap_or(0);
                let outputs: Vec<Value> = (0..num_outputs).map(|_| json!([])).collect();
                json!({
                    "outputs": outputs,
                    "mode": "rules"
                })
            }
            "expression" => {
                let num_outputs = params.number_outputs.unwrap_or(2);
                let outputs: Vec<Value> = (0..num_outputs).map(|_| json!([])).collect();
                json!({
                    "outputs": outputs,
                    "mode": "expression"
                })
            }
            _ => json!({ "outputs": [], "mode": "unknown" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_switch_node_creation() {
        let node = SwitchNode::new(
            Uuid::new_v4(),
            "Test Switch".to_string(),
            "switch".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "mode": "rules",
                "rules": [{
                    "conditions": [{
                        "field": "status",
                        "operator": "equals",
                        "value": "active"
                    }],
                    "combinator": "and"
                }]
            })),
        );

        assert_eq!(node.name(), "Test Switch");
        assert!(node.validate());
    }

    #[test]
    fn test_switch_validation_rules_mode() {
        let node = SwitchNode::new(
            Uuid::new_v4(),
            "Test Switch".to_string(),
            "switch".to_string(),
            None,
            Some(json!({
                "mode": "rules",
                "rules": []
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_switch_validation_expression_mode() {
        let node = SwitchNode::new(
            Uuid::new_v4(),
            "Test Switch".to_string(),
            "switch".to_string(),
            None,
            Some(json!({
                "mode": "expression",
                "number_outputs": 3,
                "output_expression": "$input.item.priority"
            })),
        );

        assert!(node.validate());
    }

    #[test]
    fn test_switch_rules_execution() {
        let node = SwitchNode::new(
            Uuid::new_v4(),
            "Test Switch".to_string(),
            "switch".to_string(),
            None,
            Some(json!({
                "mode": "rules",
                "rules": [{
                    "conditions": [{
                        "field": "status",
                        "operator": "equals",
                        "value": "active"
                    }],
                    "combinator": "and"
                }],
                "fallback_output": "none"
            })),
        );

        let mut input = HashMap::new();
        input.insert(Uuid::new_v4(), json!([
            {"status": "active", "name": "item1"},
            {"status": "inactive", "name": "item2"},
            {"status": "active", "name": "item3"}
        ]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(result_value) = result {
            assert_eq!(result_value["mode"], "rules");
            assert!(result_value["outputs"].is_array());
            
            if let Some(outputs) = result_value["outputs"].as_array() {
                assert_eq!(outputs.len(), 1); // One rule = one output
                if let Some(first_output) = outputs[0].as_array() {
                    assert_eq!(first_output.len(), 2); // Two active items
                }
            }
        }
    }

    #[test]
    fn test_switch_expression_execution() {
        let node = SwitchNode::new(
            Uuid::new_v4(),
            "Test Switch".to_string(),
            "switch".to_string(),
            None,
            Some(json!({
                "mode": "expression",
                "number_outputs": 3,
                "output_expression": "$input.item.priority"
            })),
        );

        let mut input = HashMap::new();
        input.insert(Uuid::new_v4(), json!([
            {"priority": 0, "name": "item1"},
            {"priority": 1, "name": "item2"},
            {"priority": 2, "name": "item3"}
        ]));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(result_value) = result {
            assert_eq!(result_value["mode"], "expression");
            assert!(result_value["outputs"].is_array());
            
            if let Some(outputs) = result_value["outputs"].as_array() {
                assert_eq!(outputs.len(), 3); // Three outputs
                // Each output should have one item
                for output in outputs {
                    if let Some(output_array) = output.as_array() {
                        assert_eq!(output_array.len(), 1);
                    }
                }
            }
        }
    }
}