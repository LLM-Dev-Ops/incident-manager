# CLI Activation Verification

## 1. CLI Overview

LLM-Incident-Manager provides two CLI interfaces:

1. **`llm-im-cli`** - Rust-based management CLI (compiled binary)
2. **`agentics-cli`** - Platform-wide CLI with incident-manager commands

Both CLIs dynamically resolve the service URL via environment configuration.

## 2. CLI Commands Verification

### 2.1 Escalation Agent Commands

| CLI Command | Agent Endpoint | Description |
|-------------|----------------|-------------|
| `llm-im-cli escalate evaluate` | `POST /v1/agents/escalation/evaluate` | Evaluate incident for escalation |
| `escalate evaluate <incident_id>` | `POST /v1/agents/escalation/evaluate` | Evaluate via agentics-cli |
| `escalate inspect <incident_id>` | `GET /v1/agents/escalation/inspect/{id}` | Inspect escalation state |
| `escalate list` | `GET /v1/agents/escalation/list` | List active escalations |

### 2.2 HITL Agent Commands

| CLI Command | Agent Endpoint | Description |
|-------------|----------------|-------------|
| `approve request <incident_id> <action_type>` | `POST /v1/agents/hitl/request` | Request approval |
| `approve decide <request_id> <approve\|reject>` | `POST /v1/agents/hitl/decide` | Record decision |
| `approve status <request_id>` | `GET /v1/agents/hitl/status/{id}` | Check approval status |
| `approve list` | `GET /v1/agents/hitl/list` | List pending approvals |
| `approve inspect <request_id>` | `GET /v1/agents/hitl/inspect/{id}` | Full audit trail |

### 2.3 Post-Mortem Agent Commands

| CLI Command | Agent Endpoint | Description |
|-------------|----------------|-------------|
| `llm-im-cli postmortem generate <incident_id>` | `POST /v1/postmortems/generate/{id}` | Generate post-mortem |
| `llm-im-cli postmortem get <postmortem_id>` | `GET /v1/postmortems/{id}` | Get post-mortem |
| `llm-im-cli postmortem list` | `GET /v1/postmortems` | List post-mortems |
| `llm-im-cli postmortem publish <postmortem_id>` | `POST /v1/postmortems/{id}/publish` | Publish post-mortem |
| `llm-im-cli postmortem inspect <postmortem_id>` | `GET /v1/postmortems/{id}/decision-event` | Inspect DecisionEvent |

### 2.4 Incident Management Commands

| CLI Command | Agent Endpoint | Description |
|-------------|----------------|-------------|
| `llm-im-cli alert` | `POST /v1/alerts` | Submit alert |
| `llm-im-cli list` | `GET /v1/incidents` | List incidents |
| `llm-im-cli get <incident_id>` | `GET /v1/incidents/{id}` | Get incident |
| `llm-im-cli resolve <incident_id>` | `POST /v1/incidents/{id}/resolve` | Resolve incident |
| `llm-im-cli health` | `GET /health` | Health check |

### 2.5 Replay Command (agentics-cli)

| CLI Command | Agent Endpoint | Description |
|-------------|----------------|-------------|
| `replay incident <incident_id>` | `GET /v1/incidents/{id}/replay` | Replay incident events |
| `replay decision <decision_id>` | `GET /v1/decisions/{id}/replay` | Replay decision event |

## 3. Dynamic Service URL Resolution

### 3.1 CLI Configuration

The CLI resolves the service URL from multiple sources (in order of precedence):

1. `--endpoint` flag (highest priority)
2. `AGENTICS_INCIDENT_MANAGER_URL` environment variable
3. `~/.agentics/config.yaml` configuration file
4. Default: `http://localhost:8080` (development only)

### 3.2 Configuration File Format

```yaml
# ~/.agentics/config.yaml
incident-manager:
  url: https://llm-incident-manager-${PROJECT_ID}.run.app
  timeout: 30s
  auth:
    type: gcloud-identity-token
```

### 3.3 No CLI Change Required for Redeployment

**CONFIRMED:** No CLI modification is required when agents are redeployed.

The CLI uses:
- Dynamic URL resolution (not hardcoded)
- Environment-based configuration
- Service discovery via GCP metadata (optional)

## 4. Example Invocations

### 4.1 Escalation Agent

```bash
# Evaluate incident for escalation
llm-im-cli --endpoint https://llm-incident-manager-prod.run.app \
  escalate evaluate inc-12345 \
  --signal-source sentinel \
  --policy-id default-policy \
  --verbose

# Expected output:
{
  "success": true,
  "execution_id": "exec-abc123",
  "decision_event": {
    "id": "evt-xyz789",
    "agent_id": "incident-escalation-agent:1.0.0",
    "decision_type": "incident_escalation_decision",
    "outputs": {
      "decision": "escalate",
      "new_severity": "P1",
      "new_escalation_level": 2,
      "reason": "SLA breach threshold exceeded"
    },
    "confidence": 0.92
  }
}

# Inspect escalation state
llm-im-cli escalate inspect inc-12345 --include-history

# List active escalations
llm-im-cli escalate list --severity P0 --status active --limit 10
```

### 4.2 HITL Agent (Human-in-the-Loop)

```bash
# Request approval for remediation
agentics-cli approve request inc-12345 remediation \
  --action-description "Rollback deployment to v1.2.3" \
  --target api-gateway \
  --priority critical \
  --deadline 2024-01-15T12:00:00Z

# Expected output:
{
  "success": true,
  "approval_request_id": "apr-abc123",
  "status": "pending",
  "required_approvers": ["incident_commander", "sre_lead"],
  "deadline": "2024-01-15T12:00:00Z"
}

# Record approval decision
agentics-cli approve decide apr-abc123 approve \
  --approver-id user@example.com \
  --rationale "Verified impact is acceptable, proceed with rollback" \
  --conditions "Monitor for 30 minutes post-rollback"

# Expected output:
{
  "success": true,
  "decision": "approved",
  "action_authorized": true,
  "decision_event": {
    "id": "evt-def456",
    "decision_type": "incident_approval_decision",
    "outputs": {
      "decision": "approved",
      "approvals_obtained": 2,
      "approvals_remaining": 0,
      "action_authorized": true
    }
  }
}

# Check approval status
agentics-cli approve status apr-abc123 --verbose

# List pending approvals
agentics-cli approve list --action-type remediation --status pending
```

### 4.3 Post-Mortem Generator Agent

```bash
# Generate post-mortem for resolved incident
llm-im-cli postmortem generate inc-12345 --output json

# Expected output:
{
  "success": true,
  "postmortem_id": "pm-abc123",
  "status": "draft",
  "content": {
    "incident_id": "inc-12345",
    "title": "API Gateway Outage - 2024-01-10",
    "summary": "...",
    "timeline": [...],
    "root_cause": "...",
    "impact": {...},
    "action_items": [...]
  },
  "decision_event": {
    "id": "evt-ghi789",
    "decision_type": "incident_postmortem_generated"
  }
}

# Retrieve post-mortem
llm-im-cli postmortem get pm-abc123

# List post-mortems for review
llm-im-cli postmortem list --status draft --limit 20

# Publish post-mortem (make immutable)
llm-im-cli postmortem publish pm-abc123 --reviewed-by user@example.com

# Expected output:
{
  "success": true,
  "postmortem_id": "pm-abc123",
  "status": "published",
  "published_at": "2024-01-15T14:30:00Z",
  "reviewed_by": "user@example.com"
}

# Inspect DecisionEvent for audit
llm-im-cli postmortem inspect pm-abc123

# Expected output:
{
  "decision_event": {
    "id": "evt-ghi789",
    "agent_id": "postmortem-generator:1.0.0",
    "agent_classification": "DOCUMENTATION",
    "decision_type": "incident_postmortem_generated",
    "inputs_hash": "sha256:abc123...",
    "timestamp": "2024-01-15T10:00:00Z",
    "requires_review": true
  }
}
```

### 4.4 Incident Replay

```bash
# Replay incident timeline
agentics-cli replay incident inc-12345 --format timeline

# Expected output:
INCIDENT REPLAY: inc-12345
========================
2024-01-10T10:00:00Z  CREATED       Alert received from Sentinel
2024-01-10T10:00:15Z  ESCALATED     Severity: P2 -> P1
2024-01-10T10:05:00Z  APPROVAL_REQ  Remediation approval requested
2024-01-10T10:07:30Z  APPROVED      Approved by incident_commander
2024-01-10T10:10:00Z  RESOLVED      Rollback completed
2024-01-10T10:30:00Z  CLOSED        Post-mortem generated
```

## 5. Health Check Verification

```bash
# Verify CLI can reach service
llm-im-cli --endpoint https://llm-incident-manager-prod.run.app health

# Expected output:
{
  "status": "healthy",
  "version": "1.0.1",
  "components": {
    "api": "healthy",
    "ruvector": "healthy",
    "telemetry": "healthy"
  },
  "timestamp": "2024-01-15T12:00:00Z"
}
```

## 6. Verification Checklist

Before deployment is considered complete:

- [ ] `llm-im-cli health` returns 200 OK
- [ ] `escalate evaluate` returns valid DecisionEvent
- [ ] `escalate inspect` returns escalation state
- [ ] `escalate list` returns list (may be empty)
- [ ] `approve request` creates approval request
- [ ] `approve status` returns request status
- [ ] `approve list` returns list (may be empty)
- [ ] `postmortem generate` generates post-mortem (for resolved incidents)
- [ ] `postmortem list` returns list (may be empty)
- [ ] `postmortem inspect` returns DecisionEvent
- [ ] No CLI changes required for redeployment
- [ ] Service URL resolves dynamically from config
