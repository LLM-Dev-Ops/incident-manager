//! Enterprise-grade message queue system for event streaming
//!
//! This module provides a unified interface for message queue operations using
//! multiple backends (NATS, Kafka) for high-throughput event streaming.
//!
//! # Features
//!
//! - **Multi-Backend Support**: NATS for low-latency messaging, Kafka for durable event logs
//! - **Pub/Sub Patterns**: Publish-subscribe messaging for event distribution
//! - **Event Streaming**: Real-time incident event streaming
//! - **Guaranteed Delivery**: At-least-once delivery semantics
//! - **Dead Letter Queues**: Failed message handling
//! - **Metrics Integration**: Prometheus metrics for monitoring
//! - **Circuit Breaker**: Fault tolerance with circuit breaker pattern
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │         Messaging Service API                    │
//! ├─────────────────────────────────────────────────┤
//! │  - publish()      - subscribe()                  │
//! │  - stream()       - consume()                   │
//! └─────────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────────┐
//! │      Message Queue Abstraction                   │
//! ├─────────────────────────────────────────────────┤
//! │  - MessageProducer trait                         │
//! │  - MessageConsumer trait                         │
//! └─────────────────────────────────────────────────┘
//!           │                        │
//!           ▼                        ▼
//! ┌──────────────────┐    ┌──────────────────┐
//! │  NATS Backend    │    │  Kafka Backend   │
//! ├──────────────────┤    ├──────────────────┤
//! │ - Low latency    │    │ - Durability     │
//! │ - Lightweight    │    │ - Replay         │
//! │ - At-most-once   │    │ - Partitioning   │
//! └──────────────────┘    └──────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use llm_incident_manager::messaging::{MessagingService, MessagingConfig, IncidentEvent};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = MessagingConfig::default();
//!     let messaging = MessagingService::new(config).await?;
//!
//!     // Publish an event
//!     let event = IncidentEvent::Created {
//!         incident_id: "inc-001".to_string(),
//!         severity: "P0".to_string(),
//!     };
//!
//!     messaging.publish("incidents.created", &event).await?;
//!
//!     Ok(())
//! }
//! ```

mod config;
mod error;
mod events;
mod kafka;
mod metrics;
mod nats;
mod service;
mod traits;

pub use config::{KafkaConfig, MessagingBackend, MessagingConfig, NatsConfig};
pub use error::{MessagingError, MessagingResult};
pub use events::{IncidentEvent, MessageEnvelope, MessageMetadata};
pub use metrics::{init_messaging_metrics, MESSAGING_METRICS};
pub use service::MessagingService;
pub use traits::{MessageConsumer, MessageProducer};
