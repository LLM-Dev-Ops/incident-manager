use llm_incident_manager::{
    config::NotificationConfig,
    models::{Incident, IncidentType, NotificationChannel, Severity},
    notifications::NotificationService,
    state::InMemoryStore,
};
use std::sync::Arc;

/// Test notification service initialization with no providers
#[tokio::test]
async fn test_notification_service_init_no_providers() {
    let config = NotificationConfig {
        slack_enabled: false,
        slack_webhook_env: None,
        slack_bot_token_env: None,
        slack_default_channel: None,
        email_enabled: false,
        smtp_server: None,
        smtp_port: 587,
        smtp_use_tls: true,
        smtp_username_env: None,
        smtp_password_env: None,
        email_from: None,
        email_from_name: None,
        pagerduty_enabled: false,
        pagerduty_api_token_env: None,
        pagerduty_integration_key_env: None,
        pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
        webhook_enabled: true,
        default_webhook_url: None,
        webhook_timeout_secs: 10,
        max_retries: 3,
        retry_backoff_secs: 1, // Shorter for tests
        queue_size: 100,
        worker_threads: 2,
    };

    let store = Arc::new(InMemoryStore::new());
    let service = NotificationService::new(config, store);
    assert!(service.is_ok());
}

/// Test notification service statistics
#[tokio::test]
async fn test_notification_service_stats() {
    let config = NotificationConfig {
        slack_enabled: false,
        slack_webhook_env: None,
        slack_bot_token_env: None,
        slack_default_channel: Some("#test".to_string()),
        email_enabled: false,
        smtp_server: None,
        smtp_port: 587,
        smtp_use_tls: true,
        smtp_username_env: None,
        smtp_password_env: None,
        email_from: None,
        email_from_name: None,
        pagerduty_enabled: false,
        pagerduty_api_token_env: None,
        pagerduty_integration_key_env: None,
        pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
        webhook_enabled: true,
        default_webhook_url: None,
        webhook_timeout_secs: 10,
        max_retries: 3,
        retry_backoff_secs: 1,
        queue_size: 1000,
        worker_threads: 4,
    };

    let store = Arc::new(InMemoryStore::new());
    let service = NotificationService::new(config, store).unwrap();

    let stats = service.get_stats();
    assert!(!stats.slack_enabled);
    assert!(!stats.email_enabled);
    assert!(!stats.pagerduty_enabled);
    assert!(stats.webhook_enabled);
    assert_eq!(stats.queue_capacity, 1000);
    assert_eq!(stats.worker_count, 4);
}

/// Test queueing a webhook notification
#[tokio::test]
async fn test_queue_webhook_notification() {
    let config = NotificationConfig {
        slack_enabled: false,
        slack_webhook_env: None,
        slack_bot_token_env: None,
        slack_default_channel: None,
        email_enabled: false,
        smtp_server: None,
        smtp_port: 587,
        smtp_use_tls: true,
        smtp_username_env: None,
        smtp_password_env: None,
        email_from: None,
        email_from_name: None,
        pagerduty_enabled: false,
        pagerduty_api_token_env: None,
        pagerduty_integration_key_env: None,
        pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
        webhook_enabled: true,
        default_webhook_url: None,
        webhook_timeout_secs: 10,
        max_retries: 3,
        retry_backoff_secs: 1,
        queue_size: 100,
        worker_threads: 2,
    };

    let store = Arc::new(InMemoryStore::new());
    let service = NotificationService::new(config, store.clone()).unwrap();

    let incident = Incident::new(
        "test-source".to_string(),
        "Test Incident".to_string(),
        "Test description".to_string(),
        Severity::P1,
        IncidentType::Infrastructure,
    );

    // Save incident to store
    store.save_incident(&incident).await.unwrap();

    // Queue notification
    let channels = vec![NotificationChannel::Webhook {
        url: "https://httpbin.org/post".to_string(), // Test endpoint
        payload: serde_json::json!({
            "test": "notification"
        }),
    }];

    let result = service
        .notify_incident(&incident, channels, "Test notification")
        .await;

    assert!(result.is_ok());
    let notification_ids = result.unwrap();
    assert_eq!(notification_ids.len(), 1);
}

/// Test notify_incident_detected helper
#[tokio::test]
async fn test_notify_incident_detected() {
    let config = NotificationConfig {
        slack_enabled: false, // Disabled to avoid external calls
        slack_webhook_env: None,
        slack_bot_token_env: None,
        slack_default_channel: Some("#incidents".to_string()),
        email_enabled: false,
        smtp_server: None,
        smtp_port: 587,
        smtp_use_tls: true,
        smtp_username_env: None,
        smtp_password_env: None,
        email_from: None,
        email_from_name: None,
        pagerduty_enabled: false,
        pagerduty_api_token_env: None,
        pagerduty_integration_key_env: None,
        pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
        webhook_enabled: true,
        default_webhook_url: None,
        webhook_timeout_secs: 10,
        max_retries: 1,
        retry_backoff_secs: 1,
        queue_size: 100,
        worker_threads: 1,
    };

    let store = Arc::new(InMemoryStore::new());
    let service = NotificationService::new(config, store.clone()).unwrap();

    let incident = Incident::new(
        "test-source".to_string(),
        "P0 Incident".to_string(),
        "Critical incident".to_string(),
        Severity::P0,
        IncidentType::Infrastructure,
    );

    store.save_incident(&incident).await.unwrap();

    let result = service.notify_incident_detected(&incident).await;
    assert!(result.is_ok());

    // Even though providers are disabled, the function should succeed
    // (notifications just won't be sent)
}

/// Test notify_incident_resolved helper
#[tokio::test]
async fn test_notify_incident_resolved() {
    let config = NotificationConfig {
        slack_enabled: false,
        slack_webhook_env: None,
        slack_bot_token_env: None,
        slack_default_channel: Some("#incidents".to_string()),
        email_enabled: false,
        smtp_server: None,
        smtp_port: 587,
        smtp_use_tls: true,
        smtp_username_env: None,
        smtp_password_env: None,
        email_from: None,
        email_from_name: None,
        pagerduty_enabled: false,
        pagerduty_api_token_env: None,
        pagerduty_integration_key_env: None,
        pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
        webhook_enabled: true,
        default_webhook_url: None,
        webhook_timeout_secs: 10,
        max_retries: 1,
        retry_backoff_secs: 1,
        queue_size: 100,
        worker_threads: 1,
    };

    let store = Arc::new(InMemoryStore::new());
    let service = NotificationService::new(config, store.clone()).unwrap();

    let mut incident = Incident::new(
        "test-source".to_string(),
        "Test Incident".to_string(),
        "Test description".to_string(),
        Severity::P1,
        IncidentType::Application,
    );

    incident.resolve(
        "test@example.com".to_string(),
        llm_incident_manager::models::ResolutionMethod::Manual,
        "Fixed manually".to_string(),
        Some("Root cause identified".to_string()),
    );

    store.save_incident(&incident).await.unwrap();

    let result = service.notify_incident_resolved(&incident).await;
    assert!(result.is_ok());
}

/// Test notification with multiple channels
#[tokio::test]
async fn test_multiple_notification_channels() {
    let config = NotificationConfig {
        slack_enabled: false,
        slack_webhook_env: None,
        slack_bot_token_env: None,
        slack_default_channel: Some("#test".to_string()),
        email_enabled: false,
        smtp_server: None,
        smtp_port: 587,
        smtp_use_tls: true,
        smtp_username_env: None,
        smtp_password_env: None,
        email_from: None,
        email_from_name: None,
        pagerduty_enabled: false,
        pagerduty_api_token_env: None,
        pagerduty_integration_key_env: None,
        pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
        webhook_enabled: true,
        default_webhook_url: None,
        webhook_timeout_secs: 10,
        max_retries: 1,
        retry_backoff_secs: 1,
        queue_size: 100,
        worker_threads: 2,
    };

    let store = Arc::new(InMemoryStore::new());
    let service = NotificationService::new(config, store.clone()).unwrap();

    let incident = Incident::new(
        "multi-test".to_string(),
        "Multi-Channel Test".to_string(),
        "Testing multiple channels".to_string(),
        Severity::P2,
        IncidentType::Performance,
    );

    store.save_incident(&incident).await.unwrap();

    let channels = vec![
        NotificationChannel::Webhook {
            url: "https://httpbin.org/post".to_string(),
            payload: serde_json::json!({"channel": 1}),
        },
        NotificationChannel::Webhook {
            url: "https://httpbin.org/post".to_string(),
            payload: serde_json::json!({"channel": 2}),
        },
    ];

    let result = service
        .notify_incident(&incident, channels, "Multi-channel test")
        .await;

    assert!(result.is_ok());
    let notification_ids = result.unwrap();
    assert_eq!(notification_ids.len(), 2);
}
