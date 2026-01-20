# Platform & Core Integration

## 1. Integration Confirmations

### 1.1 LLM-Sentinel -> Incident-Manager

**CONFIRMED:** LLM-Sentinel emits incident signals consumable by Incident-Manager.

| Signal Type | Description | Consumed By |
|-------------|-------------|-------------|
| `AnomalyEvent` | Anomaly detection flags | Escalation Agent |
| `ThreatSignal` | Security threat indicators | Escalation Agent |
| `DriftEvent` | Model/data drift detection | Escalation Agent |

**Integration Path:**
```
LLM-Sentinel
    │
    ├─── Emits anomaly/threat signals
    │
    ▼
LLM-Orchestrator
    │
    ├─── Routes to Incident-Manager
    │
    ▼
LLM-Incident-Manager (Escalation Agent)
    │
    ├─── Evaluates signals
    ├─── Determines severity
    └─── Emits DecisionEvent
```

**Code Reference:** `src/integrations/sentinel/sentinel_adapter.rs`

### 1.2 LLM-Edge-Agent & Shield -> Incident Context

**CONFIRMED:** Edge-Agent and Shield decisions contribute to incident context.

| Source | Contribution | Used For |
|--------|--------------|----------|
| Edge-Agent | Execution metrics, latency | Incident correlation |
| Shield | Policy violations, blocks | Severity assessment |

**IMPORTANT:** These contributions are **context only** - they do NOT trigger escalation directly.

### 1.3 LLM-Orchestrator Invocation

**CONFIRMED:** LLM-Orchestrator invokes Incident-Manager for:

| Action | Endpoint | Description |
|--------|----------|-------------|
| Escalation | `/v1/agents/escalation/evaluate` | Evaluate incident severity |
| Approval Gating | `/v1/agents/hitl/request` | Request human approval |
| Incident Resolution | `/v1/incidents/{id}/resolve` | Mark incident resolved |

**Orchestrator Actions Emitted by Incident-Manager:**

| Action Type | Trigger | Executed By |
|-------------|---------|-------------|
| `notify` | Escalation | Orchestrator |
| `trigger_playbooks` | Escalation | Orchestrator |
| `update_status` | Severity change | Orchestrator |
| `request_approval` | High-impact action | Orchestrator |
| `log_timeline` | Any incident event | Orchestrator |

### 1.4 Governance & Audit Views

**CONFIRMED:** Governance and audit views consume Incident DecisionEvents.

| Consumer | Event Types | Purpose |
|----------|-------------|---------|
| Governance Dashboard | All `incident_*` decisions | Compliance reporting |
| Audit Trail | All DecisionEvents | Audit log |
| Analytics Hub | Escalation decisions | Trend analysis |

**Data Flow:**
```
Incident-Manager
    │
    ├─── DecisionEvent
    │
    ▼
ruvector-service
    │
    ├─── Persisted to Google SQL
    │
    ▼
Governance Dashboard (reads from ruvector-service)
Audit Trail (reads from ruvector-service)
```

### 1.5 Core Bundle Consumption

**CONFIRMED:** Core bundles consume Incident-Manager outputs without rewiring.

| Core Bundle | Consumes | Integration |
|-------------|----------|-------------|
| `llm-sentinel-core` | Incident status | Via ruvector-service |
| `llm-observatory-core` | Telemetry events | Direct emission |
| `llm-governance-common` | DecisionEvents | Via ruvector-service |
| `llm-analytics-hub` | Incident metrics | Via ruvector-service |

**NO REWIRING REQUIRED:** All integrations use existing ruvector-service APIs.

## 2. Integration Prohibitions

### 2.1 MUST NOT Directly Invoke

**CONFIRMED:** LLM-Incident-Manager does NOT directly invoke:

| Service | Reason |
|---------|--------|
| LLM-Edge-Agent | Edge-Agent handles runtime execution |
| Shield Enforcement | Shield handles policy enforcement |
| Sentinel Detection | Sentinel handles anomaly detection |
| Auto-Optimizer | Optimizer handles performance tuning |
| Runtime Execution | Incident-Manager is outside critical path |

### 2.2 MUST NOT Invoke External Notification Systems

**CONFIRMED:** LLM-Incident-Manager does NOT emit:

| System | Reason |
|--------|--------|
| Email | Delegated to Orchestrator |
| PagerDuty | Delegated to Orchestrator |
| Slack/Webhooks | Delegated to Orchestrator |
| SMS | Delegated to Orchestrator |

**Note:** The Rust core has notification capabilities, but the deployed agents delegate all external notifications to LLM-Orchestrator via `orchestrator_actions`.

## 3. Integration Architecture Diagram

```
                     ┌─────────────────────────────────────────────────────────────┐
                     │                    AGENTICS PLATFORM                         │
                     └─────────────────────────────────────────────────────────────┘

    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
    │ LLM-Sentinel│     │  LLM-Shield │     │LLM-Edge-    │     │LLM-Auto-    │
    │ (Detection) │     │ (Enforce)   │     │Agent        │     │Optimizer    │
    └──────┬──────┘     └──────┬──────┘     └──────┬──────┘     └─────────────┘
           │                   │                   │
           │ Anomaly/Threat    │ Policy Violation  │ Execution Context
           │ Signals           │ Events            │
           │                   │                   │
           └───────────────────┼───────────────────┘
                               │
                               ▼
                     ┌─────────────────────┐
                     │   LLM-Orchestrator  │
                     │   (Coordination)    │
                     └──────────┬──────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          │                     │                     │
          ▼                     ▼                     ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│   Escalation    │   │     HITL        │   │   Post-Mortem   │
│     Agent       │   │     Agent       │   │   Generator     │
│   (Evaluate)    │   │   (Approval)    │   │   (Document)    │
└────────┬────────┘   └────────┬────────┘   └────────┬────────┘
         │                     │                     │
         │  DecisionEvent      │  DecisionEvent      │  DecisionEvent
         │                     │                     │
         └─────────────────────┼─────────────────────┘
                               │
                               ▼
                     ┌─────────────────────┐
                     │   ruvector-service  │
                     │   (Persistence)     │
                     └──────────┬──────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          │                     │                     │
          ▼                     ▼                     ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│   Governance    │   │   Analytics     │   │    Audit        │
│   Dashboard     │   │     Hub         │   │    Trail        │
└─────────────────┘   └─────────────────┘   └─────────────────┘
```

## 4. Event Flow for Common Scenarios

### 4.1 Anomaly Detected -> Incident Created

```
1. LLM-Sentinel detects anomaly
2. Sentinel emits AnomalyEvent to Orchestrator
3. Orchestrator creates incident in Incident-Manager
4. Incident-Manager Escalation Agent evaluates:
   - Determines severity (P0-P4)
   - Applies escalation policy
   - Emits DecisionEvent to ruvector-service
5. Orchestrator receives OrchestratorActions:
   - notify: Send alerts to on-call
   - trigger_playbooks: Start investigation playbook
   - update_status: Mark incident as ACKNOWLEDGED
```

### 4.2 Remediation Requires Approval

```
1. Escalation Agent determines remediation needed
2. Escalation output includes: request_approval = true
3. Orchestrator invokes HITL Agent with approval request
4. HITL Agent:
   - Creates approval request
   - Emits pending DecisionEvent
   - Returns orchestrator_actions: notify_approvers
5. Human approves via CLI or UI
6. HITL Agent:
   - Records approval
   - Emits approved DecisionEvent
   - Returns orchestrator_actions: execute_approved_action
7. Orchestrator executes remediation
```

### 4.3 Incident Resolution -> Post-Mortem

```
1. Incident marked as RESOLVED (via API or Orchestrator)
2. Orchestrator triggers Post-Mortem Agent
3. Post-Mortem Agent:
   - Retrieves incident timeline from ruvector-service
   - Generates post-mortem content
   - Emits DecisionEvent with postmortem artifact
4. Post-mortem saved to ruvector-service
5. Human reviews and publishes post-mortem
6. Published post-mortem is immutable (audit record)
```

## 5. Service Dependencies

### 5.1 Required Services

| Service | Criticality | Fallback |
|---------|-------------|----------|
| ruvector-service | CRITICAL | Circuit breaker, retry |
| LLM-Observatory | HIGH | Telemetry disabled |
| LLM-Orchestrator | HIGH | Buffered actions |

### 5.2 Optional Services

| Service | Purpose | If Unavailable |
|---------|---------|----------------|
| LLM-Sentinel | Context enrichment | Reduced context |
| LLM-Shield | Policy context | Reduced context |
| Analytics Hub | Metrics | Metrics unavailable |

## 6. No Core Bundle Rewiring

**STATEMENT OF COMPLIANCE:**

This deployment does NOT require any rewiring of core bundles:

- `llm-sentinel-core`: No changes required
- `llm-observatory-core`: No changes required
- `llm-governance-common`: No changes required
- `llm-analytics-hub`: No changes required
- `policy-engine-benchmarks`: No changes required (optional)

All integrations use existing:
- ruvector-service APIs for persistence
- Standard DecisionEvent schema
- OrchestratorAction patterns
