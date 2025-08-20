use crate::execution::NodeOutput;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeNode {
    pub id: super::NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

impl super::INode for TimeNode {
    fn new(
        id: super::NodeId,
        name: String,
        _type: String,
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

    fn id(&self) -> super::NodeId {
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

    fn execute(&self, _input: &NodeOutput) -> Result<Value, super::Error> {
        // Get current UTC time and format as YYYY-MM-DD HH:MM:SS
        let now = Utc::now();
        let formatted_time = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // If no parameters, return default time field
        let Some(params) = &self.parameters else {
            let mut result = serde_json::Map::new();
            result.insert("now".to_string(), Value::String(formatted_time));
            return Ok(Value::Object(result));
        };

        // Parameters must be an object
        let Some(obj) = params.as_object() else {
            return Err(super::Error::NodeExecution("Parameters must be an object".to_string()));
        };

        // Parameter object must have exactly one field
        if obj.len() != 1 {
            return Err(super::Error::NodeExecution(
                "Parameter object must have exactly one field".to_string(),
            ));
        };

        // Get the unique field name
        let (field_name, _) = obj.iter().next().unwrap();

        // Construct return result
        let mut result = serde_json::Map::new();
        result.insert(field_name.clone(), Value::String(formatted_time));

        Ok(Value::Object(result))
    }

    fn validate(&self) -> bool {
        // Validate node name cannot be empty
        if self.name.trim().is_empty() {
            return false;
        }

        // If no parameters, it's also valid (time nodes can work without parameters)
        let Some(params) = &self.parameters else {
            return true;
        };

        // Parameters cannot be null
        if params.is_null() {
            return false;
        }

        // Parameters must be an object
        let Some(obj) = params.as_object() else {
            return false;
        };

        // Parameter object must have exactly one field
        if obj.len() != 1 {
            return false;
        }

        // Get the unique field value
        let (_, field_value) = obj.iter().next().unwrap();

        // Field value must be a string
        let Some(value_str) = field_value.as_str() else {
            return false;
        };

        // Field value must be exactly "{{now}}"
        value_str == "{{now}}"
    }

    fn dependencies(&self) -> Vec<super::NodeId> {
        Vec::new()
    }
}
