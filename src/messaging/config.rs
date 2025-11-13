//! Messaging configuration

use serde::{Deserialize, Serialize};

/// Messaging backend type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessagingBackend {
    /// NATS - lightweight, high-performance messaging
    Nats,
    /// Kafka - distributed event streaming platform
    Kafka,
    /// Both NATS and Kafka
    Both,
}

/// NATS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URLs
    pub servers: Vec<String>,

    /// Connection name
    pub connection_name: String,

    /// Enable TLS
    pub enable_tls: bool,

    /// Maximum reconnect attempts
    pub max_reconnects: Option<usize>,

    /// Reconnect delay in milliseconds
    pub reconnect_delay_ms: u64,

    /// Ping interval in seconds
    pub ping_interval_secs: u64,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            servers: vec!["nats://localhost:4222".to_string()],
            connection_name: "llm-incident-manager".to_string(),
            enable_tls: false,
            max_reconnects: Some(10),
            reconnect_delay_ms: 1000,
            ping_interval_secs: 60,
            request_timeout_ms: 5000,
        }
    }
}

/// Kafka configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Kafka bootstrap servers
    pub bootstrap_servers: String,

    /// Client ID
    pub client_id: String,

    /// Consumer group ID
    pub group_id: String,

    /// Enable auto commit
    pub enable_auto_commit: bool,

    /// Auto commit interval in milliseconds
    pub auto_commit_interval_ms: u64,

    /// Session timeout in milliseconds
    pub session_timeout_ms: u64,

    /// Enable SASL authentication
    pub enable_sasl: bool,

    /// SASL mechanism (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512)
    pub sasl_mechanism: Option<String>,

    /// SASL username
    pub sasl_username: Option<String>,

    /// SASL password
    pub sasl_password: Option<String>,

    /// Enable SSL/TLS
    pub enable_ssl: bool,

    /// Compression type (none, gzip, snappy, lz4, zstd)
    pub compression_type: String,

    /// Message timeout in milliseconds
    pub message_timeout_ms: u64,

    /// Number of retries
    pub retries: u32,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            client_id: "llm-incident-manager".to_string(),
            group_id: "incident-manager-group".to_string(),
            enable_auto_commit: true,
            auto_commit_interval_ms: 5000,
            session_timeout_ms: 30000,
            enable_sasl: false,
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            enable_ssl: false,
            compression_type: "snappy".to_string(),
            message_timeout_ms: 30000,
            retries: 3,
        }
    }
}

/// Main messaging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingConfig {
    /// Backend to use
    pub backend: MessagingBackend,

    /// NATS configuration
    pub nats: NatsConfig,

    /// Kafka configuration
    pub kafka: KafkaConfig,

    /// Enable messaging system
    pub enabled: bool,

    /// Default topic prefix
    pub topic_prefix: String,

    /// Enable dead letter queue
    pub enable_dlq: bool,

    /// Dead letter topic
    pub dlq_topic: String,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Enable metrics
    pub enable_metrics: bool,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            backend: MessagingBackend::Nats,
            nats: NatsConfig::default(),
            kafka: KafkaConfig::default(),
            enabled: true,
            topic_prefix: "llm-im".to_string(),
            enable_dlq: true,
            dlq_topic: "dead-letter".to_string(),
            max_message_size: 1_048_576, // 1MB
            enable_metrics: true,
        }
    }
}

impl MessagingConfig {
    /// Get full topic name with prefix
    pub fn full_topic(&self, topic: &str) -> String {
        format!("{}.{}", self.topic_prefix, topic)
    }
}
