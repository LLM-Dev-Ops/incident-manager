//! Main messaging service

use crate::messaging::config::{MessagingBackend, MessagingConfig};
use crate::messaging::error::{MessagingError, MessagingResult};
use crate::messaging::events::{IncidentEvent, MessageEnvelope};
use crate::messaging::kafka::{KafkaConsumer, KafkaProducer};
use crate::messaging::metrics::MESSAGING_METRICS;
use crate::messaging::nats::{NatsConsumer, NatsProducer};
use crate::messaging::traits::{MessageConsumer, MessageProducer};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Main messaging service providing unified access to message queues
pub struct MessagingService {
    config: MessagingConfig,
    nats_producer: Option<Arc<NatsProducer>>,
    nats_consumer: Option<Arc<NatsConsumer>>,
    kafka_producer: Option<Arc<KafkaProducer>>,
    kafka_consumer: Option<Arc<KafkaConsumer>>,
}

impl MessagingService {
    /// Create a new messaging service
    pub async fn new(config: MessagingConfig) -> MessagingResult<Self> {
        if !config.enabled {
            return Ok(Self {
                config,
                nats_producer: None,
                nats_consumer: None,
                kafka_producer: None,
                kafka_consumer: None,
            });
        }

        let (nats_producer, nats_consumer) = match config.backend {
            MessagingBackend::Nats | MessagingBackend::Both => {
                let producer = NatsProducer::new(config.nats.clone()).await?;
                let consumer = NatsConsumer::new(config.nats.clone()).await?;
                (Some(Arc::new(producer)), Some(Arc::new(consumer)))
            }
            _ => (None, None),
        };

        let (kafka_producer, kafka_consumer) = match config.backend {
            MessagingBackend::Kafka | MessagingBackend::Both => {
                let producer = KafkaProducer::new(config.kafka.clone()).await?;
                let consumer = KafkaConsumer::new(config.kafka.clone()).await?;
                (Some(Arc::new(producer)), Some(Arc::new(consumer)))
            }
            _ => (None, None),
        };

        // Initialize metrics
        if config.enable_metrics {
            crate::messaging::metrics::init_messaging_metrics();
        }

        Ok(Self {
            config,
            nats_producer,
            nats_consumer,
            kafka_producer,
            kafka_consumer,
        })
    }

    /// Publish a message to a topic
    pub async fn publish<T: Serialize + Send + Sync>(
        &self,
        topic: &str,
        message: &T,
    ) -> MessagingResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let full_topic = self.config.full_topic(topic);
        let start = Instant::now();

        let result = match self.config.backend {
            MessagingBackend::Nats => {
                if let Some(ref producer) = self.nats_producer {
                    producer.publish(&full_topic, message).await
                } else {
                    Err(MessagingError::BackendUnavailable("NATS".to_string()))
                }
            }
            MessagingBackend::Kafka => {
                if let Some(ref producer) = self.kafka_producer {
                    producer.publish(&full_topic, message).await
                } else {
                    Err(MessagingError::BackendUnavailable("Kafka".to_string()))
                }
            }
            MessagingBackend::Both => {
                // Publish to both backends
                if let Some(ref producer) = self.nats_producer {
                    producer.publish(&full_topic, message).await?;
                }
                if let Some(ref producer) = self.kafka_producer {
                    producer.publish(&full_topic, message).await?;
                }
                Ok(())
            }
        };

        // Record metrics
        if self.config.enable_metrics {
            let backend_name = format!("{:?}", self.config.backend);
            let duration = start.elapsed().as_secs_f64();

            if result.is_ok() {
                MESSAGING_METRICS
                    .messages_published
                    .with_label_values(&[topic, &backend_name])
                    .inc();

                MESSAGING_METRICS
                    .publish_latency
                    .with_label_values(&[topic, &backend_name])
                    .observe(duration);
            } else {
                MESSAGING_METRICS
                    .publish_failures
                    .with_label_values(&[topic, &backend_name, "unknown"])
                    .inc();
            }
        }

        result
    }

    /// Publish an incident event
    pub async fn publish_incident_event(&self, event: IncidentEvent) -> MessagingResult<()> {
        let topic = format!("incidents.{}", event.event_type().to_lowercase());
        let envelope = MessageEnvelope::new(event);

        self.publish(&topic, &envelope).await
    }

    /// Publish incident created event
    pub async fn publish_incident_created(
        &self,
        incident_id: String,
        severity: String,
        incident_type: String,
        title: String,
    ) -> MessagingResult<()> {
        let event = IncidentEvent::Created {
            incident_id,
            severity,
            incident_type,
            title,
        };
        self.publish_incident_event(event).await
    }

    /// Publish incident state changed event
    pub async fn publish_incident_state_changed(
        &self,
        incident_id: String,
        old_state: String,
        new_state: String,
    ) -> MessagingResult<()> {
        let event = IncidentEvent::StateChanged {
            incident_id,
            old_state,
            new_state,
        };
        self.publish_incident_event(event).await
    }

    /// Publish incident resolved event
    pub async fn publish_incident_resolved(
        &self,
        incident_id: String,
        resolution_time_secs: u64,
    ) -> MessagingResult<()> {
        let event = IncidentEvent::Resolved {
            incident_id,
            resolution_time_secs,
        };
        self.publish_incident_event(event).await
    }

    /// Subscribe to a topic and receive messages
    pub async fn subscribe<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        topic: &str,
    ) -> MessagingResult<Box<dyn crate::messaging::traits::MessageStream<T>>> {
        if !self.config.enabled {
            return Err(MessagingError::BackendUnavailable("Messaging disabled".to_string()));
        }

        let full_topic = self.config.full_topic(topic);

        match self.config.backend {
            MessagingBackend::Nats => {
                if let Some(ref consumer) = self.nats_consumer {
                    consumer.subscribe(&full_topic).await
                } else {
                    Err(MessagingError::BackendUnavailable("NATS".to_string()))
                }
            }
            MessagingBackend::Kafka => {
                if let Some(ref consumer) = self.kafka_consumer {
                    consumer.subscribe(&full_topic).await
                } else {
                    Err(MessagingError::BackendUnavailable("Kafka".to_string()))
                }
            }
            MessagingBackend::Both => {
                // Default to NATS for subscriptions
                if let Some(ref consumer) = self.nats_consumer {
                    consumer.subscribe(&full_topic).await
                } else if let Some(ref consumer) = self.kafka_consumer {
                    consumer.subscribe(&full_topic).await
                } else {
                    Err(MessagingError::BackendUnavailable("Both".to_string()))
                }
            }
        }
    }

    /// Check if the service is connected
    pub async fn is_connected(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        match self.config.backend {
            MessagingBackend::Nats => {
                if let Some(ref producer) = self.nats_producer {
                    producer.is_connected().await
                } else {
                    false
                }
            }
            MessagingBackend::Kafka => {
                if let Some(ref producer) = self.kafka_producer {
                    producer.is_connected().await
                } else {
                    false
                }
            }
            MessagingBackend::Both => {
                let nats_connected = if let Some(ref producer) = self.nats_producer {
                    producer.is_connected().await
                } else {
                    false
                };

                let kafka_connected = if let Some(ref producer) = self.kafka_producer {
                    producer.is_connected().await
                } else {
                    false
                };

                nats_connected || kafka_connected
            }
        }
    }

    /// Close all connections
    pub async fn close(&self) -> MessagingResult<()> {
        if let Some(ref producer) = self.nats_producer {
            producer.close().await?;
        }

        if let Some(ref consumer) = self.nats_consumer {
            consumer.close().await?;
        }

        if let Some(ref producer) = self.kafka_producer {
            producer.close().await?;
        }

        if let Some(ref consumer) = self.kafka_consumer {
            consumer.close().await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_messaging_service_creation() {
        let config = MessagingConfig {
            enabled: false,
            ..Default::default()
        };

        let service = MessagingService::new(config).await;
        assert!(service.is_ok());
    }

    #[test]
    fn test_messaging_config() {
        let config = MessagingConfig::default();
        assert_eq!(config.topic_prefix, "llm-im");

        let full_topic = config.full_topic("test");
        assert_eq!(full_topic, "llm-im.test");
    }
}
