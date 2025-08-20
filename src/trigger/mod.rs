use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualTrigger {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
    pub output: Option<Value>,
}
