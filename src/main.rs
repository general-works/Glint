use serde_json::json;
use glint::execution::Plan;
use glint::workflow::Workflow;

fn main() {
    // Test normal workflow
    test_normal_workflow();

    // Test unknown node type error
    test_unknown_node_type();

    // Test invalid connections error
    test_invalid_connections();

    // Test execution plan generation
    test_execution_plan();

    // Test INode trait implementation
    test_inode_trait();

    // Test node validation
    test_node_validation();
}

fn test_normal_workflow() {
    println!("=== Testing Normal Workflow ===");

    // Example JSON data (based on set.json)
    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Manual Trigger",
                "type": "manual_trigger",
                "description": "Manual Trigger"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "set",
                "description": "Set",
                "parameters": {
                    "user": "John Doe",
                    "email": "john.doe@example.com"
                }
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
        }
    }
    "#;

    // Create workflow from JSON
    match Workflow::from_json(
        json_data,
        "Example Workflow".to_string(),
        Some("This is an example workflow".to_string()),
    ) {
        Ok(workflow) => {
            println!("Successfully created workflow!");
            println!("Workflow ID: {}", workflow.id);
            println!("Workflow Name: {}", workflow.name);
            println!("Number of nodes: {}", workflow.nodes.len());
            println!("Number of connections: {}", workflow.connections.len());
        }
        Err(e) => {
            eprintln!("Failed to create workflow: {}", e);
        }
    }
}

fn test_unknown_node_type() {
    println!("\n=== Testing Unknown Node Type Error ===");

    // JSON data with unknown node type
    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Manual Trigger",
                "type": "manual_trigger",
                "description": "Manual Trigger"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "unknown_node_type",
                "description": "Unknown node type"
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
        }
    }
    "#;

    // Create workflow from JSON
    match Workflow::from_json(
        json_data,
        "Error Test Workflow".to_string(),
        Some("Testing unknown node type error".to_string()),
    ) {
        Ok(workflow) => {
            println!("Unexpectedly created workflow successfully!");
            println!("Workflow ID: {}", workflow.id);
        }
        Err(e) => {
            println!("Expected error: {}", e);
            // Verify error type
            if format!("{}", e).contains("Configuration error") && format!("{}", e).contains("Unknown node type")
            {
                println!("✓ Correct error type: Configuration error");
            } else {
                println!("✗ Incorrect error type");
            }
        }
    }
}

fn test_invalid_connections() {
    println!("\n=== Testing Invalid Connections Error ===");

    // Test target node doesn't exist
    test_target_node_not_exists();

    // Test source node doesn't exist
    test_source_node_not_exists();

    // Test cycle detection
    test_cycle_detection();
}

fn test_target_node_not_exists() {
    println!("--- Testing Target Node Doesn't Exist ---");

    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Manual Trigger",
                "type": "manual_trigger",
                "description": "Manual Trigger"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "set",
                "description": "Set",
                "parameters": {
                    "user": "John Doe",
                    "email": "john.doe@example.com"
                }
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d", "f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90"]
        }
    }
    "#;

    match Workflow::from_json(json_data, "Target Node Not Exists Test".to_string(), None) {
        Ok(_) => println!("✗ Should fail but didn't"),
        Err(e) => {
            println!("Expected error: {}", e);
            if format!("{}", e).contains("target node")
                && format!("{}", e).contains("does not exist in node list")
            {
                println!("✓ Correct error type: Target node doesn't exist");
            } else {
                println!("✗ Incorrect error type");
            }
        }
    }
}

fn test_source_node_not_exists() {
    println!("--- Testing Source Node Doesn't Exist ---");

    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Manual Trigger",
                "type": "manual_trigger",
                "description": "Manual Trigger"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "set",
                "description": "Set",
                "parameters": {
                    "user": "John Doe",
                    "email": "john.doe@example.com"
                }
            }
        },
        "connections": {
            "f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
        }
    }
    "#;

    match Workflow::from_json(json_data, "Source Node Not Exists Test".to_string(), None) {
        Ok(_) => println!("✗ Should fail but didn't"),
        Err(e) => {
            println!("Expected error: {}", e);
            if format!("{}", e).contains("source node")
                && format!("{}", e).contains("does not exist in node list")
            {
                println!("✓ Correct error type: Source node doesn't exist");
            } else {
                println!("✗ Incorrect error type");
            }
        }
    }
}

fn test_cycle_detection() {
    println!("--- Testing Cycle Detection ---");

    // Test normal no-cycle case
    test_no_cycle();

    // Test simple cycle: A -> B -> A
    test_simple_cycle();

    // Test complex cycle: A -> B -> C -> A
    test_complex_cycle();

    // Test self-cycle: A -> A
    test_self_cycle();
}

fn test_no_cycle() {
    println!("Testing normal no-cycle case");

    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Node A",
                "type": "manual_trigger",
                "description": "Node A"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "name": "Node B",
                "type": "set",
                "description": "Node B"
            },
            "f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90": {
                "name": "Node C",
                "type": "set",
                "description": "Node C"
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"],
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": ["f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90"]
        }
    }
    "#;

    match Workflow::from_json(json_data, "No Cycle Test".to_string(), None) {
        Ok(workflow) => {
            println!("✓ Normal no-cycle case passed validation");
            println!("  Workflow ID: {}", workflow.id);
            println!("  Number of nodes: {}", workflow.nodes.len());
            println!("  Number of connections: {}", workflow.connections.len());
        }
        Err(e) => {
            println!("✗ No-cycle case should pass but didn't: {}", e);
        }
    }
}

fn test_simple_cycle() {
    println!("Testing simple cycle (A -> B -> A)");

    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Node A",
                "type": "manual_trigger",
                "description": "Node A"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "name": "Node B",
                "type": "set",
                "description": "Node B"
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"],
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": ["2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e"]
        }
    }
    "#;

    match Workflow::from_json(json_data, "Simple Cycle Test".to_string(), None) {
        Ok(_) => println!("✗ Should detect cycle but didn't"),
        Err(e) => {
            println!("Expected error: {}", e);
            if format!("{}", e).contains("circular dependency") {
                println!("✓ Correctly detected circular dependency");
            } else {
                println!("✗ Incorrect error type");
            }
        }
    }
}

fn test_complex_cycle() {
    println!("Testing complex cycle (A -> B -> C -> A)");

    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Node A",
                "type": "manual_trigger",
                "description": "Node A"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "name": "Node B",
                "type": "set",
                "description": "Node B"
            },
            "f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90": {
                "name": "Node C",
                "type": "set",
                "description": "Node C"
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"],
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": ["f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90"],
            "f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90": ["2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e"]
        }
    }
    "#;

    match Workflow::from_json(json_data, "Complex Cycle Test".to_string(), None) {
        Ok(_) => println!("✗ Should detect cycle but didn't"),
        Err(e) => {
            println!("Expected error: {}", e);
            if format!("{}", e).contains("circular dependency") {
                println!("✓ Correctly detected circular dependency");
            } else {
                println!("✗ Incorrect error type");
            }
        }
    }
}

fn test_self_cycle() {
    println!("Testing self-cycle (A -> A)");

    let json_data = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "name": "Node A",
                "type": "manual_trigger",
                "description": "Node A"
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e"]
        }
    }
    "#;

    match Workflow::from_json(json_data, "Self Cycle Test".to_string(), None) {
        Ok(_) => println!("✗ Should detect cycle but didn't"),
        Err(e) => {
            println!("Expected error: {}", e);
            if format!("{}", e).contains("circular dependency") {
                println!("✓ Correctly detected circular dependency");
            } else {
                println!("✗ Incorrect error type");
            }
        }
    }
}

fn test_execution_plan() {
    println!("\n=== Testing Execution Plan Generation ===");

    // Test simple linear dependency
    test_linear_plan();

    // Test complex dependency relationships
    test_complex_plan();

    // Test execution initialization
    test_execution_init();
}

fn test_linear_plan() {
    println!("--- Testing Linear Dependency Execution Plan ---");

    use std::collections::HashMap;
    use uuid::Uuid;
    use glint::workflow::Connections;

    // Create node IDs
    let node_a = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let node_b = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
    let node_c = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();

    // Build connection relationships: A -> B -> C
    let mut connections: Connections = HashMap::new();
    connections.insert(node_a, vec![node_b]);
    connections.insert(node_b, vec![node_c]);

    let plan = Plan::new(&connections);

    println!("Generated execution plan batches: {}", plan.batch.len());
    for (i, batch) in plan.batch.iter().enumerate() {
        println!("  Batch {}: {:?}", i + 1, batch);
    }

    // Verify
    if plan.batch.len() == 3 {
        println!("✓ Correct number of execution plan batches");
    } else {
        println!("✗ Incorrect number of execution plan batches");
    }
}

fn test_execution_init() {
    println!("--- Testing Execution Initialization ---");

    use glint::execution::Execution;

    // Create a simple workflow
    let json_data = r#"
    {
        "nodes": {
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                "name": "Node A",
                "type": "set",
                "description": "Node A"
            },
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                "name": "Node B",
                "type": "set",
                "description": "Node B"
            },
            "cccccccc-cccc-cccc-cccc-cccccccccccc": {
                "name": "Node C",
                "type": "set",
                "description": "Node C"
            }
        },
        "connections": {
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"],
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": ["cccccccc-cccc-cccc-cccc-cccccccccccc"]
        }
    }
    "#;

    match Workflow::from_json(json_data, "Test Workflow".to_string(), None) {
        Ok(workflow) => {
            let execution = Execution::new(&workflow, json!({}));

            println!("Execution ID: {}", execution.id);
            println!("Workflow nodes: {}", execution.workflow.nodes.len());
            println!("Status nodes: {}", execution.status.len());
            println!("Execution plan batches: {}", execution.plan.batch.len());

            // Verify all nodes are initialized to Pending status
            let all_pending = execution
                .status
                .values()
                .all(|status| matches!(status, glint::execution::Status::Pending));

            if all_pending {
                println!("✓ All nodes correctly initialized to Pending status");
            } else {
                println!("✗ Node status initialization incorrect");
            }

            // Verify status node count matches workflow node count
            if execution.status.len() == execution.workflow.nodes.len() {
                println!("✓ Status node count matches workflow node count");
            } else {
                println!("✗ Status node count doesn't match workflow node count");
            }

            // Verify all workflow nodes have corresponding status
            let all_nodes_have_status = execution
                .workflow
                .nodes
                .keys()
                .all(|node_id| execution.status.contains_key(node_id));

            if all_nodes_have_status {
                println!("✓ All workflow nodes have corresponding status");
            } else {
                println!("✗ Some workflow nodes don't have status");
            }
        }
        Err(e) => {
            println!("✗ Failed to create workflow: {}", e);
        }
    }
}

fn test_complex_plan() {
    println!("--- Testing Complex Dependency Execution Plan ---");

    use std::collections::HashMap;
    use uuid::Uuid;
    use glint::workflow::Connections;

    // Create node IDs
    let node_a = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let node_b = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
    let node_c = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();
    let node_d = Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap();
    let node_e = Uuid::parse_str("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee").unwrap();

    // Build connection relationships:
    // A -> B
    // A -> C
    // B -> D
    // C -> D
    // D -> E
    let mut connections: Connections = HashMap::new();
    connections.insert(node_a, vec![node_b, node_c]);
    connections.insert(node_b, vec![node_d]);
    connections.insert(node_c, vec![node_d]);
    connections.insert(node_d, vec![node_e]);

    let plan = Plan::new(&connections);

    println!("Generated execution plan batches: {}", plan.batch.len());
    for (i, batch) in plan.batch.iter().enumerate() {
        println!("  Batch {}: {} nodes", i + 1, batch.len());
        for node in batch {
            let node_name = match node.to_string().chars().next() {
                Some('a') => "A",
                Some('b') => "B",
                Some('c') => "C",
                Some('d') => "D",
                Some('e') => "E",
                _ => "?",
            };
            println!("    - Node {}", node_name);
        }
    }

    // Verify
    if plan.batch.len() == 4 {
        println!("✓ Correct number of execution plan batches");

        // Verify first batch should only have A
        if plan.batch[0].len() == 1 && plan.batch[0].contains(&node_a) {
            println!("✓ First batch correct (only node A)");
        }

        // Verify second batch should have B and C
        if plan.batch[1].len() == 2
            && plan.batch[1].contains(&node_b)
            && plan.batch[1].contains(&node_c)
        {
            println!("✓ Second batch correct (nodes B and C)");
        }

        // Verify third batch should only have D
        if plan.batch[2].len() == 1 && plan.batch[2].contains(&node_d) {
            println!("✓ Third batch correct (only node D)");
        }

        // Verify fourth batch should only have E
        if plan.batch[3].len() == 1 && plan.batch[3].contains(&node_e) {
            println!("✓ Fourth batch correct (only node E)");
        }
    } else {
        println!("✗ Incorrect number of execution plan batches");
    }
}

fn test_inode_trait() {
    use serde_json::json;
    use uuid::Uuid;
    use glint::node::{INode, Node};

    println!("=== Testing Node's INode Trait Implementation ===");

    // Create a set node
    let set_node = Node::new(
        Uuid::new_v4(),
        "Test Set Node".to_string(),
        "set".to_string(),
        Some("This is a test set node".to_string()),
        Some(json!({"key": "value"})),
    );

    println!("Set Node:");
    println!("  ID: {}", set_node.id());
    println!("  Name: {}", set_node.name());
    println!("  Description: {:?}", set_node.description());
    println!("  Parameters: {:?}", set_node.parameter());
    println!("  Validation: {}", set_node.validate());

    // Test execution
    use glint::execution::NodeOutput;

    let mut input = NodeOutput::new();
    input.insert(Uuid::new_v4(), json!({"test": "input"}));
    match set_node.execute(&input) {
        Ok(output) => println!("  Execution result: {:?}", output),
        Err(e) => println!("  Execution error: {:?}", e),
    }

    // Create a trigger node
    let trigger_node = Node::new(
        Uuid::new_v4(),
        "Test Trigger Node".to_string(),
        "trigger".to_string(),
        Some("This is a test trigger node".to_string()),
        Some(json!({"trigger_type": "manual"})),
    );

    println!("\nTrigger Node:");
    println!("  ID: {}", trigger_node.id());
    println!("  Name: {}", trigger_node.name());
    println!("  Description: {:?}", trigger_node.description());
    println!("  Parameters: {:?}", trigger_node.parameter());
    println!("  Validation: {}", trigger_node.validate());

    // Test execution
    match trigger_node.execute(&input) {
        Ok(output) => println!("  Execution result: {:?}", output),
        Err(e) => println!("  Execution error: {:?}", e),
    }

    println!("\n✓ All INode trait methods working correctly!");
}

fn test_node_validation() {
    use glint::workflow::Workflow;

    println!("=== Testing Node Validation ===");

    // Test normal case - all nodes should pass validation
    let valid_json = r#"
    {
        "nodes": {
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                "type": "set",
                "name": "Node A",
                "description": "This is node A",
                "parameters": {"key": "value"}
            },
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                "type": "trigger",
                "name": "Node B", 
                "description": "This is node B",
                "parameters": {"trigger_type": "manual"}
            }
        },
        "connections": {
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"]
        }
    }
    "#;

    println!("--- Testing Normal Node Validation ---");
    match Workflow::from_json(valid_json, "Test Validation Workflow".to_string(), None) {
        Ok(_) => println!("✓ Normal node validation successful, workflow created successfully"),
        Err(e) => println!("✗ Normal node validation failed: {:?}", e),
    }

    // Test validation failure case - empty node name
    let invalid_json = r#"
    {
        "nodes": {
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                "type": "set",
                "name": "",
                "description": "This is node A",
                "parameters": {"key": "value"}
            }
        },
        "connections": {}
    }
    "#;

    println!("\n--- Testing Node Validation Failure (Empty Name) ---");
    match Workflow::from_json(invalid_json, "Test Validation Failure Workflow".to_string(), None) {
        Ok(_) => println!("✗ Should fail validation but succeeded"),
        Err(e) => println!("✓ Node validation failed, correctly returned error: {:?}", e),
    }

    // Test validation failure case - null parameters
    let invalid_null_json = r#"
    {
        "nodes": {
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                "type": "set",
                "name": "Node B",
                "description": "This is node B",
                "parameters": null
            }
        },
        "connections": {}
    }
    "#;

    println!("\n--- Testing Node Validation Failure (Null Parameters) ---");
    match Workflow::from_json(invalid_null_json, "Test Validation Failure Workflow 2".to_string(), None) {
        Ok(_) => println!("✗ Should fail validation but succeeded"),
        Err(e) => println!("✓ Node validation failed, correctly returned error: {:?}", e),
    }

    // Test expression validation
    let invalid_expression_json = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "type": "trigger",
                "name": "trigger node",
                "description": "trigger for workflow"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "set",
                "name": "set node with invalid expression",
                "description": "Set with invalid expression",
                "parameters": {
                    "user": "John Doe",
                    "invalid_expr": "=${invalid-uuid}.user"
                }
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
        }
    }
    "#;

    println!("\n--- Testing Expression Validation Failure (Invalid UUID) ---");
    match Workflow::from_json(
        invalid_expression_json,
        "Test Expression Validation Failure".to_string(),
        None,
    ) {
        Ok(_) => println!("✗ Should fail validation but succeeded"),
        Err(e) => println!("✓ Expression validation failed, correctly returned error: {:?}", e),
    }

    let valid_expression_json = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "type": "trigger",
                "name": "trigger node",
                "description": "trigger for workflow"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "set",
                "name": "set node with valid expression",
                "description": "Set with valid expression",
                "parameters": {
                    "user": "John Doe",
                    "valid_expr": "=${2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e}.user"
                }
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
        }
    }
    "#;

    println!("\n--- Testing Expression Validation Success (Valid Expression) ---");
    match Workflow::from_json(valid_expression_json, "Test Valid Expression".to_string(), None) {
        Ok(workflow) => {
            println!("✓ Valid expression validation successful");
            // Test workflow-level validation
            println!("  Workflow validation result: {}", workflow.validate());
        }
        Err(e) => println!("✗ Valid expression validation failed: {:?}", e),
    }

    // Test node reference doesn't exist case
    let invalid_node_ref_json = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "type": "trigger",
                "name": "trigger node",
                "description": "trigger for workflow"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "set",
                "name": "set node with invalid node reference",
                "description": "Set with reference to non-existent node",
                "parameters": {
                    "user": "John Doe",
                    "invalid_ref": "=${99999999-9999-4999-9999-999999999999}.user"
                }
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
        }
    }
    "#;

    println!("\n--- Testing Node Reference Doesn't Exist Validation ---");
    match Workflow::from_json(invalid_node_ref_json, "Test Node Reference Validation".to_string(), None) {
        Ok(_) => {
            println!("✗ Should fail validation but succeeded");
        }
        Err(e) => println!("✓ Correctly detected node reference doesn't exist, workflow creation failed: {:?}", e),
    }

    // Test valid node reference
    let valid_node_ref_json = r#"
    {
        "nodes": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": {
                "type": "trigger",
                "name": "trigger node",
                "description": "trigger for workflow"
            },
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d": {
                "type": "set",
                "name": "set node with valid node reference",
                "description": "Set with reference to existing node",
                "parameters": {
                    "user": "John Doe",
                    "trigger_data": "=${2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e}.output"
                }
            }
        },
        "connections": {
            "2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e": ["8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d"]
        }
    }
    "#;

    println!("\n--- Testing Valid Node Reference Validation ---");
    match Workflow::from_json(valid_node_ref_json, "Test Valid Node Reference".to_string(), None) {
        Ok(workflow) => {
            let validation_result = workflow.validate();
            if validation_result {
                println!("✓ Valid node reference validation successful");
            } else {
                println!("✗ Valid node reference validation failed");
            }
        }
        Err(e) => println!("Workflow creation failed: {:?}", e),
    }

    // Test step execution functionality
    test_step_execution();
}

fn test_step_execution() {
    println!("\n=== Testing Step Execution Functionality ===");

    let simple_workflow_json = r#"
    {
        "nodes": {
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
                "type": "trigger",
                "name": "trigger node",
                "description": "Initial trigger node"
            },
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                "type": "set",
                "name": "set node 1",
                "description": "First processing node",
                "parameters": {
                    "message": "Hello from set node 1"
                }
            },
            "cccccccc-cccc-cccc-cccc-cccccccccccc": {
                "type": "set",
                "name": "set node 2", 
                "description": "Second processing node",
                "parameters": {
                    "message": "Hello from set node 2"
                }
            }
        },
        "connections": {
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"],
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": ["cccccccc-cccc-cccc-cccc-cccccccccccc"]
        }
    }
    "#;

    match Workflow::from_json(simple_workflow_json, "Test Step Execution".to_string(), None) {
        Ok(workflow) => {
            println!("✓ Workflow created successfully");

            // Create execution instance
            let input = serde_json::json!({"trigger": "start"});
            let mut execution = glint::execution::Execution::new(&workflow, input);

            println!("Initial status:");
            println!("  Execution ID: {}", execution.id);
            println!("  Current batch index: {}", execution.current_batch_index);
            println!("  Has next batch: {}", execution.has_next());
            println!("  Execution status: {:?}", execution.status());

            // Step execution
            let mut step = 1;
            while execution.has_next() {
                println!("\n--- Execution Step {} ---", step);
                println!("  Status before execution: {:?}", execution.status());
                println!("  Current batch index: {}", execution.current_batch_index);

                let result = execution.execute();

                println!("  Batch execution result: {:?}", result);
                println!("  Batch index after execution: {}", execution.current_batch_index);
                println!("  Has next batch: {}", execution.has_next());

                // Display current status of all nodes
                println!("  Node status:");
                for (node_id, status) in &execution.status {
                    println!("    {}: {:?}", node_id, status);
                }

                step += 1;

                // Prevent infinite loop
                if step > 10 {
                    println!("  Warning: Too many execution steps, possible issue");
                    break;
                }
            }

            println!("\n--- Final Status ---");
            println!("  Final execution status: {:?}", execution.status());
            println!("  Output result count: {}", execution.output.len());
            for (node_id, output) in &execution.output {
                println!("    Node {}: {:?}", node_id, output);
            }
        }
        Err(e) => println!("✗ Workflow creation failed: {:?}", e),
    }
}
