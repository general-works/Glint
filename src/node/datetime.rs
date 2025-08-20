use chrono::{DateTime, Local, TimeZone, NaiveDateTime, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::execution::NodeOutput;
use crate::node::{INode, NodeId, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateTimeNode {
    pub id: NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateTimeParams {
    pub operation: String, // "getCurrentDate", "formatDate", "addToDate", "subtractFromDate", "extractDate"
    // For getCurrentDate
    #[serde(default = "default_false")]
    pub include_time: bool,
    #[serde(default = "default_output_field")]
    pub output_field_name: String,
    pub timezone: Option<String>,
    // For formatDate
    pub date: Option<String>,
    pub format: Option<String>,
    pub custom_format: Option<String>,
    // For add/subtract operations
    pub magnitude: Option<String>, // date to modify
    pub time_unit: Option<String>, // "days", "hours", "minutes", "seconds", "months", "years"
    pub duration: Option<i64>,
    // For extractDate
    pub part: Option<String>, // "year", "month", "day", "hour", "minute", "second", "weekday"
    // General options
    #[serde(default = "default_false")]
    pub include_input_fields: bool,
}

fn default_false() -> bool { false }
fn default_output_field() -> String { "date".to_string() }

impl Default for DateTimeParams {
    fn default() -> Self {
        Self {
            operation: "getCurrentDate".to_string(),
            include_time: false,
            output_field_name: "date".to_string(),
            timezone: None,
            date: None,
            format: None,
            custom_format: None,
            magnitude: None,
            time_unit: None,
            duration: None,
            part: None,
            include_input_fields: false,
        }
    }
}

impl INode for DateTimeNode {
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
            serde_json::from_value::<DateTimeParams>(params_value.clone())
                .unwrap_or_default()
        } else {
            DateTimeParams::default()
        };

        // Process input data
        let input_data: Vec<Value> = input.values().cloned().collect();
        let mut results = Vec::new();

        // If no input data, create a single item
        let items_to_process = if input_data.is_empty() {
            vec![json!({})]
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
            let result_item = self.process_datetime_operation(&params, &item)?;
            results.push(result_item);
        }

        Ok(Value::Array(results))
    }

    fn validate(&self) -> bool {
        if let Some(params_value) = &self.parameters {
            if let Ok(params) = serde_json::from_value::<DateTimeParams>(params_value.clone()) {
                return matches!(params.operation.as_str(), 
                    "getCurrentDate" | "formatDate" | "addToDate" | "subtractFromDate" | "extractDate"
                );
            }
        }
        false
    }

    fn dependencies(&self) -> Vec<NodeId> {
        vec![]
    }
}

impl DateTimeNode {
    fn process_datetime_operation(&self, params: &DateTimeParams, item: &Value) -> Result<Value, Error> {
        let mut result_item = if params.include_input_fields {
            item.clone()
        } else {
            json!({})
        };

        match params.operation.as_str() {
            "getCurrentDate" => self.get_current_date(params, &mut result_item),
            "formatDate" => self.format_date(params, item, &mut result_item),
            "addToDate" => self.add_to_date(params, item, &mut result_item),
            "subtractFromDate" => self.subtract_from_date(params, item, &mut result_item),
            "extractDate" => self.extract_date_part(params, item, &mut result_item),
            _ => Err(Error::Validation(format!("Unsupported datetime operation: {}", params.operation)))
        }
    }

    fn get_current_date(&self, params: &DateTimeParams, result_item: &mut Value) -> Result<Value, Error> {
        let now = if params.include_time {
            Local::now().to_rfc3339()
        } else {
            Local::now().date_naive().format("%Y-%m-%d").to_string()
        };

        if let Value::Object(map) = result_item {
            map.insert(params.output_field_name.clone(), json!(now));
        }

        Ok(result_item.clone())
    }

    fn format_date(&self, params: &DateTimeParams, item: &Value, result_item: &mut Value) -> Result<Value, Error> {
        let date_str = params.date.as_ref()
            .map(|s| s.as_str())
            .or_else(|| item.get("date").and_then(|v| v.as_str()))
            .ok_or_else(|| Error::Validation("Date parameter is required for formatDate operation".to_string()))?;

        let default_format = "%Y-%m-%d %H:%M:%S".to_string();
        let format = params.format.as_ref()
            .or_else(|| params.custom_format.as_ref())
            .unwrap_or(&default_format);

        // Try to parse the input date
        let parsed_date = self.parse_date_string(date_str)?;
        
        // Format the date
        let formatted = match format.as_str() {
            "ISO" => parsed_date.to_rfc3339(),
            "yyyy-MM-dd" => parsed_date.format("%Y-%m-%d").to_string(),
            "yyyy-MM-dd HH:mm:ss" => parsed_date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "MM/dd/yyyy" => parsed_date.format("%m/%d/%Y").to_string(),
            "dd/MM/yyyy" => parsed_date.format("%d/%m/%Y").to_string(),
            custom => parsed_date.format(custom).to_string(),
        };

        if let Value::Object(map) = result_item {
            map.insert(params.output_field_name.clone(), json!(formatted));
        }

        Ok(result_item.clone())
    }

    fn add_to_date(&self, params: &DateTimeParams, item: &Value, result_item: &mut Value) -> Result<Value, Error> {
        let date_str = params.magnitude.as_ref()
            .map(|s| s.as_str())
            .or_else(|| item.get("date").and_then(|v| v.as_str()))
            .ok_or_else(|| Error::Validation("Date parameter is required for addToDate operation".to_string()))?;

        let time_unit = params.time_unit.as_ref()
            .ok_or_else(|| Error::Validation("time_unit parameter is required for addToDate operation".to_string()))?;

        let duration = params.duration
            .ok_or_else(|| Error::Validation("duration parameter is required for addToDate operation".to_string()))?;

        let parsed_date = self.parse_date_string(date_str)?;
        let result_date = self.add_duration_to_date(&parsed_date, time_unit, duration)?;

        if let Value::Object(map) = result_item {
            map.insert(params.output_field_name.clone(), json!(result_date.to_rfc3339()));
        }

        Ok(result_item.clone())
    }

    fn subtract_from_date(&self, params: &DateTimeParams, item: &Value, result_item: &mut Value) -> Result<Value, Error> {
        let date_str = params.magnitude.as_ref()
            .map(|s| s.as_str())
            .or_else(|| item.get("date").and_then(|v| v.as_str()))
            .ok_or_else(|| Error::Validation("Date parameter is required for subtractFromDate operation".to_string()))?;

        let time_unit = params.time_unit.as_ref()
            .ok_or_else(|| Error::Validation("time_unit parameter is required for subtractFromDate operation".to_string()))?;

        let duration = params.duration
            .ok_or_else(|| Error::Validation("duration parameter is required for subtractFromDate operation".to_string()))?;

        let parsed_date = self.parse_date_string(date_str)?;
        let result_date = self.add_duration_to_date(&parsed_date, time_unit, -duration)?;

        if let Value::Object(map) = result_item {
            map.insert(params.output_field_name.clone(), json!(result_date.to_rfc3339()));
        }

        Ok(result_item.clone())
    }

    fn extract_date_part(&self, params: &DateTimeParams, item: &Value, result_item: &mut Value) -> Result<Value, Error> {
        let date_str = params.date.as_ref()
            .map(|s| s.as_str())
            .or_else(|| item.get("date").and_then(|v| v.as_str()))
            .ok_or_else(|| Error::Validation("Date parameter is required for extractDate operation".to_string()))?;

        let part = params.part.as_ref()
            .ok_or_else(|| Error::Validation("part parameter is required for extractDate operation".to_string()))?;

        let parsed_date = self.parse_date_string(date_str)?;
        
        let extracted_value = match part.as_str() {
            "year" => json!(parsed_date.year()),
            "month" => json!(parsed_date.month()),
            "day" => json!(parsed_date.day()),
            "hour" => json!(parsed_date.hour()),
            "minute" => json!(parsed_date.minute()),
            "second" => json!(parsed_date.second()),
            "weekday" => json!(parsed_date.weekday().num_days_from_monday()),
            _ => return Err(Error::Validation(format!("Unsupported date part: {}", part)))
        };

        if let Value::Object(map) = result_item {
            map.insert(params.output_field_name.clone(), extracted_value);
        }

        Ok(result_item.clone())
    }

    fn parse_date_string(&self, date_str: &str) -> Result<DateTime<Local>, Error> {
        // Try different date formats
        let formats = vec![
            "%Y-%m-%dT%H:%M:%S%.fZ",           // ISO 8601 with Z
            "%Y-%m-%dT%H:%M:%S%.f%:z",        // ISO 8601 with timezone
            "%Y-%m-%dT%H:%M:%S%:z",           // ISO 8601 without milliseconds
            "%Y-%m-%d %H:%M:%S",              // Standard datetime
            "%Y-%m-%d",                       // Date only
            "%m/%d/%Y",                       // US format
            "%d/%m/%Y",                       // European format
        ];

        // First try parsing as RFC3339/ISO8601
        if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            return Ok(dt.with_timezone(&Local));
        }

        // Try other formats
        for format in formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, format) {
                return Ok(Local.from_local_datetime(&naive_dt).single()
                    .ok_or_else(|| Error::Validation("Invalid local datetime".to_string()))?);
            }
            
            if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, format) {
                let naive_dt = naive_date.and_hms_opt(0, 0, 0)
                    .ok_or_else(|| Error::Validation("Invalid date".to_string()))?;
                return Ok(Local.from_local_datetime(&naive_dt).single()
                    .ok_or_else(|| Error::Validation("Invalid local datetime".to_string()))?);
            }
        }

        Err(Error::Validation(format!("Unable to parse date: {}", date_str)))
    }

    fn add_duration_to_date(&self, date: &DateTime<Local>, unit: &str, amount: i64) -> Result<DateTime<Local>, Error> {
        match unit {
            "seconds" | "second" => Ok(*date + chrono::Duration::seconds(amount)),
            "minutes" | "minute" => Ok(*date + chrono::Duration::minutes(amount)),
            "hours" | "hour" => Ok(*date + chrono::Duration::hours(amount)),
            "days" | "day" => Ok(*date + chrono::Duration::days(amount)),
            "weeks" | "week" => Ok(*date + chrono::Duration::weeks(amount)),
            "months" | "month" => {
                // Handle months more carefully due to variable month lengths
                let new_date = if amount > 0 {
                    date.checked_add_months(chrono::Months::new(amount as u32))
                } else {
                    date.checked_sub_months(chrono::Months::new((-amount) as u32))
                };
                new_date.ok_or_else(|| Error::Validation("Invalid month calculation".to_string()))
            },
            "years" | "year" => {
                // Handle years similar to months
                let new_date = if amount > 0 {
                    date.checked_add_months(chrono::Months::new((amount * 12) as u32))
                } else {
                    date.checked_sub_months(chrono::Months::new(((-amount) * 12) as u32))
                };
                new_date.ok_or_else(|| Error::Validation("Invalid year calculation".to_string()))
            },
            _ => Err(Error::Validation(format!("Unsupported time unit: {}", unit)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_datetime_node_creation() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            Some("Test description".to_string()),
            Some(json!({
                "operation": "getCurrentDate"
            })),
        );

        assert_eq!(node.name(), "Test DateTime");
        assert!(node.validate());
    }

    #[test]
    fn test_datetime_validation() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            Some(json!({
                "operation": "invalid_operation"
            })),
        );

        assert!(!node.validate());
    }

    #[test]
    fn test_get_current_date() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            Some(json!({
                "operation": "getCurrentDate",
                "include_time": false,
                "output_field_name": "current_date"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert!(results[0]["current_date"].is_string());
        }
    }

    #[test]
    fn test_format_date() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            Some(json!({
                "operation": "formatDate",
                "date": "2023-12-25T10:30:00Z",
                "format": "yyyy-MM-dd",
                "output_field_name": "formatted_date"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert!(results[0]["formatted_date"].as_str().unwrap().contains("2023-12-25"));
        }
    }

    #[test]
    fn test_add_to_date() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            Some(json!({
                "operation": "addToDate",
                "magnitude": "2023-12-25T10:30:00Z",
                "time_unit": "days",
                "duration": 7,
                "output_field_name": "new_date"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert!(results[0]["new_date"].is_string());
        }
    }

    #[test]
    fn test_subtract_from_date() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            Some(json!({
                "operation": "subtractFromDate",
                "magnitude": "2023-12-25T10:30:00Z",
                "time_unit": "hours",
                "duration": 5,
                "output_field_name": "new_date"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert!(results[0]["new_date"].is_string());
        }
    }

    #[test]
    fn test_extract_date_part() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            Some(json!({
                "operation": "extractDate",
                "date": "2023-12-25T10:30:00Z",
                "part": "year",
                "output_field_name": "year_value"
            })),
        );

        let input = HashMap::new();
        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["year_value"], 2023);
        }
    }

    #[test]
    fn test_parse_different_date_formats() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            None,
        );

        // Test various date formats
        let test_dates = vec![
            "2023-12-25T10:30:00Z",
            "2023-12-25 10:30:00",
            "2023-12-25",
            "12/25/2023",
            "25/12/2023",
        ];

        for date_str in test_dates {
            let result = node.parse_date_string(date_str);
            assert!(result.is_ok(), "Failed to parse date: {}", date_str);
        }
    }

    #[test]
    fn test_include_input_fields() {
        let node = DateTimeNode::new(
            uuid::Uuid::new_v4(),
            "Test DateTime".to_string(),
            "dateTime".to_string(),
            None,
            Some(json!({
                "operation": "getCurrentDate",
                "output_field_name": "timestamp",
                "include_input_fields": true
            })),
        );

        let mut input = HashMap::new();
        input.insert(
            uuid::Uuid::new_v4(), 
            json!({"name": "test", "value": 42})
        );

        let result = node.execute(&input);
        assert!(result.is_ok());
        
        if let Ok(Value::Array(results)) = result {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0]["name"], "test");
            assert_eq!(results[0]["value"], 42);
            assert!(results[0]["timestamp"].is_string());
        }
    }
}