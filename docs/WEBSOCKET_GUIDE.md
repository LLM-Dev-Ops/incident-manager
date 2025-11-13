# WebSocket Streaming Guide

## Overview

The LLM Incident Manager provides real-time event streaming via WebSockets, allowing clients to receive instant notifications about incidents, alerts, escalations, and other system events.

## Architecture

### Components

1. **WebSocket Server** (`/ws` endpoint)
   - Handles connection upgrades
   - Manages connection lifecycle
   - Implements heartbeat/keepalive

2. **Session Management**
   - Tracks active sessions
   - Manages subscriptions per session
   - Handles authentication (future)

3. **Connection Manager**
   - Registry of active connections
   - Connection limits enforcement
   - Cleanup of expired sessions

4. **Event Broadcasting**
   - Pub/sub event distribution
   - Topic-based routing
   - Filter-based subscriptions

5. **Event Handlers**
   - Integration hooks for system events
   - Type-safe event publishing
   - Priority-based delivery

6. **Metrics & Monitoring**
   - Prometheus metrics
   - Connection statistics
   - Event delivery tracking

## Getting Started

### Connecting to WebSocket

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  console.log('Connected to incident manager');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  handleMessage(message);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected from incident manager');
};
```

### Message Protocol

All messages are JSON-encoded with a `type` field indicating the message type.

#### Welcome Message

Sent by server on connection:

```json
{
  "type": "welcome",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "server_time": "2025-11-12T10:30:00Z"
}
```

#### Subscribe to Events

```json
{
  "type": "subscribe",
  "subscription_id": "my-subscription",
  "filters": {
    "event_types": ["incident_created", "incident_updated"],
    "severities": ["P0", "P1"],
    "sources": ["llm-sentinel"],
    "labels": {
      "environment": "production"
    }
  }
}
```

#### Subscription Confirmed

```json
{
  "type": "subscribed",
  "subscription_id": "my-subscription",
  "filters": { ... }
}
```

#### Event Notification

```json
{
  "type": "event",
  "message_id": "evt-123",
  "timestamp": "2025-11-12T10:35:00Z",
  "event": {
    "event_type": "incident_created",
    "incident": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "severity": "P0",
      "title": "API Gateway Timeout Spike",
      "state": "Detected",
      ...
    }
  }
}
```

#### Unsubscribe

```json
{
  "type": "unsubscribe",
  "subscription_id": "my-subscription"
}
```

#### Ping/Pong

Client ping:
```json
{
  "type": "ping",
  "timestamp": "2025-11-12T10:40:00Z"
}
```

Server pong:
```json
{
  "type": "pong",
  "timestamp": "2025-11-12T10:40:00Z"
}
```

## Event Types

### Incident Events

- `incident_created` - New incident created
- `incident_updated` - Incident state or details changed
- `incident_resolved` - Incident marked as resolved
- `incident_closed` - Incident closed

### Alert Events

- `alert_received` - New alert ingested
- `alert_converted` - Alert converted to incident

### Escalation Events

- `escalated` - Incident severity escalated

### Playbook Events

- `playbook_started` - Automated playbook execution started
- `playbook_action_executed` - Individual playbook action completed
- `playbook_completed` - Playbook execution finished

### Notification Events

- `notification_sent` - Notification delivered to external system

### Other Events

- `assignment_changed` - Incident assignees modified
- `comment_added` - Comment added to incident
- `system_event` - General system event

## Subscription Filters

### Event Type Filter

Subscribe to specific event types:

```json
{
  "event_types": ["incident_created", "alert_received"]
}
```

Empty array = all event types.

### Severity Filter

Filter by incident severity:

```json
{
  "severities": ["P0", "P1"]
}
```

Empty array = all severities.

### State Filter

Filter by incident state:

```json
{
  "states": ["Detected", "Investigating"]
}
```

### Source Filter

Filter by alert/incident source:

```json
{
  "sources": ["llm-sentinel", "llm-shield"]
}
```

### Resource Filter

Filter by affected resources:

```json
{
  "affected_resources": ["api-gateway", "user-service"]
}
```

### Label Filter

Filter by exact label matches (AND logic):

```json
{
  "labels": {
    "environment": "production",
    "team": "platform"
  }
}
```

### Incident ID Filter

Subscribe to specific incidents:

```json
{
  "incident_ids": ["550e8400-e29b-41d4-a716-446655440000"]
}
```

## Example Client Implementations

### JavaScript/Browser

```javascript
class IncidentManagerClient {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.handlers = new Map();
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('Connected');
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);

      if (msg.type === 'event') {
        this.handleEvent(msg.event);
      } else if (msg.type === 'welcome') {
        this.sessionId = msg.session_id;
        this.onConnect();
      }
    };
  }

  subscribe(filters, handler) {
    const subId = `sub-${Date.now()}`;
    this.handlers.set(subId, handler);

    this.send({
      type: 'subscribe',
      subscription_id: subId,
      filters: filters
    });

    return subId;
  }

  handleEvent(event) {
    for (const handler of this.handlers.values()) {
      handler(event);
    }
  }

  send(message) {
    this.ws.send(JSON.stringify(message));
  }

  onConnect() {
    // Override in subclass
  }
}

// Usage
const client = new IncidentManagerClient('ws://localhost:8080/ws');
client.onConnect = () => {
  client.subscribe({
    event_types: ['incident_created'],
    severities: ['P0', 'P1']
  }, (event) => {
    console.log('Critical incident:', event);
  });
};
client.connect();
```

### Python

```python
import asyncio
import json
import websockets

class IncidentManagerClient:
    def __init__(self, url):
        self.url = url
        self.handlers = {}

    async def connect(self):
        async with websockets.connect(self.url) as ws:
            self.ws = ws

            # Receive welcome
            welcome = await ws.recv()
            msg = json.loads(welcome)
            self.session_id = msg['session_id']

            # Start message loop
            await self.message_loop()

    async def subscribe(self, filters, handler):
        sub_id = f"sub-{len(self.handlers)}"
        self.handlers[sub_id] = handler

        await self.send({
            'type': 'subscribe',
            'subscription_id': sub_id,
            'filters': filters
        })

        return sub_id

    async def message_loop(self):
        async for message in self.ws:
            msg = json.loads(message)

            if msg['type'] == 'event':
                await self.handle_event(msg['event'])

    async def handle_event(self, event):
        for handler in self.handlers.values():
            await handler(event)

    async def send(self, message):
        await self.ws.send(json.dumps(message))

# Usage
async def on_incident(event):
    print(f"Incident: {event}")

async def main():
    client = IncidentManagerClient('ws://localhost:8080/ws')

    async with websockets.connect(client.url) as ws:
        client.ws = ws

        # Wait for welcome
        welcome = await ws.recv()

        # Subscribe
        await client.subscribe({
            'event_types': ['incident_created']
        }, on_incident)

        # Listen for events
        await client.message_loop()

asyncio.run(main())
```

### Rust

```rust
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8080/ws";
    let (ws_stream, _) = connect_async(url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to events
    let subscribe = json!({
        "type": "subscribe",
        "subscription_id": "rust-client",
        "filters": {
            "event_types": ["incident_created", "incident_updated"],
            "severities": ["P0", "P1"]
        }
    });

    write
        .send(Message::Text(subscribe.to_string()))
        .await
        .unwrap();

    // Listen for events
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            let event: serde_json::Value = serde_json::from_str(&text).unwrap();
            println!("Received: {}", event);
        }
    }
}
```

## Configuration

### Server Configuration

```rust
use llm_incident_manager::websocket::{WebSocketConfig, WebSocketState};

let config = WebSocketConfig {
    max_pending_messages: 1000,      // Buffer size per connection
    heartbeat_interval_secs: 30,     // Ping interval
    session_timeout_secs: 300,       // 5 minutes idle timeout
    cleanup_interval_secs: 60,       // Cleanup check interval
    broadcast_capacity: 10000,       // Event channel capacity
    enable_compression: true,        // WebSocket compression
};

let state = WebSocketState::new(config);
```

### Builder Pattern

```rust
use llm_incident_manager::websocket::WebSocketStateBuilder;

let state = WebSocketStateBuilder::new()
    .max_pending_messages(500)
    .heartbeat_interval_secs(60)
    .session_timeout_secs(600)
    .broadcast_capacity(5000)
    .build();
```

## Monitoring & Metrics

### Prometheus Metrics

Available at `/metrics`:

- `websocket_active_connections` - Current active connections
- `websocket_total_connections` - Total connections since startup
- `websocket_active_subscriptions` - Current active subscriptions
- `websocket_messages_sent_total` - Messages sent to clients
- `websocket_messages_received_total` - Messages received from clients
- `websocket_events_broadcast_total` - Events broadcast
- `websocket_events_delivered_total` - Events successfully delivered
- `websocket_connection_errors_total` - Connection errors
- `websocket_send_errors_total` - Send errors
- `websocket_session_duration_seconds` - Session duration histogram
- `websocket_message_latency_seconds` - Message latency histogram

### Runtime Statistics

```rust
// Get connection stats
let conn_stats = state.connection_stats();
println!("Active connections: {}", conn_stats.active_connections);
println!("Total events delivered: {}", conn_stats.total_events_delivered);

// Get event stats
let event_stats = state.event_stats();
println!("Total events: {}", event_stats.total_events);
```

## Best Practices

### 1. Connection Management

- Implement exponential backoff for reconnection
- Handle `close` events gracefully
- Use heartbeat/ping to detect connection issues

### 2. Event Filtering

- Use specific filters to reduce bandwidth
- Subscribe only to needed event types
- Filter by severity for alerting systems

### 3. Error Handling

- Handle parse errors for malformed messages
- Implement retry logic for failed sends
- Log all WebSocket errors for debugging

### 4. Performance

- Batch event processing when possible
- Use connection pooling for multiple subscriptions
- Monitor message buffer sizes

### 5. Security

- Use WSS (WebSocket Secure) in production
- Implement authentication (token-based)
- Validate all incoming messages
- Rate limit subscriptions

## Troubleshooting

### Connection Refused

- Check server is running: `curl http://localhost:8080/health`
- Verify WebSocket endpoint: `ws://localhost:8080/ws`
- Check firewall rules

### No Events Received

- Verify subscription filters
- Check event types are correct
- Ensure incidents are being created
- Check subscription was confirmed

### Connection Drops

- Check heartbeat interval
- Verify network stability
- Review server logs for errors
- Check session timeout settings

### High Latency

- Monitor server CPU/memory
- Check broadcast channel capacity
- Review connection count
- Optimize event filters

## Advanced Usage

### Multiple Subscriptions

```javascript
// Subscribe to critical incidents
client.subscribe({
  severities: ['P0', 'P1']
}, handleCritical);

// Subscribe to security incidents
client.subscribe({
  event_types: ['incident_created'],
  labels: { category: 'security' }
}, handleSecurity);
```

### Event Acknowledgment

```javascript
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);

  if (msg.type === 'event') {
    // Process event
    handleEvent(msg.event);

    // Acknowledge receipt
    ws.send(JSON.stringify({
      type: 'ack',
      message_id: msg.message_id
    }));
  }
};
```

### Connection Pooling

```javascript
class ConnectionPool {
  constructor(url, poolSize = 5) {
    this.connections = [];

    for (let i = 0; i < poolSize; i++) {
      this.connections.push(new WebSocket(url));
    }
  }

  getConnection() {
    // Round-robin or load-based selection
    return this.connections[Math.floor(Math.random() * this.connections.length)];
  }
}
```

## Future Enhancements

- [ ] Authentication via JWT tokens
- [ ] Message compression options
- [ ] Binary protocol support
- [ ] Replay of missed events
- [ ] Subscription persistence
- [ ] Rate limiting per client
- [ ] Multi-region support
- [ ] Event schema validation

## Support

For issues or questions:
- GitHub Issues: https://github.com/llm-devops/llm-incident-manager/issues
- Documentation: https://docs.llm-incident-manager.io
- Community: https://community.llm-incident-manager.io
