//! Messaging trait abstractions

use crate::messaging::error::MessagingResult;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

/// Message producer trait
#[async_trait]
pub trait MessageProducer: Send + Sync {
    /// Publish a message to a topic
    async fn publish<T: Serialize + Send + Sync>(&self, topic: &str, message: &T) -> MessagingResult<()>;

    /// Publish multiple messages to a topic
    async fn publish_batch<T: Serialize + Send + Sync>(
        &self,
        topic: &str,
        messages: &[T],
    ) -> MessagingResult<usize>;

    /// Check if the producer is connected
    async fn is_connected(&self) -> bool;

    /// Close the producer connection
    async fn close(&self) -> MessagingResult<()>;
}

/// Message consumer trait
#[async_trait]
pub trait MessageConsumer: Send + Sync {
    /// Subscribe to a topic and receive messages
    async fn subscribe<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        topic: &str,
    ) -> MessagingResult<Box<dyn MessageStream<T>>>;

    /// Consume a single message from a topic (blocking)
    async fn consume_one<T: DeserializeOwned>(
        &self,
        topic: &str,
        timeout_ms: u64,
    ) -> MessagingResult<Option<T>>;

    /// Check if the consumer is connected
    async fn is_connected(&self) -> bool;

    /// Close the consumer connection
    async fn close(&self) -> MessagingResult<()>;
}

/// Message stream trait for consuming messages
#[async_trait]
pub trait MessageStream<T: DeserializeOwned>: Send + Sync {
    /// Get the next message from the stream
    async fn next(&mut self) -> MessagingResult<Option<T>>;

    /// Acknowledge message processing
    async fn ack(&mut self) -> MessagingResult<()>;

    /// Negative acknowledge (requeue message)
    async fn nack(&mut self) -> MessagingResult<()>;
}
