# Glint

A Rust framework for building AI applications with LLMs, embeddings, and vector stores.

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