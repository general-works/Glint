use async_trait::async_trait;
use futures::stream::{FuturesUnordered, StreamExt};
use petgraph::algo::has_path_connecting;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;

use crate::error::Error;
use crate::state::{State, StateValue};
use crate::Result;

/// Special node name for the graph entry point
pub const START: &str = "__start__";
/// Special node name for the graph exit point
pub const END: &str = "__end__";

/// Trait for node processors that operate on state.
///
/// # Examples
///
/// ```
/// use glint::graph::NodeProcessor;
/// use glint::state::{State, StateValue};
/// use glint::Result;
/// use async_trait::async_trait;
///
/// #[derive(Debug, Clone)]
/// struct MyState {
///     value: i32,
/// }
///
/// impl StateValue for MyState {}
///
/// struct MyProcessor;
///
/// #[async_trait]
/// impl NodeProcessor<MyState> for MyProcessor {
///     async fn process(&self, state: State<MyState>) -> Result<State<MyState>> {
///         // Process state here
///         Ok(state)
///     }
/// }
/// ```
#[async_trait]
pub trait NodeProcessor<S: StateValue>: Send + Sync {
    /// Process the state and return an updated state
    ///
    /// # Arguments
    ///
    /// * `state` - The current state to process
    ///
    /// # Returns
    ///
    /// A Result containing either:
    /// * Ok(State<S>) - The updated state after processing
    /// * Err(Error) - An error that occurred during processing
    async fn process(&self, state: State<S>) -> Result<State<S>>;
}

/// Type alias for edge condition functions
pub type EdgeConditionFn<S> = Arc<dyn Fn(&State<S>) -> Result<bool> + Send + Sync>;

/// Execution strategy for the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Execute nodes sequentially (default)
    Sequential,
    /// Execute independent nodes in parallel
    Parallel,
}

/// A graph of nodes that process state
pub struct Graph<S: StateValue> {
    /// The underlying directed graph
    graph: DiGraph<String, EdgeConditionFn<S>>,
    /// Map of node names to node indices
    node_map: HashMap<String, NodeIndex>,
    /// Map of node names to node processors
    processors: HashMap<String, Arc<dyn NodeProcessor<S>>>,
    /// Execution strategy
    execution_strategy: ExecutionStrategy,
    /// Maximum number of steps for parallel execution
    max_steps: usize,
}

impl<S: StateValue> fmt::Debug for Graph<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Graph")
            .field("node_map", &self.node_map)
            .field("node_count", &self.graph.node_count())
            .field("edge_count", &self.graph.edge_count())
            .field("execution_strategy", &self.execution_strategy)
            .field("max_steps", &self.max_steps)
            .finish()
    }
}

impl<S: StateValue> Default for Graph<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: StateValue> Graph<S> {
    /// Create a new empty graph
    pub fn new() -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add start and end nodes
        let start_idx = graph.add_node(START.to_string());
        let end_idx = graph.add_node(END.to_string());

        node_map.insert(START.to_string(), start_idx);
        node_map.insert(END.to_string(), end_idx);

        Self {
            graph,
            node_map,
            processors: HashMap::new(),
            execution_strategy: ExecutionStrategy::Sequential,
            max_steps: 1000,
        }
    }

    /// Set the execution strategy
    pub fn with_execution_strategy(mut self, strategy: ExecutionStrategy) -> Self {
        self.execution_strategy = strategy;
        self
    }

    /// Set the maximum number of steps for parallel execution
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Add a node to the graph
    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        processor: impl NodeProcessor<S> + 'static,
    ) -> Result<&mut Self> {
        let name = name.into();
        if name == START || name == END {
            return Err(Error::InvalidNode(format!(
                "Cannot use reserved node names: {} or {}",
                START, END
            )));
        }

        if !self.node_map.contains_key(&name) {
            let node_idx = self.graph.add_node(name.clone());
            self.node_map.insert(name.clone(), node_idx);
        }

        self.processors.insert(name, Arc::new(processor));
        Ok(self)
    }

    /// Add an edge between nodes with an optional condition
    pub fn add_edge(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        condition: Option<EdgeConditionFn<S>>,
    ) -> Result<&mut Self> {
        let from = from.into();
        let to = to.into();

        let from_idx = self
            .node_map
            .get(&from)
            .ok_or_else(|| Error::InvalidNode(format!("Source node not found: {}", from)))?;

        let to_idx = self
            .node_map
            .get(&to)
            .ok_or_else(|| Error::InvalidNode(format!("Target node not found: {}", to)))?;

        // Default condition that always returns true
        let condition = condition.unwrap_or_else(|| Arc::new(|_| Ok(true)));

        self.graph.add_edge(*from_idx, *to_idx, condition);
        Ok(self)
    }

    /// Connect a node to the start node
    pub fn add_start_edge(&mut self, to: impl Into<String>) -> Result<&mut Self> {
        self.add_edge(START, to, None)
    }

    /// Connect a node to the end node
    pub fn add_end_edge(&mut self, from: impl Into<String>) -> Result<&mut Self> {
        self.add_edge(from, END, None)
    }

    /// Check if two nodes are independent (no path between them)
    fn are_independent(&self, a: NodeIndex, b: NodeIndex) -> bool {
        !has_path_connecting(&self.graph, a, b, None)
            && !has_path_connecting(&self.graph, b, a, None)
    }

    /// Find all nodes that can be executed in parallel
    fn find_parallel_nodes(&self, current_nodes: &[NodeIndex]) -> Vec<Vec<NodeIndex>> {
        if current_nodes.len() <= 1 {
            return vec![current_nodes.to_vec()];
        }

        // Group nodes that can be executed in parallel
        let mut groups: Vec<Vec<NodeIndex>> = Vec::new();
        let mut assigned = HashSet::new();

        for &node in current_nodes {
            if assigned.contains(&node) {
                continue;
            }

            let mut group = vec![node];
            assigned.insert(node);

            for &other in current_nodes {
                if node == other || assigned.contains(&other) {
                    continue;
                }

                // Check if this node is independent of all nodes in the current group
                let can_add = group.iter().all(|&n| self.are_independent(n, other));

                if can_add {
                    group.push(other);
                    assigned.insert(other);
                }
            }

            groups.push(group);
        }

        groups
    }

    /// Merge multiple states into a single state
    async fn merge_states(&self, states: Vec<State<S>>) -> Result<State<S>> {
        if states.is_empty() {
            return Err(Error::State("No states to merge".to_string()));
        }

        // For now, we'll use a simple strategy of taking the last state
        // In a real implementation, you might want to merge specific fields
        Ok(states.last().unwrap().clone())
    }

    /// Execute the graph with the given initial state
    pub async fn execute(&self, initial_state: State<S>) -> Result<State<S>> {
        match self.execution_strategy {
            ExecutionStrategy::Sequential => self.execute_sequential(initial_state).await,
            ExecutionStrategy::Parallel => self.execute_parallel(initial_state).await,
        }
    }

    /// Execute the graph sequentially
    async fn execute_sequential(&self, initial_state: State<S>) -> Result<State<S>> {
        // Start at the START node
        let start_idx = *self.node_map.get(START).unwrap();
        let end_idx = *self.node_map.get(END).unwrap();
        let mut current_state = initial_state;
        let mut current_node = start_idx;
        let mut visited = HashSet::new();

        // Execute until we reach the END node or detect a cycle
        while current_node != end_idx {
            // Check for cycles
            if !visited.insert(current_node) {
                let node_name = self.graph.node_weight(current_node).unwrap();
                return Err(Error::CycleDetected(format!(
                    "Cycle detected at node: {}",
                    node_name
                )));
            }

            // Process current node if it's not START
            if current_node != start_idx {
                let node_name = self.graph.node_weight(current_node).unwrap();
                let processor = self.processors.get(node_name).ok_or_else(|| {
                    Error::Graph(format!("No processor found for node: {}", node_name))
                })?;

                current_state = processor.process(current_state).await?;
            }

            // Find next node based on edge conditions
            let mut next_node = None;
            for edge in self.graph.edges(current_node) {
                let condition = edge.weight();
                if condition(&current_state)? {
                    next_node = Some(edge.target());
                    break;
                }
            }

            let next_node = next_node.ok_or_else(|| {
                let node_name = self.graph.node_weight(current_node).unwrap();
                Error::Graph(format!("No valid edges from node: {}", node_name))
            })?;

            current_node = next_node;
        }

        Ok(current_state)
    }

    /// Execute the graph with parallel execution of independent nodes
    async fn execute_parallel(&self, initial_state: State<S>) -> Result<State<S>> {
        // Start at the START node
        let start_idx = *self.node_map.get(START).unwrap();
        let end_idx = *self.node_map.get(END).unwrap();
        let mut current_state = initial_state;
        let mut visited = HashSet::new();
        let mut step_count = 0;

        // Queue of nodes to process
        let mut node_queue = VecDeque::new();

        // Find initial nodes (all nodes that start can reach)
        for edge in self.graph.edges(start_idx) {
            if let Ok(true) = edge.weight()(&current_state) {
                node_queue.push_back(edge.target());
            }
        }

        // Process nodes until we reach the END node or run out of nodes
        while !node_queue.is_empty() {
            // Check for max steps
            step_count += 1;
            if step_count > self.max_steps {
                return Err(Error::Graph(format!(
                    "Exceeded maximum steps: {}",
                    self.max_steps
                )));
            }

            // Take all current nodes from the queue
            let mut current_nodes = Vec::new();
            while !node_queue.is_empty() {
                current_nodes.push(node_queue.pop_front().unwrap());
            }

            // Group nodes that can be executed in parallel
            let node_groups = self.find_parallel_nodes(&current_nodes);

            // Process each group of independent nodes
            for group in node_groups {
                // Skip empty groups
                if group.is_empty() {
                    continue;
                }

                // If there's only one node in the group, process it sequentially
                if group.len() == 1 {
                    let node_idx = group[0];

                    // Check for cycles
                    if !visited.insert(node_idx) {
                        let node_name = self.graph.node_weight(node_idx).unwrap();
                        return Err(Error::CycleDetected(format!(
                            "Cycle detected at node: {}",
                            node_name
                        )));
                    }

                    // If this is the END node, we're done
                    if node_idx == end_idx {
                        return Ok(current_state);
                    }

                    // Process the node
                    let node_name = self.graph.node_weight(node_idx).unwrap();
                    let processor = self.processors.get(node_name).ok_or_else(|| {
                        Error::Graph(format!("No processor found for node: {}", node_name))
                    })?;

                    current_state = processor.process(current_state).await?;

                    // Find next nodes
                    for edge in self.graph.edges(node_idx) {
                        if let Ok(true) = edge.weight()(&current_state) {
                            node_queue.push_back(edge.target());
                        }
                    }
                } else {
                    // Process nodes in parallel
                    let mut futures = FuturesUnordered::new();

                    // Check for cycles and prepare futures
                    for &node_idx in &group {
                        // Check for cycles
                        if !visited.insert(node_idx) {
                            let node_name = self.graph.node_weight(node_idx).unwrap();
                            return Err(Error::CycleDetected(format!(
                                "Cycle detected at node: {}",
                                node_name
                            )));
                        }

                        // If this is the END node, just add it to the queue
                        if node_idx == end_idx {
                            node_queue.push_back(node_idx);
                            continue;
                        }

                        // Add the processing future
                        let node_name = self.graph.node_weight(node_idx).unwrap().clone();
                        let processor = self.processors.get(&node_name).ok_or_else(|| {
                            Error::Graph(format!("No processor found for node: {}", node_name))
                        })?;

                        let processor_clone = processor.clone();
                        let state_clone = current_state.clone();

                        futures.push(async move {
                            let result = processor_clone.process(state_clone).await?;
                            Ok::<(String, State<S>), Error>((node_name, result))
                        });
                    }

                    // Wait for all nodes to complete
                    let mut results = Vec::new();
                    while let Some(result) = futures.next().await {
                        match result {
                            Ok((node_name, new_state)) => {
                                results.push((node_name, new_state));
                            }
                            Err(e) => return Err(e),
                        }
                    }

                    // Merge the results
                    if !results.is_empty() {
                        let states: Vec<State<S>> =
                            results.iter().map(|(_, state)| state.clone()).collect();
                        current_state = self.merge_states(states).await?;

                        // Add all next nodes to the queue
                        for (node_name, _) in results {
                            let node_idx = *self.node_map.get(&node_name).unwrap();

                            for edge in self.graph.edges(node_idx) {
                                if let Ok(true) = edge.weight()(&current_state) {
                                    node_queue.push_back(edge.target());
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(Error::Graph(
            "Graph execution completed without reaching END node".to_string(),
        ))
    }

    /// Export a serializable representation of the graph
    pub fn export_serializable(&self) -> crate::serialization::SerializableGraph {
        use crate::serialization::{SerializableEdge, SerializableGraph};

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Add all nodes except START and END
        for node_name in self.node_map.keys() {
            if node_name != START && node_name != END {
                nodes.push(node_name.clone());
            }
        }

        // Add all edges
        for edge in self.graph.edge_indices() {
            let (from_idx, to_idx) = self.graph.edge_endpoints(edge).unwrap();

            // Get the node names
            let from_name = self.graph.node_weight(from_idx).unwrap();
            let to_name = self.graph.node_weight(to_idx).unwrap();

            // Skip internal START/END edges
            if from_name == START || to_name == END {
                continue;
            }

            edges.push(SerializableEdge {
                from: from_name.clone(),
                to: to_name.clone(),
                has_condition: true, // We always have conditions, even if they're just "return true"
                condition_description: None,
            });
        }

        SerializableGraph {
            nodes,
            edges,
            metadata: HashMap::new(),
        }
    }
}

/// Builder for constructing a graph using a fluent interface
pub struct GraphBuilder<S: StateValue> {
    graph: Graph<S>,
}

impl<S: StateValue> GraphBuilder<S> {
    /// Create a new graph builder
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    /// Set the execution strategy
    pub fn with_execution_strategy(mut self, strategy: ExecutionStrategy) -> Self {
        self.graph.execution_strategy = strategy;
        self
    }

    /// Set the maximum number of steps for parallel execution
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.graph.max_steps = max_steps;
        self
    }

    /// Add a node to the graph
    pub fn with_node(
        mut self,
        name: impl Into<String>,
        processor: impl NodeProcessor<S> + 'static,
    ) -> Result<Self> {
        self.graph.add_node(name, processor)?;
        Ok(self)
    }

    /// Add an edge between nodes
    pub fn with_edge(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        condition: Option<EdgeConditionFn<S>>,
    ) -> Result<Self> {
        self.graph.add_edge(from, to, condition)?;
        Ok(self)
    }

    /// Connect a node to the start node
    pub fn with_start_edge(mut self, to: impl Into<String>) -> Result<Self> {
        self.graph.add_start_edge(to)?;
        Ok(self)
    }

    /// Connect a node to the end node
    pub fn with_end_edge(mut self, from: impl Into<String>) -> Result<Self> {
        self.graph.add_end_edge(from)?;
        Ok(self)
    }

    /// Build the graph
    pub fn build(self) -> Graph<S> {
        self.graph
    }
}

/// 单元测试模块，覆盖核心功能：
/// - 串行节点流转
/// - 条件边分支
/// - 并行节点
/// - 错误处理
#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Message, MessageRole};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct TestState {
        counter: Arc<AtomicUsize>,
        messages: Vec<Message>,
    }

    impl StateValue for TestState {}

    struct CounterNode {
        increment: usize,
    }

    #[async_trait]
    impl NodeProcessor<TestState> for CounterNode {
        async fn process(&self, state: State<TestState>) -> Result<State<TestState>> {
            state
                .data
                .counter
                .fetch_add(self.increment, Ordering::SeqCst);
            Ok(state)
        }
    }

    struct MessageNode {
        message: String,
    }

    #[async_trait]
    impl NodeProcessor<TestState> for MessageNode {
        async fn process(&self, mut state: State<TestState>) -> Result<State<TestState>> {
            state
                .data
                .messages
                .push(Message::new(MessageRole::User, self.message.clone()));
            Ok(state)
        }
    }

    #[tokio::test]
    async fn test_sequential_execution() {
        let counter = Arc::new(AtomicUsize::new(0));
        let initial_state = State::new(TestState {
            counter: counter.clone(),
            messages: vec![],
        });

        let graph = GraphBuilder::new()
            .with_node("counter1", CounterNode { increment: 1 })
            .unwrap()
            .with_node("counter2", CounterNode { increment: 2 })
            .unwrap()
            .with_node(
                "message",
                MessageNode {
                    message: "test".to_string(),
                },
            )
            .unwrap()
            .with_start_edge("counter1")
            .unwrap()
            .with_edge("counter1", "counter2", None)
            .unwrap()
            .with_edge("counter2", "message", None)
            .unwrap()
            .with_end_edge("message")
            .unwrap()
            .build();

        let final_state = graph.execute(initial_state).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(final_state.data.messages.len(), 1);
        assert_eq!(final_state.data.messages[0].content, "test");
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let counter = Arc::new(AtomicUsize::new(0));
        let initial_state = State::new(TestState {
            counter: counter.clone(),
            messages: vec![],
        });

        let graph = GraphBuilder::new()
            .with_node("counter1", CounterNode { increment: 1 })
            .unwrap()
            .with_node("counter2", CounterNode { increment: 2 })
            .unwrap()
            .with_node(
                "message",
                MessageNode {
                    message: "test".to_string(),
                },
            )
            .unwrap()
            .with_start_edge("counter1")
            .unwrap()
            .with_start_edge("counter2")
            .unwrap()
            .with_edge("counter1", "message", None)
            .unwrap()
            .with_edge("counter2", "message", None)
            .unwrap()
            .with_end_edge("message")
            .unwrap()
            .with_execution_strategy(ExecutionStrategy::Parallel)
            .build();

        let final_state = graph.execute(initial_state).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(final_state.data.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_conditional_edges() {
        let counter = Arc::new(AtomicUsize::new(0));
        let initial_state = State::new(TestState {
            counter: counter.clone(),
            messages: vec![],
        });

        let condition =
            Arc::new(|state: &State<TestState>| Ok(state.data.counter.load(Ordering::SeqCst) >= 1));

        let graph = GraphBuilder::new()
            .with_node("counter1", CounterNode { increment: 1 })
            .unwrap()
            .with_node("counter2", CounterNode { increment: 2 })
            .unwrap()
            .with_node(
                "message",
                MessageNode {
                    message: "test".to_string(),
                },
            )
            .unwrap()
            .with_start_edge("counter1")
            .unwrap()
            .with_edge("counter1", "counter2", Some(condition))
            .unwrap()
            .with_edge("counter2", "message", None)
            .unwrap()
            .with_end_edge("message")
            .unwrap()
            .build();

        let final_state = graph.execute(initial_state).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(final_state.data.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_cycle_detection() {
        let graph = GraphBuilder::new()
            .with_node("node1", CounterNode { increment: 1 })
            .unwrap()
            .with_node("node2", CounterNode { increment: 2 })
            .unwrap()
            .with_start_edge("node1")
            .unwrap()
            .with_edge("node1", "node2", None)
            .unwrap()
            .with_edge("node2", "node1", None)
            .unwrap()
            // Note: No end edge - this creates a true cycle with no escape
            .build();

        let counter = Arc::new(AtomicUsize::new(0));
        let initial_state = State::new(TestState {
            counter: counter.clone(),
            messages: vec![],
        });

        let result = graph.execute(initial_state).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::CycleDetected(_)));
    }
}
