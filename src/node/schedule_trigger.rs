use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use chrono::{DateTime, Utc, Duration, NaiveDateTime, TimeZone};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTriggerNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTriggerParams {
    #[serde(default = "default_schedule_type")]
    pub schedule_type: String, // "interval", "cron", "once", "daily", "weekly", "monthly"
    #[serde(default)]
    pub interval_seconds: Option<u64>,
    #[serde(default)]
    pub cron_expression: Option<String>,
    #[serde(default)]
    pub start_time: Option<String>, // ISO 8601 format
    #[serde(default)]
    pub end_time: Option<String>,   // ISO 8601 format
    #[serde(default)]
    pub timezone: Option<String>,   // e.g., "UTC", "America/New_York"
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_executions: Option<u64>,
    #[serde(default)]
    pub retry_on_failure: bool,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u64,
    #[serde(default)]
    pub trigger_data: Option<Value>, // Data to pass when triggered
}

fn default_schedule_type() -> String { "once".to_string() }
fn default_retry_attempts() -> u32 { 3 }
fn default_retry_delay() -> u64 { 60 }

impl Default for ScheduleTriggerParams {
    fn default() -> Self {
        Self {
            schedule_type: default_schedule_type(),
            interval_seconds: None,
            cron_expression: None,
            start_time: None,
            end_time: None,
            timezone: None,
            enabled: true,
            max_executions: None,
            retry_on_failure: false,
            retry_attempts: default_retry_attempts(),
            retry_delay_seconds: default_retry_delay(),
            trigger_data: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleInfo {
    pub schedule_type: String,
    pub next_execution: Option<String>,
    pub last_execution: Option<String>,
    pub execution_count: u64,
    pub enabled: bool,
    pub status: String, // "active", "paused", "completed", "failed"
    pub next_execution_timestamp: Option<i64>,
    pub schedule_description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerEvent {
    pub trigger_time: String,
    pub trigger_timestamp: i64,
    pub execution_count: u64,
    pub schedule_type: String,
    pub data: Option<Value>,
}

impl INode for ScheduleTriggerNode {
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
            serde_json::from_value::<ScheduleTriggerParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            ScheduleTriggerParams::default()
        };

        if !params.enabled {
            return Ok(json!({
                "status": "disabled",
                "message": "Schedule trigger is disabled"
            }));
        }

        // Calculate schedule information
        let schedule_info = self.calculate_schedule_info(&params)?;
        
        // Create trigger event for current execution
        let trigger_event = self.create_trigger_event(&params, &schedule_info)?;

        // Return comprehensive schedule information
        Ok(json!({
            "trigger_event": trigger_event,
            "schedule_info": schedule_info,
            "trigger_data": params.trigger_data.unwrap_or(json!({}))
        }))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<ScheduleTriggerParams>(params_value.clone()) {
                let valid_types = ["interval", "cron", "once", "daily", "weekly", "monthly"];
                
                if !valid_types.contains(&params.schedule_type.as_str()) {
                    return false;
                }

                // Validate interval
                if params.schedule_type == "interval" && params.interval_seconds.is_none() {
                    return false;
                }

                // Validate cron expression format (basic check)
                if params.schedule_type == "cron" {
                    if let Some(cron) = &params.cron_expression {
                        if !self.is_valid_cron_expression(cron) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Validate time formats
                if let Some(start_time) = &params.start_time {
                    if self.parse_datetime(start_time).is_err() {
                        return false;
                    }
                }

                if let Some(end_time) = &params.end_time {
                    if self.parse_datetime(end_time).is_err() {
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

impl ScheduleTriggerNode {
    fn calculate_schedule_info(&self, params: &ScheduleTriggerParams) -> Result<ScheduleInfo, Error> {
        let now = Utc::now();
        let next_execution = self.calculate_next_execution(params, &now)?;
        
        let schedule_description = self.generate_schedule_description(params);
        
        Ok(ScheduleInfo {
            schedule_type: params.schedule_type.clone(),
            next_execution: next_execution.map(|dt| dt.to_rfc3339()),
            last_execution: None, // This would be tracked in a real scheduler
            execution_count: 0,   // This would be tracked in a real scheduler
            enabled: params.enabled,
            status: if params.enabled { "active".to_string() } else { "paused".to_string() },
            next_execution_timestamp: next_execution.map(|dt| dt.timestamp()),
            schedule_description,
        })
    }

    fn calculate_next_execution(&self, params: &ScheduleTriggerParams, from_time: &DateTime<Utc>) -> Result<Option<DateTime<Utc>>, Error> {
        match params.schedule_type.as_str() {
            "once" => {
                if let Some(start_time) = &params.start_time {
                    let start_dt = self.parse_datetime(start_time)?;
                    if start_dt > *from_time {
                        Ok(Some(start_dt))
                    } else {
                        Ok(None) // Already passed
                    }
                } else {
                    Ok(Some(*from_time)) // Execute immediately
                }
            }
            "interval" => {
                if let Some(interval_seconds) = params.interval_seconds {
                    let interval = Duration::seconds(interval_seconds as i64);
                    let next_time = *from_time + interval;
                    
                    // Check end_time constraint
                    if let Some(end_time) = &params.end_time {
                        let end_dt = self.parse_datetime(end_time)?;
                        if next_time <= end_dt {
                            Ok(Some(next_time))
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(Some(next_time))
                    }
                } else {
                    Err(Error::Validation("Interval seconds not specified".to_string()))
                }
            }
            "daily" => {
                let next_day = *from_time + Duration::days(1);
                let daily_time = next_day.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let daily_dt = Utc.from_utc_datetime(&daily_time);
                Ok(Some(daily_dt))
            }
            "weekly" => {
                let next_week = *from_time + Duration::weeks(1);
                let weekly_time = next_week.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let weekly_dt = Utc.from_utc_datetime(&weekly_time);
                Ok(Some(weekly_dt))
            }
            "monthly" => {
                // Simple monthly calculation (add 30 days)
                let next_month = *from_time + Duration::days(30);
                let monthly_time = next_month.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let monthly_dt = Utc.from_utc_datetime(&monthly_time);
                Ok(Some(monthly_dt))
            }
            "cron" => {
                if let Some(_cron_expr) = &params.cron_expression {
                    // Simplified cron calculation - in a real implementation, you'd use a cron library
                    let next_time = *from_time + Duration::hours(1); // Default to hourly
                    Ok(Some(next_time))
                } else {
                    Err(Error::Validation("Cron expression not specified".to_string()))
                }
            }
            _ => Err(Error::Validation(format!("Unsupported schedule type: {}", params.schedule_type))),
        }
    }

    fn create_trigger_event(&self, params: &ScheduleTriggerParams, schedule_info: &ScheduleInfo) -> Result<TriggerEvent, Error> {
        let now = Utc::now();
        
        Ok(TriggerEvent {
            trigger_time: now.to_rfc3339(),
            trigger_timestamp: now.timestamp(),
            execution_count: schedule_info.execution_count + 1,
            schedule_type: params.schedule_type.clone(),
            data: params.trigger_data.clone(),
        })
    }

    fn generate_schedule_description(&self, params: &ScheduleTriggerParams) -> String {
        match params.schedule_type.as_str() {
            "once" => {
                if let Some(start_time) = &params.start_time {
                    format!("Execute once at {}", start_time)
                } else {
                    "Execute once immediately".to_string()
                }
            }
            "interval" => {
                if let Some(interval) = params.interval_seconds {
                    format!("Execute every {} seconds", interval)
                } else {
                    "Execute at intervals".to_string()
                }
            }
            "daily" => "Execute daily at midnight UTC".to_string(),
            "weekly" => "Execute weekly".to_string(),
            "monthly" => "Execute monthly".to_string(),
            "cron" => {
                if let Some(cron) = &params.cron_expression {
                    format!("Execute based on cron: {}", cron)
                } else {
                    "Execute based on cron expression".to_string()
                }
            }
            _ => format!("Execute using {} schedule", params.schedule_type),
        }
    }

    fn parse_datetime(&self, datetime_str: &str) -> Result<DateTime<Utc>, Error> {
        // Try parsing as RFC3339 first
        if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try parsing as naive datetime
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }

        // Try parsing as date only
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(datetime_str, "%Y-%m-%d") {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }

        Err(Error::Validation(format!("Invalid datetime format: {}", datetime_str)))
    }

    fn is_valid_cron_expression(&self, cron_expr: &str) -> bool {
        // Basic cron validation - should have 5 or 6 parts separated by spaces
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        if parts.len() < 5 || parts.len() > 6 {
            return false;
        }

        // Each part should be valid (very basic check)
        for part in &parts {
            if part.is_empty() {
                return false;
            }
            // Allow numbers, ranges, lists, and wildcards
            if !part.chars().all(|c| c.is_ascii_digit() || c == '*' || c == '-' || c == ',' || c == '/') {
                return false;
            }
        }

        true
    }

    pub fn should_execute_now(&self, params: &ScheduleTriggerParams, last_execution: Option<DateTime<Utc>>) -> Result<bool, Error> {
        if !params.enabled {
            return Ok(false);
        }

        let now = Utc::now();

        // Check if we're within the valid time window
        if let Some(start_time) = &params.start_time {
            let start_dt = self.parse_datetime(start_time)?;
            if now < start_dt {
                return Ok(false);
            }
        }

        if let Some(end_time) = &params.end_time {
            let end_dt = self.parse_datetime(end_time)?;
            if now > end_dt {
                return Ok(false);
            }
        }

        // Check schedule-specific conditions
        match params.schedule_type.as_str() {
            "once" => {
                // Execute only if never executed before
                Ok(last_execution.is_none())
            }
            "interval" => {
                if let Some(interval_seconds) = params.interval_seconds {
                    if let Some(last_exec) = last_execution {
                        let interval_duration = Duration::seconds(interval_seconds as i64);
                        Ok(now >= last_exec + interval_duration)
                    } else {
                        Ok(true) // First execution
                    }
                } else {
                    Ok(false)
                }
            }
            "daily" | "weekly" | "monthly" => {
                // Simplified check - in real implementation, you'd check specific time boundaries
                if let Some(last_exec) = last_execution {
                    let threshold = match params.schedule_type.as_str() {
                        "daily" => Duration::days(1),
                        "weekly" => Duration::weeks(1),
                        "monthly" => Duration::days(30),
                        _ => Duration::hours(1),
                    };
                    Ok(now >= last_exec + threshold)
                } else {
                    Ok(true)
                }
            }
            "cron" => {
                // Simplified cron check - in real implementation, use a proper cron library
                if let Some(last_exec) = last_execution {
                    Ok(now >= last_exec + Duration::hours(1))
                } else {
                    Ok(true)
                }
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_schedule_trigger_node_creation() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Test Schedule Trigger".to_string(),
            "scheduleTrigger".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "schedule_type": "interval",
                "interval_seconds": 300
            })),
        );

        assert_eq!(node.name(), "Test Schedule Trigger");
        assert!(node.validate());
    }

    #[test]
    fn test_once_schedule() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Once Schedule".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "once",
                "enabled": true
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert!(output.get("trigger_event").is_some());
            assert!(output.get("schedule_info").is_some());
            
            let schedule_info = &output["schedule_info"];
            assert_eq!(schedule_info["schedule_type"], "once");
            assert_eq!(schedule_info["enabled"], true);
        }
    }

    #[test]
    fn test_interval_schedule() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Interval Schedule".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "interval",
                "interval_seconds": 600,
                "enabled": true
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let schedule_info = &output["schedule_info"];
            assert_eq!(schedule_info["schedule_type"], "interval");
            assert!(schedule_info.get("next_execution").is_some());
        }
    }

    #[test]
    fn test_cron_schedule() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Cron Schedule".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "cron",
                "cron_expression": "0 */6 * * *",
                "enabled": true
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            let schedule_info = &output["schedule_info"];
            assert_eq!(schedule_info["schedule_type"], "cron");
        }
    }

    #[test]
    fn test_disabled_schedule() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Disabled Schedule".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "daily",
                "enabled": false
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert_eq!(output["status"], "disabled");
        }
    }

    #[test]
    fn test_with_trigger_data() {
        let trigger_data = json!({
            "message": "Scheduled execution",
            "priority": "high",
            "tags": ["automation", "scheduled"]
        });

        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Data Schedule".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "once",
                "enabled": true,
                "trigger_data": trigger_data.clone()
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert_eq!(output["trigger_data"], trigger_data);
            
            let trigger_event = &output["trigger_event"];
            assert_eq!(trigger_event["data"], trigger_data);
        }
    }

    #[test]
    fn test_datetime_parsing() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Test".to_string(),
            "scheduleTrigger".to_string(),
            None,
            None,
        );

        // Test RFC3339 format
        assert!(node.parse_datetime("2024-12-31T23:59:59Z").is_ok());
        
        // Test naive datetime format
        assert!(node.parse_datetime("2024-12-31 23:59:59").is_ok());
        
        // Test date only format
        assert!(node.parse_datetime("2024-12-31").is_ok());
        
        // Test invalid format
        assert!(node.parse_datetime("invalid-date").is_err());
    }

    #[test]
    fn test_cron_validation() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Test".to_string(),
            "scheduleTrigger".to_string(),
            None,
            None,
        );

        // Valid cron expressions
        assert!(node.is_valid_cron_expression("0 */6 * * *"));
        assert!(node.is_valid_cron_expression("30 2 * * 1-5"));
        assert!(node.is_valid_cron_expression("0 0 1 */3 *"));
        
        // Invalid cron expressions
        assert!(!node.is_valid_cron_expression(""));
        assert!(!node.is_valid_cron_expression("0"));
        assert!(!node.is_valid_cron_expression("* * * *")); // Too few parts
        assert!(!node.is_valid_cron_expression("0 0 0 0 0 0 0")); // Too many parts
    }

    #[test]
    fn test_should_execute_now() {
        let node = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Test".to_string(),
            "scheduleTrigger".to_string(),
            None,
            None,
        );

        // Test once schedule
        let once_params = ScheduleTriggerParams {
            schedule_type: "once".to_string(),
            enabled: true,
            ..Default::default()
        };
        
        // Should execute if never executed before
        assert!(node.should_execute_now(&once_params, None).unwrap());
        
        // Should not execute if already executed
        let past_time = Utc::now() - Duration::hours(1);
        assert!(!node.should_execute_now(&once_params, Some(past_time)).unwrap());

        // Test disabled schedule
        let disabled_params = ScheduleTriggerParams {
            enabled: false,
            ..Default::default()
        };
        assert!(!node.should_execute_now(&disabled_params, None).unwrap());
    }

    #[test]
    fn test_validation() {
        // Valid interval schedule
        let valid_interval = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Valid Interval".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "interval",
                "interval_seconds": 300
            })),
        );
        assert!(valid_interval.validate());

        // Invalid interval schedule (missing interval_seconds)
        let invalid_interval = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Interval".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "interval"
            })),
        );
        assert!(!invalid_interval.validate());

        // Valid cron schedule
        let valid_cron = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Valid Cron".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "cron",
                "cron_expression": "0 */6 * * *"
            })),
        );
        assert!(valid_cron.validate());

        // Invalid cron schedule (missing cron_expression)
        let invalid_cron = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Cron".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "cron"
            })),
        );
        assert!(!invalid_cron.validate());

        // Invalid schedule type
        let invalid_type = ScheduleTriggerNode::new(
            uuid::Uuid::new_v4(),
            "Invalid Type".to_string(),
            "scheduleTrigger".to_string(),
            None,
            Some(json!({
                "schedule_type": "invalid_type"
            })),
        );
        assert!(!invalid_type.validate());
    }
}