//! Example: Conditional branch workflow with Glint
//! Run with: cargo run --bin conditional_branch

use async_trait::async_trait;
use glint::graph::{GraphBuilder, NodeProcessor};
use glint::state::State;
use glint::Result;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct BranchState {
    pub value: i32,
}

impl glint::state::StateValue for BranchState {}

struct IncNode;

#[async_trait]
impl NodeProcessor<BranchState> for IncNode {
    async fn process(&self, mut state: State<BranchState>) -> Result<State<BranchState>> {
        state.data.value += 1;
        Ok(state)
    }
}

struct DecNode;

#[async_trait]
impl NodeProcessor<BranchState> for DecNode {
    async fn process(&self, mut state: State<BranchState>) -> Result<State<BranchState>> {
        state.data.value -= 1;
        Ok(state)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a condition that checks if the value is greater than 0
    let cond = Arc::new(|state: &State<BranchState>| Ok(state.data.value > 0));

    // Build the graph
    let graph = GraphBuilder::new()
        .with_node("inc", IncNode)?
        .with_node("dec", DecNode)?
        .with_start_edge("inc")?
        .with_edge("inc", "dec", Some(cond))?
        .with_end_edge("dec")?
        .build();

    // Create initial state with value 1
    let initial_state = State::new(BranchState { value: 1 });

    // Execute the graph
    let final_state = graph.execute(initial_state).await?;

    // Print the final value
    println!("Final value: {}", final_state.data.value);

    Ok(())
}
