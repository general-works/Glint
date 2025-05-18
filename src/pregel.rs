use async_trait::async_trait;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;

use crate::error::Error;
use crate::state::{State, StateValue};
use crate::Result;

/// Message type for node communication
#[derive(Debug, Clone)]
pub struct Message<S: StateValue> {
    /// Source node
    pub from: String,
    /// Target node
    pub to: String,
    /// Message payload
    pub payload: State<S>,
}

/// Trait for nodes in a pregel graph
#[async_trait]
pub trait PregelNode<S: StateValue>: Send + Sync {
    /// Process incoming messages and return outgoing messages
    async fn process(&self, messages: Vec<Message<S>>) -> Result<Vec<Message<S>>>;

    /// Get the nodes this node can send messages to
    fn targets(&self) -> Vec<String>;
}

/// A graph that processes messages between nodes using a pregel-like model
pub struct PregelGraph<S: StateValue> {
    /// Map of node names to node implementations
    nodes: HashMap<String, Arc<dyn PregelNode<S>>>,
    /// Map of node names to their allowed targets
    adjacency: HashMap<String, HashSet<String>>,
}

impl<S: StateValue> Default for PregelGraph<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: StateValue> PregelGraph<S> {
    /// Create a new empty pregel graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        node: impl PregelNode<S> + 'static,
    ) -> &mut Self {
        let name = name.into();
        let node = Arc::new(node);

        // Add the node
        self.nodes.insert(name.clone(), node.clone());

        // Add the adjacency list
        let targets = node.targets();
        let target_set: HashSet<String> = targets.into_iter().collect();
        self.adjacency.insert(name, target_set);

        self
    }

    /// Check if a node exists in the graph
    pub fn has_node(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    /// Get allowed targets for a node
    pub fn targets(&self, name: &str) -> Option<&HashSet<String>> {
        self.adjacency.get(name)
    }

    /// Execute the graph with an initial message
    pub async fn execute(
        &self,
        initial_message: Message<S>,
        max_steps: Option<usize>,
    ) -> Result<State<S>> {
        // Verify the target node exists
        if !self.has_node(&initial_message.to) {
            return Err(Error::InvalidNode(format!(
                "Target node not found: {}",
                initial_message.to
            )));
        }

        let mut message_queue = VecDeque::new();
        message_queue.push_back(initial_message);

        let mut step_count = 0;
        let max_steps = max_steps.unwrap_or(1000); // Default to 1000 steps to prevent infinite loops

        while !message_queue.is_empty() {
            // Check for max steps
            step_count += 1;
            if step_count > max_steps {
                return Err(Error::Graph(format!(
                    "Exceeded maximum steps: {}",
                    max_steps
                )));
            }

            // Group messages by target node
            let mut node_messages: HashMap<String, Vec<Message<S>>> = HashMap::new();

            // Process this round of messages
            let queue_size = message_queue.len();
            for _ in 0..queue_size {
                let msg = message_queue.pop_front().unwrap();
                node_messages.entry(msg.to.clone()).or_default().push(msg);
            }

            // Store any sink messages for later
            let sink_messages = if node_messages.contains_key("sink") {
                Some(node_messages.get("sink").unwrap().clone())
            } else {
                None
            };

            // Process each node's messages and collect new messages
            for (node_name, messages) in node_messages {
                let node = self
                    .nodes
                    .get(&node_name)
                    .ok_or_else(|| Error::InvalidNode(format!("Node not found: {}", node_name)))?;

                let result_messages = node.process(messages).await?;

                // Validate and enqueue new messages
                for msg in result_messages {
                    let from = &msg.from;
                    let to = &msg.to;

                    // Ensure the sender is the current node or a special case
                    if from != &node_name && from != "system" {
                        return Err(Error::Graph(format!(
                            "Node {} attempted to send a message as {}",
                            node_name, from
                        )));
                    }

                    // Check if the target node exists
                    if !self.has_node(to) {
                        return Err(Error::InvalidNode(format!("Target node not found: {}", to)));
                    }

                    // Check if this edge is allowed
                    if let Some(targets) = self.adjacency.get(from) {
                        if from != "system" && !targets.contains(to) {
                            return Err(Error::InvalidEdge(format!(
                                "Edge not allowed: {} -> {}",
                                from, to
                            )));
                        }
                    }

                    message_queue.push_back(msg);
                }
            }

            // If message queue is empty but we have sink messages, return the last one's state
            if message_queue.is_empty() && sink_messages.is_some() {
                let messages = sink_messages.unwrap();
                if !messages.is_empty() {
                    return Ok(messages.last().unwrap().payload.clone());
                }
            }
        }

        Err(Error::Graph(
            "Graph execution completed without result".to_string(),
        ))
    }
}
