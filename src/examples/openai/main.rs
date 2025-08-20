use serde_json::json;
use std::fs;
use glint::{
    execution::{Execution, Status},
    node::INode,
    workflow::Workflow,
};

fn main() {
    println!("=== OpenAI Example Workflow Execution ===\n");

    // 1. Read openai.json to construct a workflow
    println!("📖 Reading openai.json...");
    let json_content = match fs::read_to_string("src/examples/openai/openai.json") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Cannot read openai.json file: {}", e);
            return;
        }
    };

    println!("🔧 Constructing workflow...");
    let workflow = match Workflow::from_json(
        &json_content,
        "OpenAI Example Workflow".to_string(),
        Some("Travel plan generation workflow demonstrating OpenAI node functionality".to_string()),
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

    // Print workflow node information
    println!("\n📋 Workflow Node List:");
    for (node_id, node) in &workflow.nodes {
        println!("   - {} ({}): {}", 
            node.name(), 
            node_id, 
            node.description().unwrap_or_else(|| "No description".to_string())
        );
    }

    // 2. Use this workflow to construct an execution with input {}
    println!("\n🚀 Constructing execution...");
    let input = json!({
        "trigger": "Start generating travel plan"
    });
    let mut execution = Execution::new(&workflow, input);

    println!("✅ Execution constructed successfully!");
    println!("   - ID: {}", execution.id);
    println!("   - Current batch index: {}", execution.current_batch_index);
    println!("   - Total batch count: {}", execution.plan.batch.len());

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

    println!("\n🔄 Starting step-by-step execution...");
    
    // 3. Step-by-step execution of execution and output step results
    let mut step_count = 0;
    while execution.has_next() {
        step_count += 1;
        println!("\n--- Step {} ---", step_count);
        
        // Display current batch information
        let current_batch = execution.plan.batch[execution.current_batch_index].clone();
        println!("📍 Current batch index: {}", execution.current_batch_index);
        println!("📍 Current batch node count: {}", current_batch.len());
        
        // Display nodes to be executed
        println!("🎯 Nodes to be executed:");
        for node_id in &current_batch {
            if let Some(node) = execution.workflow.nodes.get(node_id) {
                println!("   - {} ({})", node.name(), node_id);
                
                // Display current node status
                if let Some(node_status) = execution.status.get(node_id) {
                    println!("     Status: {:?}", node_status);
                }
                
                // If node has output, display output
                if let Some(output) = execution.output.get(node_id) {
                    println!("     Output: {}", 
                        serde_json::to_string_pretty(&output)
                            .unwrap_or_else(|_| "Cannot serialize output".to_string())
                    );
                }
            }
        }

        // Execute current batch
        println!("⚡ Executing current batch...");
        let batch_result = execution.execute();
        
        println!("✅ Batch execution result: {:?}", batch_result);
        
        // Display node status and output after execution
        println!("📊 Node execution results:");
        for node_id in &current_batch {
            if let Some(node) = execution.workflow.nodes.get(node_id) {
                println!("   Node: {} ({})", node.name(), node_id);
                
                // Display node status
                if let Some(node_status) = execution.status.get(node_id) {
                    println!("     Status: {:?}", node_status);
                }
                
                // Display node output
                if let Some(output) = execution.output.get(node_id) {
                    match output {
                        serde_json::Value::String(s) if s.len() > 200 => {
                            println!("     Output: {}...", &s[..200]);
                        }
                        _ => {
                            println!("     Output: {}", 
                                serde_json::to_string_pretty(&output)
                                    .unwrap_or_else(|_| "Cannot serialize output".to_string())
                            );
                        }
                    }
                } else {
                    println!("     Output: None");
                }
            }
        }
        
        // If batch execution fails, stop execution
        if batch_result == Status::Failed {
            println!("❌ Batch execution failed, stopping execution");
            break;
        }
        
        println!("⏭️  Moving to next batch...");
    }

    // 4. Final output of execution's status and output
    println!("\n🏁 === Execution Completed ===");
    
    let final_status = execution.status();
    println!("📊 Final execution status: {:?}", final_status);
    
    println!("\n📋 Final status of all nodes:");
    for (node_id, node) in &execution.workflow.nodes {
        if let Some(node_status) = execution.status.get(node_id) {
            println!("   - {} ({}): {:?}", node.name(), node_id, node_status);
        }
    }
    
    println!("\n📤 Final output results:");
    if execution.output.is_empty() {
        println!("   No output");
    } else {
        for (node_id, output) in &execution.output {
            if let Some(node) = execution.workflow.nodes.get(node_id) {
                println!("   Node: {} ({})", node.name(), node_id);
                
                // Truncate display for long outputs
                match output {
                    serde_json::Value::String(s) if s.len() > 500 => {
                        println!("     Output: {}...", &s[..500]);
                        println!("     (Output truncated, total length: {} characters)", s.len());
                    }
                    serde_json::Value::Object(_) => {
                        let output_str = serde_json::to_string_pretty(&output)
                            .unwrap_or_else(|_| "Cannot serialize output".to_string());
                        if output_str.len() > 1000 {
                            // Safe truncation to avoid character boundary issues
                            let truncated = output_str.chars().take(1000).collect::<String>();
                            println!("     Output: {}...", truncated);
                            println!("     (Output truncated, total length: {} characters)", output_str.chars().count());
                        } else {
                            println!("     Output: {}", output_str);
                        }
                    }
                    _ => {
                        println!("     Output: {}", 
                            serde_json::to_string_pretty(&output)
                                .unwrap_or_else(|_| "Cannot serialize output".to_string())
                        );
                    }
                }
            }
        }
    }
    
    // Summary
    println!("\n📈 Execution Summary:");
    println!("   - Total steps: {}", step_count);
    println!("   - Total batches: {}", execution.plan.batch.len());
    println!("   - Total nodes: {}", execution.workflow.nodes.len());
    println!("   - Final status: {:?}", final_status);
    
    match final_status {
        Status::Success => println!("🎉 Workflow execution successful!"),
        Status::Failed => println!("❌ Workflow execution failed!"),
        Status::Running => println!("🔄 Workflow still running..."),
        Status::Pending => println!("⏸️  Workflow waiting..."),
    }
}
