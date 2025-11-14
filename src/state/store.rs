use crate::error::{AppError, Result};
use crate::models::Incident;
use crate::state::{IncidentFilter, IncidentStore};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

/// In-memory incident store (for MVP and testing)
#[derive(Clone)]
pub struct InMemoryStore {
    incidents: Arc<DashMap<Uuid, Incident>>,
    fingerprint_index: Arc<DashMap<String, Vec<Uuid>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            incidents: Arc::new(DashMap::new()),
            fingerprint_index: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IncidentStore for InMemoryStore {
    async fn save_incident(&self, incident: &Incident) -> Result<()> {
        self.incidents.insert(incident.id, incident.clone());

        // Update fingerprint index if present
        if let Some(ref fingerprint) = incident.fingerprint {
            self.fingerprint_index
                .entry(fingerprint.clone())
                .or_insert_with(Vec::new)
                .push(incident.id);
        }

        tracing::debug!(incident_id = %incident.id, "Incident saved");
        Ok(())
    }

    async fn get_incident(&self, id: &Uuid) -> Result<Option<Incident>> {
        Ok(self.incidents.get(id).map(|entry| entry.clone()))
    }

    async fn update_incident(&self, incident: &Incident) -> Result<()> {
        if self.incidents.contains_key(&incident.id) {
            self.incidents.insert(incident.id, incident.clone());
            tracing::debug!(incident_id = %incident.id, "Incident updated");
            Ok(())
        } else {
            Err(AppError::NotFound(format!(
                "Incident {} not found",
                incident.id
            )))
        }
    }

    async fn delete_incident(&self, id: &Uuid) -> Result<()> {
        if let Some((_, incident)) = self.incidents.remove(id) {
            // Remove from fingerprint index
            if let Some(ref fingerprint) = incident.fingerprint {
                if let Some(mut entry) = self.fingerprint_index.get_mut(fingerprint) {
                    entry.retain(|&incident_id| incident_id != *id);
                }
            }
            tracing::debug!(incident_id = %id, "Incident deleted");
            Ok(())
        } else {
            Err(AppError::NotFound(format!("Incident {} not found", id)))
        }
    }

    async fn list_incidents(
        &self,
        filter: &IncidentFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Incident>> {
        let mut incidents: Vec<Incident> = self
            .incidents
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|incident| {
                // Apply state filter
                let state_match = filter.states.is_empty()
                    || filter.states.contains(&incident.state);

                // Apply severity filter
                let severity_match = filter.severities.is_empty()
                    || filter.severities.contains(&incident.severity);

                // Apply source filter
                let source_match = filter.sources.is_empty()
                    || filter.sources.iter().any(|s| incident.source.contains(s));

                // Apply active filter
                let active_match = !filter.active_only || incident.is_active();

                state_match && severity_match && source_match && active_match
            })
            .collect();

        // Sort by creation time (newest first)
        incidents.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        let start = (page * page_size) as usize;
        let _end = start + page_size as usize;

        Ok(incidents
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect())
    }

    async fn count_incidents(&self, filter: &IncidentFilter) -> Result<u64> {
        let count = self
            .incidents
            .iter()
            .filter(|entry| {
                let incident = entry.value();

                let state_match = filter.states.is_empty()
                    || filter.states.contains(&incident.state);

                let severity_match = filter.severities.is_empty()
                    || filter.severities.contains(&incident.severity);

                let source_match = filter.sources.is_empty()
                    || filter.sources.iter().any(|s| incident.source.contains(s));

                let active_match = !filter.active_only || incident.is_active();

                state_match && severity_match && source_match && active_match
            })
            .count();

        Ok(count as u64)
    }

    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Vec<Incident>> {
        if let Some(incident_ids) = self.fingerprint_index.get(fingerprint) {
            let incidents: Vec<Incident> = incident_ids
                .iter()
                .filter_map(|id| self.incidents.get(id).map(|entry| entry.clone()))
                .collect();
            Ok(incidents)
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};

    #[tokio::test]
    async fn test_save_and_get_incident() {
        let store = InMemoryStore::new();

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
        let store = InMemoryStore::new();

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
    async fn test_list_incidents_with_filter() {
        let store = InMemoryStore::new();

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
        let store = InMemoryStore::new();

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
}
