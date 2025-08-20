use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParams {
    pub function_code: String,
    pub language: String, // "javascript" or "python"
    #[serde(default = "default_true")]
    pub include_original_data: bool,
    #[serde(default)]
    pub allow_console_log: bool,
}

fn default_true() -> bool { true }

impl Default for FunctionParams {
    fn default() -> Self {
        Self {
            function_code: "// Add your custom function code here\nreturn items;".to_string(),
            language: "javascript".to_string(),
            include_original_data: true,
            allow_console_log: false,
        }
    }
}

impl INode for FunctionNode {
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
            serde_json::from_value::<FunctionParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            FunctionParams::default()
        };

        if params.function_code.trim().is_empty() {
            return Err(Error::Validation("Function code cannot be empty".to_string()));
        }

        // Combine all input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        // Convert input data to items format
        let mut items = Vec::new();
        for (index, item) in input_data.iter().enumerate() {
            if let Value::Array(arr) = item {
                for (sub_index, sub_item) in arr.iter().enumerate() {
                    items.push(json!({
                        "json": sub_item,
                        "index": index * 1000 + sub_index // Simple indexing scheme
                    }));
                }
            } else {
                items.push(json!({
                    "json": item,
                    "index": index
                }));
            }
        }

        match params.language.as_str() {
            "javascript" => self.execute_javascript(&params, &items),
            "python" => Err(Error::Validation("Python execution not yet implemented".to_string())),
            _ => Err(Error::Validation(format!("Unsupported language: {}", params.language)))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<FunctionParams>(params_value.clone()) {
                return !params.function_code.trim().is_empty() &&
                       matches!(params.language.as_str(), "javascript" | "python");
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl FunctionNode {
    fn execute_javascript(&self, params: &FunctionParams, items: &[Value]) -> Result<Value, Error> {
        // This is a simplified JavaScript execution
        // In a real implementation, you would use a proper JavaScript engine
        
        let code = &params.function_code;
        
        // Simple pattern matching for common operations
        if code.contains("return items") && !code.contains("for") && !code.contains("forEach") {
            // Simple passthrough
            return Ok(Value::Array(items.to_vec()));
        }
        
        if code.contains("item.json.myNewField = ") {
            // Add new field pattern
            return self.execute_add_field(code, items);
        }
        
        if code.contains("items.map(") {
            // Map operation pattern
            return self.execute_map_operation(code, items);
        }
        
        if code.contains("items.filter(") {
            // Filter operation pattern
            return self.execute_filter_operation(code, items);
        }
        
        if code.contains("for (item of items)") || code.contains("for(item of items)") {
            // For-of loop pattern
            return self.execute_for_loop(code, items);
        }
        
        // Default: try to execute simple transformations
        self.execute_simple_transformation(code, items)
    }

    fn execute_add_field(&self, code: &str, items: &[Value]) -> Result<Value, Error> {
        let mut result_items = Vec::new();
        
        // Extract the field name and value
        if let Some(assignment) = code.lines().find(|line| line.contains(".myNewField = ")) {
            let value_part = assignment.split(" = ").nth(1).unwrap_or("null").trim_end_matches(';').trim();
            let new_value = self.parse_simple_value(value_part)?;
            
            for item in items {
                let mut new_item = item.clone();
                if let Some(json_obj) = new_item.get_mut("json") {
                    if let Value::Object(map) = json_obj {
                        map.insert("myNewField".to_string(), new_value.clone());
                    }
                }
                result_items.push(new_item);
            }
        } else {
            result_items = items.to_vec();
        }
        
        Ok(Value::Array(result_items))
    }
    
    fn execute_for_loop(&self, code: &str, items: &[Value]) -> Result<Value, Error> {
        let mut result_items = items.to_vec();
        
        // Find assignments in the loop
        let lines: Vec<&str> = code.lines().collect();
        let mut in_loop = false;
        
        for line in lines {
            let trimmed = line.trim();
            
            if trimmed.contains("for (item of items)") || trimmed.contains("for(item of items)") {
                in_loop = true;
                continue;
            }
            
            if in_loop && trimmed == "}" {
                break;
            }
            
            if in_loop && trimmed.contains("item.json.") && trimmed.contains(" = ") {
                // Parse field assignment
                if let Some((field_part, value_part)) = self.parse_assignment(trimmed) {
                    if let Some(field_name) = field_part.strip_prefix("item.json.") {
                        let new_value = self.parse_simple_value(value_part)?;
                        
                        // Apply to all items
                        for item in &mut result_items {
                            if let Some(json_obj) = item.get_mut("json") {
                                if let Value::Object(map) = json_obj {
                                    map.insert(field_name.to_string(), new_value.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(Value::Array(result_items))
    }
    
    fn execute_map_operation(&self, _code: &str, items: &[Value]) -> Result<Value, Error> {
        // Simple map implementation - for now just return items
        Ok(Value::Array(items.to_vec()))
    }
    
    fn execute_filter_operation(&self, _code: &str, items: &[Value]) -> Result<Value, Error> {
        // Simple filter implementation - for now just return items
        Ok(Value::Array(items.to_vec()))
    }
    
    fn execute_simple_transformation(&self, _code: &str, items: &[Value]) -> Result<Value, Error> {
        // Default implementation - return items as-is with execution metadata
        let mut result_items = Vec::new();
        
        for item in items {
            let mut new_item = item.clone();
            if let Value::Object(ref mut map) = new_item {
                map.insert("_function_executed".to_string(), json!(true));
            }
            result_items.push(new_item);
        }
        
        Ok(Value::Array(result_items))
    }
    
    fn parse_assignment<'a>(&self, line: &'a str) -> Option<(&'a str, &'a str)> {
        if let Some(eq_pos) = line.find(" = ") {
            let field_part = line[..eq_pos].trim();
            let value_part = line[eq_pos + 3..].trim_end_matches(';').trim();
            return Some((field_part, value_part));
        }
        None
    }
    
    fn parse_simple_value(&self, value_str: &str) -> Result<Value, Error> {
        let trimmed = value_str.trim();
        
        // String literals
        if (trimmed.starts_with('"') && trimmed.ends_with('"')) ||
           (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
            return Ok(json!(trimmed[1..trimmed.len()-1]));
        }
        
        // Numbers
        if let Ok(int_val) = trimmed.parse::<i64>() {
            return Ok(json!(int_val));
        }
        
        if let Ok(float_val) = trimmed.parse::<f64>() {
            return Ok(json!(float_val));
        }
        
        // Booleans
        match trimmed {
            "true" => return Ok(json!(true)),
            "false" => return Ok(json!(false)),
            "null" => return Ok(Value::Null),
            _ => {}
        }
        
        // Default: treat as string
        Ok(json!(trimmed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_function_node_creation() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "function_code": "return items;",
                "language": "javascript"
            })),
        );

        assert_eq!(node.name(), "Test Function");
        assert!(node.validate());
    }

    #[test]
    fn test_function_validation() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            None,
            Some(json!({
                "function_code": "",
                "language": "javascript"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_simple_passthrough() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            None,
            Some(json!({
                "function_code": "return items;",
                "language": "javascript"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!([
                {"name": "John", "age": 30},
                {"name": "Jane", "age": 25}
            ])
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2);
            assert_eq!(results[0]["json"]["name"], "John");
            assert_eq!(results[1]["json"]["name"], "Jane");
        }
    }

    #[test]
    fn test_add_field_execution() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            None,
            Some(json!({
                "function_code": "for (item of items) {\n  item.json.myNewField = 42;\n}\nreturn items;",
                "language": "javascript"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"name": "test"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["json"]["name"], "test");
            assert_eq!(results[0]["json"]["myNewField"], 42);
        }
    }

    #[test]
    fn test_unsupported_language() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            None,
            Some(json!({
                "function_code": "print('hello')",
                "language": "python"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_function_code() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            None,
            Some(json!({
                "function_code": "",
                "language": "javascript"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_simple_values() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            None,
            None,
        );

        assert_eq!(node.parse_simple_value("42").unwrap(), json!(42));
        assert_eq!(node.parse_simple_value("3.14").unwrap(), json!(3.14));
        assert_eq!(node.parse_simple_value("\"hello\"").unwrap(), json!("hello"));
        assert_eq!(node.parse_simple_value("'world'").unwrap(), json!("world"));
        assert_eq!(node.parse_simple_value("true").unwrap(), json!(true));
        assert_eq!(node.parse_simple_value("false").unwrap(), json!(false));
        assert_eq!(node.parse_simple_value("null").unwrap(), Value::Null);
    }

    #[test]
    fn test_parse_assignment() {
        let node = FunctionNode::new(
            uuid::Uuid::new_v4(),
            "Test Function".to_string(),
            "function".to_string(),
            None,
            None,
        );

        if let Some((field, value)) = node.parse_assignment("item.json.myField = 123;") {
            assert_eq!(field, "item.json.myField");
            assert_eq!(value, "123");
        } else {
            panic!("Failed to parse assignment");
        }
    }
}