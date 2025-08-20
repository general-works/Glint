use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileParams {
    pub operation: String, // "read", "write", "exists", "size", "create_dir", "list_dir", "delete"
    // File path settings
    pub file_path: Option<String>,
    pub file_path_field: Option<String>,
    #[serde(default = "default_output_field")]
    pub output_field: String,
    // For write operations
    pub content: Option<String>,
    pub content_field: Option<String>,
    #[serde(default = "default_false")]
    pub append_mode: bool,
    // For directory operations
    pub recursive: Option<bool>,
    // For file encoding
    #[serde(default = "default_encoding")]
    pub encoding: String,
    // General options
    #[serde(default = "default_true")]
    pub include_input_fields: bool,
    #[serde(default = "default_false")]
    pub create_directories: bool,
}

fn default_false() -> bool { false }
fn default_true() -> bool { true }
fn default_output_field() -> String { "result".to_string() }
fn default_encoding() -> String { "utf-8".to_string() }

impl Default for FileParams {
    fn default() -> Self {
        Self {
            operation: "read".to_string(),
            file_path: None,
            file_path_field: None,
            output_field: "result".to_string(),
            content: None,
            content_field: None,
            append_mode: false,
            recursive: None,
            encoding: "utf-8".to_string(),
            include_input_fields: true,
            create_directories: false,
        }
    }
}

impl INode for FileNode {
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
            serde_json::from_value::<FileParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            FileParams::default()
        };

        // Process input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        let mut results = Vec::new();

        // If no input data and we have a file path, create a single item
        let items_to_process = if input_data.is_empty() && params.file_path.is_some() {
            vec![json!({})]
        } else if input_data.is_empty() {
            return Ok(json!([]));
        } else {
            let mut all_items = Vec::new();
            for item in input_data {
                if let Value::Array(items) = item {
                    all_items.extend(items);
                } else {
                    all_items.push(item);
                }
            }
            all_items
        };

        for item in items_to_process {
            let result_item = self.process_file_operation(&params, &item)?;
            results.push(result_item);
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<FileParams>(params_value.clone()) {
                let valid_operations = [
                    "read", "write", "exists", "size", "create_dir", "list_dir", "delete"
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

impl FileNode {
    fn process_file_operation(&self, params: &FileParams, item: &Value) -> Result<Value, Error> {
        let mut result_item = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        // Get file path
        let file_path = self.get_file_path(params, item)?;

        // Apply file operation
        let result_value = match params.operation.as_str() {
            "read" => self.read_file(&file_path),
            "write" => self.write_file(params, item, &file_path),
            "exists" => self.file_exists(&file_path),
            "size" => self.file_size(&file_path),
            "create_dir" => self.create_directory(params, &file_path),
            "list_dir" => self.list_directory(&file_path),
            "delete" => self.delete_file(&file_path),
            _ => return Err(Error::Validation(format!("Unsupported file operation: {}", params.operation)))
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

    fn get_file_path(&self, params: &FileParams, item: &Value) -> Result<String, Error> {
        if let Some(file_path) = &params.file_path {
            Ok(file_path.clone())
        } else if let Some(field_name) = &params.file_path_field {
            let field_value = self.get_field_value(field_name, item)?;
            match field_value {
                Value::String(s) => Ok(s),
                _ => Err(Error::Validation("File path field must contain a string".to_string()))
            }
        } else if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
            Ok(path.to_string())
        } else if let Some(file) = item.get("file").and_then(|v| v.as_str()) {
            Ok(file.to_string())
        } else if let Some(filename) = item.get("filename").and_then(|v| v.as_str()) {
            Ok(filename.to_string())
        } else {
            Err(Error::Validation("File path is required but not found".to_string()))
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

    fn read_file(&self, file_path: &str) -> Result<Value, Error> {
        match fs::read_to_string(file_path) {
            Ok(content) => Ok(json!({
                "content": content,
                "success": true
            })),
            Err(e) => Ok(json!({
                "error": e.to_string(),
                "success": false
            }))
        }
    }

    fn write_file(&self, params: &FileParams, item: &Value, file_path: &str) -> Result<Value, Error> {
        // Get content to write
        let content = if let Some(content_str) = &params.content {
            content_str.clone()
        } else if let Some(field_name) = &params.content_field {
            let field_value = self.get_field_value(field_name, item)?;
            match field_value {
                Value::String(s) => s,
                _ => field_value.to_string()
            }
        } else if let Some(content_val) = item.get("content") {
            match content_val {
                Value::String(s) => s.clone(),
                _ => content_val.to_string()
            }
        } else {
            return Err(Error::Validation("Content is required for write operation".to_string()));
        };

        // Create parent directories if needed
        if params.create_directories {
            if let Some(parent) = Path::new(file_path).parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Ok(json!({
                        "error": format!("Failed to create directories: {}", e),
                        "success": false
                    }));
                }
            }
        }

        // Write file
        let write_result = if params.append_mode {
            fs::write(file_path, content.as_bytes())
        } else {
            fs::write(file_path, content.as_bytes())
        };

        match write_result {
            Ok(_) => Ok(json!({
                "bytes_written": content.len(),
                "success": true
            })),
            Err(e) => Ok(json!({
                "error": e.to_string(),
                "success": false
            }))
        }
    }

    fn file_exists(&self, file_path: &str) -> Result<Value, Error> {
        let exists = Path::new(file_path).exists();
        Ok(json!({
            "exists": exists,
            "success": true
        }))
    }

    fn file_size(&self, file_path: &str) -> Result<Value, Error> {
        match fs::metadata(file_path) {
            Ok(metadata) => Ok(json!({
                "size": metadata.len(),
                "success": true
            })),
            Err(e) => Ok(json!({
                "error": e.to_string(),
                "success": false
            }))
        }
    }

    fn create_directory(&self, params: &FileParams, dir_path: &str) -> Result<Value, Error> {
        let create_result = if params.recursive.unwrap_or(true) {
            fs::create_dir_all(dir_path)
        } else {
            fs::create_dir(dir_path)
        };

        match create_result {
            Ok(_) => Ok(json!({
                "created": true,
                "success": true
            })),
            Err(e) => Ok(json!({
                "error": e.to_string(),
                "success": false
            }))
        }
    }

    fn list_directory(&self, dir_path: &str) -> Result<Value, Error> {
        match fs::read_dir(dir_path) {
            Ok(entries) => {
                let mut files = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Some(filename) = entry.file_name().to_str() {
                            files.push(json!({
                                "name": filename,
                                "path": entry.path().to_string_lossy(),
                                "is_dir": entry.path().is_dir()
                            }));
                        }
                    }
                }
                Ok(json!({
                    "files": files,
                    "count": files.len(),
                    "success": true
                }))
            },
            Err(e) => Ok(json!({
                "error": e.to_string(),
                "success": false
            }))
        }
    }

    fn delete_file(&self, file_path: &str) -> Result<Value, Error> {
        let path = Path::new(file_path);
        let delete_result = if path.is_dir() {
            fs::remove_dir_all(file_path)
        } else {
            fs::remove_file(file_path)
        };

        match delete_result {
            Ok(_) => Ok(json!({
                "deleted": true,
                "success": true
            })),
            Err(e) => Ok(json!({
                "error": e.to_string(),
                "success": false
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn test_file_node_creation() {
        let node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File".to_string(),
            "file".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "operation": "exists"
            })),
        );

        assert_eq!(node.name(), "Test File");
        assert!(node.validate());
    }

    #[test]
    fn test_file_validation() {
        let node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File".to_string(),
            "file".to_string(),
            None,
            Some(json!({
                "operation": "invalid_operation"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_file_exists() {
        let node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File".to_string(),
            "file".to_string(),
            None,
            Some(json!({
                "operation": "exists",
                "file_path": "Cargo.toml",
                "output_field": "file_exists"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["file_exists"]["exists"], true);
            assert_eq!(results[0]["file_exists"]["success"], true);
        }
    }

    #[test]
    fn test_file_size() {
        let node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File".to_string(),
            "file".to_string(),
            None,
            Some(json!({
                "operation": "size",
                "file_path": "Cargo.toml",
                "output_field": "file_info"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert!(results[0]["file_info"]["size"].as_u64().unwrap() > 0);
            assert_eq!(results[0]["file_info"]["success"], true);
        }
    }

    #[test]
    fn test_write_and_read_file() {
        let test_file = "/tmp/test_workflow_file.txt";
        let test_content = "Hello, World! This is a test file.";

        // Clean up before test
        let _ = fs::remove_file(test_file);

        // Test write
        let write_node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File Write".to_string(),
            "file".to_string(),
            None,
            Some(json!({
                "operation": "write",
                "file_path": test_file,
                "content": test_content,
                "create_directories": true,
                "output_field": "write_result"
            })),
        );

        let input = HashMap::new();
        let write_result = write_node.execute(&input);
        assert!(write_result.is_ok());

        // Test read
        let read_node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File Read".to_string(),
            "file".to_string(),
            None,
            Some(json!({
                "operation": "read",
                "file_path": test_file,
                "output_field": "read_result"
            })),
        );

        let read_result = read_node.execute(&input);
        assert!(read_result.is_ok());
        
        if let Ok(Value::Array(results)) = read_result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["read_result"]["content"], test_content);
            assert_eq!(results[0]["read_result"]["success"], true);
        }

        // Clean up after test
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_list_directory() {
        let node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File List".to_string(),
            "file".to_string(),
            None,
            Some(json!({
                "operation": "list_dir",
                "file_path": "src",
                "output_field": "dir_contents"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["dir_contents"]["success"], true);
            assert!(results[0]["dir_contents"]["count"].as_u64().unwrap() > 0);
        }
    }

    #[test]
    fn test_file_path_from_field() {
        let node = FileNode::new(
            uuid::Uuid::new_v4(),
            "Test File".to_string(),
            "file".to_string(),
            None,
            Some(json!({
                "operation": "exists",
                "file_path_field": "filepath",
                "output_field": "exists_result"
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"filepath": "Cargo.toml"})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["exists_result"]["exists"], true);
        }
    }
}