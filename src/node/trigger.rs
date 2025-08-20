use crate::execution::NodeOutput;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerNode {
    pub id: super::NodeId,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

impl super::INode for TriggerNode {
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
        // For trigger nodes, we convert the input NodeOutput to a JSON object
        match self.parameters.clone() {
            Some(parameters) => {
                let input_value = serde_json::to_value(parameters)?;
                Ok(input_value)
            }
            None => Ok(Value::Null),
        }
    }

    fn validate(&self) -> bool {
        // Validate node name cannot be empty
        if self.name.trim().is_empty() {
            return false;
        }

        // If parameters exist, validate they cannot be null
        if let Some(params) = &self.parameters {
            if params.is_null() {
                return false;
            }
        }

        true
    }

    fn dependencies(&self) -> Vec<super::NodeId> {
        // Trigger nodes typically don't depend on other nodes, return empty list
        // If support for expressions in trigger node parameters is needed, implement logic similar to SetNode here
        Vec::new()
    }
}
