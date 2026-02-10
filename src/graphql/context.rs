//! GraphQL context for request handling
//!
//! Provides access to services, authentication, and DataLoaders

use crate::execution::ExecutionContext;
use crate::processing::IncidentProcessor;
use async_graphql::dataloader::DataLoader;
use std::sync::Arc;

use super::dataloaders::{IncidentLoader, PlaybookLoader, RelatedIncidentsLoader};

/// GraphQL context passed to all resolvers
pub struct GraphQLContext {
    /// Incident processor for business logic
    pub processor: Arc<IncidentProcessor>,

    /// DataLoader for batch loading incidents
    pub incident_loader: DataLoader<IncidentLoader>,

    /// DataLoader for batch loading playbooks
    pub playbook_loader: DataLoader<PlaybookLoader>,

    /// DataLoader for batch loading related incidents
    pub related_incidents_loader: DataLoader<RelatedIncidentsLoader>,

    /// Optional authenticated user
    pub user: Option<String>,

    /// Optional execution context for agentics span tracking
    pub execution_context: Option<ExecutionContext>,
}

impl GraphQLContext {
    /// Create a new GraphQL context
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        // Create DataLoaders with the processor
        let incident_loader = DataLoader::new(
            IncidentLoader::new(processor.clone()),
            tokio::spawn,
        );

        let playbook_loader = DataLoader::new(
            PlaybookLoader::new(processor.clone()),
            tokio::spawn,
        );

        let related_incidents_loader = DataLoader::new(
            RelatedIncidentsLoader::new(processor.clone()),
            tokio::spawn,
        );

        Self {
            processor,
            incident_loader,
            playbook_loader,
            related_incidents_loader,
            user: None,
            execution_context: None,
        }
    }

    /// Set the authenticated user
    pub fn with_user(mut self, user: String) -> Self {
        self.user = Some(user);
        self
    }

    /// Get the current user or default to "api"
    pub fn current_user(&self) -> String {
        self.user.clone().unwrap_or_else(|| "api".to_string())
    }
}
