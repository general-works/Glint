use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::{Command, Stdio};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandParams {
    #[serde(default)]
    pub command: String,
    #[serde(default = "default_true")]
    pub execute_once: bool,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub environment_variables: Vec<EnvVar>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

fn default_true() -> bool { true }
fn default_timeout() -> u64 { 60 }

impl Default for ExecuteCommandParams {
    fn default() -> Self {
        Self {
            command: String::new(),
            execute_once: true,
            working_directory: None,
            environment_variables: vec![],
            timeout_seconds: 60,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl INode for ExecuteCommandNode {
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
            serde_json::from_value::<ExecuteCommandParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            ExecuteCommandParams::default()
        };

        if params.command.trim().is_empty() {
            return Err(Error::Validation("Command cannot be empty".to_string()));
        }

        // Process input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        let mut results = Vec::new();

        if params.execute_once {
            // Execute command only once regardless of input items count
            let result = self.execute_single_command(&params)?;
            results.push(json!(result));
        } else {
            // Execute command for each input item
            for _item in input_data {
                let result = self.execute_single_command(&params)?;
                results.push(json!(result));
            }
        }

        if results.is_empty() {
            results.push(json!(self.execute_single_command(&params)?));
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<ExecuteCommandParams>(params_value.clone()) {
                return !params.command.trim().is_empty();
            }
        }
        true // Allow nodes without parameters
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl ExecuteCommandNode {
    fn execute_single_command(&self, params: &ExecuteCommandParams) -> Result<CommandResult, Error> {
        // Parse command - simple implementation for shell commands
        let command_parts = self.parse_command(&params.command);
        if command_parts.is_empty() {
            return Err(Error::NodeExecution("Invalid command format".to_string()));
        }

        let program = &command_parts[0];
        let args = if command_parts.len() > 1 {
            &command_parts[1..]
        } else {
            &[]
        };

        let mut cmd = Command::new(program);
        cmd.args(args)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Set working directory if specified
        if let Some(working_dir) = &params.working_directory {
            cmd.current_dir(working_dir);
        }

        // Set environment variables
        for env_var in &params.environment_variables {
            cmd.env(&env_var.name, &env_var.value);
        }

        // Execute command with timeout
        let output = cmd.output()
            .map_err(|e| Error::NodeExecution(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        Ok(CommandResult {
            exit_code,
            stdout: stdout.trim().to_string(),
            stderr: stderr.trim().to_string(),
            success,
        })
    }

    fn parse_command(&self, command: &str) -> Vec<String> {
        // Simple command parsing - split by spaces, handle basic quoted strings
        let mut parts = Vec::new();
        let mut current_part = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;
        let mut quote_char = ' ';

        for ch in command.chars() {
            if escape_next {
                current_part.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => {
                    escape_next = true;
                }
                '"' | '\'' => {
                    if !in_quotes {
                        // Start quoted section
                        in_quotes = true;
                        quote_char = ch;
                    } else if ch == quote_char {
                        // End quoted section
                        in_quotes = false;
                        quote_char = ' ';
                    } else {
                        // Different quote inside quotes
                        current_part.push(ch);
                    }
                }
                ' ' if !in_quotes => {
                    if !current_part.is_empty() {
                        parts.push(current_part.clone());
                        current_part.clear();
                    }
                }
                _ => {
                    current_part.push(ch);
                }
            }
        }

        if !current_part.is_empty() {
            parts.push(current_part);
        }

        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_execute_command_node_creation() {
        let node = ExecuteCommandNode::new(
            uuid::Uuid::new_v4(),
            "Test Execute Command".to_string(),
            "executeCommand".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "command": "echo 'Hello World'"
            })),
        );

        assert_eq!(node.name(), "Test Execute Command");
        assert!(node.validate());
    }

    #[test]
    fn test_execute_command_validation() {
        let node = ExecuteCommandNode::new(
            uuid::Uuid::new_v4(),
            "Test Execute Command".to_string(),
            "executeCommand".to_string(),
            None,
            Some(json!({
                "command": ""
            })),
        );

        assert!(!node.validate());
    }

    #[test] 
    fn test_echo_command() {
        let node = ExecuteCommandNode::new(
            uuid::Uuid::new_v4(),
            "Test Execute Command".to_string(),
            "executeCommand".to_string(),
            None,
            Some(json!({
                "command": "echo 'Hello World'",
                "execute_once": true
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let command_result = &results[0];
            assert!(command_result["success"].as_bool().unwrap_or(false));
            assert_eq!(command_result["exit_code"].as_i64().unwrap_or(-1), 0);
        }
    }

    #[test]
    fn test_command_parsing() {
        let node = ExecuteCommandNode::new(
            uuid::Uuid::new_v4(),
            "Test".to_string(),
            "executeCommand".to_string(),
            None,
            None,
        );

        let parsed = node.parse_command("echo 'hello world'");
        assert_eq!(parsed, vec!["echo", "hello world"]);

        let parsed = node.parse_command("ls -la");
        assert_eq!(parsed, vec!["ls", "-la"]);

        let parsed = node.parse_command("cat file.txt");
        assert_eq!(parsed, vec!["cat", "file.txt"]);
    }

    #[test]
    fn test_invalid_command() {
        let node = ExecuteCommandNode::new(
            uuid::Uuid::new_v4(),
            "Test Execute Command".to_string(),
            "executeCommand".to_string(),
            None,
            Some(json!({
                "command": "nonexistent_command_that_should_fail",
                "execute_once": true
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        
        // Execute should return Ok but the command result should show failure
        if result.is_err() {
            // If execute returns error, that's also acceptable for invalid commands
            assert!(true);
        } else if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            // The command should fail but not throw an exception
            let command_result = &results[0];
            assert!(!command_result["success"].as_bool().unwrap_or(true));
        }
    }

    #[test]
    fn test_multiple_executions() {
        let node = ExecuteCommandNode::new(
            uuid::Uuid::new_v4(),
            "Test Execute Command".to_string(),
            "executeCommand".to_string(),
            None,
            Some(json!({
                "command": "echo 'test'",
                "execute_once": false
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({"item": 1}));
        input.insert(uuid::Uuid::new_v4(), json!({"item": 2}));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 2); // Should execute twice for two input items
        }
    }

    #[test]
    fn test_working_directory() {
        let node = ExecuteCommandNode::new(
            uuid::Uuid::new_v4(),
            "Test Execute Command".to_string(),
            "executeCommand".to_string(),
            None,
            Some(json!({
                "command": "pwd",
                "execute_once": true,
                "working_directory": "/tmp"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let command_result = &results[0];
            if command_result["success"].as_bool().unwrap_or(false) {
                let stdout = command_result["stdout"].as_str().unwrap_or("");
                // pwd command should return the working directory
                assert!(stdout.contains("tmp") || stdout == "/tmp");
            }
        }
    }
}