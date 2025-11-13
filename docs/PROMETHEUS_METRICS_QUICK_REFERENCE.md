# Prometheus Metrics - Quick Reference

## Overview

Quick reference guide for Prometheus metrics in the LLM Incident Manager.

---

## Metric Categories

### HTTP Request Metrics

```promql
# Request rate
rate(http_requests_total[5m])

# P95 latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Error rate
100 * rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])

# In-flight requests
http_requests_in_flight
```

### Incident Metrics

```promql
# Incident creation rate
rate(incidents_total[5m])

# Active incidents
incidents_active

# MTTR (Mean Time To Resolve)
histogram_quantile(0.5, rate(incident_resolution_duration_seconds_bucket[1h])) / 60

# P0/P1 incidents
incidents_active{severity=~"P0|P1"}

# Deduplication rate
100 * rate(incidents_deduplicated_total[5m]) / rate(incidents_total[5m])
```

### LLM Integration Metrics

```promql
# Request rate by provider
rate(llm_requests_total[5m])

# P95 latency
histogram_quantile(0.95, rate(llm_request_duration_seconds_bucket[5m]))

# Token consumption per hour
rate(llm_tokens_total[1h]) * 3600

# Hourly cost
rate(llm_cost_dollars[1h]) * 3600

# Error rate by provider
100 * rate(llm_errors_total[5m]) / rate(llm_requests_total[5m])
```

### Background Job Metrics

```promql
# Job processing rate
rate(jobs_total[5m])

# Queue depth
jobs_queue_depth

# Job failure rate
100 * rate(jobs_failures_total[5m]) / rate(jobs_total[5m])

# P99 execution time
histogram_quantile(0.99, rate(jobs_duration_seconds_bucket[5m]))
```

### System Health Metrics

```promql
# Memory utilization %
100 * system_memory_bytes{type="used"} / system_memory_bytes{type="total"}

# DB connection pool utilization
100 * database_connections_active / (database_connections_active + database_connections_idle)

# Cache hit rate
100 * rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m]))

# Consumer lag
message_queue_lag_seconds > 60
```

---

## Common Patterns

### Recording Metrics

```rust
// Counter - increment
METRICS_REGISTRY.incidents_total
    .with_label_values(&["P0", "sentinel", "performance"])
    .inc();

// Gauge - set value
METRICS_REGISTRY.incidents_active
    .with_label_values(&["P0", "sentinel", "performance"])
    .set(5.0);

// Histogram - observe
METRICS_REGISTRY.http_request_duration_seconds
    .with_label_values(&["GET", "/incidents", "200"])
    .observe(0.123);
```

### Timing Operations

```rust
use std::time::Instant;

let start = Instant::now();
// ... operation ...
let duration = start.elapsed().as_secs_f64();

METRICS_REGISTRY.operation_duration_seconds
    .with_label_values(&["operation_name"])
    .observe(duration);
```

### Error Handling

```rust
match operation.await {
    Ok(result) => {
        METRICS_REGISTRY.requests_total
            .with_label_values(&["operation", "success"])
            .inc();
        Ok(result)
    }
    Err(e) => {
        METRICS_REGISTRY.requests_total
            .with_label_values(&["operation", "error"])
            .inc();
        METRICS_REGISTRY.errors_total
            .with_label_values(&["operation", error_type(&e)])
            .inc();
        Err(e)
    }
}
```

---

## Configuration

### TOML Config

```toml
[metrics]
enabled = true
listen_address = "0.0.0.0:9090"
endpoint_path = "/metrics"

[metrics.global_labels]
environment = "production"
cluster = "us-east-1"

[metrics.http]
enabled = true

[metrics.incidents]
enabled = true

[metrics.llm]
enabled = true
track_cost = true

[metrics.system]
enabled = true
collection_interval_seconds = 15
```

### Environment Variables

```bash
METRICS_ENABLED=true
METRICS_LISTEN_ADDRESS=0.0.0.0:9090
METRICS_HTTP_ENABLED=true
METRICS_SYSTEM_ENABLED=true
```

---

## Prometheus Config

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'incident-manager'
    static_configs:
      - targets: ['localhost:9090']
```

---

## Alert Rules

```yaml
# High error rate
- alert: HighErrorRate
  expr: |
    100 * rate(http_requests_total{status=~"5.."}[5m])
    / rate(http_requests_total[5m]) > 5
  for: 5m
  labels:
    severity: warning

# High latency
- alert: HighLatency
  expr: |
    histogram_quantile(0.95,
      rate(http_request_duration_seconds_bucket[5m])
    ) > 1.0
  for: 5m
  labels:
    severity: warning

# P0 incidents
- alert: CriticalIncidents
  expr: incidents_active{severity="P0"} > 0
  for: 1m
  labels:
    severity: critical

# LLM errors
- alert: LLMHighErrorRate
  expr: |
    100 * rate(llm_errors_total[5m])
    / rate(llm_requests_total[5m]) > 10
  for: 5m
  labels:
    severity: warning
```

---

## Dashboard Queries

### Request Rate Panel

```promql
sum(rate(http_requests_total[5m])) by (path, method)
```

### Latency Percentiles Panel

```promql
histogram_quantile(0.50, rate(http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))
```

### Incidents by Severity Panel

```promql
sum(incidents_active) by (severity)
```

### LLM Cost Panel

```promql
rate(llm_cost_dollars[1h]) * 3600 * 24
```

---

## Troubleshooting

### No Metrics Appearing

```bash
# Check metrics endpoint
curl http://localhost:9090/metrics

# Verify Prometheus can reach target
curl http://prometheus:9090/api/v1/targets

# Check application logs
grep -i "metrics" app.log
```

### High Memory Usage

```bash
# Check label cardinality
curl http://localhost:9090/metrics | \
  grep -o 'label="[^"]*"' | \
  sort | uniq -c | sort -nr
```

### Metrics Not Updating

```promql
# Check if Prometheus is scraping
up{job="incident-manager"}

# Check scrape duration
scrape_duration_seconds{job="incident-manager"}
```

---

## Quick Links

- **Architecture**: [PROMETHEUS_METRICS_ARCHITECTURE.md](./PROMETHEUS_METRICS_ARCHITECTURE.md)
- **Implementation**: [PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md)
- **Prometheus Docs**: https://prometheus.io/docs/
- **Grafana Docs**: https://grafana.com/docs/

---

## Metric Naming Cheatsheet

| Type | Suffix | Example |
|------|--------|---------|
| Counter | `_total` | `requests_total` |
| Gauge | (none) | `active_connections` |
| Histogram | `_seconds`, `_bytes` | `duration_seconds` |
| Summary | `_seconds`, `_bytes` | `response_size_bytes` |

## Label Guidelines

- ✅ **Good**: `{method="get", path="/incidents", status="200"}`
- ❌ **Bad**: `{user_id="12345", timestamp="1699999999"}`

Keep cardinality low!

---

## Common Operations

### Aggregate Across Labels

```promql
sum(metric) by (label)
```

### Filter by Label

```promql
metric{label="value"}
metric{label=~"pattern"}
```

### Rate Calculation

```promql
rate(counter[5m])
```

### Percentile

```promql
histogram_quantile(0.95, rate(histogram_bucket[5m]))
```

### Join Metrics

```promql
metric1 / on(label) metric2
```

---

## Testing Metrics

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_metric() {
        METRICS_REGISTRY.my_metric
            .with_label_values(&["label"])
            .inc();

        let export = METRICS_REGISTRY.export().unwrap();
        assert!(export.contains("my_metric"));
    }
}
```

---

## Performance Tips

1. **Pre-compute labels**
   ```rust
   let labels = [severity.as_str(), source.as_str()];
   metric.with_label_values(&labels).inc();
   ```

2. **Avoid high-cardinality labels**
   - No: user IDs, timestamps, UUIDs
   - Yes: user roles, status codes, categories

3. **Use histograms for latencies**
   - Not gauges or summaries

4. **Keep label count low**
   - Max 4-5 labels per metric

5. **Cache metric objects**
   - Reuse label combinations

---

## Example Integration

```rust
use axum::{Router, middleware};
use crate::metrics::{METRICS_REGISTRY, middleware::http_metrics_middleware};

fn main() {
    // Start metrics server
    tokio::spawn(start_metrics_server("0.0.0.0:9090"));

    // Build app with middleware
    let app = Router::new()
        .route("/incidents", get(list_incidents))
        .layer(middleware::from_fn(http_metrics_middleware));

    // Run app
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

---

## Deployment

### Docker

```dockerfile
EXPOSE 8080 9090
CMD ["./incident-manager"]
```

### Kubernetes

```yaml
apiVersion: v1
kind: Service
metadata:
  name: metrics
spec:
  ports:
    - port: 9090
      name: metrics
```

### Docker Compose

```yaml
services:
  app:
    ports:
      - "8080:8080"
      - "9090:9090"

  prometheus:
    image: prom/prometheus
    ports:
      - "9091:9090"
```

---

## Resources

### Documentation
- [Architecture Document](./PROMETHEUS_METRICS_ARCHITECTURE.md)
- [Implementation Guide](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md)

### External Links
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [PromQL Cheat Sheet](https://promlabs.com/promql-cheat-sheet/)
- [Grafana Dashboards](https://grafana.com/grafana/dashboards/)

### Example Dashboards
- [Incident Manager Overview](https://grafana.com/grafana/dashboards/...)
- [HTTP Metrics](https://grafana.com/grafana/dashboards/...)
- [System Health](https://grafana.com/grafana/dashboards/...)
