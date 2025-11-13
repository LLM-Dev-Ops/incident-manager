# GraphQL Implementation Guide

## Overview

This guide provides step-by-step instructions for implementing the GraphQL API for the LLM Incident Manager, including code examples, best practices, and testing strategies.

## Table of Contents

1. [Project Setup](#project-setup)
2. [Module Structure](#module-structure)
3. [Core Implementation](#core-implementation)
4. [DataLoaders](#dataloaders-implementation)
5. [Subscriptions](#subscriptions-implementation)
6. [Testing](#testing)
7. [Deployment](#deployment)

---

## Project Setup

### 1. Add Dependencies

Update `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...

# GraphQL
async-graphql = { version = "7.0", features = ["uuid", "chrono", "dataloader", "tracing"] }
async-graphql-axum = "7.0"
async-stream = "0.3"

# Additional utilities
async-trait = "0.1"
```

### 2. Create Module Structure

```bash
mkdir -p src/graphql/{resolvers,types,dataloaders}
touch src/graphql/{mod.rs,schema.rs,context.rs,scalars.rs,guards.rs}
touch src/graphql/resolvers/{mod.rs,query.rs,mutation.rs,subscription.rs}
touch src/graphql/types/{mod.rs,incident.rs,event.rs,analytics.rs}
touch src/graphql/dataloaders/{mod.rs,incident.rs,user.rs}
```

---

## Module Structure

### `src/graphql/mod.rs`

```rust
//! GraphQL API module

pub mod context;
pub mod dataloaders;
pub mod guards;
pub mod resolvers;
pub mod scalars;
pub mod schema;
pub mod types;

pub use context::GraphQLContext;
pub use schema::{build_schema, GraphQLSchema};
```

### `src/graphql/scalars.rs`

```rust
//! Custom GraphQL scalars

use async_graphql::{Scalar, ScalarType, Value};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// UUID scalar type
#[derive(Clone, Copy, Debug)]
pub struct UuidScalar(pub Uuid);

#[Scalar]
impl ScalarType for UuidScalar {
    fn parse(value: Value) -> async_graphql::InputValueResult<Self> {
        if let Value::String(s) = value {
            let uuid = Uuid::parse_str(&s)?;
            Ok(UuidScalar(uuid))
        } else {
            Err("Invalid UUID".into())
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

impl From<Uuid> for UuidScalar {
    fn from(uuid: Uuid) -> Self {
        UuidScalar(uuid)
    }
}

impl From<UuidScalar> for Uuid {
    fn from(scalar: UuidScalar) -> Self {
        scalar.0
    }
}

/// DateTime scalar type (RFC 3339)
#[derive(Clone, Copy, Debug)]
pub struct DateTimeScalar(pub DateTime<Utc>);

#[Scalar]
impl ScalarType for DateTimeScalar {
    fn parse(value: Value) -> async_graphql::InputValueResult<Self> {
        if let Value::String(s) = value {
            let dt = DateTime::parse_from_rfc3339(&s)?
                .with_timezone(&Utc);
            Ok(DateTimeScalar(dt))
        } else {
            Err("Invalid DateTime".into())
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_rfc3339())
    }
}

/// JSON scalar type
pub type JsonScalar = serde_json::Value;
```

---

## Core Implementation

### `src/graphql/context.rs`

```rust
//! GraphQL execution context

use crate::graphql::dataloaders::DataLoaders;
use crate::models::User;
use crate::services::Services;
use std::sync::Arc;
use tokio::sync::broadcast;

/// GraphQL execution context
#[derive(Clone)]
pub struct GraphQLContext {
    /// Current authenticated user
    pub current_user: Option<User>,

    /// Authentication token
    pub auth_token: Option<String>,

    /// Application services
    pub services: Arc<Services>,

    /// DataLoaders for batch loading
    pub loaders: DataLoaders,

    /// PubSub for subscriptions
    pub pubsub: Arc<PubSub>,

    /// Request metadata
    pub request_id: String,
    pub client_ip: String,
}

impl GraphQLContext {
    /// Create new context
    pub fn new(
        current_user: Option<User>,
        auth_token: Option<String>,
        services: Arc<Services>,
        loaders: DataLoaders,
        pubsub: Arc<PubSub>,
        request_id: String,
        client_ip: String,
    ) -> Self {
        Self {
            current_user,
            auth_token,
            services,
            loaders,
            pubsub,
            request_id,
            client_ip,
        }
    }

    /// Require authenticated user
    pub fn require_authenticated(&self) -> async_graphql::Result<&User> {
        self.current_user
            .as_ref()
            .ok_or_else(|| "Authentication required".into())
    }

    /// Check if user has specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.current_user
            .as_ref()
            .map(|u| u.has_role(role))
            .unwrap_or(false)
    }

    /// Require specific role
    pub fn require_role(&self, role: &str) -> async_graphql::Result<()> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err("Insufficient permissions".into())
        }
    }
}

/// PubSub for real-time subscriptions
pub struct PubSub {
    incident_created: broadcast::Sender<crate::models::Incident>,
    incident_updated: broadcast::Sender<IncidentUpdatedEvent>,
    incident_state_changed: broadcast::Sender<IncidentStateChangedEvent>,
}

impl PubSub {
    pub fn new() -> Self {
        let (incident_created, _) = broadcast::channel(1000);
        let (incident_updated, _) = broadcast::channel(1000);
        let (incident_state_changed, _) = broadcast::channel(1000);

        Self {
            incident_created,
            incident_updated,
            incident_state_changed,
        }
    }

    pub fn publish_incident_created(
        &self,
        incident: crate::models::Incident,
    ) -> Result<(), broadcast::error::SendError<crate::models::Incident>> {
        self.incident_created.send(incident)?;
        Ok(())
    }

    pub fn subscribe_incident_created(&self) -> broadcast::Receiver<crate::models::Incident> {
        self.incident_created.subscribe()
    }

    pub fn publish_incident_updated(
        &self,
        event: IncidentUpdatedEvent,
    ) -> Result<(), broadcast::error::SendError<IncidentUpdatedEvent>> {
        self.incident_updated.send(event)?;
        Ok(())
    }

    pub fn subscribe_incident_updated(&self) -> broadcast::Receiver<IncidentUpdatedEvent> {
        self.incident_updated.subscribe()
    }
}

#[derive(Clone, Debug)]
pub struct IncidentUpdatedEvent {
    pub incident: crate::models::Incident,
    pub updated_fields: Vec<String>,
    pub actor_name: String,
}

#[derive(Clone, Debug)]
pub struct IncidentStateChangedEvent {
    pub incident: crate::models::Incident,
    pub previous_state: String,
    pub new_state: String,
    pub actor_name: String,
}

/// Application services container
pub struct Services {
    pub incident_processor: Arc<crate::processing::IncidentProcessor>,
}

impl Services {
    pub fn new(incident_processor: Arc<crate::processing::IncidentProcessor>) -> Self {
        Self { incident_processor }
    }
}
```

### `src/graphql/guards.rs`

```rust
//! Authorization guards for GraphQL

use async_graphql::{Context, Guard, Result};
use crate::graphql::context::GraphQLContext;

/// Authentication guard
pub struct AuthGuard;

#[async_trait::async_trait]
impl Guard for AuthGuard {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        gql_ctx.require_authenticated()?;
        Ok(())
    }
}

/// Role-based authorization guard
pub struct RoleGuard {
    role: String,
}

impl RoleGuard {
    pub fn new(role: impl Into<String>) -> Self {
        Self { role: role.into() }
    }
}

#[async_trait::async_trait]
impl Guard for RoleGuard {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        gql_ctx.require_role(&self.role)?;
        Ok(())
    }
}
```

### `src/graphql/types/incident.rs`

```rust
//! GraphQL types for incidents

use async_graphql::{ComplexObject, Context, Enum, InputObject, Object, SimpleObject, ID};
use crate::graphql::context::GraphQLContext;
use crate::models;
use uuid::Uuid;

/// Incident severity
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum Severity {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl From<models::Severity> for Severity {
    fn from(s: models::Severity) -> Self {
        match s {
            models::Severity::P0 => Severity::P0,
            models::Severity::P1 => Severity::P1,
            models::Severity::P2 => Severity::P2,
            models::Severity::P3 => Severity::P3,
            models::Severity::P4 => Severity::P4,
        }
    }
}

impl From<Severity> for models::Severity {
    fn from(s: Severity) -> Self {
        match s {
            Severity::P0 => models::Severity::P0,
            Severity::P1 => models::Severity::P1,
            Severity::P2 => models::Severity::P2,
            Severity::P3 => models::Severity::P3,
            Severity::P4 => models::Severity::P4,
        }
    }
}

/// Incident state
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum IncidentState {
    New,
    Acknowledged,
    InProgress,
    Escalated,
    Resolved,
    Closed,
}

impl From<models::IncidentState> for IncidentState {
    fn from(s: models::IncidentState) -> Self {
        match s {
            models::IncidentState::Detected => IncidentState::New,
            models::IncidentState::Triaged => IncidentState::Acknowledged,
            models::IncidentState::Investigating => IncidentState::InProgress,
            models::IncidentState::Remediating => IncidentState::InProgress,
            models::IncidentState::Resolved => IncidentState::Resolved,
            models::IncidentState::Closed => IncidentState::Closed,
        }
    }
}

/// GraphQL Incident type
#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct Incident {
    pub id: ID,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub state: IncidentState,
    pub source: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,

    #[graphql(skip)]
    pub internal_id: Uuid,
}

impl From<models::Incident> for Incident {
    fn from(inc: models::Incident) -> Self {
        Self {
            id: inc.id.to_string().into(),
            internal_id: inc.id,
            title: inc.title,
            description: inc.description,
            severity: inc.severity.into(),
            state: inc.state.into(),
            source: inc.source,
            created_at: inc.created_at,
            updated_at: inc.updated_at,
        }
    }
}

#[ComplexObject]
impl Incident {
    /// Get incident timeline
    async fn timeline(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<TimelineEvent>> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Load full incident to get timeline
        let incident = gql_ctx
            .loaders
            .incident_loader
            .load_one(self.internal_id)
            .await?
            .ok_or("Incident not found")?;

        Ok(incident
            .timeline
            .into_iter()
            .map(TimelineEvent::from)
            .collect())
    }

    /// Get related incidents
    async fn related_incidents(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Incident>> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Load full incident to get related IDs
        let incident = gql_ctx
            .loaders
            .incident_loader
            .load_one(self.internal_id)
            .await?
            .ok_or("Incident not found")?;

        // Batch load related incidents
        let related = gql_ctx
            .loaders
            .incident_loader
            .load_many(incident.related_incidents)
            .await?;

        Ok(related.values().cloned().map(Incident::from).collect())
    }
}

/// Timeline event
#[derive(SimpleObject, Clone)]
pub struct TimelineEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub description: String,
    pub actor: String,
}

impl From<models::TimelineEvent> for TimelineEvent {
    fn from(event: models::TimelineEvent) -> Self {
        Self {
            timestamp: event.timestamp,
            event_type: event.event_type.to_string(),
            description: event.description,
            actor: event.actor,
        }
    }
}

/// Create incident input
#[derive(InputObject)]
pub struct CreateIncidentInput {
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub category: String,
}

impl From<CreateIncidentInput> for models::Incident {
    fn from(input: CreateIncidentInput) -> Self {
        models::Incident::new(
            "graphql-api".to_string(),
            input.title,
            input.description,
            input.severity.into(),
            models::IncidentType::Unknown,
        )
    }
}

/// Update incident input
#[derive(InputObject)]
pub struct UpdateIncidentInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub severity: Option<Severity>,
    pub state: Option<IncidentState>,
}

/// Incident filter
#[derive(InputObject)]
pub struct IncidentFilter {
    pub severities: Option<Vec<Severity>>,
    pub states: Option<Vec<IncidentState>>,
    pub sources: Option<Vec<String>>,
    pub active_only: Option<bool>,
}

/// Pagination input
#[derive(InputObject)]
pub struct PaginationInput {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// Incident connection for pagination
#[derive(SimpleObject)]
pub struct IncidentConnection {
    pub edges: Vec<IncidentEdge>,
    pub page_info: PageInfo,
    pub total_count: i32,
}

#[derive(SimpleObject)]
pub struct IncidentEdge {
    pub node: Incident,
    pub cursor: String,
}

#[derive(SimpleObject)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}
```

### `src/graphql/resolvers/query.rs`

```rust
//! GraphQL Query resolvers

use async_graphql::{Context, Object, Result, ID};
use crate::graphql::context::GraphQLContext;
use crate::graphql::types::incident::*;
use uuid::Uuid;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get system health
    async fn health(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: 0,
        })
    }

    /// Get incident by ID
    async fn incident(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Incident>> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        let uuid = id.parse::<Uuid>()?;

        let incident = gql_ctx
            .loaders
            .incident_loader
            .load_one(uuid)
            .await?;

        Ok(incident.map(Incident::from))
    }

    /// List incidents with filtering
    async fn incidents(
        &self,
        ctx: &Context<'_>,
        filter: Option<IncidentFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<IncidentConnection> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Require authentication
        gql_ctx.require_authenticated()?;

        // Get pagination params
        let page = pagination.as_ref().and_then(|p| p.page).unwrap_or(0);
        let page_size = pagination
            .as_ref()
            .and_then(|p| p.page_size)
            .unwrap_or(20)
            .min(100);

        // Build filter
        let mut incident_filter = crate::state::IncidentFilter::default();

        if let Some(f) = filter {
            if let Some(states) = f.states {
                incident_filter.states = states
                    .into_iter()
                    .map(|s| match s {
                        IncidentState::New => crate::models::IncidentState::Detected,
                        IncidentState::Acknowledged => crate::models::IncidentState::Triaged,
                        IncidentState::InProgress => crate::models::IncidentState::Investigating,
                        IncidentState::Resolved => crate::models::IncidentState::Resolved,
                        IncidentState::Closed => crate::models::IncidentState::Closed,
                        _ => crate::models::IncidentState::Detected,
                    })
                    .collect();
            }

            if let Some(severities) = f.severities {
                incident_filter.severities = severities
                    .into_iter()
                    .map(|s| s.into())
                    .collect();
            }

            if let Some(sources) = f.sources {
                incident_filter.sources = sources;
            }

            if let Some(active_only) = f.active_only {
                incident_filter.active_only = active_only;
            }
        }

        // Get incidents
        let incidents = gql_ctx
            .services
            .incident_processor
            .store
            .list_incidents(&incident_filter, page, page_size)
            .await?;

        let total = gql_ctx
            .services
            .incident_processor
            .store
            .count_incidents(&incident_filter)
            .await? as i32;

        // Build connection
        let edges: Vec<IncidentEdge> = incidents
            .into_iter()
            .enumerate()
            .map(|(idx, inc)| {
                let cursor = format!("{}:{}", page, idx);
                IncidentEdge {
                    node: Incident::from(inc),
                    cursor,
                }
            })
            .collect();

        let has_next = (page + 1) * page_size < total as u32;
        let has_prev = page > 0;

        Ok(IncidentConnection {
            edges,
            page_info: PageInfo {
                has_next_page: has_next,
                has_previous_page: has_prev,
                start_cursor: Some(format!("{}:0", page)),
                end_cursor: Some(format!("{}:{}", page, page_size - 1)),
            },
            total_count: total,
        })
    }
}

#[derive(async_graphql::SimpleObject)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub uptime: u64,
}
```

### `src/graphql/resolvers/mutation.rs`

```rust
//! GraphQL Mutation resolvers

use async_graphql::{Context, Object, Result, ID};
use crate::graphql::context::GraphQLContext;
use crate::graphql::guards::AuthGuard;
use crate::graphql::types::incident::*;
use uuid::Uuid;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new incident
    #[graphql(guard = "AuthGuard")]
    async fn create_incident(
        &self,
        ctx: &Context<'_>,
        input: CreateIncidentInput,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Create incident
        let incident: crate::models::Incident = input.into();

        let created = gql_ctx
            .services
            .incident_processor
            .create_incident(incident)
            .await?;

        // Publish event
        let _ = gql_ctx.pubsub.publish_incident_created(created.clone());

        Ok(Incident::from(created))
    }

    /// Update incident
    #[graphql(guard = "AuthGuard")]
    async fn update_incident(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateIncidentInput,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        let uuid = id.parse::<Uuid>()?;

        // Get incident
        let mut incident = gql_ctx
            .services
            .incident_processor
            .get_incident(&uuid)
            .await?;

        // Apply updates
        if let Some(title) = input.title {
            incident.title = title;
        }

        if let Some(description) = input.description {
            incident.description = description;
        }

        if let Some(severity) = input.severity {
            incident.severity = severity.into();
        }

        if let Some(state) = input.state {
            let new_state = match state {
                IncidentState::New => crate::models::IncidentState::Detected,
                IncidentState::Acknowledged => crate::models::IncidentState::Triaged,
                IncidentState::InProgress => crate::models::IncidentState::Investigating,
                IncidentState::Resolved => crate::models::IncidentState::Resolved,
                IncidentState::Closed => crate::models::IncidentState::Closed,
                _ => crate::models::IncidentState::Detected,
            };
            incident.update_state(new_state, "graphql-user".to_string());
        }

        // Save
        gql_ctx
            .services
            .incident_processor
            .store
            .update_incident(&incident)
            .await?;

        Ok(Incident::from(incident))
    }

    /// Resolve incident
    #[graphql(guard = "AuthGuard")]
    async fn resolve_incident(
        &self,
        ctx: &Context<'_>,
        id: ID,
        notes: String,
        root_cause: Option<String>,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        let uuid = id.parse::<Uuid>()?;

        let user = gql_ctx.require_authenticated()?;

        let incident = gql_ctx
            .services
            .incident_processor
            .resolve_incident(
                &uuid,
                user.name.clone(),
                crate::models::ResolutionMethod::Manual,
                notes,
                root_cause,
            )
            .await?;

        Ok(Incident::from(incident))
    }
}
```

---

## DataLoaders Implementation

### `src/graphql/dataloaders/mod.rs`

```rust
//! DataLoaders for batching and caching

pub mod incident;
pub mod user;

use async_graphql::dataloader::DataLoader;
use std::sync::Arc;

/// Container for all DataLoaders
#[derive(Clone)]
pub struct DataLoaders {
    pub incident_loader: DataLoader<incident::IncidentLoader>,
}

impl DataLoaders {
    pub fn new(store: Arc<dyn crate::state::IncidentStore>) -> Self {
        Self {
            incident_loader: DataLoader::new(
                incident::IncidentLoader::new(store),
                tokio::spawn,
            )
            .max_batch_size(100)
            .delay(std::time::Duration::from_millis(10)),
        }
    }
}
```

### `src/graphql/dataloaders/incident.rs`

```rust
//! Incident DataLoader

use async_graphql::dataloader::Loader;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct IncidentLoader {
    store: Arc<dyn crate::state::IncidentStore>,
}

impl IncidentLoader {
    pub fn new(store: Arc<dyn crate::state::IncidentStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl Loader<Uuid> for IncidentLoader {
    type Value = crate::models::Incident;
    type Error = Arc<crate::error::AppError>;

    async fn load(
        &self,
        keys: &[Uuid],
    ) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Batch load all incidents
        let mut results = HashMap::new();

        for id in keys {
            if let Ok(Some(incident)) = self.store.get_incident(id).await {
                results.insert(*id, incident);
            }
        }

        Ok(results)
    }
}
```

---

## Subscriptions Implementation

### `src/graphql/resolvers/subscription.rs`

```rust
//! GraphQL Subscription resolvers

use async_graphql::{Context, Subscription};
use crate::graphql::context::GraphQLContext;
use crate::graphql::types::incident::*;
use futures_util::Stream;

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to incident creation
    async fn incident_created<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        filter: Option<IncidentFilter>,
    ) -> impl Stream<Item = Incident> + 'ctx {
        let gql_ctx = ctx.data::<GraphQLContext>().unwrap();
        let mut rx = gql_ctx.pubsub.subscribe_incident_created();

        async_stream::stream! {
            while let Ok(incident) = rx.recv().await {
                // Apply filter if provided
                if matches_filter(&incident, &filter) {
                    yield Incident::from(incident);
                }
            }
        }
    }
}

fn matches_filter(
    incident: &crate::models::Incident,
    filter: &Option<IncidentFilter>,
) -> bool {
    if let Some(f) = filter {
        // Check severity
        if let Some(ref severities) = f.severities {
            if !severities.contains(&incident.severity.into()) {
                return false;
            }
        }

        // Check state
        if let Some(ref states) = f.states {
            if !states.contains(&incident.state.clone().into()) {
                return false;
            }
        }

        // Check active only
        if let Some(active_only) = f.active_only {
            if active_only && !incident.is_active() {
                return false;
            }
        }
    }

    true
}
```

---

## Schema Assembly

### `src/graphql/schema.rs`

```rust
//! GraphQL schema builder

use async_graphql::{Schema, EmptySubscription};
use crate::graphql::resolvers::{QueryRoot, MutationRoot, SubscriptionRoot};

pub type GraphQLSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub fn build_schema() -> GraphQLSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .limit_depth(10)
        .limit_complexity(1000)
        .finish()
}
```

---

## Axum Integration

### `src/main.rs` (GraphQL routes)

```rust
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};

async fn graphql_handler(
    State(schema): State<GraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

pub fn graphql_router(schema: GraphQLSchema) -> Router {
    Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        .route(
            "/graphql/ws",
            get(GraphQLSubscription::new(schema.clone())),
        )
        .with_state(schema)
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_incident() {
        let schema = build_test_schema();

        let query = r#"
            mutation {
                createIncident(input: {
                    title: "Test Incident"
                    description: "Test"
                    severity: P2
                    category: "performance"
                }) {
                    id
                    title
                    severity
                }
            }
        "#;

        let res = schema.execute(query).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_list_incidents() {
        let schema = build_test_schema();

        let query = r#"
            query {
                incidents(pagination: { pageSize: 10 }) {
                    edges {
                        node {
                            id
                            title
                        }
                    }
                    totalCount
                }
            }
        "#;

        let res = schema.execute(query).await;
        assert!(res.is_ok());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_graphql_endpoint() {
    let app = create_test_app();

    let response = reqwest::Client::new()
        .post("http://localhost:3000/graphql")
        .json(&serde_json::json!({
            "query": "{ health { status } }"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
```

---

## Deployment

### Production Configuration

```rust
pub fn build_production_schema() -> GraphQLSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .limit_depth(10)
        .limit_complexity(1000)
        .enable_federation()
        .extension(async_graphql::extensions::Tracing)
        .finish()
}
```

### Environment Variables

```bash
GRAPHQL_PLAYGROUND_ENABLED=false
GRAPHQL_DEPTH_LIMIT=10
GRAPHQL_COMPLEXITY_LIMIT=1000
GRAPHQL_MAX_BATCH_SIZE=100
```

### Monitoring

Add OpenTelemetry tracing:

```rust
use async_graphql::extensions::Tracing;

Schema::build(...)
    .extension(Tracing)
    .finish()
```

---

## Best Practices

1. **Always use DataLoaders** for related data
2. **Validate inputs** before processing
3. **Implement proper error handling** with error extensions
4. **Use guards** for authentication and authorization
5. **Set query limits** (depth, complexity)
6. **Cache frequently accessed data**
7. **Monitor query performance**
8. **Document your schema** with descriptions
9. **Version your API** carefully
10. **Test thoroughly** with unit and integration tests

---

## Next Steps

1. Implement remaining resolvers
2. Add comprehensive tests
3. Set up monitoring
4. Create client examples
5. Write API documentation
6. Performance testing
7. Security audit
8. Production deployment
