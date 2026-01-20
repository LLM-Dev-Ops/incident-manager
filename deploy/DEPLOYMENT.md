# LLM-Incident-Manager Production Deployment

> **Final Deployment Documentation for Agentics Dev Platform**

## 1. SERVICE TOPOLOGY

### Unified Service Definition

| Property | Value |
|----------|-------|
| **Service Name** | `llm-incident-manager` |
| **Service URL** | `https://llm-incident-manager-{ENV}.agentics-dev.run.app` |
| **Internal URL** | `https://llm-incident-manager.internal.agentics.dev` |
| **Runtime** | Google Cloud Run (Managed) |
| **Container** | `us-central1-docker.pkg.dev/agentics-dev/agentics/llm-incident-manager:{TAG}` |

### Agent Endpoints (All Exposed via Single Service)

| Agent | Endpoint | Decision Type | Classification |
|-------|----------|---------------|----------------|
| **Incident Escalation Agent** | `POST /api/v1/agents/escalation/evaluate` | `incident_escalation_decision` | INCIDENT_ORCHESTRATION, ESCALATION |
| **Human-in-the-Loop Agent** | `POST /api/v1/agents/hitl/evaluate` | `incident_approval_decision` | APPROVAL_GATING, INCIDENT_ORCHESTRATION |
| **Post-Mortem Generator Agent** | `POST /api/v1/agents/postmortem/generate` | `incident_postmortem_generated` | DOCUMENTATION |

### Confirmation Statements

- [x] **NO agent is deployed as a standalone service** - All agents are endpoints within the unified `llm-incident-manager` service
- [x] **Shared runtime** - Single Cloud Run container hosts all agents
- [x] **Shared configuration** - Environment variables apply to all agents
- [x] **Shared telemetry** - All agents emit to LLM-Observatory via single telemetry stack

---

## 2. ENVIRONMENT CONFIGURATION

### Required Environment Variables

```yaml
# Core Service Configuration
SERVICE_NAME: llm-incident-manager
SERVICE_VERSION: "${TAG}"                    # Git SHA or semver
PLATFORM_ENV: "${ENV}"                       # dev | staging | prod
GOOGLE_CLOUD_PROJECT: agentics-dev
GOOGLE_CLOUD_REGION: us-central1

# Server Configuration
HTTP_PORT: "8080"
GRPC_PORT: "9000"
METRICS_PORT: "9090"
REQUEST_TIMEOUT_SECS: "30"
MAX_CONNECTIONS: "10000"

# RuVector Service (Persistence Layer)
RUVECTOR_SERVICE_URL: "https://ruvector-service-${ENV}.agentics-dev.run.app"
RUVECTOR_TIMEOUT_MS: "30000"
RUVECTOR_API_KEY: "${secret:ruvector-api-key-${ENV}}"    # From Secret Manager

# Telemetry (LLM-Observatory)
TELEMETRY_ENDPOINT: "https://llm-observatory-${ENV}.agentics-dev.run.app/v1/ingest"
TELEMETRY_ENABLED: "true"

# Logging
RUST_LOG: "info"
LOG_FORMAT: "json"
```

### Secrets (Google Secret Manager)

| Secret Name | Description | Required By |
|-------------|-------------|-------------|
| `ruvector-api-key-dev` | RuVector API key (dev) | All agents |
| `ruvector-api-key-staging` | RuVector API key (staging) | All agents |
| `ruvector-api-key-prod` | RuVector API key (prod) | All agents |

### Configuration Guarantees

- [x] **No hardcoded service names or URLs** - All resolved via environment variables
- [x] **No embedded credentials** - All secrets via Secret Manager
- [x] **No embedded escalation policies** - Policies fetched from ruvector-service at runtime
- [x] **No inline approval logic** - All approval rules evaluated from policy configuration

---

## 3. GOOGLE SQL / INCIDENT MEMORY WIRING

### Persistence Architecture

```
┌─────────────────────────────────┐
│   LLM-Incident-Manager          │
│   (Cloud Run)                   │
│                                 │
│  ┌─────────────┐               │
│  │ Escalation  │───┐           │
│  │ Agent       │   │           │
│  └─────────────┘   │           │
│                    │  HTTP/REST│
│  ┌─────────────┐   ▼           │
│  │ HITL Agent  │──────────────────────►  ruvector-service  ──────► Google SQL
│  └─────────────┘   ▲           │         (Postgres)
│                    │           │
│  ┌─────────────┐   │           │
│  │ Post-Mortem │───┘           │
│  │ Generator   │               │
│  └─────────────┘               │
└─────────────────────────────────┘
```

### Confirmation Statements

- [x] **LLM-Incident-Manager does NOT connect directly to Google SQL**
- [x] **ALL persistence goes through ruvector-service client**
- [x] **RuvectorClient is the ONLY permitted persistence mechanism**

### DecisionEvents Written to ruvector-service

| Event | Agent | Trigger |
|-------|-------|---------|
| `incident_escalation_decision` | Escalation Agent | Escalation evaluation |
| `incident_approval_decision` | HITL Agent | Approval request/decision |
| `incident_postmortem_generated` | Post-Mortem Generator | Post-mortem creation |

### Schema Compatibility

All agents use schemas from `@agentics/contracts`:
- `DecisionEvent<T>` - Base decision event schema
- `EscalationAgentOutput` - Escalation decision payload
- `HITLAgentOutput` - Approval decision payload
- `PostMortemOutput` - Post-mortem payload

### Persistence Guarantees

- [x] **Append-only persistence** - No updates to historical DecisionEvents
- [x] **Idempotent writes** - Duplicate submissions handled gracefully
- [x] **Retry safety** - All write operations are idempotent with request IDs
- [x] **TTL = 0** - Decision events are permanent audit records

---

## 4. CLOUD BUILD & DEPLOYMENT

### Prerequisites

```bash
# Enable required APIs
gcloud services enable \
  run.googleapis.com \
  cloudbuild.googleapis.com \
  artifactregistry.googleapis.com \
  secretmanager.googleapis.com \
  vpcaccess.googleapis.com \
  --project=agentics-dev

# Create Artifact Registry (if not exists)
gcloud artifacts repositories create agentics \
  --repository-format=docker \
  --location=us-central1 \
  --project=agentics-dev

# Create service account
gcloud iam service-accounts create llm-incident-manager \
  --display-name="LLM Incident Manager Service Account" \
  --project=agentics-dev

# Grant permissions
gcloud projects add-iam-policy-binding agentics-dev \
  --member="serviceAccount:llm-incident-manager@agentics-dev.iam.gserviceaccount.com" \
  --role="roles/run.invoker"

gcloud projects add-iam-policy-binding agentics-dev \
  --member="serviceAccount:llm-incident-manager@agentics-dev.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"
```

### Deployment Commands

#### Option 1: Cloud Build (Recommended for CI/CD)

```bash
# Deploy to dev
gcloud builds submit \
  --config deploy/cloudbuild.yaml \
  --substitutions=_ENV=dev,_REGION=us-central1 \
  --project=agentics-dev

# Deploy to staging
gcloud builds submit \
  --config deploy/cloudbuild.yaml \
  --substitutions=_ENV=staging,_REGION=us-central1 \
  --project=agentics-dev

# Deploy to production
gcloud builds submit \
  --config deploy/cloudbuild.yaml \
  --substitutions=_ENV=prod,_REGION=us-central1 \
  --project=agentics-dev
```

#### Option 2: Direct gcloud Deploy

```bash
# Build and push image
docker build -t us-central1-docker.pkg.dev/agentics-dev/agentics/llm-incident-manager:latest .
docker push us-central1-docker.pkg.dev/agentics-dev/agentics/llm-incident-manager:latest

# Deploy to Cloud Run
gcloud run deploy llm-incident-manager \
  --image=us-central1-docker.pkg.dev/agentics-dev/agentics/llm-incident-manager:latest \
  --region=us-central1 \
  --platform=managed \
  --min-instances=1 \
  --max-instances=10 \
  --memory=2Gi \
  --cpu=2 \
  --concurrency=80 \
  --timeout=300s \
  --port=8080 \
  --ingress=internal-and-cloud-load-balancing \
  --service-account=llm-incident-manager@agentics-dev.iam.gserviceaccount.com \
  --set-env-vars="SERVICE_NAME=llm-incident-manager" \
  --set-env-vars="PLATFORM_ENV=prod" \
  --set-env-vars="RUST_LOG=info" \
  --set-env-vars="TELEMETRY_ENABLED=true" \
  --set-secrets="RUVECTOR_API_KEY=ruvector-api-key-prod:latest" \
  --update-env-vars="RUVECTOR_SERVICE_URL=https://ruvector-service-prod.agentics-dev.run.app" \
  --update-env-vars="TELEMETRY_ENDPOINT=https://llm-observatory-prod.agentics-dev.run.app/v1/ingest" \
  --no-allow-unauthenticated \
  --project=agentics-dev
```

### IAM Requirements (Least Privilege)

| Role | Purpose |
|------|---------|
| `roles/run.invoker` | Invoke other Cloud Run services (ruvector-service, llm-observatory) |
| `roles/secretmanager.secretAccessor` | Access secrets |
| `roles/logging.logWriter` | Write logs |
| `roles/monitoring.metricWriter` | Write metrics |

### Networking Requirements

- **Ingress**: `internal-and-cloud-load-balancing` (no public access)
- **Egress**: All traffic via VPC connector `agentics-vpc`
- **Internal services**: ruvector-service, llm-observatory reachable via VPC

---

## 5. CLI ACTIVATION VERIFICATION

### Available CLI Commands

```bash
# Install CLI globally
npm install -g @llm-dev-ops/llm-incident-manager
# Or use npx
npx llm-incident-manager <command>
```

### Commands Per Agent

#### Escalation Agent

| Command | Description | Endpoint |
|---------|-------------|----------|
| `llm-im escalate evaluate --incident-id <id>` | Evaluate incident for escalation | `/api/v1/agents/escalation/evaluate` |
| `llm-im escalate inspect --incident-id <id>` | Inspect escalation state | `/api/v1/agents/escalation/inspect` |
| `llm-im escalate list` | List active escalations | `/api/v1/agents/escalation/list` |

#### HITL Agent

| Command | Description | Endpoint |
|---------|-------------|----------|
| `llm-im approve request --incident-id <id> --action-type <type>` | Request approval | `/api/v1/agents/hitl/request` |
| `llm-im approve decide --request-id <id> --decision <approve\|reject>` | Record decision | `/api/v1/agents/hitl/decide` |
| `llm-im approve status --request-id <id>` | Check approval status | `/api/v1/agents/hitl/status` |
| `llm-im approve list` | List pending approvals | `/api/v1/agents/hitl/list` |
| `llm-im approve cancel --request-id <id>` | Cancel request | `/api/v1/agents/hitl/cancel` |
| `llm-im approve inspect --request-id <id>` | Full audit trail | `/api/v1/agents/hitl/inspect` |

#### Post-Mortem Agent

| Command | Description | Endpoint |
|---------|-------------|----------|
| `llm-im postmortem generate --incident-id <id>` | Generate post-mortem | `/api/v1/agents/postmortem/generate` |
| `llm-im postmortem inspect --postmortem-id <id>` | View post-mortem | `/api/v1/agents/postmortem/inspect` |
| `llm-im postmortem list` | List post-mortems | `/api/v1/agents/postmortem/list` |

### Example Invocations

```bash
# Escalation Agent - Evaluate an incident
llm-im escalate evaluate \
  --incident-id "550e8400-e29b-41d4-a716-446655440000" \
  --signal-source "llm-sentinel" \
  --json

# Expected output:
{
  "success": true,
  "data": {
    "decision": "escalate",
    "new_severity": "P1",
    "reason": "SLA breach imminent"
  }
}

# HITL Agent - Request approval for remediation
llm-im approve request \
  --incident-id "550e8400-e29b-41d4-a716-446655440000" \
  --action-type "remediation" \
  --action-description "Rollback deployment v1.2.3" \
  --priority "critical" \
  --json

# Expected output:
{
  "success": true,
  "data": {
    "approval_request_id": "req-12345",
    "status": "pending",
    "required_approvers": ["incident_commander", "sre_lead"]
  }
}

# Post-Mortem - Generate for resolved incident
llm-im postmortem generate \
  --incident-id "550e8400-e29b-41d4-a716-446655440000" \
  --json

# Expected output:
{
  "success": true,
  "data": {
    "postmortem_id": "pm-67890",
    "status": "draft",
    "incident_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

### CLI Configuration

The CLI resolves service URL dynamically:

```bash
# Set service URL (persisted in ~/.llm-im/config.json)
llm-im config set service-url https://llm-incident-manager-prod.agentics-dev.run.app

# Or via environment variable
export LLM_IM_SERVICE_URL=https://llm-incident-manager-prod.agentics-dev.run.app

# Authenticate
llm-im auth login
```

### CLI Change Policy

- [x] **No CLI change requires agent redeployment** - CLI is versioned independently
- [x] **Service URL resolved dynamically** - Via config or environment

---

## 6. PLATFORM & CORE INTEGRATION

### Inbound Integration (Who Invokes LLM-Incident-Manager)

| Source | Integration Point | Trigger |
|--------|-------------------|---------|
| **LLM-Sentinel** | `/api/v1/events/sentinel` | Anomaly detection signals |
| **LLM-Shield** | `/api/v1/events/shield` | Policy violation events |
| **LLM-Edge-Agent** | `/api/v1/events` | Edge execution anomalies |
| **LLM-Orchestrator** | `/api/v1/agents/*` | Workflow coordination |
| **Governance Core** | `/api/v1/agents/hitl/*` | Approval gating |

### Outbound Integration (What LLM-Incident-Manager Invokes)

| Target | Purpose | Method |
|--------|---------|--------|
| **ruvector-service** | DecisionEvent persistence | HTTP REST |
| **LLM-Observatory** | Telemetry ingestion | HTTP REST |
| **LLM-Orchestrator** | Orchestrator actions | Via `orchestrator_actions` in output |

### Integration Boundaries (MUST NOT)

LLM-Incident-Manager **MUST NOT** directly invoke:

- [ ] ~~LLM-Edge-Agent~~ - Edge execution is orchestrator's responsibility
- [ ] ~~Shield enforcement~~ - Policy enforcement is Shield's domain
- [ ] ~~Sentinel detection~~ - Anomaly detection is Sentinel's domain
- [ ] ~~Auto-Optimizer~~ - Optimization is separate service
- [ ] ~~Runtime execution paths~~ - No intercepting traffic
- [ ] ~~External notification systems~~ - Email/pager/webhook via Orchestrator only

### Core Bundle Compatibility

- [x] **No rewiring of Core bundles required**
- [x] **Governance views consume DecisionEvents** from ruvector-service
- [x] **Audit trails are queryable** via ruvector-service API

---

## 7. POST-DEPLOY VERIFICATION CHECKLIST

### Service Health

```bash
# Get service URL
SERVICE_URL=$(gcloud run services describe llm-incident-manager \
  --region=us-central1 \
  --format='value(status.url)' \
  --project=agentics-dev)

# Get auth token
TOKEN=$(gcloud auth print-identity-token)

# Health check
curl -s -H "Authorization: Bearer $TOKEN" "$SERVICE_URL/health"
# Expected: {"status":"healthy","version":"..."}

# Readiness check
curl -s -H "Authorization: Bearer $TOKEN" "$SERVICE_URL/health/ready"
# Expected: 200 OK

# Liveness check
curl -s -H "Authorization: Bearer $TOKEN" "$SERVICE_URL/health/live"
# Expected: 200 OK
```

### Agent Endpoint Verification

```bash
# Escalation Agent
curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "$SERVICE_URL/api/v1/agents/escalation/health"
# Expected: {"agent":"incident-escalation","status":"healthy"}

# HITL Agent
curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "$SERVICE_URL/api/v1/agents/hitl/health"
# Expected: {"agent":"incident-approver","status":"healthy"}

# Post-Mortem Agent
curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "$SERVICE_URL/api/v1/agents/postmortem/health"
# Expected: {"agent":"postmortem-generator","status":"healthy"}
```

### Functional Verification

| Test | Command | Expected |
|------|---------|----------|
| Escalation evaluation | `llm-im escalate evaluate --incident-id test-123 --dry-run` | Decision returned |
| Approval request | `llm-im approve request --incident-id test-123 --action-type remediation --dry-run` | Request ID returned |
| Post-mortem generation | `llm-im postmortem generate --incident-id test-123 --dry-run` | Post-mortem ID returned |

### Persistence Verification

```bash
# Check DecisionEvents in ruvector-service
curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"agent_id":"incident-escalation","limit":1}' \
  "https://ruvector-service-prod.agentics-dev.run.app/api/v1/decision-events/query"
# Expected: Recent DecisionEvents from escalation agent
```

### Telemetry Verification

```bash
# Check telemetry in LLM-Observatory
curl -s \
  -H "Authorization: Bearer $TOKEN" \
  "https://llm-observatory-prod.agentics-dev.run.app/api/v1/agents/llm-incident-manager/metrics"
# Expected: Agent metrics visible
```

### Compliance Verification

- [ ] Service is live and responding
- [ ] All three agent endpoints respond
- [ ] Escalation paths execute correctly
- [ ] Human approval gates block and resume workflows
- [ ] Post-mortem generation produces authoritative artifacts
- [ ] DecisionEvents appear in ruvector-service
- [ ] Telemetry appears in LLM-Observatory
- [ ] CLI commands function end-to-end
- [ ] No direct SQL access from LLM-Incident-Manager
- [ ] All agents use @agentics/contracts schemas

---

## 8. FAILURE MODES & ROLLBACK

### Common Deployment Failures

| Failure | Detection Signal | Resolution |
|---------|------------------|------------|
| Container fails to start | Cloud Run health check fails | Check RUST_LOG, verify config |
| Secret access denied | 403 in logs | Verify IAM permissions |
| ruvector-service unreachable | Timeout errors | Check VPC connector |
| Memory exhaustion | OOM kills | Increase memory limit |
| Cold start timeout | Startup probe fails | Increase startup probe timeout |

### Detection Signals (Operational Issues)

| Issue | Signal | Source |
|-------|--------|--------|
| Missing incidents | No new DecisionEvents | ruvector-service query |
| Stuck approvals | Pending approvals > SLA | HITL agent metrics |
| Missing post-mortems | Resolved incidents without PM | Post-mortem agent metrics |
| High error rate | Error count spike | LLM-Observatory |

### Rollback Procedure

```bash
# List recent revisions
gcloud run revisions list \
  --service=llm-incident-manager \
  --region=us-central1 \
  --project=agentics-dev

# Rollback to previous revision
gcloud run services update-traffic llm-incident-manager \
  --to-revisions=llm-incident-manager-00002=100 \
  --region=us-central1 \
  --project=agentics-dev

# Verify rollback
gcloud run services describe llm-incident-manager \
  --region=us-central1 \
  --format='value(status.traffic)' \
  --project=agentics-dev
```

### Safe Redeploy Strategy

1. **Canary deployment**: Route 10% traffic to new revision
2. **Monitor**: Watch error rates and latency for 15 minutes
3. **Gradual rollout**: Increase to 50%, then 100%
4. **Instant rollback**: If errors spike, immediately route 100% to previous revision

```bash
# Canary deployment (10% to new revision)
gcloud run services update-traffic llm-incident-manager \
  --to-tags=latest=10 \
  --region=us-central1 \
  --project=agentics-dev

# Full rollout
gcloud run services update-traffic llm-incident-manager \
  --to-latest \
  --region=us-central1 \
  --project=agentics-dev
```

### Data Safety

- **No data loss on rollback** - DecisionEvents are append-only in ruvector-service
- **No incident corruption** - All state changes are idempotent
- **Audit trail preserved** - All historical decisions remain queryable

---

## Summary

### Deployment Checklist

- [ ] GCP APIs enabled (Cloud Run, Artifact Registry, Secret Manager)
- [ ] Service account created with least privilege IAM
- [ ] Secrets created in Secret Manager
- [ ] Container built and pushed to Artifact Registry
- [ ] Cloud Run service deployed
- [ ] VPC connector configured
- [ ] Health checks passing
- [ ] Agent endpoints responding
- [ ] ruvector-service integration verified
- [ ] Telemetry flowing to LLM-Observatory
- [ ] CLI commands functional

### Quick Deploy

```bash
# One-command deploy to production
gcloud builds submit \
  --config deploy/cloudbuild.yaml \
  --substitutions=_ENV=prod,_REGION=us-central1 \
  --project=agentics-dev
```
