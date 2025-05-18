use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A serializable representation of a graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableGraph {
    /// Nodes in the graph
    pub nodes: Vec<String>,
    /// Edges in the graph with their conditions
    pub edges: Vec<SerializableEdge>,
    /// Graph metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A serializable edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableEdge {
    /// Source node
    pub from: String,
    /// Target node
    pub to: String,
    /// If this edge has a condition
    pub has_condition: bool,
    /// Description of the condition (for documentation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_description: Option<String>,
}

impl SerializableGraph {
    /// Create a DOT graph representation for visualization
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph G {\n");

        // Add nodes
        for node in &self.nodes {
            dot.push_str(&format!("    \"{}\";\n", node));
        }

        // Add edges
        for edge in &self.edges {
            if edge.has_condition {
                // If the edge has a condition, add a label
                let label = edge.condition_description.as_deref().unwrap_or("condition");
                dot.push_str(&format!(
                    "    \"{}\" -> \"{}\" [label=\"{}\"];\n",
                    edge.from, edge.to, label
                ));
            } else {
                dot.push_str(&format!("    \"{}\" -> \"{}\";\n", edge.from, edge.to));
            }
        }

        dot.push_str("}\n");
        dot
    }
}

/// Generate a DOT graph representation from a SerializableGraph
pub fn graph_to_dot(graph: &SerializableGraph) -> String {
    graph.to_dot()
}
