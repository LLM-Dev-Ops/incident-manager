#!/bin/bash
# Example: List incidents with filtering

ENDPOINT="${LLM_IM_ENDPOINT:-http://localhost:8080}"

echo "=== Active Critical Incidents ==="
curl -s "${ENDPOINT}/v1/incidents?active_only=true" | jq '.incidents[] | select(.severity == "P0" or .severity == "P1")'

echo ""
echo "=== All Incidents (Paginated) ==="
curl -s "${ENDPOINT}/v1/incidents?page=0&page_size=10" | jq '.'

echo ""
echo "=== Incident Count ==="
curl -s "${ENDPOINT}/v1/incidents?page=0&page_size=1" | jq '.total'
