# @llm-dev-ops/incident-manager-client

Official TypeScript/JavaScript client SDK for the LLM Incident Manager - Enterprise-grade incident management system for LLM operations.

## Features

✅ **WebSocket Streaming** - Real-time incident updates via GraphQL subscriptions
✅ **Auto-Reconnection** - Exponential backoff with configurable retry logic
✅ **Type-Safe** - Full TypeScript support with type definitions
✅ **GraphQL API** - Query and mutate incidents with GraphQL
✅ **Subscription Helpers** - Easy-to-use subscription methods
✅ **Browser & Node.js** - Works in both environments

## Installation

```bash
npm install @llm-dev-ops/incident-manager-client
```

For Node.js environments, also install the `ws` package:

```bash
npm install ws
```

Or with yarn:

```bash
yarn add @llm-dev-ops/incident-manager-client
yarn add ws  # Node.js only
```

## Quick Start

### Subscribe to Critical Incidents

```typescript
import { IncidentManagerClient } from '@llm-dev-ops/incident-manager-client';

const client = new IncidentManagerClient({
  wsUrl: 'ws://localhost:8080/graphql/ws',
  authToken: 'your-jwt-token'
});

// Subscribe to P0 and P1 incidents
client.subscribeToCriticalIncidents((incident) => {
  console.log('🚨 Critical incident:', incident.title);
  console.log('   Severity:', incident.severity);
  console.log('   Affected:', incident.affectedResources);

  // Trigger alerts, notifications, etc.
  if (incident.severity === 'P0') {
    sendPagerDutyAlert(incident);
  }
});
```

### Subscribe to Incident Updates

```typescript
// Subscribe to updates for specific severities
client.subscribeToIncidentUpdates(['P0', 'P1', 'P2'], (update) => {
  console.log('📊 Incident update:', update.updateType);
  console.log('   Incident ID:', update.incidentId);
  console.log('   Timestamp:', update.timestamp);
});
```

### Subscribe to New Incidents

```typescript
// Get notified when new incidents are created
client.subscribeToNewIncidents((incident) => {
  console.log('🆕 New incident created:', incident.id);
  console.log('   Title:', incident.title);
  console.log('   Severity:', incident.severity);
});
```

### Subscribe to State Changes

```typescript
// Track incident lifecycle state transitions
client.subscribeToStateChanges((stateChange) => {
  console.log(`Incident ${stateChange.incidentId} changed state`);
  console.log(`  From: ${stateChange.oldState} → To: ${stateChange.newState}`);
  console.log(`  Changed by: ${stateChange.changedBy}`);
});
```

### Subscribe to All Alerts

```typescript
// Monitor all incoming alerts from source systems
client.subscribeToAlerts((alert) => {
  console.log('⚠️  Alert received from:', alert.source);
  console.log('   Type:', alert.eventType);
  console.log('   Severity:', alert.severity);
});
```

## Advanced Usage

### Custom Subscription with Filter

```typescript
import { IncidentManagerClient } from '@llm-dev-ops/incident-manager-client';

const client = new IncidentManagerClient({
  wsUrl: 'ws://localhost:8080/graphql/ws',
  authToken: 'your-jwt-token',
  retryAttempts: 10,
  retryWait: (retries) => Math.min(1000 * 2 ** retries, 60000)
});

// Custom GraphQL subscription
const unsubscribe = client.subscribe(
  `
    subscription ProductionIncidents {
      incidents(filter: { environment: "production", severity: ["P0", "P1"] }) {
        id
        title
        severity
        environment
        createdAt
        assignedTo {
          name
          email
        }
      }
    }
  `,
  (data) => {
    console.log('Production incident:', data.incidents);
  },
  (error) => {
    console.error('Subscription error:', error);
  }
);

// Later, unsubscribe
unsubscribe();
```

### Connection Event Handlers

```typescript
const client = new IncidentManagerClient({
  wsUrl: 'ws://localhost:8080/graphql/ws',
  authToken: 'your-jwt-token',
  onConnected: () => {
    console.log('✅ Connected to incident stream');
  },
  onDisconnected: () => {
    console.log('❌ Disconnected from incident stream');
  },
  onError: (error) => {
    console.error('Connection error:', error);
  }
});
```

### Cleanup

```typescript
// Unsubscribe from specific subscription
client.unsubscribe('critical-incidents');

// Unsubscribe from all subscriptions
client.unsubscribeAll();

// Close the WebSocket connection
client.close();
```

## API Reference

### Constructor Options

```typescript
interface ClientOptions {
  wsUrl: string;              // WebSocket URL (e.g., 'ws://localhost:8080/graphql/ws')
  authToken: string;          // JWT authentication token
  retryAttempts?: number;     // Max retry attempts (default: 5)
  retryWait?: (retries: number) => Promise<void>;  // Custom retry logic
  onConnected?: () => void;   // Connection established callback
  onDisconnected?: () => void; // Disconnected callback
  onError?: (error: any) => void; // Error callback
}
```

### Methods

#### `subscribeToCriticalIncidents(handler)`
Subscribe to P0 and P1 incidents.

**Parameters:**
- `handler: (incident: Incident) => void` - Callback for each critical incident

**Returns:** `void`

#### `subscribeToIncidentUpdates(severities, handler)`
Subscribe to incident lifecycle updates.

**Parameters:**
- `severities: string[]` - Array of severity levels (e.g., ['P0', 'P1'])
- `handler: (update: IncidentUpdate) => void` - Callback for each update

**Returns:** `void`

#### `subscribeToNewIncidents(handler)`
Subscribe to newly created incidents.

**Parameters:**
- `handler: (incident: Incident) => void` - Callback for each new incident

**Returns:** `void`

#### `subscribeToStateChanges(handler)`
Subscribe to incident state transitions.

**Parameters:**
- `handler: (stateChange: StateChange) => void` - Callback for state changes

**Returns:** `void`

#### `subscribeToAlerts(handler)`
Subscribe to all incoming alerts.

**Parameters:**
- `handler: (alert: Alert) => void` - Callback for each alert

**Returns:** `void`

#### `subscribe(query, onData, onError?)`
Execute a custom GraphQL subscription.

**Parameters:**
- `query: string` - GraphQL subscription query
- `onData: (data: any) => void` - Data callback
- `onError?: (error: any) => void` - Error callback

**Returns:** `() => void` - Unsubscribe function

#### `unsubscribe(subscriptionId)`
Unsubscribe from a specific subscription.

**Parameters:**
- `subscriptionId: string` - Subscription identifier

**Returns:** `void`

#### `unsubscribeAll()`
Unsubscribe from all active subscriptions.

**Returns:** `void`

#### `close()`
Close the WebSocket connection.

**Returns:** `void`

## TypeScript Types

All types are automatically imported from `@llm-dev-ops/incident-manager-types`:

```typescript
import type {
  Incident,
  Severity,
  IncidentStatus,
  IncidentUpdate,
  StateChange,
  Alert
} from '@llm-dev-ops/incident-manager-client';
```

## Environment Support

### Browser

```typescript
import { IncidentManagerClient } from '@llm-dev-ops/incident-manager-client';

const client = new IncidentManagerClient({
  wsUrl: 'wss://your-domain.com/graphql/ws',
  authToken: getAuthToken()
});
```

### Node.js

```typescript
import { IncidentManagerClient } from '@llm-dev-ops/incident-manager-client';
import WebSocket from 'ws';

const client = new IncidentManagerClient({
  wsUrl: 'ws://localhost:8080/graphql/ws',
  authToken: process.env.AUTH_TOKEN,
  webSocketImpl: WebSocket  // Required for Node.js
});
```

## Error Handling

```typescript
client.subscribeToIncidents((incident) => {
  // Handle incident
}, (error) => {
  if (error.code === 'AUTHENTICATION_ERROR') {
    console.error('Invalid auth token');
    // Refresh token and reconnect
  } else if (error.code === 'NETWORK_ERROR') {
    console.error('Network error - will auto-retry');
  }
});
```

## Examples

See the [examples directory](https://github.com/globalbusinessadvisors/llm-incident-manager/tree/main/examples/websocket) for complete examples:

- TypeScript client example
- React dashboard integration
- Node.js background worker
- Python client (for comparison)

## Related Packages

- **[@llm-dev-ops/llm-incident-manager](https://www.npmjs.com/package/@llm-dev-ops/llm-incident-manager)** - Main Rust server with npm CLI
- **[@llm-dev-ops/incident-manager-types](https://www.npmjs.com/package/@llm-dev-ops/incident-manager-types)** - TypeScript type definitions

## Documentation

For complete documentation, see the [LLM Incident Manager repository](https://github.com/globalbusinessadvisors/llm-incident-manager).

## License

MIT OR Apache-2.0

## Version

Current version: 1.0.1 (matches main package version)
