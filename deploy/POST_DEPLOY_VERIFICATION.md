# Post-Deploy Verification Checklist

## 1. Service Health Verification

### 1.1 Service Is Live

```bash
# Check service exists and is serving
gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --format='value(status.conditions[0].status)'

# Expected: True
```

**Checklist:**
- [ ] Service status shows `Ready: True`
- [ ] At least 1 instance is running
- [ ] No recent failed revisions

### 1.2 All Agent Endpoints Respond

```bash
# Get auth token
TOKEN=$(gcloud auth print-identity-token)
SERVICE_URL=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --format='value(status.url)')

# Test each endpoint
curl -s -w "\n%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/health"

curl -s -w "\n%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/health/ready"

curl -s -w "\n%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/v1/incidents?limit=1"
```

**Checklist:**
- [ ] `/health` returns 200 OK
- [ ] `/health/ready` returns 200 OK
- [ ] `/health/live` returns 200 OK
- [ ] `/v1/incidents` returns 200 OK (may be empty list)
- [ ] `/v1/postmortems` returns 200 OK (may be empty list)

## 2. Agent Functionality Verification

### 2.1 Escalation Agent

```bash
# Test escalation evaluation (dry run with test incident)
curl -X POST \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  "${SERVICE_URL}/v1/agents/escalation/evaluate" \
  -d '{
    "incident_id": "test-verification-001",
    "current_severity": "P3",
    "current_escalation_level": 0,
    "incident_title": "Verification Test Incident",
    "signal_source": "manual_test",
    "signals": [],
    "execution_id": "exec-test-001"
  }'
```

**Checklist:**
- [ ] Escalation evaluation returns DecisionEvent
- [ ] DecisionEvent has valid `decision_type: incident_escalation_decision`
- [ ] DecisionEvent has valid `confidence` between 0-1
- [ ] DecisionEvent has `inputs_hash` (SHA256)
- [ ] Response includes `orchestrator_actions` array

### 2.2 Human Approval Gates

```bash
# Test approval request creation
curl -X POST \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  "${SERVICE_URL}/v1/agents/hitl/request" \
  -d '{
    "incident_id": "test-verification-001",
    "action_type": "remediation",
    "action_description": "Verification test - rollback deployment",
    "requester_type": "system",
    "requester_id": "verification-test",
    "min_approvals_required": 1,
    "required_approvers": [{"approver_type": "incident_commander", "required": true}],
    "approval_deadline": "2099-12-31T23:59:59Z",
    "current_status": "pending"
  }'
```

**Checklist:**
- [ ] Approval request creation returns approval_request_id
- [ ] Status check returns current approval state
- [ ] Approve/reject workflow completes successfully
- [ ] DecisionEvent emitted for approval decisions

### 2.3 Post-Mortem Generation

```bash
# Create a test resolved incident first, then:
curl -X POST \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/v1/postmortems/generate/test-resolved-incident"
```

**Checklist:**
- [ ] Post-mortem generation returns postmortem_id
- [ ] Post-mortem contains required sections (timeline, root_cause, impact)
- [ ] Post-mortem can be retrieved by ID
- [ ] Post-mortem can be published (made immutable)
- [ ] DecisionEvent emitted for post-mortem generation

## 3. Persistence Verification

### 3.1 DecisionEvents in ruvector-service

```bash
# Verify DecisionEvents are being persisted
# (Requires access to ruvector-service or its backing store)

# Via ruvector API (if accessible)
curl -H "Authorization: Bearer ${RUVECTOR_TOKEN}" \
  "https://ruvector-service-${ENV}.run.app/v1/decisions?agent_id=incident-escalation-agent&limit=5"
```

**Checklist:**
- [ ] DecisionEvents appear in ruvector-service
- [ ] Events have correct `agent_id` and `agent_version`
- [ ] Events have valid `inputs_hash`
- [ ] Events have `timestamp` in ISO8601 format
- [ ] Events are queryable by `incident_id`

### 3.2 No Direct SQL Access

**Verification Method:** Code audit + runtime verification

```bash
# Check no SQL connections in logs
gcloud logging read \
  "resource.type=cloud_run_revision AND \
   resource.labels.service_name=llm-incident-manager AND \
   (textPayload:\"postgres\" OR textPayload:\"SQL\" OR textPayload:\"database connection\")" \
  --limit=50

# Expected: No results (or only ruvector-client logs)
```

**Checklist:**
- [ ] No `postgres://` connection strings in logs
- [ ] No SQL query logging
- [ ] All persistence goes through ruvector-service
- [ ] `RUVECTOR_SERVICE_URL` is used for persistence

## 4. Telemetry Verification

### 4.1 Telemetry in LLM-Observatory

```bash
# Check for telemetry events
# (Requires access to LLM-Observatory)

# Via Observatory API
curl -H "Authorization: Bearer ${OBSERVATORY_TOKEN}" \
  "https://llm-observatory-${ENV}.run.app/v1/events?service=llm-incident-manager&limit=10"
```

**Checklist:**
- [ ] Telemetry events appear in Observatory
- [ ] Events have correct `service_name`
- [ ] Events have `execution_id` for correlation
- [ ] Metrics are being exported (check Prometheus endpoint)

### 4.2 Prometheus Metrics

```bash
# Note: Metrics port may only be accessible internally
# Test via port-forward or internal network

curl "${SERVICE_URL}/metrics" | head -50
```

**Checklist:**
- [ ] Prometheus metrics endpoint responds
- [ ] Custom metrics (incident counts, latency) present
- [ ] No metric export errors in logs

## 5. Integration Verification

### 5.1 CLI Commands Function

```bash
# Test CLI end-to-end
export LLM_IM_ENDPOINT="${SERVICE_URL}"

# Health check
llm-im-cli health

# List incidents
llm-im-cli list --limit 5

# List post-mortems
llm-im-cli postmortem list --limit 5
```

**Checklist:**
- [ ] `llm-im-cli health` returns healthy status
- [ ] `llm-im-cli list` returns incident list
- [ ] `llm-im-cli postmortem list` returns post-mortem list
- [ ] CLI respects `--endpoint` flag
- [ ] CLI respects environment variable configuration

### 5.2 Agent Contract Compliance

**Checklist:**
- [ ] Escalation Agent outputs match `EscalationAgentOutput` schema
- [ ] HITL Agent outputs match `HITLAgentOutput` schema
- [ ] Post-Mortem outputs match contract schema
- [ ] All DecisionEvents pass `validateDecisionEvent()`

## 6. Security Verification

### 6.1 Authentication Required

```bash
# Test without auth (should fail)
curl -s -w "\n%{http_code}" "${SERVICE_URL}/health"
# Expected: 403 or 401

# Test with invalid token (should fail)
curl -s -w "\n%{http_code}" \
  -H "Authorization: Bearer invalid-token" \
  "${SERVICE_URL}/health"
# Expected: 401 or 403
```

**Checklist:**
- [ ] Unauthenticated requests are rejected
- [ ] Invalid tokens are rejected
- [ ] Only valid GCP identity tokens accepted
- [ ] Service account has minimal required permissions

### 6.2 Secrets Are Not Exposed

```bash
# Check logs don't contain secrets
gcloud logging read \
  "resource.type=cloud_run_revision AND \
   resource.labels.service_name=llm-incident-manager AND \
   (textPayload:\"api_key\" OR textPayload:\"RUVECTOR_API_KEY\" OR textPayload:\"secret\")" \
  --limit=50

# Expected: No secret values in logs
```

**Checklist:**
- [ ] No API keys in logs
- [ ] No secrets in environment variable dumps
- [ ] Secrets resolved from Secret Manager

## 7. Architectural Compliance

### 7.1 No Direct SQL Access

**Checklist:**
- [ ] No `sqlx`, `diesel`, `postgres` in dependencies
- [ ] No SQL strings in source code
- [ ] All persistence via ruvector-service client

### 7.2 No Agent Bypasses

**Checklist:**
- [ ] All DecisionEvents use `@agentics/contracts` schemas
- [ ] All persistence uses `@agentics/ruvector-client`
- [ ] No hardcoded URLs or credentials

## 8. Summary Verification Script

Save as `deploy/verify-deployment.sh`:

```bash
#!/bin/bash
set -e

REGION=${REGION:-us-central1}
ENV=${ENV:-dev}

echo "=== LLM-Incident-Manager Post-Deploy Verification ==="
echo "Environment: ${ENV}"
echo "Region: ${REGION}"
echo ""

# Get service URL
SERVICE_URL=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --format='value(status.url)')
echo "Service URL: ${SERVICE_URL}"

# Get auth token
TOKEN=$(gcloud auth print-identity-token)

# 1. Health checks
echo ""
echo "=== Health Checks ==="

echo -n "Health endpoint: "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/health")
[ "$HTTP_CODE" -eq 200 ] && echo "PASS (${HTTP_CODE})" || echo "FAIL (${HTTP_CODE})"

echo -n "Readiness endpoint: "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/health/ready")
[ "$HTTP_CODE" -eq 200 ] && echo "PASS (${HTTP_CODE})" || echo "FAIL (${HTTP_CODE})"

echo -n "Liveness endpoint: "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/health/live")
[ "$HTTP_CODE" -eq 200 ] && echo "PASS (${HTTP_CODE})" || echo "FAIL (${HTTP_CODE})"

# 2. API endpoints
echo ""
echo "=== API Endpoints ==="

echo -n "Incidents list: "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/v1/incidents?limit=1")
[ "$HTTP_CODE" -eq 200 ] && echo "PASS (${HTTP_CODE})" || echo "FAIL (${HTTP_CODE})"

echo -n "Post-mortems list: "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/v1/postmortems?limit=1")
[ "$HTTP_CODE" -eq 200 ] && echo "PASS (${HTTP_CODE})" || echo "FAIL (${HTTP_CODE})"

# 3. Security
echo ""
echo "=== Security Checks ==="

echo -n "Unauthenticated blocked: "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  "${SERVICE_URL}/health")
[ "$HTTP_CODE" -eq 403 ] || [ "$HTTP_CODE" -eq 401 ] && echo "PASS (${HTTP_CODE})" || echo "FAIL (${HTTP_CODE})"

echo ""
echo "=== Verification Complete ==="
```

**Run verification:**

```bash
chmod +x deploy/verify-deployment.sh
ENV=prod REGION=us-central1 ./deploy/verify-deployment.sh
```
