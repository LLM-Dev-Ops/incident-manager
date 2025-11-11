use llm_incident_manager::{
    models::{Incident, IncidentState, IncidentType, Severity},
    state::{IncidentFilter, IncidentStore, InMemoryStore, RedisStore, SledStore},
};
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create test incident
fn create_test_incident(title: &str, severity: Severity) -> Incident {
    Incident::new(
        "test-source".to_string(),
        title.to_string(),
        "Test description".to_string(),
        severity,
        IncidentType::Infrastructure,
    )
}

/// Test suite that runs against any IncidentStore implementation
async fn test_store_operations<S: IncidentStore + Send + Sync + 'static>(store: Arc<S>) {
    // Test 1: Save and retrieve incident
    let incident = create_test_incident("Test Incident", Severity::P1);
    let id = incident.id;

    store.save_incident(&incident).await.unwrap();

    let retrieved = store.get_incident(&id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.as_ref().unwrap().id, id);
    assert_eq!(retrieved.as_ref().unwrap().title, "Test Incident");

    // Test 2: Update incident
    let mut incident = retrieved.unwrap();
    incident.update_state(
        IncidentState::Investigating,
        "user@example.com".to_string(),
    );
    store.update_incident(&incident).await.unwrap();

    let updated = store.get_incident(&id).await.unwrap().unwrap();
    assert_eq!(updated.state, IncidentState::Investigating);

    // Test 3: List incidents
    let filter = IncidentFilter::default();
    let incidents = store.list_incidents(&filter, 0, 10).await.unwrap();
    assert!(!incidents.is_empty());

    // Test 4: Count incidents
    let count = store.count_incidents(&filter).await.unwrap();
    assert!(count > 0);

    // Test 5: Delete incident
    store.delete_incident(&id).await.unwrap();

    let deleted = store.get_incident(&id).await.unwrap();
    assert!(deleted.is_none());
}

async fn test_filtering<S: IncidentStore + Send + Sync + 'static>(store: Arc<S>) {
    // Create incidents with different severities
    let mut ids = Vec::new();

    for i in 0..10 {
        let severity = match i % 4 {
            0 => Severity::P0,
            1 => Severity::P1,
            2 => Severity::P2,
            _ => Severity::P3,
        };

        let incident = create_test_incident(&format!("Incident {}", i), severity);
        ids.push(incident.id);
        store.save_incident(&incident).await.unwrap();
    }

    // Test severity filter
    let filter = IncidentFilter {
        severities: vec![Severity::P0],
        ..Default::default()
    };

    let incidents = store.list_incidents(&filter, 0, 100).await.unwrap();
    assert_eq!(incidents.len(), 3); // Incidents 0, 4, 8
    assert!(incidents.iter().all(|i| i.severity == Severity::P0));

    // Test multiple severity filter
    let filter = IncidentFilter {
        severities: vec![Severity::P0, Severity::P1],
        ..Default::default()
    };

    let incidents = store.list_incidents(&filter, 0, 100).await.unwrap();
    assert_eq!(incidents.len(), 5); // P0: 0,4,8 + P1: 1,5,9

    // Test state filter
    let mut investigating_incident = incidents[0].clone();
    investigating_incident.update_state(
        IncidentState::Investigating,
        "user@example.com".to_string(),
    );
    store
        .update_incident(&investigating_incident)
        .await
        .unwrap();

    let filter = IncidentFilter {
        states: vec![IncidentState::Investigating],
        ..Default::default()
    };

    let incidents = store.list_incidents(&filter, 0, 100).await.unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0].state, IncidentState::Investigating);

    // Cleanup
    for id in ids {
        store.delete_incident(&id).await.ok();
    }
}

async fn test_pagination<S: IncidentStore + Send + Sync + 'static>(store: Arc<S>) {
    let mut ids = Vec::new();

    // Create 25 incidents
    for i in 0..25 {
        let incident = create_test_incident(&format!("Pagination Test {}", i), Severity::P1);
        ids.push(incident.id);
        store.save_incident(&incident).await.unwrap();
    }

    let filter = IncidentFilter::default();

    // Get first page (10 incidents)
    let page1 = store.list_incidents(&filter, 0, 10).await.unwrap();
    assert_eq!(page1.len(), 10);

    // Get second page
    let page2 = store.list_incidents(&filter, 1, 10).await.unwrap();
    assert_eq!(page2.len(), 10);

    // Get third page
    let page3 = store.list_incidents(&filter, 2, 10).await.unwrap();
    assert_eq!(page3.len(), 5);

    // Verify no overlap
    let page1_ids: Vec<_> = page1.iter().map(|i| i.id).collect();
    let page2_ids: Vec<_> = page2.iter().map(|i| i.id).collect();

    for id in page2_ids {
        assert!(!page1_ids.contains(&id));
    }

    // Cleanup
    for id in ids {
        store.delete_incident(&id).await.ok();
    }
}

async fn test_fingerprint_indexing<S: IncidentStore + Send + Sync + 'static>(store: Arc<S>) {
    let mut ids = Vec::new();

    // Create incidents with same fingerprint
    for i in 0..3 {
        let mut incident =
            create_test_incident(&format!("Fingerprint Test {}", i), Severity::P2);
        incident.fingerprint = Some("test-fingerprint-123".to_string());
        ids.push(incident.id);
        store.save_incident(&incident).await.unwrap();
    }

    // Create incident with different fingerprint
    let mut different = create_test_incident("Different", Severity::P2);
    different.fingerprint = Some("different-fingerprint".to_string());
    ids.push(different.id);
    store.save_incident(&different).await.unwrap();

    // Find by fingerprint
    let found = store
        .find_by_fingerprint("test-fingerprint-123")
        .await
        .unwrap();
    assert_eq!(found.len(), 3);

    let found = store
        .find_by_fingerprint("different-fingerprint")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);

    let found = store
        .find_by_fingerprint("nonexistent")
        .await
        .unwrap();
    assert_eq!(found.len(), 0);

    // Cleanup
    for id in ids {
        store.delete_incident(&id).await.ok();
    }
}

async fn test_concurrent_operations<S: IncidentStore + Send + Sync + 'static>(store: Arc<S>) {
    let mut handles = Vec::new();
    let mut ids = Vec::new();

    // Spawn 10 concurrent tasks that each create incidents
    for i in 0..10 {
        let store = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let incident = create_test_incident(&format!("Concurrent Test {}", i), Severity::P1);
            let id = incident.id;
            store.save_incident(&incident).await.unwrap();
            id
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let id = handle.await.unwrap();
        ids.push(id);
    }

    // Verify all incidents were created
    let filter = IncidentFilter::default();
    let count = store.count_incidents(&filter).await.unwrap();
    assert!(count >= 10);

    // Cleanup
    for id in ids {
        store.delete_incident(&id).await.ok();
    }
}

// InMemoryStore tests
#[tokio::test]
async fn test_inmemory_operations() {
    let store = Arc::new(InMemoryStore::new());
    test_store_operations(store).await;
}

#[tokio::test]
async fn test_inmemory_filtering() {
    let store = Arc::new(InMemoryStore::new());
    test_filtering(store).await;
}

#[tokio::test]
async fn test_inmemory_pagination() {
    let store = Arc::new(InMemoryStore::new());
    test_pagination(store).await;
}

#[tokio::test]
async fn test_inmemory_fingerprint() {
    let store = Arc::new(InMemoryStore::new());
    test_fingerprint_indexing(store).await;
}

#[tokio::test]
async fn test_inmemory_concurrent() {
    let store = Arc::new(InMemoryStore::new());
    test_concurrent_operations(store).await;
}

// SledStore tests
#[tokio::test]
async fn test_sled_operations() {
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(SledStore::new(temp_dir.path()).unwrap());
    test_store_operations(store).await;
}

#[tokio::test]
async fn test_sled_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(SledStore::new(temp_dir.path()).unwrap());
    test_filtering(store).await;
}

#[tokio::test]
async fn test_sled_pagination() {
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(SledStore::new(temp_dir.path()).unwrap());
    test_pagination(store).await;
}

#[tokio::test]
async fn test_sled_fingerprint() {
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(SledStore::new(temp_dir.path()).unwrap());
    test_fingerprint_indexing(store).await;
}

#[tokio::test]
async fn test_sled_concurrent() {
    let temp_dir = TempDir::new().unwrap();
    let store = Arc::new(SledStore::new(temp_dir.path()).unwrap());
    test_concurrent_operations(store).await;
}

#[tokio::test]
async fn test_sled_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    let incident_id = {
        // Create and save incident
        let store = SledStore::new(&path).unwrap();
        let incident = create_test_incident("Persistence Test", Severity::P0);
        let id = incident.id;
        store.save_incident(&incident).await.unwrap();
        store.flush().await.unwrap();
        id
    };

    // Reopen database and verify incident persisted
    {
        let store = SledStore::new(&path).unwrap();
        let retrieved = store.get_incident(&incident_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Persistence Test");
    }
}

// RedisStore tests (conditional on Redis being available)
async fn redis_available() -> bool {
    RedisStore::new("redis://127.0.0.1:6379/15")
        .await
        .is_ok()
}

#[tokio::test]
async fn test_redis_operations() {
    if !redis_available().await {
        eprintln!("Skipping test: Redis not available");
        return;
    }

    let store = Arc::new(
        RedisStore::new_with_prefix("redis://127.0.0.1:6379/15", "test")
            .await
            .unwrap(),
    );
    test_store_operations(store).await;
}

#[tokio::test]
async fn test_redis_filtering() {
    if !redis_available().await {
        eprintln!("Skipping test: Redis not available");
        return;
    }

    let store = Arc::new(
        RedisStore::new_with_prefix("redis://127.0.0.1:6379/15", "test")
            .await
            .unwrap(),
    );
    test_filtering(store).await;
}

#[tokio::test]
async fn test_redis_pagination() {
    if !redis_available().await {
        eprintln!("Skipping test: Redis not available");
        return;
    }

    let store = Arc::new(
        RedisStore::new_with_prefix("redis://127.0.0.1:6379/15", "test")
            .await
            .unwrap(),
    );
    test_pagination(store).await;
}

#[tokio::test]
async fn test_redis_fingerprint() {
    if !redis_available().await {
        eprintln!("Skipping test: Redis not available");
        return;
    }

    let store = Arc::new(
        RedisStore::new_with_prefix("redis://127.0.0.1:6379/15", "test")
            .await
            .unwrap(),
    );
    test_fingerprint_indexing(store).await;
}

#[tokio::test]
async fn test_redis_concurrent() {
    if !redis_available().await {
        eprintln!("Skipping test: Redis not available");
        return;
    }

    let store = Arc::new(
        RedisStore::new_with_prefix("redis://127.0.0.1:6379/15", "test")
            .await
            .unwrap(),
    );
    test_concurrent_operations(store).await;
}

// Cross-store consistency tests
#[tokio::test]
async fn test_store_parity() {
    // Create same incident in both stores
    let incident = create_test_incident("Parity Test", Severity::P1);
    let id = incident.id;

    let inmemory = Arc::new(InMemoryStore::new());
    let temp_dir = TempDir::new().unwrap();
    let sled = Arc::new(SledStore::new(temp_dir.path()).unwrap());

    // Save to both
    inmemory.save_incident(&incident).await.unwrap();
    sled.save_incident(&incident).await.unwrap();

    // Retrieve from both
    let inmemory_retrieved = inmemory.get_incident(&id).await.unwrap().unwrap();
    let sled_retrieved = sled.get_incident(&id).await.unwrap().unwrap();

    // Verify they match
    assert_eq!(inmemory_retrieved.id, sled_retrieved.id);
    assert_eq!(inmemory_retrieved.title, sled_retrieved.title);
    assert_eq!(inmemory_retrieved.severity, sled_retrieved.severity);
}
