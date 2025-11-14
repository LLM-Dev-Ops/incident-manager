use crate::error::Result;
use crate::models::{Alert, Incident};
use crate::state::IncidentStore;
use chrono::{Duration, Utc};
use std::sync::Arc;

/// Deduplication engine
pub struct DeduplicationEngine {
    store: Arc<dyn IncidentStore>,
    window_secs: i64,
}

impl DeduplicationEngine {
    pub fn new(store: Arc<dyn IncidentStore>, window_secs: i64) -> Self {
        Self { store, window_secs }
    }

    /// Check if an alert is a duplicate and find existing incident
    pub async fn find_duplicate(&self, alert: &Alert) -> Result<Option<Incident>> {
        let fingerprint = alert.generate_fingerprint();

        // Find incidents with same fingerprint
        let candidates = self.store.find_by_fingerprint(&fingerprint).await?;

        if candidates.is_empty() {
            return Ok(None);
        }

        // Filter candidates within time window
        let window_start = Utc::now() - Duration::seconds(self.window_secs);

        let duplicate = candidates
            .into_iter()
            .filter(|incident| {
                // Check if incident is within time window
                incident.created_at >= window_start &&
                // Check if incident is still active
                incident.is_active()
            })
            .max_by_key(|incident| incident.created_at);

        Ok(duplicate)
    }

    /// Check if an incident is a duplicate
    pub async fn is_duplicate_incident(&self, incident: &Incident) -> Result<bool> {
        if let Some(ref fingerprint) = incident.fingerprint {
            let candidates = self.store.find_by_fingerprint(fingerprint).await?;

            let window_start = Utc::now() - Duration::seconds(self.window_secs);

            let has_duplicate = candidates
                .iter()
                .any(|existing| {
                    existing.id != incident.id &&
                    existing.created_at >= window_start &&
                    existing.is_active()
                });

            Ok(has_duplicate)
        } else {
            Ok(false)
        }
    }

    /// Merge alert into existing incident
    pub async fn merge_into_incident(
        &self,
        alert: &Alert,
        incident: &mut Incident,
    ) -> Result<()> {
        // Add timeline event
        incident.add_timeline_event(crate::models::TimelineEvent {
            timestamp: Utc::now(),
            event_type: crate::models::EventType::Created,
            actor: "deduplication-engine".to_string(),
            description: format!(
                "Merged duplicate alert from {} (alert_id: {})",
                alert.source, alert.external_id
            ),
            metadata: std::collections::HashMap::from([
                ("alert_id".to_string(), alert.id.to_string()),
                ("external_id".to_string(), alert.external_id.clone()),
            ]),
        });

        // Update incident in store
        self.store.update_incident(incident).await?;

        tracing::info!(
            incident_id = %incident.id,
            alert_id = %alert.id,
            "Alert merged into existing incident"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};
    use crate::state::InMemoryStore;

    #[tokio::test]
    async fn test_deduplication_find_duplicate() {
        let store = Arc::new(InMemoryStore::new());
        let engine = DeduplicationEngine::new(store.clone(), 900); // 15 min window

        // Create and save an incident
        let mut incident = Incident::new(
            "sentinel".to_string(),
            "API Latency High".to_string(),
            "P95 > 5s".to_string(),
            Severity::P1,
            IncidentType::Performance,
        );

        incident.fingerprint = Some(incident.generate_fingerprint());
        store.save_incident(&incident).await.unwrap();

        // Create alert with same fingerprint
        let mut alert = Alert::new(
            "ext-123".to_string(),
            "sentinel".to_string(),
            "API Latency High".to_string(),
            "P95 > 5s".to_string(),
            Severity::P1,
            IncidentType::Performance,
        );

        // Should find duplicate
        let duplicate = engine.find_duplicate(&alert).await.unwrap();
        assert!(duplicate.is_some());
        assert_eq!(duplicate.unwrap().id, incident.id);
    }

    #[tokio::test]
    async fn test_deduplication_no_duplicate_outside_window() {
        let store = Arc::new(InMemoryStore::new());
        let engine = DeduplicationEngine::new(store.clone(), 1); // 1 second window

        // Create incident
        let mut incident = Incident::new(
            "sentinel".to_string(),
            "Test".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        incident.fingerprint = Some(incident.generate_fingerprint());
        store.save_incident(&incident).await.unwrap();

        // Wait for window to expire
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Create alert
        let alert = Alert::new(
            "ext-456".to_string(),
            "sentinel".to_string(),
            "Test".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        // Should not find duplicate (outside window)
        let duplicate = engine.find_duplicate(&alert).await.unwrap();
        assert!(duplicate.is_none());
    }
}
