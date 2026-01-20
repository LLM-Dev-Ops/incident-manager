# Failure Modes & Rollback Procedures

## 1. Common Deployment Failures

### 1.1 Build Failures

| Failure | Detection | Resolution |
|---------|-----------|------------|
| Rust compilation error | Cloud Build fails at `build-rust` step | Check `cargo build` output, fix code |
| TypeScript compilation error | Cloud Build fails at `build-agents` step | Check `npm run build` output |
| Docker build failure | Cloud Build fails at `build-image` step | Check Dockerfile, dependencies |
| Image push failure | Cloud Build fails at `push-image` step | Check Artifact Registry permissions |

**Resolution Commands:**

```bash
# View build logs
gcloud builds log ${BUILD_ID}

# Retry build
gcloud builds submit --config=deploy/cloudbuild.yaml \
  --substitutions=_ENV=${ENV},_REGION=${REGION}
```

### 1.2 Deployment Failures

| Failure | Detection | Resolution |
|---------|-----------|------------|
| Service account missing | Deploy fails with IAM error | Create service account, grant roles |
| Secret not found | Deploy fails with secret error | Create secret in Secret Manager |
| VPC connector unavailable | Deploy fails with network error | Create/check VPC connector |
| Resource exhausted | Deploy fails with quota error | Request quota increase |

**Resolution Commands:**

```bash
# Check service account
gcloud iam service-accounts describe \
  llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com

# Check secret exists
gcloud secrets describe ruvector-api-key-${ENV}

# Check VPC connector
gcloud compute networks vpc-access connectors describe agentics-vpc \
  --region=${REGION}
```

### 1.3 Runtime Failures

| Failure | Detection | Resolution |
|---------|-----------|------------|
| Container crash | Revision shows `CrashLoopBackOff` | Check startup logs, env vars |
| Health check failure | Revision shows `Unhealthy` | Check `/health` endpoint, dependencies |
| Memory exhaustion | OOM kills in logs | Increase memory limit |
| Connection timeout | ruvector-service unreachable | Check VPC, service URL |

## 2. Detection Signals

### 2.1 Missing Incidents

**Signal:** New incidents not appearing in the system

**Detection:**
```bash
# Check incident count over last hour
gcloud logging read \
  "resource.type=cloud_run_revision AND \
   resource.labels.service_name=llm-incident-manager AND \
   jsonPayload.message:\"incident created\"" \
  --freshness=1h \
  | grep -c "incident created"
```

**Possible Causes:**
- Escalation Agent not receiving signals
- ruvector-service connection failure
- DecisionEvent persistence failure

**Resolution:**
1. Check Orchestrator logs for signal delivery
2. Verify ruvector-service connectivity
3. Check DecisionEvent persistence errors

### 2.2 Stuck Approvals

**Signal:** Approval requests not being processed

**Detection:**
```bash
# Check for stale pending approvals
curl -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/v1/agents/hitl/list?status=pending&created_before=$(date -d '1 hour ago' -Iseconds)"
```

**Possible Causes:**
- HITL Agent not receiving requests
- Approval notifications not sent
- Orchestrator not invoking approval workflow

**Resolution:**
1. Check HITL Agent logs
2. Verify Orchestrator is routing approval requests
3. Check notification delivery

### 2.3 Missing Post-Mortems

**Signal:** Resolved incidents not generating post-mortems

**Detection:**
```bash
# Check resolved incidents without post-mortems
curl -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/v1/incidents?status=RESOLVED&has_postmortem=false"
```

**Possible Causes:**
- Post-Mortem Agent not triggered
- Generation failing silently
- Incident not marked as resolved

**Resolution:**
1. Check Post-Mortem Agent logs
2. Manually trigger post-mortem generation
3. Verify incident resolution status

## 3. Rollback Procedure

### 3.1 Identify Current Revision

```bash
# List recent revisions
gcloud run revisions list \
  --service=llm-incident-manager \
  --region=${REGION} \
  --limit=10

# Note the current serving revision and previous stable revision
```

### 3.2 Rollback to Previous Revision

```bash
# Option 1: Route all traffic to previous revision
gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-revisions=llm-incident-manager-00002-xyz=100

# Option 2: Gradual rollback (safer for production)
gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-revisions=llm-incident-manager-00002-xyz=90,llm-incident-manager-00003-abc=10
```

### 3.3 Verify Rollback

```bash
# Check traffic routing
gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --format='yaml(spec.traffic)'

# Verify health
curl -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/health"
```

### 3.4 Full Revision Rollback Script

```bash
#!/bin/bash
set -e

REGION=${REGION:-us-central1}
SERVICE=llm-incident-manager

echo "=== Rolling back ${SERVICE} ==="

# Get current revision
CURRENT=$(gcloud run services describe ${SERVICE} \
  --region=${REGION} \
  --format='value(status.traffic[0].revisionName)')
echo "Current revision: ${CURRENT}"

# Get previous revision
PREVIOUS=$(gcloud run revisions list \
  --service=${SERVICE} \
  --region=${REGION} \
  --format='value(metadata.name)' \
  --limit=2 | tail -1)
echo "Previous revision: ${PREVIOUS}"

# Confirm rollback
read -p "Rollback from ${CURRENT} to ${PREVIOUS}? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "Rollback cancelled"
  exit 1
fi

# Execute rollback
echo "Routing 100% traffic to ${PREVIOUS}..."
gcloud run services update-traffic ${SERVICE} \
  --region=${REGION} \
  --to-revisions=${PREVIOUS}=100

# Verify
echo ""
echo "Verifying rollback..."
sleep 5

TOKEN=$(gcloud auth print-identity-token)
SERVICE_URL=$(gcloud run services describe ${SERVICE} \
  --region=${REGION} \
  --format='value(status.url)')

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/health")

if [ "$HTTP_CODE" -eq 200 ]; then
  echo "Rollback successful - health check passed"
else
  echo "WARNING: Health check returned ${HTTP_CODE}"
fi

echo ""
echo "=== Rollback Complete ==="
```

## 4. Safe Redeploy Strategy

### 4.1 Canary Deployment

Deploy new version to small percentage of traffic first:

```bash
# Deploy new revision without traffic
gcloud run deploy llm-incident-manager \
  --image=${REGION}-docker.pkg.dev/${PROJECT_ID}/agentics/llm-incident-manager:${NEW_TAG} \
  --region=${REGION} \
  --no-traffic

# Route 10% traffic to new revision
NEW_REVISION=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --format='value(status.latestCreatedRevisionName)')

gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-revisions=${NEW_REVISION}=10

# Monitor for 15 minutes, then increase
# ... monitor health, error rates, latency ...

# If healthy, increase to 50%
gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-revisions=${NEW_REVISION}=50

# If still healthy, route 100%
gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-revisions=${NEW_REVISION}=100
```

### 4.2 Blue-Green Deployment

Deploy to separate service, then switch:

```bash
# Deploy new version as separate service
gcloud run deploy llm-incident-manager-green \
  --image=${REGION}-docker.pkg.dev/${PROJECT_ID}/agentics/llm-incident-manager:${NEW_TAG} \
  --region=${REGION} \
  --no-allow-unauthenticated

# Verify green deployment
GREEN_URL=$(gcloud run services describe llm-incident-manager-green \
  --region=${REGION} \
  --format='value(status.url)')

curl -H "Authorization: Bearer ${TOKEN}" "${GREEN_URL}/health"

# If healthy, update load balancer / service mesh to route to green
# This depends on your infrastructure setup

# After verification, delete blue
gcloud run services delete llm-incident-manager-blue --region=${REGION}
```

## 5. Data Safety

### 5.1 No Data Loss Guarantee

**DecisionEvents are append-only and persisted to ruvector-service.**

- Rollback does NOT delete DecisionEvents
- Historical incident data remains intact
- Audit trail is never modified

### 5.2 Incident State Consistency

During rollback:
- In-flight requests may fail (retry safely)
- Approval requests remain in their current state
- Post-mortems remain in their current state

### 5.3 Recovery Verification

After rollback, verify:

```bash
# Check recent DecisionEvents are intact
curl -H "Authorization: Bearer ${RUVECTOR_TOKEN}" \
  "https://ruvector-service-${ENV}.run.app/v1/decisions?limit=10"

# Check incident states
curl -H "Authorization: Bearer ${TOKEN}" \
  "${SERVICE_URL}/v1/incidents?limit=10"
```

## 6. Escalation Contacts

| Level | Contact | When |
|-------|---------|------|
| L1 | On-call SRE | Deployment failures, health check failures |
| L2 | Platform Team Lead | Persistent failures after rollback |
| L3 | VP Engineering | Data loss, security incident |

## 7. Post-Incident Actions

After resolving deployment issue:

1. Create incident record in LLM-Incident-Manager
2. Document root cause
3. Generate post-mortem using `/postmortem generate`
4. Update deployment runbook if needed
5. Schedule blameless post-mortem meeting
