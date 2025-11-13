//! NATS message queue implementation

use crate::messaging::config::NatsConfig;
use crate::messaging::error::{MessagingError, MessagingResult};
use crate::messaging::traits::{MessageConsumer, MessageProducer, MessageStream};
use async_nats::Client;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// NATS producer
pub struct NatsProducer {
    client: Arc<Client>,
    config: NatsConfig,
}

impl NatsProducer {
    /// Create a new NATS producer
    pub async fn new(config: NatsConfig) -> MessagingResult<Self> {
        let client = async_nats::connect(&config.servers[0])
            .await
            .map_err(|e| MessagingError::ConnectionFailed(format!("NATS connection failed: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }
}

#[async_trait]
impl MessageProducer for NatsProducer {
    async fn publish<T: Serialize + Send + Sync>(&self, topic: &str, message: &T) -> MessagingResult<()> {
        let payload = serde_json::to_vec(message)?;

        self.client
            .publish(topic.to_string(), payload.into())
            .await
            .map_err(|e| MessagingError::PublishFailed(format!("NATS publish failed: {}", e)))?;

        Ok(())
    }

    async fn publish_batch<T: Serialize + Send + Sync>(
        &self,
        topic: &str,
        messages: &[T],
    ) -> MessagingResult<usize> {
        for message in messages {
            self.publish(topic, message).await?;
        }
        Ok(messages.len())
    }

    async fn is_connected(&self) -> bool {
        // async_nats Client doesn't have is_closed method
        // We assume connected if the client exists
        true
    }

    async fn close(&self) -> MessagingResult<()> {
        // NATS client closes automatically on drop
        Ok(())
    }
}

/// NATS consumer
pub struct NatsConsumer {
    client: Arc<Client>,
    config: NatsConfig,
}

impl NatsConsumer {
    /// Create a new NATS consumer
    pub async fn new(config: NatsConfig) -> MessagingResult<Self> {
        let client = async_nats::connect(&config.servers[0])
            .await
            .map_err(|e| MessagingError::ConnectionFailed(format!("NATS connection failed: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }
}

#[async_trait]
impl MessageConsumer for NatsConsumer {
    async fn subscribe<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        topic: &str,
    ) -> MessagingResult<Box<dyn MessageStream<T>>> {
        let subscriber = self
            .client
            .subscribe(topic.to_string())
            .await
            .map_err(|e| MessagingError::SubscribeFailed(format!("NATS subscribe failed: {}", e)))?;

        Ok(Box::new(NatsMessageStream::new(subscriber)))
    }

    async fn consume_one<T: DeserializeOwned>(
        &self,
        topic: &str,
        timeout_ms: u64,
    ) -> MessagingResult<Option<T>> {
        let mut subscriber = self
            .client
            .subscribe(topic.to_string())
            .await
            .map_err(|e| MessagingError::SubscribeFailed(format!("NATS subscribe failed: {}", e)))?;

        let timeout = Duration::from_millis(timeout_ms);
        let message_future = subscriber.next();

        match tokio::time::timeout(timeout, message_future).await {
            Ok(Some(msg)) => {
                let payload: T = serde_json::from_slice(&msg.payload)?;
                Ok(Some(payload))
            }
            Ok(None) => Ok(None),
            Err(_) => Err(MessagingError::Timeout("NATS consume timeout".to_string())),
        }
    }

    async fn is_connected(&self) -> bool {
        // async_nats Client doesn't have is_closed method
        // We assume connected if the client exists
        true
    }

    async fn close(&self) -> MessagingResult<()> {
        Ok(())
    }
}

/// NATS message stream
pub struct NatsMessageStream<T> {
    subscriber: async_nats::Subscriber,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> NatsMessageStream<T> {
    fn new(subscriber: async_nats::Subscriber) -> Self {
        Self {
            subscriber,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T: DeserializeOwned + Send + Sync> MessageStream<T> for NatsMessageStream<T> {
    async fn next(&mut self) -> MessagingResult<Option<T>> {
        match self.subscriber.next().await {
            Some(msg) => {
                let payload: T = serde_json::from_slice(&msg.payload)?;
                Ok(Some(payload))
            }
            None => Ok(None),
        }
    }

    async fn ack(&mut self) -> MessagingResult<()> {
        // NATS core doesn't require explicit acks (use JetStream for that)
        Ok(())
    }

    async fn nack(&mut self) -> MessagingResult<()> {
        // NATS core doesn't support nack (use JetStream for that)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nats_config() {
        let config = NatsConfig::default();
        assert!(!config.servers.is_empty());
        assert_eq!(config.connection_name, "llm-incident-manager");
    }
}
