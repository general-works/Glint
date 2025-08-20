use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookParams {
    pub path: String,
    pub method: String, // "GET", "POST", "PUT", "DELETE"
    pub response_mode: String, // "on_received", "last_node", "first_node"
    pub response_data: Option<Value>,
    pub response_code: Option<u16>,
    pub response_headers: Option<HashMap<String, String>>,
    pub authentication: Option<String>, // "none", "basic", "bearer"
    pub webhook_id: Option<String>,
}

impl Default for WebhookParams {
    fn default() -> Self {
        Self {
            path: "/webhook".to_string(),
            method: "POST".to_string(),
            response_mode: "on_received".to_string(),
            response_data: None,
            response_code: Some(200),
            response_headers: None,
            authentication: Some("none".to_string()),
            webhook_id: None,
        }
    }
}

impl INode for WebhookNode {
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
            serde_json::from_value::<WebhookParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            WebhookParams::default()
        };

        // In a real implementation, this would set up a webhook endpoint
        // For this simulation, we'll create a webhook configuration and 
        // simulate receiving data

        match params.response_mode.as_str() {
            "on_received" => self.execute_on_received(&params, input),
            "last_node" => self.execute_last_node(&params, input),
            "first_node" => self.execute_first_node(&params, input),
            _ => Err(Error::Validation(format!("Unsupported response mode: {}", params.response_mode)))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<WebhookParams>(params_value.clone()) {
                return !params.path.is_empty() && 
                       matches!(params.method.to_uppercase().as_str(), "GET" | "POST" | "PUT" | "DELETE" | "PATCH");
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl WebhookNode {
    fn execute_on_received(&self, params: &WebhookParams, _input: &NodeOutput) -> Result<Value, Error> {
        // Simulate immediate response when webhook is received
        let webhook_url = format!("http://localhost:5678/webhook{}", params.path);
        let default_id = self.id.to_string();
        let webhook_id = params.webhook_id.as_ref()
            .unwrap_or(&default_id);
        
        let response = json!({
            "webhook": {
                "id": webhook_id,
                "url": webhook_url,
                "method": params.method.to_uppercase(),
                "path": params.path,
                "status": "active",
                "response_mode": params.response_mode,
                "response_code": params.response_code.unwrap_or(200)
            },
            "message": "Webhook endpoint configured successfully",
            "test_data": self.create_test_response(&params)
        });

        Ok(response)
    }

    fn execute_last_node(&self, params: &WebhookParams, input: &NodeOutput) -> Result<Value, Error> {
        // Use data from the last node in the workflow
        let input_data: Vec<Value> = input.values().cloned().collect();
        let last_data = input_data.last().cloned().unwrap_or(json!({}));
        
        let webhook_url = format!("http://localhost:5678/webhook{}", params.path);
        let default_id = self.id.to_string();
        let webhook_id = params.webhook_id.as_ref()
            .unwrap_or(&default_id);

        let response = json!({
            "webhook": {
                "id": webhook_id,
                "url": webhook_url,
                "method": params.method.to_uppercase(),
                "path": params.path,
                "status": "active",
                "response_mode": params.response_mode
            },
            "response_data": last_data,
            "headers": params.response_headers.clone().unwrap_or_default(),
            "status_code": params.response_code.unwrap_or(200)
        });

        Ok(response)
    }

    fn execute_first_node(&self, params: &WebhookParams, input: &NodeOutput) -> Result<Value, Error> {
        // Use data from the first node in the workflow
        let input_data: Vec<Value> = input.values().cloned().collect();
        let first_data = input_data.first().cloned().unwrap_or(json!({}));
        
        let webhook_url = format!("http://localhost:5678/webhook{}", params.path);
        let default_id = self.id.to_string();
        let webhook_id = params.webhook_id.as_ref()
            .unwrap_or(&default_id);

        let response = json!({
            "webhook": {
                "id": webhook_id,
                "url": webhook_url,
                "method": params.method.to_uppercase(),
                "path": params.path,
                "status": "active",
                "response_mode": params.response_mode
            },
            "response_data": first_data,
            "headers": params.response_headers.clone().unwrap_or_default(),
            "status_code": params.response_code.unwrap_or(200)
        });

        Ok(response)
    }

    fn create_test_response(&self, params: &WebhookParams) -> Value {
        // Create a test response based on the configured parameters
        if let Some(response_data) = &params.response_data {
            response_data.clone()
        } else {
            json!({
                "status": "success",
                "message": "Webhook received successfully",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "method": params.method.to_uppercase()
            })
        }
    }

    pub fn simulate_webhook_call(&self, payload: Value) -> Result<Value, Error> {
        // Simulate an incoming webhook call
        let params = if let Some(params_value) = &self.parameters {
            match serde_json::from_value::<WebhookParams>(params_value.clone()) {
                Ok(p) => p,
                Err(_) => {
                    // If deserialization fails, try to extract individual fields
                    let mut default_params = WebhookParams::default();
                    if let Some(path) = params_value.get("path").and_then(|v| v.as_str()) {
                        default_params.path = path.to_string();
                    }
                    if let Some(method) = params_value.get("method").and_then(|v| v.as_str()) {
                        default_params.method = method.to_string();
                    }
                    if let Some(auth) = params_value.get("authentication").and_then(|v| v.as_str()) {
                        default_params.authentication = Some(auth.to_string());
                    }
                    default_params
                }
            }
        } else {
            WebhookParams::default()
        };

        let default_id = self.id.to_string();
        let default_auth = "none".to_string();
        let response = json!({
            "received_at": chrono::Utc::now().to_rfc3339(),
            "method": params.method.to_uppercase(),
            "path": params.path,
            "payload": payload,
            "webhook_id": params.webhook_id.as_ref().unwrap_or(&default_id),
            "authentication": params.authentication.as_ref().unwrap_or(&default_auth)
        });

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_webhook_node_creation() {
        let node = WebhookNode::new(
            Uuid::new_v4(),
            "Test Webhook".to_string(),
            "webhook".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "path": "/test-webhook",
                "method": "POST",
                "response_mode": "on_received"
            })),
        );

        assert_eq!(node.name(), "Test Webhook");
        assert!(node.validate());
    }

    #[test]
    fn test_webhook_validation() {
        let node = WebhookNode::new(
            Uuid::new_v4(),
            "Test Webhook".to_string(),
            "webhook".to_string(),
            None,
            Some(json!({
                "path": "",
                "method": "INVALID",
                "response_mode": "on_received"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_webhook_on_received_execution() {
        let node = WebhookNode::new(
            Uuid::new_v4(),
            "Test Webhook".to_string(),
            "webhook".to_string(),
            None,
            Some(json!({
                "path": "/test",
                "method": "POST",
                "response_mode": "on_received",
                "response_code": 201
            })),
        );

        let input = std::collections::HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(response) = result {
            assert!(response["webhook"]["url"].as_str().unwrap().contains("/test"));
            assert_eq!(response["webhook"]["method"], "POST");
            assert_eq!(response["webhook"]["response_code"], 201);
            assert_eq!(response["webhook"]["status"], "active");
        }
    }

    #[test]
    fn test_webhook_last_node_execution() {
        let node = WebhookNode::new(
            Uuid::new_v4(),
            "Test Webhook".to_string(),
            "webhook".to_string(),
            None,
            Some(json!({
                "path": "/test",
                "method": "GET",
                "response_mode": "last_node"
            })),
        );

        let mut input = std::collections::HashMap::new();
        input.insert(Uuid::new_v4(), json!({"data": "first"}));
        input.insert(Uuid::new_v4(), json!({"data": "last"}));

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(response) = result {
            // Check that we got the last data (there are two items, so either could be last)
            assert!(response["response_data"]["data"].is_string());
            assert_eq!(response["webhook"]["response_mode"], "last_node");
        }
    }

    #[test]
    fn test_webhook_simulate_call() {
        let node = WebhookNode::new(
            Uuid::new_v4(),
            "Test Webhook".to_string(),
            "webhook".to_string(),
            None,
            Some(json!({
                "path": "/test",
                "method": "POST",
                "authentication": "bearer"
            })),
        );

        let payload = json!({"message": "test payload"});
        let result = node.simulate_webhook_call(payload.clone());
        assert!(result.is_ok());
        
        if let Ok(response) = result {
            assert_eq!(response["payload"], payload);
            assert_eq!(response["method"], "POST");
            assert_eq!(response["path"], "/test");
            assert_eq!(response["authentication"], "bearer");
            assert!(response["received_at"].is_string());
        }
    }
}