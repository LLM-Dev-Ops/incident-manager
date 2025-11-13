# Prometheus Metrics Architecture

## Table of Contents
1. [Overview](#overview)
2. [Metrics Taxonomy](#metrics-taxonomy)
3. [Architecture Design](#architecture-design)
4. [Naming Conventions](#naming-conventions)
5. [Integration Strategy](#integration-strategy)
6. [Configuration Schema](#configuration-schema)
7. [Implementation Patterns](#implementation-patterns)
8. [Best Practices](#best-practices)

---

## 1. Overview

This document defines the enterprise-grade Prometheus metrics architecture for the LLM Incident Manager. The design prioritizes production readiness, scalability, and operational excellence while maintaining clean abstractions and minimal performance overhead.

### Design Goals

- **Production-Ready**: Battle-tested patterns for enterprise observability
- **Non-Invasive**: Metrics collection with minimal code changes
- **Performance**: Sub-millisecond overhead for metric collection
- **Commercially Viable**: Clean abstractions for easy extension
- **Graceful Degradation**: System continues if metrics fail

### Technology Stack

```
prometheus = "0.13"              # Official Prometheus client
lazy_static = "1.4"              # Static metric registry
tokio = "1.35"                   # Async runtime
axum = "0.7"                     # HTTP server (for /metrics)
```

---

## 2. Metrics Taxonomy

### 2.1 HTTP Request Metrics

Track all HTTP API request/response characteristics.

#### Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `http_requests_total` | Counter | Total HTTP requests received | `method`, `path`, `status` |
| `http_request_duration_seconds` | Histogram | HTTP request latency | `method`, `path`, `status` |
| `http_request_size_bytes` | Histogram | HTTP request body size | `method`, `path` |
| `http_response_size_bytes` | Histogram | HTTP response body size | `method`, `path` |
| `http_requests_in_flight` | Gauge | Current number of HTTP requests | `method`, `path` |

#### Histogram Buckets

```rust
// Duration buckets (seconds): 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]

// Size buckets (bytes): 100, 1KB, 10KB, 100KB, 1MB, 10MB
[100, 1024, 10240, 102400, 1048576, 10485760]
```

#### Labels

- `method`: HTTP method (GET, POST, PUT, DELETE)
- `path`: Route pattern (e.g., `/v1/incidents/:id`)
- `status`: HTTP status code (200, 404, 500)

#### Example Queries

```promql
# Request rate by endpoint
rate(http_requests_total[5m])

# P95 latency by endpoint
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Error rate percentage
100 * rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])
```

---

### 2.2 Incident Metrics

Track incident lifecycle and business metrics.

#### Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `incidents_total` | Counter | Total incidents created | `severity`, `source`, `category` |
| `incidents_resolved_total` | Counter | Total incidents resolved | `severity`, `source`, `category` |
| `incidents_active` | Gauge | Currently active incidents | `severity`, `source`, `category` |
| `incidents_deduplicated_total` | Counter | Incidents deduplicated | `source` |
| `incident_resolution_duration_seconds` | Histogram | Time to resolve incidents | `severity`, `source` |
| `incident_acknowledgment_duration_seconds` | Histogram | Time to acknowledge incidents | `severity` |
| `incident_state_transitions_total` | Counter | State transition count | `from_state`, `to_state` |

#### Histogram Buckets

```rust
// Resolution time buckets (seconds): 1m, 5m, 15m, 30m, 1h, 4h, 12h, 24h
[60, 300, 900, 1800, 3600, 14400, 43200, 86400]
```

#### Labels

- `severity`: Incident severity (P0, P1, P2, P3, P4)
- `source`: Source system (llm-sentinel, llm-shield, llm-edge-agent, llm-governance-core)
- `category`: Incident category (performance, security, availability, compliance, cost)
- `from_state`/`to_state`: State names (NEW, ACKNOWLEDGED, IN_PROGRESS, RESOLVED, CLOSED)

#### Example Queries

```promql
# Incident creation rate by severity
rate(incidents_total[5m])

# Active P0/P1 incidents
incidents_active{severity=~"P0|P1"}

# Mean time to resolve (MTTR) by severity
histogram_quantile(0.5, rate(incident_resolution_duration_seconds_bucket[1h]))

# Deduplication effectiveness
100 * rate(incidents_deduplicated_total[5m]) / rate(incidents_total[5m])
```

---

### 2.3 LLM Integration Metrics

Track LLM API calls, performance, and costs.

#### Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `llm_requests_total` | Counter | Total LLM API requests | `provider`, `model`, `operation`, `status` |
| `llm_request_duration_seconds` | Histogram | LLM request latency | `provider`, `model`, `operation` |
| `llm_tokens_total` | Counter | Total tokens consumed | `provider`, `model`, `token_type` |
| `llm_errors_total` | Counter | LLM request errors | `provider`, `model`, `error_type` |
| `llm_cost_dollars` | Counter | Estimated LLM cost | `provider`, `model` |
| `llm_rate_limit_hits_total` | Counter | Rate limit encounters | `provider` |
| `llm_circuit_breaker_state` | Gauge | Circuit breaker state (0=closed, 1=open, 2=half-open) | `provider`, `model` |

#### Histogram Buckets

```rust
// LLM latency buckets (seconds): 100ms, 250ms, 500ms, 1s, 2s, 5s, 10s, 30s
[0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]
```

#### Labels

- `provider`: LLM provider (openai, anthropic, azure, vertex)
- `model`: Model name (gpt-4, claude-3-opus-20240229, etc.)
- `operation`: Operation type (classification, enrichment, analysis, embedding)
- `status`: Success status (success, error)
- `token_type`: Token type (prompt, completion, total)
- `error_type`: Error category (rate_limit, timeout, authentication, service_unavailable)

#### Example Queries

```promql
# LLM request rate by provider
rate(llm_requests_total[5m])

# P95 LLM latency
histogram_quantile(0.95, rate(llm_request_duration_seconds_bucket[5m]))

# Token consumption rate
rate(llm_tokens_total[1h])

# Estimated hourly cost
rate(llm_cost_dollars[1h]) * 3600

# Error rate by provider
100 * rate(llm_errors_total[5m]) / rate(llm_requests_total[5m])
```

---

### 2.4 Background Job Metrics

Track async processing, workers, and job queues.

#### Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `jobs_total` | Counter | Total jobs executed | `job_type`, `status` |
| `jobs_duration_seconds` | Histogram | Job execution time | `job_type` |
| `jobs_active` | Gauge | Currently running jobs | `job_type` |
| `jobs_queue_depth` | Gauge | Jobs waiting in queue | `queue_name` |
| `jobs_failures_total` | Counter | Job failures | `job_type`, `error_type` |
| `jobs_retries_total` | Counter | Job retry attempts | `job_type` |

#### Histogram Buckets

```rust
// Job duration buckets (seconds): 100ms, 500ms, 1s, 5s, 10s, 30s, 60s
[0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]
```

#### Labels

- `job_type`: Job type (deduplication, enrichment, notification, escalation, correlation)
- `queue_name`: Queue identifier (high_priority, standard, low_priority)
- `status`: Execution result (success, failure, retry)
- `error_type`: Failure reason (timeout, dependency_error, validation_error)

#### Example Queries

```promql
# Job processing rate
rate(jobs_total[5m])

# Queue backlog
jobs_queue_depth

# Job failure rate
100 * rate(jobs_failures_total[5m]) / rate(jobs_total[5m])

# P99 job execution time
histogram_quantile(0.99, rate(jobs_duration_seconds_bucket[5m]))
```

---

### 2.5 System Health Metrics

Track system resources, connections, and health.

#### Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `system_memory_bytes` | Gauge | Memory usage | `type` (used, available, total) |
| `system_cpu_usage_percent` | Gauge | CPU utilization | `core` |
| `database_connections_active` | Gauge | Active DB connections | `pool` |
| `database_connections_idle` | Gauge | Idle DB connections | `pool` |
| `database_query_duration_seconds` | Histogram | Database query time | `operation` |
| `cache_hits_total` | Counter | Cache hits | `cache_name` |
| `cache_misses_total` | Counter | Cache misses | `cache_name` |
| `cache_size_bytes` | Gauge | Cache memory usage | `cache_name` |
| `message_queue_lag_seconds` | Gauge | Consumer lag | `topic`, `consumer_group` |
| `grpc_connections_active` | Gauge | Active gRPC connections | `service` |

#### Histogram Buckets

```rust
// DB query buckets (seconds): 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms
[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5]
```

#### Labels

- `type`: Resource type (used, available, total)
- `pool`: Connection pool name (postgres_main, redis_cache)
- `operation`: Database operation (select, insert, update, delete)
- `cache_name`: Cache identifier (deduplication, enrichment, routing)
- `topic`: Message queue topic name
- `consumer_group`: Consumer group identifier
- `service`: gRPC service name

#### Example Queries

```promql
# Memory utilization percentage
100 * system_memory_bytes{type="used"} / system_memory_bytes{type="total"}

# Database connection pool utilization
100 * database_connections_active / (database_connections_active + database_connections_idle)

# Cache hit rate
100 * rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m]))

# Message queue consumer lag
message_queue_lag_seconds > 60
```

---

## 3. Architecture Design

### 3.1 Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     METRICS ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  METRIC REGISTRY                         │  │
│  │  (Singleton, Thread-Safe, Lazy-Static)                   │  │
│  │                                                           │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │  │
│  │  │   HTTP      │  │  Incident   │  │    LLM      │      │  │
│  │  │  Metrics    │  │  Metrics    │  │  Metrics    │      │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │  │
│  │  ┌─────────────┐  ┌─────────────┐                       │  │
│  │  │    Job      │  │   System    │                       │  │
│  │  │  Metrics    │  │  Metrics    │                       │  │
│  │  └─────────────┘  └─────────────┘                       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                             │                                  │
│  ┌──────────────────────────┼───────────────────────────────┐  │
│  │                          ▼                               │  │
│  │           INSTRUMENTATION LAYER                          │  │
│  │                                                           │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │  │
│  │  │    HTTP     │  │  Function   │  │   Aspect    │      │  │
│  │  │ Middleware  │  │ Decorators  │  │  Macros     │      │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                             │                                  │
│  ┌──────────────────────────┼───────────────────────────────┐  │
│  │                          ▼                               │  │
│  │              COLLECTION LAYER                            │  │
│  │                                                           │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │  │
│  │  │  Collectors │  │  Exporters  │  │    Push     │      │  │
│  │  │  (Pull)     │  │             │  │  Gateway    │      │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                             │                                  │
│                             ▼                                  │
│                   /metrics Endpoint (HTTP)                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Module Organization

```
src/metrics/
├── mod.rs                    # Public API and registry
├── registry.rs               # Central metrics registry
├── http.rs                   # HTTP metrics
├── incident.rs               # Incident metrics
├── llm.rs                    # LLM integration metrics
├── jobs.rs                   # Background job metrics
├── system.rs                 # System health metrics
├── middleware/
│   ├── mod.rs               # Middleware exports
│   ├── http_metrics.rs      # HTTP request middleware
│   └── timing.rs            # Timing utilities
├── decorators/
│   ├── mod.rs               # Decorator exports
│   ├── timed.rs             # Function timing decorator
│   └── counted.rs           # Counter decorator
├── collectors/
│   ├── mod.rs               # Collector exports
│   ├── system.rs            # System metrics collector
│   └── custom.rs            # Custom collectors
├── config.rs                # Metrics configuration
└── exporter.rs              # Prometheus exporter endpoint
```

### 3.3 Core Components

#### 3.3.1 Metrics Registry

Central singleton registry for all metrics.

```rust
use lazy_static::lazy_static;
use prometheus::{Registry, CounterVec, HistogramVec, GaugeVec};
use std::sync::Arc;

lazy_static! {
    /// Global metrics registry
    pub static ref METRICS_REGISTRY: Arc<MetricsRegistry> =
        Arc::new(MetricsRegistry::new());
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
    pub incident_resolution_duration_seconds: HistogramVec,

    // LLM Metrics
    pub llm_requests_total: CounterVec,
    pub llm_request_duration_seconds: HistogramVec,
    pub llm_tokens_total: CounterVec,
    pub llm_cost_dollars: CounterVec,

    // Job Metrics
    pub jobs_total: CounterVec,
    pub jobs_duration_seconds: HistogramVec,
    pub jobs_queue_depth: GaugeVec,

    // System Metrics
    pub system_memory_bytes: GaugeVec,
    pub database_connections_active: GaugeVec,
    pub cache_hits_total: CounterVec,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        let registry = Registry::new();

        // Initialize all metrics
        // ... (see implementation details below)

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            // ... all other metrics
        }
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> Result<String, prometheus::Error> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}
```

#### 3.3.2 HTTP Middleware

Automatic HTTP request instrumentation.

```rust
use axum::{
    body::Body,
    extract::MatchedPath,
    http::{Request, Response},
    middleware::Next,
    response::IntoResponse,
};
use std::time::Instant;

/// Middleware for HTTP metrics collection
pub async fn http_metrics_middleware(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let start = Instant::now();

    // Extract labels
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

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

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
```

#### 3.3.3 Function Decorators

Instrumentation for specific functions.

```rust
use std::time::Instant;

/// Macro for timing function execution
#[macro_export]
macro_rules! timed {
    ($histogram:expr, $labels:expr, $body:expr) => {{
        let _timer = $crate::metrics::Timer::new($histogram, $labels);
        $body
    }};
}

/// Timer guard for automatic metric recording
pub struct Timer {
    histogram: HistogramVec,
    labels: Vec<String>,
    start: Instant,
}

impl Timer {
    pub fn new(histogram: HistogramVec, labels: Vec<String>) -> Self {
        Self {
            histogram,
            labels,
            start: Instant::now(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        let label_refs: Vec<&str> = self.labels.iter().map(|s| s.as_str()).collect();
        self.histogram
            .with_label_values(&label_refs)
            .observe(duration);
    }
}

// Usage example:
pub async fn classify_incident(event: &Event) -> Result<Severity> {
    timed!(
        METRICS_REGISTRY.llm_request_duration_seconds,
        vec!["openai".to_string(), "gpt-4".to_string(), "classification".to_string()],
        {
            // Function body
            let result = llm_client.classify(event).await?;

            METRICS_REGISTRY
                .llm_requests_total
                .with_label_values(&["openai", "gpt-4", "classification", "success"])
                .inc();

            Ok(result)
        }
    )
}
```

#### 3.3.4 System Collectors

Background collectors for system metrics.

```rust
use tokio::time::{interval, Duration};
use sysinfo::{System, SystemExt};

/// System metrics collector
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
        METRICS_REGISTRY
            .system_memory_bytes
            .with_label_values(&["used"])
            .set(self.system.used_memory() as f64);

        METRICS_REGISTRY
            .system_memory_bytes
            .with_label_values(&["total"])
            .set(self.system.total_memory() as f64);

        METRICS_REGISTRY
            .system_memory_bytes
            .with_label_values(&["available"])
            .set(self.system.available_memory() as f64);

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

---

## 4. Naming Conventions

Follow Prometheus best practices for metric naming.

### 4.1 General Rules

1. **Use snake_case**: All lowercase with underscores
2. **Include unit suffix**: `_seconds`, `_bytes`, `_total`
3. **Use descriptive names**: Self-documenting metric names
4. **Namespace prefix**: `llm_incident_manager_` for all custom metrics (optional)

### 4.2 Metric Type Suffixes

| Type | Suffix | Example |
|------|--------|---------|
| Counter | `_total` | `http_requests_total` |
| Gauge | (none) | `incidents_active` |
| Histogram | `_seconds`, `_bytes` | `http_request_duration_seconds` |
| Summary | `_seconds`, `_bytes` | `job_execution_duration_seconds` |

### 4.3 Label Guidelines

1. **Keep cardinality low**: Avoid high-cardinality labels (user IDs, timestamps)
2. **Use consistent naming**: Same label names across metrics
3. **Avoid redundancy**: Don't include metric name in label
4. **Lowercase only**: Label names and values in lowercase

#### Good Labels

```prometheus
http_requests_total{method="get", path="/v1/incidents", status="200"}
incidents_total{severity="p0", source="llm-sentinel", category="performance"}
```

#### Bad Labels

```prometheus
# HIGH CARDINALITY - DON'T DO THIS
http_requests_total{user_id="12345", timestamp="1699999999"}

# INCONSISTENT - DON'T DO THIS
incidents_total{Severity="P0", SOURCE="LLM-Sentinel"}
```

### 4.4 Histogram Bucket Selection

Choose buckets that align with SLOs and operational needs.

```rust
// Latency histograms - cover expected range + outliers
pub const LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

// Duration histograms (longer operations)
pub const DURATION_BUCKETS: &[f64] = &[
    1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0,
];

// Size histograms (bytes)
pub const SIZE_BUCKETS: &[f64] = &[
    100.0, 1024.0, 10240.0, 102400.0, 1048576.0, 10485760.0,
];
```

---

## 5. Integration Strategy

### 5.1 Non-Invasive Integration

Minimize code changes with strategic instrumentation points.

#### Level 1: Automatic (No Code Changes)

- HTTP middleware captures all API requests
- System collector runs in background
- Database connection pool metrics (via library hooks)

#### Level 2: Decorator-Based (Minimal Changes)

```rust
// Before
pub async fn process_incident(incident: Incident) -> Result<()> {
    // processing logic
}

// After - single line addition
#[timed_metric("jobs_duration_seconds", &["process_incident"])]
pub async fn process_incident(incident: Incident) -> Result<()> {
    // processing logic unchanged
}
```

#### Level 3: Explicit Instrumentation (Strategic Points)

```rust
// At critical business logic points
pub async fn create_incident(&self, event: Event) -> Result<Incident> {
    // Check deduplication
    let dedupe_result = self.deduplicator.check(&event).await?;

    if dedupe_result.is_duplicate {
        METRICS_REGISTRY
            .incidents_deduplicated_total
            .with_label_values(&[event.source()])
            .inc();
        return Ok(dedupe_result.existing_incident);
    }

    // Create new incident
    let incident = Incident::from_event(event);

    METRICS_REGISTRY
        .incidents_total
        .with_label_values(&[
            incident.severity.as_str(),
            incident.source.as_str(),
            incident.category.as_str(),
        ])
        .inc();

    METRICS_REGISTRY
        .incidents_active
        .with_label_values(&[
            incident.severity.as_str(),
            incident.source.as_str(),
            incident.category.as_str(),
        ])
        .inc();

    Ok(incident)
}
```

### 5.2 Performance Optimization

#### Use Pre-Allocated Labels

```rust
// Inefficient - allocates on every call
metrics.counter.with_label_values(&[&method, &path, &status]).inc();

// Efficient - pre-compute label vector
let labels = [method.as_str(), path.as_str(), status.as_str()];
metrics.counter.with_label_values(&labels).inc();
```

#### Batch Updates

```rust
// For high-frequency updates, batch if possible
let mut updates = Vec::new();
for event in events {
    updates.push((event.severity, event.source));
}

for (severity, source) in updates {
    metrics.incidents_total
        .with_label_values(&[severity, source])
        .inc();
}
```

#### Use Local Caching

```rust
// Cache metric objects to avoid label lookups
thread_local! {
    static HTTP_COUNTER: RefCell<HashMap<String, Counter>> =
        RefCell::new(HashMap::new());
}

fn record_http_request(method: &str, path: &str) {
    HTTP_COUNTER.with(|cache| {
        let key = format!("{}:{}", method, path);
        let counter = cache.borrow_mut()
            .entry(key.clone())
            .or_insert_with(|| {
                METRICS_REGISTRY
                    .http_requests_total
                    .with_label_values(&[method, path, "200"])
            })
            .clone();
        counter.inc();
    });
}
```

### 5.3 Graceful Degradation

Ensure metrics never crash the application.

```rust
/// Safe metric recording with error suppression
pub fn record_metric_safe<F>(operation: F)
where
    F: FnOnce() -> Result<(), prometheus::Error>,
{
    if let Err(e) = operation() {
        tracing::warn!("Failed to record metric: {}", e);
        // Continue execution - metrics are non-critical
    }
}

// Usage
record_metric_safe(|| {
    METRICS_REGISTRY
        .incidents_total
        .with_label_values(&[severity, source, category])
        .inc();
    Ok(())
});
```

#### Circuit Breaker for Metrics

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub struct MetricsCircuitBreaker {
    enabled: AtomicBool,
    failure_count: AtomicU64,
    failure_threshold: u64,
}

impl MetricsCircuitBreaker {
    pub fn new(threshold: u64) -> Self {
        Self {
            enabled: AtomicBool::new(true),
            failure_count: AtomicU64::new(0),
            failure_threshold: threshold,
        }
    }

    pub fn record<F>(&self, operation: F)
    where
        F: FnOnce() -> Result<(), prometheus::Error>,
    {
        if !self.enabled.load(Ordering::Relaxed) {
            return; // Circuit open, skip metrics
        }

        match operation() {
            Ok(_) => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            Err(e) => {
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed);
                if failures >= self.failure_threshold {
                    tracing::error!("Metrics circuit breaker opened after {} failures", failures);
                    self.enabled.store(false, Ordering::Relaxed);
                }
            }
        }
    }
}
```

---

## 6. Configuration Schema

### 6.1 TOML Configuration

```toml
[metrics]
# Enable/disable metrics collection
enabled = true

# Metrics HTTP server configuration
listen_address = "0.0.0.0:9090"
endpoint_path = "/metrics"

# Global labels applied to all metrics
[metrics.global_labels]
environment = "production"
cluster = "us-east-1"
instance = "incident-manager-01"

# Metric-specific configuration
[metrics.http]
enabled = true
track_request_size = true
track_response_size = true

[metrics.incidents]
enabled = true
track_resolution_time = true
track_acknowledgment_time = true

[metrics.llm]
enabled = true
track_token_usage = true
track_cost = true
cost_per_1k_tokens_prompt = 0.03    # GPT-4 pricing
cost_per_1k_tokens_completion = 0.06

[metrics.jobs]
enabled = true
track_queue_depth = true

[metrics.system]
enabled = true
collection_interval_seconds = 15
track_memory = true
track_cpu = true
track_disk = false

# Histogram bucket configuration
[metrics.histograms]
http_request_duration_seconds = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
llm_request_duration_seconds = [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]
incident_resolution_duration_seconds = [60, 300, 900, 1800, 3600, 14400, 43200, 86400]

# Push gateway configuration (optional)
[metrics.push_gateway]
enabled = false
url = "http://pushgateway:9091"
job_name = "incident-manager"
push_interval_seconds = 60

# Alerting rules (optional - for documentation)
[metrics.alerts]
high_error_rate_threshold = 0.05  # 5%
high_latency_p95_seconds = 1.0
active_p0_incidents_threshold = 5
```

### 6.2 Rust Configuration Struct

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub listen_address: String,
    pub endpoint_path: String,
    pub global_labels: HashMap<String, String>,
    pub http: HttpMetricsConfig,
    pub incidents: IncidentMetricsConfig,
    pub llm: LlmMetricsConfig,
    pub jobs: JobMetricsConfig,
    pub system: SystemMetricsConfig,
    pub histograms: HistogramConfig,
    pub push_gateway: Option<PushGatewayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMetricsConfig {
    pub enabled: bool,
    pub track_request_size: bool,
    pub track_response_size: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentMetricsConfig {
    pub enabled: bool,
    pub track_resolution_time: bool,
    pub track_acknowledgment_time: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMetricsConfig {
    pub enabled: bool,
    pub track_token_usage: bool,
    pub track_cost: bool,
    pub cost_per_1k_tokens_prompt: f64,
    pub cost_per_1k_tokens_completion: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetricsConfig {
    pub enabled: bool,
    pub track_queue_depth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsConfig {
    pub enabled: bool,
    pub collection_interval_seconds: u64,
    pub track_memory: bool,
    pub track_cpu: bool,
    pub track_disk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramConfig {
    pub http_request_duration_seconds: Vec<f64>,
    pub llm_request_duration_seconds: Vec<f64>,
    pub incident_resolution_duration_seconds: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushGatewayConfig {
    pub enabled: bool,
    pub url: String,
    pub job_name: String,
    pub push_interval_seconds: u64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            listen_address: "0.0.0.0:9090".to_string(),
            endpoint_path: "/metrics".to_string(),
            global_labels: HashMap::new(),
            http: HttpMetricsConfig {
                enabled: true,
                track_request_size: true,
                track_response_size: true,
            },
            incidents: IncidentMetricsConfig {
                enabled: true,
                track_resolution_time: true,
                track_acknowledgment_time: true,
            },
            llm: LlmMetricsConfig {
                enabled: true,
                track_token_usage: true,
                track_cost: true,
                cost_per_1k_tokens_prompt: 0.03,
                cost_per_1k_tokens_completion: 0.06,
            },
            jobs: JobMetricsConfig {
                enabled: true,
                track_queue_depth: true,
            },
            system: SystemMetricsConfig {
                enabled: true,
                collection_interval_seconds: 15,
                track_memory: true,
                track_cpu: true,
                track_disk: false,
            },
            histograms: HistogramConfig {
                http_request_duration_seconds: vec![
                    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                ],
                llm_request_duration_seconds: vec![
                    0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0,
                ],
                incident_resolution_duration_seconds: vec![
                    60.0, 300.0, 900.0, 1800.0, 3600.0, 14400.0, 43200.0, 86400.0,
                ],
            },
            push_gateway: None,
        }
    }
}
```

---

## 7. Implementation Patterns

### 7.1 Metrics Exporter Endpoint

```rust
use axum::{routing::get, Router};
use prometheus::TextEncoder;

/// Create metrics router
pub fn metrics_router() -> Router {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Metrics endpoint handler
async fn metrics_handler() -> impl IntoResponse {
    match METRICS_REGISTRY.export() {
        Ok(metrics) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            metrics,
        ),
        Err(e) => {
            tracing::error!("Failed to export metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                format!("Error exporting metrics: {}", e),
            )
        }
    }
}
```

### 7.2 Application Initialization

```rust
use tokio::spawn;

pub async fn init_metrics(config: MetricsConfig) -> Result<()> {
    if !config.enabled {
        tracing::info!("Metrics collection disabled");
        return Ok(());
    }

    // Initialize registry with global labels
    METRICS_REGISTRY.set_global_labels(config.global_labels);

    // Start system metrics collector
    if config.system.enabled {
        let collector = SystemMetricsCollector::new(
            config.system.collection_interval_seconds
        );
        spawn(async move {
            collector.start().await;
        });
    }

    // Start push gateway (if configured)
    if let Some(pg_config) = config.push_gateway {
        if pg_config.enabled {
            start_push_gateway(pg_config).await?;
        }
    }

    // Start metrics HTTP server
    start_metrics_server(config.listen_address, config.endpoint_path).await?;

    tracing::info!("Metrics collection initialized");
    Ok(())
}

async fn start_metrics_server(listen_addr: String, path: String) -> Result<()> {
    use axum::Server;

    let router = metrics_router();

    spawn(async move {
        tracing::info!("Metrics server listening on {}{}", listen_addr, path);

        Server::bind(&listen_addr.parse().unwrap())
            .serve(router.into_make_service())
            .await
            .expect("Metrics server failed");
    });

    Ok(())
}
```

### 7.3 Incident Lifecycle Metrics

```rust
impl IncidentManager {
    pub async fn create_incident(&self, event: Event) -> Result<Incident> {
        let start = Instant::now();

        // Business logic
        let incident = Incident::from_event(event);
        self.store.save(&incident).await?;

        // Record metrics
        METRICS_REGISTRY.incidents_total
            .with_label_values(&[
                incident.severity.as_str(),
                &incident.source,
                incident.category.as_str(),
            ])
            .inc();

        METRICS_REGISTRY.incidents_active
            .with_label_values(&[
                incident.severity.as_str(),
                &incident.source,
                incident.category.as_str(),
            ])
            .inc();

        Ok(incident)
    }

    pub async fn resolve_incident(&self, id: &str) -> Result<()> {
        let incident = self.store.get(id).await?;
        let resolution_time = incident.resolution_duration().as_secs_f64();

        // Business logic
        self.store.update_status(id, IncidentStatus::Resolved).await?;

        // Record metrics
        METRICS_REGISTRY.incidents_resolved_total
            .with_label_values(&[
                incident.severity.as_str(),
                &incident.source,
                incident.category.as_str(),
            ])
            .inc();

        METRICS_REGISTRY.incidents_active
            .with_label_values(&[
                incident.severity.as_str(),
                &incident.source,
                incident.category.as_str(),
            ])
            .dec();

        METRICS_REGISTRY.incident_resolution_duration_seconds
            .with_label_values(&[
                incident.severity.as_str(),
                &incident.source,
            ])
            .observe(resolution_time);

        Ok(())
    }
}
```

### 7.4 LLM Client Instrumentation

```rust
impl LlmClient {
    pub async fn classify(&self, event: &Event) -> Result<Classification> {
        let start = Instant::now();
        let provider = self.config.provider.clone();
        let model = self.config.model.clone();

        let result = self.call_llm(event).await;

        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };

        // Record request metrics
        METRICS_REGISTRY.llm_requests_total
            .with_label_values(&[&provider, &model, "classification", status])
            .inc();

        METRICS_REGISTRY.llm_request_duration_seconds
            .with_label_values(&[&provider, &model, "classification"])
            .observe(duration);

        match &result {
            Ok(response) => {
                // Record token usage
                METRICS_REGISTRY.llm_tokens_total
                    .with_label_values(&[&provider, &model, "prompt"])
                    .inc_by(response.prompt_tokens as f64);

                METRICS_REGISTRY.llm_tokens_total
                    .with_label_values(&[&provider, &model, "completion"])
                    .inc_by(response.completion_tokens as f64);

                // Record cost
                let cost = self.calculate_cost(response);
                METRICS_REGISTRY.llm_cost_dollars
                    .with_label_values(&[&provider, &model])
                    .inc_by(cost);
            }
            Err(e) => {
                let error_type = self.classify_error(e);
                METRICS_REGISTRY.llm_errors_total
                    .with_label_values(&[&provider, &model, &error_type])
                    .inc();
            }
        }

        result
    }
}
```

### 7.5 Background Job Metrics

```rust
pub struct JobExecutor {
    metrics_enabled: bool,
}

impl JobExecutor {
    pub async fn execute<F, T>(&self, job_type: &str, job: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        if !self.metrics_enabled {
            return job.await;
        }

        let start = Instant::now();

        // Track active jobs
        METRICS_REGISTRY.jobs_active
            .with_label_values(&[job_type])
            .inc();

        // Execute job
        let result = job.await;

        // Record completion
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "failure" };

        METRICS_REGISTRY.jobs_total
            .with_label_values(&[job_type, status])
            .inc();

        METRICS_REGISTRY.jobs_duration_seconds
            .with_label_values(&[job_type])
            .observe(duration);

        METRICS_REGISTRY.jobs_active
            .with_label_values(&[job_type])
            .dec();

        if result.is_err() {
            METRICS_REGISTRY.jobs_failures_total
                .with_label_values(&[job_type, "unknown"])
                .inc();
        }

        result
    }
}
```

---

## 8. Best Practices

### 8.1 Operational Guidelines

#### DO:
- Use histograms for latencies and durations
- Use counters for event counts
- Use gauges for current state values
- Keep label cardinality low (< 100 unique values per label)
- Document all custom metrics
- Test metric collection in CI/CD
- Monitor metrics scrape duration

#### DON'T:
- Use high-cardinality labels (UUIDs, timestamps, user IDs)
- Create metrics dynamically at runtime
- Use metrics for debugging (use logs instead)
- Expose sensitive data in labels
- Create duplicate metrics with different names

### 8.2 Performance Considerations

```rust
// Efficient: Pre-compute labels
let labels = [severity.as_str(), source.as_str()];
metric.with_label_values(&labels).inc();

// Inefficient: Repeated string allocations
metric.with_label_values(&[&severity.to_string(), &source.to_string()]).inc();

// Efficient: Batch processing
for incident in incidents {
    metrics.update(incident);
}

// Inefficient: Individual database queries
for incident in incidents {
    db.get(incident.id).await?;
    metrics.update(incident);
}
```

### 8.3 Testing Metrics

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_incident_metrics() {
        let manager = IncidentManager::new();

        // Create test incident
        let event = Event::new_test();
        let incident = manager.create_incident(event).await.unwrap();

        // Verify metrics
        let metrics = METRICS_REGISTRY.export().unwrap();
        assert!(metrics.contains("incidents_total"));
        assert!(metrics.contains("incidents_active"));

        // Verify labels
        let total = METRICS_REGISTRY.incidents_total
            .with_label_values(&["P0", "llm-sentinel", "performance"])
            .get();
        assert_eq!(total, 1.0);
    }

    #[tokio::test]
    async fn test_http_metrics_middleware() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(middleware::from_fn(http_metrics_middleware));

        let response = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify metrics recorded
        let metrics = METRICS_REGISTRY.export().unwrap();
        assert!(metrics.contains("http_requests_total"));
        assert!(metrics.contains("http_request_duration_seconds"));
    }
}
```

### 8.4 Alerting Rules

Example Prometheus alerting rules for the metrics.

```yaml
# prometheus-alerts.yml
groups:
  - name: incident_manager_alerts
    interval: 30s
    rules:
      # High error rate
      - alert: HighHTTPErrorRate
        expr: |
          100 * rate(http_requests_total{status=~"5.."}[5m])
          / rate(http_requests_total[5m]) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High HTTP error rate detected"
          description: "{{ $value }}% of HTTP requests are failing"

      # High latency
      - alert: HighAPILatency
        expr: |
          histogram_quantile(0.95,
            rate(http_request_duration_seconds_bucket[5m])
          ) > 1.0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High API latency detected"
          description: "P95 latency is {{ $value }}s"

      # Active P0 incidents
      - alert: CriticalIncidentsActive
        expr: incidents_active{severity="P0"} > 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "P0 incidents active"
          description: "{{ $value }} P0 incidents require immediate attention"

      # LLM high error rate
      - alert: LLMHighErrorRate
        expr: |
          100 * rate(llm_errors_total[5m])
          / rate(llm_requests_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High LLM error rate"
          description: "{{ $value }}% of LLM requests failing"

      # Job queue backlog
      - alert: JobQueueBacklog
        expr: jobs_queue_depth > 1000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Job queue backlog"
          description: "{{ $value }} jobs queued for processing"
```

### 8.5 Dashboard Templates

Example Grafana dashboard JSON structure.

```json
{
  "dashboard": {
    "title": "LLM Incident Manager - Overview",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])",
            "legendFormat": "{{ method }} {{ path }}"
          }
        ],
        "type": "graph"
      },
      {
        "title": "P95 Latency",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "{{ path }}"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Active Incidents by Severity",
        "targets": [
          {
            "expr": "incidents_active",
            "legendFormat": "{{ severity }}"
          }
        ],
        "type": "graph"
      },
      {
        "title": "LLM Token Usage",
        "targets": [
          {
            "expr": "rate(llm_tokens_total[1h]) * 3600",
            "legendFormat": "{{ provider }} - {{ token_type }}"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
```

---

## 9. Example Usage

### 9.1 Complete Integration Example

```rust
use axum::{Router, middleware};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_file("config.toml")?;

    // Initialize metrics
    init_metrics(config.metrics.clone()).await?;

    // Create application state
    let state = Arc::new(AppState {
        incident_manager: IncidentManager::new(config.clone()),
        llm_client: LlmClient::new(config.llm.clone()),
    });

    // Build router with metrics middleware
    let app = Router::new()
        .route("/v1/incidents", post(create_incident))
        .route("/v1/incidents/:id", get(get_incident))
        .route("/health", get(health_check))
        .layer(middleware::from_fn(http_metrics_middleware))
        .with_state(state);

    // Start metrics server on separate port
    spawn_metrics_server(config.metrics.listen_address).await?;

    // Start main application server
    let addr = config.server.listen_address.parse()?;
    tracing::info!("Server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn create_incident(
    State(state): State<Arc<AppState>>,
    Json(event): Json<Event>,
) -> Result<Json<Incident>> {
    // Automatic HTTP metrics via middleware

    // Business logic with metrics
    let incident = state.incident_manager.create_incident(event).await?;

    Ok(Json(incident))
}
```

### 9.2 Query Examples

```promql
# Dashboard queries

# Request throughput
sum(rate(http_requests_total[5m])) by (path)

# Error percentage
100 * sum(rate(http_requests_total{status=~"5.."}[5m]))
/ sum(rate(http_requests_total[5m]))

# P50, P95, P99 latency
histogram_quantile(0.50, rate(http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))

# Incident creation rate (last hour)
increase(incidents_total[1h])

# Active incidents by severity
sum(incidents_active) by (severity)

# Mean time to resolve (MTTR)
histogram_quantile(0.5, rate(incident_resolution_duration_seconds_bucket[24h])) / 60

# LLM costs per hour
rate(llm_cost_dollars[1h]) * 3600

# Top 5 slowest endpoints
topk(5, histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])))

# Cache hit rate
100 * sum(rate(cache_hits_total[5m]))
/ (sum(rate(cache_hits_total[5m])) + sum(rate(cache_misses_total[5m])))

# Database connection pool utilization
100 * database_connections_active / (database_connections_active + database_connections_idle)
```

---

## 10. Summary

This Prometheus metrics architecture provides:

1. **Comprehensive Coverage**: Metrics for all critical system components
2. **Production-Ready**: Battle-tested patterns and configurations
3. **Performance Optimized**: Sub-millisecond overhead
4. **Easy Integration**: Non-invasive middleware and decorators
5. **Commercially Viable**: Clean abstractions for customization
6. **Graceful Degradation**: System continues if metrics fail

### Next Steps

1. Implement core metrics registry (`src/metrics/registry.rs`)
2. Add HTTP middleware (`src/metrics/middleware/http_metrics.rs`)
3. Instrument critical business logic
4. Configure Prometheus scraping
5. Create Grafana dashboards
6. Set up alerting rules
7. Document custom metrics

### References

- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [Prometheus Naming Conventions](https://prometheus.io/docs/practices/naming/)
- [Prometheus Client for Rust](https://docs.rs/prometheus/)
- [Grafana Dashboards](https://grafana.com/docs/)
