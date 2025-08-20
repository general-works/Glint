use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathParams {
    pub operation: String, // "add", "subtract", "multiply", "divide", "power", "sqrt", "abs", "round", "floor", "ceil", "min", "max", "sum", "avg"
    // Input fields
    pub operand1_field: Option<String>,
    pub operand2_field: Option<String>,
    pub operand1_value: Option<f64>,
    pub operand2_value: Option<f64>,
    #[serde(default = "default_output_field")]
    pub output_field: String,
    // For array operations (sum, avg, min, max)
    pub array_field: Option<String>,
    // For rounding operations
    pub decimal_places: Option<u32>,
    // General options
    #[serde(default = "default_true")]
    pub include_input_fields: bool,
    #[serde(default = "default_false")]
    pub handle_arrays: bool,
}

fn default_false() -> bool { false }
fn default_true() -> bool { true }
fn default_output_field() -> String { "result".to_string() }

impl Default for MathParams {
    fn default() -> Self {
        Self {
            operation: "add".to_string(),
            operand1_field: None,
            operand2_field: None,
            operand1_value: None,
            operand2_value: None,
            output_field: "result".to_string(),
            array_field: None,
            decimal_places: None,
            include_input_fields: true,
            handle_arrays: false,
        }
    }
}

impl INode for MathNode {
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
            serde_json::from_value::<MathParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            MathParams::default()
        };

        // Process input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        let mut results = Vec::new();

        // If no input data, return empty array
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
            let result_item = self.process_math_operation(&params, &item)?;
            results.push(result_item);
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<MathParams>(params_value.clone()) {
                let valid_operations = [
                    "add", "subtract", "multiply", "divide", "power", "sqrt", "abs", 
                    "round", "floor", "ceil", "min", "max", "sum", "avg"
                ];
                return valid_operations.contains(&params.operation.as_str());
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl MathNode {
    fn process_math_operation(&self, params: &MathParams, item: &Value) -> Result<Value, Error> {
        let mut result_item = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Apply math operation
        let result_value = match params.operation.as_str() {
            "add" => self.binary_operation(params, item, |a, b| a + b),
            "subtract" => self.binary_operation(params, item, |a, b| a - b),
            "multiply" => self.binary_operation(params, item, |a, b| a * b),
            "divide" => self.binary_operation(params, item, |a, b| {
                if b == 0.0 {
                    f64::NAN
                } else {
                    a / b
                }
            }),
            "power" => self.binary_operation(params, item, |a, b| a.powf(b)),
            "sqrt" => self.unary_operation(params, item, |a| a.sqrt()),
            "abs" => self.unary_operation(params, item, |a| a.abs()),
            "round" => self.round_operation(params, item),
            "floor" => self.unary_operation(params, item, |a| a.floor()),
            "ceil" => self.unary_operation(params, item, |a| a.ceil()),
            "min" => self.array_operation(params, item, |arr| {
                arr.iter().fold(f64::INFINITY, |acc, &x| acc.min(x))
            }),
            "max" => self.array_operation(params, item, |arr| {
                arr.iter().fold(f64::NEG_INFINITY, |acc, &x| acc.max(x))
            }),
            "sum" => self.array_operation(params, item, |arr| arr.iter().sum()),
            "avg" => self.array_operation(params, item, |arr| {
                if arr.is_empty() {
                    f64::NAN
                } else {
                    arr.iter().sum::<f64>() / arr.len() as f64
                }
            }),
            _ => return Err(Error::Validation(format!("Unsupported math operation: {}", params.operation)))
        }?;

        // Add result to output
        if let Value::Object(map) = &mut result_item {
            map.insert(params.output_field.clone(), result_value);
        } else {
            result_item = json!({
                params.output_field.clone(): result_value
            });
        }

        Ok(result_item)
    }

    fn binary_operation<F>(&self, params: &MathParams, item: &Value, op: F) -> Result<Value, Error>
    where
        F: Fn(f64, f64) -> f64,
    {
        let operand1 = self.get_operand(params.operand1_field.as_ref(), params.operand1_value, item)?;
        let operand2 = self.get_operand(params.operand2_field.as_ref(), params.operand2_value, item)?;
        
        let result = op(operand1, operand2);
        Ok(json!(result))
    }

    fn unary_operation<F>(&self, params: &MathParams, item: &Value, op: F) -> Result<Value, Error>
    where
        F: Fn(f64) -> f64,
    {
        let operand = self.get_operand(params.operand1_field.as_ref(), params.operand1_value, item)?;
        let result = op(operand);
        Ok(json!(result))
    }

    fn round_operation(&self, params: &MathParams, item: &Value) -> Result<Value, Error> {
        let operand = self.get_operand(params.operand1_field.as_ref(), params.operand1_value, item)?;
        let decimal_places = params.decimal_places.unwrap_or(0);
        
        let multiplier = 10f64.powi(decimal_places as i32);
        let result = (operand * multiplier).round() / multiplier;
        
        Ok(json!(result))
    }

    fn array_operation<F>(&self, params: &MathParams, item: &Value, op: F) -> Result<Value, Error>
    where
        F: Fn(&[f64]) -> f64,
    {
        let array_values = if let Some(array_field) = &params.array_field {
            // Get array from specified field
            let field_value = self.get_field_value(array_field, item)?;
            self.value_to_number_array(&field_value)?
        } else {
            // Try to extract numbers from the whole item
            self.extract_numbers_from_item(item)?
        };

        let result = op(&array_values);
        Ok(json!(result))
    }

    fn get_operand(&self, field: Option<&String>, value: Option<f64>, item: &Value) -> Result<f64, Error> {
        if let Some(field_name) = field {
            // Get value from field
            let field_value = self.get_field_value(field_name, item)?;
            self.value_to_number(&field_value)
        } else if let Some(val) = value {
            // Use provided value
            Ok(val)
        } else {
            // Try to get number from common fields
            if let Some(num) = item.get("value").and_then(|v| v.as_f64()) {
                Ok(num)
            } else if let Some(num) = item.get("number").and_then(|v| v.as_f64()) {
                Ok(num)
            } else if let Some(num) = item.get("amount").and_then(|v| v.as_f64()) {
                Ok(num)
            } else {
                // Try to convert the whole item to a number
                self.value_to_number(item)
            }
        }
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

    fn value_to_number(&self, value: &Value) -> Result<f64, Error> {
        match value {
            Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
            Value::String(s) => {
                s.parse::<f64>()
                    .map_err(|_| Error::Validation(format!("Cannot convert '{}' to number", s)))
            },
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Null => Ok(0.0),
            _ => Err(Error::Validation(format!("Cannot convert {} to number", value)))
        }
    }

    fn value_to_number_array(&self, value: &Value) -> Result<Vec<f64>, Error> {
        match value {
            Value::Array(arr) => {
                let mut numbers = Vec::new();
                for item in arr {
                    numbers.push(self.value_to_number(item)?);
                }
                Ok(numbers)
            },
            Value::Object(obj) => {
                let mut numbers = Vec::new();
                for (_, val) in obj {
                    numbers.push(self.value_to_number(val)?);
                }
                Ok(numbers)
            },
            other => {
                // Single value
                Ok(vec![self.value_to_number(other)?])
            }
        }
    }

    fn extract_numbers_from_item(&self, item: &Value) -> Result<Vec<f64>, Error> {
        let mut numbers = Vec::new();
        self.collect_numbers(item, &mut numbers)?;
        Ok(numbers)
    }

    fn collect_numbers(&self, value: &Value, numbers: &mut Vec<f64>) -> Result<(), Error> {
        match value {
            Value::Number(n) => {
                if let Some(num) = n.as_f64() {
                    numbers.push(num);
                }
            },
            Value::String(s) => {
                if let Ok(num) = s.parse::<f64>() {
                    numbers.push(num);
                }
            },
            Value::Array(arr) => {
                for item in arr {
                    self.collect_numbers(item, numbers)?;
                }
            },
            Value::Object(obj) => {
                for (_, val) in obj {
                    self.collect_numbers(val, numbers)?;
                }
            },
            _ => {} // Skip non-numeric types
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_math_node_creation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "operation": "add"
            })),
        );

        assert_eq!(node.name(), "Test Math");
        assert!(node.validate());
    }

    #[test]
    fn test_math_validation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "invalid_operation"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_add_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "add",
                "operand1_value": 10.0,
                "operand2_value": 5.0,
                "output_field": "sum"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"id": 1})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["sum"], 15.0);
        }
    }

    #[test]
    fn test_subtract_with_fields() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "subtract",
                "operand1_field": "total",
                "operand2_field": "discount",
                "output_field": "final_amount"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"total": 100, "discount": 15})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["final_amount"], 85.0);
            assert_eq!(results[0]["total"], 100);
            assert_eq!(results[0]["discount"], 15);
        }
    }

    #[test]
    fn test_multiply_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "multiply",
                "operand1_field": "price",
                "operand2_field": "quantity",
                "output_field": "total"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"price": 12.5, "quantity": 3})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["total"], 37.5);
        }
    }

    #[test]
    fn test_divide_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "divide",
                "operand1_value": 100.0,
                "operand2_value": 4.0,
                "output_field": "result"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["result"], 25.0);
        }
    }

    #[test]
    fn test_sqrt_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "sqrt",
                "operand1_value": 16.0,
                "output_field": "square_root"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["square_root"], 4.0);
        }
    }

    #[test]
    fn test_round_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "round",
                "operand1_value": 3.14159,
                "decimal_places": 2,
                "output_field": "rounded"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["rounded"], 3.14);
        }
    }

    #[test]
    fn test_sum_array_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "sum",
                "array_field": "numbers",
                "output_field": "total"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"numbers": [1, 2, 3, 4, 5]})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["total"], 15.0);
        }
    }

    #[test]
    fn test_avg_array_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "avg",
                "array_field": "scores",
                "output_field": "average"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"scores": [85, 90, 78, 92, 88]})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["average"], 86.6);
        }
    }

    #[test]
    fn test_max_array_operation() {
        let node = MathNode::new(
            uuid::Uuid::new_v4(),
            "Test Math".to_string(),
            "math".to_string(),
            None,
            Some(json!({
                "operation": "max",
                "array_field": "values",
                "output_field": "maximum"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"values": [12, 45, 3, 67, 23]})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["maximum"], 67.0);
        }
    }
}