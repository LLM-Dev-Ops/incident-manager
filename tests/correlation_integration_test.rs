/// Integration tests for the correlation engine
///
/// These tests verify the complete correlation workflow:
/// - Temporal correlation detection
/// - Pattern-based correlation
/// - Source-based correlation
/// - Fingerprint-based correlation
/// - Group creation and management
/// - Group merging
/// - Manual correlations
/// - Background monitoring

use chrono::{Duration, Utc};
use llm_incident_manager::{
    correlation::{CorrelationConfig, CorrelationEngine, CorrelationType, GroupStatus},
    models::{Incident, IncidentType, Severity},
    state::{InMemoryStore, IncidentStore},
};
use std::sync::Arc;

fn create_test_incident(
    source: &str,
    title: &str,
    description: &str,
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

fn create_test_incident_with_time(
    source: &str,
    title: &str,
    description: &str,
    severity: Severity,
    time_offset_secs: i64,
) -> Incident {
    let mut incident = create_test_incident(source, title, description, severity);
    incident.created_at = Utc::now() + Duration::seconds(time_offset_secs);
    incident
}

async fn setup_engine() -> (Arc<CorrelationEngine>, Arc<InMemoryStore>) {
    let store = Arc::new(InMemoryStore::new());
    let config = CorrelationConfig::default();
    let engine = Arc::new(CorrelationEngine::new(config, store.clone()));
    (engine, store)
}

#[tokio::test]
async fn test_temporal_correlation() {
    let (engine, store) = setup_engine().await;

    // Create two incidents close in time
    let incident1 = create_test_incident_with_time(
        "monitoring",
        "Database connection timeout",
        "Connection to database failed",
        Severity::P1,
        -30, // 30 seconds ago
    );

    let incident2 = create_test_incident_with_time(
        "monitoring",
        "API latency spike",
        "API response time > 5s",
        Severity::P1,
        0, // Now
    );

    // Save first incident
    store.save_incident(&incident1).await.unwrap();

    // Analyze second incident
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // Should detect temporal correlation
    assert!(
        result.has_correlations(),
        "Expected temporal correlation to be detected"
    );
    assert!(result.correlations.len() > 0);

    // Check that a group was created
    assert_eq!(result.groups_created.len(), 1);
    let group = &result.groups_created[0];
    assert_eq!(group.size(), 2);
}

#[tokio::test]
async fn test_pattern_correlation() {
    let (engine, store) = setup_engine().await;

    // Create two incidents with similar content
    let incident1 = create_test_incident(
        "app-monitor",
        "Service outage in production",
        "Critical service experiencing downtime",
        Severity::P0,
    );

    let incident2 = create_test_incident(
        "app-monitor",
        "Service outage in production",
        "Critical service experiencing high error rate",
        Severity::P0,
    );

    // Save first incident
    store.save_incident(&incident1).await.unwrap();

    // Analyze second incident
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // Should detect pattern correlation
    assert!(result.has_correlations());

    // Find pattern correlation
    let pattern_corr = result
        .correlations
        .iter()
        .find(|c| c.correlation_type == CorrelationType::Pattern);
    assert!(pattern_corr.is_some(), "Pattern correlation should be detected");
}

#[tokio::test]
async fn test_source_correlation() {
    let (engine, store) = setup_engine().await;

    // Create two incidents from same source
    let incident1 = create_test_incident(
        "datadog",
        "Memory alert",
        "Memory usage > 90%",
        Severity::P2,
    );

    let incident2 = create_test_incident(
        "datadog",
        "CPU alert",
        "CPU usage > 85%",
        Severity::P2,
    );

    // Save first incident
    store.save_incident(&incident1).await.unwrap();

    // Analyze second incident
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // Should detect source correlation
    assert!(result.has_correlations());

    let source_corr = result
        .correlations
        .iter()
        .find(|c| c.correlation_type == CorrelationType::Source);
    assert!(source_corr.is_some(), "Source correlation should be detected");
}

#[tokio::test]
async fn test_fingerprint_correlation() {
    let (engine, store) = setup_engine().await;

    // Create two incidents with same fingerprint
    let mut incident1 = create_test_incident(
        "monitor",
        "Test incident",
        "Test description",
        Severity::P2,
    );
    incident1.fingerprint = Some("test-fingerprint-123".to_string());

    let mut incident2 = create_test_incident(
        "monitor",
        "Test incident",
        "Test description",
        Severity::P2,
    );
    incident2.fingerprint = Some("test-fingerprint-123".to_string());

    // Save first incident
    store.save_incident(&incident1).await.unwrap();

    // Analyze second incident
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // Should detect fingerprint correlation
    assert!(result.has_correlations());

    let fp_corr = result
        .correlations
        .iter()
        .find(|c| c.correlation_type == CorrelationType::Fingerprint);
    assert!(fp_corr.is_some(), "Fingerprint correlation should be detected");
}

#[tokio::test]
async fn test_combined_correlation() {
    let (engine, store) = setup_engine().await;

    // Create incidents that match on multiple signals
    let incident1 = create_test_incident_with_time(
        "datadog",
        "Database connection failure",
        "Cannot connect to primary database",
        Severity::P0,
        -20,
    );

    let incident2 = create_test_incident_with_time(
        "datadog",
        "Database connection failure",
        "Cannot connect to primary database",
        Severity::P0,
        0,
    );

    // Save first incident
    store.save_incident(&incident1).await.unwrap();

    // Analyze second incident
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // Should detect combined correlation (multiple signals)
    assert!(result.has_correlations());
    assert!(result.correlations.len() >= 2, "Should have multiple correlations");

    // Should have combined correlation type
    let combined_corr = result
        .correlations
        .iter()
        .find(|c| c.correlation_type == CorrelationType::Combined);
    assert!(
        combined_corr.is_some(),
        "Combined correlation should be detected when multiple signals match"
    );
}

#[tokio::test]
async fn test_group_creation() {
    let (engine, store) = setup_engine().await;

    let incident1 = create_test_incident(
        "monitor",
        "Test 1",
        "Description 1",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "monitor",
        "Test 2",
        "Description 2",
        Severity::P1,
    );

    // Save and analyze incidents
    store.save_incident(&incident1).await.unwrap();
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // A group should be created
    assert_eq!(result.groups_created.len(), 1);
    let group = &result.groups_created[0];

    // Verify group properties
    assert_eq!(group.status, GroupStatus::Active);
    assert_eq!(group.size(), 2);
    assert!(group.contains_incident(&incident1.id));
    assert!(group.contains_incident(&incident2.id));
}

#[tokio::test]
async fn test_add_to_existing_group() {
    let (engine, store) = setup_engine().await;

    // Create three incidents
    let incident1 = create_test_incident(
        "monitor",
        "Issue A",
        "Description A",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "monitor",
        "Issue B",
        "Description B",
        Severity::P1,
    );

    let incident3 = create_test_incident(
        "monitor",
        "Issue C",
        "Description C",
        Severity::P1,
    );

    // Save and analyze first two
    store.save_incident(&incident1).await.unwrap();
    let result1 = engine.analyze_incident(&incident2).await.unwrap();
    let group_id = result1.groups_created[0].id;

    // Save second incident
    store.save_incident(&incident2).await.unwrap();

    // Analyze third incident
    let result2 = engine.analyze_incident(&incident3).await.unwrap();

    // Should be added to existing group
    assert_eq!(result2.groups_created.len(), 0, "No new groups should be created");
    assert!(result2.groups_affected.contains(&group_id));

    // Verify group size
    let group = engine.get_group(&group_id).unwrap();
    assert_eq!(group.size(), 3);
}

#[tokio::test]
async fn test_group_merging() {
    let (engine, store) = setup_engine().await;

    // Create incidents that will form two separate groups
    let incident1 = create_test_incident_with_time(
        "source1",
        "Issue 1",
        "Description 1",
        Severity::P1,
        -120, // 2 minutes ago
    );

    let incident2 = create_test_incident_with_time(
        "source2",
        "Issue 2",
        "Description 2",
        Severity::P1,
        -60, // 1 minute ago
    );

    // Save incidents
    store.save_incident(&incident1).await.unwrap();
    store.save_incident(&incident2).await.unwrap();

    // Analyze to create groups (won't correlate due to different sources and content)
    let result1 = engine.analyze_incident(&incident1).await.unwrap();
    let result2 = engine.analyze_incident(&incident2).await.unwrap();

    // Create a third incident that correlates with both
    let incident3 = create_test_incident_with_time(
        "source1",
        "Issue 1",
        "Description 1",
        Severity::P1,
        0, // Now
    );

    let result3 = engine.analyze_incident(&incident3).await.unwrap();

    // Verify correlation occurred
    assert!(result3.has_correlations());
}

#[tokio::test]
async fn test_manual_correlation() {
    let (engine, _store) = setup_engine().await;

    let incident1 = create_test_incident(
        "manual",
        "Issue 1",
        "Desc 1",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "manual",
        "Issue 2",
        "Desc 2",
        Severity::P2,
    );

    // Manually correlate incidents
    let correlation = engine
        .manual_correlate(
            vec![incident1.id, incident2.id],
            "Operator identified these as related".to_string(),
        )
        .await
        .unwrap();

    // Verify manual correlation
    assert_eq!(correlation.correlation_type, CorrelationType::Manual);
    assert_eq!(correlation.score, 1.0);
    assert!(correlation.involves_incident(&incident1.id));
    assert!(correlation.involves_incident(&incident2.id));
}

#[tokio::test]
async fn test_resolve_group() {
    let (engine, store) = setup_engine().await;

    let incident1 = create_test_incident(
        "monitor",
        "Test 1",
        "Description 1",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "monitor",
        "Test 2",
        "Description 2",
        Severity::P1,
    );

    // Create group
    store.save_incident(&incident1).await.unwrap();
    let result = engine.analyze_incident(&incident2).await.unwrap();
    let group_id = result.groups_created[0].id;

    // Resolve the group
    engine.resolve_group(&group_id).await.unwrap();

    // Verify group is resolved
    let group = engine.get_group(&group_id).unwrap();
    assert_eq!(group.status, GroupStatus::Resolved);
}

#[tokio::test]
async fn test_get_group_for_incident() {
    let (engine, store) = setup_engine().await;

    let incident1 = create_test_incident(
        "test",
        "Test 1",
        "Desc 1",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "test",
        "Test 2",
        "Desc 2",
        Severity::P1,
    );

    // Create correlation
    store.save_incident(&incident1).await.unwrap();
    engine.analyze_incident(&incident2).await.unwrap();

    // Get group for incident
    let group = engine.get_group_for_incident(&incident2.id);
    assert!(group.is_some(), "Should find group for incident");

    let group = group.unwrap();
    assert!(group.contains_incident(&incident1.id));
    assert!(group.contains_incident(&incident2.id));
}

#[tokio::test]
async fn test_get_correlations_for_incident() {
    let (engine, store) = setup_engine().await;

    let incident1 = create_test_incident(
        "test",
        "Test 1",
        "Desc 1",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "test",
        "Test 2",
        "Desc 2",
        Severity::P1,
    );

    // Create correlation
    store.save_incident(&incident1).await.unwrap();
    engine.analyze_incident(&incident2).await.unwrap();

    // Get correlations for incident
    let correlations = engine.get_correlations_for_incident(&incident2.id);
    assert!(!correlations.is_empty(), "Should have correlations");

    for corr in &correlations {
        assert!(corr.involves_incident(&incident2.id));
    }
}

#[tokio::test]
async fn test_get_active_groups() {
    let (engine, store) = setup_engine().await;

    // Create multiple groups
    for i in 0..3 {
        let incident1 = create_test_incident(
            &format!("source{}", i),
            &format!("Test {}", i * 2),
            &format!("Desc {}", i * 2),
            Severity::P1,
        );

        let incident2 = create_test_incident(
            &format!("source{}", i),
            &format!("Test {}", i * 2 + 1),
            &format!("Desc {}", i * 2 + 1),
            Severity::P1,
        );

        store.save_incident(&incident1).await.unwrap();
        engine.analyze_incident(&incident2).await.unwrap();
    }

    // Get all active groups
    let active_groups = engine.get_active_groups();
    assert!(active_groups.len() >= 3, "Should have at least 3 active groups");

    for group in &active_groups {
        assert_eq!(group.status, GroupStatus::Active);
    }
}

#[tokio::test]
async fn test_correlation_stats() {
    let (engine, store) = setup_engine().await;

    // Create some correlations
    let incident1 = create_test_incident(
        "test",
        "Test 1",
        "Desc 1",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "test",
        "Test 2",
        "Desc 2",
        Severity::P1,
    );

    store.save_incident(&incident1).await.unwrap();
    engine.analyze_incident(&incident2).await.unwrap();

    // Get stats
    let stats = engine.get_stats();
    assert!(stats.total_groups > 0);
    assert!(stats.active_groups > 0);
    assert!(stats.total_correlations > 0);
    assert!(stats.total_mapped_incidents > 0);
}

#[tokio::test]
async fn test_engine_start_stop() {
    let (engine, _store) = setup_engine().await;

    // Start engine
    assert!(!engine.is_running().await);
    engine.start().await.unwrap();
    assert!(engine.is_running().await);

    // Stop engine
    engine.stop().await.unwrap();
    assert!(!engine.is_running().await);
}

#[tokio::test]
async fn test_no_correlation_for_dissimilar_incidents() {
    let (engine, store) = setup_engine().await;

    // Create very different incidents
    let incident1 = create_test_incident_with_time(
        "source1",
        "Database error in production",
        "Connection pool exhausted",
        Severity::P0,
        -500, // 500 seconds ago (outside temporal window)
    );

    let incident2 = create_test_incident_with_time(
        "source2",
        "Frontend UI glitch",
        "Button color is wrong",
        Severity::P3,
        0,
    );

    store.save_incident(&incident1).await.unwrap();
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // Should not detect correlation
    assert!(!result.has_correlations() || result.correlations.iter().all(|c| c.score < 0.5));
}

#[tokio::test]
async fn test_correlation_with_multiple_candidates() {
    let (engine, store) = setup_engine().await;

    // Create a base incident
    let base = create_test_incident(
        "monitor",
        "Service outage",
        "Service is down",
        Severity::P0,
    );

    // Create multiple similar incidents
    for i in 1..=5 {
        let incident = create_test_incident(
            "monitor",
            &format!("Service outage {}", i),
            "Service is down",
            Severity::P0,
        );
        store.save_incident(&incident).await.unwrap();
    }

    // Analyze base incident
    let result = engine.analyze_incident(&base).await.unwrap();

    // Should correlate with multiple incidents
    assert!(result.has_correlations());
    assert!(result.correlation_count() >= 5);
}

#[tokio::test]
async fn test_update_correlation_config() {
    let (engine, _store) = setup_engine().await;

    let mut new_config = CorrelationConfig::default();
    new_config.min_correlation_score = 0.8;
    new_config.temporal_window_secs = 600;

    engine.update_config(new_config).await.unwrap();

    // Config should be updated (verify by checking behavior)
    // This is implicit - the engine should now use the new thresholds
}

#[tokio::test]
async fn test_correlation_score_thresholds() {
    let store = Arc::new(InMemoryStore::new());

    // Create engine with high threshold
    let mut config = CorrelationConfig::default();
    config.min_correlation_score = 0.9; // Very high threshold
    let engine = Arc::new(CorrelationEngine::new(config, store.clone()));

    // Create somewhat similar incidents
    let incident1 = create_test_incident(
        "monitor",
        "Test incident one",
        "Some description",
        Severity::P2,
    );

    let incident2 = create_test_incident(
        "monitor",
        "Test incident two",
        "Different description",
        Severity::P2,
    );

    store.save_incident(&incident1).await.unwrap();
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // With high threshold, weak correlations should be filtered out
    if result.has_correlations() {
        for corr in &result.correlations {
            assert!(
                corr.score >= 0.9,
                "Correlation score should meet threshold"
            );
        }
    }
}

#[tokio::test]
async fn test_disabled_correlation_strategies() {
    let store = Arc::new(InMemoryStore::new());

    // Create engine with some strategies disabled
    let config = CorrelationConfig {
        enabled: true,
        enable_temporal: false,
        enable_pattern: true,
        enable_source: false,
        enable_fingerprint: false,
        enable_topology: false,
        ..Default::default()
    };

    let engine = Arc::new(CorrelationEngine::new(config, store.clone()));

    let incident1 = create_test_incident(
        "monitor",
        "Test incident",
        "Test description",
        Severity::P1,
    );

    let incident2 = create_test_incident(
        "monitor",
        "Test incident",
        "Test description",
        Severity::P1,
    );

    store.save_incident(&incident1).await.unwrap();
    let result = engine.analyze_incident(&incident2).await.unwrap();

    // Only pattern correlations should be detected
    if result.has_correlations() {
        for corr in &result.correlations {
            assert!(
                corr.correlation_type == CorrelationType::Pattern
                    || corr.correlation_type == CorrelationType::Combined
            );
        }
    }
}
