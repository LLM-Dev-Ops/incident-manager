#!/bin/bash
# Example: Resolve an incident

if [ -z "$1" ]; then
  echo "Usage: $0 <incident-id>"
  exit 1
fi

INCIDENT_ID="$1"
ENDPOINT="${LLM_IM_ENDPOINT:-http://localhost:8080}"

curl -X POST "${ENDPOINT}/v1/incidents/${INCIDENT_ID}/resolve" \
  -H "Content-Type: application/json" \
  -d '{
    "resolved_by": "operator@example.com",
    "method": "Manual",
    "notes": "Restarted affected services after identifying memory leak. Deployed hotfix v2.3.1.",
    "root_cause": "Memory leak in cache management introduced in v2.3.0 deployment"
  }' | jq .

echo ""
echo "Incident ${INCIDENT_ID} resolved!"
