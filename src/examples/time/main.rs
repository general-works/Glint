use serde_json::json;
use std::fs;
use glint::{
    execution::{Execution, Status},
    node::INode,
    workflow::Workflow,
};

fn main() {
    println!("=== Time Example Workflow Execution ===\n");

    // 1. Read time.json to construct a workflow
    println!("📖 Reading time.json...");
    let json_content = match fs::read_to_string("src/examples/time/time.json") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Cannot read time.json file: {}", e);
            return;
        }
    };

    println!("🔧 Constructing workflow...");
    let workflow = match Workflow::from_json(
        &json_content,
        "Time Example Workflow".to_string(),
        Some("Example workflow demonstrating time node functionality".to_string()),
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

    // 3. Complete step-by-step execution of execution and output step execution results
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

    // 4. Output final status of all nodes and output
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

    // 5. Print final status of all nodes
    println!("\n📈 Final node status:");
    for (node_id, status) in &execution.status {
        if let Some(node) = workflow.nodes.get(node_id) {
            println!("   {} ({}): {:?}", node.name(), node_id, status);
        }
    }

    // 6. Verify output results match expectations
    println!("\n🔍 Verifying output results...");

    // Verify final status
    let all_success = execution
        .status
        .values()
        .all(|status| matches!(status, Status::Success));
    if all_success {
        println!("   ✅ All node execution status correct");
    } else {
        println!("   ❌ Some nodes failed to execute");
    }

    // Verify time node correctly outputs timestamp
    let time_node_id = uuid::Uuid::parse_str("f3a9c1be-7d24-4b8e-9a63-2d5b4c7e1f90").unwrap();
    if let Some(time_output) = execution.output.get(&time_node_id) {
        if let Some(now_value) = time_output.get("now") {
            if let Some(time_str) = now_value.as_str() {
                println!("   ✅ Time node correctly outputs timestamp: {}", time_str);
            } else {
                println!("   ❌ Time node output format incorrect");
            }
        } else {
            println!("   ❌ Time node has no 'now' field output");
        }
    } else {
        println!("   ❌ Time node has no output");
    }

    // Verify merge node correctly merges information
    let merge_node_id = uuid::Uuid::parse_str("a3d7f2c1-6e45-4b9a-8c2f-1d3e5f7a9b20").unwrap();
    if let Some(merge_output) = execution.output.get(&merge_node_id) {
        let has_user = merge_output.get("user").is_some();
        let has_email = merge_output.get("email").is_some();
        let has_now = merge_output.get("now").is_some();

        if has_user && has_email && has_now {
            println!("   ✅ Merge node correctly includes user info and timestamp");
        } else {
            println!(
                "   ❌ Merge node missing required fields (user: {}, email: {}, now: {})",
                has_user, has_email, has_now
            );
        }
    } else {
        println!("   ❌ Merge node has no output");
    }

    // Final verification result
    println!("\n🎉 Verification result:");
    if all_success {
        println!("   ✅ Workflow execution completed, all node status normal!");
    } else {
        println!("   ❌ Workflow execution has issues, please check node status.");
        std::process::exit(1);
    }

    println!("\n=== Example execution completed ===");
}
