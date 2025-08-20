use std::collections::HashMap;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestParams {
    pub method: String,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
    pub body: Option<Value>,
    pub timeout: Option<u64>,
    pub response_format: Option<String>, // "json", "text", "file"
    pub full_response: Option<bool>,
}

impl Default for HttpRequestParams {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            url: String::new(),
            headers: None,
            query_params: None,
            body: None,
            timeout: Some(30), // 30 seconds default
            response_format: Some("json".to_string()),
            full_response: Some(false),
        }
    }
}

impl INode for HttpRequestNode {
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

    fn execute(&self, _input: &NodeOutput) -> Result<Value, Error> {
        let params = if let Some(params_value) = &self.parameters {
            serde_json::from_value::<HttpRequestParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            HttpRequestParams::default()
        };

        // Validate required parameters
        if params.url.is_empty() {
            return Err(Error::Validation("URL is required".to_string()));
        }

        // Create HTTP client
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(params.timeout.unwrap_or(30)))
            .build()
            .map_err(|e| Error::NodeExecution(format!("Failed to create HTTP client: {}", e)))?;

        // Build request
        let method = match params.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => return Err(Error::Validation(format!("Unsupported HTTP method: {}", params.method))),
        };

        let mut request_builder = client.request(method, &params.url);

        // Add headers
        if let Some(headers) = &params.headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }

        // Add query parameters
        if let Some(query_params) = &params.query_params {
            for (key, value) in query_params {
                request_builder = request_builder.query(&[(key, value)]);
            }
        }

        // Add body for methods that support it
        if matches!(params.method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH") {
            if let Some(body) = &params.body {
                match body {
                    Value::String(s) => {
                        request_builder = request_builder.body(s.clone());
                    }
                    _ => {
                        request_builder = request_builder.json(body);
                    }
                }
            }
        }

        // Execute request
        let response = request_builder
            .send()
            .map_err(|e| Error::NodeExecution(format!("HTTP request failed: {}", e)))?;

        // Get response data
        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or("").to_string(),
                )
            })
            .collect();

        let response_format = params.response_format.unwrap_or_else(|| "json".to_string());
        let full_response = params.full_response.unwrap_or(false);

        // Handle response based on format
        let body_value = match response_format.as_str() {
            "json" => {
                let text = response
                    .text()
                    .map_err(|e| Error::NodeExecution(format!("Failed to read response body: {}", e)))?;
                
                if text.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_str(&text)
                        .unwrap_or_else(|_| Value::String(text))
                }
            }
            "text" => {
                let text = response
                    .text()
                    .map_err(|e| Error::NodeExecution(format!("Failed to read response body: {}", e)))?;
                Value::String(text)
            }
            "file" => {
                let bytes = response
                    .bytes()
                    .map_err(|e| Error::NodeExecution(format!("Failed to read response bytes: {}", e)))?;
                
                // For file responses, we encode as base64 for JSON serialization
                let base64_data = general_purpose::STANDARD.encode(&bytes);
                json!({
                    "data": base64_data,
                    "contentType": headers.get("content-type").unwrap_or(&"application/octet-stream".to_string()),
                    "size": bytes.len()
                })
            }
            _ => {
                return Err(Error::Validation(format!("Unsupported response format: {}", response_format)));
            }
        };

        // Return response based on full_response setting
        let result = if full_response {
            json!({
                "body": body_value,
                "headers": headers,
                "statusCode": status_code,
                "statusMessage": status_code.to_string()
            })
        } else {
            body_value
        };

        Ok(result)
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<HttpRequestParams>(params_value.clone()) {
                return !params.url.is_empty();
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_http_request_node_creation() {
        let node = HttpRequestNode::new(
            Uuid::new_v4(),
            "Test HTTP Request".to_string(),
            "http_request".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "method": "GET",
                "url": "https://httpbin.org/get"
            })),
        );

        assert_eq!(node.name(), "Test HTTP Request");
        assert!(node.validate());
    }

    #[test]
    fn test_http_request_validation() {
        let node = HttpRequestNode::new(
            Uuid::new_v4(),
            "Test HTTP Request".to_string(),
            "http_request".to_string(),
            None,
            Some(json!({
                "method": "GET",
                "url": ""
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_http_request_execute_get() {
        let node = HttpRequestNode::new(
            Uuid::new_v4(),
            "Test HTTP Request".to_string(),
            "http_request".to_string(),
            None,
            Some(json!({
                "method": "GET",
                "url": "https://httpbin.org/get",
                "response_format": "json"
            })),
        );

        let output = NodeOutput::new();
        let result = node.execute(&output);
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_request_execute_post() {
        let node = HttpRequestNode::new(
            Uuid::new_v4(),
            "Test HTTP Request".to_string(),
            "http_request".to_string(),
            None,
            Some(json!({
                "method": "POST",
                "url": "https://httpbin.org/post",
                "body": {"test": "data"},
                "response_format": "json"
            })),
        );

        let output = NodeOutput::new();
        let result = node.execute(&output);
        assert!(result.is_ok());
    }
}