# WebSocket API Reference

## Overview

This document provides complete API reference for the WebSocket streaming interface of the LLM Incident Manager. The WebSocket API is built on GraphQL subscriptions and provides real-time incident updates.

## Endpoint

### WebSocket URL

```
ws://hostname:port/graphql/ws
wss://hostname:port/graphql/ws  (TLS/SSL)
```

**Default Port:** 8080 (configurable via `server.http_port`)

**Example:**
```
ws://localhost:8080/graphql/ws
wss://api.example.com/graphql/ws
```

## Protocol

The WebSocket API uses the **graphql-ws** protocol (formerly known as graphql-transport-ws).

**Protocol Identifier:** `graphql-transport-ws`

**Subprotocol Header:**
```
Sec-WebSocket-Protocol: graphql-transport-ws
```

## Message Types Reference

### Client → Server Messages

#### connection_init

Initiates the connection with optional authentication.

**Message Format:**
```json
{
  "type": "connection_init",
  "payload": {
    "Authorization": "Bearer YOUR_JWT_TOKEN"
  }
}
```

**Fields:**
- `type` (string, required): Must be "connection_init"
- `payload` (object, optional): Connection parameters including auth tokens

**Response:** Server will send `connection_ack` or `connection_error`

---

#### subscribe

Starts a new subscription.

**Message Format:**
```json
{
  "id": "unique-subscription-id",
  "type": "subscribe",
  "payload": {
    "query": "subscription { incidentUpdates { updateType incidentId } }",
    "variables": {
      "severities": ["P0", "P1"]
    },
    "operationName": "IncidentUpdates"
  }
}
```

**Fields:**
- `id` (string, required): Unique identifier for this subscription
- `type` (string, required): Must be "subscribe"
- `payload` (object, required):
  - `query` (string, required): GraphQL subscription query
  - `variables` (object, optional): Query variables
  - `operationName` (string, optional): Operation name if query has multiple operations

**Response:** Server will stream `next` messages with subscription data

---

#### complete

Stops an active subscription.

**Message Format:**
```json
{
  "id": "unique-subscription-id",
  "type": "complete"
}
```

**Fields:**
- `id` (string, required): ID of the subscription to stop
- `type` (string, required): Must be "complete"

**Response:** Server will send `complete` acknowledgment

---

#### ping

Keep-alive message to maintain connection.

**Message Format:**
```json
{
  "type": "ping"
}
```

**Fields:**
- `type` (string, required): Must be "ping"

**Response:** Server will send `pong`

---

### Server → Client Messages

#### connection_ack

Acknowledges successful connection initialization.

**Message Format:**
```json
{
  "type": "connection_ack"
}
```

**Fields:**
- `type` (string): "connection_ack"

---

#### connection_error

Indicates connection initialization failure.

**Message Format:**
```json
{
  "type": "connection_error",
  "payload": {
    "message": "Authentication required",
    "extensions": {
      "code": "UNAUTHENTICATED"
    }
  }
}
```

**Fields:**
- `type` (string): "connection_error"
- `payload` (object): Error details

**Note:** Connection will be closed after sending this message.

---

#### next

Delivers subscription data.

**Message Format:**
```json
{
  "id": "unique-subscription-id",
  "type": "next",
  "payload": {
    "data": {
      "incidentUpdates": {
        "updateType": "CREATED",
        "incidentId": "550e8400-e29b-41d4-a716-446655440000",
        "timestamp": "2025-11-12T20:30:45.123456Z"
      }
    },
    "errors": []
  }
}
```

**Fields:**
- `id` (string): Subscription ID
- `type` (string): "next"
- `payload` (object):
  - `data` (object): Subscription result data
  - `errors` (array, optional): GraphQL errors if any

---

#### error

Indicates a subscription error (non-terminal).

**Message Format:**
```json
{
  "id": "unique-subscription-id",
  "type": "error",
  "payload": [
    {
      "message": "Internal server error",
      "path": ["incidentUpdates"],
      "extensions": {
        "code": "INTERNAL_SERVER_ERROR"
      }
    }
  ]
}
```

**Fields:**
- `id` (string): Subscription ID
- `type` (string): "error"
- `payload` (array): Array of GraphQL errors

**Note:** Subscription continues after error; use `complete` to stop.

---

#### complete

Indicates subscription completion.

**Message Format:**
```json
{
  "id": "unique-subscription-id",
  "type": "complete"
}
```

**Fields:**
- `id` (string): Subscription ID
- `type` (string): "complete"

**Note:** No more messages will be sent for this subscription ID.

---

#### pong

Response to ping message.

**Message Format:**
```json
{
  "type": "pong"
}
```

**Fields:**
- `type` (string): "pong"

---

## Subscription Types

### incidentUpdates

Subscribe to incident update events with optional filtering.

**Subscription Definition:**
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

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `incidentIds` | `[UUID!]` | No | Filter by specific incident IDs |
| `severities` | `[Severity!]` | No | Filter by severity levels (P0, P1, P2, P3, P4) |
| `activeOnly` | `Boolean` | No | If true, only active incidents (default: false) |

**Return Type:**
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

**Example Request:**
```json
{
  "id": "sub-1",
  "type": "subscribe",
  "payload": {
    "query": "subscription($severities: [Severity!]) { incidentUpdates(severities: $severities) { updateType incidentId timestamp } }",
    "variables": {
      "severities": ["P0", "P1"]
    }
  }
}
```

**Example Response:**
```json
{
  "id": "sub-1",
  "type": "next",
  "payload": {
    "data": {
      "incidentUpdates": {
        "updateType": "CREATED",
        "incidentId": "550e8400-e29b-41d4-a716-446655440000",
        "timestamp": "2025-11-12T20:30:45.123456Z"
      }
    }
  }
}
```

---

### newIncidents

Subscribe to newly created incidents.

**Subscription Definition:**
```graphql
subscription NewIncidents($severities: [Severity!]) {
  newIncidents(severities: $severities) {
    id
    title
    description
    severity
    state
    incidentType
    source
    createdAt
    updatedAt
    assignedTo
    affectedResources
    labels
    metadata
  }
}
```

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `severities` | `[Severity!]` | No | Filter by severity levels |

**Return Type:**
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

**Enums:**
```graphql
enum Severity {
  P0  # Critical
  P1  # High
  P2  # Medium
  P3  # Low
  P4  # Informational
}

enum IncidentState {
  NEW
  INVESTIGATING
  IDENTIFIED
  MONITORING
  RESOLVED
  CLOSED
}

enum IncidentType {
  AVAILABILITY
  PERFORMANCE
  SECURITY
  DATA_INTEGRITY
  CAPACITY
  CONFIGURATION
  DEPENDENCY
  OTHER
}
```

**Example Request:**
```json
{
  "id": "sub-2",
  "type": "subscribe",
  "payload": {
    "query": "subscription { newIncidents(severities: [P0]) { id title severity state createdAt } }"
  }
}
```

**Example Response:**
```json
{
  "id": "sub-2",
  "type": "next",
  "payload": {
    "data": {
      "newIncidents": {
        "id": "660e8400-e29b-41d4-a716-446655440001",
        "title": "Database connection pool exhausted",
        "severity": "P0",
        "state": "NEW",
        "createdAt": "2025-11-12T20:35:12.789123Z"
      }
    }
  }
}
```

---

### criticalIncidents

Subscribe to critical incidents (P0 and P1 only).

**Subscription Definition:**
```graphql
subscription CriticalIncidents {
  criticalIncidents {
    id
    title
    description
    severity
    state
    incidentType
    source
    createdAt
    affectedResources
  }
}
```

**Parameters:** None (automatically filters to P0 and P1)

**Return Type:** `Incident` (same as newIncidents)

**Example Request:**
```json
{
  "id": "sub-3",
  "type": "subscribe",
  "payload": {
    "query": "subscription { criticalIncidents { id title severity state affectedResources } }"
  }
}
```

---

### incidentStateChanges

Subscribe to incident state transitions.

**Subscription Definition:**
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

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `incidentId` | `UUID` | No | Watch specific incident (omit for all incidents) |

**Return Type:**
```graphql
type IncidentStateChange {
  incidentId: UUID!
  oldState: IncidentState!
  newState: IncidentState!
  changedBy: String!
  timestamp: DateTime!
}
```

**Example Request:**
```json
{
  "id": "sub-4",
  "type": "subscribe",
  "payload": {
    "query": "subscription($incidentId: UUID) { incidentStateChanges(incidentId: $incidentId) { incidentId oldState newState changedBy timestamp } }",
    "variables": {
      "incidentId": "550e8400-e29b-41d4-a716-446655440000"
    }
  }
}
```

**Example Response:**
```json
{
  "id": "sub-4",
  "type": "next",
  "payload": {
    "data": {
      "incidentStateChanges": {
        "incidentId": "550e8400-e29b-41d4-a716-446655440000",
        "oldState": "NEW",
        "newState": "INVESTIGATING",
        "changedBy": "ops-team@example.com",
        "timestamp": "2025-11-12T20:40:00.123456Z"
      }
    }
  }
}
```

---

### alerts

Subscribe to incoming alert submissions.

**Subscription Definition:**
```graphql
subscription Alerts($sources: [String!]) {
  alerts(sources: $sources) {
    id
    externalId
    source
    title
    description
    severity
    alertType
    labels
    affectedServices
    runbookUrl
    annotations
    receivedAt
  }
}
```

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `sources` | `[String!]` | No | Filter by alert sources (e.g., "prometheus", "datadog") |

**Return Type:**
```graphql
type Alert {
  id: UUID!
  externalId: String!
  source: String!
  title: String!
  description: String!
  severity: Severity!
  alertType: AlertType!
  labels: Map!
  affectedServices: [String!]!
  runbookUrl: String
  annotations: Map!
  receivedAt: DateTime!
  status: AlertStatus!
}

enum AlertType {
  METRIC
  LOG
  TRACE
  EVENT
  SYNTHETIC
}

enum AlertStatus {
  PENDING
  PROCESSING
  DEDUPLICATED
  CREATED_INCIDENT
  MERGED
  IGNORED
}
```

**Example Request:**
```json
{
  "id": "sub-5",
  "type": "subscribe",
  "payload": {
    "query": "subscription { alerts(sources: [\"prometheus\", \"datadog\"]) { id source title severity receivedAt } }"
  }
}
```

---

## Scalar Types

### UUID

**Format:** RFC 4122 UUID string

**Example:** `"550e8400-e29b-41d4-a716-446655440000"`

**Validation:** Must be valid UUID v4 or v7

---

### DateTime

**Format:** ISO 8601 with timezone

**Example:** `"2025-11-12T20:30:45.123456Z"`

**Timezone:** Always UTC (Z suffix)

---

### Map

**Format:** JSON object with string keys

**Example:**
```json
{
  "environment": "production",
  "region": "us-east-1",
  "cluster": "prod-primary"
}
```

---

## Error Codes and Meanings

### Authentication Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHENTICATED` | 401 | No authentication provided |
| `UNAUTHORIZED` | 403 | Invalid or expired token |
| `FORBIDDEN` | 403 | Insufficient permissions |

---

### Validation Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `BAD_USER_INPUT` | 400 | Invalid input parameters |
| `GRAPHQL_PARSE_FAILED` | 400 | Invalid GraphQL syntax |
| `GRAPHQL_VALIDATION_FAILED` | 400 | Query validation failed |

---

### Server Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INTERNAL_SERVER_ERROR` | 500 | Unexpected server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |
| `TIMEOUT` | 504 | Request timeout |

---

### Rate Limiting Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `SUBSCRIPTION_LIMIT_EXCEEDED` | 429 | Too many active subscriptions |

---

## Rate Limits and Quotas

### Connection Limits

| Resource | Default Limit | Configurable |
|----------|---------------|--------------|
| Concurrent connections per IP | 100 | Yes (future) |
| Subscriptions per connection | 10 | Yes (future) |
| Message rate (msgs/sec) | 100 | Yes (future) |

### Data Limits

| Resource | Limit |
|----------|-------|
| Max message size | 256 KB |
| Max subscription query depth | 10 |
| Max subscription complexity | 100 |

---

## Connection Parameters

### Query Parameters

None. Connection initiated via WebSocket upgrade.

### Headers

**Required Headers:**
```
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Version: 13
Sec-WebSocket-Key: <base64-encoded-key>
```

**Optional Headers:**
```
Sec-WebSocket-Protocol: graphql-transport-ws
Authorization: Bearer <token>
```

**Example Connection Request:**
```http
GET /graphql/ws HTTP/1.1
Host: localhost:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Version: 13
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Protocol: graphql-transport-ws
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Server Response:**
```http
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
Sec-WebSocket-Protocol: graphql-transport-ws
```

---

## Protocol Versioning

### Current Version

**Protocol:** `graphql-transport-ws`
**Version:** 1.0

### Version Negotiation

The server supports only the `graphql-transport-ws` protocol. If a client requests a different subprotocol, the connection will be established without a subprotocol agreement, and the server will close the connection after the first invalid message.

**Supported Subprotocols:**
- `graphql-transport-ws` (preferred)

**Deprecated Subprotocols:**
- `graphql-ws` (old protocol, not supported)
- `subscriptions-transport-ws` (legacy, not supported)

---

## Complete Example

### Full Connection Flow

```javascript
// 1. Establish WebSocket connection
const ws = new WebSocket('ws://localhost:8080/graphql/ws', 'graphql-transport-ws');

// 2. Initialize connection
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'connection_init',
    payload: {
      Authorization: 'Bearer YOUR_TOKEN'
    }
  }));
};

// 3. Handle connection acknowledgment
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  if (message.type === 'connection_ack') {
    console.log('Connection established');

    // 4. Subscribe to critical incidents
    ws.send(JSON.stringify({
      id: '1',
      type: 'subscribe',
      payload: {
        query: `
          subscription {
            criticalIncidents {
              id
              title
              severity
              state
              createdAt
            }
          }
        `
      }
    }));
  }

  if (message.type === 'next') {
    console.log('Received data:', message.payload.data);
  }

  if (message.type === 'error') {
    console.error('Subscription error:', message.payload);
  }

  if (message.type === 'complete') {
    console.log('Subscription completed:', message.id);
  }
};

// 5. Unsubscribe when done
setTimeout(() => {
  ws.send(JSON.stringify({
    id: '1',
    type: 'complete'
  }));
}, 60000); // After 1 minute

// 6. Close connection gracefully
ws.onclose = () => {
  console.log('Connection closed');
};
```

---

## Testing the API

### Using wscat (Command Line)

```bash
# Install wscat
npm install -g wscat

# Connect to WebSocket endpoint
wscat -c ws://localhost:8080/graphql/ws -s graphql-transport-ws

# Send connection_init
> {"type":"connection_init"}

# Wait for connection_ack
< {"type":"connection_ack"}

# Subscribe to incidents
> {"id":"1","type":"subscribe","payload":{"query":"subscription { newIncidents { id title severity } }"}}

# Receive updates
< {"id":"1","type":"next","payload":{"data":{"newIncidents":{"id":"...","title":"...","severity":"P0"}}}}

# Unsubscribe
> {"id":"1","type":"complete"}
```

### Using GraphQL Playground

1. Navigate to: `http://localhost:8080/graphql/playground`
2. Click "SUBSCRIPTIONS" tab
3. Enter subscription query
4. Click play button
5. View real-time updates in the results pane

---

## See Also

- [WebSocket Streaming Guide](./WEBSOCKET_STREAMING_GUIDE.md) - Architecture and concepts
- [WebSocket Client Guide](./WEBSOCKET_CLIENT_GUIDE.md) - Client implementation examples
- [WebSocket Deployment Guide](./WEBSOCKET_DEPLOYMENT_GUIDE.md) - Production deployment
- [GraphQL Schema Reference](./GRAPHQL_SCHEMA_REFERENCE.md) - Complete type system
