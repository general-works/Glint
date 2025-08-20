use serde_json::json;
use std::fs;
use glint::{
    execution::{Execution, Status},
    node::INode,
    workflow::Workflow,
};

fn main() {
    println!("=== Set Example Workflow Execution ===\n");

    // 1. Read set.json to construct a workflow
    println!("📖 Reading set.json...");
    let json_content = match fs::read_to_string("src/examples/set/set.json") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Cannot read set.json file: {}", e);
            return;
        }
    };

    println!("🔧 Constructing workflow...");
    let workflow = match Workflow::from_json(
        &json_content,
        "Set Example Workflow".to_string(),
        Some("Example workflow demonstrating set node functionality".to_string()),
    ) {
        Ok(wf) => wf,
        Err(e) => {
            eprintln!("❌ Cannot construct workflow: {}", e);
            return;
        }
    };

    println!("✅ Workflow constructed successfully!");
    println!("   - ID: {}", workflow.id);
    println!("   - Name: {}", workflow.name);
    println!(
        "   - Description: {}",
        workflow.description.as_deref().unwrap_or("None")
    );
    println!("   - Number of nodes: {}", workflow.nodes.len());
    println!("   - Number of connections: {}", workflow.connections.len());

    // 2. Use this workflow to construct an execution with input {}
    println!("\n🚀 Constructing execution...");
    let input = json!({});
    let mut execution = Execution::new(&workflow, input);

    println!("✅ Execution constructed successfully!");
    println!("   - Execution ID: {}", execution.id);
    println!("   - Current status: {:?}", execution.status());
    println!("   - Execution plan batches: {}", execution.plan.batch.len());

    // Print execution plan
    println!("\n📋 Execution Plan:");
    for (batch_index, batch) in execution.plan.batch.iter().enumerate() {
        println!("   Batch {}: {} nodes", batch_index, batch.len());
        for node_id in batch {
            if let Some(node) = workflow.nodes.get(node_id) {
                println!("     - {} ({})", node.name(), node_id);
            }
        }
    }

    // Print node dependencies
    println!("\n🔗 Node Dependencies:");
    for (node_id, node) in &workflow.nodes {
        let deps = node.dependencies();
        println!("   {} ({}):", node.name(), node_id);
        if deps.is_empty() {
            println!("     No dependencies");
        } else {
            println!("     Depends on {} nodes:", deps.len());
            for dep_id in &deps {
                if let Some(dep_node) = workflow.nodes.get(dep_id) {
                    println!("       - {} ({})", dep_node.name(), dep_id);
                } else {
                    println!("       - Unknown node ({})", dep_id);
                }
            }
        }
    }

    // 3. Execute this execution and print the execution's status and output
    println!("\n⚡ Starting workflow execution...");

    let mut step = 0;
    while execution.has_next() {
        step += 1;
        println!("\n--- Execution Step {} ---", step);

        println!("  Status before execution: {:?}", execution.status());
        let batch_result = execution.execute();
        println!("  Batch execution result: {:?}", batch_result);
        println!("  Status after execution: {:?}", execution.status());

        // Print current node status
        println!("  Node status details:");
        for (node_id, status) in &execution.status {
            if let Some(node) = workflow.nodes.get(node_id) {
                println!("    {} ({}): {:?}", node.name(), node_id, status);
            }
        }
    }

    println!("\n🎯 Workflow execution completed!");
    println!("   - Final execution status: {:?}", execution.status());
    println!("   - Output result count: {}", execution.output.len());

    // Print detailed output results
    println!("\n📊 Execution output results:");
    for (node_id, output) in &execution.output {
        if let Some(node) = workflow.nodes.get(node_id) {
            println!("   {} ({}):", node.name(), node_id);
            println!(
                "     {}",
                serde_json::to_string_pretty(output).unwrap_or_else(|_| "Cannot serialize".to_string())
            );
        }
    }

    // 4. Verify output results match expectations
    println!("\n🔍 Verifying output results...");

    let expected_results = vec![
        // trigger node outputs the empty input parameters
        ("2f7d3c9a-1b42-4c83-9e5a-3a7f6d2c1b0e", json!({})),
        // first set node should output user info
        (
            "8e1f2a34-5c67-4b89-a1d2-3f4e5a6b7c8d",
            json!({
                "user": "John Doe",
                "email": "john.doe@example.com"
            }),
        ),
        // second set node should output user and mobile
        (
            "f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90",
            json!({
                "user": "John Doe",
                "mobile": "+0123456789"
            }),
        ),
        // third set node should merge outputs from first two nodes
        (
            "a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20",
            json!({
                "user": "John Doe",
                "email": "john.doe@example.com",
                "mobile": "+0123456789"
            }),
        ),
    ];

    let mut all_correct = true;
    for (expected_node_id_str, expected_output) in expected_results {
        let expected_node_id = uuid::Uuid::parse_str(expected_node_id_str).unwrap();

        if let Some(actual_output) = execution.output.get(&expected_node_id) {
            if actual_output == &expected_output {
                println!("   ✅ Node {} output correct", expected_node_id_str);
            } else {
                println!("   ❌ Node {} output mismatch", expected_node_id_str);
                println!(
                    "      Expected: {}",
                    serde_json::to_string_pretty(&expected_output).unwrap()
                );
                println!(
                    "      Actual: {}",
                    serde_json::to_string_pretty(actual_output).unwrap()
                );
                all_correct = false;
            }
        } else {
            println!("   ❌ Node {} has no output", expected_node_id_str);
            all_correct = false;
        }
    }

    // Verify final status
    let all_success = execution
        .status
        .values()
        .all(|status| matches!(status, Status::Success));
    if all_success {
        println!("   ✅ All node execution status correct");
    } else {
        println!("   ❌ Some nodes failed to execute");
        all_correct = false;
    }

    // Final verification result
    println!("\n🎉 Verification result:");
    if all_correct {
        println!("   ✅ All verifications passed! Workflow execution results completely match expectations.");
    } else {
        println!("   ❌ Some verifications failed! Please check output results.");
        std::process::exit(1);
    }

    println!("\n=== Example execution completed ===");
}
