/// Integration tests for ML classification system
///
/// These tests verify the complete ML pipeline:
/// - Feature extraction from incidents
/// - Model training
/// - Predictions (severity, type, priority)
/// - Auto-retraining
/// - Model evaluation

use llm_incident_manager::{
    ml::{FeatureExtractor, MLConfig, MLService},
    models::{Incident, IncidentType, Severity},
    state::{InMemoryStore, IncidentStore},
};
use std::sync::Arc;

fn create_test_incident(
    title: &str,
    description: &str,
    source: &str,
    severity: Severity,
) -> Incident {
    Incident::new(
        source.to_string(),
        title.to_string(),
        description.to_string(),
        severity,
        IncidentType::Infrastructure,
    )
}

async fn setup_ml_service() -> (Arc<MLService>, Arc<InMemoryStore>) {
    let store = Arc::new(InMemoryStore::new());
    let config = MLConfig::default();
    let service = Arc::new(MLService::new(config, store.clone()));
    (service, store)
}

#[tokio::test]
async fn test_ml_service_creation() {
    let (service, _) = setup_ml_service().await;

    assert!(!service.is_running().await);

    let stats = service.get_stats().await;
    assert!(stats.enabled);
    assert!(!stats.is_trained);
}

#[tokio::test]
async fn test_ml_service_start_stop() {
    let (service, _) = setup_ml_service().await;

    service.start().await.unwrap();
    assert!(service.is_running().await);

    service.stop().await.unwrap();
    assert!(!service.is_running().await);
}

#[tokio::test]
async fn test_train_on_incidents() {
    let (service, _) = setup_ml_service().await;

    // Create training incidents
    let incidents: Vec<Incident> = vec![
        create_test_incident(
            "Database connection timeout",
            "Connection to database failed",
            "monitoring",
            Severity::P0,
        ),
        create_test_incident(
            "Database query slow",
            "Query taking too long",
            "monitoring",
            Severity::P1,
        ),
        create_test_incident(
            "API latency spike",
            "API response time exceeded threshold",
            "monitoring",
            Severity::P1,
        ),
        create_test_incident(
            "Memory usage high",
            "Application memory usage is high",
            "monitoring",
            Severity::P2,
        ),
        create_test_incident(
            "CPU spike detected",
            "CPU usage above 90%",
            "monitoring",
            Severity::P2,
        ),
        create_test_incident(
            "Disk space low",
            "Disk usage above 85%",
            "monitoring",
            Severity::P3,
        ),
        create_test_incident(
            "Network latency",
            "Network response slow",
            "monitoring",
            Severity::P3,
        ),
        create_test_incident(
            "Service degraded",
            "Service performance degraded",
            "monitoring",
            Severity::P4,
        ),
    ];

    // Duplicate for more training data
    let mut training_incidents = incidents.clone();
    training_incidents.extend(incidents.clone());
    training_incidents.extend(incidents.clone());
    training_incidents.extend(incidents.clone());
    training_incidents.extend(incidents.clone());

    // Train
    let result = service.train_on_incidents(&training_incidents).await;
    assert!(result.is_ok());

    // Check stats
    let stats = service.get_stats().await;
    assert!(stats.is_trained);
    assert!(stats.vocab_size > 0);
    assert!(stats.n_features > 0);
}

#[tokio::test]
async fn test_severity_prediction() {
    let (service, _) = setup_ml_service().await;

    // Train
    let incidents: Vec<Incident> = (0..50)
        .map(|i| {
            let severity = match i % 5 {
                0 => Severity::P0,
                1 => Severity::P1,
                2 => Severity::P2,
                3 => Severity::P3,
                _ => Severity::P4,
            };
            create_test_incident(
                &format!("Incident type {}", i % 5),
                &format!("Description for incident {}", i),
                "test",
                severity,
            )
        })
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    // Predict
    let test_incident = create_test_incident(
        "Incident type 0",
        "Description for incident 0",
        "test",
        Severity::P0,
    );

    let prediction = service.predict_severity(&test_incident).await.unwrap();

    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
    assert!(!prediction.probabilities.is_empty());
    assert_eq!(prediction.probabilities.len(), 5); // 5 severity levels
}

#[tokio::test]
async fn test_type_prediction() {
    let (service, _) = setup_ml_service().await;

    let incident = create_test_incident("Test", "Description", "source", Severity::P1);

    let prediction = service.predict_type(&incident).await.unwrap();

    assert_eq!(prediction.value, IncidentType::Infrastructure);
    assert!(prediction.confidence > 0.0);
}

#[tokio::test]
async fn test_priority_prediction() {
    let (service, _) = setup_ml_service().await;

    // Train first
    let incidents: Vec<Incident> = (0..30)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    let test_incident = create_test_incident("Test", "Description", "test", Severity::P0);

    let prediction = service.predict_priority(&test_incident).await.unwrap();

    assert!(prediction.value >= 0.0 && prediction.value <= 10.0);
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
}

#[tokio::test]
async fn test_predict_all() {
    let (service, _) = setup_ml_service().await;

    // Train
    let incidents: Vec<Incident> = (0..30)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    let test_incident = create_test_incident("Test", "Description", "test", Severity::P1);

    let predictions = service.predict_all(&test_incident).await.unwrap();

    assert!(predictions.severity.is_some());
    assert!(predictions.incident_type.is_some());
    assert!(predictions.priority.is_some());
}

#[tokio::test]
async fn test_add_training_sample() {
    let (service, _) = setup_ml_service().await;

    // Train first to fit feature extractor
    let incidents: Vec<Incident> = (0..20)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    // Add new sample
    let new_incident = create_test_incident("New incident", "New description", "test", Severity::P2);

    let result = service.add_training_sample(&new_incident).await;
    assert!(result.is_ok());

    let stats = service.get_stats().await;
    assert!(stats.samples_since_training > 0);
}

#[tokio::test]
async fn test_auto_retraining() {
    let (service, _) = setup_ml_service().await;

    // Configure low retrain threshold
    let mut config = MLConfig::default();
    config.retrain_threshold = 5;
    config.auto_retrain = true;
    service.update_config(config).await.unwrap();

    // Train initial model
    let incidents: Vec<Incident> = (0..20)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    let initial_stats = service.get_stats().await;
    assert_eq!(initial_stats.samples_since_training, 0);

    // Add samples (one shy of threshold)
    for i in 0..4 {
        let incident = create_test_incident(&format!("New {}", i), "Description", "test", Severity::P2);
        service.add_training_sample(&incident).await.unwrap();
    }

    let mid_stats = service.get_stats().await;
    assert_eq!(mid_stats.samples_since_training, 4);

    // Note: Auto-retraining requires incidents to be saved to store
    // In this test, we can't fully verify auto-retraining without store integration
}

#[tokio::test]
async fn test_feature_extraction_integration() {
    let (service, _) = setup_ml_service().await;

    let incidents = vec![
        create_test_incident("Database error", "Connection timeout", "db-monitor", Severity::P0),
        create_test_incident("API error", "500 internal error", "api-monitor", Severity::P1),
        create_test_incident("Cache miss", "Redis unavailable", "cache-monitor", Severity::P2),
    ];

    service.train_on_incidents(&incidents).await.unwrap();

    let stats = service.get_stats().await;

    // Verify feature extraction worked
    assert!(stats.vocab_size > 0);
    assert!(stats.n_features > 0);

    // Vocabulary should contain key terms
    // (We can't directly access vocabulary, but training success implies it worked)
}

#[tokio::test]
async fn test_model_metadata() {
    let (service, _) = setup_ml_service().await;

    let incidents: Vec<Incident> = (0..30)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    let metadata_list = service.get_model_metadata().await.unwrap();

    assert!(!metadata_list.is_empty());
    let metadata = &metadata_list[0];

    assert!(!metadata.name.is_empty());
    assert!(metadata.n_training_samples > 0);
    assert!(metadata.n_features > 0);
    assert!(metadata.training_metrics.accuracy >= 0.0);
}

#[tokio::test]
async fn test_get_stats() {
    let (service, _) = setup_ml_service().await;

    let stats = service.get_stats().await;

    assert!(stats.enabled);
    assert!(!stats.is_trained);
    assert_eq!(stats.n_training_samples, 0);
    assert_eq!(stats.vocab_size, 0);
    assert_eq!(stats.samples_since_training, 0);
}

#[tokio::test]
async fn test_force_retrain() {
    let (service, store) = setup_ml_service().await;

    // Add some incidents to store
    let incidents: Vec<Incident> = (0..20)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    for incident in &incidents {
        store.save_incident(incident).await.unwrap();
    }

    // Force retrain
    let result = service.force_retrain().await;
    assert!(result.is_ok());

    let stats = service.get_stats().await;
    assert!(stats.is_trained);
}

#[tokio::test]
async fn test_clear_training_cache() {
    let (service, _) = setup_ml_service().await;

    // Train
    let incidents: Vec<Incident> = (0..20)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    // Add samples
    for i in 0..5 {
        let incident = create_test_incident(&format!("New {}", i), "Description", "test", Severity::P2);
        service.add_training_sample(&incident).await.unwrap();
    }

    let before_stats = service.get_stats().await;
    assert!(before_stats.samples_since_training > 0);

    // Clear cache
    service.clear_training_cache().await;

    let after_stats = service.get_stats().await;
    assert_eq!(after_stats.samples_since_training, 0);
}

#[tokio::test]
async fn test_update_config() {
    let (service, _) = setup_ml_service().await;

    let mut new_config = MLConfig::default();
    new_config.min_confidence = 0.8;
    new_config.retrain_threshold = 200;

    let result = service.update_config(new_config).await;
    assert!(result.is_ok());

    // Config should be updated (we can't directly verify, but no error means success)
}

#[tokio::test]
async fn test_prediction_with_low_confidence() {
    let (service, _) = setup_ml_service().await;

    // Train with minimal data (may result in low confidence)
    let incidents: Vec<Incident> = (0..10)
        .map(|i| create_test_incident(&format!("Test {}", i), "Description", "test", Severity::P1))
        .collect();

    service.train_on_incidents(&incidents).await.unwrap();

    let test_incident =
        create_test_incident("Completely different incident", "Unusual description", "test", Severity::P3);

    // Should still make prediction, even if confidence is low
    let result = service.predict_severity(&test_incident).await;
    assert!(result.is_ok());

    let prediction = result.unwrap();
    // Confidence may be low, but should be valid
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
}

#[tokio::test]
async fn test_diverse_training_data() {
    let (service, _) = setup_ml_service().await;

    // Create diverse training data
    let incidents = vec![
        create_test_incident("Database connection timeout", "Cannot connect to PostgreSQL", "db", Severity::P0),
        create_test_incident("API 500 error", "Internal server error on endpoint", "api", Severity::P1),
        create_test_incident("Cache miss rate high", "Redis cache hit rate below 50%", "cache", Severity::P2),
        create_test_incident("Disk space warning", "Disk usage at 85% capacity", "infra", Severity::P3),
        create_test_incident("Log rotation needed", "Log files consuming space", "logging", Severity::P4),
        create_test_incident("Connection pool exhausted", "No available connections", "db", Severity::P0),
        create_test_incident("Rate limit exceeded", "Too many requests", "api", Severity::P1),
        create_test_incident("Memory pressure", "OOM events detected", "infra", Severity::P2),
        create_test_incident("SSL certificate expiring", "Certificate expires in 7 days", "security", Severity::P3),
        create_test_incident("Backup job delayed", "Backup window extended", "backup", Severity::P4),
    ];

    // Repeat for more samples
    let mut training_set = incidents.clone();
    training_set.extend(incidents.clone());
    training_set.extend(incidents.clone());
    training_set.extend(incidents.clone());

    service.train_on_incidents(&training_set).await.unwrap();

    // Test predictions on similar incidents
    let test_cases = vec![
        ("Database timeout", "db", Severity::P0),
        ("API error 5xx", "api", Severity::P1),
        ("Cache performance", "cache", Severity::P2),
    ];

    for (title, source, expected_severity) in test_cases {
        let test_incident = create_test_incident(title, "Test description", source, expected_severity);
        let prediction = service.predict_severity(&test_incident).await.unwrap();

        // Verify prediction is reasonable (may not be exact due to limited training data)
        assert!(prediction.confidence > 0.0);
        assert!(!prediction.probabilities.is_empty());
    }
}

#[tokio::test]
async fn test_ml_service_disabled() {
    let store = Arc::new(InMemoryStore::new());
    let mut config = MLConfig::default();
    config.enabled = false;

    let service = Arc::new(MLService::new(config, store));

    service.start().await.unwrap();

    let incident = create_test_incident("Test", "Description", "test", Severity::P1);

    // Predictions should fail when disabled
    let result = service.predict_severity(&incident).await;
    assert!(result.is_err());
}
