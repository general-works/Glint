use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::execution::Plan;
use crate::node::{INode, Node, NodeId};
use crate::util;

type WorkflowId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
    pub description: Option<String>,
    pub nodes: HashMap<NodeId, Node>,
    pub connections: Connections,
}

pub type Connections = HashMap<NodeId, Vec<NodeId>>;

/// Parse connections from JSON
pub fn connections_from_json(connections_obj: &Value) -> util::Result<Connections> {
    let mut edges = HashMap::new();

    for (node_id_str, targets) in connections_obj.as_object().unwrap() {
        let node_id = Uuid::parse_str(node_id_str)
            .map_err(|_| util::Error::Configuration(format!("Invalid source node ID: {}", node_id_str)))?;

        let mut target_ids = Vec::new();
        for target in targets.as_array().unwrap() {
            if let Some(target_str) = target.as_str() {
                let target_id = Uuid::parse_str(target_str).map_err(|_| {
                    util::Error::Configuration(format!("Invalid target node ID: {}", target_str))
                })?;
                target_ids.push(target_id);
            }
        }
        edges.insert(node_id, target_ids);
    }

    Ok(edges)
}

/// Validate that all keys and values in connections must be keys in nodes, and detect cycles
pub fn validate_connections(
    connections: &Connections,
    nodes: &HashMap<NodeId, Node>,
) -> util::Result<()> {
    for (source_node_id, target_node_ids) in connections {
        // Validate source node exists
        if !nodes.contains_key(source_node_id) {
            return Err(util::Error::Configuration(format!(
                "Source node {} in connections does not exist in node list",
                source_node_id
            )));
        }

        // Validate all target nodes exist
        for target_node_id in target_node_ids {
            if !nodes.contains_key(target_node_id) {
                return Err(util::Error::Configuration(format!(
                    "Target node {} in connections does not exist in node list",
                    target_node_id
                )));
            }
        }
    }

    // Detect cycles
    detect_cycles(connections)?;

    Ok(())
}

/// Use depth-first search to detect cycles in the graph
fn detect_cycles(connections: &Connections) -> util::Result<()> {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    // Perform DFS cycle detection for each node
    for &node_id in connections.keys() {
        if !visited.contains(&node_id) {
            if has_cycle_dfs(connections, node_id, &mut visited, &mut rec_stack) {
                return Err(util::Error::Configuration(format!(
                    "Circular dependency detected, node {} is involved in a cycle",
                    node_id
                )));
            }
        }
    }

    Ok(())
}

/// Helper function for depth-first search cycle detection
fn has_cycle_dfs(
    connections: &Connections,
    node_id: NodeId,
    visited: &mut HashSet<NodeId>,
    rec_stack: &mut HashSet<NodeId>,
) -> bool {
    visited.insert(node_id);
    rec_stack.insert(node_id);

    // Get all neighbors of current node
    if let Some(neighbors) = connections.get(&node_id) {
        for &neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if has_cycle_dfs(connections, neighbor, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(&neighbor) {
                // If neighbor is already in recursion stack, cycle exists
                return true;
            }
        }
    }

    rec_stack.remove(&node_id);
    false
}

impl Workflow {
    pub fn from_json(
        json_str: &str,
        name: String,
        description: Option<String>,
    ) -> util::Result<Self> {
        let json_value: Value = serde_json::from_str(json_str)?;

        // Parse nodes
        let nodes = if let Some(nodes_obj) = json_value.get("nodes") {
            Node::parse_nodes_from_json(nodes_obj)?
        } else {
            return Err(util::Error::Configuration("nodes field does not exist".to_string()));
        };

        // Parse connections
        let connections = if let Some(connections_obj) = json_value.get("connections") {
            connections_from_json(connections_obj)?
        } else {
            return Err(util::Error::Configuration(
                "connections field does not exist".to_string(),
            ));
        };

        // Validate all nodes
        for (node_id, node) in &nodes {
            if !node.validate() {
                return Err(util::Error::Configuration(format!(
                    "Node {} ({}) validation failed",
                    node_id,
                    node.name()
                )));
            }
        }

        // Create workflow object
        let workflow = Workflow {
            id: Uuid::new_v4(),
            name,
            description,
            nodes,
            connections,
        };

        // Perform complete workflow validation (including node reference checks)
        if !workflow.validate() {
            return Err(util::Error::Configuration(
                "Workflow validation failed: invalid node references exist".to_string(),
            ));
        }

        Ok(workflow)
    }

    pub fn validate(&self) -> bool {
        // Validate basic validity of all nodes
        for (_, node) in &self.nodes {
            if !node.validate() {
                return false;
            }
        }

        // Validate connection validity
        if validate_connections(&self.connections, &self.nodes).is_err() {
            return false;
        }

        // Validate expression batch number dependencies
        if !self.validate_expression_batch_dependencies() {
            return false;
        }

        true
    }

    /// Validate expression batch number dependencies
    /// Ensure that nodes referenced by expressions are executed in earlier batches
    fn validate_expression_batch_dependencies(&self) -> bool {
        // Generate execution plan
        let plan = Plan::new(&self.connections);

        // Create mapping from node ID to batch number
        let mut node_to_batch = HashMap::new();
        for (batch_index, batch) in plan.batch.iter().enumerate() {
            for &node_id in batch {
                node_to_batch.insert(node_id, batch_index);
            }
        }

        // Validate expression dependencies for each node
        for (node_id, node) in &self.nodes {
            if let Some(current_batch) = node_to_batch.get(node_id) {
                // Use node's dependencies method to get list of dependent node IDs
                let dependencies = node.dependencies();

                // Validate batch number of each dependent node
                for dep_node_id in dependencies {
                    if let Some(&dep_batch) = node_to_batch.get(&dep_node_id) {
                        // Validate that dependent node's batch number must be strictly less than current node's batch number
                        if dep_batch >= *current_batch {
                            return false;
                        }
                    } else {
                        // Dependent node not in execution plan (node doesn't exist)
                        return false;
                    }
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_validate_valid_node_references() {
        let workflow_json = r#"
        {
            "nodes": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "trigger for workflow"
                },
                "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                    "type": "set",
                    "name": "set node",
                    "description": "Set with valid reference",
                    "parameters": {
                        "trigger_data": "=${2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e}.output"
                    }
                }
            },
            "connections": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
            }
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        assert!(workflow.validate());
    }

    #[test]
    fn test_workflow_validate_invalid_node_references() {
        let workflow_json = r#"
        {
            "nodes": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "trigger for workflow"
                },
                "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                    "type": "set",
                    "name": "set node",
                    "description": "Set with invalid reference",
                    "parameters": {
                        "invalid_ref": "=${99999999-9999-4999-9999-999999999999}.output"
                    }
                }
            },
            "connections": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
            }
        }
        "#;

        // Should now succeed in creation because we removed expression validation
        // But will fail in batch dependency validation because referenced nodes don't exist
        let result = Workflow::from_json(workflow_json, "test".to_string(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Workflow validation failed"));
    }

    #[test]
    fn test_workflow_validate_nested_references() {
        let workflow_json = r#"
        {
            "nodes": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "trigger for workflow"
                },
                "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                    "type": "set",
                    "name": "set node",
                    "description": "Set with nested references",
                    "parameters": {
                        "nested": {
                            "data": "=${2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e}.output",
                            "array": [
                                "=${2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e}.items[0]",
                                "normal value"
                            ]
                        }
                    }
                }
            },
            "connections": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
            }
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        assert!(workflow.validate());
    }

    #[test]
    fn test_workflow_validate_invalid_connections() {
        let workflow_json = r#"
        {
            "nodes": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "trigger for workflow"
                }
            },
            "connections": {
                "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["99999999-9999-4999-9999-999999999999"]
            }
        }
        "#;

        let result = Workflow::from_json(workflow_json, "test".to_string(), None);
        assert!(result.is_err());

        // Connection validation should be caught in workflow.validate()
        assert!(result.unwrap_err().to_string().contains("Workflow validation failed"));
    }

    #[test]
    fn test_workflow_validate_valid_batch_dependencies() {
        // Test valid batch dependencies: nodes in earlier batches are referenced by nodes in later batches
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Trigger node - batch 0",
                    "parameters": {
                        "trigger": "start"
                    }
                },
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                    "type": "set",
                    "name": "set node 1",
                    "description": "Set node - batch 1",
                    "parameters": {
                        "trigger_data": "=${aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa}.trigger"
                    }
                },
                "cccccccc-cccc-cccc-cccc-cccccccccccc": {
                    "type": "set",
                    "name": "set node 2", 
                    "description": "Set node - batch 2",
                    "parameters": {
                        "previous_data": "=${bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb}.result"
                    }
                }
            },
            "connections": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"],
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": ["cccccccc-cccc-cccc-cccc-cccccccccccc"]
            }
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        assert!(workflow.validate());
    }

    #[test]
    fn test_workflow_validate_invalid_batch_dependencies_same_batch() {
        // Test invalid batch dependencies: nodes in the same batch reference each other
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Trigger node",
                    "parameters": {
                        "trigger": "start"
                    }
                },
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                    "type": "set",
                    "name": "set node 1",
                    "description": "Set node 1 - same batch as set node 2",
                    "parameters": {
                        "sibling_data": "=${cccccccc-cccc-cccc-cccc-cccccccccccc}.data"
                    }
                },
                "cccccccc-cccc-cccc-cccc-cccccccccccc": {
                    "type": "set",
                    "name": "set node 2",
                    "description": "Set node 2 - same batch as set node 1",
                    "parameters": {
                        "trigger_data": "=${aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa}.trigger"
                    }
                }
            },
            "connections": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb", "cccccccc-cccc-cccc-cccc-cccccccccccc"]
            }
        }
        "#;

        let result = Workflow::from_json(workflow_json, "test".to_string(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Workflow validation failed"));
    }

    #[test]
    fn test_workflow_validate_invalid_batch_dependencies_later_batch() {
        // Test invalid batch dependencies: nodes in later batches are referenced by nodes in earlier batches
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "set",
                    "name": "set node early",
                    "description": "Set node - early batch",
                    "parameters": {
                        "future_data": "=${bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb}.result"
                    }
                },
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                    "type": "set",
                    "name": "set node later",
                    "description": "Set node - later batch",
                    "parameters": {
                        "data": "some data"
                    }
                }
            },
            "connections": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"]
            }
        }
        "#;

        let result = Workflow::from_json(workflow_json, "test".to_string(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Workflow validation failed"));
    }

    #[test]
    fn test_workflow_validate_nested_expression_batch_dependencies() {
        // Test batch dependencies in nested expressions
        let workflow_json = r#"
        {
            "nodes": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                    "type": "trigger",
                    "name": "trigger node",
                    "description": "Trigger node",
                    "parameters": {
                        "trigger": "start"
                    }
                },
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                    "type": "set",
                    "name": "set node",
                    "description": "Set node with nested expressions",
                    "parameters": {
                        "config": {
                            "source": "=${aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa}.trigger",
                            "items": [
                                "=${aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa}.data",
                                "static value"
                            ]
                        }
                    }
                }
            },
            "connections": {
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"]
            }
        }
        "#;

        let workflow = Workflow::from_json(workflow_json, "test".to_string(), None).unwrap();
        assert!(workflow.validate());
    }
}
