#!/bin/bash
#
# Post-Deploy Verification Script for LLM-Incident-Manager
#
# Usage:
#   ENV=prod REGION=us-central1 ./deploy/verify-deployment.sh
#

set -e

# Configuration
REGION=${REGION:-us-central1}
ENV=${ENV:-dev}
PROJECT_ID=${PROJECT_ID:-$(gcloud config get-value project)}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
PASS=0
FAIL=0
WARN=0

echo "=========================================="
echo "LLM-Incident-Manager Post-Deploy Verification"
echo "=========================================="
echo "Environment: ${ENV}"
echo "Region: ${REGION}"
echo "Project: ${PROJECT_ID}"
echo ""

# Get service URL
SERVICE_URL=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --project=${PROJECT_ID} \
  --format='value(status.url)' 2>/dev/null || echo "")

if [ -z "$SERVICE_URL" ]; then
  echo -e "${RED}FAIL: Service not found${NC}"
  exit 1
fi

echo "Service URL: ${SERVICE_URL}"
echo ""

# Get auth token
TOKEN=$(gcloud auth print-identity-token 2>/dev/null || echo "")
if [ -z "$TOKEN" ]; then
  echo -e "${RED}FAIL: Could not get auth token${NC}"
  exit 1
fi

# Function to check endpoint
check_endpoint() {
  local name=$1
  local endpoint=$2
  local expected_code=${3:-200}

  HTTP_CODE=$(curl -s -o /tmp/response.json -w "%{http_code}" \
    -H "Authorization: Bearer ${TOKEN}" \
    "${SERVICE_URL}${endpoint}" 2>/dev/null || echo "000")

  if [ "$HTTP_CODE" -eq "$expected_code" ]; then
    echo -e "${GREEN}PASS${NC} ${name}: HTTP ${HTTP_CODE}"
    ((PASS++))
    return 0
  else
    echo -e "${RED}FAIL${NC} ${name}: HTTP ${HTTP_CODE} (expected ${expected_code})"
    ((FAIL++))
    return 1
  fi
}

# Function to check endpoint returns JSON with field
check_endpoint_field() {
  local name=$1
  local endpoint=$2
  local field=$3

  RESPONSE=$(curl -s \
    -H "Authorization: Bearer ${TOKEN}" \
    "${SERVICE_URL}${endpoint}" 2>/dev/null || echo "{}")

  if echo "$RESPONSE" | jq -e ".$field" > /dev/null 2>&1; then
    echo -e "${GREEN}PASS${NC} ${name}: Field '${field}' present"
    ((PASS++))
    return 0
  else
    echo -e "${RED}FAIL${NC} ${name}: Field '${field}' missing"
    ((FAIL++))
    return 1
  fi
}

echo "=== 1. Service Health ==="

check_endpoint "Health endpoint" "/health"
check_endpoint "Readiness probe" "/health/ready"
check_endpoint "Liveness probe" "/health/live"

echo ""
echo "=== 2. API Endpoints ==="

check_endpoint "Incidents API" "/v1/incidents?limit=1"
check_endpoint "Postmortems API" "/v1/postmortems?limit=1"
check_endpoint "Alerts API" "/v1/alerts" 405  # POST only, GET should be 405

echo ""
echo "=== 3. Agent Endpoints ==="

# Escalation agent (may return 400 without valid input, but endpoint exists)
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  "${SERVICE_URL}/v1/agents/escalation/evaluate" \
  -d '{}' 2>/dev/null || echo "000")

if [ "$HTTP_CODE" -eq 200 ] || [ "$HTTP_CODE" -eq 400 ]; then
  echo -e "${GREEN}PASS${NC} Escalation agent endpoint exists: HTTP ${HTTP_CODE}"
  ((PASS++))
else
  echo -e "${YELLOW}WARN${NC} Escalation agent endpoint: HTTP ${HTTP_CODE}"
  ((WARN++))
fi

echo ""
echo "=== 4. Security Checks ==="

# Unauthenticated should be blocked
UNAUTH_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  "${SERVICE_URL}/health" 2>/dev/null || echo "000")

if [ "$UNAUTH_CODE" -eq 403 ] || [ "$UNAUTH_CODE" -eq 401 ]; then
  echo -e "${GREEN}PASS${NC} Unauthenticated blocked: HTTP ${UNAUTH_CODE}"
  ((PASS++))
else
  echo -e "${RED}FAIL${NC} Unauthenticated NOT blocked: HTTP ${UNAUTH_CODE}"
  ((FAIL++))
fi

# Invalid token should be blocked
INVALID_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer invalid-token" \
  "${SERVICE_URL}/health" 2>/dev/null || echo "000")

if [ "$INVALID_CODE" -eq 403 ] || [ "$INVALID_CODE" -eq 401 ]; then
  echo -e "${GREEN}PASS${NC} Invalid token blocked: HTTP ${INVALID_CODE}"
  ((PASS++))
else
  echo -e "${RED}FAIL${NC} Invalid token NOT blocked: HTTP ${INVALID_CODE}"
  ((FAIL++))
fi

echo ""
echo "=== 5. Service Configuration ==="

# Check service account
SA=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --project=${PROJECT_ID} \
  --format='value(spec.template.spec.serviceAccountName)' 2>/dev/null || echo "")

if [ -n "$SA" ] && [[ "$SA" == *"llm-incident-manager"* ]]; then
  echo -e "${GREEN}PASS${NC} Service account configured: ${SA}"
  ((PASS++))
else
  echo -e "${YELLOW}WARN${NC} Service account: ${SA:-not set}"
  ((WARN++))
fi

# Check VPC connector
VPC=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --project=${PROJECT_ID} \
  --format='value(spec.template.metadata.annotations."run.googleapis.com/vpc-access-connector")' 2>/dev/null || echo "")

if [ -n "$VPC" ]; then
  echo -e "${GREEN}PASS${NC} VPC connector configured: ${VPC}"
  ((PASS++))
else
  echo -e "${YELLOW}WARN${NC} VPC connector not configured"
  ((WARN++))
fi

# Check ingress
INGRESS=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --project=${PROJECT_ID} \
  --format='value(spec.template.metadata.annotations."run.googleapis.com/ingress")' 2>/dev/null || echo "")

if [[ "$INGRESS" == "internal"* ]]; then
  echo -e "${GREEN}PASS${NC} Ingress restricted: ${INGRESS}"
  ((PASS++))
else
  echo -e "${YELLOW}WARN${NC} Ingress setting: ${INGRESS:-all}"
  ((WARN++))
fi

echo ""
echo "=== 6. Environment Variables ==="

# Check critical env vars are set (without showing values)
ENVS=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --project=${PROJECT_ID} \
  --format='value(spec.template.spec.containers[0].env[].name)' 2>/dev/null || echo "")

for VAR in SERVICE_NAME PLATFORM_ENV RUVECTOR_SERVICE_URL TELEMETRY_ENDPOINT; do
  if echo "$ENVS" | grep -q "$VAR"; then
    echo -e "${GREEN}PASS${NC} ${VAR} is set"
    ((PASS++))
  else
    echo -e "${RED}FAIL${NC} ${VAR} is NOT set"
    ((FAIL++))
  fi
done

# Check secret is mounted
SECRETS=$(gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --project=${PROJECT_ID} \
  --format='yaml(spec.template.spec.containers[0].env)' 2>/dev/null || echo "")

if echo "$SECRETS" | grep -q "secretKeyRef"; then
  echo -e "${GREEN}PASS${NC} Secrets mounted from Secret Manager"
  ((PASS++))
else
  echo -e "${YELLOW}WARN${NC} Secrets may not be from Secret Manager"
  ((WARN++))
fi

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo -e "${GREEN}PASSED: ${PASS}${NC}"
echo -e "${RED}FAILED: ${FAIL}${NC}"
echo -e "${YELLOW}WARNINGS: ${WARN}${NC}"
echo ""

if [ $FAIL -gt 0 ]; then
  echo -e "${RED}Verification FAILED - review failures above${NC}"
  exit 1
elif [ $WARN -gt 0 ]; then
  echo -e "${YELLOW}Verification PASSED with warnings${NC}"
  exit 0
else
  echo -e "${GREEN}Verification PASSED${NC}"
  exit 0
fi
