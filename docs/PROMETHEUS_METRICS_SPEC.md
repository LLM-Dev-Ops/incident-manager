# Prometheus Metrics Implementation Guide

## Overview

This guide provides specifications for implementing Prometheus metrics in the LLM Incident Manager. The implementation should follow Prometheus best practices and client library guidelines.

## Implementation Location

Create the metrics module at:
```
src/api/metrics.rs
```

And add it to `src/api/mod.rs`:
```rust
pub mod metrics;
```

## Required Components

### 1. Metrics Registry

```rust
use prometheus::{Registry, Encoder, TextEncoder};
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref METRICS_REGISTRY: Arc<Registry> = Arc::new(Registry::new());
}
```

### 2. Application Metrics

Define the following metrics for the incident management system:

#### HTTP Metrics
```rust
use prometheus::{IntCounterVec, HistogramVec, Opts, register_int_counter_vec, register_histogram_vec};

lazy_static! {
    // HTTP request counter
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "http_requests_total",
            "Total number of HTTP requests"
        ),
        &["method", "endpoint", "status"]
    ).unwrap();

    // HTTP request duration histogram
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "endpoint"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).unwrap();
}
```

#### Incident Metrics
```rust
lazy_static! {
    // Incident counter by severity
    pub static ref INCIDENTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "incidents_total",
            "Total number of incidents created"
        ),
        &["severity", "type", "source"]
    ).unwrap();

    // Active incidents gauge
    pub static ref INCIDENTS_ACTIVE: IntGaugeVec = register_int_gauge_vec!(
        Opts::new(
            "incidents_active",
            "Number of currently active incidents"
        ),
        &["severity", "state"]
    ).unwrap();

    // Incident resolution time histogram
    pub static ref INCIDENT_RESOLUTION_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "incident_resolution_duration_seconds",
        "Time taken to resolve incidents",
        &["severity", "type"],
        vec![60.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0, 14400.0, 28800.0, 86400.0]
    ).unwrap();
}
```

#### Alert Metrics
```rust
lazy_static! {
    // Alert ingestion counter
    pub static ref ALERTS_RECEIVED_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "alerts_received_total",
            "Total number of alerts received"
        ),
        &["source", "severity", "type"]
    ).unwrap();

    // Alert processing duration
    pub static ref ALERT_PROCESSING_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "alert_processing_duration_seconds",
        "Time taken to process alerts",
        &["source"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();
}
```

#### Correlation Metrics
```rust
lazy_static! {
    // Correlation operations
    pub static ref CORRELATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "correlations_total",
            "Total number of alert correlations"
        ),
        &["strategy", "result"]
    ).unwrap();

    // Correlation duration
    pub static ref CORRELATION_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "correlation_duration_seconds",
        "Time taken for correlation analysis",
        &["strategy"],
        vec![0.001, 0.01, 0.1, 1.0, 5.0]
    ).unwrap();
}
```

#### Enrichment Metrics
```rust
lazy_static! {
    // Enrichment operations
    pub static ref ENRICHMENTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "enrichments_total",
            "Total number of enrichment operations"
        ),
        &["enricher", "status"]
    ).unwrap();

    // Enrichment duration
    pub static ref ENRICHMENT_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "enrichment_duration_seconds",
        "Time taken for enrichment",
        &["enricher"],
        vec![0.01, 0.1, 0.5, 1.0, 5.0, 10.0]
    ).unwrap();
}
```

#### Notification Metrics
```rust
lazy_static! {
    // Notification delivery
    pub static ref NOTIFICATIONS_SENT_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "notifications_sent_total",
            "Total number of notifications sent"
        ),
        &["channel", "status"]
    ).unwrap();

    // Notification delivery duration
    pub static ref NOTIFICATION_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "notification_duration_seconds",
        "Time taken to send notifications",
        &["channel"],
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
    ).unwrap();
}
```

#### LLM Integration Metrics
```rust
lazy_static! {
    // LLM API calls
    pub static ref LLM_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "llm_requests_total",
            "Total number of LLM API requests"
        ),
        &["provider", "model", "status"]
    ).unwrap();

    // LLM request duration
    pub static ref LLM_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "llm_request_duration_seconds",
        "LLM API request duration",
        &["provider", "model"],
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
    ).unwrap();

    // LLM token usage
    pub static ref LLM_TOKENS_USED_TOTAL: IntCounterVec = register_int_counter_vec!(
        Opts::new(
            "llm_tokens_used_total",
            "Total number of tokens used"
        ),
        &["provider", "model", "type"]  // type: prompt, completion
    ).unwrap();
}
```

### 3. Metrics Endpoint Handler

```rust
use axum::{response::IntoResponse, http::StatusCode};

pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = vec![];
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => {
            (
                StatusCode::OK,
                [("Content-Type", encoder.format_type())],
                buffer
            )
        }
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("Content-Type", "text/plain")],
                format!("Failed to encode metrics: {}", e).into_bytes()
            )
        }
    }
}
```

### 4. HTTP Middleware for Request Tracking

```rust
use axum::{
    middleware::Next,
    extract::Request,
    response::Response,
};
use std::time::Instant;

pub async fn metrics_middleware(
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status().as_u16().to_string();

    // Record metrics
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();

    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[&method, &path])
        .observe(duration.as_secs_f64());

    response
}
```

### 5. Add Route to Router

In `src/api/routes.rs`:

```rust
use crate::api::metrics;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // ... existing routes ...
        .route("/metrics", get(metrics::metrics_handler))
        // Add middleware
        .layer(middleware::from_fn(metrics::metrics_middleware))
        .with_state(state)
        // ... existing layers ...
}
```

## Integration Points

### 1. Alert Processing

In `src/processing/processor.rs`:

```rust
use crate::api::metrics::*;

pub async fn process_alert(&self, alert: Alert) -> Result<AlertAck> {
    let start = Instant::now();

    // Record alert received
    ALERTS_RECEIVED_TOTAL
        .with_label_values(&[
            &alert.source,
            &alert.severity.to_string(),
            &alert.alert_type.to_string()
        ])
        .inc();

    // Process alert...
    let result = self.do_process_alert(alert).await;

    // Record processing time
    ALERT_PROCESSING_DURATION_SECONDS
        .with_label_values(&[&alert.source])
        .observe(start.elapsed().as_secs_f64());

    result
}
```

### 2. Incident Creation

In `src/processing/processor.rs`:

```rust
pub async fn create_incident(&self, incident: Incident) -> Result<Incident> {
    // Create incident...

    // Record metrics
    INCIDENTS_TOTAL
        .with_label_values(&[
            &incident.severity.to_string(),
            &incident.incident_type.to_string(),
            &incident.source
        ])
        .inc();

    INCIDENTS_ACTIVE
        .with_label_values(&[
            &incident.severity.to_string(),
            &incident.state.to_string()
        ])
        .inc();

    Ok(incident)
}
```

### 3. Incident Resolution

```rust
pub async fn resolve_incident(&self, id: &Uuid, ...) -> Result<Incident> {
    let incident = self.get_incident(id).await?;
    let start_time = incident.created_at;

    // Resolve incident...

    // Record resolution time
    let duration = (Utc::now() - start_time).num_seconds() as f64;
    INCIDENT_RESOLUTION_DURATION_SECONDS
        .with_label_values(&[
            &incident.severity.to_string(),
            &incident.incident_type.to_string()
        ])
        .observe(duration);

    // Update active incidents gauge
    INCIDENTS_ACTIVE
        .with_label_values(&[
            &incident.severity.to_string(),
            "open"
        ])
        .dec();

    Ok(incident)
}
```

### 4. LLM Integrations

In `src/integrations/*/client.rs`:

```rust
pub async fn call_llm(&self, prompt: &str) -> Result<LLMResponse> {
    let start = Instant::now();

    LLM_REQUESTS_TOTAL
        .with_label_values(&[&self.provider, &self.model, "started"])
        .inc();

    match self.do_call_llm(prompt).await {
        Ok(response) => {
            LLM_REQUESTS_TOTAL
                .with_label_values(&[&self.provider, &self.model, "success"])
                .inc();

            LLM_REQUEST_DURATION_SECONDS
                .with_label_values(&[&self.provider, &self.model])
                .observe(start.elapsed().as_secs_f64());

            // Record token usage
            if let Some(usage) = &response.usage {
                LLM_TOKENS_USED_TOTAL
                    .with_label_values(&[&self.provider, &self.model, "prompt"])
                    .inc_by(usage.prompt_tokens);

                LLM_TOKENS_USED_TOTAL
                    .with_label_values(&[&self.provider, &self.model, "completion"])
                    .inc_by(usage.completion_tokens);
            }

            Ok(response)
        }
        Err(e) => {
            LLM_REQUESTS_TOTAL
                .with_label_values(&[&self.provider, &self.model, "error"])
                .inc();
            Err(e)
        }
    }
}
```

## Testing

Once implemented, activate the tests by:

1. Remove `TODO` comments from test file
2. Uncomment test code
3. Run tests:
   ```bash
   cargo test --test prometheus_metrics_test
   ```

4. Run benchmarks:
   ```bash
   cargo bench
   ```

## Prometheus Configuration

### Scrape Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'llm-incident-manager'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

### Example Queries

```promql
# Request rate by endpoint
rate(http_requests_total[5m])

# Average response time
rate(http_request_duration_seconds_sum[5m]) / rate(http_request_duration_seconds_count[5m])

# P95 response time
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Incident creation rate
rate(incidents_total[5m])

# Active incidents by severity
sum(incidents_active) by (severity)

# LLM token usage per hour
increase(llm_tokens_used_total[1h])

# Alert processing latency
histogram_quantile(0.99, rate(alert_processing_duration_seconds_bucket[5m]))
```

## Best Practices

1. **Metric Naming**
   - Use `snake_case`
   - Counter names end with `_total`
   - Duration metrics end with `_seconds`
   - Size metrics end with `_bytes`

2. **Labels**
   - Keep cardinality reasonable (< 1000 unique combinations per metric)
   - Avoid high-cardinality labels (user IDs, timestamps, UUIDs)
   - Use consistent label names across metrics

3. **Performance**
   - Metrics operations should be < 1μs
   - Use `IntCounter` instead of `Counter` when possible
   - Pre-register label combinations when possible

4. **Documentation**
   - Every metric needs a `HELP` text
   - Document what labels mean
   - Provide example queries

## References

- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [prometheus-rs Documentation](https://docs.rs/prometheus/)
- [OpenMetrics Specification](https://openmetrics.io/)
