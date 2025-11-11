# LLM-Incident-Manager Integration Guide

## Overview

This guide provides detailed instructions for integrating the LLM-Incident-Manager with ecosystem components (LLM-Sentinel, LLM-Shield, LLM-Edge-Agent, LLM-Governance-Core) and external services.

---

## Table of Contents

1. [Integration Architecture](#integration-architecture)
2. [LLM-Sentinel Integration](#llm-sentinel-integration)
3. [LLM-Shield Integration](#llm-shield-integration)
4. [LLM-Edge-Agent Integration](#llm-edge-agent-integration)
5. [LLM-Governance-Core Integration](#llm-governance-core-integration)
6. [Notification Integrations](#notification-integrations)
7. [Ticketing System Integrations](#ticketing-system-integrations)
8. [Custom Integrations](#custom-integrations)

---

## 1. Integration Architecture

### Integration Patterns

```
┌─────────────────────────────────────────────────────────────┐
│              INTEGRATION PATTERN TYPES                      │
└─────────────────────────────────────────────────────────────┘

1. PUSH (Event-Driven)
   Source System ──[HTTP/gRPC]──▶ Incident Manager
   - Real-time event delivery
   - Low latency
   - Requires retry logic in source

2. PULL (Polling)
   Incident Manager ──[Periodic Fetch]──▶ Source System
   - Controlled load
   - Higher latency
   - Simple source integration

3. MESSAGE QUEUE (Async)
   Source ──▶ [Queue] ──▶ Incident Manager
   - Decoupled systems
   - Guaranteed delivery
   - Requires queue infrastructure

4. WEBHOOK (Callback)
   Incident Manager ──[HTTP POST]──▶ External System
   - Notification delivery
   - Status updates
   - Requires external endpoint
```

### Authentication Methods

| Method | Use Case | Security Level |
|--------|----------|----------------|
| API Key | Simple integrations | Medium |
| OAuth 2.0 | Third-party services | High |
| mTLS | Internal services | Very High |
| Basic Auth | Legacy systems | Low |
| JWT | Stateless auth | High |

---

## 2. LLM-Sentinel Integration

### Overview

LLM-Sentinel detects anomalies in LLM system behavior (latency, throughput, error rates) and sends events to the Incident Manager.

### Integration Flow

```
LLM-Sentinel                           Incident Manager
─────────────                          ────────────────

┌─────────────┐
│  Metrics    │
│  Collector  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Anomaly    │
│  Detector   │──── Anomaly Detected
└──────┬──────┘
       │
       │ HTTP POST
       │ /api/v1/events/sentinel
       ▼
              ┌──────────────────┐
              │  Event Ingestion │
              │  - Validate      │
              │  - Deduplicate   │
              │  - Enrich        │
              └────────┬─────────┘
                       │
                       ▼
              ┌──────────────────┐
              │  Incident Created│
              │  INC-2025-12345  │
              └──────────────────┘
```

### Configuration

**Sentinel Configuration** (`sentinel-config.yaml`):

```yaml
integrations:
  incident_manager:
    enabled: true
    endpoint: https://incidents.example.com/api/v1/events/sentinel
    authentication:
      type: api_key
      api_key: ${INCIDENT_MANAGER_API_KEY}

    # Event filtering
    filters:
      min_severity: medium
      categories:
        - performance
        - availability

    # Retry configuration
    retry:
      max_attempts: 3
      initial_delay_ms: 1000
      max_delay_ms: 30000
      backoff_multiplier: 2

    # Timeout
    timeout_ms: 5000

    # Batching (optional)
    batching:
      enabled: false
      max_batch_size: 50
      max_wait_ms: 5000
```

**Incident Manager Configuration** (`config.yaml`):

```yaml
integrations:
  llm-sentinel:
    enabled: true

    # Authentication
    authentication:
      type: api_key
      api_keys:
        - key_id: sentinel-prod-001
          key_hash: ${SENTINEL_API_KEY_HASH}
          permissions: ["event:create"]

    # Rate limiting
    rate_limit:
      requests_per_minute: 1000
      burst: 100

    # Severity mapping
    severity_mapping:
      critical: P0
      high: P1
      medium: P2
      low: P3
      info: P4

    # Auto-routing
    routing:
      default_team: platform-team
      escalation_policy: sentinel-escalation
```

### API Contract

**Request Format**:

```typescript
POST /api/v1/events/sentinel
Content-Type: application/json
X-API-Key: ${SENTINEL_API_KEY}

{
  "event_id": "sent-2025-001234",
  "source": "llm-sentinel",
  "source_version": "1.2.3",
  "timestamp": "2025-11-11T10:30:00Z",
  "event_type": "anomaly",
  "category": "performance",
  "title": "Latency Spike Detected",
  "description": "95th percentile latency exceeded threshold by 270%",
  "severity": "high",
  "resource": {
    "type": "endpoint",
    "id": "ep-chat-completion",
    "name": "/v1/chat/completions",
    "metadata": {
      "model": "gpt-4",
      "region": "us-west-2",
      "deployment_id": "dep-12345"
    }
  },
  "metrics": {
    "p95_latency_ms": 5400,
    "threshold_ms": 2000,
    "baseline_ms": 800,
    "duration_sec": 300,
    "affected_requests": 1234
  },
  "tags": {
    "environment": "production",
    "tenant_id": "tenant-123",
    "cluster": "prod-us-west-2-a"
  },
  "payload": {
    "anomaly_score": 0.89,
    "detection_algorithm": "isolation_forest",
    "confidence": 0.95,
    "contributing_factors": [
      "database_connection_pool_saturation",
      "increased_model_inference_time"
    ]
  }
}
```

**Response Format**:

```typescript
HTTP/1.1 202 Accepted
Content-Type: application/json

{
  "status": "accepted",
  "event_id": "sent-2025-001234",
  "incident_id": "INC-20251111-00042",
  "message": "Event received and queued for processing",
  "links": {
    "incident": "https://incidents.example.com/incidents/INC-20251111-00042",
    "api": "https://incidents.example.com/api/v1/incidents/INC-20251111-00042"
  }
}
```

### Code Example (Sentinel SDK)

```typescript
import { IncidentManagerClient } from '@llm-devops/incident-manager-sdk';

// Initialize client
const incidentClient = new IncidentManagerClient({
  endpoint: 'https://incidents.example.com',
  apiKey: process.env.INCIDENT_MANAGER_API_KEY,
  retry: {
    maxAttempts: 3,
    backoff: 'exponential'
  }
});

// Send anomaly event
async function reportAnomaly(anomaly: Anomaly) {
  try {
    const response = await incidentClient.createEvent({
      eventId: anomaly.id,
      source: 'llm-sentinel',
      sourceVersion: '1.2.3',
      timestamp: new Date().toISOString(),
      eventType: 'anomaly',
      category: 'performance',
      title: anomaly.title,
      description: anomaly.description,
      severity: anomaly.severity,
      resource: anomaly.resource,
      metrics: anomaly.metrics,
      tags: anomaly.tags,
      payload: anomaly.details
    });

    console.log(`Incident created: ${response.incidentId}`);
    return response;

  } catch (error) {
    console.error('Failed to report anomaly', error);
    // Fallback: write to local file for retry
    await writeToFailureQueue(anomaly);
    throw error;
  }
}
```

---

## 3. LLM-Shield Integration

### Overview

LLM-Shield detects security violations (prompt injection, data leakage, policy violations) and sends critical security alerts.

### Integration Flow

```
LLM-Shield                             Incident Manager
───────────                            ────────────────

┌─────────────┐
│  Security   │
│  Scanner    │──── Violation Detected
└──────┬──────┘
       │
       │ gRPC Call
       │ CreateIncident()
       ▼
              ┌──────────────────┐
              │  gRPC Endpoint   │
              │  Validate & Auth │
              └────────┬─────────┘
                       │
                       ▼
              ┌──────────────────┐
              │  High-Priority   │
              │  Queue           │
              │  (Security)      │
              └────────┬─────────┘
                       │
                       ▼
              ┌──────────────────┐
              │  Auto-Escalate   │
              │  to Security Team│
              └──────────────────┘
```

### Configuration

**Shield Configuration** (`shield-config.yaml`):

```yaml
integrations:
  incident_manager:
    enabled: true
    endpoint: incidents.example.com:9090
    protocol: grpc

    # mTLS authentication
    authentication:
      type: mtls
      client_cert_path: /etc/shield/certs/client.crt
      client_key_path: /etc/shield/certs/client.key
      ca_cert_path: /etc/shield/certs/ca.crt

    # Security-specific settings
    security:
      encrypt_payload: true
      redact_pii: true
      include_request_hash: true

    # Only send critical violations
    filters:
      min_severity: high
      event_types:
        - prompt_injection
        - data_leakage
        - policy_violation
        - privilege_escalation
```

**Incident Manager Configuration**:

```yaml
integrations:
  llm-shield:
    enabled: true

    # mTLS configuration
    authentication:
      type: mtls
      ca_cert_path: /etc/incident-manager/certs/ca.crt
      verify_client_cert: true

    # Priority routing for security incidents
    routing:
      default_team: security-team
      escalation_policy: security-critical
      auto_page: true  # Auto-page on-call for P0/P1

    # Auto-remediation
    auto_remediation:
      enabled: true
      actions:
        - type: block_user
          conditions:
            severity: [P0]
            event_type: [prompt_injection, data_leakage]
        - type: rate_limit
          conditions:
            severity: [P1]
            event_type: [policy_violation]
```

### gRPC Service Definition

```protobuf
syntax = "proto3";

package incident_manager;

service IncidentService {
  rpc CreateIncident(CreateIncidentRequest) returns (CreateIncidentResponse);
  rpc GetIncident(GetIncidentRequest) returns (Incident);
  rpc UpdateIncident(UpdateIncidentRequest) returns (Incident);
  rpc StreamIncidents(StreamIncidentsRequest) returns (stream Incident);
}

message CreateIncidentRequest {
  string event_id = 1;
  string source = 2;
  string source_version = 3;
  string timestamp = 4;
  string event_type = 5;
  string category = 6;
  string title = 7;
  string description = 8;
  string severity = 9;
  Resource resource = 10;
  map<string, double> metrics = 11;
  map<string, string> tags = 12;
  bytes encrypted_payload = 13;  // Encrypted sensitive data
}

message Resource {
  string type = 1;
  string id = 2;
  string name = 3;
  map<string, string> metadata = 4;
}

message CreateIncidentResponse {
  string status = 1;
  string incident_id = 2;
  string message = 3;
  map<string, string> links = 4;
}

message GetIncidentRequest {
  string incident_id = 1;
}

message Incident {
  string id = 1;
  string status = 2;
  string severity = 3;
  string title = 4;
  string description = 5;
  string created_at = 6;
  string updated_at = 7;
  // ... other fields
}
```

### Code Example (Shield Integration)

```go
package main

import (
    "context"
    "crypto/tls"
    "crypto/x509"
    "io/ioutil"
    "log"

    pb "github.com/llm-devops/incident-manager/proto"
    "google.golang.org/grpc"
    "google.golang.org/grpc/credentials"
)

func main() {
    // Load mTLS certificates
    cert, err := tls.LoadX509KeyPair(
        "/etc/shield/certs/client.crt",
        "/etc/shield/certs/client.key",
    )
    if err != nil {
        log.Fatalf("Failed to load client cert: %v", err)
    }

    // Load CA certificate
    caCert, err := ioutil.ReadFile("/etc/shield/certs/ca.crt")
    if err != nil {
        log.Fatalf("Failed to load CA cert: %v", err)
    }

    caCertPool := x509.NewCertPool()
    caCertPool.AppendCertsFromPEM(caCert)

    // Create TLS credentials
    tlsConfig := &tls.Config{
        Certificates: []tls.Certificate{cert},
        RootCAs:      caCertPool,
    }
    creds := credentials.NewTLS(tlsConfig)

    // Connect to incident manager
    conn, err := grpc.Dial(
        "incidents.example.com:9090",
        grpc.WithTransportCredentials(creds),
    )
    if err != nil {
        log.Fatalf("Failed to connect: %v", err)
    }
    defer conn.Close()

    client := pb.NewIncidentServiceClient(conn)

    // Create incident for security violation
    ctx := context.Background()
    resp, err := client.CreateIncident(ctx, &pb.CreateIncidentRequest{
        EventId:       "shield-2025-005678",
        Source:        "llm-shield",
        SourceVersion: "2.1.0",
        Timestamp:     "2025-11-11T10:35:00Z",
        EventType:     "violation",
        Category:      "security",
        Title:         "Prompt Injection Detected",
        Description:   "Malicious prompt injection attempt blocked",
        Severity:      "critical",
        Resource: &pb.Resource{
            Type: "model",
            Id:   "model-gpt4-prod",
            Name: "GPT-4 Production",
            Metadata: map[string]string{
                "model_version":  "gpt-4-0613",
                "deployment_id":  "dep-12345",
            },
        },
        Metrics: map[string]float64{
            "confidence_score": 0.95,
            "attack_vectors":   3,
        },
        Tags: map[string]string{
            "environment": "production",
            "user_id":     "user-789",
            "session_id":  "sess-abc123",
        },
        EncryptedPayload: encryptedDetails,
    })

    if err != nil {
        log.Fatalf("Failed to create incident: %v", err)
    }

    log.Printf("Incident created: %s", resp.IncidentId)
}
```

---

## 4. LLM-Edge-Agent Integration

### Overview

LLM-Edge-Agent acts as a runtime proxy and sends real-time alerts for rate limiting, quota violations, and request patterns.

### Integration Flow

```
LLM-Edge-Agent                         Incident Manager
──────────────                         ────────────────

┌─────────────┐
│  Runtime    │
│  Proxy      │
└──────┬──────┘
       │
       │ WebSocket Connection
       │ wss://incidents.example.com/ws/events
       ▼
              ┌──────────────────┐
              │  WebSocket       │
              │  Handler         │
              └────────┬─────────┘
                       │
                       │ Real-time Events
                       ▼
              ┌──────────────────┐
              │  Event Stream    │
              │  Processor       │
              └────────┬─────────┘
                       │
                       ▼
              ┌──────────────────┐
              │  Incident Queue  │
              └──────────────────┘
```

### Configuration

**Edge Agent Configuration** (`edge-agent-config.yaml`):

```yaml
integrations:
  incident_manager:
    enabled: true
    endpoint: wss://incidents.example.com/ws/events

    # Authentication
    authentication:
      type: jwt
      jwt_token: ${INCIDENT_MANAGER_JWT}

    # Connection settings
    connection:
      auto_reconnect: true
      reconnect_interval_ms: 5000
      max_reconnect_attempts: 10
      heartbeat_interval_ms: 30000
      ping_timeout_ms: 10000

    # Batching for high-volume events
    batching:
      enabled: true
      max_batch_size: 100
      max_wait_ms: 5000
      compression: gzip

    # Buffer for offline mode
    offline_buffer:
      enabled: true
      max_size: 10000
      persistence_path: /var/lib/edge-agent/incident-buffer.db
```

### WebSocket Protocol

**Connection Handshake**:

```javascript
// Client initiates connection
const ws = new WebSocket('wss://incidents.example.com/ws/events');

// Server responds with connection confirmation
{
  "type": "connection_ack",
  "session_id": "sess-abc123",
  "protocol_version": "1.0",
  "features": ["batching", "compression", "filtering"]
}
```

**Message Types**:

```typescript
// 1. Single Event
{
  "type": "event",
  "event_id": "edge-2025-009876",
  "source": "llm-edge-agent",
  "timestamp": "2025-11-11T10:40:00Z",
  "event_type": "alert",
  "category": "availability",
  "title": "Rate Limit Exceeded",
  "description": "Tenant exceeded rate limit threshold",
  "severity": "medium",
  "resource": {
    "type": "tenant",
    "id": "tenant-456",
    "name": "Acme Corp"
  },
  "metrics": {
    "current_rpm": 12500,
    "limit_rpm": 10000,
    "throttled_requests": 2500
  },
  "tags": {
    "environment": "production",
    "edge_location": "edge-us-east-1-a"
  }
}

// 2. Batched Events
{
  "type": "batch",
  "batch_id": "batch-20251111-001",
  "timestamp": "2025-11-11T10:40:05Z",
  "compression": "gzip",
  "events": [
    // Array of events (gzip compressed)
  ],
  "count": 87
}

// 3. Heartbeat
{
  "type": "heartbeat",
  "timestamp": "2025-11-11T10:40:30Z"
}

// 4. Acknowledgment
{
  "type": "ack",
  "event_id": "edge-2025-009876",
  "incident_id": "INC-20251111-00043",
  "status": "processed"
}
```

### Code Example (Edge Agent Integration)

```typescript
import WebSocket from 'ws';
import pako from 'pako';

class IncidentManagerWSClient {
  private ws: WebSocket;
  private eventBuffer: any[] = [];
  private batchTimer: NodeJS.Timeout | null = null;

  constructor(
    private endpoint: string,
    private jwt: string,
    private config: {
      batchSize: number;
      batchInterval: number;
    }
  ) {}

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.endpoint, {
        headers: {
          'Authorization': `Bearer ${this.jwt}`
        }
      });

      this.ws.on('open', () => {
        console.log('Connected to Incident Manager');
        this.startHeartbeat();
        resolve();
      });

      this.ws.on('message', (data) => {
        this.handleMessage(JSON.parse(data.toString()));
      });

      this.ws.on('close', () => {
        console.log('Disconnected from Incident Manager');
        this.reconnect();
      });

      this.ws.on('error', (error) => {
        console.error('WebSocket error:', error);
        reject(error);
      });
    });
  }

  sendEvent(event: any): void {
    this.eventBuffer.push(event);

    if (this.eventBuffer.length >= this.config.batchSize) {
      this.flushBatch();
    } else if (!this.batchTimer) {
      this.batchTimer = setTimeout(
        () => this.flushBatch(),
        this.config.batchInterval
      );
    }
  }

  private flushBatch(): void {
    if (this.eventBuffer.length === 0) return;

    const batch = {
      type: 'batch',
      batch_id: `batch-${Date.now()}`,
      timestamp: new Date().toISOString(),
      compression: 'gzip',
      events: pako.gzip(JSON.stringify(this.eventBuffer)),
      count: this.eventBuffer.length
    };

    this.ws.send(JSON.stringify(batch));
    this.eventBuffer = [];

    if (this.batchTimer) {
      clearTimeout(this.batchTimer);
      this.batchTimer = null;
    }
  }

  private handleMessage(message: any): void {
    switch (message.type) {
      case 'connection_ack':
        console.log('Connection acknowledged:', message.session_id);
        break;
      case 'ack':
        console.log('Event acknowledged:', message.event_id);
        break;
      case 'error':
        console.error('Server error:', message.message);
        break;
    }
  }

  private startHeartbeat(): void {
    setInterval(() => {
      if (this.ws.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify({
          type: 'heartbeat',
          timestamp: new Date().toISOString()
        }));
      }
    }, 30000);
  }

  private reconnect(): void {
    setTimeout(() => {
      console.log('Attempting to reconnect...');
      this.connect().catch(console.error);
    }, 5000);
  }
}

// Usage
const client = new IncidentManagerWSClient(
  'wss://incidents.example.com/ws/events',
  process.env.INCIDENT_MANAGER_JWT!,
  {
    batchSize: 100,
    batchInterval: 5000
  }
);

await client.connect();

// Send alert when rate limit exceeded
client.sendEvent({
  event_id: `edge-${Date.now()}`,
  source: 'llm-edge-agent',
  timestamp: new Date().toISOString(),
  event_type: 'alert',
  category: 'availability',
  title: 'Rate Limit Exceeded',
  description: 'Tenant exceeded rate limit threshold',
  severity: 'medium',
  resource: {
    type: 'tenant',
    id: 'tenant-456',
    name: 'Acme Corp'
  },
  metrics: {
    current_rpm: 12500,
    limit_rpm: 10000
  }
});
```

---

## 5. LLM-Governance-Core Integration

### Overview

Bi-directional integration for audit reporting, compliance tracking, and incident analytics.

### Integration Patterns

**1. Pull: Governance pulls incident data**

```graphql
query GetIncidentReport {
  incidents(
    filter: {
      dateRange: { start: "2025-11-01T00:00:00Z", end: "2025-11-30T23:59:59Z" }
      severity: [P0, P1]
      category: [security, compliance]
    }
    pagination: { page: 1, pageSize: 100 }
  ) {
    edges {
      node {
        id
        severity
        category
        status
        created_at
        resolved_at
        resolution {
          root_cause
          actions_taken
        }
        sla {
          acknowledgment_breached
          resolution_breached
        }
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
    totalCount
  }

  incidentMetrics(
    timeRange: { start: "2025-11-01T00:00:00Z", end: "2025-11-30T23:59:59Z" }
    groupBy: [SEVERITY, CATEGORY]
  ) {
    totalIncidents
    bySeverity {
      severity
      count
    }
    averageMTTR
    averageMTTA
    slaCompliance
  }
}
```

**2. Push: Incident Manager pushes audit logs**

```json
POST /api/v1/audit-events
Content-Type: application/json
X-API-Key: ${GOVERNANCE_API_KEY}

{
  "event_id": "audit-2025-123456",
  "timestamp": "2025-11-11T10:45:00Z",
  "event_type": "incident_resolved",
  "incident_id": "INC-20251111-00042",
  "actor": {
    "type": "user",
    "id": "user-john.doe",
    "name": "John Doe"
  },
  "changes": [
    {
      "field": "status",
      "old_value": "IN_PROGRESS",
      "new_value": "RESOLVED"
    }
  ]
}
```

### Configuration

```yaml
integrations:
  llm-governance-core:
    enabled: true

    # GraphQL endpoint for governance to pull data
    graphql:
      endpoint: https://incidents.example.com/graphql
      authentication:
        type: oauth2
        client_id: ${GOVERNANCE_CLIENT_ID}
        client_secret: ${GOVERNANCE_CLIENT_SECRET}
        token_url: https://auth.example.com/oauth/token

    # REST endpoint for pushing audit logs
    audit:
      endpoint: https://governance.example.com/api/v1/audit-events
      authentication:
        type: api_key
        api_key: ${GOVERNANCE_API_KEY}

      # What to push
      push_events:
        - incident_created
        - incident_resolved
        - incident_closed
        - sla_breached
        - escalated

      # Batching
      batching:
        enabled: true
        max_batch_size: 100
        max_wait_ms: 60000
```

---

## 6. Notification Integrations

### Slack Integration

**Configuration**:

```yaml
notifications:
  slack:
    enabled: true
    webhook_url: ${SLACK_WEBHOOK_URL}
    bot_token: ${SLACK_BOT_TOKEN}  # For interactive features

    # Channel routing
    channel_routing:
      - severity: [P0, P1]
        channel: "#incidents-critical"
      - severity: [P2]
        channel: "#incidents-medium"
      - category: [security]
        channel: "#security-alerts"
      - default: "#incidents-general"

    # Message formatting
    message_format: blocks  # blocks or text
    mention_on_create: true
    mention_groups:
      P0: ["@oncall", "@incident-commander"]
      P1: ["@oncall"]
```

**Slack Block Example**:

```json
{
  "blocks": [
    {
      "type": "header",
      "text": {
        "type": "plain_text",
        "text": " P1 Incident: Latency Spike Detected"
      }
    },
    {
      "type": "section",
      "fields": [
        {
          "type": "mrkdwn",
          "text": "*Incident ID:*\nINC-20251111-00042"
        },
        {
          "type": "mrkdwn",
          "text": "*Status:*\nNEW"
        },
        {
          "type": "mrkdwn",
          "text": "*Severity:*\nP1"
        },
        {
          "type": "mrkdwn",
          "text": "*Category:*\nPerformance"
        }
      ]
    },
    {
      "type": "section",
      "text": {
        "type": "mrkdwn",
        "text": "*Description:*\n95th percentile latency exceeded threshold by 270%"
      }
    },
    {
      "type": "actions",
      "elements": [
        {
          "type": "button",
          "text": {
            "type": "plain_text",
            "text": "Acknowledge"
          },
          "value": "INC-20251111-00042",
          "action_id": "acknowledge_incident"
        },
        {
          "type": "button",
          "text": {
            "type": "plain_text",
            "text": "View Details"
          },
          "url": "https://incidents.example.com/incidents/INC-20251111-00042"
        }
      ]
    }
  ]
}
```

### PagerDuty Integration

**Configuration**:

```yaml
notifications:
  pagerduty:
    enabled: true
    api_key: ${PAGERDUTY_API_KEY}
    integration_key: ${PAGERDUTY_INTEGRATION_KEY}

    # Service routing
    service_routing:
      - severity: [P0, P1]
        service_id: "PXXXXXX"  # Critical service
      - severity: [P2]
        service_id: "PYYYYYY"  # Medium service
      - default: "PZZZZZZ"

    # Auto-resolve
    auto_resolve: true

    # Escalation
    escalate_on_timeout: true
    escalation_timeout_minutes: 15
```

**PagerDuty Event Format**:

```json
{
  "routing_key": "${PAGERDUTY_INTEGRATION_KEY}",
  "event_action": "trigger",
  "payload": {
    "summary": "P1: Latency Spike Detected",
    "source": "llm-sentinel",
    "severity": "critical",
    "timestamp": "2025-11-11T10:30:00Z",
    "custom_details": {
      "incident_id": "INC-20251111-00042",
      "category": "performance",
      "resource": "/v1/chat/completions",
      "metrics": {
        "p95_latency_ms": 5400,
        "threshold_ms": 2000
      }
    }
  },
  "links": [
    {
      "href": "https://incidents.example.com/incidents/INC-20251111-00042",
      "text": "View Incident"
    }
  ]
}
```

---

## 7. Ticketing System Integrations

### JIRA Integration

**Configuration**:

```yaml
integrations:
  jira:
    enabled: true
    base_url: https://your-company.atlassian.net
    authentication:
      type: basic
      username: ${JIRA_USERNAME}
      api_token: ${JIRA_API_TOKEN}

    # Project mapping
    project_mapping:
      - category: [security]
        project_key: "SEC"
        issue_type: "Security Incident"
      - category: [performance, availability]
        project_key: "OPS"
        issue_type: "Incident"
      - default:
        project_key: "INCIDENT"
        issue_type: "Bug"

    # Field mapping
    field_mapping:
      severity:
        P0: "Critical"
        P1: "High"
        P2: "Medium"
        P3: "Low"
        P4: "Trivial"
      priority:
        P0: "Highest"
        P1: "High"
        P2: "Medium"
        P3: "Low"
        P4: "Lowest"

    # Auto-sync
    sync:
      enabled: true
      bidirectional: true  # Sync JIRA updates back to incidents
      sync_interval_seconds: 300
```

**Create JIRA Issue**:

```json
POST /rest/api/3/issue
Authorization: Basic ${BASE64_ENCODED_CREDS}

{
  "fields": {
    "project": {
      "key": "OPS"
    },
    "summary": "[INC-20251111-00042] Latency Spike Detected",
    "description": {
      "type": "doc",
      "version": 1,
      "content": [
        {
          "type": "paragraph",
          "content": [
            {
              "type": "text",
              "text": "95th percentile latency exceeded threshold by 270%"
            }
          ]
        }
      ]
    },
    "issuetype": {
      "name": "Incident"
    },
    "priority": {
      "name": "High"
    },
    "labels": ["llm-incident-manager", "automated", "performance"],
    "customfield_10001": "INC-20251111-00042"  // Incident ID custom field
  }
}
```

---

## 8. Custom Integrations

### Webhook Integration

**Configuration**:

```yaml
integrations:
  webhooks:
    - name: "custom-monitoring-system"
      enabled: true
      url: https://monitoring.example.com/webhooks/incidents
      authentication:
        type: bearer
        token: ${MONITORING_WEBHOOK_TOKEN}

      # Trigger conditions
      triggers:
        - event: incident_created
          conditions:
            severity: [P0, P1]
        - event: incident_resolved
        - event: sla_breached

      # Request configuration
      method: POST
      headers:
        Content-Type: application/json
        X-Source: llm-incident-manager

      # Payload template
      payload_template: |
        {
          "event_type": "{{event_type}}",
          "incident": {
            "id": "{{incident.id}}",
            "title": "{{incident.title}}",
            "severity": "{{incident.severity}}",
            "status": "{{incident.status}}",
            "url": "https://incidents.example.com/incidents/{{incident.id}}"
          },
          "timestamp": "{{timestamp}}"
        }

      # Retry configuration
      retry:
        max_attempts: 3
        backoff: exponential

      # Timeout
      timeout_ms: 10000
```

### Custom SDK

Create a custom integration using the SDK:

```typescript
import { IncidentManagerSDK } from '@llm-devops/incident-manager-sdk';

const sdk = new IncidentManagerSDK({
  endpoint: 'https://incidents.example.com',
  apiKey: process.env.API_KEY
});

// Subscribe to incident events
sdk.on('incident:created', async (incident) => {
  console.log('New incident:', incident.id);

  // Custom logic
  if (incident.severity === 'P0') {
    await notifyExecutiveTeam(incident);
  }
});

sdk.on('incident:resolved', async (incident) => {
  console.log('Incident resolved:', incident.id);

  // Update external systems
  await updateStatusPage(incident);
});

// Create custom incident
await sdk.incidents.create({
  source: 'custom-monitor',
  title: 'Custom Alert',
  severity: 'P2',
  category: 'other',
  description: 'Custom monitoring alert',
  resource: {
    type: 'service',
    id: 'svc-123',
    name: 'Custom Service'
  }
});
```

---

## Testing Integrations

### Integration Test Suite

```bash
# Test Sentinel integration
curl -X POST https://incidents.example.com/api/v1/events/sentinel \
  -H "X-API-Key: ${SENTINEL_API_KEY}" \
  -H "Content-Type: application/json" \
  -d @test-data/sentinel-event.json

# Test Shield gRPC integration
grpcurl -d @ \
  -H "Authorization: Bearer ${JWT_TOKEN}" \
  incidents.example.com:9090 \
  incident_manager.IncidentService/CreateIncident \
  < test-data/shield-event.json

# Test Edge Agent WebSocket
wscat -c wss://incidents.example.com/ws/events \
  -H "Authorization: Bearer ${JWT_TOKEN}"

# Test Governance GraphQL
curl -X POST https://incidents.example.com/graphql \
  -H "Authorization: Bearer ${OAUTH_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ incidents { edges { node { id } } } }"}'
```

---

## Troubleshooting

### Common Issues

**1. Authentication failures**
```bash
# Check API key
curl -I https://incidents.example.com/api/v1/health \
  -H "X-API-Key: ${API_KEY}"

# Verify mTLS certificates
openssl s_client -connect incidents.example.com:9090 \
  -cert client.crt -key client.key -CAfile ca.crt
```

**2. Connection timeouts**
```bash
# Check network connectivity
telnet incidents.example.com 3000
telnet incidents.example.com 9090

# Check DNS resolution
nslookup incidents.example.com
```

**3. Event not received**
```bash
# Check event logs
kubectl logs -f deployment/incident-manager-api -n incident-manager | grep "event_id"

# Check dead letter queue
redis-cli LLEN incident-manager:dlq
```

---

## Support

For integration support:
- Documentation: https://docs.example.com/incident-manager/integrations
- SDK Repository: https://github.com/globalbusinessadvisors/llm-incident-manager-sdk
- Issues: https://github.com/globalbusinessadvisors/llm-incident-manager/issues
- Slack: #incident-manager-support
