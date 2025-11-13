# GraphQL Architecture for LLM Incident Manager

## Executive Summary

This document defines a comprehensive, enterprise-grade GraphQL API architecture for the LLM Incident Manager. The design emphasizes scalability, type safety, performance optimization, and production readiness while maintaining commercial viability through an intuitive, developer-friendly API.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Library Selection](#library-selection)
3. [Schema Design](#schema-design)
4. [Resolver Architecture](#resolver-architecture)
5. [Context Design](#context-design)
6. [DataLoader Pattern](#dataloader-pattern)
7. [Subscription System](#subscription-system)
8. [Performance Optimization](#performance-optimization)
9. [Security Considerations](#security-considerations)
10. [Error Handling](#error-handling)
11. [Integration with REST API](#integration-with-rest-api)
12. [Implementation Roadmap](#implementation-roadmap)

---

## Architecture Overview

### Design Principles

1. **Type Safety**: Leverage Rust's type system and GraphQL's schema for end-to-end type safety
2. **Performance**: Use DataLoaders, batching, and caching to prevent N+1 queries
3. **Developer Experience**: Intuitive schema design with clear naming and comprehensive documentation
4. **Scalability**: Stateless resolvers, efficient subscription management, and horizontal scaling support
5. **Security**: Field-level authorization, rate limiting, and query complexity analysis

### Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    GraphQL Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Schema    │  │  Resolvers  │  │ Subscriptions│        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                    Context Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │    Auth     │  │ DataLoaders │  │   Services  │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                    Service Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │  Incident   │  │   Event     │  │  Analytics  │        │
│  │  Processor  │  │  Processor  │  │   Service   │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                    Storage Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Redis     │  │    Sled     │  │   PostgreSQL│        │
│  │  (Cache)    │  │  (Local)    │  │  (Future)   │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

---

## Library Selection

### Chosen: `async-graphql`

**Decision**: Use `async-graphql` as the primary GraphQL library.

### Justification

#### Pros of async-graphql:

1. **Modern & Mature**: Actively maintained, production-ready
2. **Async/Await Native**: Built for Tokio, perfect fit for our async architecture
3. **Excellent Performance**: Efficient query execution and minimal overhead
4. **Rich Feature Set**:
   - Built-in DataLoader support
   - WebSocket subscriptions (GraphQL over WebSocket)
   - Query complexity analysis
   - Custom directives
   - Federation support (for future microservices)
5. **Great DX**: Procedural macros for easy schema definition
6. **Strong Type Safety**: Compile-time schema validation
7. **Axum Integration**: First-class support for Axum framework
8. **Documentation**: Comprehensive docs and examples

#### Cons:

1. Smaller ecosystem compared to JavaScript GraphQL libraries
2. Some advanced features require manual implementation
3. Learning curve for GraphQL-specific patterns in Rust

#### Alternative Considered: Juniper

**Why Not Juniper**:
- Less active development
- No built-in DataLoader support
- Weaker subscription support
- Less intuitive API design
- No query complexity analysis out-of-the-box

### Dependencies

```toml
[dependencies]
async-graphql = { version = "7.0", features = ["uuid", "chrono", "dataloader"] }
async-graphql-axum = "7.0"
```

---

## Schema Design

### Schema Organization

The schema is organized into logical domains:

1. **Core Domain**: Incidents, Events, Alerts
2. **Analytics Domain**: Metrics, Trends, Reports
3. **Policy Domain**: Escalation Policies, SLA Rules
4. **User Domain**: Users, Teams, Permissions
5. **System Domain**: Health, Configuration

### Key Design Patterns

#### 1. Connection Pattern (Relay Specification)

Used for pagination with cursor-based navigation:

```graphql
type IncidentConnection {
  edges: [IncidentEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type IncidentEdge {
  node: Incident!
  cursor: String!
}

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}
```

**Benefits**:
- Efficient pagination for large datasets
- Stable cursors across data changes
- Bidirectional pagination support
- Client-side caching friendly

#### 2. Input Object Pattern

Separate input types for mutations:

```graphql
input CreateIncidentInput {
  title: String!
  description: String!
  severity: Severity!
  category: Category!
  # ...
}

type CreateIncidentPayload {
  incident: Incident!
}
```

**Benefits**:
- Clear mutation signatures
- Reusable input types
- Better validation
- Extensible without breaking changes

#### 3. Payload Pattern

Mutations return payload types with additional metadata:

```graphql
type CreateEventPayload {
  status: EventStatus!
  event: Event
  incident: Incident
  duplicateOf: UUID
  message: String!
}
```

**Benefits**:
- Rich mutation responses
- Error handling without exceptions
- Additional context for clients

#### 4. Filter Pattern

Flexible filtering with typed input objects:

```graphql
input IncidentFilter {
  severities: [Severity!]
  states: [IncidentState!]
  categories: [Category!]
  startDate: DateTime
  endDate: DateTime
  assignedToMe: Boolean
  tags: JSON
}
```

**Benefits**:
- Type-safe filtering
- Composable filters
- Self-documenting API

### Custom Scalars

```rust
// DateTime: RFC 3339 compliant
scalar DateTime

// JSON: Flexible metadata
scalar JSON

// UUID: Unique identifiers
scalar UUID
```

### Field Resolution Strategy

1. **Eager Loading**: Load primary fields immediately
2. **Lazy Loading**: Use DataLoaders for related entities
3. **Smart Loading**: Batch and cache based on query pattern

---

## Resolver Architecture

### Resolver Organization

```
src/graphql/
├── mod.rs              # GraphQL module entry
├── schema.rs           # Schema assembly
├── context.rs          # Context definition
├── resolvers/
│   ├── mod.rs          # Resolver module
│   ├── query.rs        # Query resolvers
│   ├── mutation.rs     # Mutation resolvers
│   ├── subscription.rs # Subscription resolvers
│   ├── incident.rs     # Incident type resolvers
│   ├── event.rs        # Event type resolvers
│   ├── analytics.rs    # Analytics resolvers
│   └── policy.rs       # Policy resolvers
├── dataloaders/
│   ├── mod.rs          # DataLoader module
│   ├── incident.rs     # Incident DataLoader
│   ├── user.rs         # User DataLoader
│   └── team.rs         # Team DataLoader
├── types/
│   ├── mod.rs          # GraphQL types
│   ├── incident.rs     # Incident GraphQL types
│   ├── event.rs        # Event GraphQL types
│   └── analytics.rs    # Analytics types
├── scalars.rs          # Custom scalar implementations
├── guards.rs           # Authorization guards
└── directives.rs       # Custom directives
```

### Resolver Implementation Pattern

```rust
use async_graphql::{Context, Object, Result, ID};
use crate::graphql::context::GraphQLContext;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a single incident by ID
    async fn incident(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Option<Incident>> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        let uuid = id.parse::<Uuid>()?;

        // Use DataLoader to batch and cache
        let incident = gql_ctx
            .loaders
            .incident_loader
            .load_one(uuid)
            .await?;

        Ok(incident)
    }

    /// List incidents with filtering and pagination
    async fn incidents(
        &self,
        ctx: &Context<'_>,
        filter: Option<IncidentFilter>,
        pagination: Option<PaginationInput>,
        sort: Option<SortInput>,
    ) -> Result<IncidentConnection> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Authorization check
        gql_ctx.require_authenticated()?;

        // Get incidents from service layer
        let incidents = gql_ctx
            .services
            .incident_service
            .list_incidents(filter, pagination, sort)
            .await?;

        // Convert to connection
        Ok(incidents.into_connection())
    }
}
```

### Mutation Pattern

```rust
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new incident
    #[graphql(guard = "AuthGuard::new(Role::Responder)")]
    async fn create_incident(
        &self,
        ctx: &Context<'_>,
        input: CreateIncidentInput,
    ) -> Result<CreateIncidentPayload> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Validate input
        input.validate()?;

        // Create incident via service
        let incident = gql_ctx
            .services
            .incident_service
            .create_incident(input.into())
            .await?;

        // Publish subscription event
        gql_ctx
            .pubsub
            .publish_incident_created(incident.clone())
            .await?;

        Ok(CreateIncidentPayload { incident })
    }
}
```

### Type Resolver Pattern

```rust
use async_graphql::{ComplexObject, SimpleObject};

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Incident {
    pub id: ID,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub state: IncidentState,
    // Simple fields...

    #[graphql(skip)] // Skip complex fields
    _events: Vec<Event>,
}

#[ComplexObject]
impl Incident {
    /// Get related incidents (uses DataLoader)
    async fn related_incidents(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Incident>> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        let incident_ids = self.get_related_ids();

        let incidents = gql_ctx
            .loaders
            .incident_loader
            .load_many(incident_ids)
            .await?;

        Ok(incidents.values().cloned().collect())
    }

    /// Get incident events (uses DataLoader)
    async fn events(&self, ctx: &Context<'_>) -> Result<Vec<Event>> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        gql_ctx
            .loaders
            .event_by_incident_loader
            .load_one(self.id.parse()?)
            .await?
            .ok_or_else(|| "Events not found".into())
    }

    /// Get comments with pagination
    async fn comments(
        &self,
        ctx: &Context<'_>,
        pagination: Option<PaginationInput>,
    ) -> Result<CommentConnection> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        let comments = gql_ctx
            .services
            .comment_service
            .list_comments(&self.id, pagination)
            .await?;

        Ok(comments.into_connection())
    }
}
```

---

## Context Design

### Context Structure

```rust
use std::sync::Arc;
use async_graphql::dataloader::DataLoader;
use crate::services::*;

/// GraphQL execution context
pub struct GraphQLContext {
    /// Current user (if authenticated)
    pub current_user: Option<User>,

    /// Authentication token
    pub auth_token: Option<String>,

    /// Services
    pub services: Services,

    /// DataLoaders (for batching and caching)
    pub loaders: DataLoaders,

    /// PubSub for subscriptions
    pub pubsub: Arc<PubSub>,

    /// Request metadata
    pub request_id: String,
    pub client_ip: String,
    pub user_agent: String,
}

/// Service layer container
pub struct Services {
    pub incident_service: Arc<IncidentService>,
    pub event_service: Arc<EventService>,
    pub analytics_service: Arc<AnalyticsService>,
    pub policy_service: Arc<PolicyService>,
    pub user_service: Arc<UserService>,
    pub comment_service: Arc<CommentService>,
}

/// DataLoader container
pub struct DataLoaders {
    pub incident_loader: DataLoader<IncidentLoader>,
    pub user_loader: DataLoader<UserLoader>,
    pub team_loader: DataLoader<TeamLoader>,
    pub event_by_incident_loader: DataLoader<EventByIncidentLoader>,
    pub comment_by_incident_loader: DataLoader<CommentByIncidentLoader>,
}

impl GraphQLContext {
    /// Create context from HTTP request
    pub async fn from_request(
        req: &Request,
        services: Services,
        loaders: DataLoaders,
        pubsub: Arc<PubSub>,
    ) -> Result<Self> {
        let auth_token = extract_auth_token(req);
        let current_user = if let Some(token) = &auth_token {
            services.user_service.verify_token(token).await?
        } else {
            None
        };

        Ok(Self {
            current_user,
            auth_token,
            services,
            loaders,
            pubsub,
            request_id: generate_request_id(),
            client_ip: extract_client_ip(req),
            user_agent: extract_user_agent(req),
        })
    }

    /// Require authenticated user
    pub fn require_authenticated(&self) -> Result<&User> {
        self.current_user
            .as_ref()
            .ok_or_else(|| "Authentication required".into())
    }

    /// Check if user has role
    pub fn has_role(&self, role: &Role) -> bool {
        self.current_user
            .as_ref()
            .map(|u| u.has_role(role))
            .unwrap_or(false)
    }

    /// Require specific role
    pub fn require_role(&self, role: &Role) -> Result<()> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err("Insufficient permissions".into())
        }
    }
}
```

### Context Creation in Axum

```rust
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

pub async fn graphql_context_middleware(
    req: Request,
    next: Next,
) -> Response {
    // Extract context data from request
    let context = GraphQLContext::from_request(
        &req,
        services.clone(),
        create_dataloaders(),
        pubsub.clone(),
    ).await;

    // Add context to request extensions
    req.extensions_mut().insert(context);

    next.run(req).await
}
```

---

## DataLoader Pattern

### Problem: N+1 Query Problem

Without DataLoaders:
```
Query: Get 100 incidents with their assignees

1. SELECT * FROM incidents LIMIT 100
2. SELECT * FROM users WHERE id = 1  (for incident 1)
3. SELECT * FROM users WHERE id = 2  (for incident 2)
...
101. SELECT * FROM users WHERE id = 100 (for incident 100)

Total: 101 queries
```

With DataLoaders:
```
Query: Get 100 incidents with their assignees

1. SELECT * FROM incidents LIMIT 100
2. SELECT * FROM users WHERE id IN (1,2,3,...,100)

Total: 2 queries
```

### DataLoader Implementation

```rust
use async_graphql::dataloader::{DataLoader, Loader};
use std::collections::HashMap;
use uuid::Uuid;

/// Incident DataLoader
pub struct IncidentLoader {
    store: Arc<dyn IncidentStore>,
}

#[async_trait::async_trait]
impl Loader<Uuid> for IncidentLoader {
    type Value = Incident;
    type Error = Arc<AppError>;

    async fn load(
        &self,
        keys: &[Uuid],
    ) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Batch load all requested incidents
        let incidents = self.store
            .get_incidents_by_ids(keys)
            .await
            .map_err(Arc::new)?;

        // Convert to HashMap for DataLoader
        let map = incidents
            .into_iter()
            .map(|inc| (inc.id, inc))
            .collect();

        Ok(map)
    }
}

/// User DataLoader
pub struct UserLoader {
    store: Arc<dyn UserStore>,
}

#[async_trait::async_trait]
impl Loader<Uuid> for UserLoader {
    type Value = User;
    type Error = Arc<AppError>;

    async fn load(
        &self,
        keys: &[Uuid],
    ) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let users = self.store
            .get_users_by_ids(keys)
            .await
            .map_err(Arc::new)?;

        let map = users
            .into_iter()
            .map(|user| (user.id, user))
            .collect();

        Ok(map)
    }
}

/// Event by Incident DataLoader (one-to-many)
pub struct EventByIncidentLoader {
    store: Arc<dyn EventStore>,
}

#[async_trait::async_trait]
impl Loader<Uuid> for EventByIncidentLoader {
    type Value = Vec<Event>;
    type Error = Arc<AppError>;

    async fn load(
        &self,
        keys: &[Uuid],
    ) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Batch load all events for these incidents
        let events = self.store
            .get_events_by_incident_ids(keys)
            .await
            .map_err(Arc::new)?;

        // Group events by incident_id
        let mut map: HashMap<Uuid, Vec<Event>> = HashMap::new();
        for event in events {
            if let Some(incident_id) = event.incident_id {
                map.entry(incident_id)
                    .or_insert_with(Vec::new)
                    .push(event);
            }
        }

        Ok(map)
    }
}
```

### DataLoader Configuration

```rust
fn create_dataloaders(services: &Services) -> DataLoaders {
    DataLoaders {
        incident_loader: DataLoader::new(
            IncidentLoader {
                store: services.incident_service.store.clone(),
            },
            tokio::spawn,
        )
        .max_batch_size(100)
        .delay(std::time::Duration::from_millis(10)),

        user_loader: DataLoader::new(
            UserLoader {
                store: services.user_service.store.clone(),
            },
            tokio::spawn,
        )
        .max_batch_size(100),

        event_by_incident_loader: DataLoader::new(
            EventByIncidentLoader {
                store: services.event_service.store.clone(),
            },
            tokio::spawn,
        ),

        // ... other loaders
    }
}
```

### Best Practices

1. **Batch Size**: Set appropriate `max_batch_size` (typically 50-100)
2. **Delay**: Add small delay to collect more requests (5-10ms)
3. **Caching**: DataLoaders cache within single request
4. **Scoping**: Create new DataLoaders per request
5. **Error Handling**: Use `Arc<Error>` for cloneable errors

---

## Subscription System

### WebSocket Transport

Use GraphQL over WebSocket (graphql-ws protocol):

```rust
use async_graphql::http::{WebSocketProtocols, GraphQLWebSocket};
use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
};

pub async fn graphql_ws_handler(
    ws: WebSocketUpgrade,
    protocol: WebSocketProtocols,
) -> Response {
    ws.protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |socket| {
            GraphQLWebSocket::new(socket, schema.clone(), protocol)
                .on_connection_init(on_connection_init)
                .serve()
        })
}

async fn on_connection_init(
    value: serde_json::Value,
) -> async_graphql::Result<Data> {
    // Authenticate WebSocket connection
    let token = extract_token_from_payload(&value)?;
    let user = verify_token(&token).await?;

    let mut data = Data::default();
    data.insert(user);
    Ok(data)
}
```

### PubSub Implementation

```rust
use tokio::sync::broadcast;
use std::collections::HashMap;

pub struct PubSub {
    /// Incident update channel
    incident_updates: broadcast::Sender<IncidentUpdatedEvent>,

    /// Incident creation channel
    incident_created: broadcast::Sender<Incident>,

    /// State change channel
    state_changes: broadcast::Sender<IncidentStateChangedEvent>,

    /// Escalation channel
    escalations: broadcast::Sender<EscalationEvent>,
}

impl PubSub {
    pub fn new() -> Self {
        let (incident_updates, _) = broadcast::channel(1000);
        let (incident_created, _) = broadcast::channel(1000);
        let (state_changes, _) = broadcast::channel(1000);
        let (escalations, _) = broadcast::channel(1000);

        Self {
            incident_updates,
            incident_created,
            state_changes,
            escalations,
        }
    }

    pub async fn publish_incident_created(
        &self,
        incident: Incident,
    ) -> Result<()> {
        self.incident_created.send(incident)?;
        Ok(())
    }

    pub async fn publish_incident_updated(
        &self,
        event: IncidentUpdatedEvent,
    ) -> Result<()> {
        self.incident_updates.send(event)?;
        Ok(())
    }

    pub fn subscribe_incident_created(&self) -> broadcast::Receiver<Incident> {
        self.incident_created.subscribe()
    }

    pub fn subscribe_incident_updates(&self) -> broadcast::Receiver<IncidentUpdatedEvent> {
        self.incident_updates.subscribe()
    }
}
```

### Subscription Resolvers

```rust
use async_graphql::{Subscription, Context};
use futures_util::Stream;

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to incident updates
    async fn incident_updated<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        id: Option<ID>,
        filter: Option<IncidentFilter>,
    ) -> impl Stream<Item = IncidentUpdatedEvent> + 'ctx {
        let gql_ctx = ctx.data::<GraphQLContext>().unwrap();
        let mut rx = gql_ctx.pubsub.subscribe_incident_updates();

        async_stream::stream! {
            while let Ok(event) = rx.recv().await {
                // Apply filtering
                if should_emit_event(&event, &id, &filter) {
                    yield event;
                }
            }
        }
    }

    /// Subscribe to new incidents
    async fn incident_created<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        filter: Option<IncidentFilter>,
    ) -> impl Stream<Item = Incident> + 'ctx {
        let gql_ctx = ctx.data::<GraphQLContext>().unwrap();
        let mut rx = gql_ctx.pubsub.subscribe_incident_created();

        async_stream::stream! {
            while let Ok(incident) = rx.recv().await {
                // Apply filtering
                if matches_filter(&incident, &filter) {
                    yield incident;
                }
            }
        }
    }

    /// Subscribe to incident state changes
    async fn incident_state_changed<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        id: Option<ID>,
        states: Option<Vec<IncidentState>>,
    ) -> impl Stream<Item = IncidentStateChangedEvent> + 'ctx {
        let gql_ctx = ctx.data::<GraphQLContext>().unwrap();
        let mut rx = gql_ctx.pubsub.subscribe_state_changes();

        async_stream::stream! {
            while let Ok(event) = rx.recv().await {
                // Filter by incident ID and states
                if let Some(ref incident_id) = id {
                    if event.incident.id.to_string() != *incident_id {
                        continue;
                    }
                }

                if let Some(ref state_filter) = states {
                    if !state_filter.contains(&event.new_state) {
                        continue;
                    }
                }

                yield event;
            }
        }
    }
}

fn should_emit_event(
    event: &IncidentUpdatedEvent,
    id: &Option<ID>,
    filter: &Option<IncidentFilter>,
) -> bool {
    // Check ID filter
    if let Some(ref incident_id) = id {
        if event.incident.id.to_string() != *incident_id {
            return false;
        }
    }

    // Apply incident filter
    if let Some(ref filter) = filter {
        return matches_filter(&event.incident, filter);
    }

    true
}
```

### Subscription Patterns

1. **Filtered Subscriptions**: Apply filters server-side to reduce bandwidth
2. **Authentication**: Verify auth on connection init
3. **Authorization**: Check permissions before emitting events
4. **Heartbeat**: Implement ping/pong for connection health
5. **Backpressure**: Use bounded channels to prevent memory issues
6. **Reconnection**: Client-side automatic reconnection with exponential backoff

---

## Performance Optimization

### 1. Query Complexity Analysis

Prevent expensive queries:

```rust
use async_graphql::*;

fn build_schema() -> Schema<QueryRoot, MutationRoot, SubscriptionRoot> {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .limit_depth(10)           // Max nesting depth
        .limit_complexity(1000)     // Max query complexity
        .enable_federation()
        .finish()
}

// Add complexity to fields
#[Object]
impl Incident {
    #[graphql(complexity = 10)]
    async fn related_incidents(&self) -> Vec<Incident> {
        // Complex query
    }

    #[graphql(complexity = "pagination.page_size * 5")]
    async fn comments(
        &self,
        pagination: PaginationInput,
    ) -> CommentConnection {
        // Complexity based on page size
    }
}
```

### 2. Caching Strategy

```rust
use async_graphql::extensions::CacheControl;

#[Object]
impl Query {
    #[graphql(cache_control(max_age = 60))]
    async fn health(&self) -> HealthStatus {
        // Cache for 60 seconds
    }

    #[graphql(cache_control(max_age = 300, scope = CacheControlScope::Private))]
    async fn me(&self) -> User {
        // Cache user data for 5 minutes (private)
    }
}
```

### 3. Field-Level Performance

```rust
// Use field resolvers only when needed
#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Incident {
    // Direct fields (no resolver)
    pub id: ID,
    pub title: String,

    // Complex fields (with resolver)
    #[graphql(skip)]
    _assignee: Option<User>,
}

#[ComplexObject]
impl Incident {
    // Only resolve when requested
    async fn assignee(&self, ctx: &Context<'_>) -> Option<User> {
        // Use DataLoader
    }
}
```

### 4. Pagination Best Practices

- **Limit Page Size**: Max 100 items per page
- **Cursor-Based**: Use cursors for large datasets
- **Total Count**: Make `totalCount` optional (expensive query)
- **Connection Pattern**: Standard pagination interface

### 5. Batching & Debouncing

```rust
DataLoader::new(loader, tokio::spawn)
    .max_batch_size(100)
    .delay(Duration::from_millis(10))
```

### 6. Monitoring

```rust
use async_graphql::extensions::{Tracing, Logger};

Schema::build(...)
    .extension(Tracing)           // OpenTelemetry tracing
    .extension(Logger)            // Query logging
    .extension(Analyzer)          // Performance analysis
    .finish()
```

---

## Security Considerations

### 1. Authentication

```rust
pub async fn extract_user(ctx: &Context<'_>) -> Result<User> {
    let gql_ctx = ctx.data::<GraphQLContext>()?;
    gql_ctx.require_authenticated()
}
```

### 2. Authorization Guards

```rust
use async_graphql::Guard;

pub struct RoleGuard {
    role: Role,
}

#[async_trait::async_trait]
impl Guard for RoleGuard {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;
        gql_ctx.require_role(&self.role)?;
        Ok(())
    }
}

// Usage
#[Object]
impl Mutation {
    #[graphql(guard = "RoleGuard::new(Role::Admin)")]
    async fn delete_incident(&self, id: ID) -> Result<bool> {
        // Only admins can delete
    }
}
```

### 3. Field-Level Authorization

```rust
#[ComplexObject]
impl Incident {
    async fn sensitive_data(&self, ctx: &Context<'_>) -> Result<String> {
        let user = extract_user(ctx).await?;

        if !user.can_view_sensitive_data(&self) {
            return Err("Unauthorized".into());
        }

        Ok(self.sensitive_data.clone())
    }
}
```

### 4. Rate Limiting

```rust
use async_graphql::extensions::RateLimiter;

#[Object]
impl Mutation {
    #[graphql(rate_limit(limit = 10, duration = 60))]
    async fn create_incident(&self, input: CreateIncidentInput) -> Result<Incident> {
        // Limited to 10 requests per 60 seconds
    }
}
```

### 5. Query Depth & Complexity

```rust
Schema::build(...)
    .limit_depth(10)
    .limit_complexity(1000)
    .finish()
```

### 6. Input Validation

```rust
use validator::Validate;

#[derive(InputObject, Validate)]
pub struct CreateIncidentInput {
    #[validate(length(min = 1, max = 500))]
    pub title: String,

    #[validate(email)]
    pub assigned_to: Option<String>,
}

async fn create_incident(input: CreateIncidentInput) -> Result<Incident> {
    input.validate()?;
    // ...
}
```

### 7. CSRF Protection

Use CSRF tokens for mutations from web clients:

```rust
pub async fn verify_csrf_token(ctx: &Context<'_>) -> Result<()> {
    let token = ctx.data::<CsrfToken>()?;
    token.verify()?;
    Ok(())
}
```

---

## Error Handling

### Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphQLError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Database(#[from] DatabaseError),
}

impl From<GraphQLError> for async_graphql::Error {
    fn from(err: GraphQLError) -> Self {
        let message = err.to_string();
        let code = match &err {
            GraphQLError::NotFound(_) => "NOT_FOUND",
            GraphQLError::Unauthorized => "UNAUTHORIZED",
            GraphQLError::Forbidden(_) => "FORBIDDEN",
            GraphQLError::ValidationError(_) => "VALIDATION_ERROR",
            _ => "INTERNAL_ERROR",
        };

        async_graphql::Error::new(message)
            .extend_with(|_, e| {
                e.set("code", code);
            })
    }
}
```

### Error Extensions

```graphql
{
  "errors": [
    {
      "message": "Not found: Incident with ID xyz",
      "locations": [{"line": 2, "column": 3}],
      "path": ["incident"],
      "extensions": {
        "code": "NOT_FOUND",
        "timestamp": "2024-01-15T10:30:00Z",
        "requestId": "req-123"
      }
    }
  ]
}
```

### Result Pattern

```rust
pub async fn get_incident(id: ID) -> Result<Incident, GraphQLError> {
    let incident = store.get_incident(&id).await?;

    incident.ok_or_else(|| GraphQLError::NotFound(
        format!("Incident with ID {}", id)
    ))
}
```

---

## Integration with REST API

### Dual API Strategy

Maintain both REST and GraphQL APIs:

```
/api/v1/            → REST API (existing)
/graphql            → GraphQL API (new)
/graphql/playground → GraphQL Playground (dev only)
/graphql/ws         → GraphQL WebSocket subscriptions
```

### Shared Business Logic

```rust
// Service layer (shared)
pub struct IncidentService {
    store: Arc<dyn IncidentStore>,
}

impl IncidentService {
    pub async fn create_incident(&self, input: CreateIncidentInput) -> Result<Incident> {
        // Business logic
    }
}

// REST handler
pub async fn rest_create_incident(
    State(state): State<AppState>,
    Json(input): Json<RestCreateIncidentRequest>,
) -> Result<Json<Incident>> {
    let incident = state.incident_service
        .create_incident(input.into())
        .await?;
    Ok(Json(incident))
}

// GraphQL resolver
pub async fn graphql_create_incident(
    ctx: &Context<'_>,
    input: GraphQLCreateIncidentInput,
) -> Result<Incident> {
    let gql_ctx = ctx.data::<GraphQLContext>()?;
    let incident = gql_ctx.services.incident_service
        .create_incident(input.into())
        .await?;
    Ok(incident)
}
```

### Migration Strategy

1. **Phase 1**: Deploy GraphQL alongside REST
2. **Phase 2**: Update clients to use GraphQL
3. **Phase 3**: Deprecate REST endpoints (with warning period)
4. **Phase 4**: Remove REST endpoints

### Feature Parity

Ensure GraphQL provides:
- All REST functionality
- Better query flexibility
- Real-time updates (subscriptions)
- Reduced over-fetching
- Type safety

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

- [ ] Add async-graphql dependencies
- [ ] Create GraphQL module structure
- [ ] Define core types (Incident, Event, Alert)
- [ ] Implement basic Query resolvers
- [ ] Set up Axum integration
- [ ] Add GraphQL Playground

### Phase 2: Core Functionality (Week 3-4)

- [ ] Implement all Query resolvers
- [ ] Implement Mutation resolvers
- [ ] Add input validation
- [ ] Create DataLoaders
- [ ] Implement pagination
- [ ] Add filtering and sorting

### Phase 3: Advanced Features (Week 5-6)

- [ ] Implement subscriptions
- [ ] Add PubSub system
- [ ] WebSocket support
- [ ] Query complexity analysis
- [ ] Rate limiting
- [ ] Caching strategy

### Phase 4: Security & Auth (Week 7-8)

- [ ] Authentication middleware
- [ ] Authorization guards
- [ ] Field-level permissions
- [ ] CSRF protection
- [ ] Audit logging

### Phase 5: Analytics & Reporting (Week 9-10)

- [ ] Analytics resolvers
- [ ] Metrics aggregation
- [ ] Trend analysis
- [ ] SLA reporting
- [ ] Custom dashboards

### Phase 6: Testing & Documentation (Week 11-12)

- [ ] Unit tests for resolvers
- [ ] Integration tests
- [ ] Load testing
- [ ] API documentation
- [ ] Client examples
- [ ] Migration guide

### Phase 7: Production Readiness (Week 13-14)

- [ ] Performance optimization
- [ ] Monitoring & alerting
- [ ] Error tracking
- [ ] Load balancing
- [ ] Deployment automation
- [ ] Rollout plan

---

## Appendix

### Example Queries

#### Get Incident with Related Data

```graphql
query GetIncident($id: UUID!) {
  incident(id: $id) {
    id
    title
    description
    severity
    state
    createdAt
    assignedTo {
      id
      name
      email
    }
    assignedTeam {
      id
      name
      members {
        name
      }
    }
    relatedIncidents {
      id
      title
      severity
    }
    timeline {
      timestamp
      eventType
      description
      actor {
        name
      }
    }
    comments(pagination: { pageSize: 10 }) {
      edges {
        node {
          content
          author {
            name
          }
          createdAt
        }
      }
      pageInfo {
        hasNextPage
      }
    }
  }
}
```

#### List Incidents with Filtering

```graphql
query ListIncidents(
  $filter: IncidentFilter
  $pagination: PaginationInput
  $sort: SortInput
) {
  incidents(
    filter: $filter
    pagination: $pagination
    sort: $sort
  ) {
    edges {
      node {
        id
        title
        severity
        state
        createdAt
        assignedTo {
          name
        }
      }
      cursor
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

Variables:
```json
{
  "filter": {
    "severities": ["P0", "P1"],
    "states": ["NEW", "ACKNOWLEDGED"],
    "assignedToMe": true
  },
  "pagination": {
    "pageSize": 20,
    "page": 1
  },
  "sort": {
    "field": "CREATED_AT",
    "order": "DESC"
  }
}
```

#### Create Incident

```graphql
mutation CreateIncident($input: CreateIncidentInput!) {
  createIncident(input: $input) {
    incident {
      id
      title
      severity
      state
      createdAt
    }
  }
}
```

Variables:
```json
{
  "input": {
    "title": "High CPU Usage on prod-api-01",
    "description": "CPU usage exceeded 90% threshold",
    "severity": "P1",
    "category": "PERFORMANCE",
    "environment": "PRODUCTION",
    "tags": {
      "host": "prod-api-01",
      "region": "us-east-1"
    }
  }
}
```

#### Subscribe to Incident Updates

```graphql
subscription IncidentUpdates($filter: IncidentFilter) {
  incidentUpdated(filter: $filter) {
    incident {
      id
      title
      state
      severity
      updatedAt
    }
    updatedFields
    actor {
      type
      name
    }
  }
}
```

#### Get Analytics

```graphql
query GetAnalytics(
  $startDate: DateTime!
  $endDate: DateTime!
) {
  incidentMetrics(
    startDate: $startDate
    endDate: $endDate
    groupBy: [SEVERITY, CATEGORY]
  ) {
    totalIncidents
    incidentsBySeverity {
      key
      value
      percentage
    }
    incidentsByCategory {
      key
      value
      percentage
    }
    averageMTTA
    averageMTTR
    slaCompliance
  }

  incidentTrends(
    startDate: $startDate
    endDate: $endDate
    interval: DAY
    metric: INCIDENT_COUNT
  ) {
    timestamp
    value
  }
}
```

### Performance Benchmarks

Target metrics:
- **Query Latency**: p50 < 50ms, p95 < 200ms, p99 < 500ms
- **Mutation Latency**: p50 < 100ms, p95 < 300ms, p99 < 1s
- **Subscription Latency**: < 100ms from event to client
- **Throughput**: > 1000 queries/second
- **DataLoader Hit Rate**: > 95%
- **Memory Usage**: < 100MB per 1000 concurrent connections

### References

- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)
- [Relay Cursor Connections Specification](https://relay.dev/graphql/connections.htm)
- [GraphQL over WebSocket Protocol](https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md)
