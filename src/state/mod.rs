pub mod store;
pub mod cache;
pub mod sled_store;
pub mod redis_store;
pub mod factory;

pub use store::*;
pub use cache::*;
pub use sled_store::SledStore;
pub use redis_store::RedisStore;
pub use factory::{create_store, create_in_memory_store};

use crate::error::{AppError, Result};
use crate::models::Incident;
use async_trait::async_trait;
use uuid::Uuid;

/// Trait for incident storage operations
#[async_trait]
pub trait IncidentStore: Send + Sync {
    /// Save an incident
    async fn save_incident(&self, incident: &Incident) -> Result<()>;

    /// Get an incident by ID
    async fn get_incident(&self, id: &Uuid) -> Result<Option<Incident>>;

    /// Update an incident
    async fn update_incident(&self, incident: &Incident) -> Result<()>;

    /// Delete an incident
    async fn delete_incident(&self, id: &Uuid) -> Result<()>;

    /// List incidents with filtering
    async fn list_incidents(
        &self,
        filter: &IncidentFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Incident>>;

    /// Count incidents matching filter
    async fn count_incidents(&self, filter: &IncidentFilter) -> Result<u64>;

    /// Find incidents by fingerprint
    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Vec<Incident>>;
}

/// Filter for querying incidents
#[derive(Debug, Clone, Default)]
pub struct IncidentFilter {
    pub states: Vec<crate::models::IncidentState>,
    pub severities: Vec<crate::models::Severity>,
    pub sources: Vec<String>,
    pub active_only: bool,
}
