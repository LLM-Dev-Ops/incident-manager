# GraphQL Development Guide

## Guide for Implementing and Extending the GraphQL API

This guide provides detailed instructions for developers working on the LLM Incident Manager GraphQL API implementation using async-graphql in Rust.

## Table of Contents

- [Project Setup](#project-setup)
- [Architecture Overview](#architecture-overview)
- [Adding New Types](#adding-new-types)
- [Adding Queries](#adding-queries)
- [Adding Mutations](#adding-mutations)
- [Adding Subscriptions](#adding-subscriptions)
- [DataLoader Patterns](#dataloader-patterns)
- [Testing Guidelines](#testing-guidelines)
- [Performance Optimization](#performance-optimization)
- [Security Considerations](#security-considerations)
- [Deployment](#deployment)

## Project Setup

### Dependencies

Add these dependencies to `Cargo.toml`:

```toml
[dependencies]
# GraphQL
async-graphql = { version = "7.0", features = ["dataloader", "chrono", "uuid"] }
async-graphql-axum = "7.0"

# Existing dependencies
tokio = { version = "1.35", features = ["full"] }
axum = { version = "0.7", features = ["ws", "macros"] }
tower-http = { version = "0.5", features = ["cors"] }
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "v7", "serde"] }
```

### Project Structure

```
src/
├── graphql/
│   ├── mod.rs                 # Module exports
│   ├── schema.rs              # Schema definition
│   ├── context.rs             # GraphQL context
│   ├── types/
│   │   ├── mod.rs
│   │   ├── incident.rs        # Incident types
│   │   ├── user.rs            # User types
│   │   ├── team.rs            # Team types
│   │   ├── scalars.rs         # Custom scalars
│   │   └── enums.rs           # Enum types
│   ├── queries/
│   │   ├── mod.rs
│   │   ├── incident.rs        # Incident queries
│   │   └── analytics.rs       # Analytics queries
│   ├── mutations/
│   │   ├── mod.rs
│   │   ├── incident.rs        # Incident mutations
│   │   └── playbook.rs        # Playbook mutations
│   ├── subscriptions/
│   │   ├── mod.rs
│   │   └── incident.rs        # Incident subscriptions
│   ├── dataloaders/
│   │   ├── mod.rs
│   │   ├── user.rs            # User DataLoader
│   │   └── team.rs            # Team DataLoader
│   └── guards.rs              # Authorization guards
├── api/
│   └── graphql_server.rs      # Axum server setup
└── main.rs
```

## Architecture Overview

### Schema Definition

```rust
// src/graphql/schema.rs
use async_graphql::{Schema, EmptySubscription};
use crate::graphql::{QueryRoot, MutationRoot, SubscriptionRoot};

pub type AppSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub fn build_schema(
    store: Arc<dyn IncidentStore>,
    // ... other dependencies
) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(store)
        .data(/* other services */)
        .limit_depth(10)
        .limit_complexity(1000)
        .finish()
}
```

### GraphQL Context

```rust
// src/graphql/context.rs
use async_graphql::Context;
use std::sync::Arc;

pub struct GraphQLContext {
    pub store: Arc<dyn IncidentStore>,
    pub escalation_engine: Arc<EscalationEngine>,
    pub enrichment_service: Arc<EnrichmentService>,
    pub correlation_engine: Arc<CorrelationEngine>,
    pub ml_service: Arc<MLService>,
    pub current_user: Option<User>,
}

impl GraphQLContext {
    pub fn new(/* ... */) -> Self {
        Self {
            store,
            escalation_engine,
            enrichment_service,
            correlation_engine,
            ml_service,
            current_user: None,
        }
    }
}
```

### Axum Integration

```rust
// src/api/graphql_server.rs
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router, Json,
};
use async_graphql::http::{GraphQLPlaygroundConfig, playground_source};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};

pub fn graphql_routes(schema: AppSchema) -> Router {
    Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/graphql", get(graphql_playground))
        .route("/graphql/ws", get(graphql_subscription))
        .layer(Extension(schema))
}

async fn graphql_handler(
    Extension(schema): Extension<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(
        GraphQLPlaygroundConfig::new("/graphql")
            .subscription_endpoint("/graphql/ws")
    ))
}

async fn graphql_subscription(
    Extension(schema): Extension<AppSchema>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        GraphQLSubscription::new(schema).serve(socket)
    })
}
```

## Adding New Types

### Simple Object Type

```rust
// src/graphql/types/incident.rs
use async_graphql::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, SimpleObject)]
#[graphql(complex)]
pub struct Incident {
    /// Unique incident identifier
    pub id: ID,

    /// Incident title
    pub title: String,

    /// Incident severity
    pub severity: Severity,

    /// Current status
    pub status: IncidentStatus,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    // Non-GraphQL fields (skipped)
    #[graphql(skip)]
    pub internal_data: String,
}

// Complex fields with resolvers
#[ComplexObject]
impl Incident {
    /// User assigned to this incident
    async fn assigned_to(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let loader = ctx.data_unchecked::<DataLoader<UserLoader>>();
        if let Some(user_id) = &self.assigned_to_id {
            Ok(loader.load_one(user_id.clone()).await?)
        } else {
            Ok(None)
        }
    }

    /// Related incidents
    async fn related_incidents(&self, ctx: &Context<'_>) -> Result<Vec<Incident>> {
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();
        let incidents = store.get_related_incidents(&self.id).await?;
        Ok(incidents.into_iter().map(Into::into).collect())
    }

    /// Check if incident is overdue
    async fn is_overdue(&self) -> bool {
        if let Some(deadline) = self.sla.resolution_deadline {
            deadline < Utc::now()
        } else {
            false
        }
    }
}
```

### Enum Type

```rust
// src/graphql/types/enums.rs
use async_graphql::Enum;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum Severity {
    /// Critical - System down, major impact
    P0,
    /// High - Significant impact, immediate attention required
    P1,
    /// Medium - Moderate impact, attention needed soon
    P2,
    /// Low - Minor impact, can be scheduled
    P3,
    /// Informational - Minimal or no impact
    P4,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum IncidentStatus {
    New,
    Acknowledged,
    InProgress,
    Escalated,
    Resolved,
    Closed,
}
```

### Input Object Type

```rust
// src/graphql/types/incident.rs
use async_graphql::InputObject;

#[derive(InputObject)]
pub struct CreateIncidentInput {
    /// Raw event data
    pub event: RawEventInput,

    /// Creation options
    pub options: Option<IncidentCreationOptions>,
}

#[derive(InputObject)]
pub struct RawEventInput {
    pub event_id: String,
    pub source: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub category: Category,
    pub resource: ResourceInfoInput,

    #[graphql(default)]
    pub metrics: Option<JSON>,

    #[graphql(default)]
    pub tags: Option<JSON>,
}

#[derive(InputObject)]
pub struct IncidentFilterInput {
    pub severity: Option<Vec<Severity>>,
    pub status: Option<Vec<IncidentStatus>>,
    pub category: Option<Vec<Category>>,
    pub environment: Option<Vec<Environment>>,
    pub date_range: Option<DateRangeInput>,
    pub tags: Option<JSON>,
    pub search: Option<String>,
}
```

### Custom Scalar

```rust
// src/graphql/types/scalars.rs
use async_graphql::{Scalar, ScalarType, Value};
use serde_json::Value as JsonValue;

pub struct JSON(pub JsonValue);

#[Scalar]
impl ScalarType for JSON {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::Object(map) => {
                let json = serde_json::to_value(map)?;
                Ok(JSON(json))
            }
            Value::List(list) => {
                let json = serde_json::to_value(list)?;
                Ok(JSON(json))
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        match &self.0 {
            JsonValue::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone().into(), json_to_value(v)))
                    .collect()
            ),
            JsonValue::Array(arr) => Value::List(
                arr.iter().map(json_to_value).collect()
            ),
            _ => Value::Null,
        }
    }
}
```

## Adding Queries

### Root Query Object

```rust
// src/graphql/queries/mod.rs
use async_graphql::*;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a single incident by ID
    async fn incident(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Incident>> {
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();
        let incident = store.get_incident(&id.to_string()).await?;
        Ok(incident.map(Into::into))
    }

    /// List incidents with filtering and pagination
    async fn incidents(
        &self,
        ctx: &Context<'_>,
        first: Option<usize>,
        after: Option<String>,
        filter: Option<IncidentFilterInput>,
        order_by: Option<Vec<IncidentOrderByInput>>,
    ) -> Result<IncidentConnection> {
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();

        let first = first.unwrap_or(20).min(100); // Max 100 per page

        // Build query from filter
        let query = build_incident_query(filter, order_by);

        // Fetch incidents
        let incidents = store.query_incidents(&query, first, after.as_deref()).await?;

        // Build connection
        Ok(build_connection(incidents, first))
    }

    /// Get incident analytics
    async fn analytics(
        &self,
        ctx: &Context<'_>,
        time_range: TimeRangeInput,
    ) -> Result<IncidentAnalytics> {
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();
        let analytics = store.get_analytics(
            time_range.start,
            time_range.end
        ).await?;
        Ok(analytics.into())
    }

    /// Search incidents with full-text search
    #[graphql(complexity = "first.unwrap_or(20) * 2")]
    async fn search_incidents(
        &self,
        ctx: &Context<'_>,
        query: String,
        first: Option<usize>,
        after: Option<String>,
        filter: Option<IncidentFilterInput>,
    ) -> Result<IncidentConnection> {
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();

        // Perform full-text search
        let incidents = store.search_incidents(
            &query,
            filter,
            first.unwrap_or(20),
            after.as_deref()
        ).await?;

        Ok(build_connection(incidents, first.unwrap_or(20)))
    }
}
```

### Connection/Pagination Helper

```rust
// src/graphql/types/connection.rs
use async_graphql::*;

#[derive(SimpleObject)]
pub struct IncidentConnection {
    pub edges: Vec<IncidentEdge>,
    pub page_info: PageInfo,
    pub total_count: usize,
}

#[derive(SimpleObject)]
pub struct IncidentEdge {
    pub cursor: String,
    pub node: Incident,
}

#[derive(SimpleObject)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

fn build_connection(
    incidents: Vec<models::Incident>,
    page_size: usize,
) -> IncidentConnection {
    let has_next_page = incidents.len() > page_size;
    let items: Vec<_> = incidents.into_iter().take(page_size).collect();

    let edges: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(idx, inc)| {
            let cursor = encode_cursor(inc.id.clone(), idx);
            IncidentEdge {
                cursor,
                node: inc.clone().into(),
            }
        })
        .collect();

    let start_cursor = edges.first().map(|e| e.cursor.clone());
    let end_cursor = edges.last().map(|e| e.cursor.clone());

    IncidentConnection {
        edges,
        page_info: PageInfo {
            has_next_page,
            has_previous_page: false, // Set based on actual query
            start_cursor,
            end_cursor,
        },
        total_count: items.len(),
    }
}
```

## Adding Mutations

### Root Mutation Object

```rust
// src/graphql/mutations/mod.rs
use async_graphql::*;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new incident
    async fn create_incident(
        &self,
        ctx: &Context<'_>,
        input: CreateIncidentInput,
    ) -> Result<CreateIncidentResponse> {
        // Get services from context
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();
        let processor = ctx.data_unchecked::<Arc<IncidentProcessor>>();

        // Validate input
        validate_create_input(&input)?;

        // Convert to internal model
        let alert = convert_to_alert(input.event)?;

        // Process alert
        let ack = processor.process_alert(alert).await?;

        Ok(CreateIncidentResponse {
            incident: ack.incident.into(),
            status: if ack.is_duplicate {
                CreateStatus::Duplicate
            } else {
                CreateStatus::Created
            },
            message: "Incident created successfully".to_string(),
            duplicate_of: ack.duplicate_of.map(ID::from),
        })
    }

    /// Acknowledge an incident
    #[graphql(guard = "RoleGuard::new(UserRole::Responder)")]
    async fn acknowledge_incident(
        &self,
        ctx: &Context<'_>,
        incident_id: ID,
        actor: String,
        notes: Option<String>,
    ) -> Result<AcknowledgeIncidentResponse> {
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();

        // Get incident
        let mut incident = store
            .get_incident(&incident_id.to_string())
            .await?
            .ok_or_else(|| Error::new("Incident not found"))?;

        // Validate state transition
        if incident.status != IncidentStatus::New {
            return Err(Error::new("Incident already acknowledged"));
        }

        // Update incident
        incident.status = IncidentStatus::Acknowledged;
        incident.acknowledged_at = Some(Utc::now());

        store.update_incident(&incident).await?;

        // Log action
        store.add_resolution_log(ResolutionLog {
            incident_id: incident.id.clone(),
            event_type: LogEventType::Acknowledged,
            actor: Actor {
                actor_type: ActorType::User,
                id: actor.clone(),
                name: actor,
            },
            timestamp: Utc::now(),
            notes,
            ..Default::default()
        }).await?;

        Ok(AcknowledgeIncidentResponse {
            incident: incident.into(),
            success: true,
            message: "Incident acknowledged".to_string(),
        })
    }

    /// Resolve an incident
    async fn resolve_incident(
        &self,
        ctx: &Context<'_>,
        input: ResolveIncidentInput,
    ) -> Result<ResolveIncidentResponse> {
        let store = ctx.data_unchecked::<Arc<dyn IncidentStore>>();

        // Get incident
        let mut incident = store
            .get_incident(&input.incident_id.to_string())
            .await?
            .ok_or_else(|| Error::new("Incident not found"))?;

        // Update incident
        incident.status = IncidentStatus::Resolved;
        incident.resolved_at = Some(Utc::now());
        incident.resolution = Some(ResolutionInfo {
            resolved_by: Some(input.resolved_by.clone()),
            resolution_method: input.method,
            root_cause: input.root_cause,
            resolution_notes: input.notes,
            actions_taken: input.actions.into_iter().map(Into::into).collect(),
            playbook_used: input.playbook_used.map(|id| id.to_string()),
        });

        store.update_incident(&incident).await?;

        Ok(ResolveIncidentResponse {
            incident: incident.into(),
            success: true,
            message: "Incident resolved".to_string(),
        })
    }
}
```

### Input Validation

```rust
// src/graphql/mutations/validation.rs
use async_graphql::Result;

pub fn validate_create_input(input: &CreateIncidentInput) -> Result<()> {
    // Validate title length
    if input.event.title.len() < 5 {
        return Err(Error::new("Title must be at least 5 characters"));
    }

    if input.event.title.len() > 200 {
        return Err(Error::new("Title must not exceed 200 characters"));
    }

    // Validate description
    if input.event.description.is_empty() {
        return Err(Error::new("Description is required"));
    }

    // Validate severity
    if !["P0", "P1", "P2", "P3", "P4"].contains(&input.event.severity.as_str()) {
        return Err(Error::new("Invalid severity level"));
    }

    Ok(())
}
```

## Adding Subscriptions

### Root Subscription Object

```rust
// src/graphql/subscriptions/mod.rs
use async_graphql::*;
use futures_util::Stream;
use tokio::sync::broadcast;

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to incident updates
    async fn incident_updated(
        &self,
        ctx: &Context<'_>,
        filter: Option<IncidentFilterInput>,
    ) -> Result<impl Stream<Item = IncidentUpdateEvent>> {
        let broadcaster = ctx.data_unchecked::<IncidentBroadcaster>();
        let mut receiver = broadcaster.subscribe();

        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                // Filter events
                if should_send_event(&event, &filter) {
                    yield event;
                }
            }
        })
    }

    /// Subscribe to new incidents
    async fn incident_created(
        &self,
        ctx: &Context<'_>,
        severity: Option<Vec<Severity>>,
        environment: Option<Vec<Environment>>,
    ) -> Result<impl Stream<Item = Incident>> {
        let broadcaster = ctx.data_unchecked::<IncidentBroadcaster>();
        let mut receiver = broadcaster.subscribe();

        Ok(async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                if event.update_type == UpdateType::Created {
                    // Check filters
                    let matches_severity = severity.as_ref()
                        .map(|s| s.contains(&event.incident.severity))
                        .unwrap_or(true);

                    let matches_env = environment.as_ref()
                        .map(|e| e.contains(&event.incident.environment))
                        .unwrap_or(true);

                    if matches_severity && matches_env {
                        yield event.incident;
                    }
                }
            }
        })
    }
}
```

### Event Broadcasting

```rust
// src/graphql/subscriptions/broadcaster.rs
use tokio::sync::broadcast;

pub struct IncidentBroadcaster {
    sender: broadcast::Sender<IncidentUpdateEvent>,
}

impl IncidentBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IncidentUpdateEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: IncidentUpdateEvent) {
        let _ = self.sender.send(event);
    }
}

// Publish events when incidents change
impl IncidentStore {
    async fn update_incident(&self, incident: &Incident) -> Result<()> {
        // ... update logic ...

        // Broadcast update
        if let Some(broadcaster) = self.broadcaster.as_ref() {
            broadcaster.publish(IncidentUpdateEvent {
                incident: incident.clone(),
                update_type: UpdateType::Updated,
                changed_fields: vec!["status".to_string()],
                actor: /* ... */,
                timestamp: Utc::now(),
            });
        }

        Ok(())
    }
}
```

## DataLoader Patterns

### User DataLoader

```rust
// src/graphql/dataloaders/user.rs
use async_graphql::dataloader::*;
use std::collections::HashMap;

pub struct UserLoader {
    store: Arc<dyn UserStore>,
}

impl UserLoader {
    pub fn new(store: Arc<dyn UserStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl Loader<String> for UserLoader {
    type Value = User;
    type Error = Arc<anyhow::Error>;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let users = self.store.get_users_by_ids(keys).await
            .map_err(|e| Arc::new(e.into()))?;

        Ok(users.into_iter()
            .map(|user| (user.id.clone(), user))
            .collect())
    }
}
```

### Using DataLoader

```rust
// In schema setup
let user_loader = DataLoader::new(
    UserLoader::new(user_store),
    tokio::spawn
).max_batch_size(100);

let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
    .data(user_loader)
    .finish();

// In resolver
#[ComplexObject]
impl Incident {
    async fn assigned_to(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let loader = ctx.data_unchecked::<DataLoader<UserLoader>>();
        if let Some(user_id) = &self.assigned_to_id {
            Ok(loader.load_one(user_id.clone()).await?)
        } else {
            Ok(None)
        }
    }
}
```

## Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::*;

    #[tokio::test]
    async fn test_incident_query() {
        let store = Arc::new(InMemoryStore::new());

        // Create test incident
        let incident = create_test_incident();
        store.create_incident(&incident).await.unwrap();

        // Build schema
        let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
            .data(store)
            .finish();

        // Execute query
        let query = r#"
            query {
                incident(id: "inc_123") {
                    id
                    title
                    severity
                }
            }
        "#;

        let result = schema.execute(query).await;
        assert!(result.errors.is_empty());

        let data = result.data.into_json().unwrap();
        assert_eq!(data["incident"]["id"], "inc_123");
    }

    #[tokio::test]
    async fn test_create_incident_mutation() {
        let schema = build_test_schema();

        let mutation = r#"
            mutation {
                createIncident(input: {
                    event: {
                        eventId: "evt_123"
                        source: "test"
                        title: "Test Incident"
                        description: "Test description"
                        severity: "P1"
                        category: PERFORMANCE
                        resource: {
                            type: "service"
                            id: "svc_123"
                            name: "Test Service"
                        }
                    }
                }) {
                    incident {
                        id
                        title
                    }
                    status
                }
            }
        "#;

        let result = schema.execute(mutation).await;
        assert!(result.errors.is_empty());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_graphql_endpoint() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/graphql")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"query": "{ incidents(first: 10) { totalCount } }"}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["errors"].is_null());
    assert!(json["data"]["incidents"]["totalCount"].is_number());
}
```

## Performance Optimization

### Query Complexity Limits

```rust
let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
    .limit_depth(10)
    .limit_complexity(1000)
    .finish();
```

### Custom Complexity

```rust
#[Object]
impl QueryRoot {
    #[graphql(complexity = "first.unwrap_or(20) * 2")]
    async fn incidents(
        &self,
        ctx: &Context<'_>,
        first: Option<usize>,
    ) -> Result<IncidentConnection> {
        // ...
    }
}
```

### Connection Pooling

```rust
// Use connection pool for database
let pool = sqlx::PgPool::connect(&database_url).await?;

// Share pool across resolvers
let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
    .data(pool)
    .finish();
```

### Caching with DataLoader

DataLoader automatically batches and caches requests within a single query execution:

```rust
// Multiple calls to load_one() with same key will only query once
let user1 = user_loader.load_one("user_123").await?;
let user2 = user_loader.load_one("user_123").await?; // Cached
```

## Security Considerations

### Authorization Guards

```rust
// src/graphql/guards.rs
use async_graphql::*;

pub struct RoleGuard {
    required_role: UserRole,
}

impl RoleGuard {
    pub fn new(role: UserRole) -> Self {
        Self { required_role: role }
    }
}

#[async_trait::async_trait]
impl Guard for RoleGuard {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let user = ctx.data_opt::<User>()
            .ok_or_else(|| Error::new("Unauthenticated"))?;

        if user.role >= self.required_role {
            Ok(())
        } else {
            Err(Error::new("Unauthorized"))
        }
    }
}

// Usage
#[Object]
impl MutationRoot {
    #[graphql(guard = "RoleGuard::new(UserRole::Admin)")]
    async fn delete_incident(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        // Only admins can delete
    }
}
```

### Field-Level Authorization

```rust
#[ComplexObject]
impl Incident {
    #[graphql(guard = "RoleGuard::new(UserRole::Admin)")]
    async fn sensitive_data(&self) -> Option<String> {
        self.internal_sensitive_field.clone()
    }
}
```

### Rate Limiting

```rust
use tower_governor::{GovernorLayer, GovernorConfigBuilder};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .finish()
        .unwrap(),
);

let app = Router::new()
    .route("/graphql", post(graphql_handler))
    .layer(GovernorLayer { config: Box::leak(governor_conf) });
```

## Deployment

### Docker Configuration

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/llm-incident-manager /usr/local/bin/
CMD ["llm-incident-manager"]
```

### Production Checklist

- [ ] Enable production logging
- [ ] Set appropriate complexity limits
- [ ] Configure CORS properly
- [ ] Enable request batching
- [ ] Set up monitoring/metrics
- [ ] Configure rate limiting
- [ ] Enable persistent query cache
- [ ] Set up health checks
- [ ] Configure WebSocket keep-alive
- [ ] Enable GraphQL introspection only in dev

### Environment Variables

```bash
# GraphQL Configuration
GRAPHQL_MAX_DEPTH=10
GRAPHQL_MAX_COMPLEXITY=1000
GRAPHQL_PLAYGROUND_ENABLED=false  # Disable in production
GRAPHQL_INTROSPECTION_ENABLED=false  # Disable in production

# Server
HOST=0.0.0.0
PORT=8080

# Database
DATABASE_URL=postgresql://localhost/incidents
DATABASE_POOL_SIZE=20
```

## Further Reading

- [GraphQL API Guide](./GRAPHQL_API_GUIDE.md)
- [GraphQL Schema Reference](./GRAPHQL_SCHEMA_REFERENCE.md)
- [GraphQL Integration Guide](./GRAPHQL_INTEGRATION_GUIDE.md)
- [GraphQL Examples](./GRAPHQL_EXAMPLES.md)
- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
