# Prometheus Metrics - Quick Reference

## Quick Start

```rust
// 1. Initialize (in main.rs - already done)
use llm_incident_manager::metrics;
metrics::init_metrics()?;

// 2. Access metrics endpoint
curl http://localhost:8080/metrics
```

## Common Patterns

### Track HTTP Requests (Automatic)
```rust
use llm_incident_manager::metrics::MetricsMiddleware;

let app = Router::new()
    .layer(MetricsMiddleware::layer());
```

### Track LLM Calls
```rust
use llm_incident_manager::metrics::decorators::LLMCallTracker;

let tracker = LLMCallTracker::start("openai", "gpt-4", "completion");
let response = llm_client.complete(prompt).await?;
tracker.success(input_tokens, output_tokens, cost_usd);
```

### Track Incidents
```rust
use llm_incident_manager::metrics::decorators::IncidentTracker;

let tracker = IncidentTracker::start("critical");
// ... process incident ...
tracker.success("resolved");
```

### Track gRPC Calls
```rust
use llm_incident_manager::metrics::MetricsInterceptor;

let interceptor = MetricsInterceptor::new();
let guard = interceptor.start_request("ServiceName", "MethodName");
// ... handle request ...
guard.success();
```

### Record Custom Metrics
```rust
use llm_incident_manager::metrics::*;

// Counter
HTTP_REQUESTS_TOTAL
    .with_label_values(&["GET", "/api", "200"])
    .inc();

// Gauge
INCIDENTS_ACTIVE
    .with_label_values(&["critical"])
    .set(5.0);

// Histogram
HTTP_REQUEST_DURATION_SECONDS
    .with_label_values(&["GET", "/api"])
    .observe(0.123);
```

### Helper Functions
```rust
use llm_incident_manager::metrics::collectors::helpers::*;

record_storage_operation("read", "redis", 0.005);
record_notification("slack", true, 0.5);
record_deduplication();
record_escalation("level2");
update_notification_queue_size("email", 42.0);
```

## Configuration

```rust
use llm_incident_manager::metrics::MetricsConfig;

let config = MetricsConfig::new()
    .with_sample_rate(0.1)          // Sample 10%
    .exclude_path("/health")        // Exclude from metrics
    .with_global_label("env", "prod");
```

## Key Metrics

### HTTP
- `llm_incident_manager_http_requests_total{method,path,status_code}`
- `llm_incident_manager_http_request_duration_seconds{method,path}`

### LLM
- `llm_incident_manager_llm_requests_total{provider,model,operation}`
- `llm_incident_manager_llm_tokens_total{provider,model,token_type}`
- `llm_incident_manager_llm_cost_usd{provider,model}`

### Incidents
- `llm_incident_manager_incidents_total{severity,status}`
- `llm_incident_manager_incidents_active{severity}`

## Prometheus Queries

```promql
# Request rate
rate(llm_incident_manager_http_requests_total[5m])

# 99th percentile latency
histogram_quantile(0.99, rate(llm_incident_manager_http_request_duration_seconds_bucket[5m]))

# Error rate
rate(llm_incident_manager_errors_total[5m])

# LLM cost per hour
rate(llm_incident_manager_llm_cost_usd[1h]) * 3600
```

## Troubleshooting

```bash
# View all metrics
curl http://localhost:8080/metrics

# Count total metrics
curl http://localhost:8080/metrics | grep "^llm_incident_manager" | wc -l

# Check specific metric
curl http://localhost:8080/metrics | grep http_requests_total

# Run example
cargo run --example metrics_example
```

## Files

- **Documentation**: `/src/metrics/README.md`
- **Implementation**: `/METRICS_IMPLEMENTATION.md`
- **Example**: `/examples/metrics_example.rs`
- **Source**: `/src/metrics/`
