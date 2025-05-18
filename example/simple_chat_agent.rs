//! Example: Minimal chat agent using Glint
//! Run with: cargo run --bin chat

use async_trait::async_trait;
use glint::graph::{GraphBuilder, NodeProcessor};
use glint::llms::MockLLM;
use glint::schema::{Message, MessageRole};
use glint::state::State;
use glint::traits::Runnable;
use glint::utils::SimpleMessagesState;
use glint::Result;

/// A simple chat processor node that uses an LLM to generate responses
struct ChatProcessor {
    llm: MockLLM,
}

#[async_trait]
impl NodeProcessor<SimpleMessagesState> for ChatProcessor {
    async fn process(
        &self,
        mut state: State<SimpleMessagesState>,
    ) -> Result<State<SimpleMessagesState>> {
        // Get the last message from the state
        if let Some(last_message) = state.data.messages.last() {
            // Generate a response using the LLM
            let response = self.llm.invoke(last_message.content.clone()).await?;

            // Add the response to the state
            state
                .data
                .messages
                .push(Message::new(MessageRole::Assistant, response));
        }

        Ok(state)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a mock LLM for testing
    let llm = MockLLM::new();

    // Create a chat processor
    let chat_processor = ChatProcessor { llm };

    // Build your workflow
    let graph = GraphBuilder::new()
        .with_node("chat", chat_processor)?
        .with_start_edge("chat")?
        .with_end_edge("chat")?
        .build();

    // Create initial state with a greeting message
    let initial_state = State::new(SimpleMessagesState {
        messages: vec![Message::new(
            MessageRole::User,
            "Hello! How can I help you today?",
        )],
    });

    // Run your workflow
    let final_state = graph.execute(initial_state).await?;

    // Print the conversation
    for message in final_state.data.messages {
        println!("{}: {}", format!("{:?}", message.role), message.content);
    }

    Ok(())
}
