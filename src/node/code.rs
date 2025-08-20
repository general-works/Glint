use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeParams {
    pub mode: String, // "runOnceForAllItems" or "runOnceForEachItem"
    pub language: String, // "javascript" or "python"
    pub code: String,
}

impl Default for CodeParams {
    fn default() -> Self {
        Self {
            mode: "runOnceForAllItems".to_string(),
            language: "javascript".to_string(),
            code: String::new(),
        }
    }
}

impl INode for CodeNode {
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
            serde_json::from_value::<CodeParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            CodeParams::default()
        };

        // Validate required parameters
        if params.code.is_empty() {
            return Err(Error::Validation("Code is required".to_string()));
        }

        // For now, we'll implement a simple JavaScript-like evaluation
        // In a real implementation, you would use a proper JavaScript engine like V8 or QuickJS
        // This is a simplified demonstration
        match params.language.as_str() {
            "javascript" => self.execute_javascript(&params, input),
            "python" => Err(Error::Validation("Python execution not yet implemented".to_string())),
            _ => Err(Error::Validation(format!("Unsupported language: {}", params.language))),
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<CodeParams>(params_value.clone()) {
                return !params.code.is_empty();
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl CodeNode {
    fn execute_javascript(&self, params: &CodeParams, input: &NodeOutput) -> Result<Value, Error> {
        // This is a simplified implementation for demonstration purposes
        // In a real system, you would use a proper JavaScript engine
        
        // Combine all input data from previous nodes
        let input_data: Vec<Value> = input.values().cloned().collect();
        
        // If no input data, create an empty array
        let data_to_process = if input_data.is_empty() {
            vec![json!({})]
        } else {
            input_data
        };

        match params.mode.as_str() {
            "runOnceForAllItems" => {
                // Process all items at once
                self.execute_simple_javascript(&params.code, &json!(data_to_process))
            }
            "runOnceForEachItem" => {
                // Process each item individually
                let mut results = Vec::new();
                for item in data_to_process {
                    let result = self.execute_simple_javascript(&params.code, &item)?;
                    if let Value::Array(arr) = result {
                        results.extend(arr);
                    } else {
                        results.push(result);
                    }
                }
                Ok(Value::Array(results))
            }
            _ => Err(Error::Validation(format!("Unsupported mode: {}", params.mode))),
        }
    }

    fn execute_simple_javascript(&self, code: &str, input: &Value) -> Result<Value, Error> {
        // This is a very simplified JavaScript-like evaluation
        // In practice, you would use a proper JavaScript engine
        
        // Support for simple patterns like:
        // - return $input.item.field
        // - return { newField: $input.item.oldField }
        // - return $input.all()
        
        if code.contains("$input.all()") {
            return Ok(input.clone());
        }
        
        if code.contains("$input.item") && input.is_object() {
            // Simple field access patterns
            if let Some(field_access) = self.extract_field_access(code) {
                if let Some(field_value) = input.get(&field_access) {
                    return Ok(json!({ field_access: field_value }));
                }
            }
            
            // Return the input item as-is for now
            return Ok(input.clone());
        }
        
        // For other cases, try to create a simple transformation
        if code.contains("return {") && code.contains("}") {
            // Try to extract simple object creation
            return self.execute_object_creation(code, input);
        }
        
        // Default: return input as-is
        Ok(input.clone())
    }
    
    fn extract_field_access(&self, code: &str) -> Option<String> {
        // Simple regex-like extraction for $input.item.fieldName
        if let Some(start) = code.find("$input.item.") {
            let field_start = start + "$input.item.".len();
            let remaining = &code[field_start..];
            
            // Find the end of the field name
            let field_end = remaining.find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(remaining.len());
            
            return Some(remaining[..field_end].to_string());
        }
        None
    }
    
    fn execute_object_creation(&self, _code: &str, input: &Value) -> Result<Value, Error> {
        // Very simplified object creation handling
        // This would need a proper parser in a real implementation
        
        // For now, just return the input wrapped in a simple object
        Ok(json!({
            "result": input,
            "processed": true
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_node_creation() {
        let node = CodeNode::new(
            uuid::Uuid::new_v4(),
            "Test Code".to_string(),
            "code".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "mode": "runOnceForAllItems",
                "language": "javascript",
                "code": "return $input.all()"
            })),
        );

        assert_eq!(node.name(), "Test Code");
        assert!(node.validate());
    }

    #[test]
    fn test_code_validation() {
        let node = CodeNode::new(
            uuid::Uuid::new_v4(),
            "Test Code".to_string(),
            "code".to_string(),
            None,
            Some(json!({
                "mode": "runOnceForAllItems",
                "language": "javascript",
                "code": ""
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_simple_javascript_execution() {
        let node = CodeNode::new(
            uuid::Uuid::new_v4(),
            "Test Code".to_string(),
            "code".to_string(),
            None,
            Some(json!({
                "mode": "runOnceForAllItems",
                "language": "javascript",
                "code": "return $input.all()"
            })),
        );

        let mut input = NodeOutput::new();
        input.insert(uuid::Uuid::new_v4(), json!({"name": "test", "value": 123}));

        let result = node.execute(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unsupported_language() {
        let node = CodeNode::new(
            uuid::Uuid::new_v4(),
            "Test Code".to_string(),
            "code".to_string(),
            None,
            Some(json!({
                "mode": "runOnceForAllItems",
                "language": "python",
                "code": "return input"
            })),
        );

        let input = NodeOutput::new();

        let result = node.execute(&input);
        assert!(result.is_err());
    }
}