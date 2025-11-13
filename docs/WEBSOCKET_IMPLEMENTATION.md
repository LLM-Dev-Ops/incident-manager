# WebSocket Implementation Details

## Overview

This document provides detailed technical information about the WebSocket streaming implementation in the LLM Incident Manager.

## Architecture

### Module Structure

```
src/websocket/
├── mod.rs              # Module exports and configuration
├── messages.rs         # Message protocol definitions
├── events.rs           # Event types and envelopes
├── session.rs          # Session management
├── connection.rs       # Connection management
├── broadcaster.rs      # Event broadcasting
├── server.rs           # WebSocket server implementation
├── handlers.rs         # Event handler integration
└── metrics.rs          # Prometheus metrics
```

### Component Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    WebSocket Server                      │
│                      (/ws endpoint)                      │
└───────────────┬─────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────────────┐
│              Connection Manager                          │
│  ┌──────────┬──────────┬──────────┬──────────┐         │
│  │ Session 1│ Session 2│ Session 3│ Session N│         │
│  └──────────┴──────────┴──────────┴──────────┘         │
└───────────────┬─────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────────────┐
│              Event Broadcaster                           │
│         (tokio::sync::broadcast channel)                 │
└───────────────┬─────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────────────┐
│              Event Handlers                              │
│  ├─ IncidentEventHandler                                │
│  ├─ AlertEventHandler                                   │
│  ├─ EscalationEventHandler                              │
│  ├─ PlaybookEventHandler                                │
│  └─ NotificationEventHandler                            │
└─────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Message Protocol (messages.rs)

Defines the client-server communication protocol:

```rust
pub enum ClientMessage {
    Subscribe { subscription_id: String, filters: SubscriptionFilters },
    Unsubscribe { subscription_id: String },
    Ping { timestamp: DateTime<Utc> },
    Ack { message_id: String },
}

pub enum ServerMessage {
    Welcome { session_id: String, server_time: DateTime<Utc> },
    Subscribed { subscription_id: String, filters: SubscriptionFilters },
    Unsubscribed { subscription_id: String },
    Pong { timestamp: DateTime<Utc> },
    Event { message_id: String, event: Event, timestamp: DateTime<Utc> },
    Error { code: String, message: String },
    Closing { reason: String },
}
```

**Key Features:**
- JSON serialization via serde
- Tagged enum for type-safe messages
- Comprehensive error handling

### 2. Event Types (events.rs)

Type-safe event definitions:

```rust
pub enum Event {
    IncidentCreated { incident: Incident },
    IncidentUpdated { incident: Incident, previous_state: Option<IncidentState> },
    IncidentResolved { incident: Incident },
    AlertReceived { alert: Alert },
    Escalated { incident_id: Uuid, from_severity: Severity, to_severity: Severity, reason: String },
    PlaybookStarted { incident_id: Uuid, playbook_id: Uuid, playbook_name: String },
    // ... more event types
}
```

**Event Envelope:**
```rust
pub struct EventEnvelope {
    pub id: String,              // Unique event ID
    pub timestamp: DateTime<Utc>, // Event timestamp
    pub event: Event,            // Event payload
    pub priority: EventPriority, // Delivery priority
}
```

**Priority Levels:**
- `Critical` - P0 incidents, critical failures
- `High` - P1 incidents, escalations
- `Normal` - Regular incidents, updates
- `Low` - Comments, informational events

### 3. Session Management (session.rs)

Manages WebSocket session lifecycle:

```rust
pub struct Session {
    pub id: String,                              // Unique session ID
    pub created_at: DateTime<Utc>,               // Creation timestamp
    pub last_active: DateTime<Utc>,              // Last activity
    pub user_id: Option<String>,                 // Authenticated user (future)
    pub subscriptions: HashMap<String, Subscription>, // Active subscriptions
    pub metadata: HashMap<String, String>,       // Session metadata
    pub message_count: u64,                      // Message counter
}
```

**Session Features:**
- Automatic expiration after idle timeout
- Subscription tracking per session
- Activity monitoring
- Metadata support for extensibility

### 4. Connection Management (connection.rs)

Manages active WebSocket connections:

```rust
pub struct ConnectionManager {
    connections: Arc<DashMap<String, Arc<Connection>>>,
    stats: Arc<RwLock<ConnectionStats>>,
}

pub struct Connection {
    session: Arc<RwLock<Session>>,
    tx: mpsc::UnboundedSender<ServerMessage>,
    remote_addr: Option<String>,
}
```

**Key Features:**
- Thread-safe connection registry (DashMap)
- Per-connection message channels
- Connection statistics tracking
- Automatic cleanup of stale connections

**Connection Lifecycle:**
```
┌─────────┐    register    ┌──────────┐    broadcast    ┌─────────┐
│ Client  │───────────────>│ Manager  │<────────────────│ Event   │
└─────────┘                └──────────┘                 └─────────┘
     │                           │
     │      send_message         │
     │<──────────────────────────│
     │                           │
     │      disconnect           │
     │───────────────────────────>│
     │                           │
     │                      unregister
     │                           │
```

### 5. Event Broadcasting (broadcaster.rs)

Pub/sub event distribution system:

```rust
pub struct EventBroadcaster {
    tx: broadcast::Sender<EventEnvelope>,
    connections: Arc<ConnectionManager>,
    stats: Arc<RwLock<EventStats>>,
    capacity: usize,
}
```

**Broadcasting Flow:**
```
┌──────────────┐
│   Producer   │ (IncidentProcessor, etc.)
└──────┬───────┘
       │ publish()
       ▼
┌──────────────┐
│ Broadcaster  │
└──────┬───────┘
       │
       ├─────> broadcast::channel (for internal subscribers)
       │
       └─────> ConnectionManager.broadcast_event()
               │
               ▼
       ┌───────────────────────────────┐
       │  Filter & Route to Connections │
       └───────────────────────────────┘
               │
               ├──> Connection 1 (matches filters)
               ├──> Connection 2 (matches filters)
               └──> Connection N (matches filters)
```

**Filtering Logic:**
1. Check event type subscription
2. Apply severity filters
3. Apply state filters
4. Apply source filters
5. Apply resource filters
6. Apply label filters
7. Apply incident ID filters

### 6. WebSocket Server (server.rs)

Axum-based WebSocket endpoint:

```rust
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebSocketState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response
```

**Connection Handling:**
```
Client connects
    │
    ▼
WebSocketUpgrade
    │
    ▼
Split into sender/receiver
    │
    ├──> Sender task (outbound messages)
    │    └──> Reads from mpsc channel
    │         └──> Sends to WebSocket
    │
    └──> Receiver task (inbound messages)
         └──> Reads from WebSocket
              └──> Processes ClientMessage
```

**Concurrent Tasks per Connection:**
- **Receiver Task**: Processes incoming client messages
- **Sender Task**: Sends outbound messages from queue
- **Heartbeat Task**: Periodic ping/pong keepalive

### 7. Event Handlers (handlers.rs)

Integration points for system events:

```rust
pub struct EventHandlers {
    pub incidents: IncidentEventHandler,
    pub alerts: AlertEventHandler,
    pub escalations: EscalationEventHandler,
    pub playbooks: PlaybookEventHandler,
    pub notifications: NotificationEventHandler,
    pub assignments: AssignmentEventHandler,
    pub comments: CommentEventHandler,
    pub system: SystemEventHandler,
}
```

**Usage in IncidentProcessor:**
```rust
// In process_alert()
if let Some(ref ws_handlers) = self.websocket_handlers {
    ws_handlers.alerts.on_alert_received(alert.clone()).await;
    ws_handlers.incidents.on_incident_created(incident.clone()).await;
}
```

### 8. Metrics (metrics.rs)

Prometheus metrics for monitoring:

```rust
lazy_static! {
    pub static ref WS_ACTIVE_CONNECTIONS: IntGauge = ...;
    pub static ref WS_MESSAGES_SENT: IntCounter = ...;
    pub static ref WS_EVENTS_BROADCAST: IntCounter = ...;
    pub static ref WS_SESSION_DURATION: Histogram = ...;
    // ... more metrics
}
```

**Available Metrics:**
- Connection metrics (active, total, errors)
- Message metrics (sent, received, errors)
- Event metrics (broadcast, delivered)
- Performance metrics (latency, duration)
- Resource metrics (channel usage, subscriptions)

## Data Flow

### Alert Processing with WebSocket Events

```
┌──────────────┐
│  Alert API   │
│   Endpoint   │
└──────┬───────┘
       │
       ▼
┌──────────────────────┐
│ IncidentProcessor    │
│  .process_alert()    │
└──────┬───────────────┘
       │
       ├──> DeduplicationEngine
       │
       ├──> Convert to Incident
       │
       ├──> Save to Store
       │
       ├──> WebSocket Events ────┐
       │    ├─ alert_received    │
       │    ├─ alert_converted   │
       │    └─ incident_created  │
       │                         │
       ├──> NotificationService  │
       │                         │
       ├──> PlaybookService      │
       │                         │
       └──> EscalationEngine     │
                                 │
                                 ▼
                    ┌────────────────────┐
                    │  EventBroadcaster  │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ ConnectionManager  │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │  Filter & Deliver  │
                    │  to Subscriptions  │
                    └────────────────────┘
```

## Concurrency & Thread Safety

### Thread-Safe Components

1. **DashMap** for connection registry
   - Lock-free concurrent HashMap
   - No global locks for read operations

2. **RwLock** for shared state
   - Multiple readers, single writer
   - Used for sessions, stats

3. **Arc** for shared ownership
   - Reference-counted pointers
   - Thread-safe sharing

4. **Tokio channels** for message passing
   - `broadcast::channel` for events
   - `mpsc::unbounded_channel` for connection messages

### Backpressure Handling

```rust
const MAX_PENDING_MESSAGES: usize = 1000;

// If channel is full, drop connection or oldest messages
if tx.send(message).is_err() {
    // Connection closed or buffer full
    metrics::record_send_error();
    // Trigger cleanup
}
```

**Strategies:**
- Bounded message buffers per connection
- Slow consumer detection
- Automatic disconnect on overflow
- Priority-based message queuing

## Performance Optimizations

### 1. Lock-Free Data Structures

```rust
// DashMap provides lock-free reads
connections: Arc<DashMap<String, Arc<Connection>>>
```

### 2. Zero-Copy Message Passing

```rust
// Arc for shared event data
broadcast::channel<EventEnvelope>
```

### 3. Batch Processing

```rust
// Process multiple events in single broadcast
async fn broadcast_events(&self, events: Vec<Event>)
```

### 4. Efficient Filtering

```rust
// Early filtering by event type
if !session.interested_event_types().contains(&event_type) {
    continue; // Skip connection
}
```

### 5. Connection Pooling

- Reuse TCP connections
- Connection keepalive
- Efficient resource cleanup

## Error Handling

### Connection Errors

```rust
pub enum ConnectionError {
    SendFailed,    // Failed to send to connection
    NotFound,      // Connection not in registry
}
```

### Message Errors

- Invalid JSON → Error message to client
- Unknown message type → Error message
- Protocol violations → Close connection

### Event Errors

- Serialization errors → Log and skip
- Broadcast errors → Retry or drop
- Filter errors → Log and allow all

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_subscription_filters() { ... }

    #[tokio::test]
    async fn test_connection_send() { ... }

    #[test]
    fn test_event_priority() { ... }
}
```

### Integration Tests

- WebSocket connection lifecycle
- Message protocol compliance
- Event filtering accuracy
- Concurrent connection handling

### Load Tests

- 1000+ concurrent connections
- High-frequency event broadcasting
- Message throughput benchmarks
- Memory usage under load

## Security Considerations

### Current Implementation

- No authentication (trust network)
- No encryption (use reverse proxy)
- No rate limiting (planned)

### Future Enhancements

```rust
// JWT-based authentication
pub struct Session {
    pub user_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: HashSet<Permission>,
}

// Rate limiting
pub struct RateLimiter {
    max_connections_per_ip: usize,
    max_subscriptions_per_session: usize,
    max_messages_per_second: f64,
}
```

### Best Practices

1. Use WSS (WebSocket Secure) in production
2. Implement authentication tokens
3. Validate all client messages
4. Rate limit connections and subscriptions
5. Monitor for abuse patterns
6. Implement IP-based restrictions

## Configuration

### Default Configuration

```rust
WebSocketConfig {
    max_pending_messages: 1000,
    heartbeat_interval_secs: 30,
    session_timeout_secs: 300,
    cleanup_interval_secs: 60,
    broadcast_capacity: 10000,
    enable_compression: true,
}
```

### Environment Variables

```bash
# Future configuration options
WS_MAX_CONNECTIONS=10000
WS_HEARTBEAT_INTERVAL=30
WS_SESSION_TIMEOUT=300
WS_BROADCAST_CAPACITY=10000
```

## Troubleshooting

### High Memory Usage

**Symptoms:**
- Memory grows unbounded
- OOM errors

**Causes:**
- Too many pending messages
- Stale connections not cleaned up
- Event buffer overflow

**Solutions:**
```rust
// Reduce buffer sizes
max_pending_messages: 500

// More aggressive cleanup
cleanup_interval_secs: 30
session_timeout_secs: 180

// Lower broadcast capacity
broadcast_capacity: 5000
```

### Connection Drops

**Symptoms:**
- Clients frequently disconnect
- Connection timeouts

**Causes:**
- Network instability
- Heartbeat timeout
- Server overload

**Solutions:**
```rust
// Increase timeouts
heartbeat_interval_secs: 60
session_timeout_secs: 600

// Monitor metrics
WS_CONNECTION_ERRORS
WS_SESSION_DURATION
```

### Event Delivery Delays

**Symptoms:**
- High latency for events
- Events arrive out of order

**Causes:**
- Slow subscribers
- Channel congestion
- Too many filters

**Solutions:**
- Increase channel capacity
- Optimize filter logic
- Monitor `WS_MESSAGE_LATENCY`

## Future Improvements

### Planned Features

1. **Authentication & Authorization**
   - JWT token validation
   - Role-based access control
   - Per-subscription permissions

2. **Message Compression**
   - Per-message compression
   - Configurable compression levels
   - Binary protocol support

3. **Event Replay**
   - Store recent events
   - Allow clients to request history
   - Resume from last event

4. **Advanced Filtering**
   - Complex query language
   - Regular expression support
   - Time-based filters

5. **Multi-Region Support**
   - Event federation
   - Regional broadcasters
   - Cross-region replication

6. **Rate Limiting**
   - Per-client limits
   - Per-subscription limits
   - Adaptive throttling

## References

- [Axum WebSocket Documentation](https://docs.rs/axum/latest/axum/extract/ws/index.html)
- [Tokio Broadcast Channel](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [WebSocket RFC 6455](https://tools.ietf.org/html/rfc6455)
- [Prometheus Client Rust](https://docs.rs/prometheus/latest/prometheus/)
