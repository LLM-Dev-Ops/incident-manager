# LLM-Incident-Manager Service Topology

## 1. Unified Service Definition

**Service Name:** `llm-incident-manager`
**Service Type:** Google Cloud Run (unified service)
**Deployment Region:** Configurable via `GOOGLE_CLOUD_REGION` (default: `us-central1`)
**Service URL Pattern:** `https://llm-incident-manager-{hash}-{region}.a.run.app`

## 2. Agent Endpoints

All agents are deployed as **endpoints within the unified service**, NOT as standalone services.

| Agent | Endpoint Path | Method | Description |
|-------|--------------|--------|-------------|
| **Escalation Agent** | `/v1/agents/escalation/evaluate` | POST | Evaluate incident for escalation |
| **Escalation Agent** | `/v1/agents/escalation/inspect/{incident_id}` | GET | Inspect escalation state |
| **Escalation Agent** | `/v1/agents/escalation/list` | GET | List active escalations |
| **HITL Agent** | `/v1/agents/hitl/request` | POST | Request approval for action |
| **HITL Agent** | `/v1/agents/hitl/decide` | POST | Record approval decision |
| **HITL Agent** | `/v1/agents/hitl/status/{request_id}` | GET | Check approval status |
| **HITL Agent** | `/v1/agents/hitl/list` | GET | List pending approvals |
| **Post-Mortem Agent** | `/v1/agents/postmortem/generate/{incident_id}` | POST | Generate post-mortem |
| **Post-Mortem Agent** | `/v1/agents/postmortem/{id}` | GET | Retrieve post-mortem |
| **Post-Mortem Agent** | `/v1/agents/postmortem/list` | GET | List post-mortems |
| **Post-Mortem Agent** | `/v1/agents/postmortem/{id}/publish` | POST | Publish post-mortem |

### 2.1 Backward-Compatible Endpoints (Existing API)

The unified service also exposes the existing REST API endpoints:

| Category | Endpoint | Method | Description |
|----------|----------|--------|-------------|
| Incidents | `/v1/incidents` | GET | List incidents |
| Incidents | `/v1/incidents/{id}` | GET | Get incident details |
| Incidents | `/v1/incidents/{id}/resolve` | POST | Resolve incident |
| Alerts | `/v1/alerts` | POST | Submit alert |
| Post-Mortems | `/v1/postmortems/generate/{id}` | POST | Generate post-mortem |
| Post-Mortems | `/v1/postmortems/{id}` | GET | Get post-mortem |
| Post-Mortems | `/v1/postmortems` | GET | List post-mortems |
| Post-Mortems | `/v1/postmortems/{id}/publish` | POST | Publish post-mortem |
| Health | `/health` | GET | General health |
| Health | `/health/live` | GET | Liveness probe |
| Health | `/health/ready` | GET | Readiness probe |

## 3. Deployment Confirmation

### 3.1 NO Agent Deployed as Standalone Service

**CONFIRMED:** No agent is deployed as a standalone service.

- Escalation Agent: Embedded in unified service
- HITL Agent: Embedded in unified service
- Post-Mortem Agent: Embedded in unified service

### 3.2 Shared Runtime, Configuration, and Telemetry

**CONFIRMED:** All agents share:

| Component | Shared Resource |
|-----------|-----------------|
| **Runtime** | Single Cloud Run container instance |
| **Configuration** | Environment variables + `config/default.toml` |
| **Telemetry** | LLM-Observatory via `TELEMETRY_ENDPOINT` |
| **Persistence** | ruvector-service via `RUVECTOR_SERVICE_URL` |
| **Tracing** | OpenTelemetry with shared trace context |
| **Metrics** | Prometheus endpoint at `/metrics` |

## 4. Service Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Google Cloud Run                              │
│                  llm-incident-manager                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  Escalation     │  │  Human-in-the-  │  │  Post-Mortem    │ │
│  │  Agent          │  │  Loop Agent     │  │  Generator      │ │
│  │  /v1/agents/    │  │  /v1/agents/    │  │  /v1/agents/    │ │
│  │  escalation/*   │  │  hitl/*         │  │  postmortem/*   │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                     │          │
│  ┌────────┴────────────────────┴─────────────────────┴────────┐ │
│  │                  Shared Services Layer                      │ │
│  │  - RuvectorClient (persistence)                             │ │
│  │  - TelemetryEmitter (observability)                         │ │
│  │  - DecisionEventBuilder (contract compliance)               │ │
│  └─────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│  Health: /health, /health/live, /health/ready                   │
│  Metrics: /metrics (Prometheus)                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │      ruvector-service         │
              │   (Google SQL - Postgres)     │
              │   DecisionEvent Persistence   │
              └───────────────────────────────┘
```

## 5. Port Mappings

| Port | Protocol | Purpose | Cloud Run Behavior |
|------|----------|---------|-------------------|
| 8080 | HTTP | Primary API | Mapped to HTTPS automatically |
| 9000 | gRPC | gRPC API | Requires `--use-http2` flag |
| 9090 | HTTP | Prometheus metrics | Internal only, scraped by monitoring |

**Note:** Cloud Run maps container port 8080 to HTTPS by default. gRPC and metrics ports are configured via Cloud Run port configuration.

## 6. Scaling Configuration

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Min Instances | 1 | Always warm for incident response |
| Max Instances | 10 | Cost control, horizontal scaling |
| CPU | 2 vCPU | ML model processing |
| Memory | 2Gi | In-memory state + ML models |
| Request Timeout | 300s | Long-running post-mortem generation |
| Concurrency | 80 | Requests per instance |

## 7. Networking

| Configuration | Value |
|--------------|-------|
| Ingress | `internal-and-cloud-load-balancing` |
| VPC Connector | Required for ruvector-service access |
| Egress | All traffic through VPC connector |

**CRITICAL:** LLM-Incident-Manager receives invocations from LLM-Orchestrator only. It does NOT receive external traffic directly.
