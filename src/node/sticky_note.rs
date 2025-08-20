use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use chrono::{DateTime, Utc, TimeZone};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyNoteNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyNoteParams {
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_note_type")]
    pub note_type: String, // "info", "warning", "error", "success", "todo", "documentation"
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: Option<String>, // "low", "medium", "high", "critical"
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub color: Option<String>, // Hex color code
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub pass_through: bool, // Whether to pass input data through unchanged
    #[serde(default)]
    pub log_content: bool,  // Whether to log the note content during execution
    #[serde(default)]
    pub metadata: Option<Value>,
}

fn default_note_type() -> String { "info".to_string() }

impl Default for StickyNoteParams {
    fn default() -> Self {
        Self {
            content: String::new(),
            note_type: default_note_type(),
            title: None,
            tags: vec![],
            priority: None,
            author: None,
            created_at: None,
            updated_at: None,
            expires_at: None,
            color: None,
            visible: true,
            pass_through: true,
            log_content: false,
            metadata: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StickyNoteInfo {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub note_type: String,
    pub priority: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub color: Option<String>,
    pub visible: bool,
    pub is_expired: bool,
    pub metadata: Option<Value>,
    pub workflow_context: WorkflowContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub node_name: String,
    pub execution_time: String,
    pub execution_timestamp: i64,
    pub workflow_step: Option<String>,
}

impl INode for StickyNoteNode {
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
            serde_json::from_value::<StickyNoteParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            StickyNoteParams::default()
        };

        let now = Utc::now();
        
        // Create sticky note info
        let note_info = self.create_note_info(&params, &now)?;
        
        // Log content if requested
        if params.log_content && !params.content.is_empty() {
            println!("[STICKY NOTE] {}: {}", 
                params.title.as_deref().unwrap_or("Note"),
                params.content
            );
        }

        // Handle pass-through behavior
        if params.pass_through {
            let input_data: Vec<Value> = input.values().cloned().collect();
            
            if input_data.is_empty() {
                // No input data, return just the note
                Ok(json!({
                    "sticky_note": note_info,
                    "note_content": params.content,
                    "note_type": params.note_type
                }))
            } else {
                // Pass through input data with note attached
                let mut results = Vec::new();
                
                for mut item in input_data {
                    if let Some(obj) = item.as_object_mut() {
                        obj.insert("_sticky_note".to_string(), json!(note_info));
                        obj.insert("_note_content".to_string(), json!(params.content));
                        obj.insert("_note_type".to_string(), json!(params.note_type));
                    }
                    results.push(item);
                }
                
                Ok(Value::Array(results))
            }
        } else {
            // Return only the note information
            Ok(json!({
                "sticky_note": note_info,
                "note_content": params.content,
                "note_type": params.note_type,
                "message": format!("Sticky note: {}", 
                    params.title.as_deref().unwrap_or(&params.content))
            }))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<StickyNoteParams>(params_value.clone()) {
                let valid_types = ["info", "warning", "error", "success", "todo", "documentation"];
                let valid_priorities = ["low", "medium", "high", "critical"];
                
                // Validate note type
                if !valid_types.contains(&params.note_type.as_str()) {
                    return false;
                }
                
                // Validate priority if provided
                if let Some(priority) = &params.priority {
                    if !valid_priorities.contains(&priority.as_str()) {
                        return false;
                    }
                }
                
                // Validate color format if provided (basic hex check)
                if let Some(color) = &params.color {
                    if !self.is_valid_hex_color(color) {
                        return false;
                    }
                }
                
                // Validate datetime formats
                if let Some(created_at) = &params.created_at {
                    if self.parse_datetime(created_at).is_err() {
                        return false;
                    }
                }
                
                if let Some(updated_at) = &params.updated_at {
                    if self.parse_datetime(updated_at).is_err() {
                        return false;
                    }
                }
                
                if let Some(expires_at) = &params.expires_at {
                    if self.parse_datetime(expires_at).is_err() {
                        return false;
                    }
                }
                
                return true;
            }
        }
        true
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl StickyNoteNode {
    fn create_note_info(&self, params: &StickyNoteParams, execution_time: &DateTime<Utc>) -> Result<StickyNoteInfo, Error> {
        let now = execution_time.to_rfc3339();
        
        let created_at = if let Some(created) = &params.created_at {
            created.clone()
        } else {
            now.clone()
        };
        
        let updated_at = if let Some(updated) = &params.updated_at {
            updated.clone()
        } else {
            now.clone()
        };
        
        let is_expired = if let Some(expires) = &params.expires_at {
            match self.parse_datetime(expires) {
                Ok(expiry_time) => *execution_time > expiry_time,
                Err(_) => false,
            }
        } else {
            false
        };
        
        let workflow_context = WorkflowContext {
            node_name: self.name.clone(),
            execution_time: now.clone(),
            execution_timestamp: execution_time.timestamp(),
            workflow_step: None, // Could be enhanced to track workflow step
        };
        
        Ok(StickyNoteInfo {
            id: self.id.to_string(),
            title: params.title.clone(),
            content: params.content.clone(),
            note_type: params.note_type.clone(),
            priority: params.priority.clone(),
            tags: params.tags.clone(),
            author: params.author.clone(),
            created_at,
            updated_at,
            expires_at: params.expires_at.clone(),
            color: params.color.clone(),
            visible: params.visible,
            is_expired,
            metadata: params.metadata.clone(),
            workflow_context,
        })
    }
    
    fn parse_datetime(&self, datetime_str: &str) -> Result<DateTime<Utc>, Error> {
        // Try parsing as RFC3339 first
        if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try parsing as naive datetime
        if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }

        // Try parsing as date only
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(datetime_str, "%Y-%m-%d") {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }

        Err(Error::Validation(format!("Invalid datetime format: {}", datetime_str)))
    }
    
    fn is_valid_hex_color(&self, color: &str) -> bool {
        if !color.starts_with('#') {
            return false;
        }
        
        let hex_part = &color[1..];
        if hex_part.len() != 3 && hex_part.len() != 6 {
            return false;
        }
        
        hex_part.chars().all(|c| c.is_ascii_hexdigit())
    }
    
    pub fn format_note_for_display(&self, params: &StickyNoteParams) -> String {
        let title = params.title.as_deref().unwrap_or("Note");
        let priority_indicator = match params.priority.as_deref() {
            Some("critical") => "🔴 ",
            Some("high") => "🟠 ",
            Some("medium") => "🟡 ",
            Some("low") => "🟢 ",
            _ => "",
        };
        
        let type_indicator = match params.note_type.as_str() {
            "warning" => "⚠️  ",
            "error" => "❌ ",
            "success" => "✅ ",
            "todo" => "📝 ",
            "documentation" => "📚 ",
            _ => "ℹ️  ",
        };
        
        let tags_str = if params.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", params.tags.join(", "))
        };
        
        format!("{}{}{}{}\n{}", 
            priority_indicator, 
            type_indicator, 
            title, 
            tags_str,
            params.content
        )
    }
    
    pub fn get_notes_by_type<'a>(notes: &'a [StickyNoteInfo], note_type: &str) -> Vec<&'a StickyNoteInfo> {
        notes.iter()
            .filter(|note| note.note_type == note_type && note.visible && !note.is_expired)
            .collect()
    }
    
    pub fn get_notes_by_priority<'a>(notes: &'a [StickyNoteInfo], priority: &str) -> Vec<&'a StickyNoteInfo> {
        notes.iter()
            .filter(|note| {
                note.priority.as_deref() == Some(priority) && note.visible && !note.is_expired
            })
            .collect()
    }
    
    pub fn search_notes<'a>(notes: &'a [StickyNoteInfo], query: &str) -> Vec<&'a StickyNoteInfo> {
        let query_lower = query.to_lowercase();
        notes.iter()
            .filter(|note| {
                note.visible && !note.is_expired && (
                    note.content.to_lowercase().contains(&query_lower) ||
                    note.title.as_ref().map_or(false, |t| t.to_lowercase().contains(&query_lower)) ||
                    note.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_sticky_note_node_creation() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Test Sticky Note".to_string(),
            "stickyNote".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "content": "This is a test note",
                "note_type": "info"
            })),
        );

        assert_eq!(node.name(), "Test Sticky Note");
        assert!(node.validate());
    }

    #[test]
    fn test_basic_note() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Basic Note".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "content": "This workflow processes user data",
                "note_type": "documentation",
                "title": "Workflow Documentation",
                "pass_through": false
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert!(output.get("sticky_note").is_some());
            assert_eq!(output["note_content"], "This workflow processes user data");
            assert_eq!(output["note_type"], "documentation");
            
            let note_info = &output["sticky_note"];
            assert_eq!(note_info["title"], "Workflow Documentation");
            assert_eq!(note_info["content"], "This workflow processes user data");
            assert_eq!(note_info["note_type"], "documentation");
        }
    }

    #[test]
    fn test_pass_through_behavior() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Pass Through Note".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "content": "Processing step completed",
                "note_type": "success",
                "pass_through": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(uuid::Uuid::new_v4(), json!({
            "user_id": 123,
            "name": "John Doe"
        }));

        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            
            // Original data should be preserved
            assert_eq!(item["user_id"], 123);
            assert_eq!(item["name"], "John Doe");
            
            // Note data should be added
            assert!(item.get("_sticky_note").is_some());
            assert_eq!(item["_note_content"], "Processing step completed");
            assert_eq!(item["_note_type"], "success");
        }
    }

    #[test]
    fn test_warning_note() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Warning Note".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "content": "This operation may take a long time",
                "note_type": "warning",
                "title": "Performance Warning",
                "priority": "high",
                "tags": ["performance", "warning"]
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let note_info = &output["sticky_note"];
            assert_eq!(note_info["note_type"], "warning");
            assert_eq!(note_info["priority"], "high");
            assert!(note_info["tags"].as_array().unwrap().contains(&json!("performance")));
        }
    }

    #[test]
    fn test_todo_note() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "TODO Note".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "content": "Add input validation for email field",
                "note_type": "todo",
                "title": "Enhancement Task",
                "priority": "medium",
                "author": "Developer",
                "tags": ["validation", "enhancement"]
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let note_info = &output["sticky_note"];
            assert_eq!(note_info["note_type"], "todo");
            assert_eq!(note_info["author"], "Developer");
        }
    }

    #[test]
    fn test_note_with_expiration() {
        let future_date = "2025-12-31T23:59:59Z";
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Expiring Note".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "content": "Temporary note for testing",
                "note_type": "info",
                "expires_at": future_date
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let note_info = &output["sticky_note"];
            assert_eq!(note_info["expires_at"], future_date);
            assert_eq!(note_info["is_expired"], false);
        }
    }

    #[test]
    fn test_note_with_color() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Colored Note".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "content": "Important reminder",
                "note_type": "info",
                "color": "#FF5733",
                "title": "Important"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let note_info = &output["sticky_note"];
            assert_eq!(note_info["color"], "#FF5733");
        }
    }

    #[test]
    fn test_validation() {
        // Valid note
        let valid_node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Valid Note".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "content": "Valid note content",
                "note_type": "info",
                "priority": "medium",
                "color": "#123456"
            })),
        );
        assert!(valid_node.validate());

        // Invalid note type
        let invalid_type = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Type".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "note_type": "invalid_type"
            })),
        );
        assert!(!invalid_type.validate());

        // Invalid priority
        let invalid_priority = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Priority".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "priority": "invalid_priority"
            })),
        );
        assert!(!invalid_priority.validate());

        // Invalid color
        let invalid_color = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Color".to_string(),
            "stickyNote".to_string(),
            None,
            Some(json!({
                "color": "not-a-hex-color"
            })),
        );
        assert!(!invalid_color.validate());
    }

    #[test]
    fn test_hex_color_validation() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Test".to_string(),
            "stickyNote".to_string(),
            None,
            None,
        );

        // Valid hex colors
        assert!(node.is_valid_hex_color("#123456"));
        assert!(node.is_valid_hex_color("#ABC"));
        assert!(node.is_valid_hex_color("#fff"));
        assert!(node.is_valid_hex_color("#000000"));

        // Invalid hex colors
        assert!(!node.is_valid_hex_color("123456")); // Missing #
        assert!(!node.is_valid_hex_color("#12")); // Too short
        assert!(!node.is_valid_hex_color("#1234567")); // Too long
        assert!(!node.is_valid_hex_color("#GGG")); // Invalid characters
        assert!(!node.is_valid_hex_color("#12G456")); // Invalid character
    }

    #[test]
    fn test_format_note_for_display() {
        let node = StickyNoteNode::new(
            uuid::Uuid::new_v4(),
            "Test".to_string(),
            "stickyNote".to_string(),
            None,
            None,
        );

        let params = StickyNoteParams {
            content: "This is a test note content".to_string(),
            note_type: "warning".to_string(),
            title: Some("Test Warning".to_string()),
            priority: Some("high".to_string()),
            tags: vec!["test".to_string(), "demo".to_string()],
            ..Default::default()
        };

        let display = node.format_note_for_display(&params);
        assert!(display.contains("🟠")); // High priority indicator
        assert!(display.contains("⚠️")); // Warning type indicator
        assert!(display.contains("Test Warning"));
        assert!(display.contains("[test, demo]"));
        assert!(display.contains("This is a test note content"));
    }

    #[test]
    fn test_note_filtering() {
        let notes = vec![
            StickyNoteInfo {
                id: "1".to_string(),
                title: Some("Note 1".to_string()),
                content: "Error in processing".to_string(),
                note_type: "error".to_string(),
                priority: Some("high".to_string()),
                tags: vec!["error".to_string()],
                visible: true,
                is_expired: false,
                author: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                expires_at: None,
                color: None,
                metadata: None,
                workflow_context: WorkflowContext {
                    node_name: "test".to_string(),
                    execution_time: "2024-01-01T00:00:00Z".to_string(),
                    execution_timestamp: 0,
                    workflow_step: None,
                },
            },
            StickyNoteInfo {
                id: "2".to_string(),
                title: Some("Note 2".to_string()),
                content: "Information note".to_string(),
                note_type: "info".to_string(),
                priority: Some("low".to_string()),
                tags: vec!["info".to_string()],
                visible: true,
                is_expired: false,
                author: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                expires_at: None,
                color: None,
                metadata: None,
                workflow_context: WorkflowContext {
                    node_name: "test".to_string(),
                    execution_time: "2024-01-01T00:00:00Z".to_string(),
                    execution_timestamp: 0,
                    workflow_step: None,
                },
            },
        ];

        // Filter by type
        let error_notes = StickyNoteNode::get_notes_by_type(&notes, "error");
        assert_eq!(error_notes.len(), 1);
        assert_eq!(error_notes[0].note_type, "error");

        // Filter by priority
        let high_priority = StickyNoteNode::get_notes_by_priority(&notes, "high");
        assert_eq!(high_priority.len(), 1);
        assert_eq!(high_priority[0].priority.as_ref().unwrap(), "high");

        // Search notes
        let search_results = StickyNoteNode::search_notes(&notes, "processing");
        assert_eq!(search_results.len(), 1);
        assert!(search_results[0].content.contains("processing"));
    }
}