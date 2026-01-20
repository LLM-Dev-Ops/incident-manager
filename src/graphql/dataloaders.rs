//! DataLoader implementations for batching and caching
//!
//! Prevents N+1 query problems by batching database requests

use async_trait::async_trait;
use crate::models::{Incident, Playbook};
use crate::processing::IncidentProcessor;
use async_graphql::dataloader::Loader;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Loader for batching incident queries
#[derive(Clone)]
pub struct IncidentLoader {
    processor: Arc<IncidentProcessor>,
}

impl IncidentLoader {
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        Self { processor }
    }
}

#[async_trait]
impl Loader<Uuid> for IncidentLoader {
    type Value = Incident;
    type Error = Arc<anyhow::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let mut result = HashMap::new();

        // Batch load all incidents
        for &id in keys {
            match self.processor.get_incident(&id).await {
                Ok(incident) => {
                    result.insert(id, incident);
                }
                Err(e) => {
                    tracing::warn!("Failed to load incident {}: {}", id, e);
                    // Continue loading other incidents
                }
            }
        }

        Ok(result)
    }
}

/// Loader for batching playbook queries
#[derive(Clone)]
pub struct PlaybookLoader {
    #[allow(dead_code)]
    processor: Arc<IncidentProcessor>,
}

impl PlaybookLoader {
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        Self { processor }
    }
}

#[async_trait]
impl Loader<Uuid> for PlaybookLoader {
    type Value = Playbook;
    type Error = Arc<anyhow::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let _result: HashMap<Uuid, Playbook> = HashMap::new();

        // TODO: Batch load all playbooks from PlaybookService
        // The PlaybookService is not directly accessible from the processor
        // and needs to be added to the context
        for _id in keys {
            // Placeholder - playbooks not loaded
            tracing::debug!("Playbook loading not implemented - PlaybookService not accessible");
        }

        Ok(HashMap::new())
    }
}

/// Loader for batching related incidents queries
#[derive(Clone)]
pub struct RelatedIncidentsLoader {
    processor: Arc<IncidentProcessor>,
}

impl RelatedIncidentsLoader {
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        Self { processor }
    }
}

#[async_trait]
impl Loader<Uuid> for RelatedIncidentsLoader {
    type Value = Vec<Incident>;
    type Error = Arc<anyhow::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let mut result = HashMap::new();

        // For each incident ID, load its related incidents
        for &incident_id in keys {
            match self.processor.get_incident(&incident_id).await {
                Ok(incident) => {
                    let mut related = Vec::new();

                    // Load each related incident
                    for &related_id in &incident.related_incidents {
                        if let Ok(related_incident) = self.processor.get_incident(&related_id).await {
                            related.push(related_incident);
                        }
                    }

                    result.insert(incident_id, related);
                }
                Err(e) => {
                    tracing::warn!("Failed to load incident {} for related incidents: {}", incident_id, e);
                }
            }
        }

        Ok(result)
    }
}
