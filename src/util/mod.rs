use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Database(String),
    Validation(String),
    WorkflowExecution(String),
    NodeExecution(String),
    ExpressionEvaluation(String),
    BinaryData(String),
    Queue(String),
    Configuration(String),
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Database(msg) => write!(f, "Database error: {}", msg),
            Error::Validation(msg) => write!(f, "Validation error: {}", msg),
            Error::WorkflowExecution(msg) => write!(f, "Workflow execution error: {}", msg),
            Error::NodeExecution(msg) => write!(f, "Node execution error: {}", msg),
            Error::ExpressionEvaluation(msg) => write!(f, "Expression evaluation error: {}", msg),
            Error::BinaryData(msg) => write!(f, "Binary data error: {}", msg),
            Error::Queue(msg) => write!(f, "Queue error: {}", msg),
            Error::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Configuration(format!("JSON parsing error: {}", err))
    }
}

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Error::Configuration(format!("UUID parsing error: {}", err))
    }
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Error::Internal(format!("Internal error: {}", err))
    }
}
