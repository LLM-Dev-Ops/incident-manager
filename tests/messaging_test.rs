use llm_incident_manager::messaging::{
    IncidentEvent, KafkaConfig, MessageEnvelope, MessagingBackend, MessagingConfig,
    MessagingService, NatsConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: String,
    content: String,
    timestamp: i64,
}

/// Test messaging service creation with disabled config
#[tokio::test]
async fn test_messaging_service_disabled() {
    let config = MessagingConfig {
        enabled: false,
        backend: MessagingBackend::Nats,
        nats: NatsConfig::default(),
        kafka: KafkaConfig::default(),
        topic_prefix: "test".to_string(),
        enable_dlq: false,
        dlq_topic: "dlq".to_string(),
        max_message_size: 1024 * 1024,
        enable_metrics: false,
    };

    let service = MessagingService::new(config).await;
    assert!(service.is_ok());

    let service = service.unwrap();
    assert!(!service.is_connected().await);
}

/// Test messaging config defaults
#[test]
fn test_messaging_config_defaults() {
    let config = MessagingConfig::default();
    assert_eq!(config.topic_prefix, "llm-im");
    assert!(config.enabled);
    assert_eq!(config.max_message_size, 1024 * 1024); // 1MB
}

/// Test NATS config defaults
#[test]
fn test_nats_config_defaults() {
    let config = NatsConfig::default();
    assert_eq!(config.servers.len(), 1);
    assert_eq!(config.servers[0], "nats://localhost:4222");
    assert_eq!(config.connection_name, "llm-incident-manager");
    assert_eq!(config.max_reconnect_attempts, 10);
}

/// Test Kafka config defaults
#[test]
fn test_kafka_config_defaults() {
    let config = KafkaConfig::default();
    assert_eq!(config.bootstrap_servers, "localhost:9092");
    assert_eq!(config.client_id, "llm-incident-manager");
    assert_eq!(config.group_id, "llm-incident-manager");
    assert_eq!(config.compression_type, "snappy");
    assert!(config.enable_auto_commit);
}

/// Test topic prefixing
#[test]
fn test_topic_prefix() {
    let config = MessagingConfig {
        topic_prefix: "prod".to_string(),
        ..Default::default()
    };

    assert_eq!(config.full_topic("incidents"), "prod.incidents");
    assert_eq!(config.full_topic("alerts"), "prod.alerts");
}

/// Test message envelope creation
#[test]
fn test_message_envelope() {
    let message = TestMessage {
        id: "test-1".to_string(),
        content: "Hello, World!".to_string(),
        timestamp: 1234567890,
    };

    let envelope = MessageEnvelope::new(message.clone());

    assert_eq!(envelope.payload, message);
    assert!(!envelope.metadata.message_id.is_empty());
    assert!(envelope.metadata.timestamp > 0);
    assert_eq!(envelope.metadata.version, "1.0");
}

/// Test message envelope with metadata
#[test]
fn test_message_envelope_with_metadata() {
    let message = TestMessage {
        id: "test-2".to_string(),
        content: "Test".to_string(),
        timestamp: 1234567890,
    };

    let mut envelope = MessageEnvelope::new(message);
    envelope.metadata.correlation_id = Some("correlation-123".to_string());
    envelope.metadata.source = Some("test-service".to_string());

    assert_eq!(envelope.metadata.correlation_id, Some("correlation-123".to_string()));
    assert_eq!(envelope.metadata.source, Some("test-service".to_string()));
}

/// Test incident event types
#[test]
fn test_incident_event_created() {
    let event = IncidentEvent::Created {
        incident_id: "inc-001".to_string(),
        severity: "critical".to_string(),
        incident_type: "outage".to_string(),
        title: "Database down".to_string(),
    };

    assert_eq!(event.event_type(), "Created");
}

#[test]
fn test_incident_event_state_changed() {
    let event = IncidentEvent::StateChanged {
        incident_id: "inc-001".to_string(),
        old_state: "new".to_string(),
        new_state: "acknowledged".to_string(),
    };

    assert_eq!(event.event_type(), "StateChanged");
}

#[test]
fn test_incident_event_assigned() {
    let event = IncidentEvent::Assigned {
        incident_id: "inc-001".to_string(),
        assignee: "user@example.com".to_string(),
    };

    assert_eq!(event.event_type(), "Assigned");
}

#[test]
fn test_incident_event_resolved() {
    let event = IncidentEvent::Resolved {
        incident_id: "inc-001".to_string(),
        resolution_time_secs: 3600,
    };

    assert_eq!(event.event_type(), "Resolved");
}

#[test]
fn test_incident_event_escalated() {
    let event = IncidentEvent::Escalated {
        incident_id: "inc-001".to_string(),
        escalation_level: 2,
    };

    assert_eq!(event.event_type(), "Escalated");
}

#[test]
fn test_incident_event_comment_added() {
    let event = IncidentEvent::CommentAdded {
        incident_id: "inc-001".to_string(),
        comment_id: "comment-1".to_string(),
        author: "user@example.com".to_string(),
    };

    assert_eq!(event.event_type(), "CommentAdded");
}

#[test]
fn test_incident_event_playbook_started() {
    let event = IncidentEvent::PlaybookStarted {
        incident_id: "inc-001".to_string(),
        playbook_id: "playbook-1".to_string(),
    };

    assert_eq!(event.event_type(), "PlaybookStarted");
}

#[test]
fn test_incident_event_playbook_completed() {
    let event = IncidentEvent::PlaybookCompleted {
        incident_id: "inc-001".to_string(),
        playbook_id: "playbook-1".to_string(),
        success: true,
    };

    assert_eq!(event.event_type(), "PlaybookCompleted");
}

#[test]
fn test_incident_event_alert_correlated() {
    let event = IncidentEvent::AlertCorrelated {
        incident_id: "inc-001".to_string(),
        alert_id: "alert-1".to_string(),
        correlation_score: 0.95,
    };

    assert_eq!(event.event_type(), "AlertCorrelated");
}

/// Test message serialization
#[test]
fn test_message_serialization() {
    let message = TestMessage {
        id: "test-3".to_string(),
        content: "Serialize me".to_string(),
        timestamp: 1234567890,
    };

    let envelope = MessageEnvelope::new(message.clone());
    let json = serde_json::to_string(&envelope).unwrap();

    assert!(json.contains("test-3"));
    assert!(json.contains("Serialize me"));

    let deserialized: MessageEnvelope<TestMessage> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.payload, message);
}

/// Test incident event serialization
#[test]
fn test_incident_event_serialization() {
    let event = IncidentEvent::Created {
        incident_id: "inc-002".to_string(),
        severity: "high".to_string(),
        incident_type: "performance".to_string(),
        title: "Slow response times".to_string(),
    };

    let envelope = MessageEnvelope::new(event);
    let json = serde_json::to_string(&envelope).unwrap();

    assert!(json.contains("inc-002"));
    assert!(json.contains("high"));

    let deserialized: MessageEnvelope<IncidentEvent> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.payload.event_type(), "Created");
}

/// Test publishing with disabled service
#[tokio::test]
async fn test_publish_disabled_service() {
    let config = MessagingConfig {
        enabled: false,
        ..Default::default()
    };

    let service = MessagingService::new(config).await.unwrap();

    let message = TestMessage {
        id: "test-4".to_string(),
        content: "Should not publish".to_string(),
        timestamp: 1234567890,
    };

    // Publishing to disabled service should succeed but do nothing
    let result = service.publish("test-topic", &message).await;
    assert!(result.is_ok());
}

/// Test backend configuration
#[test]
fn test_messaging_backend_nats() {
    let config = MessagingConfig {
        backend: MessagingBackend::Nats,
        ..Default::default()
    };

    assert!(matches!(config.backend, MessagingBackend::Nats));
}

#[test]
fn test_messaging_backend_kafka() {
    let config = MessagingConfig {
        backend: MessagingBackend::Kafka,
        ..Default::default()
    };

    assert!(matches!(config.backend, MessagingBackend::Kafka));
}

#[test]
fn test_messaging_backend_both() {
    let config = MessagingConfig {
        backend: MessagingBackend::Both,
        ..Default::default()
    };

    assert!(matches!(config.backend, MessagingBackend::Both));
}

/// Test DLQ configuration
#[test]
fn test_dlq_configuration() {
    let config = MessagingConfig {
        enable_dlq: true,
        dlq_topic: "my-dlq".to_string(),
        ..Default::default()
    };

    assert!(config.enable_dlq);
    assert_eq!(config.dlq_topic, "my-dlq");
}

/// Test metrics configuration
#[test]
fn test_metrics_configuration() {
    let config = MessagingConfig {
        enable_metrics: true,
        ..Default::default()
    };

    assert!(config.enable_metrics);
}

/// Test NATS TLS configuration
#[test]
fn test_nats_tls_config() {
    let mut config = NatsConfig::default();
    config.enable_tls = true;
    config.tls_cert_file = Some("/path/to/cert.pem".to_string());
    config.tls_key_file = Some("/path/to/key.pem".to_string());

    assert!(config.enable_tls);
    assert_eq!(config.tls_cert_file, Some("/path/to/cert.pem".to_string()));
}

/// Test Kafka SASL configuration
#[test]
fn test_kafka_sasl_config() {
    let mut config = KafkaConfig::default();
    config.enable_sasl = true;
    config.sasl_mechanism = Some("PLAIN".to_string());
    config.sasl_username = Some("user".to_string());
    config.sasl_password = Some("pass".to_string());

    assert!(config.enable_sasl);
    assert_eq!(config.sasl_mechanism, Some("PLAIN".to_string()));
}

/// Test max message size
#[test]
fn test_max_message_size() {
    let config = MessagingConfig {
        max_message_size: 5 * 1024 * 1024, // 5MB
        ..Default::default()
    };

    assert_eq!(config.max_message_size, 5 * 1024 * 1024);
}

/// Test Kafka configuration options
#[test]
fn test_kafka_advanced_config() {
    let mut config = KafkaConfig::default();
    config.retries = 5;
    config.message_timeout_ms = 60000;
    config.session_timeout_ms = 30000;
    config.enable_auto_commit = false;

    assert_eq!(config.retries, 5);
    assert_eq!(config.message_timeout_ms, 60000);
    assert!(!config.enable_auto_commit);
}

/// Test NATS reconnect configuration
#[test]
fn test_nats_reconnect_config() {
    let mut config = NatsConfig::default();
    config.max_reconnect_attempts = 20;
    config.reconnect_delay_ms = 5000;

    assert_eq!(config.max_reconnect_attempts, 20);
    assert_eq!(config.reconnect_delay_ms, 5000);
}
