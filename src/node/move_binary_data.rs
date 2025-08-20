use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use base64::{Engine as _, engine::general_purpose};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveBinaryDataNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveBinaryDataParams {
    #[serde(default = "default_mode")]
    pub mode: String, // "jsonToBinary", "binaryToJson", "binaryToBase64", "base64ToBinary"
    #[serde(default)]
    pub encoding: String, // "base64", "hex", "utf8"
    #[serde(default)]
    pub source_field: Option<String>,
    #[serde(default = "default_target_field")]
    pub target_field: String,
    #[serde(default)]
    pub keep_source: bool,
    #[serde(default)]
    pub options: BinaryDataOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDataOptions {
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub charset: Option<String>,
}

impl Default for BinaryDataOptions {
    fn default() -> Self {
        Self {
            file_name: None,
            mime_type: None,
            charset: None,
        }
    }
}

fn default_mode() -> String { "jsonToBinary".to_string() }
fn default_target_field() -> String { "data".to_string() }

impl Default for MoveBinaryDataParams {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            encoding: "base64".to_string(),
            source_field: None,
            target_field: default_target_field(),
            keep_source: false,
            options: BinaryDataOptions::default(),
        }
    }
}

impl INode for MoveBinaryDataNode {
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
            serde_json::from_value::<MoveBinaryDataParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            MoveBinaryDataParams::default()
        };

        let input_data: Vec<Value> = input.values().cloned().collect();
        let mut results = Vec::new();

        for item in input_data {
            let processed_item = self.process_item(&item, &params)?;
            results.push(processed_item);
        }

        if results.is_empty() {
            results.push(json!({}));
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<MoveBinaryDataParams>(params_value.clone()) {
                let valid_modes = ["jsonToBinary", "binaryToJson", "binaryToBase64", "base64ToBinary"];
                let valid_encodings = ["base64", "hex", "utf8"];
                
                return valid_modes.contains(&params.mode.as_str()) &&
                       valid_encodings.contains(&params.encoding.as_str());
            }
        }
        true
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl MoveBinaryDataNode {
    fn process_item(&self, item: &Value, params: &MoveBinaryDataParams) -> Result<Value, Error> {
        let mut result = item.clone();

        match params.mode.as_str() {
            "jsonToBinary" => {
                self.json_to_binary(&mut result, params)?;
            }
            "binaryToJson" => {
                self.binary_to_json(&mut result, params)?;
            }
            "binaryToBase64" => {
                self.binary_to_base64(&mut result, params)?;
            }
            "base64ToBinary" => {
                self.base64_to_binary(&mut result, params)?;
            }
            _ => {
                return Err(Error::Validation(format!("Unsupported mode: {}", params.mode)));
            }
        }

        Ok(result)
    }

    fn json_to_binary(&self, item: &mut Value, params: &MoveBinaryDataParams) -> Result<(), Error> {
        let source_data = if let Some(source_field) = &params.source_field {
            item.get(source_field).cloned().unwrap_or(Value::Null)
        } else {
            item.clone()
        };

        let json_string = serde_json::to_string(&source_data)
            .map_err(|e| Error::NodeExecution(format!("Failed to serialize JSON: {}", e)))?;

        let encoded_data = match params.encoding.as_str() {
            "base64" => general_purpose::STANDARD.encode(json_string.as_bytes()),
            "hex" => hex::encode(json_string.as_bytes()),
            "utf8" => json_string,
            _ => return Err(Error::Validation(format!("Unsupported encoding: {}", params.encoding))),
        };

        let binary_data = json!({
            "data": encoded_data,
            "mimeType": params.options.mime_type.as_ref().unwrap_or(&"application/json".to_string()),
            "fileName": params.options.file_name.as_ref().unwrap_or(&"data.json".to_string()),
            "fileExtension": "json"
        });

        if let Some(obj) = item.as_object_mut() {
            obj.insert(params.target_field.clone(), binary_data);
            
            if !params.keep_source {
                if let Some(source_field) = &params.source_field {
                    obj.remove(source_field);
                }
            }
        }

        Ok(())
    }

    fn binary_to_json(&self, item: &mut Value, params: &MoveBinaryDataParams) -> Result<(), Error> {
        let binary_data = if let Some(source_field) = &params.source_field {
            item.get(source_field).cloned().unwrap_or(Value::Null)
        } else {
            item.clone()
        };

        let data_str = if let Some(data_value) = binary_data.get("data") {
            data_value.as_str().unwrap_or("")
        } else {
            binary_data.as_str().unwrap_or("")
        };

        if data_str.is_empty() {
            return Err(Error::NodeExecution("No binary data found".to_string()));
        }

        let decoded_bytes = match params.encoding.as_str() {
            "base64" => general_purpose::STANDARD.decode(data_str)
                .map_err(|e| Error::NodeExecution(format!("Failed to decode base64: {}", e)))?,
            "hex" => hex::decode(data_str)
                .map_err(|e| Error::NodeExecution(format!("Failed to decode hex: {}", e)))?,
            "utf8" => data_str.as_bytes().to_vec(),
            _ => return Err(Error::Validation(format!("Unsupported encoding: {}", params.encoding))),
        };

        let decoded_string = String::from_utf8(decoded_bytes)
            .map_err(|e| Error::NodeExecution(format!("Failed to convert to UTF-8: {}", e)))?;

        let parsed_json: Value = serde_json::from_str(&decoded_string)
            .map_err(|e| Error::NodeExecution(format!("Failed to parse JSON: {}", e)))?;

        if let Some(obj) = item.as_object_mut() {
            obj.insert(params.target_field.clone(), parsed_json);
            
            if !params.keep_source {
                if let Some(source_field) = &params.source_field {
                    obj.remove(source_field);
                }
            }
        }

        Ok(())
    }

    fn binary_to_base64(&self, item: &mut Value, params: &MoveBinaryDataParams) -> Result<(), Error> {
        let source_data = if let Some(source_field) = &params.source_field {
            item.get(source_field).cloned().unwrap_or(Value::Null)
        } else {
            item.clone()
        };

        let data_bytes = if let Some(data_str) = source_data.as_str() {
            data_str.as_bytes().to_vec()
        } else if let Some(data_value) = source_data.get("data") {
            if let Some(data_str) = data_value.as_str() {
                match params.encoding.as_str() {
                    "base64" => general_purpose::STANDARD.decode(data_str)
                        .map_err(|e| Error::NodeExecution(format!("Failed to decode base64: {}", e)))?,
                    "hex" => hex::decode(data_str)
                        .map_err(|e| Error::NodeExecution(format!("Failed to decode hex: {}", e)))?,
                    "utf8" => data_str.as_bytes().to_vec(),
                    _ => return Err(Error::Validation(format!("Unsupported encoding: {}", params.encoding))),
                }
            } else {
                return Err(Error::NodeExecution("Invalid binary data format".to_string()));
            }
        } else {
            return Err(Error::NodeExecution("No data to encode".to_string()));
        };

        let base64_data = general_purpose::STANDARD.encode(&data_bytes);

        if let Some(obj) = item.as_object_mut() {
            obj.insert(params.target_field.clone(), json!(base64_data));
            
            if !params.keep_source {
                if let Some(source_field) = &params.source_field {
                    obj.remove(source_field);
                }
            }
        }

        Ok(())
    }

    fn base64_to_binary(&self, item: &mut Value, params: &MoveBinaryDataParams) -> Result<(), Error> {
        let base64_data = if let Some(source_field) = &params.source_field {
            item.get(source_field).and_then(|v| v.as_str()).unwrap_or("")
        } else {
            item.as_str().unwrap_or("")
        };

        if base64_data.is_empty() {
            return Err(Error::NodeExecution("No base64 data found".to_string()));
        }

        let decoded_bytes = general_purpose::STANDARD.decode(base64_data)
            .map_err(|e| Error::NodeExecution(format!("Failed to decode base64: {}", e)))?;

        let encoded_data = match params.encoding.as_str() {
            "base64" => general_purpose::STANDARD.encode(&decoded_bytes),
            "hex" => hex::encode(&decoded_bytes),
            "utf8" => String::from_utf8(decoded_bytes)
                .map_err(|e| Error::NodeExecution(format!("Failed to convert to UTF-8: {}", e)))?,
            _ => return Err(Error::Validation(format!("Unsupported encoding: {}", params.encoding))),
        };

        let binary_data = json!({
            "data": encoded_data,
            "mimeType": params.options.mime_type.as_ref().unwrap_or(&"application/octet-stream".to_string()),
            "fileName": params.options.file_name.as_ref().unwrap_or(&"data.bin".to_string())
        });

        if let Some(obj) = item.as_object_mut() {
            obj.insert(params.target_field.clone(), binary_data);
            
            if !params.keep_source {
                if let Some(source_field) = &params.source_field {
                    obj.remove(source_field);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_move_binary_data_node_creation() {
        let node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Test Binary Data".to_string(),
            "moveBinaryData".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "mode": "jsonToBinary",
                "encoding": "base64"
            })),
        );

        assert_eq!(node.name(), "Test Binary Data");
        assert!(node.validate());
    }

    #[test]
    fn test_json_to_binary() {
        let node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "JSON to Binary".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "jsonToBinary",
                "encoding": "base64",
                "source_field": "json_data",
                "target_field": "binary_data"
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "json_data": {"message": "Hello World", "number": 42}
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert!(item.get("binary_data").is_some());
            
            let binary_data = &item["binary_data"];
            assert!(binary_data.get("data").is_some());
            assert!(binary_data.get("mimeType").is_some());
            assert!(binary_data.get("fileName").is_some());
        }
    }

    #[test]
    fn test_binary_to_json() {
        let json_data = json!({"message": "Hello World", "number": 42});
        let json_string = serde_json::to_string(&json_data).unwrap();
        let base64_data = general_purpose::STANDARD.encode(json_string.as_bytes());

        let node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Binary to JSON".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "binaryToJson",
                "encoding": "base64",
                "source_field": "binary_data",
                "target_field": "json_result"
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "binary_data": {
                "data": base64_data
            }
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert!(item.get("json_result").is_some());
            
            let json_result = &item["json_result"];
            assert_eq!(json_result["message"], "Hello World");
            assert_eq!(json_result["number"], 42);
        }
    }

    #[test]
    fn test_base64_encoding() {
        let node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Base64 Encode".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "binaryToBase64",
                "encoding": "utf8",
                "source_field": "text_data",
                "target_field": "encoded_data"
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "text_data": "Hello World"
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert!(item.get("encoded_data").is_some());
            
            let encoded = item["encoded_data"].as_str().unwrap();
            let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
            let decoded_text = String::from_utf8(decoded).unwrap();
            assert_eq!(decoded_text, "Hello World");
        }
    }

    #[test]
    fn test_hex_encoding() {
        let node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Hex Test".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "jsonToBinary",
                "encoding": "hex",
                "target_field": "hex_data"
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "message": "test"
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert!(item.get("hex_data").is_some());
            
            let hex_data = &item["hex_data"];
            assert!(hex_data.get("data").is_some());
            let hex_string = hex_data["data"].as_str().unwrap();
            
            // Verify it's valid hex
            assert!(hex::decode(hex_string).is_ok());
        }
    }

    #[test]
    fn test_validation() {
        let valid_node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Valid".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "jsonToBinary",
                "encoding": "base64"
            })),
        );
        assert!(valid_node.validate());

        let invalid_mode_node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Mode".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "invalidMode",
                "encoding": "base64"
            })),
        );
        assert!(!invalid_mode_node.validate());

        let invalid_encoding_node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Encoding".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "jsonToBinary",
                "encoding": "invalidEncoding"
            })),
        );
        assert!(!invalid_encoding_node.validate());
    }

    #[test]
    fn test_keep_source() {
        let node = MoveBinaryDataNode::new(
            uuid::Uuid::new_v4(),
            "Keep Source".to_string(),
            "moveBinaryData".to_string(),
            None,
            Some(json!({
                "mode": "jsonToBinary",
                "encoding": "base64",
                "source_field": "original",
                "target_field": "converted",
                "keep_source": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "original": {"data": "test"}
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            
            // Both source and target should exist
            assert!(item.get("original").is_some());
            assert!(item.get("converted").is_some());
        }
    }
}