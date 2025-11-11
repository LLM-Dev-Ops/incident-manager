#!/bin/bash
# Example: Submit an alert via gRPC using grpcurl
# Install grpcurl: https://github.com/fullstorydev/grpcurl

GRPC_ENDPOINT="${GRPC_ENDPOINT:-localhost:9000}"

echo "Submitting alert to gRPC endpoint: $GRPC_ENDPOINT"

grpcurl -plaintext \
  -d '{
    "alert_id": "grpc-alert-123",
    "source": "llm-sentinel",
    "timestamp": {
      "seconds": '"$(date +%s)"'
    },
    "severity": 2,
    "type": 5,
    "title": "High API Latency",
    "description": "P95 latency exceeded threshold",
    "labels": {
      "environment": "production",
      "service": "api-gateway"
    },
    "affected_services": ["api-gateway", "backend-api"],
    "runbook_url": "https://runbooks.example.com/latency"
  }' \
  "$GRPC_ENDPOINT" \
  alerts.AlertIngestion/SubmitAlert

echo ""
echo "Alert submitted successfully via gRPC!"
