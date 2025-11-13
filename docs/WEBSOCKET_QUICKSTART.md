# WebSocket Quick Start Guide

## 30-Second Overview

The LLM Incident Manager provides real-time event streaming via WebSockets at `ws://host:port/ws`.

```javascript
// Connect
const ws = new WebSocket('ws://localhost:8080/ws');

// Subscribe to critical incidents
ws.send(JSON.stringify({
  type: 'subscribe',
  subscription_id: 'critical-incidents',
  filters: { severities: ['P0', 'P1'] }
}));

// Receive events
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === 'event') {
    console.log('New incident:', msg.event.incident);
  }
};
```

## Quick Reference

### Client → Server Messages

#### Subscribe
```json
{
  "type": "subscribe",
  "subscription_id": "my-sub",
  "filters": {
    "event_types": ["incident_created"],
    "severities": ["P0", "P1"],
    "sources": ["llm-sentinel"]
  }
}
```

#### Unsubscribe
```json
{
  "type": "unsubscribe",
  "subscription_id": "my-sub"
}
```

#### Ping
```json
{
  "type": "ping",
  "timestamp": "2025-11-12T10:00:00Z"
}
```

### Server → Client Messages

#### Welcome (on connect)
```json
{
  "type": "welcome",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "server_time": "2025-11-12T10:00:00Z"
}
```

#### Event
```json
{
  "type": "event",
  "message_id": "evt-123",
  "timestamp": "2025-11-12T10:01:00Z",
  "event": {
    "event_type": "incident_created",
    "incident": { ... }
  }
}
```

#### Error
```json
{
  "type": "error",
  "code": "INVALID_MESSAGE",
  "message": "Failed to parse message"
}
```

## Event Types

| Event Type | Description |
|------------|-------------|
| `incident_created` | New incident detected |
| `incident_updated` | Incident state changed |
| `incident_resolved` | Incident resolved |
| `alert_received` | New alert ingested |
| `escalated` | Severity escalated |
| `playbook_started` | Playbook execution began |
| `notification_sent` | External notification sent |

## Filter Options

| Filter | Type | Example |
|--------|------|---------|
| `event_types` | Array | `["incident_created"]` |
| `severities` | Array | `["P0", "P1"]` |
| `states` | Array | `["Detected", "Investigating"]` |
| `sources` | Array | `["llm-sentinel"]` |
| `affected_resources` | Array | `["api-gateway"]` |
| `labels` | Object | `{"env": "production"}` |
| `incident_ids` | Array | `["uuid-here"]` |

**Empty arrays = match all**

## Common Patterns

### Monitor Critical Incidents
```javascript
ws.send(JSON.stringify({
  type: 'subscribe',
  subscription_id: 'critical',
  filters: { severities: ['P0', 'P1'] }
}));
```

### Monitor Specific Service
```javascript
ws.send(JSON.stringify({
  type: 'subscribe',
  subscription_id: 'api-gateway',
  filters: { affected_resources: ['api-gateway'] }
}));
```

### Monitor Production Only
```javascript
ws.send(JSON.stringify({
  type: 'subscribe',
  subscription_id: 'production',
  filters: { labels: { environment: 'production' } }
}));
```

### Track Specific Incident
```javascript
ws.send(JSON.stringify({
  type: 'subscribe',
  subscription_id: 'incident-123',
  filters: { incident_ids: ['550e8400-e29b-41d4-a716-446655440000'] }
}));
```

## Client Libraries

### JavaScript/Node.js
```bash
npm install ws
```
```javascript
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:8080/ws');
```

### Python
```bash
pip install websockets
```
```python
import asyncio
import websockets

async with websockets.connect('ws://localhost:8080/ws') as ws:
    await ws.send('...')
```

### Rust
```toml
tokio-tungstenite = "0.21"
```
```rust
use tokio_tungstenite::connect_async;
let (ws, _) = connect_async("ws://localhost:8080/ws").await?;
```

### Go
```bash
go get github.com/gorilla/websocket
```
```go
import "github.com/gorilla/websocket"
c, _, err := websocket.DefaultDialer.Dial("ws://localhost:8080/ws", nil)
```

## Configuration

### Default Settings
- **Heartbeat:** 30 seconds
- **Session timeout:** 5 minutes
- **Max pending messages:** 1000/connection
- **Broadcast capacity:** 10,000 events

### Custom Configuration
```rust
use llm_incident_manager::websocket::WebSocketStateBuilder;

let state = WebSocketStateBuilder::new()
    .heartbeat_interval_secs(60)
    .session_timeout_secs(600)
    .broadcast_capacity(5000)
    .build();
```

## Monitoring

### Prometheus Metrics (at /metrics)
```
# Active connections
websocket_active_connections

# Total messages sent
websocket_messages_sent_total

# Event delivery rate
rate(websocket_events_delivered_total[1m])

# Average session duration
websocket_session_duration_seconds
```

### Health Check
```bash
curl http://localhost:8080/health
```

## Troubleshooting

### Connection refused
```bash
# Check server is running
curl http://localhost:8080/health

# Verify WebSocket endpoint
wscat -c ws://localhost:8080/ws
```

### No events received
1. Check subscription was confirmed
2. Verify filters are correct
3. Ensure incidents are being created
4. Check server logs

### Connection drops
1. Increase heartbeat interval
2. Check network stability
3. Monitor server resources

## Testing Tools

### wscat (CLI)
```bash
npm install -g wscat
wscat -c ws://localhost:8080/ws
```

### websocat
```bash
brew install websocat  # macOS
websocat ws://localhost:8080/ws
```

### Browser DevTools
```javascript
// In browser console
const ws = new WebSocket('ws://localhost:8080/ws');
ws.onmessage = (e) => console.log(JSON.parse(e.data));
```

## Production Checklist

- [ ] Use WSS (WebSocket Secure) with TLS
- [ ] Deploy behind reverse proxy
- [ ] Implement authentication
- [ ] Set up monitoring/alerts
- [ ] Configure rate limiting
- [ ] Test reconnection logic
- [ ] Document client integration
- [ ] Load test with expected traffic

## Example: Full Client

```javascript
class IncidentStreamClient {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.reconnectDelay = 1000;
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('Connected');
      this.reconnectDelay = 1000; // Reset backoff
      this.subscribe();
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      this.handleMessage(msg);
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onclose = () => {
      console.log('Disconnected, reconnecting...');
      setTimeout(() => this.connect(), this.reconnectDelay);
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000);
    };
  }

  subscribe() {
    this.send({
      type: 'subscribe',
      subscription_id: 'all-incidents',
      filters: {}  // All events
    });
  }

  handleMessage(msg) {
    switch (msg.type) {
      case 'welcome':
        console.log('Session ID:', msg.session_id);
        break;
      case 'event':
        this.onEvent(msg.event);
        break;
      case 'error':
        console.error('Server error:', msg.message);
        break;
    }
  }

  onEvent(event) {
    // Override in subclass or set as callback
    console.log('Event:', event);
  }

  send(message) {
    this.ws.send(JSON.stringify(message));
  }
}

// Usage
const client = new IncidentStreamClient('ws://localhost:8080/ws');
client.onEvent = (event) => {
  if (event.event_type === 'incident_created') {
    // Handle new incident
    alert(`New ${event.incident.severity} incident!`);
  }
};
client.connect();
```

## More Information

- **User Guide:** [WEBSOCKET_GUIDE.md](./WEBSOCKET_GUIDE.md)
- **Technical Docs:** [WEBSOCKET_IMPLEMENTATION.md](./WEBSOCKET_IMPLEMENTATION.md)
- **Summary:** [WEBSOCKET_SUMMARY.md](./WEBSOCKET_SUMMARY.md)
- **API Docs:** https://docs.llm-incident-manager.io/api/websocket
