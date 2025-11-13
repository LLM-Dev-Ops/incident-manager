# Prometheus Metrics Implementation Guide

## Table of Contents
1. [Quick Start](#quick-start)
2. [Step-by-Step Implementation](#step-by-step-implementation)
3. [Code Examples](#code-examples)
4. [Testing Guide](#testing-guide)
5. [Deployment Guide](#deployment-guide)
6. [Troubleshooting](#troubleshooting)

---

## 1. Quick Start

### Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
prometheus = "0.13"
lazy_static = "1.4"
tokio = { version = "1.35", features = ["full"] }
axum = "0.7"
sysinfo = "0.30"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
```

### Minimal Setup

```rust
// src/metrics/mod.rs
use lazy_static::lazy_static;
use prometheus::{Registry, Counter, register_counter_with_registry};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();

    pub static ref HTTP_REQUESTS: Counter =
        register_counter_with_registry!(
            "http_requests_total",
            "Total HTTP requests",
            REGISTRY
        ).unwrap();
}

pub fn export_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metrics = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metrics, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
```

### Basic Usage

```rust
// Increment counter
HTTP_REQUESTS.inc();

// Export metrics
let metrics_text = export_metrics();
```

---

## 2. Step-by-Step Implementation

### Phase 1: Core Registry (Day 1)

#### Step 1.1: Create Module Structure

```bash
mkdir -p src/metrics/middleware
mkdir -p src/metrics/collectors
touch src/metrics/mod.rs
touch src/metrics/registry.rs
touch src/metrics/config.rs
touch src/metrics/middleware/http_metrics.rs
```

#### Step 1.2: Implement Metrics Registry

```rust
// src/metrics/registry.rs
use lazy_static::lazy_static;
use prometheus::{
    Registry, Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec,
    HistogramOpts, Opts, register_counter_vec_with_registry,
    register_gauge_vec_with_registry, register_histogram_vec_with_registry,
};
use std::sync::Arc;

lazy_static! {
    pub static ref METRICS_REGISTRY: Arc<MetricsRegistry> = Arc::new(MetricsRegistry::new());
}

pub struct MetricsRegistry {
    pub registry: Registry,

    // HTTP Metrics
    pub http_requests_total: CounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub http_requests_in_flight: GaugeVec,

    // Incident Metrics
    pub incidents_total: CounterVec,
    pub incidents_active: GaugeVec,
    pub incidents_resolved_total: CounterVec,
    pub incident_resolution_duration_seconds: HistogramVec,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        let registry = Registry::new();

        // HTTP Metrics
        let http_requests_total = register_counter_vec_with_registry!(
            "http_requests_total",
            "Total number of HTTP requests",
            &["method", "path", "status"],
            registry
        ).unwrap();

        let http_request_duration_seconds = register_histogram_vec_with_registry!(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request latency in seconds"
            ).buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            &["method", "path", "status"],
            registry
        ).unwrap();

        let http_requests_in_flight = register_gauge_vec_with_registry!(
            "http_requests_in_flight",
            "Current number of HTTP requests being processed",
            &["method", "path"],
            registry
        ).unwrap();

        // Incident Metrics
        let incidents_total = register_counter_vec_with_registry!(
            "incidents_total",
            "Total number of incidents created",
            &["severity", "source", "category"],
            registry
        ).unwrap();

        let incidents_active = register_gauge_vec_with_registry!(
            "incidents_active",
            "Number of currently active incidents",
            &["severity", "source", "category"],
            registry
        ).unwrap();

        let incidents_resolved_total = register_counter_vec_with_registry!(
            "incidents_resolved_total",
            "Total number of incidents resolved",
            &["severity", "source", "category"],
            registry
        ).unwrap();

        let incident_resolution_duration_seconds = register_histogram_vec_with_registry!(
            HistogramOpts::new(
                "incident_resolution_duration_seconds",
                "Time taken to resolve incidents in seconds"
            ).buckets(vec![60.0, 300.0, 900.0, 1800.0, 3600.0, 14400.0, 43200.0, 86400.0]),
            &["severity", "source"],
            registry
        ).unwrap();

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_requests_in_flight,
            incidents_total,
            incidents_active,
            incidents_resolved_total,
            incident_resolution_duration_seconds,
        }
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> Result<String, prometheus::Error> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = MetricsRegistry::new();
        assert_eq!(registry.registry.gather().len(), 7);
    }

    #[test]
    fn test_export_metrics() {
        let export = METRICS_REGISTRY.export().unwrap();
        assert!(export.contains("http_requests_total"));
        assert!(export.contains("incidents_total"));
    }
}
```

#### Step 1.3: Create Module Exports

```rust
// src/metrics/mod.rs
pub mod registry;
pub mod config;
pub mod middleware;
pub mod collectors;

pub use registry::{METRICS_REGISTRY, MetricsRegistry};
pub use config::MetricsConfig;
```

### Phase 2: HTTP Middleware (Day 2)

#### Step 2.1: Implement HTTP Metrics Middleware

```rust
// src/metrics/middleware/http_metrics.rs
use axum::{
    body::Body,
    extract::MatchedPath,
    http::{Request, Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use std::time::Instant;
use crate::metrics::METRICS_REGISTRY;

/// Middleware for automatic HTTP metrics collection
pub async fn http_metrics_middleware(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let start = Instant::now();

    // Extract request metadata
    let method = req.method().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Track in-flight requests
    METRICS_REGISTRY
        .http_requests_in_flight
        .with_label_values(&[&method, &path])
        .inc();

    // Process request
    let response = next.run(req).await;

    // Extract response metadata
    let status = response.status().as_u16().to_string();
    let duration = start.elapsed().as_secs_f64();

    // Record metrics
    METRICS_REGISTRY
        .http_requests_total
        .with_label_values(&[&method, &path, &status])
        .inc();

    METRICS_REGISTRY
        .http_request_duration_seconds
        .with_label_values(&[&method, &path, &status])
        .observe(duration);

    METRICS_REGISTRY
        .http_requests_in_flight
        .with_label_values(&[&method, &path])
        .dec();

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_http_metrics_middleware() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(http_metrics_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify metrics were recorded
        let export = METRICS_REGISTRY.export().unwrap();
        assert!(export.contains("http_requests_total"));
    }
}
```

#### Step 2.2: Add Middleware Module

```rust
// src/metrics/middleware/mod.rs
mod http_metrics;

pub use http_metrics::http_metrics_middleware;
```

#### Step 2.3: Integrate with Main Application

```rust
// src/main.rs (or wherever you build your router)
use axum::{Router, routing::get, middleware};
use crate::metrics::middleware::http_metrics_middleware;

fn build_router() -> Router {
    Router::new()
        .route("/v1/incidents", get(list_incidents))
        .route("/v1/incidents/:id", get(get_incident))
        .route("/health", get(health_check))
        .layer(middleware::from_fn(http_metrics_middleware)) // Add this line
}
```

### Phase 3: Metrics Endpoint (Day 2)

#### Step 3.1: Create Metrics Exporter

```rust
// src/metrics/exporter.rs
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use crate::metrics::METRICS_REGISTRY;

/// Create a router for the metrics endpoint
pub fn metrics_router() -> Router {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Handler for /metrics endpoint
async fn metrics_handler() -> Response {
    match METRICS_REGISTRY.export() {
        Ok(metrics) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            metrics,
        ).into_response(),
        Err(e) => {
            tracing::error!("Failed to export metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                format!("Error exporting metrics: {}", e),
            ).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    use axum::body::Body;
    use axum::http::Request;

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let app = metrics_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains("# HELP"));
        assert!(text.contains("# TYPE"));
    }
}
```

#### Step 3.2: Start Metrics Server

```rust
// src/metrics/server.rs
use axum::Server;
use std::net::SocketAddr;
use tokio::task;
use crate::metrics::exporter::metrics_router;

/// Start the metrics HTTP server on a separate port
pub async fn start_metrics_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let router = metrics_router();

    task::spawn(async move {
        tracing::info!("Metrics server listening on {}", addr);

        Server::bind(&addr)
            .serve(router.into_make_service())
            .await
            .expect("Metrics server failed to start");
    });

    Ok(())
}
```

#### Step 3.3: Initialize in Main

```rust
// src/main.rs
use crate::metrics::server::start_metrics_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... other initialization ...

    // Start metrics server on port 9090
    let metrics_addr = "0.0.0.0:9090".parse()?;
    start_metrics_server(metrics_addr).await?;

    // Start main application server on port 8080
    let app_addr = "0.0.0.0:8080".parse()?;
    let app = build_router();

    axum::Server::bind(&app_addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

### Phase 4: Business Logic Instrumentation (Day 3-4)

#### Step 4.1: Instrument Incident Creation

```rust
// src/processing/processor.rs (or similar)
use crate::metrics::METRICS_REGISTRY;
use std::time::Instant;

impl IncidentProcessor {
    pub async fn create_incident(&self, event: Event) -> Result<Incident> {
        let start = Instant::now();

        // Check for duplicates
        if let Some(existing) = self.deduplicator.check(&event).await? {
            METRICS_REGISTRY
                .incidents_deduplicated_total
                .with_label_values(&[&event.source])
                .inc();
            return Ok(existing);
        }

        // Create new incident
        let incident = Incident::from_event(event);
        self.store.save(&incident).await?;

        // Record metrics
        let labels = [
            incident.severity.as_str(),
            &incident.source,
            incident.category.as_str(),
        ];

        METRICS_REGISTRY
            .incidents_total
            .with_label_values(&labels)
            .inc();

        METRICS_REGISTRY
            .incidents_active
            .with_label_values(&labels)
            .inc();

        tracing::info!(
            incident_id = %incident.id,
            severity = %incident.severity,
            "Incident created"
        );

        Ok(incident)
    }

    pub async fn resolve_incident(&self, id: &str, resolution: Resolution) -> Result<()> {
        let incident = self.store.get(id).await?;

        // Calculate resolution time
        let resolution_time = incident.created_at.elapsed().as_secs_f64();

        // Update incident
        self.store.update_status(id, IncidentStatus::Resolved).await?;

        // Record metrics
        let labels = [
            incident.severity.as_str(),
            &incident.source,
            incident.category.as_str(),
        ];

        METRICS_REGISTRY
            .incidents_resolved_total
            .with_label_values(&labels)
            .inc();

        METRICS_REGISTRY
            .incidents_active
            .with_label_values(&labels)
            .dec();

        METRICS_REGISTRY
            .incident_resolution_duration_seconds
            .with_label_values(&[incident.severity.as_str(), &incident.source])
            .observe(resolution_time);

        tracing::info!(
            incident_id = %id,
            resolution_time_seconds = resolution_time,
            "Incident resolved"
        );

        Ok(())
    }
}
```

#### Step 4.2: Add LLM Metrics to Registry

```rust
// src/metrics/registry.rs (add to MetricsRegistry)

// LLM Metrics
pub llm_requests_total: CounterVec,
pub llm_request_duration_seconds: HistogramVec,
pub llm_tokens_total: CounterVec,
pub llm_cost_dollars: CounterVec,
pub llm_errors_total: CounterVec,

// In new() method:
let llm_requests_total = register_counter_vec_with_registry!(
    "llm_requests_total",
    "Total number of LLM API requests",
    &["provider", "model", "operation", "status"],
    registry
).unwrap();

let llm_request_duration_seconds = register_histogram_vec_with_registry!(
    HistogramOpts::new(
        "llm_request_duration_seconds",
        "LLM request latency in seconds"
    ).buckets(vec![0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]),
    &["provider", "model", "operation"],
    registry
).unwrap();

let llm_tokens_total = register_counter_vec_with_registry!(
    "llm_tokens_total",
    "Total number of tokens consumed",
    &["provider", "model", "token_type"],
    registry
).unwrap();

let llm_cost_dollars = register_counter_vec_with_registry!(
    "llm_cost_dollars",
    "Estimated cost in dollars",
    &["provider", "model"],
    registry
).unwrap();

let llm_errors_total = register_counter_vec_with_registry!(
    "llm_errors_total",
    "Total number of LLM errors",
    &["provider", "model", "error_type"],
    registry
).unwrap();
```

#### Step 4.3: Instrument LLM Client

```rust
// src/llm/client.rs (or similar)
use crate::metrics::METRICS_REGISTRY;
use std::time::Instant;

impl LlmClient {
    pub async fn classify(&self, event: &Event) -> Result<Classification> {
        let start = Instant::now();
        let provider = "openai";
        let model = "gpt-4";
        let operation = "classification";

        // Make LLM request
        let result = self.call_api(event).await;

        let duration = start.elapsed().as_secs_f64();

        // Record metrics based on result
        match &result {
            Ok(response) => {
                // Success metrics
                METRICS_REGISTRY
                    .llm_requests_total
                    .with_label_values(&[provider, model, operation, "success"])
                    .inc();

                METRICS_REGISTRY
                    .llm_request_duration_seconds
                    .with_label_values(&[provider, model, operation])
                    .observe(duration);

                // Token metrics
                METRICS_REGISTRY
                    .llm_tokens_total
                    .with_label_values(&[provider, model, "prompt"])
                    .inc_by(response.prompt_tokens as f64);

                METRICS_REGISTRY
                    .llm_tokens_total
                    .with_label_values(&[provider, model, "completion"])
                    .inc_by(response.completion_tokens as f64);

                // Cost metrics
                let cost = self.calculate_cost(response);
                METRICS_REGISTRY
                    .llm_cost_dollars
                    .with_label_values(&[provider, model])
                    .inc_by(cost);
            }
            Err(e) => {
                // Error metrics
                let error_type = match e {
                    LlmError::RateLimit => "rate_limit",
                    LlmError::Timeout => "timeout",
                    LlmError::Authentication => "authentication",
                    _ => "unknown",
                };

                METRICS_REGISTRY
                    .llm_requests_total
                    .with_label_values(&[provider, model, operation, "error"])
                    .inc();

                METRICS_REGISTRY
                    .llm_errors_total
                    .with_label_values(&[provider, model, error_type])
                    .inc();
            }
        }

        result
    }

    fn calculate_cost(&self, response: &LlmResponse) -> f64 {
        // Example pricing for GPT-4
        let prompt_cost = (response.prompt_tokens as f64 / 1000.0) * 0.03;
        let completion_cost = (response.completion_tokens as f64 / 1000.0) * 0.06;
        prompt_cost + completion_cost
    }
}
```

### Phase 5: System Metrics Collector (Day 5)

#### Step 5.1: Add System Metrics to Registry

```rust
// src/metrics/registry.rs (add to MetricsRegistry)

pub system_memory_bytes: GaugeVec,
pub system_cpu_usage_percent: GaugeVec,

// In new() method:
let system_memory_bytes = register_gauge_vec_with_registry!(
    "system_memory_bytes",
    "System memory usage in bytes",
    &["type"],
    registry
).unwrap();

let system_cpu_usage_percent = register_gauge_vec_with_registry!(
    "system_cpu_usage_percent",
    "System CPU usage percentage",
    &["core"],
    registry
).unwrap();
```

#### Step 5.2: Implement System Collector

```rust
// src/metrics/collectors/system.rs
use sysinfo::{System, SystemExt, CpuExt};
use tokio::time::{interval, Duration};
use crate::metrics::METRICS_REGISTRY;

pub struct SystemMetricsCollector {
    system: System,
    interval: Duration,
}

impl SystemMetricsCollector {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            system: System::new_all(),
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub async fn start(mut self) {
        let mut ticker = interval(self.interval);

        loop {
            ticker.tick().await;
            self.collect();
        }
    }

    fn collect(&mut self) {
        self.system.refresh_all();

        // Memory metrics
        let used = self.system.used_memory() as f64;
        let total = self.system.total_memory() as f64;
        let available = self.system.available_memory() as f64;

        METRICS_REGISTRY
            .system_memory_bytes
            .with_label_values(&["used"])
            .set(used);

        METRICS_REGISTRY
            .system_memory_bytes
            .with_label_values(&["total"])
            .set(total);

        METRICS_REGISTRY
            .system_memory_bytes
            .with_label_values(&["available"])
            .set(available);

        // CPU metrics
        for (idx, cpu) in self.system.cpus().iter().enumerate() {
            METRICS_REGISTRY
                .system_cpu_usage_percent
                .with_label_values(&[&format!("core_{}", idx)])
                .set(cpu.cpu_usage() as f64);
        }
    }
}
```

#### Step 5.3: Start Collector in Main

```rust
// src/main.rs
use crate::metrics::collectors::SystemMetricsCollector;
use tokio::spawn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... other initialization ...

    // Start system metrics collector
    let collector = SystemMetricsCollector::new(15); // Collect every 15 seconds
    spawn(async move {
        collector.start().await;
    });

    // ... rest of application ...
}
```

---

## 3. Code Examples

### Example 1: Complete Instrumented Handler

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::metrics::METRICS_REGISTRY;

pub async fn get_incident(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Incident>, AppError> {
    // HTTP metrics are automatic via middleware

    // Database query (could add DB metrics here)
    let incident = state
        .incident_store
        .get(&id)
        .await
        .map_err(|e| AppError::NotFound(format!("Incident not found: {}", id)))?;

    Ok(Json(incident))
}

pub async fn create_incident(
    State(state): State<Arc<AppState>>,
    Json(event): Json<Event>,
) -> Result<Json<Incident>, AppError> {
    // Process incident with metrics
    let incident = state
        .incident_processor
        .create_incident(event)
        .await
        .map_err(AppError::Internal)?;

    // Metrics are recorded inside create_incident()

    Ok(Json(incident))
}
```

### Example 2: Reusable Timing Wrapper

```rust
// src/metrics/utils.rs
use std::time::Instant;
use prometheus::HistogramVec;

pub async fn timed<F, T>(
    histogram: &HistogramVec,
    labels: &[&str],
    operation: F,
) -> T
where
    F: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = operation.await;
    let duration = start.elapsed().as_secs_f64();
    histogram.with_label_values(labels).observe(duration);
    result
}

// Usage:
use crate::metrics::{METRICS_REGISTRY, utils::timed};

pub async fn expensive_operation() -> Result<()> {
    timed(
        &METRICS_REGISTRY.job_duration_seconds,
        &["expensive_operation"],
        async {
            // Operation logic
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok(())
        }
    ).await
}
```

### Example 3: Decorator Pattern for Functions

```rust
// src/metrics/decorators.rs
use std::time::Instant;

pub struct MetricsDecorator<F> {
    func: F,
    metric_name: &'static str,
}

impl<F, T, E> MetricsDecorator<F>
where
    F: Fn() -> Result<T, E>,
{
    pub fn new(func: F, metric_name: &'static str) -> Self {
        Self { func, metric_name }
    }

    pub fn call(&self) -> Result<T, E> {
        let start = Instant::now();
        let result = (self.func)();
        let duration = start.elapsed().as_secs_f64();

        // Record metrics
        // ... implementation ...

        result
    }
}

// Macro for easier usage
#[macro_export]
macro_rules! instrumented {
    ($func:expr, $metric:expr) => {{
        let start = std::time::Instant::now();
        let result = $func;
        let duration = start.elapsed().as_secs_f64();
        $metric.observe(duration);
        result
    }};
}
```

### Example 4: Background Job Metrics

```rust
// src/jobs/executor.rs
use crate::metrics::METRICS_REGISTRY;
use std::time::Instant;

pub struct JobExecutor;

impl JobExecutor {
    pub async fn execute_job<F, T>(&self, job_type: &str, job: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = Instant::now();

        // Track active jobs
        METRICS_REGISTRY
            .jobs_active
            .with_label_values(&[job_type])
            .inc();

        // Execute
        let result = job.await;

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "failure" };

        METRICS_REGISTRY
            .jobs_total
            .with_label_values(&[job_type, status])
            .inc();

        METRICS_REGISTRY
            .jobs_duration_seconds
            .with_label_values(&[job_type])
            .observe(duration);

        METRICS_REGISTRY
            .jobs_active
            .with_label_values(&[job_type])
            .dec();

        result
    }
}

// Usage:
let executor = JobExecutor;
executor.execute_job("deduplication", async {
    // Job logic
    deduplicator.run().await
}).await?;
```

---

## 4. Testing Guide

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry() {
        let registry = MetricsRegistry::new();

        // Test counter
        registry.incidents_total
            .with_label_values(&["P0", "sentinel", "performance"])
            .inc();

        let export = registry.export().unwrap();
        assert!(export.contains("incidents_total"));
        assert!(export.contains("P0"));
    }

    #[tokio::test]
    async fn test_incident_metrics() {
        let processor = IncidentProcessor::new();

        // Create incident
        let event = Event::test_event();
        let incident = processor.create_incident(event).await.unwrap();

        // Verify metrics
        let export = METRICS_REGISTRY.export().unwrap();
        assert!(export.contains("incidents_total{"));
        assert!(export.contains("incidents_active{"));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_metrics_endpoint_integration() {
    // Start application
    let app = build_test_app();

    // Make some requests
    let client = TestClient::new(app);
    client.get("/v1/incidents").await;
    client.post("/v1/incidents").json(&test_event()).await;

    // Fetch metrics
    let metrics = client.get("/metrics").await.text();

    // Verify metrics recorded
    assert!(metrics.contains("http_requests_total"));
    assert!(metrics.contains("method=\"GET\""));
    assert!(metrics.contains("method=\"POST\""));
    assert!(metrics.contains("incidents_total"));
}
```

### Load Testing

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_metrics_recording(c: &mut Criterion) {
    c.bench_function("record_http_metric", |b| {
        b.iter(|| {
            METRICS_REGISTRY
                .http_requests_total
                .with_label_values(&["GET", "/test", "200"])
                .inc();
        });
    });
}

criterion_group!(benches, bench_metrics_recording);
criterion_main!(benches);
```

---

## 5. Deployment Guide

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'incident-manager'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'

  - job_name: 'incident-manager-ha'
    kubernetes_sd_configs:
      - role: pod
        namespaces:
          names:
            - production
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        regex: incident-manager
        action: keep
      - source_labels: [__meta_kubernetes_pod_ip]
        target_label: __address__
        replacement: ${1}:9090
```

### Kubernetes Deployment

```yaml
# kubernetes/metrics-deployment.yaml
apiVersion: v1
kind: Service
metadata:
  name: incident-manager-metrics
  labels:
    app: incident-manager
    component: metrics
spec:
  type: ClusterIP
  ports:
    - port: 9090
      targetPort: 9090
      name: metrics
  selector:
    app: incident-manager

---
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: incident-manager
  labels:
    app: incident-manager
spec:
  selector:
    matchLabels:
      app: incident-manager
  endpoints:
    - port: metrics
      interval: 15s
      path: /metrics
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  incident-manager:
    build: .
    ports:
      - "8080:8080"  # Application port
      - "9090:9090"  # Metrics port
    environment:
      - METRICS_ENABLED=true
      - METRICS_LISTEN_ADDRESS=0.0.0.0:9090

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin

volumes:
  prometheus-data:
  grafana-data:
```

---

## 6. Troubleshooting

### Issue: Metrics Not Appearing

**Symptom**: `/metrics` endpoint returns empty or missing metrics

**Solutions**:
1. Verify metrics are registered:
```rust
println!("Metrics count: {}", METRICS_REGISTRY.registry.gather().len());
```

2. Check if metrics server is running:
```bash
curl http://localhost:9090/metrics
```

3. Verify labels match when recording:
```rust
// Wrong: labels don't match definition
metric.with_label_values(&["wrong", "label", "count"]).inc();

// Correct: matches definition
metric.with_label_values(&["GET", "/incidents", "200"]).inc();
```

### Issue: High Memory Usage

**Symptom**: Memory usage grows over time

**Causes**:
- High-cardinality labels (UUIDs, timestamps)
- Too many unique label combinations

**Solutions**:
1. Audit label cardinality:
```bash
curl http://localhost:9090/metrics | grep -o 'label="[^"]*"' | sort | uniq -c | sort -nr
```

2. Remove high-cardinality labels:
```rust
// Bad: Unique per user
metric.with_label_values(&[&user_id]).inc();

// Good: Aggregate by role
metric.with_label_values(&[&user_role]).inc();
```

### Issue: Slow Metrics Collection

**Symptom**: `/metrics` endpoint is slow

**Solutions**:
1. Use faster encoders:
```rust
use prometheus::Encoder;

// Faster for large metric sets
let encoder = prometheus::ProtobufEncoder::new();
```

2. Sample metrics instead of tracking everything
3. Increase scrape interval in Prometheus

### Issue: Metrics Not Updating

**Symptom**: Metrics show stale values

**Verify**:
1. Check if code is executing:
```rust
tracing::debug!("Recording metric: {}", value);
METRICS_REGISTRY.my_metric.set(value);
```

2. Verify Prometheus is scraping:
```promql
up{job="incident-manager"}
```

3. Check metric timestamp:
```bash
curl http://localhost:9090/metrics | grep my_metric
```

---

## Summary

This implementation guide provides:

1. **Step-by-step instructions** for adding metrics to your application
2. **Complete code examples** for all common patterns
3. **Testing strategies** for validation
4. **Deployment configurations** for production
5. **Troubleshooting guide** for common issues

### Implementation Checklist

- [ ] Phase 1: Core registry implemented
- [ ] Phase 2: HTTP middleware added
- [ ] Phase 3: Metrics endpoint working
- [ ] Phase 4: Business logic instrumented
- [ ] Phase 5: System metrics collecting
- [ ] Tests passing
- [ ] Prometheus scraping successfully
- [ ] Grafana dashboards created
- [ ] Alerts configured

### Next Steps

1. Review [PROMETHEUS_METRICS_ARCHITECTURE.md](./PROMETHEUS_METRICS_ARCHITECTURE.md) for design details
2. Implement metrics following this guide
3. Set up Prometheus and Grafana
4. Create dashboards and alerts
5. Monitor and iterate on metrics

