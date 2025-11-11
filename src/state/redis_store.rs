use crate::error::{AppError, Result};
use crate::models::Incident;
use crate::state::{IncidentFilter, IncidentStore};
use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client, RedisError};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Redis-based persistent incident store
#[derive(Clone)]
pub struct RedisStore {
    client: Arc<Client>,
    connection: ConnectionManager,
    key_prefix: String,
}

impl RedisStore {
    /// Create a new Redis store
    pub async fn new(redis_url: &str) -> Result<Self> {
        Self::new_with_prefix(redis_url, "llm-im").await
    }

    /// Create a new Redis store with custom key prefix
    pub async fn new_with_prefix(redis_url: &str, prefix: &str) -> Result<Self> {
        let client = Client::open(redis_url).map_err(|e| {
            AppError::Internal(format!("Failed to create Redis client: {}", e))
        })?;

        let connection = ConnectionManager::new(client.clone())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to Redis: {}", e)))?;

        // Test connection
        let mut test_conn = connection.clone();
        redis::cmd("PING")
            .query_async::<_, String>(&mut test_conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis connection test failed: {}", e)))?;

        tracing::info!("Initialized Redis store with prefix '{}'", prefix);

        Ok(Self {
            client: Arc::new(client),
            connection,
            key_prefix: prefix.to_string(),
        })
    }

    /// Get incident key
    fn incident_key(&self, id: &Uuid) -> String {
        format!("{}:incident:{}", self.key_prefix, id)
    }

    /// Get all incidents set key
    fn incidents_set_key(&self) -> String {
        format!("{}:incidents", self.key_prefix)
    }

    /// Get fingerprint key
    fn fingerprint_key(&self, fingerprint: &str) -> String {
        format!("{}:fingerprint:{}", self.key_prefix, fingerprint)
    }

    /// Get incidents by severity index key
    fn severity_index_key(&self, severity: &str) -> String {
        format!("{}:severity:{}", self.key_prefix, severity)
    }

    /// Get incidents by state index key
    fn state_index_key(&self, state: &str) -> String {
        format!("{}:state:{}", self.key_prefix, state)
    }

    /// Get incidents by source index key
    fn source_index_key(&self, source: &str) -> String {
        format!("{}:source:{}", self.key_prefix, source)
    }

    /// Serialize incident to JSON
    fn serialize_incident(incident: &Incident) -> Result<String> {
        serde_json::to_string(incident).map_err(|e| {
            AppError::Internal(format!("Failed to serialize incident: {}", e))
        })
    }

    /// Deserialize incident from JSON
    fn deserialize_incident(json: &str) -> Result<Incident> {
        serde_json::from_str(json).map_err(|e| {
            AppError::Internal(format!("Failed to deserialize incident: {}", e))
        })
    }

    /// Update indices for an incident
    async fn update_indices(&mut self, incident: &Incident) -> Result<()> {
        let incident_id_str = incident.id.to_string();

        // Add to all incidents set
        let _: () = self
            .connection
            .sadd(self.incidents_set_key(), &incident_id_str)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update incidents set: {}", e)))?;

        // Add to severity index
        let severity_str = format!("{:?}", incident.severity);
        let _: () = self
            .connection
            .sadd(self.severity_index_key(&severity_str), &incident_id_str)
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to update severity index: {}", e))
            })?;

        // Add to state index
        let state_str = format!("{:?}", incident.state);
        let _: () = self
            .connection
            .sadd(self.state_index_key(&state_str), &incident_id_str)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update state index: {}", e)))?;

        // Add to source index
        let _: () = self
            .connection
            .sadd(self.source_index_key(&incident.source), &incident_id_str)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update source index: {}", e)))?;

        // Add to fingerprint index if present
        if let Some(ref fingerprint) = incident.fingerprint {
            let _: () = self
                .connection
                .sadd(self.fingerprint_key(fingerprint), &incident_id_str)
                .await
                .map_err(|e| {
                    AppError::Internal(format!("Failed to update fingerprint index: {}", e))
                })?;
        }

        Ok(())
    }

    /// Remove incident from indices
    async fn remove_from_indices(&mut self, incident: &Incident) -> Result<()> {
        let incident_id_str = incident.id.to_string();

        // Remove from all incidents set
        let _: () = self
            .connection
            .srem(self.incidents_set_key(), &incident_id_str)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update incidents set: {}", e)))?;

        // Remove from severity index
        let severity_str = format!("{:?}", incident.severity);
        let _: () = self
            .connection
            .srem(self.severity_index_key(&severity_str), &incident_id_str)
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to update severity index: {}", e))
            })?;

        // Remove from state index
        let state_str = format!("{:?}", incident.state);
        let _: () = self
            .connection
            .srem(self.state_index_key(&state_str), &incident_id_str)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update state index: {}", e)))?;

        // Remove from source index
        let _: () = self
            .connection
            .srem(self.source_index_key(&incident.source), &incident_id_str)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update source index: {}", e)))?;

        // Remove from fingerprint index if present
        if let Some(ref fingerprint) = incident.fingerprint {
            let _: () = self
                .connection
                .srem(self.fingerprint_key(fingerprint), &incident_id_str)
                .await
                .map_err(|e| {
                    AppError::Internal(format!("Failed to update fingerprint index: {}", e))
                })?;
        }

        Ok(())
    }

    /// Get incident IDs matching filter
    async fn get_filtered_incident_ids(&mut self, filter: &IncidentFilter) -> Result<Vec<String>> {
        let mut sets_to_intersect: Vec<String> = Vec::new();

        // If severity filter is specified
        if !filter.severities.is_empty() {
            let severity_keys: Vec<String> = filter
                .severities
                .iter()
                .map(|s| self.severity_index_key(&format!("{:?}", s)))
                .collect();

            // For multiple severities, we need to union them
            if severity_keys.len() == 1 {
                sets_to_intersect.push(severity_keys[0].clone());
            } else {
                // Create temporary union key
                let temp_key = format!("{}:temp:severity_union:{}", self.key_prefix, Uuid::new_v4());
                let _: () = redis::cmd("SUNIONSTORE")
                    .arg(&temp_key)
                    .arg(&severity_keys)
                    .query_async(&mut self.connection)
                    .await
                    .map_err(|e| {
                        AppError::Internal(format!("Failed to union severity sets: {}", e))
                    })?;

                // Set expiration on temp key
                let _: () = self
                    .connection
                    .expire(&temp_key, 60)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to expire temp key: {}", e)))?;

                sets_to_intersect.push(temp_key);
            }
        }

        // If state filter is specified
        if !filter.states.is_empty() {
            let state_keys: Vec<String> = filter
                .states
                .iter()
                .map(|s| self.state_index_key(&format!("{:?}", s)))
                .collect();

            if state_keys.len() == 1 {
                sets_to_intersect.push(state_keys[0].clone());
            } else {
                let temp_key = format!("{}:temp:state_union:{}", self.key_prefix, Uuid::new_v4());
                let _: () = redis::cmd("SUNIONSTORE")
                    .arg(&temp_key)
                    .arg(&state_keys)
                    .query_async(&mut self.connection)
                    .await
                    .map_err(|e| {
                        AppError::Internal(format!("Failed to union state sets: {}", e))
                    })?;

                let _: () = self
                    .connection
                    .expire(&temp_key, 60)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to expire temp key: {}", e)))?;

                sets_to_intersect.push(temp_key);
            }
        }

        // If source filter is specified
        if !filter.sources.is_empty() {
            let source_keys: Vec<String> = filter
                .sources
                .iter()
                .map(|s| self.source_index_key(s))
                .collect();

            if source_keys.len() == 1 {
                sets_to_intersect.push(source_keys[0].clone());
            } else {
                let temp_key = format!("{}:temp:source_union:{}", self.key_prefix, Uuid::new_v4());
                let _: () = redis::cmd("SUNIONSTORE")
                    .arg(&temp_key)
                    .arg(&source_keys)
                    .query_async(&mut self.connection)
                    .await
                    .map_err(|e| {
                        AppError::Internal(format!("Failed to union source sets: {}", e))
                    })?;

                let _: () = self
                    .connection
                    .expire(&temp_key, 60)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to expire temp key: {}", e)))?;

                sets_to_intersect.push(temp_key);
            }
        }

        // Get incident IDs
        let incident_ids: Vec<String> = if sets_to_intersect.is_empty() {
            // No filters, return all incidents
            self.connection
                .smembers(self.incidents_set_key())
                .await
                .map_err(|e| {
                    AppError::Internal(format!("Failed to get all incidents: {}", e))
                })?
        } else if sets_to_intersect.len() == 1 {
            // Single set
            self.connection
                .smembers(&sets_to_intersect[0])
                .await
                .map_err(|e| AppError::Internal(format!("Failed to get incidents: {}", e)))?
        } else {
            // Multiple sets - intersect them
            redis::cmd("SINTER")
                .arg(&sets_to_intersect)
                .query_async(&mut self.connection)
                .await
                .map_err(|e| {
                    AppError::Internal(format!("Failed to intersect incident sets: {}", e))
                })?
        };

        Ok(incident_ids)
    }
}

#[async_trait]
impl IncidentStore for RedisStore {
    async fn save_incident(&self, incident: &Incident) -> Result<()> {
        let key = self.incident_key(&incident.id);
        let value = Self::serialize_incident(incident)?;

        let mut conn = self.connection.clone();

        // Save incident data
        let _: () = conn
            .set(&key, &value)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to save incident: {}", e)))?;

        // Update indices
        self.clone().update_indices(incident).await?;

        tracing::debug!(incident_id = %incident.id, "Incident saved to Redis");
        Ok(())
    }

    async fn get_incident(&self, id: &Uuid) -> Result<Option<Incident>> {
        let key = self.incident_key(id);

        let mut conn = self.connection.clone();

        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get incident: {}", e)))?;

        match value {
            Some(json) => {
                let incident = Self::deserialize_incident(&json)?;
                Ok(Some(incident))
            }
            None => Ok(None),
        }
    }

    async fn update_incident(&self, incident: &Incident) -> Result<()> {
        let key = self.incident_key(&incident.id);

        let mut conn = self.connection.clone();

        // Check if incident exists
        let exists: bool = conn
            .exists(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to check incident existence: {}", e)))?;

        if !exists {
            return Err(AppError::NotFound(format!(
                "Incident {} not found",
                incident.id
            )));
        }

        let value = Self::serialize_incident(incident)?;

        // Update incident data
        let _: () = conn
            .set(&key, &value)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update incident: {}", e)))?;

        // Update indices
        self.clone().update_indices(incident).await?;

        tracing::debug!(incident_id = %incident.id, "Incident updated in Redis");
        Ok(())
    }

    async fn delete_incident(&self, id: &Uuid) -> Result<()> {
        // Get incident first to access its data for index cleanup
        let incident = match self.get_incident(id).await? {
            Some(i) => i,
            None => {
                return Err(AppError::NotFound(format!("Incident {} not found", id)));
            }
        };

        let key = self.incident_key(id);

        let mut conn = self.connection.clone();

        // Delete incident data
        let _: () = conn
            .del(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to delete incident: {}", e)))?;

        // Remove from indices
        self.clone().remove_from_indices(&incident).await?;

        tracing::debug!(incident_id = %id, "Incident deleted from Redis");
        Ok(())
    }

    async fn list_incidents(
        &self,
        filter: &IncidentFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Incident>> {
        let mut conn = self.connection.clone();

        // Get filtered incident IDs
        let incident_ids = self.clone().get_filtered_incident_ids(filter).await?;

        // Fetch incidents
        let mut incidents: Vec<Incident> = Vec::new();
        for id_str in incident_ids {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                if let Some(incident) = self.get_incident(&id).await? {
                    // Apply active filter
                    if !filter.active_only || incident.is_active() {
                        incidents.push(incident);
                    }
                }
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
        // Get filtered incident IDs
        let incident_ids = self.clone().get_filtered_incident_ids(filter).await?;

        if !filter.active_only {
            return Ok(incident_ids.len() as u64);
        }

        // If active_only filter is set, need to check each incident
        let mut count = 0u64;
        for id_str in incident_ids {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                if let Some(incident) = self.get_incident(&id).await? {
                    if incident.is_active() {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Vec<Incident>> {
        let key = self.fingerprint_key(fingerprint);

        let mut conn = self.connection.clone();

        let incident_ids: Vec<String> = conn
            .smembers(&key)
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to query fingerprint index: {}", e))
            })?;

        let mut incidents = Vec::new();
        for id_str in incident_ids {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                if let Some(incident) = self.get_incident(&id).await? {
                    incidents.push(incident);
                }
            }
        }

        Ok(incidents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};

    // Helper to check if Redis is available
    async fn redis_available() -> bool {
        match Client::open("redis://127.0.0.1:6379/15") {
            Ok(client) => match ConnectionManager::new(client).await {
                Ok(mut conn) => {
                    redis::cmd("PING")
                        .query_async::<_, String>(&mut conn)
                        .await
                        .is_ok()
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    async fn create_test_store() -> Option<RedisStore> {
        if !redis_available().await {
            return None;
        }

        RedisStore::new_with_prefix("redis://127.0.0.1:6379/15", "test")
            .await
            .ok()
    }

    #[tokio::test]
    async fn test_save_and_get_incident() {
        let Some(store) = create_test_store().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

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

        // Cleanup
        store.delete_incident(&id).await.ok();
    }

    #[tokio::test]
    async fn test_update_incident() {
        let Some(store) = create_test_store().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

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

        // Cleanup
        store.delete_incident(&incident.id).await.ok();
    }

    #[tokio::test]
    async fn test_list_incidents_with_filter() {
        let Some(store) = create_test_store().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let mut ids = Vec::new();

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
            ids.push(incident.id);
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

        // Cleanup
        for id in ids {
            store.delete_incident(&id).await.ok();
        }
    }

    #[tokio::test]
    async fn test_fingerprint_indexing() {
        let Some(store) = create_test_store().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let mut incident1 = Incident::new(
            "sentinel".to_string(),
            "API Latency".to_string(),
            "High latency".to_string(),
            Severity::P1,
            IncidentType::Performance,
        );

        incident1.fingerprint = Some("test-fingerprint-redis".to_string());
        store.save_incident(&incident1).await.unwrap();

        let found = store
            .find_by_fingerprint("test-fingerprint-redis")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, incident1.id);

        // Cleanup
        store.delete_incident(&incident1.id).await.ok();
    }
}
