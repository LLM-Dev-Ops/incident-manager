#!/bin/bash
# Example: List incidents via gRPC using grpcurl

GRPC_ENDPOINT="${GRPC_ENDPOINT:-localhost:9000}"

echo "Listing incidents from gRPC endpoint: $GRPC_ENDPOINT"

grpcurl -plaintext \
  -d '{
    "page": 0,
    "page_size": 20,
    "states": [1, 2, 3],
    "severities": [1, 2]
  }' \
  "$GRPC_ENDPOINT" \
  incidents.IncidentService/ListIncidents | jq .

echo ""
