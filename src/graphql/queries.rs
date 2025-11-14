//! GraphQL query resolvers
//!
//! All read operations for the incident management system

use async_graphql::*;
use uuid::Uuid;

use crate::state::IncidentFilter;
use super::context::GraphQLContext;
use super::types::*;

/// Root query object
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get system health information
    async fn health(&self) -> HealthInfo {
        HealthInfo {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Get an incident by ID
    async fn incident(&self, ctx: &Context<'_>, id: Uuid) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        let incident = gql_ctx
            .incident_loader
            .load_one(id)
            .await
            .map_err(|e| Error::new(format!("Failed to load incident: {}", e)))?
            .ok_or_else(|| Error::new("Incident not found"))?;

        Ok(Incident(incident))
    }

    /// List incidents with filtering, sorting, and pagination
    async fn incidents(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filter incidents")] filter: Option<IncidentFilterInput>,
        #[graphql(desc = "Pagination parameters")] pagination: Option<PaginationInput>,
        #[graphql(desc = "Sort parameters")] sort: Option<IncidentSortInput>,
    ) -> Result<IncidentConnection> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        let pagination = pagination.unwrap_or_default();

        // Build filter from input
        let incident_filter = if let Some(filter) = filter {
            IncidentFilter {
                states: filter
                    .states
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| s.into())
                    .collect(),
                severities: filter
                    .severities
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| s.into())
                    .collect(),
                sources: filter.sources.unwrap_or_default(),
                active_only: filter.active_only.unwrap_or(false),
            }
        } else {
            IncidentFilter {
                states: vec![],
                severities: vec![],
                sources: vec![],
                active_only: false,
            }
        };

        // Get incidents and total count
        let incidents = gql_ctx
            .processor
            .store()
            .list_incidents(&incident_filter, pagination.page, pagination.page_size)
            .await
            .map_err(|e| Error::new(format!("Failed to list incidents: {}", e)))?;

        let total_count = gql_ctx
            .processor
            .store()
            .count_incidents(&incident_filter)
            .await
            .map_err(|e| Error::new(format!("Failed to count incidents: {}", e)))?;

        // TODO: Implement sorting based on sort parameter
        let _ = sort; // Suppress unused warning for now

        let page_info = PageInfo::new(pagination.page, pagination.page_size, total_count);

        Ok(IncidentConnection {
            incidents: incidents.into_iter().map(Incident).collect(),
            page_info,
        })
    }

    /// Get active incidents (convenience query)
    async fn active_incidents(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Pagination parameters")] pagination: Option<PaginationInput>,
    ) -> Result<IncidentConnection> {
        self.incidents(
            ctx,
            Some(IncidentFilterInput {
                states: None,
                severities: None,
                sources: None,
                active_only: Some(true),
            }),
            pagination,
            None,
        )
        .await
    }

    /// Get critical incidents (P0 and P1)
    async fn critical_incidents(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Pagination parameters")] pagination: Option<PaginationInput>,
    ) -> Result<IncidentConnection> {
        self.incidents(
            ctx,
            Some(IncidentFilterInput {
                states: None,
                severities: Some(vec![Severity::P0, Severity::P1]),
                sources: None,
                active_only: Some(true),
            }),
            pagination,
            None,
        )
        .await
    }

    /// Get a playbook by ID
    async fn playbook(&self, ctx: &Context<'_>, id: Uuid) -> Result<Playbook> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        let playbook = gql_ctx
            .playbook_loader
            .load_one(id)
            .await
            .map_err(|e| Error::new(format!("Failed to load playbook: {}", e)))?
            .ok_or_else(|| Error::new("Playbook not found"))?;

        Ok(Playbook(playbook))
    }

    /// List all playbooks
    async fn playbooks(&self, ctx: &Context<'_>) -> Result<Vec<Playbook>> {
        let _gql_ctx = ctx.data::<GraphQLContext>()?;

        // Access playbook_service through processor
        // Note: This requires the playbook_service to be set on the processor
        // For now, return an empty list as a placeholder
        // TODO: Add proper playbook service access to GraphQLContext
        Ok(vec![])
    }

    /// Search incidents by text
    async fn search_incidents(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Search query")] query: String,
        #[graphql(desc = "Pagination parameters")] pagination: Option<PaginationInput>,
    ) -> Result<IncidentConnection> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        let pagination = pagination.unwrap_or_default();

        // Simple search: list all incidents and filter by query string
        // In production, this should use a proper search index
        let filter = IncidentFilter {
            states: vec![],
            severities: vec![],
            sources: vec![],
            active_only: false,
        };

        let all_incidents = gql_ctx
            .processor
            .store()
            .list_incidents(&filter, 0, 1000) // Get more to search through
            .await
            .map_err(|e| Error::new(format!("Failed to list incidents: {}", e)))?;

        // Filter by query (case-insensitive search in title and description)
        let query_lower = query.to_lowercase();
        let filtered: Vec<_> = all_incidents
            .into_iter()
            .filter(|i| {
                i.title.to_lowercase().contains(&query_lower)
                    || i.description.to_lowercase().contains(&query_lower)
            })
            .collect();

        let total_count = filtered.len() as u64;

        // Apply pagination
        let start = (pagination.page * pagination.page_size) as usize;
        let _end = start + pagination.page_size as usize;
        let paginated = filtered
            .into_iter()
            .skip(start)
            .take(pagination.page_size as usize)
            .collect::<Vec<_>>();

        let page_info = PageInfo::new(pagination.page, pagination.page_size, total_count);

        Ok(IncidentConnection {
            incidents: paginated.into_iter().map(Incident).collect(),
            page_info,
        })
    }

    /// Get incident statistics
    async fn incident_stats(&self, ctx: &Context<'_>) -> Result<IncidentStats> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Get all incidents to calculate stats
        let filter = IncidentFilter {
            states: vec![],
            severities: vec![],
            sources: vec![],
            active_only: false,
        };

        let total = gql_ctx
            .processor
            .store()
            .count_incidents(&filter)
            .await
            .map_err(|e| Error::new(format!("Failed to count incidents: {}", e)))?;

        let active_filter = IncidentFilter {
            active_only: true,
            ..filter.clone()
        };

        let active = gql_ctx
            .processor
            .store()
            .count_incidents(&active_filter)
            .await
            .map_err(|e| Error::new(format!("Failed to count active incidents: {}", e)))?;

        // Count by severity
        let mut p0_count = 0;
        let mut p1_count = 0;
        let mut p2_count = 0;
        let mut p3_count = 0;
        let mut p4_count = 0;

        for severity in [
            crate::models::Severity::P0,
            crate::models::Severity::P1,
            crate::models::Severity::P2,
            crate::models::Severity::P3,
            crate::models::Severity::P4,
        ] {
            let sev_filter = IncidentFilter {
                severities: vec![severity],
                active_only: true,
                ..Default::default()
            };

            let count = gql_ctx
                .processor
                .store()
                .count_incidents(&sev_filter)
                .await
                .map_err(|e| Error::new(format!("Failed to count incidents by severity: {}", e)))?;

            match severity {
                crate::models::Severity::P0 => p0_count = count,
                crate::models::Severity::P1 => p1_count = count,
                crate::models::Severity::P2 => p2_count = count,
                crate::models::Severity::P3 => p3_count = count,
                crate::models::Severity::P4 => p4_count = count,
            }
        }

        Ok(IncidentStats {
            total,
            active,
            resolved: total - active,
            by_severity: SeverityStats {
                p0: p0_count,
                p1: p1_count,
                p2: p2_count,
                p3: p3_count,
                p4: p4_count,
            },
        })
    }
}

/// Health information
#[derive(SimpleObject)]
pub struct HealthInfo {
    pub status: String,
    pub version: String,
}

/// Incident statistics
#[derive(SimpleObject)]
pub struct IncidentStats {
    /// Total number of incidents
    pub total: u64,

    /// Number of active incidents
    pub active: u64,

    /// Number of resolved incidents
    pub resolved: u64,

    /// Breakdown by severity
    pub by_severity: SeverityStats,
}

/// Severity statistics
#[derive(SimpleObject)]
pub struct SeverityStats {
    pub p0: u64,
    pub p1: u64,
    pub p2: u64,
    pub p3: u64,
    pub p4: u64,
}
