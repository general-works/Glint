use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::thread;
use std::time::Duration;
use chrono::{DateTime, Utc, NaiveDateTime};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitParams {
    pub resume: String, // "timeInterval", "specificTime"
    #[serde(default = "default_amount")]
    pub amount: f64, // For timeInterval mode
    #[serde(default = "default_unit")]
    pub unit: String, // "seconds", "minutes", "hours", "days"
    pub date_time: Option<String>, // For specificTime mode (ISO 8601 format)
    #[serde(default = "default_true")]
    pub include_input_fields: bool, // Whether to include original fields in output
}

fn default_amount() -> f64 {
    5.0
}

fn default_unit() -> String {
    "seconds".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for WaitParams {
    fn default() -> Self {
        Self {
            resume: "timeInterval".to_string(),
            amount: 5.0,
            unit: "seconds".to_string(),
            date_time: None,
            include_input_fields: true,
        }
    }
}

impl INode for WaitNode {
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
            serde_json::from_value::<WaitParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            WaitParams::default()
        };

        // Perform the wait operation
        match params.resume.as_str() {
            "timeInterval" => self.wait_time_interval(&params)?,
            "specificTime" => self.wait_specific_time(&params)?,
            _ => return Err(Error::Validation(format!("Unsupported resume mode: {}", params.resume)))
        }

        // After waiting, return the input data
        let input_data: Vec<Value> = input.values().cloned().collect();

        if params.include_input_fields && !input_data.is_empty() {
            // Return the original input data with timing information
            let mut results = Vec::new();
            for item in input_data {
                if let Value::Array(items) = item {
                    for single_item in items {
                        let mut result = single_item;
                        if let Value::Object(ref mut map) = result {
                            map.insert("waitCompleted".to_string(), json!(true));
                            map.insert("waitCompletedAt".to_string(), json!(Utc::now().to_rfc3339()));
                        }
                        results.push(result);
                    }
                } else {
                    let mut result = item;
                    if let Value::Object(ref mut map) = result {
                        map.insert("waitCompleted".to_string(), json!(true));
                        map.insert("waitCompletedAt".to_string(), json!(Utc::now().to_rfc3339()));
                    }
                    results.push(result);
                }
            }
            Ok(Value::Array(results))
        } else {
            // Return just the timing information
            Ok(json!([{
                "waitCompleted": true,
                "waitCompletedAt": Utc::now().to_rfc3339(),
                "waitDuration": format!("{} {}", params.amount, params.unit)
            }]))
        }
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<WaitParams>(params_value.clone()) {
                return matches!(params.resume.as_str(), "timeInterval" | "specificTime") &&
                       (params.resume != "timeInterval" || (params.amount >= 0.0 && matches!(params.unit.as_str(), "seconds" | "minutes" | "hours" | "days"))) &&
                       (params.resume != "specificTime" || params.date_time.is_some());
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl WaitNode {
    fn wait_time_interval(&self, params: &WaitParams) -> Result<(), Error> {
        if params.amount < 0.0 {
            return Err(Error::Validation("Wait amount cannot be negative".to_string()));
        }

        // Convert the amount to milliseconds
        let mut wait_amount_ms = (params.amount * 1000.0) as u64;
        
        match params.unit.as_str() {
            "seconds" => {
                // Already in correct units (converted to ms above)
            }
            "minutes" => {
                wait_amount_ms *= 60;
            }
            "hours" => {
                wait_amount_ms *= 60 * 60;
            }
            "days" => {
                wait_amount_ms *= 60 * 60 * 24;
            }
            _ => return Err(Error::Validation(format!("Unsupported time unit: {}", params.unit)))
        }

        // For very short waits (< 1ms), just return immediately
        if wait_amount_ms == 0 {
            return Ok(());
        }

        // Perform the actual wait
        thread::sleep(Duration::from_millis(wait_amount_ms));
        
        Ok(())
    }

    fn wait_specific_time(&self, params: &WaitParams) -> Result<(), Error> {
        let date_time_str = params.date_time.as_ref()
            .ok_or_else(|| Error::Validation("date_time is required for specificTime mode".to_string()))?;

        // Parse the date time string (expecting ISO 8601 format)
        let target_time = self.parse_datetime(date_time_str)?;
        let now = Utc::now();

        if target_time <= now {
            // If the target time is in the past or now, don't wait
            return Ok(());
        }

        let duration = target_time.signed_duration_since(now);
        let wait_duration = Duration::from_millis(duration.num_milliseconds() as u64);

        // Perform the actual wait
        thread::sleep(wait_duration);
        
        Ok(())
    }

    fn parse_datetime(&self, date_time_str: &str) -> Result<DateTime<Utc>, Error> {
        // Try to parse as ISO 8601 format first
        if let Ok(dt) = DateTime::parse_from_rfc3339(date_time_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try to parse as naive datetime and assume UTC
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_time_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
        }

        // Try to parse date only and assume start of day
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_time_str, "%Y-%m-%d") {
            let naive_dt = date.and_hms_opt(0, 0, 0).unwrap();
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
        }

        Err(Error::Validation(format!("Invalid date time format: {}. Expected ISO 8601 format (e.g., '2023-12-31T23:59:59Z') or YYYY-MM-DD HH:MM:SS", date_time_str)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Instant;

    #[test]
    fn test_wait_node_creation() {
        let node = WaitNode::new(
            uuid::Uuid::new_v4(),
            "Test Wait".to_string(),
            "wait".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "resume": "timeInterval",
                "amount": 1.0,
                "unit": "seconds"
            })),
        );

        assert_eq!(node.name(), "Test Wait");
        assert!(node.validate());
    }

    #[test]
    fn test_wait_validation() {
        let node = WaitNode::new(
            uuid::Uuid::new_v4(),
            "Test Wait".to_string(),
            "wait".to_string(),
            None,
            Some(json!({
                "resume": "invalid_mode",
                "amount": 1.0,
                "unit": "seconds"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_wait_time_interval_execution() {
        let node = WaitNode::new(
            uuid::Uuid::new_v4(),
            "Test Wait".to_string(),
            "wait".to_string(),
            None,
            Some(json!({
                "resume": "timeInterval",
                "amount": 0.1, // 100ms wait
                "unit": "seconds",
                "include_input_fields": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({
                "message": "test data",
                "value": 123
            })
        );

        let start_time = Instant::now();
        let result = node.execute(&input);
        let elapsed = start_time.elapsed();

        assert!(result.is_ok());
        assert!(elapsed >= Duration::from_millis(100)); // Should have waited at least 100ms
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert_eq!(item["message"].as_str().unwrap(), "test data");
            assert_eq!(item["value"].as_i64().unwrap(), 123);
            assert_eq!(item["waitCompleted"].as_bool().unwrap(), true);
            assert!(item["waitCompletedAt"].is_string());
        }
    }

    #[test]
    fn test_wait_specific_time_validation() {
        let node = WaitNode::new(
            uuid::Uuid::new_v4(),
            "Test Wait".to_string(),
            "wait".to_string(),
            None,
            Some(json!({
                "resume": "specificTime",
                "date_time": "2099-12-31T23:59:59Z"
            })),
        );

        assert!(node.validate());
    }

    #[test]
    fn test_wait_specific_time_past_date() {
        let node = WaitNode::new(
            uuid::Uuid::new_v4(),
            "Test Wait".to_string(),
            "wait".to_string(),
            None,
            Some(json!({
                "resume": "specificTime",
                "date_time": "2020-01-01T00:00:00Z", // Past date
                "include_input_fields": false
            })),
        );

        let input = HashMap::new();
        let start_time = Instant::now();
        let result = node.execute(&input);
        let elapsed = start_time.elapsed();

        assert!(result.is_ok());
        assert!(elapsed < Duration::from_millis(10)); // Should not have waited
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            let item = &results[0];
            assert_eq!(item["waitCompleted"].as_bool().unwrap(), true);
        }
    }

    #[test]
    fn test_parse_datetime_formats() {
        let node = WaitNode::new(
            uuid::Uuid::new_v4(),
            "Test Wait".to_string(),
            "wait".to_string(),
            None,
            None,
        );

        // Test ISO 8601 format
        assert!(node.parse_datetime("2023-12-31T23:59:59Z").is_ok());
        assert!(node.parse_datetime("2023-12-31T23:59:59+00:00").is_ok());
        
        // Test simple datetime format
        assert!(node.parse_datetime("2023-12-31 23:59:59").is_ok());
        
        // Test date only format
        assert!(node.parse_datetime("2023-12-31").is_ok());
        
        // Test invalid format
        assert!(node.parse_datetime("invalid-date").is_err());
    }

    #[test]
    fn test_wait_different_units() {
        let test_cases = vec![
            ("seconds", 0.01), // 10ms
            ("minutes", 0.0001), // 6ms (0.0001 * 60 * 1000)
            ("hours", 0.000001), // ~3.6ms
        ];

        for (unit, amount) in test_cases {
            let node = WaitNode::new(
                uuid::Uuid::new_v4(),
                "Test Wait".to_string(),
                "wait".to_string(),
                None,
                Some(json!({
                    "resume": "timeInterval",
                    "amount": amount,
                    "unit": unit,
                    "include_input_fields": false
                })),
            );

            let input = HashMap::new();
            let result = node.execute(&input);
            assert!(result.is_ok(), "Failed for unit: {}", unit);
        }
    }
}