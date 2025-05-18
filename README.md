# Glint

Glint is a high-performance Rust framework for building stateful, graph-based AI systems. It enables developers to construct dynamic, multi-step workflows powered by LLMs, embeddings, and vector stores—executed through an async, checkpointable state machine. Inspired by modern AI orchestration tools, Glint provides a fast and memory-safe foundation for building agent runtimes, autonomous pipelines, and complex control flows.

At its core, Glint leverages Rust’s zero-cost abstractions to deliver predictable performance, thread safety, and fine-grained control over state transitions. Its graph-based architecture makes it easy to model conditional logic, parallel branches, and persistent memory—all within a composable and extensible framework.

## Features

- **Graph-based Workflows**: Build complex AI workflows using a graph-based architecture
- **LLM Integration**: Support for OpenAI and other LLM providers
- **Embeddings**: Generate and work with embeddings for semantic search
- **Vector Stores**: Store and search vectors efficiently
- **Checkpoints**: Save and restore workflow state
- **Async**: Built with async/await for high performance

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
glint = "0.1.0"
```

## Quick Start

```rust
use glint::graph::{GraphBuilder, NodeProcessor};
use glint::state::State;
use glint::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
struct MyState {
    value: i32,
}

impl glint::state::StateValue for MyState {}

struct MyProcessor;

#[async_trait]
impl NodeProcessor<MyState> for MyProcessor {
    async fn process(&self, mut state: State<MyState>) -> Result<State<MyState>> {
        state.data.value += 1;
        Ok(state)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let graph = GraphBuilder::new()
        .with_node("node", MyProcessor)?
        .with_start_edge("node")?
        .with_end_edge("node")?
        .build();

    let initial_state = State::new(MyState { value: 0 });
    let final_state = graph.execute(initial_state).await?;
    println!("Final value: {}", final_state.data.value);
    Ok(())
}
```

## Examples

- [Simple Chat Agent](example/simple_chat_agent.rs)
- [Conditional Branch](example/conditional_branch.rs)
- [Embeddings](example/embeddings_example.rs)
- [Vector Store](example/vector_store_example.rs)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

MIT
