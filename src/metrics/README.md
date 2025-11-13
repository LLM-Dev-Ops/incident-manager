# Prometheus Metrics Exporter

Production-ready Prometheus metrics exporter for the LLM Incident Manager system.

## Features

- **Comprehensive Coverage**: Tracks HTTP, gRPC, incidents, LLM operations, notifications, playbooks, and storage
- **Low Overhead**: < 1ms per request, optimized for production use
- **Type-Safe**: Full Rust type safety with compile-time guarantees
- **Flexible Configuration**: Customizable sampling, exclusions, and behavior
- **RAII-Based Tracking**: Automatic cleanup with guard patterns
- **Zero Allocation Hot Paths**: Memory-efficient atomic operations

## Quick Start

### 1. Initialize Metrics

Metrics are automatically initialized in `main.rs` when `prometheus_enabled` is true in the config:

```rust
use llm_incident_manager::metrics;

// Initialize metrics (called once at startup)
metrics::init_metrics()?;
```

### 2. Access Metrics Endpoint

Metrics are exposed at `http://localhost:8080/metrics` in Prometheus text format:

```bash
curl http://localhost:8080/metrics
```

### 3. Basic Usage

#### HTTP Requests (Automatic)

HTTP requests are automatically tracked when using the Axum router:

```rust
use llm_incident_manager::metrics::MetricsMiddleware;

let app = Router::new()
    .route("/api", get(handler))
    .layer(MetricsMiddleware::layer());
```

#### gRPC Requests

Use the request guard for automatic tracking:

```rust
use llm_incident_manager::metrics::MetricsInterceptor;

let interceptor = MetricsInterceptor::new();
let guard = interceptor.start_request("IncidentService", "CreateIncident");

// ... handle request ...

guard.success(); // or guard.error(&status)
```

#### LLM Calls

Track LLM operations with automatic cost and token counting:

```rust
use llm_incident_manager::metrics::decorators::LLMCallTracker;

let tracker = LLMCallTracker::start("openai", "gpt-4", "completion");

// Make LLM call...
let response = client.complete(prompt).await?;

// Record success with token usage
tracker.success(
    response.usage.input_tokens,
    response.usage.output_tokens,
    0.005 // cost in USD
);
```

#### Incident Processing

Track incident processing automatically:

```rust
use llm_incident_manager::metrics::decorators::IncidentTracker;

let tracker = IncidentTracker::start("critical");

// Process incident...
match process_incident(incident).await {
    Ok(_) => tracker.success("resolved"),
    Err(_) => tracker.error(),
}
```

## Available Metrics

### HTTP Metrics

- `llm_incident_manager_http_requests_total` - Total HTTP requests (labels: method, path, status_code)
- `llm_incident_manager_http_request_duration_seconds` - Request duration histogram
- `llm_incident_manager_http_request_size_bytes` - Request body size
- `llm_incident_manager_http_response_size_bytes` - Response body size
- `llm_incident_manager_http_connections_active` - Active connections

### gRPC Metrics

- `llm_incident_manager_grpc_requests_total` - Total gRPC requests (labels: service, method, status)
- `llm_incident_manager_grpc_request_duration_seconds` - Request duration histogram
- `llm_incident_manager_grpc_streams_active` - Active streams

### Incident Metrics

- `llm_incident_manager_incidents_total` - Total incidents (labels: severity, status)
- `llm_incident_manager_incidents_active` - Active incidents (label: severity)
- `llm_incident_manager_incident_processing_duration_seconds` - Processing duration
- `llm_incident_manager_incidents_deduplicated_total` - Deduplicated incidents
- `llm_incident_manager_incidents_correlated_total` - Correlated incidents
- `llm_incident_manager_incidents_escalated_total` - Escalated incidents (label: level)

### LLM Metrics

- `llm_incident_manager_llm_requests_total` - Total LLM requests (labels: provider, model, operation)
- `llm_incident_manager_llm_request_duration_seconds` - Request duration histogram
- `llm_incident_manager_llm_tokens_total` - Token usage (labels: provider, model, token_type)
- `llm_incident_manager_llm_errors_total` - LLM errors (labels: provider, error_type)
- `llm_incident_manager_llm_cost_usd` - LLM costs in USD

### Notification Metrics

- `llm_incident_manager_notifications_total` - Total notifications (labels: channel, status)
- `llm_incident_manager_notification_duration_seconds` - Delivery duration
- `llm_incident_manager_notification_queue_size` - Queue size (label: channel)

### Playbook Metrics

- `llm_incident_manager_playbook_executions_total` - Total executions (labels: playbook_id, status)
- `llm_incident_manager_playbook_execution_duration_seconds` - Execution duration
- `llm_incident_manager_playbook_executions_active` - Active executions

### Storage Metrics

- `llm_incident_manager_storage_operations_total` - Total operations (labels: operation, backend)
- `llm_incident_manager_storage_operation_duration_seconds` - Operation duration
- `llm_incident_manager_storage_size_bytes` - Storage size (label: backend)

### Error Metrics

- `llm_incident_manager_errors_total` - Total errors (labels: component, error_type)

### System Metrics

- `llm_incident_manager_build_info` - Build information (labels: version, git_commit, build_timestamp)
- `llm_incident_manager_uptime_seconds` - Application uptime

## Configuration

Create a `MetricsConfig` to customize behavior:

```rust
use llm_incident_manager::metrics::MetricsConfig;

let config = MetricsConfig::new()
    .with_endpoint("/custom-metrics")
    .with_sample_rate(0.1)  // Sample 10% of requests
    .exclude_path("/health")
    .exclude_path("/internal")
    .with_global_label("env", "production")
    .with_global_label("region", "us-east-1");
```

### Configuration Options

- `enabled` - Enable/disable metrics collection (default: true)
- `endpoint` - Metrics endpoint path (default: "/metrics")
- `metrics_port` - Separate port for metrics (optional)
- `include_runtime` - Include runtime metrics (default: true)
- `include_request_details` - Include request/response sizes (default: true)
- `max_label_cardinality` - Max unique label combinations (default: 10,000)
- `sample_rate` - Sample rate 0.0-1.0 (default: 1.0)
- `enable_histograms` - Enable histogram metrics (default: true)
- `global_labels` - Labels added to all metrics
- `excluded_paths` - Paths excluded from HTTP metrics

## Advanced Usage

### Custom Metrics

Record custom storage operations:

```rust
use llm_incident_manager::metrics::collectors::helpers::*;

record_storage_operation("read", "redis", 0.005);
record_notification("slack", true, 0.5);
record_deduplication();
record_escalation("level2");
```

### Async Operations

Track async operations with decorators:

```rust
use llm_incident_manager::metrics::decorators::*;

let result = measure_async("enrichment", "lookup", async {
    // ... async operation ...
    Ok(data)
}).await;
```

### Error Tracking

Track errors automatically:

```rust
use llm_incident_manager::metrics::decorators::*;

let result = measure_async_with_error("validation", "check", async {
    // ... operation that may fail ...
    validate_data().await
}).await;
// Automatically increments error counter on Err
```

## Performance Considerations

### Overhead

- HTTP middleware: ~0.3ms per request
- gRPC interceptor: ~0.2ms per request
- LLM tracker: ~0.1ms per call
- Storage operation: ~0.05ms

### Memory Usage

- Global metrics registry: ~500KB
- Per-request overhead: ~200 bytes
- Atomic counters: no allocations
- Histograms: pre-allocated buckets

### Cardinality Management

To prevent cardinality explosion:

1. **Limit dynamic labels**: Don't use UUIDs or timestamps as labels
2. **Use label values wisely**: Keep to predefined sets
3. **Configure max_label_cardinality**: Set appropriate limits
4. **Use sampling**: For high-frequency endpoints

Example of high vs low cardinality:

```rust
// ❌ HIGH CARDINALITY (avoid)
METRIC.with_label_values(&[&incident_id.to_string()]).inc();

// ✅ LOW CARDINALITY (good)
METRIC.with_label_values(&[&incident.severity.to_string()]).inc();
```

## Prometheus Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'llm-incident-manager'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

## Grafana Dashboards

### Example Queries

**HTTP Request Rate:**
```promql
rate(llm_incident_manager_http_requests_total[5m])
```

**99th Percentile Latency:**
```promql
histogram_quantile(0.99,
  rate(llm_incident_manager_http_request_duration_seconds_bucket[5m])
)
```

**Error Rate:**
```promql
rate(llm_incident_manager_errors_total[5m])
```

**Active Incidents by Severity:**
```promql
llm_incident_manager_incidents_active
```

**LLM Cost per Hour:**
```promql
rate(llm_incident_manager_llm_cost_usd[1h]) * 3600
```

## Testing

Run tests:

```bash
cargo test --package llm-incident-manager --lib metrics
```

Test metrics endpoint:

```bash
# Start the server
cargo run

# In another terminal
curl http://localhost:8080/metrics | grep llm_incident_manager
```

## Troubleshooting

### Metrics not appearing

1. Check if metrics are enabled in config
2. Verify initialization didn't fail (check logs)
3. Ensure requests are being made (metrics only appear after use)

### High memory usage

1. Check label cardinality: `curl localhost:8080/metrics | wc -l`
2. Reduce sample_rate if too high
3. Disable histograms if not needed
4. Review excluded_paths configuration

### Missing metrics

1. Ensure middleware is applied to routes
2. Check if paths are excluded
3. Verify sample_rate isn't too low
4. Check for initialization errors in logs

## Best Practices

1. **Initialize once**: Call `init_metrics()` only at startup
2. **Use RAII guards**: Let guards auto-record on drop
3. **Minimize labels**: Keep label cardinality low
4. **Sample high-volume endpoints**: Use sample_rate < 1.0
5. **Exclude health checks**: Don't pollute metrics with noise
6. **Monitor cardinality**: Track unique label combinations
7. **Use helpers**: Leverage provided helper functions
8. **Test in staging**: Validate metrics before production

## License

MIT - See LICENSE file for details
