#!/bin/bash
# Example: Submit an alert to LLM Incident Manager

ENDPOINT="${LLM_IM_ENDPOINT:-http://localhost:8080}"

curl -X POST "${ENDPOINT}/v1/alerts" \
  -H "Content-Type: application/json" \
  -d '{
    "source": "llm-sentinel",
    "title": "High API Latency Detected",
    "description": "P95 latency exceeded 5 seconds for api-service",
    "severity": "P1",
    "alert_type": "Performance",
    "labels": {
      "environment": "production",
      "region": "us-east-1",
      "service": "api-service"
    },
    "affected_services": ["api-service", "user-service"],
    "runbook_url": "https://runbooks.example.com/latency"
  }' | jq .

echo ""
echo "Alert submitted successfully!"
