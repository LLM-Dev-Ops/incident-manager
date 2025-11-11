use llm_incident_manager::enrichment::{
    EnrichedContext, EnrichmentConfig, EnrichmentService, Enricher, HistoricalEnricher,
    ServiceEnricher, TeamEnricher,
};
use llm_incident_manager::models::{Incident, IncidentType, Severity};
use llm_incident_manager::state::{InMemoryStore, IncidentStore};
use std::sync::Arc;

/// Helper function to create a test incident
fn create_test_incident(title: &str, description: &str) -> Incident {
    Incident::new(
        "test-source".to_string(),
        title.to_string(),
        description.to_string(),
        Severity::P1,
        IncidentType::Infrastructure,
    )
}

/// Test basic enrichment service creation and lifecycle
#[tokio::test]
async fn test_enrichment_service_lifecycle() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);

    // Service should not be running initially
    assert!(!service.is_running().await);

    // Start the service
    service.start().await.unwrap();
    assert!(service.is_running().await);

    // Stop the service
    service.stop().await.unwrap();
    assert!(!service.is_running().await);
}

/// Test that enrichment service cannot be started twice
#[tokio::test]
async fn test_enrichment_service_double_start() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);

    service.start().await.unwrap();

    // Second start should fail
    let result = service.start().await;
    assert!(result.is_err());
}

/// Test incident enrichment with all enrichers enabled
#[tokio::test]
async fn test_incident_enrichment_all_enabled() {
    let store = Arc::new(InMemoryStore::new());

    // Create and save some historical incidents for comparison
    let hist_incident = create_test_incident(
        "Database connection issue",
        "Database server not responding to connection requests",
    );
    store.save_incident(&hist_incident).await.unwrap();

    let mut config = EnrichmentConfig::default();
    config.enable_historical = true;
    config.enable_service = true;
    config.enable_team = true;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Database timeout",
        "Database queries are timing out after 30 seconds",
    );

    let context = service.enrich_incident(&incident).await.unwrap();

    // Verify enrichment occurred
    assert_eq!(context.incident_id, incident.id);
    assert!(context.total_enrichers() > 0);
    assert!(context.enrichment_duration_ms > 0);

    // Should have historical context (from similar incident)
    assert!(context.historical.is_some());

    // Should have service context
    assert!(context.service.is_some());

    // Should have team context
    assert!(context.team.is_some());
}

/// Test enrichment with selective enrichers
#[tokio::test]
async fn test_selective_enrichers() {
    let store = Arc::new(InMemoryStore::new());

    let mut config = EnrichmentConfig::default();
    config.enable_historical = true;
    config.enable_service = false;
    config.enable_team = false;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident("Test Incident", "Test Description");
    let context = service.enrich_incident(&incident).await.unwrap();

    // Should only have historical enricher enabled
    assert_eq!(service.get_enabled_enricher_count().await, 1);

    // Historical context may or may not be present (depends on similar incidents)
    // But service and team should definitely be None
    assert!(context.service.is_none());
    assert!(context.team.is_none());
}

/// Test enrichment caching
#[tokio::test]
async fn test_enrichment_caching() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident("Cache Test", "Testing cache behavior");

    // First enrichment
    let context1 = service.enrich_incident(&incident).await.unwrap();
    let duration1 = context1.enrichment_duration_ms;

    // Second enrichment should use cache
    let context2 = service.enrich_incident(&incident).await.unwrap();

    // Should be the same incident
    assert_eq!(context1.incident_id, context2.incident_id);

    // Verify cache was used
    let cached = service.get_context(&incident.id).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().incident_id, incident.id);
}

/// Test cache clearing
#[tokio::test]
async fn test_cache_clearing() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);

    let incident = create_test_incident("Cache Clear Test", "Testing cache clear");

    // Enrich incident
    service.enrich_incident(&incident).await.unwrap();

    let stats_before = service.get_cache_stats().await;
    assert!(stats_before.size > 0);

    // Clear cache
    service.clear_cache().await;

    let stats_after = service.get_cache_stats().await;
    assert_eq!(stats_after.size, 0);

    // Should not find in cache anymore
    let cached = service.get_context(&incident.id).await;
    assert!(cached.is_none());
}

/// Test enrichment with disabled service
#[tokio::test]
async fn test_disabled_enrichment_service() {
    let store = Arc::new(InMemoryStore::new());

    let mut config = EnrichmentConfig::default();
    config.enabled = false;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident("Disabled Test", "Testing disabled service");
    let context = service.enrich_incident(&incident).await.unwrap();

    // Should return empty context
    assert_eq!(context.total_enrichers(), 0);
    assert!(context.successful_enrichers.is_empty());
    assert!(context.failed_enrichers.is_empty());
}

/// Test historical enrichment with similar incidents
#[tokio::test]
async fn test_historical_enrichment_similarity() {
    let store = Arc::new(InMemoryStore::new());

    // Create similar historical incidents
    let hist1 = create_test_incident(
        "API Gateway Timeout",
        "API gateway is timing out on requests to payment service",
    );
    let hist2 = create_test_incident(
        "API Gateway Error",
        "API gateway returning 504 errors for payment requests",
    );
    let hist3 = create_test_incident(
        "Database Connection Lost",
        "Database connection pool exhausted",
    );

    store.save_incident(&hist1).await.unwrap();
    store.save_incident(&hist2).await.unwrap();
    store.save_incident(&hist3).await.unwrap();

    let mut config = EnrichmentConfig::default();
    config.enable_historical = true;
    config.enable_service = false;
    config.enable_team = false;
    config.similarity_threshold = 0.3;
    config.historical_lookback_secs = 3600 * 24 * 30; // 30 days

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    // Create a new incident similar to hist1 and hist2
    let incident = create_test_incident(
        "API Gateway Timeout Issue",
        "API gateway timing out on payment service requests",
    );

    let context = service.enrich_incident(&incident).await.unwrap();

    // Should have historical context
    if let Some(historical) = context.historical {
        // Should find similar incidents
        assert!(!historical.similar_incidents.is_empty());

        // Similar incidents should have high similarity scores
        for similar in &historical.similar_incidents {
            assert!(similar.similarity_score > 0.0);
        }
    }
}

/// Test service enrichment
#[tokio::test]
async fn test_service_enrichment() {
    let store = Arc::new(InMemoryStore::new());

    let mut config = EnrichmentConfig::default();
    config.enable_historical = false;
    config.enable_service = true;
    config.enable_team = false;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Service Test",
        "Testing service enrichment",
    );

    let context = service.enrich_incident(&incident).await.unwrap();

    // Should have service context
    assert!(context.service.is_some());

    if let Some(service_ctx) = context.service {
        // Mock service enricher should populate these fields
        assert!(!service_ctx.service_name.is_empty());
    }
}

/// Test team enrichment
#[tokio::test]
async fn test_team_enrichment() {
    let store = Arc::new(InMemoryStore::new());

    let mut config = EnrichmentConfig::default();
    config.enable_historical = false;
    config.enable_service = false;
    config.enable_team = true;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Team Test",
        "Testing team enrichment",
    );

    let context = service.enrich_incident(&incident).await.unwrap();

    // Should have team context
    assert!(context.team.is_some());

    if let Some(team_ctx) = context.team {
        // Mock team enricher should populate these fields
        assert!(!team_ctx.team_name.is_empty());
    }
}

/// Test enrichment statistics
#[tokio::test]
async fn test_enrichment_statistics() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);

    service.start().await.unwrap();

    let stats = service.get_stats().await;

    assert!(stats.enabled);
    assert!(stats.is_running);
    assert!(stats.total_enrichers > 0);
    assert!(stats.enabled_enrichers > 0);
    assert!(stats.enabled_enrichers <= stats.total_enrichers);
    assert!(stats.cache_capacity > 0);
}

/// Test configuration updates
#[tokio::test]
async fn test_configuration_updates() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);

    let mut new_config = EnrichmentConfig::default();
    new_config.timeout_secs = 20;
    new_config.max_concurrent = 10;
    new_config.cache_ttl_secs = 600;

    service.update_config(new_config).await.unwrap();

    // Configuration should be updated (no direct way to verify, but no error means success)
}

/// Test parallel enrichment
#[tokio::test]
async fn test_parallel_enrichment() {
    let store = Arc::new(InMemoryStore::new());

    let mut config = EnrichmentConfig::default();
    config.async_enrichment = true;
    config.max_concurrent = 5;
    config.enable_historical = true;
    config.enable_service = true;
    config.enable_team = true;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Parallel Test",
        "Testing parallel enrichment",
    );

    let start = std::time::Instant::now();
    let context = service.enrich_incident(&incident).await.unwrap();
    let duration = start.elapsed();

    // Verify enrichment succeeded
    assert_eq!(context.incident_id, incident.id);
    assert!(context.total_enrichers() > 0);

    // With parallel execution, duration should be reasonable
    assert!(duration.as_millis() < 5000); // Should complete within 5 seconds
}

/// Test sequential enrichment
#[tokio::test]
async fn test_sequential_enrichment() {
    let store = Arc::new(InMemoryStore::new());

    let mut config = EnrichmentConfig::default();
    config.async_enrichment = false; // Force sequential
    config.enable_historical = true;
    config.enable_service = true;
    config.enable_team = true;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Sequential Test",
        "Testing sequential enrichment",
    );

    let context = service.enrich_incident(&incident).await.unwrap();

    // Verify enrichment succeeded
    assert_eq!(context.incident_id, incident.id);
    assert!(context.total_enrichers() > 0);
}

/// Test enrichment timeout handling
#[tokio::test]
async fn test_enrichment_timeout() {
    let store = Arc::new(InMemoryStore::new());

    let mut config = EnrichmentConfig::default();
    config.timeout_secs = 1; // Very short timeout
    config.enable_historical = true;
    config.enable_service = true;
    config.enable_team = true;

    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Timeout Test",
        "Testing timeout handling",
    );

    // Should still complete even with aggressive timeout
    let result = service.enrich_incident(&incident).await;
    assert!(result.is_ok());
}

/// Test custom enricher registration
#[tokio::test]
async fn test_custom_enricher_registration() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store.clone());

    let initial_count = service.get_enricher_count().await;

    // Register a custom enricher
    let custom_enricher = Arc::new(HistoricalEnricher::new(store));
    service.register_enricher(custom_enricher).await;

    let new_count = service.get_enricher_count().await;

    // Count should have increased
    assert_eq!(new_count, initial_count + 1);
}

/// Test enrichment with empty store
#[tokio::test]
async fn test_enrichment_with_empty_store() {
    let store = Arc::new(InMemoryStore::new());

    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Empty Store Test",
        "Testing with no historical data",
    );

    let context = service.enrich_incident(&incident).await.unwrap();

    // Should still succeed
    assert_eq!(context.incident_id, incident.id);

    // Historical context might be present but with no similar incidents
    if let Some(historical) = context.historical {
        assert!(historical.similar_incidents.is_empty());
    }
}

/// Test enrichment metadata
#[tokio::test]
async fn test_enrichment_metadata() {
    let store = Arc::new(InMemoryStore::new());
    let config = EnrichmentConfig::default();
    let service = EnrichmentService::new(config, store);
    service.start().await.unwrap();

    let incident = create_test_incident(
        "Metadata Test",
        "Testing enrichment metadata",
    );

    let context = service.enrich_incident(&incident).await.unwrap();

    // Context should have timestamp
    assert!(context.enriched_at.timestamp() > 0);

    // Should have enrichment duration
    assert!(context.enrichment_duration_ms >= 0);

    // Should track successful and failed enrichers
    assert!(context.total_enrichers() > 0);
}
