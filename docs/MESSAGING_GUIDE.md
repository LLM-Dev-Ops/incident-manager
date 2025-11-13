# Message Queue Integration Guide

## Overview

The messaging module provides enterprise-grade message queue integration for the LLM Incident Manager, supporting both **NATS** (low-latency, lightweight) and **Apache Kafka** (high-durability, distributed) backends. It enables event-driven architectures, real-time notifications, and distributed processing across the incident management system.

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────┐
│              MessagingService (Unified API)              │
├─────────────────────────────────────────────────────────┤
│  ┌──────────────────┐       ┌────────────────────────┐ │
│  │ MessageProducer  │       │  MessageConsumer       │ │
│  │  - publish()     │       │  - subscribe()         │ │
│  │  - batch()       │       │  - consume_one()       │ │
│  └──────────────────┘       └────────────────────────┘ │
├──────────────────┬──────────────────┬───────────────────┤
│  NATS Backend    │  Kafka Backend   │  Metrics Layer   │
│  - async-nats    │  - rdkafka       │  - Prometheus    │
│  - Low latency   │  - Durability    │  - Counters      │
│  - Lightweight   │  - Replication   │  - Histograms    │
└──────────────────┴──────────────────┴───────────────────┘
```

### Design Principles

1. **Backend Abstraction**: Unified API works with NATS, Kafka, or both simultaneously
2. **Type Safety**: Generic serialization with compile-time guarantees
3. **Thread Safety**: All components are `Send + Sync` for concurrent usage
4. **Production Ready**: Comprehensive error handling, metrics, and testing
5. **Event-Driven**: Built-in incident event types with structured envelopes

## Configuration

### Basic Configuration

```rust
use llm_incident_manager::messaging::{
    MessagingConfig, MessagingBackend, NatsConfig, KafkaConfig
};

// NATS configuration
let config = MessagingConfig {
    enabled: true,
    backend: MessagingBackend::Nats,
    nats: NatsConfig {
        servers: vec!["nats://localhost:4222".to_string()],
        connection_name: "llm-im".to_string(),
        max_reconnect_attempts: 10,
        reconnect_delay_ms: 1000,
        ..Default::default()
    },
    topic_prefix: "llm-im".to_string(),
    enable_metrics: true,
    ..Default::default()
};

let service = MessagingService::new(config).await?;
```

### Kafka Configuration

```rust
let config = MessagingConfig {
    enabled: true,
    backend: MessagingBackend::Kafka,
    kafka: KafkaConfig {
        bootstrap_servers: "localhost:9092".to_string(),
        client_id: "llm-im".to_string(),
        group_id: "llm-im-group".to_string(),
        compression_type: "snappy".to_string(),
        enable_auto_commit: true,
        ..Default::default()
    },
    topic_prefix: "llm-im".to_string(),
    enable_dlq: true,
    dlq_topic: "llm-im-dlq".to_string(),
    ..Default::default()
};

let service = MessagingService::new(config).await?;
```

### Dual Backend Configuration

```rust
let config = MessagingConfig {
    enabled: true,
    backend: MessagingBackend::Both,
    nats: NatsConfig::default(),
    kafka: KafkaConfig::default(),
    ..Default::default()
};

// Publishes to both NATS and Kafka
// Subscribes to NATS by default (can configure priority)
let service = MessagingService::new(config).await?;
```

## Usage

### Publishing Messages

#### Simple Publishing

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct AlertMessage {
    alert_id: String,
    severity: String,
    message: String,
}

let alert = AlertMessage {
    alert_id: "alert-001".to_string(),
    severity: "critical".to_string(),
    message: "Database connection lost".to_string(),
};

// Publish to topic
service.publish("alerts.critical", &alert).await?;
```

#### Publishing Incident Events

The module provides pre-built incident event types:

```rust
use llm_incident_manager::messaging::IncidentEvent;

// Incident created
service.publish_incident_created(
    "inc-001".to_string(),
    "critical".to_string(),
    "outage".to_string(),
    "Production database down".to_string(),
).await?;

// State changed
service.publish_incident_state_changed(
    "inc-001".to_string(),
    "new".to_string(),
    "acknowledged".to_string(),
).await?;

// Incident resolved
service.publish_incident_resolved(
    "inc-001".to_string(),
    3600, // resolution time in seconds
).await?;
```

#### Custom Event Publishing

```rust
let event = IncidentEvent::Escalated {
    incident_id: "inc-001".to_string(),
    escalation_level: 2,
};

service.publish_incident_event(event).await?;
```

### Consuming Messages

#### Subscribe to Topic

```rust
use llm_incident_manager::messaging::MessageEnvelope;

// Subscribe to alerts
let mut stream = service.subscribe::<MessageEnvelope<AlertMessage>>("alerts.critical").await?;

// Consume messages
while let Ok(Some(envelope)) = stream.next().await {
    let alert = envelope.payload;
    println!("Received alert: {:?}", alert);

    // Acknowledge processing
    stream.ack().await?;
}
```

#### Consume Single Message

```rust
// Consume one message with timeout
let envelope: Option<MessageEnvelope<AlertMessage>> =
    service.consume_one("alerts.critical", 5000).await?;

if let Some(envelope) = envelope {
    println!("Received: {:?}", envelope.payload);
}
```

### Message Envelopes

All messages are wrapped in a `MessageEnvelope` that provides metadata:

```rust
pub struct MessageEnvelope<T> {
    pub payload: T,
    pub metadata: MessageMetadata,
}

pub struct MessageMetadata {
    pub message_id: String,        // Unique message ID
    pub correlation_id: Option<String>, // For request correlation
    pub timestamp: i64,            // Unix timestamp
    pub source: Option<String>,    // Message source
    pub version: String,           // Protocol version
}
```

Example with metadata:

```rust
let mut envelope = MessageEnvelope::new(alert);
envelope.metadata.correlation_id = Some("req-123".to_string());
envelope.metadata.source = Some("alert-service".to_string());

service.publish("alerts", &envelope).await?;
```

## Event Types

### Incident Events

```rust
pub enum IncidentEvent {
    Created {
        incident_id: String,
        severity: String,
        incident_type: String,
        title: String,
    },
    StateChanged {
        incident_id: String,
        old_state: String,
        new_state: String,
    },
    Assigned {
        incident_id: String,
        assignee: String,
    },
    Resolved {
        incident_id: String,
        resolution_time_secs: u64,
    },
    Escalated {
        incident_id: String,
        escalation_level: u32,
    },
    CommentAdded {
        incident_id: String,
        comment_id: String,
        author: String,
    },
    PlaybookStarted {
        incident_id: String,
        playbook_id: String,
    },
    PlaybookCompleted {
        incident_id: String,
        playbook_id: String,
        success: bool,
    },
    AlertCorrelated {
        incident_id: String,
        alert_id: String,
        correlation_score: f64,
    },
}
```

## Advanced Features

### Topic Prefixing

Topics are automatically prefixed to avoid collisions:

```rust
let config = MessagingConfig {
    topic_prefix: "prod".to_string(),
    ..Default::default()
};

// Publishing to "alerts" becomes "prod.alerts"
service.publish("alerts", &message).await?;
```

### Dead Letter Queue (DLQ)

Configure DLQ for failed message handling:

```rust
let config = MessagingConfig {
    enable_dlq: true,
    dlq_topic: "failed-messages".to_string(),
    ..Default::default()
};
```

### TLS/SSL (NATS)

```rust
let nats_config = NatsConfig {
    enable_tls: true,
    tls_cert_file: Some("/path/to/cert.pem".to_string()),
    tls_key_file: Some("/path/to/key.pem".to_string()),
    tls_ca_file: Some("/path/to/ca.pem".to_string()),
    ..Default::default()
};
```

### SASL Authentication (Kafka)

```rust
let kafka_config = KafkaConfig {
    enable_sasl: true,
    sasl_mechanism: Some("PLAIN".to_string()),
    sasl_username: Some("user".to_string()),
    sasl_password: Some("password".to_string()),
    ..Default::default()
};
```

## Metrics

### Available Metrics

When `enable_metrics: true`:

```
# Messages published
messaging_messages_published_total{topic, backend}

# Messages consumed
messaging_messages_consumed_total{topic, backend}

# Publish failures
messaging_publish_failures_total{topic, backend, error}

# Consume failures
messaging_consume_failures_total{topic, backend, error}

# Active connections
messaging_active_connections{backend}

# Publish latency
messaging_publish_latency_seconds{topic, backend}

# Message size
messaging_message_size_bytes{topic, backend}
```

### Accessing Metrics

```rust
use llm_incident_manager::messaging::MESSAGING_METRICS;

// Metrics are automatically recorded
// Access them for custom dashboards
let published = MESSAGING_METRICS
    .messages_published
    .with_label_values(&["alerts", "nats"])
    .get();
```

## Error Handling

### Error Types

```rust
pub enum MessagingError {
    ConnectionFailed(String),
    PublishFailed(String),
    SubscribeFailed(String),
    ConsumeFailed(String),
    Timeout(String),
    SerializationError(String),
    BackendUnavailable(String),
    ConfigurationError(String),
}
```

### Error Handling Pattern

```rust
match service.publish("topic", &message).await {
    Ok(_) => println!("Published successfully"),
    Err(MessagingError::ConnectionFailed(e)) => {
        eprintln!("Connection failed: {}", e);
        // Retry logic
    }
    Err(MessagingError::PublishFailed(e)) => {
        eprintln!("Publish failed: {}", e);
        // Send to DLQ
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Production Deployment

### NATS Deployment

```yaml
# docker-compose.yml
services:
  nats:
    image: nats:2.10-alpine
    ports:
      - "4222:4222"
      - "8222:8222"  # HTTP monitoring
    command: ["--jetstream", "--max_payload=10MB"]
```

### Kafka Deployment

```yaml
services:
  zookeeper:
    image: confluentinc/cp-zookeeper:7.5.0
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181

  kafka:
    image: confluentinc/cp-kafka:7.5.0
    depends_on:
      - zookeeper
    ports:
      - "9092:9092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:9092
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
```

### Configuration Best Practices

1. **Use TLS in production**: Always enable TLS for secure communication
2. **Enable authentication**: Use SASL for Kafka, credentials for NATS
3. **Configure retries**: Set appropriate retry counts and backoff
4. **Monitor metrics**: Track publish/consume rates and failures
5. **Size limits**: Configure `max_message_size` based on your needs
6. **DLQ setup**: Always enable DLQ in production
7. **Topic naming**: Use consistent, hierarchical topic names

### Performance Tuning

#### NATS

```rust
let nats_config = NatsConfig {
    max_reconnect_attempts: 20,
    reconnect_delay_ms: 2000,
    ping_interval_secs: 30,
    ..Default::default()
};
```

#### Kafka

```rust
let kafka_config = KafkaConfig {
    compression_type: "zstd".to_string(),  // Better compression
    message_timeout_ms: 60000,             // 1 minute timeout
    retries: 10,                           // More retries
    enable_auto_commit: false,             // Manual commit for reliability
    session_timeout_ms: 30000,             // 30 second session timeout
    ..Default::default()
};
```

## Testing

Run the comprehensive test suite:

```bash
cargo test --test messaging_test
```

Test coverage includes:
- 35 unit tests covering all functionality
- Configuration validation
- Message serialization/deserialization
- Event type handling
- Error scenarios
- Multi-backend support

## Troubleshooting

### Connection Issues

**NATS connection failed:**
```
Error: ConnectionFailed("NATS connection failed: connection refused")
```
- Check NATS server is running: `docker ps | grep nats`
- Verify connection string: `nats://localhost:4222`
- Check firewall rules

**Kafka connection failed:**
```
Error: ConnectionFailed("Kafka producer creation failed: broker not available")
```
- Verify Kafka is running: `docker ps | grep kafka`
- Check `bootstrap_servers` configuration
- Ensure Zookeeper is healthy

### Publishing Issues

**Serialization error:**
```
Error: SerializationError("invalid type: ...")
```
- Ensure your type implements `Serialize + Send + Sync`
- Check for unsupported types (e.g., raw pointers)

**Message too large:**
- Increase `max_message_size` in config
- For NATS: Update `--max_payload` server flag
- For Kafka: Update `max.message.bytes` broker config

## Integration Examples

### With REST API

```rust
#[post("/incidents")]
async fn create_incident(
    incident: Json<Incident>,
    messaging: Extension<Arc<MessagingService>>,
) -> Result<Json<Incident>, AppError> {
    // Create incident
    let inc = create_incident_in_db(&incident).await?;

    // Publish event
    messaging.publish_incident_created(
        inc.id.clone(),
        inc.severity.clone(),
        inc.incident_type.clone(),
        inc.title.clone(),
    ).await?;

    Ok(Json(inc))
}
```

### With GraphQL Subscriptions

```rust
#[Subscription]
async fn incident_events(
    &self,
    ctx: &Context<'_>,
) -> impl Stream<Item = IncidentEvent> {
    let messaging = ctx.data::<Arc<MessagingService>>().unwrap();

    let mut stream = messaging
        .subscribe::<MessageEnvelope<IncidentEvent>>("incidents.*")
        .await
        .unwrap();

    stream! {
        while let Ok(Some(envelope)) = stream.next().await {
            yield envelope.payload;
        }
    }
}
```

### With WebSocket Streaming

```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(messaging): Extension<Arc<MessagingService>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, messaging))
}

async fn handle_socket(
    mut socket: WebSocket,
    messaging: Arc<MessagingService>,
) {
    let mut stream = messaging
        .subscribe::<MessageEnvelope<IncidentEvent>>("incidents.*")
        .await
        .unwrap();

    while let Ok(Some(envelope)) = stream.next().await {
        let json = serde_json::to_string(&envelope.payload).unwrap();
        socket.send(Message::Text(json)).await.ok();
    }
}
```

## References

- [NATS Documentation](https://docs.nats.io/)
- [Apache Kafka Documentation](https://kafka.apache.org/documentation/)
- [async-nats Crate](https://docs.rs/async-nats/)
- [rdkafka Crate](https://docs.rs/rdkafka/)
- [Messaging Patterns](https://www.enterpriseintegrationpatterns.com/patterns/messaging/)
