# gRPC API Guide - LLM Incident Manager

This guide explains how to use the gRPC API for the LLM Incident Manager.

## Overview

The LLM Incident Manager provides a production-ready gRPC API alongside the REST API, offering:

- **High Performance**: Binary protocol with efficient serialization
- **Streaming**: Real-time incident updates via server streaming
- **Strong Typing**: Protocol Buffer definitions ensure type safety
- **Bi-directional**: Client and server streaming support
- **Language Agnostic**: Generate clients in any supported language

## gRPC Services

### 1. IncidentService

Manages the full incident lifecycle.

**Endpoint**: `localhost:9000` (default)

**Methods**:
- `CreateIncident` - Create a new incident
- `GetIncident` - Retrieve incident by ID
- `ListIncidents` - List incidents with filtering
- `UpdateIncident` - Update incident state/severity
- `ResolveIncident` - Mark incident as resolved
- `StreamIncidents` - Stream real-time incident updates (server streaming)

### 2. AlertIngestion

Handles alert submission and processing.

**Endpoint**: `localhost:9000` (default)

**Methods**:
- `SubmitAlert` - Submit a single alert
- `StreamAlerts` - Submit alerts via bidirectional streaming
- `HealthCheck` - Check service health

## Protocol Buffer Definitions

Located in `proto/` directory:

- `incidents.proto` - Incident service and data models
- `alerts.proto` - Alert ingestion service
- `integrations.proto` - LLM module integrations

## Quick Start

### Using grpcurl (CLI)

Install grpcurl:
```bash
# macOS
brew install grpcurl

# Linux
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest
```

**Submit an alert**:
```bash
grpcurl -plaintext \
  -d '{
    "alert_id": "alert-001",
    "source": "llm-sentinel",
    "title": "High Latency",
    "description": "P95 > 5s",
    "severity": 2,
    "type": 5
  }' \
  localhost:9000 \
  alerts.AlertIngestion/SubmitAlert
```

**List incidents**:
```bash
grpcurl -plaintext \
  -d '{"page": 0, "page_size": 10}' \
  localhost:9000 \
  incidents.IncidentService/ListIncidents
```

**Get incident**:
```bash
grpcurl -plaintext \
  -d '{"id": "incident-uuid-here"}' \
  localhost:9000 \
  incidents.IncidentService/GetIncident
```

### Using Rust Client

Add dependencies to `Cargo.toml`:
```toml
[dependencies]
tonic = "0.11"
prost = "0.12"
tokio = { version = "1", features = ["full"] }
```

Example client code:
```rust
use llm_incident_manager::grpc::proto::incidents;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = incidents::incident_service_client::IncidentServiceClient::connect(
        "http://localhost:9000"
    ).await?;

    let request = incidents::CreateIncidentRequest {
        source: "my-app".to_string(),
        title: "Service Down".to_string(),
        description: "API is not responding".to_string(),
        severity: incidents::Severity::SeverityP0 as i32,
        r#type: incidents::IncidentType::IncidentTypeAvailability as i32,
        affected_resources: vec!["api-service".to_string()],
        labels: Default::default(),
        metadata: Default::default(),
    };

    let response = client.create_incident(request).await?;
    let incident = response.into_inner().incident.unwrap();

    println!("Created incident: {}", incident.id);

    Ok(())
}
```

See `examples/grpc/grpc_client.rs` for a complete example.

### Using Python Client

Generate Python client:
```bash
python -m grpc_tools.protoc \
  -I./proto \
  --python_out=. \
  --grpc_python_out=. \
  proto/incidents.proto proto/alerts.proto
```

Example Python code:
```python
import grpc
import incidents_pb2
import incidents_pb2_grpc

# Create channel
channel = grpc.insecure_channel('localhost:9000')
client = incidents_pb2_grpc.IncidentServiceStub(channel)

# Create incident
request = incidents_pb2.CreateIncidentRequest(
    source="python-app",
    title="Database Connection Failed",
    description="Cannot connect to PostgreSQL",
    severity=incidents_pb2.SEVERITY_P0,
    type=incidents_pb2.INCIDENT_TYPE_INFRASTRUCTURE
)

response = client.CreateIncident(request)
print(f"Created incident: {response.incident.id}")
```

### Using Go Client

Generate Go client:
```bash
protoc --go_out=. --go-grpc_out=. \
  proto/incidents.proto proto/alerts.proto
```

Example Go code:
```go
package main

import (
    "context"
    "log"

    pb "github.com/your-org/llm-incident-manager/proto"
    "google.golang.org/grpc"
)

func main() {
    conn, err := grpc.Dial("localhost:9000", grpc.WithInsecure())
    if err != nil {
        log.Fatal(err)
    }
    defer conn.Close()

    client := pb.NewIncidentServiceClient(conn)

    req := &pb.CreateIncidentRequest{
        Source: "go-app",
        Title: "Memory Leak Detected",
        Description: "Heap usage growing unbounded",
        Severity: pb.Severity_SEVERITY_P1,
        Type: pb.IncidentType_INCIDENT_TYPE_PERFORMANCE,
    }

    resp, err := client.CreateIncident(context.Background(), req)
    if err != nil {
        log.Fatal(err)
    }

    log.Printf("Created incident: %s", resp.Incident.Id)
}
```

## Streaming Examples

### Server Streaming (Incident Updates)

**Rust**:
```rust
let request = incidents::StreamIncidentsRequest {
    states: vec![],
    min_severity: vec![incidents::Severity::SeverityP0 as i32],
};

let mut stream = client.stream_incidents(request).await?.into_inner();

while let Some(event) = stream.message().await? {
    println!("Incident event: {} - {:?}",
        event.incident_id,
        event.event_type
    );
}
```

**Python**:
```python
request = incidents_pb2.StreamIncidentsRequest(
    min_severity=[incidents_pb2.SEVERITY_P0]
)

for event in client.StreamIncidents(request):
    print(f"Event: {event.incident_id} - {event.event_type}")
```

### Bidirectional Streaming (Alert Batch Upload)

**Rust**:
```rust
let (mut tx, rx) = tokio::sync::mpsc::channel(10);

// Send alerts
tokio::spawn(async move {
    for i in 0..10 {
        let alert = alerts::AlertMessage {
            alert_id: format!("alert-{}", i),
            source: "batch-upload".to_string(),
            title: format!("Alert {}", i),
            // ... other fields
        };
        tx.send(alert).await.unwrap();
    }
});

let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
let mut response_stream = client.stream_alerts(request_stream).await?.into_inner();

while let Some(ack) = response_stream.message().await? {
    println!("Alert {} processed: {}", ack.alert_id, ack.status);
}
```

## Data Types

### Severity Levels
```protobuf
enum Severity {
    SEVERITY_UNSPECIFIED = 0;
    SEVERITY_P0 = 1;  // Critical
    SEVERITY_P1 = 2;  // High
    SEVERITY_P2 = 3;  // Medium
    SEVERITY_P3 = 4;  // Low
    SEVERITY_P4 = 5;  // Informational
}
```

### Incident States
```protobuf
enum IncidentState {
    INCIDENT_STATE_UNSPECIFIED = 0;
    INCIDENT_STATE_DETECTED = 1;
    INCIDENT_STATE_TRIAGED = 2;
    INCIDENT_STATE_INVESTIGATING = 3;
    INCIDENT_STATE_REMEDIATING = 4;
    INCIDENT_STATE_RESOLVED = 5;
    INCIDENT_STATE_CLOSED = 6;
}
```

### Incident Types
```protobuf
enum IncidentType {
    INCIDENT_TYPE_UNSPECIFIED = 0;
    INCIDENT_TYPE_INFRASTRUCTURE = 1;
    INCIDENT_TYPE_APPLICATION = 2;
    INCIDENT_TYPE_SECURITY = 3;
    INCIDENT_TYPE_DATA = 4;
    INCIDENT_TYPE_PERFORMANCE = 5;
    INCIDENT_TYPE_AVAILABILITY = 6;
    INCIDENT_TYPE_COMPLIANCE = 7;
}
```

## Error Handling

gRPC errors map to standard gRPC status codes:

| Application Error | gRPC Status |
|-------------------|-------------|
| NotFound | NOT_FOUND |
| Validation | INVALID_ARGUMENT |
| Authentication | UNAUTHENTICATED |
| Authorization | PERMISSION_DENIED |
| RateLimit | RESOURCE_EXHAUSTED |
| Timeout | DEADLINE_EXCEEDED |
| InvalidStateTransition | FAILED_PRECONDITION |
| Internal | INTERNAL |

Example error handling:
```rust
match client.get_incident(request).await {
    Ok(response) => {
        // Process response
    }
    Err(status) => {
        match status.code() {
            tonic::Code::NotFound => println!("Incident not found"),
            tonic::Code::InvalidArgument => println!("Invalid request"),
            _ => println!("Error: {}", status.message()),
        }
    }
}
```

## Performance Tuning

### Connection Pooling
```rust
let channel = tonic::transport::Channel::from_static("http://localhost:9000")
    .connect_timeout(Duration::from_secs(5))
    .timeout(Duration::from_secs(30))
    .concurrency_limit(256)
    .connect()
    .await?;
```

### Compression
```rust
let client = incidents::incident_service_client::IncidentServiceClient::new(channel)
    .send_compressed(CompressionEncoding::Gzip)
    .accept_compressed(CompressionEncoding::Gzip);
```

### Keep-Alive
```rust
let channel = tonic::transport::Channel::from_static("http://localhost:9000")
    .keep_alive_while_idle(true)
    .http2_keep_alive_interval(Duration::from_secs(30))
    .connect()
    .await?;
```

## Production Deployment

### TLS/SSL

Enable TLS in configuration:
```toml
[server]
tls_enabled = true
tls_cert = "/path/to/server.crt"
tls_key = "/path/to/server.key"
```

Client with TLS:
```rust
let tls = tonic::transport::ClientTlsConfig::new()
    .ca_certificate(Certificate::from_pem(ca_cert))
    .domain_name("incident-manager.example.com");

let channel = Channel::from_static("https://incident-manager.example.com:9000")
    .tls_config(tls)?
    .connect()
    .await?;
```

### Load Balancing

Use a gRPC load balancer (e.g., Envoy, NGINX):
```yaml
# envoy.yaml
static_resources:
  listeners:
  - address:
      socket_address:
        address: 0.0.0.0
        port_value: 9000
    filter_chains:
    - filters:
      - name: envoy.filters.network.http_connection_manager
        typed_config:
          "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
          codec_type: AUTO
          stat_prefix: grpc
          route_config:
            name: local_route
            virtual_hosts:
            - name: backend
              domains: ["*"]
              routes:
              - match: { prefix: "/" }
                route:
                  cluster: incident_manager_cluster
          http_filters:
          - name: envoy.filters.http.router
  clusters:
  - name: incident_manager_cluster
    type: STRICT_DNS
    lb_policy: ROUND_ROBIN
    http2_protocol_options: {}
    load_assignment:
      cluster_name: incident_manager_cluster
      endpoints:
      - lb_endpoints:
        - endpoint:
            address:
              socket_address:
                address: incident-manager-1
                port_value: 9000
        - endpoint:
            address:
              socket_address:
                address: incident-manager-2
                port_value: 9000
```

## Monitoring

### Metrics

The gRPC server exposes metrics on the metrics port (default 9090):

```bash
curl http://localhost:9090/metrics | grep grpc
```

Key metrics:
- `grpc_requests_total` - Total requests by method
- `grpc_request_duration_seconds` - Request latency
- `grpc_errors_total` - Errors by method and code

### Health Checking

```bash
grpcurl -plaintext localhost:9000 alerts.AlertIngestion/HealthCheck
```

Response:
```json
{
  "status": "HEALTH_STATUS_HEALTHY",
  "version": "1.0.0",
  "uptime_seconds": "3600"
}
```

## Troubleshooting

### Connection Refused
```
Error: transport error
```

**Solution**: Ensure gRPC server is running on the correct port:
```bash
# Check if port is listening
netstat -an | grep 9000

# Check server logs
docker logs llm-incident-manager
```

### Invalid Argument
```
Status { code: InvalidArgument, message: "Invalid UUID" }
```

**Solution**: Validate input before sending:
```rust
let uuid = Uuid::parse_str(&incident_id)
    .map_err(|_| "Invalid incident ID format")?;
```

### Deadline Exceeded
```
Status { code: DeadlineExceeded, message: "Timeout" }
```

**Solution**: Increase client timeout:
```rust
let client = client.timeout(Duration::from_secs(60));
```

## Best Practices

1. **Connection Reuse**: Create one client and reuse it
2. **Error Handling**: Always handle gRPC status codes
3. **Timeouts**: Set reasonable timeouts for all calls
4. **Retries**: Implement exponential backoff for retries
5. **Streaming**: Use streaming for large data sets
6. **Compression**: Enable compression for large payloads
7. **TLS**: Always use TLS in production
8. **Monitoring**: Track request metrics and errors

## Examples

Run the complete example:
```bash
# Terminal 1: Start server
cargo run --release

# Terminal 2: Run gRPC client example
cargo run --example grpc_client

# Or use shell scripts
./examples/grpc_submit_alert.sh
./examples/grpc_list_incidents.sh
```

## Additional Resources

- [Protocol Buffers](https://protobuf.dev/)
- [gRPC Documentation](https://grpc.io/docs/)
- [Tonic (Rust gRPC)](https://github.com/hyperium/tonic)
- [grpcurl Tool](https://github.com/fullstorydev/grpcurl)

---

**Version**: 1.0.0
**Last Updated**: 2025-11-11
