# WebSocket Streaming Guide

## Overview

The LLM Incident Manager provides real-time incident updates via WebSocket streaming through a GraphQL subscription API. This enables clients to receive immediate notifications when incidents are created, updated, or resolved without polling the REST API.

### Key Features

- **Real-Time Updates**: Instant notifications for incident lifecycle events
- **GraphQL Subscriptions**: Type-safe, schema-driven subscription API
- **Filtering Capabilities**: Subscribe to specific incidents, severities, or event types
- **Scalable Architecture**: Built on Axum with async Rust for high performance
- **Auto-Reconnection**: WebSocket protocol supports automatic reconnection with state recovery
- **Authentication Ready**: Extensible context system for auth integration
- **DataLoader Integration**: Efficient batch loading to prevent N+1 queries

## Architecture

### Technology Stack

The WebSocket streaming implementation uses:
- **async-graphql**: GraphQL server with subscription support
- **async-graphql-axum**: Axum integration for WebSocket handling
- **Axum WebSocket**: High-performance WebSocket implementation
- **tokio**: Async runtime for concurrent connections
- **futures**: Stream processing and combinators

### System Architecture

```
┌─────────────────┐
│   Client        │
│  (Browser/App)  │
└────────┬────────┘
         │ ws://host/graphql/ws
         │ (WebSocket Upgrade)
         ↓
┌─────────────────────────────────────┐
│         Axum HTTP Server            │
│  ┌──────────────────────────────┐  │
│  │  GraphQL Subscription Handler │  │
│  │  (graphql_subscription_handler)│  │
│  └──────────┬───────────────────┘  │
└─────────────┼───────────────────────┘
              │
              ↓
┌─────────────────────────────────────┐
│      async-graphql Schema           │
│  ┌──────────────────────────────┐  │
│  │  SubscriptionRoot             │  │
│  │  - incident_updates()         │  │
│  │  - new_incidents()            │  │
│  │  - critical_incidents()       │  │
│  │  - incident_state_changes()   │  │
│  │  - alerts()                   │  │
│  └──────────┬───────────────────┘  │
└─────────────┼───────────────────────┘
              │
              ↓
┌─────────────────────────────────────┐
│      GraphQLContext                 │
│  - IncidentProcessor                │
│  - DataLoaders                      │
│  - Authentication (Optional)        │
└─────────────────────────────────────┘
              │
              ↓
┌─────────────────────────────────────┐
│    IncidentProcessor & Store        │
│  - State Management                 │
│  - Event Publishing (Future)        │
└─────────────────────────────────────┘
```

### Connection Flow

```
Client                          Server
  │                               │
  │──── HTTP Upgrade Request ────→│
  │     (WebSocket)               │
  │                               │
  │←──── 101 Switching Protocols ─│
  │                               │
  │──── connection_init ─────────→│
  │                               │
  │←──── connection_ack ──────────│
  │                               │
  │──── subscribe ───────────────→│
  │     (GraphQL subscription)    │
  │                               │
  │←──── next ────────────────────│
  │     (subscription data)       │
  │                               │
  │←──── next ────────────────────│
  │     (more data)               │
  │                               │
  │──── complete ────────────────→│
  │     (unsubscribe)             │
  │                               │
  │──── connection_terminate ────→│
  │                               │
  └───────────────────────────────┘
```

## Connection Establishment

### WebSocket Endpoint

```
ws://hostname:port/graphql/ws
```

For TLS/SSL:
```
wss://hostname:port/graphql/ws
```

### Connection Parameters

The WebSocket connection uses the GraphQL WebSocket protocol (graphql-ws):

**Initial Connection Message:**
```json
{
  "type": "connection_init",
  "payload": {
    "Authorization": "Bearer YOUR_TOKEN"
  }
}
```

**Server Acknowledgment:**
```json
{
  "type": "connection_ack"
}
```

## Authentication and Authorization

### Current Implementation

The current implementation provides a GraphQLContext with optional user authentication:

```rust
pub struct GraphQLContext {
    pub processor: Arc<IncidentProcessor>,
    pub incident_loader: DataLoader<IncidentLoader>,
    pub playbook_loader: DataLoader<PlaybookLoader>,
    pub related_incidents_loader: DataLoader<RelatedIncidentsLoader>,
    pub user: Option<String>,
}
```

### Future Authentication Integration

To add authentication, extend the `graphql_subscription_handler`:

```rust
async fn graphql_subscription_handler(
    schema: Extension<GraphQLSchema>,
    processor: Extension<Arc<IncidentProcessor>>,
    ws: axum::extract::ws::WebSocketUpgrade,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Extract auth token from headers or connection params
    let user = extract_user_from_headers(&headers);

    let ctx = GraphQLContext::new(processor.0.clone())
        .with_user(user);

    ws.on_upgrade(move |socket| {
        GraphQLSubscription::new(socket)
            .data(ctx)
            .serve(schema.0)
    })
}
```

### Authorization Patterns

**Per-Subscription Authorization:**
```rust
async fn incident_updates(
    &self,
    ctx: &Context<'_>,
) -> Result<impl Stream<Item = IncidentUpdate>> {
    let gql_ctx = ctx.data::<GraphQLContext>()?;

    // Check if user has permission
    if !has_permission(&gql_ctx.user, "incidents.subscribe") {
        return Err(Error::new("Unauthorized"));
    }

    // Return filtered stream based on user permissions
    Ok(stream)
}
```

## Message Protocol Reference

### GraphQL-WS Protocol

The implementation uses the `graphql-ws` protocol (not the older `subscriptions-transport-ws`).

#### Client → Server Messages

**1. connection_init**
```json
{
  "type": "connection_init",
  "payload": {
    "Authorization": "Bearer token"
  }
}
```

**2. subscribe**
```json
{
  "id": "1",
  "type": "subscribe",
  "payload": {
    "query": "subscription { incidentUpdates { updateType incidentId timestamp } }"
  }
}
```

**3. complete**
```json
{
  "id": "1",
  "type": "complete"
}
```

**4. ping**
```json
{
  "type": "ping"
}
```

#### Server → Client Messages

**1. connection_ack**
```json
{
  "type": "connection_ack"
}
```

**2. next** (subscription data)
```json
{
  "id": "1",
  "type": "next",
  "payload": {
    "data": {
      "incidentUpdates": {
        "updateType": "CREATED",
        "incidentId": "550e8400-e29b-41d4-a716-446655440000",
        "timestamp": "2025-11-12T20:30:45Z"
      }
    }
  }
}
```

**3. error**
```json
{
  "id": "1",
  "type": "error",
  "payload": [
    {
      "message": "Unauthorized",
      "extensions": {
        "code": "UNAUTHORIZED"
      }
    }
  ]
}
```

**4. complete**
```json
{
  "id": "1",
  "type": "complete"
}
```

**5. pong**
```json
{
  "type": "pong"
}
```

## Event Types and Schemas

### Available Subscriptions

#### 1. incidentUpdates

Subscribe to all incident update events.

**Subscription:**
```graphql
subscription IncidentUpdates(
  $incidentIds: [UUID!]
  $severities: [Severity!]
  $activeOnly: Boolean
) {
  incidentUpdates(
    incidentIds: $incidentIds
    severities: $severities
    activeOnly: $activeOnly
  ) {
    updateType
    incidentId
    timestamp
  }
}
```

**Response Type:**
```graphql
type IncidentUpdate {
  updateType: IncidentUpdateType!
  incidentId: UUID
  timestamp: DateTime!
}

enum IncidentUpdateType {
  CREATED
  UPDATED
  STATE_CHANGED
  RESOLVED
  ASSIGNED
  COMMENT_ADDED
  HEARTBEAT
}
```

**Example Response:**
```json
{
  "data": {
    "incidentUpdates": {
      "updateType": "CREATED",
      "incidentId": "550e8400-e29b-41d4-a716-446655440000",
      "timestamp": "2025-11-12T20:30:45.123456Z"
    }
  }
}
```

#### 2. newIncidents

Subscribe to newly created incidents.

**Subscription:**
```graphql
subscription NewIncidents($severities: [Severity!]) {
  newIncidents(severities: $severities) {
    id
    title
    description
    severity
    state
    createdAt
  }
}
```

**Response Type:**
```graphql
type Incident {
  id: UUID!
  title: String!
  description: String!
  severity: Severity!
  state: IncidentState!
  incidentType: IncidentType!
  source: String!
  createdAt: DateTime!
  updatedAt: DateTime!
  resolvedAt: DateTime
  assignedTo: String
  affectedResources: [String!]!
  labels: Map!
  metadata: Map!
}
```

#### 3. criticalIncidents

Subscribe to critical (P0/P1) incidents only.

**Subscription:**
```graphql
subscription CriticalIncidents {
  criticalIncidents {
    id
    title
    severity
    state
    createdAt
    affectedResources
  }
}
```

#### 4. incidentStateChanges

Subscribe to incident state transitions.

**Subscription:**
```graphql
subscription IncidentStateChanges($incidentId: UUID) {
  incidentStateChanges(incidentId: $incidentId) {
    incidentId
    oldState
    newState
    changedBy
    timestamp
  }
}
```

**Response Type:**
```graphql
type IncidentStateChange {
  incidentId: UUID!
  oldState: IncidentState!
  newState: IncidentState!
  changedBy: String!
  timestamp: DateTime!
}
```

#### 5. alerts

Subscribe to incoming alert submissions.

**Subscription:**
```graphql
subscription Alerts($sources: [String!]) {
  alerts(sources: $sources) {
    id
    externalId
    source
    title
    severity
    alertType
    receivedAt
  }
}
```

## Subscription Model

### Filter-Based Subscriptions

All subscriptions support filtering to reduce bandwidth and processing:

**By Incident IDs:**
```graphql
subscription {
  incidentUpdates(
    incidentIds: [
      "550e8400-e29b-41d4-a716-446655440000",
      "660e8400-e29b-41d4-a716-446655440001"
    ]
  ) {
    updateType
    incidentId
  }
}
```

**By Severity:**
```graphql
subscription {
  newIncidents(severities: [P0, P1]) {
    id
    title
    severity
  }
}
```

**Active Only:**
```graphql
subscription {
  incidentUpdates(activeOnly: true) {
    updateType
    incidentId
  }
}
```

### Multiple Subscriptions

A single WebSocket connection can handle multiple concurrent subscriptions:

```javascript
const subscription1 = client.subscribe({
  query: `subscription { criticalIncidents { id title } }`
});

const subscription2 = client.subscribe({
  query: `subscription { alerts { id source } }`
});
```

Each subscription gets a unique ID for lifecycle management.

## Error Handling

### Error Types

**1. Connection Errors**
- Connection refused
- Connection timeout
- Invalid protocol
- TLS/SSL errors

**2. Authentication Errors**
- Invalid token
- Expired token
- Insufficient permissions

**3. Subscription Errors**
- Invalid query syntax
- Unknown subscription field
- Type validation errors
- Missing required arguments

**4. Runtime Errors**
- Internal server error
- Database unavailable
- Rate limit exceeded

### Error Response Format

```json
{
  "id": "1",
  "type": "error",
  "payload": [
    {
      "message": "Unauthorized: Invalid authentication token",
      "locations": [{"line": 2, "column": 3}],
      "path": ["incidentUpdates"],
      "extensions": {
        "code": "UNAUTHORIZED",
        "timestamp": "2025-11-12T20:30:45Z"
      }
    }
  ]
}
```

### Error Handling Strategies

**Client-Side Error Handling:**
```javascript
subscription.subscribe({
  next: (data) => console.log('Received:', data),
  error: (error) => {
    console.error('Subscription error:', error);

    if (error.extensions?.code === 'UNAUTHORIZED') {
      // Refresh auth token and reconnect
      refreshTokenAndReconnect();
    } else if (error.extensions?.code === 'RATE_LIMIT_EXCEEDED') {
      // Back off and retry
      setTimeout(() => reconnect(), 5000);
    }
  },
  complete: () => console.log('Subscription completed')
});
```

## Best Practices

### Connection Management

**1. Implement Exponential Backoff**
```javascript
let retryDelay = 1000;
const maxRetryDelay = 30000;

function reconnect() {
  setTimeout(() => {
    createConnection().catch((err) => {
      retryDelay = Math.min(retryDelay * 2, maxRetryDelay);
      reconnect();
    });
  }, retryDelay);
}
```

**2. Handle Connection Lifecycle**
```javascript
websocket.onopen = () => {
  console.log('Connected');
  retryDelay = 1000; // Reset on successful connection
};

websocket.onclose = (event) => {
  if (event.code === 1000) {
    // Normal closure
    console.log('Connection closed normally');
  } else {
    // Abnormal closure - reconnect
    reconnect();
  }
};
```

**3. Implement Heartbeat/Ping**
```javascript
setInterval(() => {
  websocket.send(JSON.stringify({ type: 'ping' }));
}, 30000); // Every 30 seconds
```

### Performance Optimization

**1. Use Specific Filters**
```graphql
# Good - specific filter
subscription {
  incidentUpdates(
    incidentIds: ["550e8400-e29b-41d4-a716-446655440000"]
    activeOnly: true
  ) {
    updateType
    incidentId
  }
}

# Bad - unfiltered subscription
subscription {
  incidentUpdates {
    updateType
    incidentId
  }
}
```

**2. Request Only Needed Fields**
```graphql
# Good - minimal fields
subscription {
  newIncidents {
    id
    title
    severity
  }
}

# Bad - requesting all fields
subscription {
  newIncidents {
    id
    title
    description
    severity
    state
    incidentType
    # ... many more fields
  }
}
```

**3. Batch Updates**
```javascript
let updateBuffer = [];
let flushTimeout = null;

function handleUpdate(update) {
  updateBuffer.push(update);

  if (!flushTimeout) {
    flushTimeout = setTimeout(() => {
      processUpdates(updateBuffer);
      updateBuffer = [];
      flushTimeout = null;
    }, 100); // Flush every 100ms
  }
}
```

### Security Best Practices

**1. Always Use WSS in Production**
```javascript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const wsUrl = `${protocol}//${window.location.host}/graphql/ws`;
```

**2. Validate Authentication Tokens**
```javascript
const token = localStorage.getItem('auth_token');
if (!token || isTokenExpired(token)) {
  await refreshToken();
}

const client = new GraphQLWSClient({
  url: wsUrl,
  connectionParams: {
    Authorization: `Bearer ${token}`
  }
});
```

**3. Implement Rate Limiting Client-Side**
```javascript
const RateLimiter = {
  subscriptions: new Set(),
  maxSubscriptions: 10,

  canSubscribe() {
    return this.subscriptions.size < this.maxSubscriptions;
  },

  addSubscription(id) {
    if (!this.canSubscribe()) {
      throw new Error('Maximum subscriptions exceeded');
    }
    this.subscriptions.add(id);
  },

  removeSubscription(id) {
    this.subscriptions.delete(id);
  }
};
```

### Monitoring and Observability

**1. Track Connection Metrics**
```javascript
const metrics = {
  connectTime: 0,
  reconnectCount: 0,
  messagesReceived: 0,
  errors: 0
};

websocket.onopen = () => {
  metrics.connectTime = Date.now();
  sendMetric('websocket.connected', metrics);
};

websocket.onmessage = () => {
  metrics.messagesReceived++;
};

websocket.onerror = () => {
  metrics.errors++;
  sendMetric('websocket.error', metrics);
};
```

**2. Log Important Events**
```javascript
function logSubscriptionEvent(event, data) {
  console.log({
    timestamp: new Date().toISOString(),
    event,
    data,
    connectionState: websocket.readyState
  });
}
```

## Production Deployment Considerations

### Current Implementation Note

The current subscription implementation includes placeholder streams for demonstration. In production, you should:

1. **Implement Event Publishing**: Use a message broker (Redis Pub/Sub, NATS, Kafka)
2. **State Synchronization**: Ensure events are published when incidents change
3. **Scalability**: Distribute subscriptions across multiple server instances
4. **Persistence**: Handle subscription recovery after server restart

### Example Production Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client 1  │────→│  Server 1   │     │  Server 2   │
└─────────────┘     └──────┬──────┘     └──────┬──────┘
                           │                    │
┌─────────────┐           │   ┌─────────────┐  │
│   Client 2  │──────────→│───│ Redis       │──│
└─────────────┘           │   │ Pub/Sub     │  │
                          │   └─────────────┘  │
┌─────────────┐           │                    │
│   Client 3  │───────────┴────────────────────┘
└─────────────┘

Incident Updates → Publish to Redis → All Servers Subscribe → Stream to Clients
```

### Scaling Considerations

**Horizontal Scaling:**
- Use sticky sessions or connection state sharing
- Redis Pub/Sub for event distribution across instances
- Load balancer with WebSocket support (e.g., HAProxy, NGINX)

**Vertical Scaling:**
- Tokio async runtime handles thousands of concurrent connections
- Tune Tokio worker threads based on CPU cores
- Monitor memory usage per connection

**Rate Limiting:**
```rust
// Future enhancement
struct SubscriptionRateLimiter {
    max_subscriptions_per_client: usize,
    max_events_per_second: usize,
}
```

## Next Steps

1. Review [WebSocket API Reference](./WEBSOCKET_API_REFERENCE.md) for detailed API documentation
2. Check [WebSocket Client Guide](./WEBSOCKET_CLIENT_GUIDE.md) for integration examples
3. Read [WebSocket Deployment Guide](./WEBSOCKET_DEPLOYMENT_GUIDE.md) for production setup
4. Explore [GraphQL Schema Reference](./GRAPHQL_SCHEMA_REFERENCE.md) for complete type definitions

## Related Documentation

- [GraphQL Guide](./GRAPHQL_GUIDE.md) - Overall GraphQL API documentation
- [GraphQL Architecture](./GRAPHQL_ARCHITECTURE.md) - System architecture details
- [API Specification](./api-specification.yaml) - OpenAPI REST API spec
- [Integration Guide](./integration-guide.md) - Integration patterns and examples
