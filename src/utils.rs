use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::graph::NodeProcessor;
use crate::state::{State, StateValue};
use crate::Result;

/// Type alias for async processor functions to reduce complexity
pub type AsyncProcessorFn<S> =
    Arc<dyn Fn(State<S>) -> Pin<Box<dyn Future<Output = Result<State<S>>> + Send>> + Send + Sync>;

/// Messages state trait for states that contain a messages field
pub trait MessagesState {
    /// Get the messages from the state
    fn get_messages(&self) -> &[crate::schema::Message];

    /// Set the messages in the state
    fn set_messages(&mut self, messages: Vec<crate::schema::Message>);

    /// Add a message to the state
    fn add_message(&mut self, message: crate::schema::Message);
}

/// Add messages to a state that implements MessagesState
pub fn add_messages<S: MessagesState + Clone>(
    state: &mut S,
    messages: Vec<crate::schema::Message>,
) {
    for message in messages {
        state.add_message(message);
    }
}

/// Create a node processor from an async function
pub fn create_node_processor<S, F>(f: F) -> impl NodeProcessor<S>
where
    S: StateValue,
    F: Fn(State<S>) -> Pin<Box<dyn Future<Output = Result<State<S>>> + Send>>
        + Send
        + Sync
        + 'static,
{
    NodeProcessorFn(Arc::new(f))
}

/// A processor that wraps a function
struct NodeProcessorFn<S>(AsyncProcessorFn<S>)
where
    S: StateValue;

#[async_trait]
impl<S> NodeProcessor<S> for NodeProcessorFn<S>
where
    S: StateValue,
{
    async fn process(&self, state: State<S>) -> Result<State<S>> {
        (self.0)(state).await
    }
}

/// A simple state implementation with just messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMessagesState {
    /// The messages in the state
    pub messages: Vec<crate::schema::Message>,
}

impl StateValue for SimpleMessagesState {}

impl MessagesState for SimpleMessagesState {
    fn get_messages(&self) -> &[crate::schema::Message] {
        &self.messages
    }

    fn set_messages(&mut self, messages: Vec<crate::schema::Message>) {
        self.messages = messages;
    }

    fn add_message(&mut self, message: crate::schema::Message) {
        self.messages.push(message);
    }
}
