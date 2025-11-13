use crate::error::{AppError, Result};
use crate::models::Incident;
use crate::state::{IncidentFilter, IncidentStore};
use async_trait::async_trait;
use sled::Db;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Persistent incident store using Sled embedded database
#[derive(Clone)]
pub struct SledStore {
    db: Arc<Db>,
    incidents_tree: sled::Tree,
    fingerprint_tree: sled::Tree,
}

impl SledStore {
    /// Create a new Sled store at the specified path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref();
        let db = sled::open(&path).map_err(|e| {
            AppError::Internal(format!("Failed to open Sled database: {}", e))
        })?;

        let incidents_tree = db.open_tree("incidents").map_err(|e| {
            AppError::Internal(format!("Failed to open incidents tree: {}", e))
        })?;

        let fingerprint_tree = db.open_tree("fingerprints").map_err(|e| {
            AppError::Internal(format!("Failed to open fingerprints tree: {}", e))
        })?;

        tracing::info!("Initialized Sled store at {:?}", path_str);

        Ok(Self {
            db: Arc::new(db),
            incidents_tree,
            fingerprint_tree,
        })
    }

    /// Serialize incident to bytes
    fn serialize_incident(incident: &Incident) -> Result<Vec<u8>> {
        bincode::serialize(incident).map_err(|e| {
            AppError::Internal(format!("Failed to serialize incident: {}", e))
        })
    }

    /// Deserialize incident from bytes
    fn deserialize_incident(bytes: &[u8]) -> Result<Incident> {
        bincode::deserialize(bytes).map_err(|e| {
            AppError::Internal(format!("Failed to deserialize incident: {}", e))
        })
    }

    /// Get incident key
    fn incident_key(id: &Uuid) -> Vec<u8> {
        id.as_bytes().to_vec()
    }

    /// Get fingerprint key
    fn fingerprint_key(fingerprint: &str) -> Vec<u8> {
        fingerprint.as_bytes().to_vec()
    }

    /// Update fingerprint index
    fn update_fingerprint_index(&self, incident: &Incident) -> Result<()> {
        if let Some(ref fingerprint) = incident.fingerprint {
            let key = Self::fingerprint_key(fingerprint);

            // Get existing incident IDs for this fingerprint
            let mut incident_ids: Vec<Uuid> = if let Some(existing) = self.fingerprint_tree.get(&key).map_err(|e| {
                AppError::Internal(format!("Failed to read fingerprint index: {}", e))
            })? {
                bincode::deserialize(&existing).unwrap_or_default()
            } else {
                Vec::new()
            };

            // Add this incident if not already present
            if !incident_ids.contains(&incident.id) {
                incident_ids.push(incident.id);
            }

            // Serialize and store
            let serialized = bincode::serialize(&incident_ids).map_err(|e| {
                AppError::Internal(format!("Failed to serialize fingerprint index: {}", e))
            })?;

            self.fingerprint_tree.insert(&key, serialized).map_err(|e| {
                AppError::Internal(format!("Failed to update fingerprint index: {}", e))
            })?;
        }

        Ok(())
    }

    /// Remove from fingerprint index
    fn remove_from_fingerprint_index(&self, incident_id: &Uuid, fingerprint: &str) -> Result<()> {
        let key = Self::fingerprint_key(fingerprint);

        if let Some(existing) = self.fingerprint_tree.get(&key).map_err(|e| {
            AppError::Internal(format!("Failed to read fingerprint index: {}", e))
        })? {
            let mut incident_ids: Vec<Uuid> = bincode::deserialize(&existing).unwrap_or_default();
            incident_ids.retain(|id| id != incident_id);

            if incident_ids.is_empty() {
                self.fingerprint_tree.remove(&key).map_err(|e| {
                    AppError::Internal(format!("Failed to remove fingerprint index: {}", e))
                })?;
            } else {
                let serialized = bincode::serialize(&incident_ids).map_err(|e| {
                    AppError::Internal(format!("Failed to serialize fingerprint index: {}", e))
                })?;

                self.fingerprint_tree.insert(&key, serialized).map_err(|e| {
                    AppError::Internal(format!("Failed to update fingerprint index: {}", e))
                })?;
            }
        }

        Ok(())
    }

    /// Flush pending writes to disk
    pub async fn flush(&self) -> Result<()> {
        self.db.flush_async().await.map_err(|e| {
            AppError::Internal(format!("Failed to flush database: {}", e))
        })?;
        Ok(())
    }

    /// Get database size in bytes
    pub fn size_on_disk(&self) -> Result<u64> {
        self.db.size_on_disk().map_err(|e| {
            AppError::Internal(format!("Failed to get database size: {}", e))
        })
    }
}

#[async_trait]
impl IncidentStore for SledStore {
    async fn save_incident(&self, incident: &Incident) -> Result<()> {
        let key = Self::incident_key(&incident.id);
        let value = Self::serialize_incident(incident)?;

        self.incidents_tree.insert(&key, value).map_err(|e| {
            AppError::Internal(format!("Failed to save incident: {}", e))
        })?;

        // Update fingerprint index
        self.update_fingerprint_index(incident)?;

        // Flush to ensure durability
        self.incidents_tree.flush().map_err(|e| {
            AppError::Internal(format!("Failed to flush incidents tree: {}", e))
        })?;

        tracing::debug!(incident_id = %incident.id, "Incident saved to Sled");
        Ok(())
    }

    async fn get_incident(&self, id: &Uuid) -> Result<Option<Incident>> {
        let key = Self::incident_key(id);

        match self.incidents_tree.get(&key) {
            Ok(Some(bytes)) => {
                let incident = Self::deserialize_incident(&bytes)?;
                Ok(Some(incident))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to get incident: {}",
                e
            ))),
        }
    }

    async fn update_incident(&self, incident: &Incident) -> Result<()> {
        let key = Self::incident_key(&incident.id);

        // Check if incident exists
        if !self.incidents_tree.contains_key(&key).map_err(|e| {
            AppError::Internal(format!("Failed to check incident existence: {}", e))
        })? {
            return Err(AppError::NotFound(format!(
                "Incident {} not found",
                incident.id
            )));
        }

        let value = Self::serialize_incident(incident)?;

        self.incidents_tree.insert(&key, value).map_err(|e| {
            AppError::Internal(format!("Failed to update incident: {}", e))
        })?;

        // Update fingerprint index
        self.update_fingerprint_index(incident)?;

        // Flush to ensure durability
        self.incidents_tree.flush().map_err(|e| {
            AppError::Internal(format!("Failed to flush incidents tree: {}", e))
        })?;

        tracing::debug!(incident_id = %incident.id, "Incident updated in Sled");
        Ok(())
    }

    async fn delete_incident(&self, id: &Uuid) -> Result<()> {
        let key = Self::incident_key(id);

        // Get incident first to access fingerprint
        let incident = match self.get_incident(id).await? {
            Some(i) => i,
            None => {
                return Err(AppError::NotFound(format!("Incident {} not found", id)));
            }
        };

        // Remove from incidents tree
        self.incidents_tree.remove(&key).map_err(|e| {
            AppError::Internal(format!("Failed to delete incident: {}", e))
        })?;

        // Remove from fingerprint index
        if let Some(ref fingerprint) = incident.fingerprint {
            self.remove_from_fingerprint_index(id, fingerprint)?;
        }

        // Flush to ensure durability
        self.incidents_tree.flush().map_err(|e| {
            AppError::Internal(format!("Failed to flush incidents tree: {}", e))
        })?;

        tracing::debug!(incident_id = %id, "Incident deleted from Sled");
        Ok(())
    }

    async fn list_incidents(
        &self,
        filter: &IncidentFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Incident>> {
        let mut incidents: Vec<Incident> = Vec::new();

        // Iterate over all incidents
        for result in self.incidents_tree.iter() {
            let (_, value) = result.map_err(|e| {
                AppError::Internal(format!("Failed to iterate incidents: {}", e))
            })?;

            let incident = Self::deserialize_incident(&value)?;

            // Apply filters
            let state_match =
                filter.states.is_empty() || filter.states.contains(&incident.state);

            let severity_match = filter.severities.is_empty()
                || filter.severities.contains(&incident.severity);

            let source_match = filter.sources.is_empty()
                || filter
                    .sources
                    .iter()
                    .any(|s| incident.source.contains(s));

            let active_match = !filter.active_only || incident.is_active();

            if state_match && severity_match && source_match && active_match {
                incidents.push(incident);
            }
        }

        // Sort by creation time (newest first)
        incidents.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        let start = (page * page_size) as usize;

        Ok(incidents
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect())
    }

    async fn count_incidents(&self, filter: &IncidentFilter) -> Result<u64> {
        let mut count = 0u64;

        // Iterate over all incidents
        for result in self.incidents_tree.iter() {
            let (_, value) = result.map_err(|e| {
                AppError::Internal(format!("Failed to iterate incidents: {}", e))
            })?;

            let incident = Self::deserialize_incident(&value)?;

            // Apply filters
            let state_match =
                filter.states.is_empty() || filter.states.contains(&incident.state);

            let severity_match = filter.severities.is_empty()
                || filter.severities.contains(&incident.severity);

            let source_match = filter.sources.is_empty()
                || filter
                    .sources
                    .iter()
                    .any(|s| incident.source.contains(s));

            let active_match = !filter.active_only || incident.is_active();

            if state_match && severity_match && source_match && active_match {
                count += 1;
            }
        }

        Ok(count)
    }

    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Vec<Incident>> {
        let key = Self::fingerprint_key(fingerprint);

        match self.fingerprint_tree.get(&key) {
            Ok(Some(bytes)) => {
                let incident_ids: Vec<Uuid> = bincode::deserialize(&bytes).unwrap_or_default();

                let mut incidents = Vec::new();
                for id in incident_ids {
                    if let Some(incident) = self.get_incident(&id).await? {
                        incidents.push(incident);
                    }
                }

                Ok(incidents)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to query fingerprint index: {}",
                e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};
    use tempfile::TempDir;

    fn create_test_store() -> (SledStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = SledStore::new(temp_dir.path()).unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_save_and_get_incident() {
        let (store, _temp_dir) = create_test_store();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Application,
        );

        let id = incident.id;
        store.save_incident(&incident).await.unwrap();

        let retrieved = store.get_incident(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_update_incident() {
        let (store, _temp_dir) = create_test_store();

        let mut incident = Incident::new(
            "test-source".to_string(),
            "Test".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Infrastructure,
        );

        store.save_incident(&incident).await.unwrap();

        incident.update_state(IncidentState::Investigating, "user@example.com".to_string());
        store.update_incident(&incident).await.unwrap();

        let retrieved = store.get_incident(&incident.id).await.unwrap().unwrap();
        assert_eq!(retrieved.state, IncidentState::Investigating);
    }

    #[tokio::test]
    async fn test_delete_incident() {
        let (store, _temp_dir) = create_test_store();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Infrastructure,
        );

        let id = incident.id;
        store.save_incident(&incident).await.unwrap();

        store.delete_incident(&id).await.unwrap();

        let retrieved = store.get_incident(&id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_incidents_with_filter() {
        let (store, _temp_dir) = create_test_store();

        // Create multiple incidents
        for i in 0..5 {
            let severity = if i % 2 == 0 { Severity::P0 } else { Severity::P2 };
            let incident = Incident::new(
                "test-source".to_string(),
                format!("Incident {}", i),
                "Description".to_string(),
                severity,
                IncidentType::Application,
            );
            store.save_incident(&incident).await.unwrap();
        }

        // Filter for P0 severity only
        let filter = IncidentFilter {
            severities: vec![Severity::P0],
            ..Default::default()
        };

        let incidents = store.list_incidents(&filter, 0, 10).await.unwrap();
        assert_eq!(incidents.len(), 3); // 0, 2, 4
        assert!(incidents.iter().all(|i| i.severity == Severity::P0));
    }

    #[tokio::test]
    async fn test_fingerprint_indexing() {
        let (store, _temp_dir) = create_test_store();

        let mut incident1 = Incident::new(
            "sentinel".to_string(),
            "API Latency".to_string(),
            "High latency".to_string(),
            Severity::P1,
            IncidentType::Performance,
        );

        incident1.fingerprint = Some("test-fingerprint".to_string());
        store.save_incident(&incident1).await.unwrap();

        let found = store
            .find_by_fingerprint("test-fingerprint")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, incident1.id);
    }

    #[tokio::test]
    async fn test_persistence_across_reopens() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create incident in first store instance
        {
            let store = SledStore::new(&path).unwrap();
            let incident = Incident::new(
                "test-source".to_string(),
                "Test Incident".to_string(),
                "Description".to_string(),
                Severity::P1,
                IncidentType::Application,
            );
            let id = incident.id;
            store.save_incident(&incident).await.unwrap();
            store.flush().await.unwrap();

            // Verify it exists
            let retrieved = store.get_incident(&id).await.unwrap();
            assert!(retrieved.is_some());
        }

        // Reopen database and verify incident still exists
        {
            let store = SledStore::new(&path).unwrap();
            let filter = IncidentFilter::default();
            let incidents = store.list_incidents(&filter, 0, 10).await.unwrap();
            assert_eq!(incidents.len(), 1);
            assert_eq!(incidents[0].title, "Test Incident");
        }
    }

    #[tokio::test]
    async fn test_count_incidents() {
        let (store, _temp_dir) = create_test_store();

        // Create incidents
        for i in 0..10 {
            let severity = if i < 3 { Severity::P0 } else { Severity::P2 };
            let incident = Incident::new(
                "test-source".to_string(),
                format!("Incident {}", i),
                "Description".to_string(),
                severity,
                IncidentType::Application,
            );
            store.save_incident(&incident).await.unwrap();
        }

        // Count all
        let filter = IncidentFilter::default();
        let count = store.count_incidents(&filter).await.unwrap();
        assert_eq!(count, 10);

        // Count P0 only
        let filter = IncidentFilter {
            severities: vec![Severity::P0],
            ..Default::default()
        };
        let count = store.count_incidents(&filter).await.unwrap();
        assert_eq!(count, 3);
    }
}
