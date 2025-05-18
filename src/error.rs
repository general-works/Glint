use std::io;
use thiserror::Error;

/// Error type for Glint framework
#[derive(Error, Debug)]
pub enum Error {
    /// Error related to graph construction/execution
    #[error("Graph error: {0}")]
    Graph(String),

    /// Error related to node execution
    #[error("Node execution error: {0}")]
    NodeExecution(String),

    /// Error related to invalid node
    #[error("Invalid node: {0}")]
    InvalidNode(String),

    /// Error related to invalid edge
    #[error("Invalid edge: {0}")]
    InvalidEdge(String),

    /// Error with cycle detection
    #[error("Cycle detected: {0}")]
    CycleDetected(String),

    /// Error related to state
    #[error("State error: {0}")]
    State(String),

    /// Error related to edge conditions
    #[error("Edge condition error: {0}")]
    EdgeCondition(String),

    /// Error related to checkpoints
    #[error("Checkpoint error: {0}")]
    Checkpoint(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization or deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// LLM error
    #[error("LLM error: {0}")]
    LLM(String),

    /// HTTP request error
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    /// Prompt template error
    #[error("Prompt template error: {0}")]
    PromptTemplate(String),

    /// Document loader error
    #[error("Document loader error: {0}")]
    DocumentLoader(String),

    /// Error from pregel execution
    #[error("Pregel error: {0}")]
    Pregel(String),

    /// Other general errors
    #[error("Other error: {0}")]
    Other(String),
}
