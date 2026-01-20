# Google SQL / Incident Memory Wiring

## 1. Architectural Confirmation

### 1.1 NO Direct SQL Connection

**CONFIRMED:** LLM-Incident-Manager does NOT connect directly to Google SQL.

Evidence from codebase:
- No `sqlx`, `diesel`, `postgres`, or `tokio-postgres` dependencies in `Cargo.toml`
- No SQL query strings in source code
- All persistence goes through `@agentics/ruvector-client` in TypeScript agents
- All persistence goes through `RuvectorClient` in Rust code

### 1.2 ALL DecisionEvents via ruvector-service

**CONFIRMED:** All incident lifecycle DecisionEvents are written via ruvector-service.

| Event Type | Decision Type | Agent | Persistence Path |
|------------|---------------|-------|------------------|
| Incident Creation | `incident_open` | Escalation Agent | `DecisionEventStore.store()` |
| Severity Transition | `incident_escalation_decision` | Escalation Agent | `DecisionEventStore.store()` |
| Escalation | `incident_escalation_decision` | Escalation Agent | `DecisionEventStore.store()` |
| Human Approval | `incident_approval_decision` | HITL Agent | `DecisionEventStore.store()` |
| Post-Mortem Generation | `incident_postmortem_generated` | Post-Mortem Agent | `DecisionEventStore.store()` |
| Incident Resolution | `incident_resolution_decision` | (via API) | `RuvectorClient.updateIncidentState()` |
| Incident Close | `incident_close` | (via API) | `RuvectorClient.updateIncidentState()` |

## 2. ruvector-client Usage

### 2.1 TypeScript Agents (packages/agents/*)

```typescript
// From packages/agents/escalation-agent/src/handler.ts

import {
  RuvectorClient,
  DecisionEventStore,
  TelemetryEmitter
} from '@agentics/ruvector-client';

// Initialize client
const ruvectorClient = new RuvectorClient({
  baseUrl: config.ruvector.baseUrl,    // From RUVECTOR_SERVICE_URL
  apiKey: config.ruvector.apiKey,       // From RUVECTOR_API_KEY
  timeoutMs: config.ruvector.timeoutMs  // From RUVECTOR_TIMEOUT_MS
});

// Create store
const decisionStore = new DecisionEventStore(ruvectorClient);

// Persist DecisionEvent (CANONICAL OUTPUT)
const storeResult = await decisionStore.store(decisionEvent, ESCALATION_PERSISTENCE);

// Update incident state
await ruvectorClient.updateIncidentState(input.incident_id, {
  severity: result.output.new_severity,
  escalation_level: result.output.new_escalation_level,
  status: 'ESCALATED'
});
```

### 2.2 Rust Core (src/*)

The Rust core uses internal state management but emits DecisionEvents to ruvector-service for persistence:

```rust
// Conceptual - actual implementation in src/state/mod.rs
pub trait IncidentStore: Send + Sync {
    async fn store_decision_event(&self, event: DecisionEvent) -> Result<(), StoreError>;
    async fn update_incident_state(&self, id: &str, state: IncidentState) -> Result<(), StoreError>;
}

// RuvectorStore implementation calls ruvector-service HTTP API
impl IncidentStore for RuvectorStore {
    async fn store_decision_event(&self, event: DecisionEvent) -> Result<(), StoreError> {
        self.client.post(&format!("{}/v1/decisions", self.base_url))
            .json(&event)
            .send()
            .await?;
        Ok(())
    }
}
```

## 3. Schema Compatibility with agentics-contracts

### 3.1 DecisionEvent Schema

The `DecisionEvent` schema in `@agentics/contracts` is the canonical format:

```typescript
interface DecisionEvent<TOutput = unknown> {
  // REQUIRED - all must be present
  id: UUID;
  agent_id: AgentId;
  agent_version: SemVer;
  agent_classification: AgentClassification;
  decision_type: DecisionType;
  inputs_hash: SHA256Hash;
  outputs: TOutput;
  confidence: number;           // 0.0 - 1.0
  constraints_applied: DecisionConstraints;
  execution_ref: UUID;
  timestamp: ISO8601Timestamp;
  requires_review: boolean;

  // OPTIONAL
  environment?: Environment;
  trace_id?: string;
  audit_metadata?: AuditMetadata;
}
```

### 3.2 Decision Types for Incident Lifecycle

```typescript
type DecisionType =
  | 'incident_open'
  | 'incident_escalation_decision'
  | 'incident_approval_decision'
  | 'incident_resolution_decision'
  | 'incident_inspection'
  | 'incident_close'
  | 'incident_postmortem_generated'  // Custom for post-mortem
  | 'remediation_trigger'
  | 'notification_dispatch'
  | 'policy_evaluation'
  | 'no_action_required'
  | 'deferred_decision';
```

## 4. Append-Only Persistence Behavior

### 4.1 Confirmation

**CONFIRMED:** All DecisionEvents are append-only.

- `DECISION_EVENT_PERSISTENCE.ttl_seconds = 0` (no expiry)
- DecisionEvents are permanent audit records
- No DELETE operations on DecisionEvents
- No UPDATE operations on DecisionEvents (only new events)

### 4.2 Audit Trail Guarantee

Every agent invocation produces **exactly ONE** DecisionEvent:

```typescript
// From packages/agents/escalation-agent/src/handler.ts:214-216
// Persist DecisionEvent to ruvector-service
// THIS IS THE CANONICAL OUTPUT - exactly ONE DecisionEvent per invocation
const storeResult = await decisionStore.store(decisionEvent, ESCALATION_PERSISTENCE);
```

## 5. Idempotent Writes and Retry Safety

### 5.1 Idempotency Implementation

DecisionEvents are idempotent based on:

1. **Event ID**: Each DecisionEvent has a unique UUID
2. **Execution Ref**: Each invocation has a unique execution_ref
3. **Inputs Hash**: SHA-256 hash of inputs for deduplication

```typescript
// Idempotency check in ruvector-service (server-side)
// If a DecisionEvent with the same id exists, return 200 OK without re-writing
```

### 5.2 Retry Safety

The client implements retry with exponential backoff:

```typescript
// @agentics/ruvector-client retry logic
const storeResult = await decisionStore.store(decisionEvent, persistence, {
  retries: 3,
  backoffMs: 100,
  maxBackoffMs: 5000
});
```

Retries are safe because:
- Same `id` = idempotent write
- Server returns `200 OK` if event already exists
- No duplicate events in audit trail

## 6. Validation Checklist

Before deployment, verify:

- [ ] No `sqlx`, `diesel`, `postgres` in Cargo.toml
- [ ] No SQL strings in source code (`grep -r "SELECT|INSERT|UPDATE|DELETE" src/`)
- [ ] All agents use `@agentics/ruvector-client`
- [ ] `DecisionEvent` schema matches `@agentics/contracts`
- [ ] `RUVECTOR_SERVICE_URL` configured for target environment
- [ ] `RUVECTOR_API_KEY` available in Secret Manager
- [ ] Append-only behavior confirmed (no DELETE/UPDATE on events)
- [ ] Idempotent writes enabled
- [ ] Retry logic configured with exponential backoff

## 7. Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                     LLM-Incident-Manager                            │
│                                                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │ Escalation  │  │    HITL     │  │ Post-Mortem │                 │
│  │   Agent     │  │    Agent    │  │   Agent     │                 │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                 │
│         │                │                │                         │
│         └────────────────┼────────────────┘                         │
│                          │                                          │
│                          ▼                                          │
│         ┌────────────────────────────────────┐                      │
│         │      DecisionEventStore            │                      │
│         │      (@agentics/ruvector-client)   │                      │
│         └────────────────┬───────────────────┘                      │
│                          │ HTTPS                                    │
└──────────────────────────┼──────────────────────────────────────────┘
                           │
                           ▼
              ┌────────────────────────────────┐
              │       ruvector-service         │
              │                                │
              │  ┌──────────────────────────┐  │
              │  │   DecisionEvent API      │  │
              │  │   POST /v1/decisions     │  │
              │  │   GET /v1/decisions/{id} │  │
              │  └────────────┬─────────────┘  │
              │               │                │
              │               ▼                │
              │  ┌──────────────────────────┐  │
              │  │   Google SQL (Postgres)  │  │
              │  │   Append-Only Tables     │  │
              │  └──────────────────────────┘  │
              └────────────────────────────────┘
```
