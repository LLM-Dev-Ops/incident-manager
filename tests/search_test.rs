//! Comprehensive tests for the search module

use llm_incident_manager::models::{Incident, IncidentState, IncidentType, Severity};
use llm_incident_manager::search::*;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use tempfile::TempDir;

/// Helper to create test search service
async fn create_test_service() -> SearchService {
    let temp_dir = TempDir::new().unwrap();
    let config = SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        realtime_indexing: true,
        ..Default::default()
    };

    SearchService::new(config).await.unwrap()
}

/// Helper to create test incident
fn create_test_incident(
    id: &str,
    title: &str,
    description: &str,
    severity: Severity,
    incident_type: IncidentType,
    state: IncidentState,
) -> Incident {
    Incident {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        severity,
        incident_type,
        state,
        source: "test-source".to_string(),
        assignee: None,
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        resolved_at: None,
        correlation_id: None,
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_search_service_creation() {
    let service = create_test_service().await;
    let stats = service.get_stats().await.unwrap();

    assert_eq!(stats.total_documents, 0);
    assert!(stats.index_size_bytes > 0); // Index directory exists
}

#[tokio::test]
async fn test_index_single_incident() {
    let service = create_test_service().await;

    let incident = create_test_incident(
        "inc-001",
        "Database connection timeout",
        "PostgreSQL connection pool exhausted",
        Severity::P1,
        IncidentType::Infrastructure,
        IncidentState::Open,
    );

    let result = service.index_incident(&incident).await;
    assert!(result.is_ok());

    // Verify it was indexed
    let stats = service.get_stats().await.unwrap();
    assert_eq!(stats.total_documents, 1);
}

#[tokio::test]
async fn test_index_multiple_incidents() {
    let service = create_test_service().await;

    let incidents = vec![
        create_test_incident(
            "inc-001",
            "API timeout",
            "Gateway timeout",
            Severity::P2,
            IncidentType::Application,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "Database error",
            "Connection failed",
            Severity::P0,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-003",
            "Security breach",
            "Unauthorized access",
            Severity::P0,
            IncidentType::Security,
            IncidentState::Investigating,
        ),
    ];

    let indexed = service.index_incidents(&incidents).await.unwrap();
    assert_eq!(indexed, 3);

    let stats = service.get_stats().await.unwrap();
    assert_eq!(stats.total_documents, 3);
}

#[tokio::test]
async fn test_simple_text_search() {
    let service = create_test_service().await;

    let incidents = vec![
        create_test_incident(
            "inc-001",
            "Database connection error",
            "PostgreSQL connection failed",
            Severity::P1,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "API gateway timeout",
            "Request timeout after 30s",
            Severity::P2,
            IncidentType::Application,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Search for "database"
    let query = SearchQuery::new("database");
    let results = service.search(&query).await.unwrap();

    assert_eq!(results.total_hits, 1);
    assert_eq!(results.hits[0].id, "inc-001");
    assert!(results.hits[0].title.contains("Database"));
}

#[tokio::test]
async fn test_search_with_filters() {
    let service = create_test_service().await;

    let incidents = vec![
        create_test_incident(
            "inc-001",
            "Critical database error",
            "DB down",
            Severity::P0,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "Minor API issue",
            "Slow response",
            Severity::P3,
            IncidentType::Application,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-003",
            "High priority alert",
            "Service degraded",
            Severity::P1,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Search with severity filter
    let query = SearchQuery::new("")
        .with_severity(vec!["P0", "P1"])
        .with_limit(10);

    let results = service.search(&query).await.unwrap();

    assert_eq!(results.total_hits, 2);
    assert!(results.hits.iter().all(|h| h.severity == "P0" || h.severity == "P1"));
}

#[tokio::test]
async fn test_faceted_search() {
    let service = create_test_service().await;

    let incidents = vec![
        create_test_incident(
            "inc-001",
            "Error 1",
            "Description 1",
            Severity::P0,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "Error 2",
            "Description 2",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-003",
            "Error 3",
            "Description 3",
            Severity::P0,
            IncidentType::Security,
            IncidentState::Resolved,
        ),
        create_test_incident(
            "inc-004",
            "Error 4",
            "Description 4",
            Severity::P2,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Search with multiple aggregations
    let query = SearchQuery::new("")
        .with_aggregation(Aggregation::BySeverity)
        .with_aggregation(Aggregation::ByIncidentType)
        .with_aggregation(Aggregation::ByState);

    let results = service.search(&query).await.unwrap();

    // Check severity facets
    assert!(results.facets.contains_key("severity"));
    let severity_facets = &results.facets["severity"];
    assert!(severity_facets.iter().any(|f| f.name == "P0" && f.count == 2));
    assert!(severity_facets.iter().any(|f| f.name == "P1" && f.count == 1));

    // Check incident type facets
    assert!(results.facets.contains_key("incident_type"));
    let type_facets = &results.facets["incident_type"];
    assert!(type_facets.iter().any(|f| f.name == "Infrastructure" && f.count == 2));

    // Check state facets
    assert!(results.facets.contains_key("state"));
    let state_facets = &results.facets["state"];
    assert!(state_facets.iter().any(|f| f.name == "Open" && f.count == 3));
    assert!(state_facets.iter().any(|f| f.name == "Resolved" && f.count == 1));
}

#[tokio::test]
async fn test_search_with_pagination() {
    let service = create_test_service().await;

    // Create 15 incidents
    let mut incidents = Vec::new();
    for i in 0..15 {
        incidents.push(create_test_incident(
            &format!("inc-{:03}", i),
            &format!("Incident {}", i),
            "Test description",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        ));
    }

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Get first page
    let query1 = SearchQuery::new("").with_limit(5).with_offset(0);
    let results1 = service.search(&query1).await.unwrap();

    assert_eq!(results1.total_hits, 15);
    assert_eq!(results1.hits.len(), 5);

    // Get second page
    let query2 = SearchQuery::new("").with_limit(5).with_offset(5);
    let results2 = service.search(&query2).await.unwrap();

    assert_eq!(results2.total_hits, 15);
    assert_eq!(results2.hits.len(), 5);

    // Ensure different results
    assert_ne!(results1.hits[0].id, results2.hits[0].id);
}

#[tokio::test]
async fn test_delete_incident() {
    let service = create_test_service().await;

    let incident = create_test_incident(
        "inc-001",
        "Test incident",
        "Description",
        Severity::P1,
        IncidentType::Application,
        IncidentState::Open,
    );

    service.index_incident(&incident).await.unwrap();
    service.commit().await.unwrap();

    // Verify it exists
    let query = SearchQuery::new("test");
    let results = service.search(&query).await.unwrap();
    assert_eq!(results.total_hits, 1);

    // Delete it
    service.delete_incident("inc-001").await.unwrap();
    service.commit().await.unwrap();

    // Verify it's gone
    let results = service.search(&query).await.unwrap();
    assert_eq!(results.total_hits, 0);
}

#[tokio::test]
async fn test_update_incident() {
    let service = create_test_service().await;

    let mut incident = create_test_incident(
        "inc-001",
        "Original title",
        "Original description",
        Severity::P1,
        IncidentType::Application,
        IncidentState::Open,
    );

    service.index_incident(&incident).await.unwrap();
    service.commit().await.unwrap();

    // Update the incident
    incident.title = "Updated title".to_string();
    incident.description = "Updated description".to_string();
    incident.state = IncidentState::Resolved;

    service.update_incident(&incident).await.unwrap();
    service.commit().await.unwrap();

    // Search for updated content
    let query = SearchQuery::new("updated");
    let results = service.search(&query).await.unwrap();

    assert_eq!(results.total_hits, 1);
    assert_eq!(results.hits[0].title, "Updated title");
    assert_eq!(results.hits[0].state, "Resolved");
}

#[tokio::test]
async fn test_complex_query() {
    let service = create_test_service().await;

    let incidents = vec![
        create_test_incident(
            "inc-001",
            "Database connection timeout in production",
            "PostgreSQL pool exhausted",
            Severity::P0,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "API timeout in development",
            "Gateway timeout",
            Severity::P2,
            IncidentType::Application,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Complex query: "timeout" with severity filter
    let query = SearchQuery::new("timeout")
        .with_severity(vec!["P0"])
        .with_incident_type(vec!["Infrastructure"]);

    let results = service.search(&query).await.unwrap();

    assert_eq!(results.total_hits, 1);
    assert_eq!(results.hits[0].id, "inc-001");
}

#[tokio::test]
async fn test_phrase_search() {
    let service = create_test_service().await;

    let incidents = vec![
        create_test_incident(
            "inc-001",
            "Connection timeout error",
            "Database connection timeout",
            Severity::P1,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "Timeout connection issue",
            "Different order",
            Severity::P1,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Phrase search
    let query = SearchQuery::new("\"connection timeout\"");
    let results = service.search(&query).await.unwrap();

    assert!(results.total_hits >= 1);
    assert!(results.hits.iter().any(|h| h.id == "inc-001"));
}

#[tokio::test]
async fn test_search_suggestions() {
    let service = create_test_service().await;

    let incidents = vec![
        create_test_incident(
            "inc-001",
            "Database error",
            "Description",
            Severity::P1,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "Database timeout",
            "Description",
            Severity::P1,
            IncidentType::Infrastructure,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Get suggestions for partial query
    let suggestions = service.suggest("data", 5).await.unwrap();

    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.text.to_lowercase().contains("data")));
}

#[tokio::test]
async fn test_clear_index() {
    let service = create_test_service().await;

    // Index some incidents
    let incidents = vec![
        create_test_incident(
            "inc-001",
            "Test 1",
            "Description",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-002",
            "Test 2",
            "Description",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Verify they exist
    let stats = service.get_stats().await.unwrap();
    assert_eq!(stats.total_documents, 2);

    // Clear the index
    service.clear_index().await.unwrap();

    // Verify it's empty
    let stats = service.get_stats().await.unwrap();
    assert_eq!(stats.total_documents, 0);
}

#[tokio::test]
async fn test_rebuild_index() {
    let service = create_test_service().await;

    // Create initial incidents
    let incidents1 = vec![
        create_test_incident(
            "inc-001",
            "Old incident",
            "Description",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        ),
    ];

    service.index_incidents(&incidents1).await.unwrap();
    service.commit().await.unwrap();

    // Rebuild with new incidents
    let incidents2 = vec![
        create_test_incident(
            "inc-002",
            "New incident",
            "Description",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        ),
        create_test_incident(
            "inc-003",
            "Another new incident",
            "Description",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        ),
    ];

    service.rebuild_index(&incidents2).await.unwrap();

    // Verify only new incidents exist
    let stats = service.get_stats().await.unwrap();
    assert_eq!(stats.total_documents, 2);

    let query = SearchQuery::new("old");
    let results = service.search(&query).await.unwrap();
    assert_eq!(results.total_hits, 0);

    let query = SearchQuery::new("new");
    let results = service.search(&query).await.unwrap();
    assert_eq!(results.total_hits, 2);
}

#[tokio::test]
async fn test_search_config_builder() {
    let config = SearchConfigBuilder::new()
        .index_path(std::path::PathBuf::from("/tmp/test_index"))
        .writer_heap_size(100_000_000)
        .indexing_threads(8)
        .max_results(500)
        .enable_facets(true)
        .build();

    assert_eq!(config.writer_heap_size, 100_000_000);
    assert_eq!(config.indexing_threads, 8);
    assert_eq!(config.max_results, 500);
    assert!(config.enable_facets);
}

#[tokio::test]
async fn test_sort_by_created_at() {
    let service = create_test_service().await;

    let mut incidents = Vec::new();
    for i in 0..3 {
        let mut incident = create_test_incident(
            &format!("inc-{}", i),
            &format!("Incident {}", i),
            "Description",
            Severity::P1,
            IncidentType::Application,
            IncidentState::Open,
        );
        // Set different timestamps
        incident.created_at = Utc::now() - Duration::hours(i as i64);
        incidents.push(incident);
    }

    service.index_incidents(&incidents).await.unwrap();
    service.commit().await.unwrap();

    // Sort by created_at descending (newest first)
    let query = SearchQuery::new("")
        .with_sort(SearchSort::CreatedAt(SortOrder::Descending));

    let results = service.search(&query).await.unwrap();

    assert_eq!(results.hits.len(), 3);
    // First result should be the newest (inc-0)
    assert_eq!(results.hits[0].id, "inc-0");
}
