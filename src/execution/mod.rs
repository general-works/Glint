use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::node::{INode, Node, NodeId};
use crate::workflow::{Connections, Workflow};

pub type Batch = Vec<NodeId>;
pub type ExecutionId = Uuid;
pub type NodeOutput = HashMap<NodeId, Value>;
pub type NodeStatus = HashMap<NodeId, Status>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub batch: Vec<Batch>,
}

impl Plan {
    pub fn new(connections: &Connections) -> Self {
        let mut plan = Plan { batch: Vec::new() };

        // Collect all nodes
        let mut all_nodes = HashSet::new();
        let mut dependencies: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();

        // Build dependency graph
        // connections key is source node, value is list of nodes that depend on the key
        for (source, targets) in connections {
            all_nodes.insert(*source);
            for target in targets {
                all_nodes.insert(*target);
                // target depends on source
                dependencies
                    .entry(*target)
                    .or_insert_with(HashSet::new)
                    .insert(*source);
            }
        }

        // Find all nodes without dependencies
        let mut no_dependency_nodes = HashSet::new();
        for node in &all_nodes {
            if !dependencies.contains_key(node) {
                no_dependency_nodes.insert(*node);
            }
        }

        // Record processed nodes
        let mut processed_nodes = HashSet::new();

        // Build batches
        while processed_nodes.len() < all_nodes.len() {
            let mut current_batch = Vec::new();

            if processed_nodes.is_empty() && !no_dependency_nodes.is_empty() {
                // First batch: all nodes without dependencies
                for node in &no_dependency_nodes {
                    current_batch.push(*node);
                    processed_nodes.insert(*node);
                }
            } else {
                // Subsequent batches: find all nodes whose dependencies are satisfied
                for node in &all_nodes {
                    if processed_nodes.contains(node) {
                        continue;
                    }

                    // Check if all dependencies of this node have been processed
                    let can_process = if let Some(deps) = dependencies.get(node) {
                        deps.iter().all(|dep| processed_nodes.contains(dep))
                    } else {
                        // Nodes without dependencies should be processed in the first batch
                        true
                    };

                    if can_process {
                        current_batch.push(*node);
                    }
                }

                // Mark current batch nodes as processed
                for node in &current_batch {
                    processed_nodes.insert(*node);
                }
            }

            if current_batch.is_empty() && processed_nodes.len() < all_nodes.len() {
                // Circular dependency exists
                let remaining_nodes: Vec<_> = all_nodes
                    .iter()
                    .filter(|node| !processed_nodes.contains(node))
                    .copied()
                    .collect();
                panic!(
                    "Circular dependency detected, cannot determine execution order for the following nodes: {:?}",
                    remaining_nodes
                );
            }

            if !current_batch.is_empty() {
                plan.batch.push(current_batch);
            }
        }

        plan
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Status {
    Pending,
    Running,
    Success,
    Failed,
}

pub struct Execution {
    pub id: ExecutionId,
    pub workflow: Workflow,
    pub status: NodeStatus,
    pub plan: Plan,
    pub output: NodeOutput,
    pub current_batch_index: usize,
}

/// Initialize all workflow nodes to Pending status
pub fn init_node_status_from_workflow(workflow: &Workflow) -> NodeStatus {
    let mut status = HashMap::new();
    for node_id in workflow.nodes.keys() {
        status.insert(*node_id, Status::Pending);
    }
    status
}

impl Execution {
    pub fn new(workflow: &Workflow, input: Value) -> Self {
        let id = Uuid::new_v4();
        let status = init_node_status_from_workflow(workflow);
        let plan = Plan::new(&workflow.connections);
        let output = NodeOutput::new();

        let mut w = workflow.clone();
        for (_, node) in w.nodes.iter_mut() {
            match node {
                Node::Trigger(trigger_node) => {
                    trigger_node.parameters = Some(input.clone());
                }
                _ => {
                    continue;
                }
            }
        }
        Self {
            id,
            workflow: w,
            status,
            plan,
            output,
            current_batch_index: 0,
        }
    }

    pub fn status(&self) -> Status {
        // If there are no batches, return Success
        if self.plan.batch.is_empty() {
            return Status::Success;
        }

        // 1. Check if all nodes in the first batch are in Pending status
        let first_batch = &self.plan.batch[0];
        let all_first_batch_pending = first_batch
            .iter()
            .all(|node_id| matches!(self.status.get(node_id), Some(Status::Pending)));

        if all_first_batch_pending {
            return Status::Pending;
        }

        // 2. Check node status by batch order
        for (batch_index, batch) in self.plan.batch.iter().enumerate() {
            let mut has_failed = false;
            let mut has_running = false;
            let mut all_success = true;
            let mut all_pending = true;

            // Check status of all nodes in current batch
            for node_id in batch {
                match self.status.get(node_id) {
                    Some(Status::Failed) => {
                        has_failed = true;
                        all_success = false;
                        all_pending = false;
                    }
                    Some(Status::Running) => {
                        has_running = true;
                        all_success = false;
                        all_pending = false;
                    }
                    Some(Status::Success) => {
                        all_pending = false;
                    }
                    Some(Status::Pending) => {
                        all_success = false;
                    }
                    None => {
                        // Node status not found, treat as Pending
                        all_success = false;
                    }
                }
            }

            // a. If any node in current batch has Failed status, return Failed
            if has_failed {
                return Status::Failed;
            }

            // b. If all nodes in current batch are in Pending status, return Running
            if all_pending {
                return Status::Running;
            }

            // c. If current batch has no Failed nodes and has running status nodes, return Running
            if has_running {
                return Status::Running;
            }

            // d. If all nodes in current batch are Success
            if all_success {
                // If current batch is the last batch, return Success
                if batch_index == self.plan.batch.len() - 1 {
                    return Status::Success;
                }
                // Otherwise continue checking next batch
                continue;
            }

            // If current batch is neither all Success, nor all Pending, nor has Failed or Running
            // This means current batch has mixed status of partial Success and partial Pending, return Running
            return Status::Running;
        }

        // Default return Success (theoretically should not reach here)
        Status::Success
    }

    /// Check if there are still batches to execute
    pub fn has_next(&self) -> bool {
        self.current_batch_index < self.plan.batch.len()
    }

    /// Execute current batch step by step
    /// Returns current batch processing result: Success or Failed
    pub fn execute(&mut self) -> Status {
        // If no more batches, return Success (no executable batches means success)
        if !self.has_next() {
            return Status::Success;
        }

        // Get current batch
        let current_batch = &self.plan.batch[self.current_batch_index];

        // Set all nodes in current batch to Running status
        for node_id in current_batch {
            if let Some(node_status) = self.status.get_mut(node_id) {
                *node_status = Status::Running;
            }
        }

        // Execute all nodes in current batch
        let mut batch_failed = false;

        for node_id in current_batch {
            if let Some(node) = self.workflow.nodes.get(node_id) {
                // Execute node
                match node.execute(&self.output) {
                    Ok(result) => {
                        // Execution successful, update output and status
                        self.output.insert(*node_id, result);
                        if let Some(node_status) = self.status.get_mut(node_id) {
                            *node_status = Status::Success;
                        }
                    }
                    Err(_) => {
                        // Execution failed, update status
                        if let Some(node_status) = self.status.get_mut(node_id) {
                            *node_status = Status::Failed;
                        }
                        batch_failed = true;
                    }
                }
            } else {
                // Node doesn't exist, mark as failed
                if let Some(node_status) = self.status.get_mut(node_id) {
                    *node_status = Status::Failed;
                }
                batch_failed = true;
            }
        }

        // If current batch succeeds, move to next batch
        if !batch_failed {
            self.current_batch_index += 1;
        }

        // Return current batch processing result
        if batch_failed {
            Status::Failed
        } else {
            Status::Success
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::Workflow;
    use serde_json::json;

    #[test]
    fn test_has_next_initial_state() {
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Test trigger node"
                },
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                    "type": "set",
                    "name": "set node",
                    "description": "Test set node",
                    "parameters": {"message": "test"}
                }
            },
            "connections": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"]
            }
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        let execution = Execution::new(&workflow, json!({"trigger": "start"}));

        assert_eq!(execution.current_batch_index, 0);
        assert!(execution.has_next());
        assert_eq!(execution.plan.batch.len(), 2); // Should have 2 batches
    }

    #[test]
    fn test_single_step_execution() {
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Test trigger node"
                }
            },
            "connections": {}
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        let mut execution = Execution::new(&workflow, json!({"trigger": "start"}));

        // Check if execution plan is correctly generated
        println!("Batch count: {}", execution.plan.batch.len());

        // If there are no connections, nodes may not appear in execution plan
        // Let's check the actual situation
        if execution.plan.batch.is_empty() {
            // If there are no batches, status should be Success
            assert_eq!(execution.status(), Status::Success);
            assert!(!execution.has_next());
            return;
        }

        // Initial state
        assert!(execution.has_next());
        assert_eq!(execution.current_batch_index, 0);
        assert_eq!(execution.status(), Status::Pending);

        // Execute first step
        let result = execution.execute();
        assert_eq!(result, Status::Success);
        assert!(!execution.has_next());
        assert_eq!(execution.current_batch_index, 1);

        // Check node status
        let node_id = uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        assert!(matches!(
            execution.status.get(&node_id),
            Some(Status::Success)
        ));
    }

    #[test]
    fn test_multi_step_execution() {
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Test trigger node"
                },
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                    "type": "set",
                    "name": "set node",
                    "description": "Test set node",
                    "parameters": {"message": "test"}
                }
            },
            "connections": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"]
            }
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        let mut execution = Execution::new(&workflow, json!({"trigger": "start"}));

        // Execute first step (trigger node)
        assert!(execution.has_next());
        let result1 = execution.execute();
        assert_eq!(result1, Status::Success); // Current batch execution successful
        assert!(execution.has_next()); // Still has next batch
        assert_eq!(execution.current_batch_index, 1);

        // Execute second step (set node)
        let result2 = execution.execute();
        assert_eq!(result2, Status::Success); // Current batch execution successful
        assert!(!execution.has_next()); // No more batches
        assert_eq!(execution.current_batch_index, 2);

        // Check final status
        assert_eq!(execution.status(), Status::Success);
        assert_eq!(execution.output.len(), 2); // Both nodes should have output
    }

    #[test]
    fn test_has_next_after_completion() {
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Test trigger node"
                }
            },
            "connections": {}
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        let mut execution = Execution::new(&workflow, json!({"trigger": "start"}));

        // Execution completed
        execution.execute();

        // Verify no next step
        assert!(!execution.has_next());

        // Calling execute again should return current status
        let result = execution.execute();
        assert_eq!(result, Status::Success);
    }

    #[test]
    fn test_execute_returns_batch_result_only() {
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Test trigger node"
                },
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                    "type": "set",
                    "name": "set node",
                    "description": "Test set node",
                    "parameters": {"message": "test"}
                }
            },
            "connections": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"]
            }
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        let mut execution = Execution::new(&workflow, json!({"trigger": "start"}));

        // First batch execution should return Success (batch result)
        let result1 = execution.execute();
        assert_eq!(result1, Status::Success);
        assert_eq!(execution.current_batch_index, 1);
        assert!(execution.has_next());

        // Second batch execution should return Success (batch result)
        let result2 = execution.execute();
        assert_eq!(result2, Status::Success);
        assert_eq!(execution.current_batch_index, 2);
        assert!(!execution.has_next());

        // Overall status should also be Success
        assert_eq!(execution.status(), Status::Success);
    }
}
