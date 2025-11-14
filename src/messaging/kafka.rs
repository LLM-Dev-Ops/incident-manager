//! Kafka message queue implementation

use crate::messaging::config::KafkaConfig;
use crate::messaging::error::{MessagingError, MessagingResult};
use crate::messaging::traits::{MessageConsumer, MessageProducer, MessageStream};
use async_trait::async_trait;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::OwnedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Kafka producer
pub struct KafkaProducer {
    producer: Arc<FutureProducer>,
    #[allow(dead_code)]
    config: KafkaConfig,
}

impl KafkaProducer {
    /// Create a new Kafka producer
    pub async fn new(config: KafkaConfig) -> MessagingResult<Self> {
        let mut client_config = ClientConfig::new();
        client_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("client.id", &config.client_id)
            .set("compression.type", &config.compression_type)
            .set("message.timeout.ms", config.message_timeout_ms.to_string())
            .set("retries", config.retries.to_string());

        if config.enable_sasl {
            if let (Some(mechanism), Some(username), Some(password)) = (
                &config.sasl_mechanism,
                &config.sasl_username,
                &config.sasl_password,
            ) {
                client_config
                    .set("security.protocol", "SASL_SSL")
                    .set("sasl.mechanism", mechanism)
                    .set("sasl.username", username)
                    .set("sasl.password", password);
            }
        }

        let producer: FutureProducer = client_config
            .create()
            .map_err(|e| MessagingError::ConnectionFailed(format!("Kafka producer creation failed: {}", e)))?;

        Ok(Self {
            producer: Arc::new(producer),
            config,
        })
    }
}

#[async_trait]
impl MessageProducer for KafkaProducer {
    async fn publish<T: Serialize + Send + Sync>(&self, topic: &str, message: &T) -> MessagingResult<()> {
        let payload = serde_json::to_string(message)?;

        let record: FutureRecord<'_, str, str> = FutureRecord::to(topic).payload(&payload);

        self.producer
            .send(record, Duration::from_secs(0))
            .await
            .map_err(|(e, _)| MessagingError::PublishFailed(format!("Kafka publish failed: {}", e)))?;

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
        // Kafka producer doesn't have an explicit connected state
        true
    }

    async fn close(&self) -> MessagingResult<()> {
        // Kafka producer flushes on drop
        Ok(())
    }
}

/// Kafka consumer
pub struct KafkaConsumer {
    consumer: Arc<StreamConsumer>,
    #[allow(dead_code)]
    config: KafkaConfig,
}

impl KafkaConsumer {
    /// Create a new Kafka consumer
    pub async fn new(config: KafkaConfig) -> MessagingResult<Self> {
        let mut client_config = ClientConfig::new();
        client_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("group.id", &config.group_id)
            .set("client.id", &config.client_id)
            .set("enable.auto.commit", config.enable_auto_commit.to_string())
            .set(
                "auto.commit.interval.ms",
                config.auto_commit_interval_ms.to_string(),
            )
            .set("session.timeout.ms", config.session_timeout_ms.to_string());

        if config.enable_sasl {
            if let (Some(mechanism), Some(username), Some(password)) = (
                &config.sasl_mechanism,
                &config.sasl_username,
                &config.sasl_password,
            ) {
                client_config
                    .set("security.protocol", "SASL_SSL")
                    .set("sasl.mechanism", mechanism)
                    .set("sasl.username", username)
                    .set("sasl.password", password);
            }
        }

        let consumer: StreamConsumer = client_config
            .create()
            .map_err(|e| MessagingError::ConnectionFailed(format!("Kafka consumer creation failed: {}", e)))?;

        Ok(Self {
            consumer: Arc::new(consumer),
            config,
        })
    }
}

#[async_trait]
impl MessageConsumer for KafkaConsumer {
    async fn subscribe<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        topic: &str,
    ) -> MessagingResult<Box<dyn MessageStream<T>>> {
        self.consumer
            .subscribe(&[topic])
            .map_err(|e| MessagingError::SubscribeFailed(format!("Kafka subscribe failed: {}", e)))?;

        Ok(Box::new(KafkaMessageStream::new(
            self.consumer.clone(),
            None,
        )))
    }

    async fn consume_one<T: DeserializeOwned>(
        &self,
        topic: &str,
        _timeout_ms: u64,
    ) -> MessagingResult<Option<T>> {
        self.consumer
            .subscribe(&[topic])
            .map_err(|e| MessagingError::SubscribeFailed(format!("Kafka subscribe failed: {}", e)))?;

        match self
            .consumer
            .recv()
            .await
            .map_err(|e| MessagingError::ConsumeFailed(format!("Kafka recv failed: {}", e)))?
        {
            msg => {
                if let Some(payload) = msg.payload() {
                    let message: T = serde_json::from_slice(payload)?;
                    Ok(Some(message))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn close(&self) -> MessagingResult<()> {
        Ok(())
    }
}

/// Kafka message stream
pub struct KafkaMessageStream<T> {
    consumer: Arc<StreamConsumer>,
    current_message: Option<OwnedMessage>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> KafkaMessageStream<T> {
    fn new(consumer: Arc<StreamConsumer>, current_message: Option<OwnedMessage>) -> Self {
        Self {
            consumer,
            current_message,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T: DeserializeOwned + Send + Sync> MessageStream<T> for KafkaMessageStream<T> {
    async fn next(&mut self) -> MessagingResult<Option<T>> {
        match self
            .consumer
            .recv()
            .await
            .map_err(|e| MessagingError::ConsumeFailed(format!("Kafka recv failed: {}", e)))?
        {
            msg => {
                self.current_message = Some(msg.detach());

                if let Some(payload) = self.current_message.as_ref().and_then(|m| m.payload()) {
                    let message: T = serde_json::from_slice(payload)?;
                    Ok(Some(message))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn ack(&mut self) -> MessagingResult<()> {
        // Auto-commit handles this if enabled
        // Manual commit is not supported with OwnedMessage, use consumer.commit() instead
        self.consumer
            .commit_consumer_state(rdkafka::consumer::CommitMode::Async)
            .map_err(|e| MessagingError::ConsumeFailed(format!("Kafka commit failed: {}", e)))?;
        Ok(())
    }

    async fn nack(&mut self) -> MessagingResult<()> {
        // Kafka doesn't have explicit nack - consumer will re-fetch on rebalance
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_config() {
        let config = KafkaConfig::default();
        assert_eq!(config.client_id, "llm-incident-manager");
        assert_eq!(config.compression_type, "snappy");
    }
}
