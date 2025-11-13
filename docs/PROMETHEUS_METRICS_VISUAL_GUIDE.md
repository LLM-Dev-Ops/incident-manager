# Prometheus Metrics - Visual Architecture Guide

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         LLM INCIDENT MANAGER                                     │
│                         with Prometheus Metrics                                  │
└─────────────────────────────────────────────────────────────────────────────────┘

                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    ▼               ▼               ▼
        ┌────────────────┐  ┌──────────────┐  ┌──────────────┐
        │   HTTP API     │  │   gRPC API   │  │  Background  │
        │   Port 8080    │  │   Port 8081  │  │    Jobs      │
        └───────┬────────┘  └──────┬───────┘  └──────┬───────┘
                │                  │                  │
                │ (1) HTTP Request │                  │
                │     Intercepted  │                  │
                ▼                  │                  │
     ┌─────────────────────┐      │                  │
     │  HTTP Metrics       │      │                  │
     │  Middleware         │      │                  │
     │  ┌──────────────┐   │      │                  │
     │  │ Start Timer  │   │      │                  │
     │  │ Track Flight │   │      │                  │
     │  │ Log Labels   │   │      │                  │
     │  └──────┬───────┘   │      │                  │
     └─────────┼───────────┘      │                  │
               │                  │                  │
               │ (2) Pass through │                  │
               ▼                  ▼                  ▼
     ┌──────────────────────────────────────────────────────┐
     │         APPLICATION BUSINESS LOGIC                    │
     │                                                       │
     │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
     │  │  Incident   │  │     LLM     │  │    Job      │  │
     │  │  Manager    │  │   Client    │  │  Executor   │  │
     │  │             │  │             │  │             │  │
     │  │ (3) Record  │  │ (4) Record  │  │ (5) Record  │  │
     │  │  Incident   │  │    LLM      │  │    Job      │  │
     │  │  Metrics    │  │   Metrics   │  │  Metrics    │  │
     │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │
     └─────────┼─────────────────┼─────────────────┼────────┘
               │                 │                 │
               └─────────────────┼─────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │   METRICS REGISTRY     │
                    │   (Lazy Static)        │
                    │                        │
                    │  ┌──────────────────┐  │
                    │  │ Prometheus       │  │
                    │  │ Registry         │  │
                    │  │                  │  │
                    │  │ - Counters       │  │
                    │  │ - Gauges         │  │
                    │  │ - Histograms     │  │
                    │  └────────┬─────────┘  │
                    └───────────┼────────────┘
                                │
                                │ (6) Scrape Request
                                ▼
                    ┌────────────────────────┐
                    │  /metrics Endpoint     │
                    │  Port 9090             │
                    │                        │
                    │  GET /metrics          │
                    │  → Text Format         │
                    └───────────┬────────────┘
                                │
                                │ (7) Prometheus Scrapes
                                ▼
                    ┌────────────────────────┐
                    │    PROMETHEUS          │
                    │    Server              │
                    │                        │
                    │  - Time Series DB      │
                    │  - Query Engine        │
                    │  - Alert Manager       │
                    └───────────┬────────────┘
                                │
                                │ (8) Query Metrics
                                ▼
                    ┌────────────────────────┐
                    │     GRAFANA            │
                    │     Dashboard          │
                    │                        │
                    │  - Visualizations      │
                    │  - Alerts              │
                    │  - Dashboards          │
                    └────────────────────────┘
```

---

## Metrics Collection Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    METRICS COLLECTION FLOW                      │
└─────────────────────────────────────────────────────────────────┘

USER REQUEST
     │
     │ HTTP GET /v1/incidents
     │
     ▼
┌──────────────────────────────┐
│  HTTP Middleware             │
│  ┌────────────────────────┐  │
│  │ Start = now()          │  │
│  │ method = "GET"         │  │
│  │ path = "/v1/incidents" │  │
│  │                        │  │
│  │ in_flight.inc()        │  │──┐
│  └────────────────────────┘  │  │
└──────────────┬───────────────┘  │
               │                  │
               │ Pass Through     │
               ▼                  │
┌──────────────────────────┐     │
│  Route Handler           │     │
│  list_incidents()        │     │
└──────────────┬───────────┘     │
               │                  │
               │ Query Database   │
               ▼                  │
┌──────────────────────────┐     │
│  Database Layer          │     │
│  ┌────────────────────┐  │     │
│  │ Start query timer  │  │     │
│  │ Execute query      │  │     │
│  │ db_query_duration  │  │──┐  │
│  │   .observe()       │  │  │  │
│  └────────────────────┘  │  │  │
└──────────────┬───────────┘  │  │
               │              │  │
               │ Return Data   │  │
               ▼              │  │
┌──────────────────────────┐  │  │
│  Serialize Response      │  │  │
│  JSON/Protobuf          │  │  │
└──────────────┬───────────┘  │  │
               │              │  │
               │ Return to    │  │
               │ Middleware   │  │
               ▼              │  │
┌──────────────────────────────┐│ │
│  HTTP Middleware (cont'd)    ││ │
│  ┌────────────────────────┐  ││ │
│  │ duration = now() - start│ ││ │
│  │ status = 200           │  ││ │
│  │                        │  ││ │
│  │ requests_total.inc()   │  ││◄┘
│  │ duration_seconds       │  ││
│  │   .observe()           │  ││
│  │ in_flight.dec()        │  ││◄─┘
│  └────────────────────────┘  ││
└──────────────┬────────────────┘│
               │                 │
               │ Response        │
               ▼                 │
           USER RESPONSE         │
                                 │
                                 │
┌────────────────────────────────┼───────────────────────────┐
│       METRICS REGISTRY         │                           │
│                                ▼                           │
│  http_requests_total{method="GET",path="/v1/incidents",   │
│                      status="200"} = 1                     │
│                                                            │
│  http_request_duration_seconds_sum{...} = 0.025           │
│  http_request_duration_seconds_count{...} = 1             │
│                                                            │
│  http_requests_in_flight{...} = 0                         │
│                                                            │
│  database_query_duration_seconds{operation="select"} ...  │
└────────────────────────────────────────────────────────────┘
```

---

## Incident Lifecycle Metrics

```
┌─────────────────────────────────────────────────────────────────┐
│              INCIDENT LIFECYCLE WITH METRICS                    │
└─────────────────────────────────────────────────────────────────┘

    EVENT RECEIVED
         │
         │ (from Sentinel/Shield/Edge-Agent)
         │
         ▼
    ┌─────────────┐
    │  Validate   │
    │  & Parse    │
    └──────┬──────┘
           │
           │ Check Deduplication
           ▼
    ┌─────────────┐              YES
    │ Dedupe?     │───────────────────► incidents_deduplicated_total.inc()
    └──────┬──────┘                     └─► Return Existing
           │ NO
           │
           │ Create New Incident
           ▼
    ┌──────────────────────────────────────────────────────┐
    │  incidents_total.inc()                               │
    │    labels: {severity, source, category}              │
    │                                                       │
    │  incidents_active.inc()                              │
    │    labels: {severity, source, category}              │
    │                                                       │
    │  incident.state = NEW                                │
    │  incident.created_at = now()                         │
    └──────────────────────┬───────────────────────────────┘
                           │
                           │ INCIDENT CREATED
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
    ┌─────────┐      ┌─────────┐     ┌──────────┐
    │ Routing │      │  Enrich │     │  Notify  │
    │ Engine  │      │  Data   │     │  Teams   │
    └─────────┘      └─────────┘     └──────────┘
          │                │                │
          │                │                │
          └────────────────┼────────────────┘
                           │
                           │ Assign to Team/User
                           │
                           ▼
                    ┌──────────────┐
                    │  ASSIGNED    │
                    └──────┬───────┘
                           │
                           │ Team Acknowledges
                           ▼
    ┌──────────────────────────────────────────────────────┐
    │  incident.state = ACKNOWLEDGED                       │
    │  incident.acknowledged_at = now()                    │
    │                                                       │
    │  incident_acknowledgment_duration_seconds            │
    │    .observe(acknowledged_at - created_at)            │
    │    labels: {severity}                                │
    └──────────────────────┬───────────────────────────────┘
                           │
                           │ Team Working
                           ▼
                    ┌──────────────┐
                    │ IN_PROGRESS  │
                    └──────┬───────┘
                           │
                           │ Investigation Complete
                           │
                           ▼
    ┌──────────────────────────────────────────────────────┐
    │  incident.state = RESOLVED                           │
    │  incident.resolved_at = now()                        │
    │                                                       │
    │  incidents_resolved_total.inc()                      │
    │    labels: {severity, source, category}              │
    │                                                       │
    │  incidents_active.dec()                              │
    │    labels: {severity, source, category}              │
    │                                                       │
    │  incident_resolution_duration_seconds                │
    │    .observe(resolved_at - created_at)                │
    │    labels: {severity, source}                        │
    └──────────────────────────────────────────────────────┘
                           │
                           │ INCIDENT RESOLVED
                           ▼
                      [End State]

METRICS CAPTURED:
─────────────────────────────────────────────────────────────────
Creation:     incidents_total, incidents_active
Deduplication: incidents_deduplicated_total
Acknowledgment: incident_acknowledgment_duration_seconds
Resolution:   incidents_resolved_total, incidents_active (dec),
              incident_resolution_duration_seconds
```

---

## LLM Integration Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                  LLM INTEGRATION METRICS FLOW                   │
└─────────────────────────────────────────────────────────────────┘

INCIDENT NEEDS CLASSIFICATION
         │
         ▼
┌────────────────────────┐
│  LLM Client            │
│  classify(incident)    │
│                        │
│  ┌──────────────────┐  │
│  │ start = now()    │  │
│  │ provider = "..."  │  │
│  │ model = "..."     │  │
│  └──────┬───────────┘  │
└─────────┼──────────────┘
          │
          │ Call LLM API
          ▼
┌──────────────────────────────┐
│  External LLM Provider       │
│  (OpenAI/Anthropic/Azure)    │
│                              │
│  Request Processing...       │
│  ┌────────────────────────┐  │
│  │ Analyze incident       │  │
│  │ Classify severity      │  │
│  │ Generate insights      │  │
│  └────────────────────────┘  │
└──────────────┬───────────────┘
               │
               │ Response (or Error)
               ▼
┌────────────────────────────────────────────────────────┐
│  LLM Client Response Handler                          │
│                                                        │
│  duration = now() - start                             │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │ SUCCESS PATH                                      │ │
│  │                                                   │ │
│  │ llm_requests_total.inc()                         │ │
│  │   labels: {provider, model, operation, "success"}│ │
│  │                                                   │ │
│  │ llm_request_duration_seconds.observe(duration)   │ │
│  │   labels: {provider, model, operation}           │ │
│  │                                                   │ │
│  │ llm_tokens_total.inc_by(prompt_tokens)          │ │
│  │   labels: {provider, model, "prompt"}            │ │
│  │                                                   │ │
│  │ llm_tokens_total.inc_by(completion_tokens)      │ │
│  │   labels: {provider, model, "completion"}        │ │
│  │                                                   │ │
│  │ llm_cost_dollars.inc_by(calculated_cost)        │ │
│  │   labels: {provider, model}                      │ │
│  └──────────────────────────────────────────────────┘ │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │ ERROR PATH                                        │ │
│  │                                                   │ │
│  │ llm_requests_total.inc()                         │ │
│  │   labels: {provider, model, operation, "error"}  │ │
│  │                                                   │ │
│  │ llm_errors_total.inc()                           │ │
│  │   labels: {provider, model, error_type}          │ │
│  │                                                   │ │
│  │ error_type:                                       │ │
│  │   - "rate_limit"                                 │ │
│  │   - "timeout"                                     │ │
│  │   - "authentication"                             │ │
│  │   - "service_unavailable"                        │ │
│  └──────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────┘
               │
               │ Return Classification
               ▼
         USE RESULT

METRICS CAPTURED:
─────────────────────────────────────────────────────────────────
Request Count:  llm_requests_total {provider, model, operation, status}
Latency:        llm_request_duration_seconds {provider, model, operation}
Token Usage:    llm_tokens_total {provider, model, token_type}
Cost:           llm_cost_dollars {provider, model}
Errors:         llm_errors_total {provider, model, error_type}
```

---

## Metrics Export & Scraping

```
┌─────────────────────────────────────────────────────────────────┐
│              METRICS EXPORT AND SCRAPING FLOW                   │
└─────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────┐
│  APPLICATION RUNTIME                                       │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  METRICS REGISTRY (In-Memory)                        │ │
│  │                                                       │ │
│  │  http_requests_total = 1523                          │ │
│  │  http_request_duration_seconds_sum = 45.23           │ │
│  │  incidents_active = 12                               │ │
│  │  llm_tokens_total = 156789                           │ │
│  │  ...                                                  │ │
│  └──────────────────────────────────────────────────────┘ │
│                           │                               │
│                           │ Registry.gather()             │
│                           ▼                               │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Prometheus Text Encoder                             │ │
│  │                                                       │ │
│  │  # HELP http_requests_total Total HTTP requests      │ │
│  │  # TYPE http_requests_total counter                  │ │
│  │  http_requests_total{method="GET",path="/incidents", │ │
│  │    status="200"} 1523 1699999999000                  │ │
│  │                                                       │ │
│  │  # HELP http_request_duration_seconds HTTP latency   │ │
│  │  # TYPE http_request_duration_seconds histogram      │ │
│  │  http_request_duration_seconds_bucket{le="0.005"} 12│ │
│  │  http_request_duration_seconds_bucket{le="0.01"} 45 │ │
│  │  http_request_duration_seconds_sum 45.23            │ │
│  │  http_request_duration_seconds_count 1523           │ │
│  │  ...                                                  │ │
│  └──────────────────────────────────────────────────────┘ │
│                           │                               │
│                           │ /metrics Endpoint             │
│                           ▼                               │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  HTTP Server (Port 9090)                             │ │
│  │                                                       │ │
│  │  GET /metrics  →  Return Text                        │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────┬───────────────────────────────┘
                             │
                             │ (Every 15s)
                             │ HTTP GET Request
                             ▼
┌────────────────────────────────────────────────────────────┐
│  PROMETHEUS SERVER                                         │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Scraper                                             │ │
│  │                                                       │ │
│  │  targets:                                            │ │
│  │    - incident-manager-1:9090                         │ │
│  │    - incident-manager-2:9090                         │ │
│  │    - incident-manager-3:9090                         │ │
│  │                                                       │ │
│  │  scrape_interval: 15s                                │ │
│  └────────────────┬─────────────────────────────────────┘ │
│                   │                                        │
│                   │ Parse & Store                          │
│                   ▼                                        │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Time Series Database                                │ │
│  │                                                       │ │
│  │  metric{labels} = value @ timestamp                  │ │
│  │                                                       │ │
│  │  http_requests_total{...} = 1523 @ 1699999999       │ │
│  │  http_requests_total{...} = 1524 @ 1700000014       │ │
│  │  http_requests_total{...} = 1526 @ 1700000029       │ │
│  │  ...                                                  │ │
│  └──────────────────┬───────────────────────────────────┘ │
└────────────────────┼────────────────────────────────────────┘
                     │
                     │ PromQL Queries
                     ▼
┌────────────────────────────────────────────────────────────┐
│  GRAFANA / DASHBOARDS                                      │
│                                                            │
│  Query: rate(http_requests_total[5m])                     │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Request Rate Over Time                              │ │
│  │  ┌────────────────────────────────────────────────┐  │ │
│  │  │                                   /\           │  │ │
│  │  │                                  /  \          │  │ │
│  │  │                        /\       /    \         │  │ │
│  │  │                       /  \     /      \        │  │ │
│  │  │            /\        /    \___/        \       │  │ │
│  │  │           /  \______/                   \___   │  │ │
│  │  │  ________/                                      │  │ │
│  │  └────────────────────────────────────────────────┘  │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Active Incidents by Severity                        │ │
│  │  ┌────────────────────────────────────────────────┐  │ │
│  │  │  P0: 2    ████                                 │  │ │
│  │  │  P1: 5    ██████████                           │  │ │
│  │  │  P2: 12   ████████████████████████             │  │ │
│  │  │  P3: 23   ██████████████████████████████████   │  │ │
│  │  └────────────────────────────────────────────────┘  │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

---

## Module Organization

```
src/metrics/
│
├── mod.rs                          # Public API
│   ├── pub use registry::*
│   ├── pub use middleware::*
│   └── pub use collectors::*
│
├── registry.rs                     # Central metrics registry
│   ├── struct MetricsRegistry
│   │   ├── http_requests_total: CounterVec
│   │   ├── http_request_duration_seconds: HistogramVec
│   │   ├── incidents_total: CounterVec
│   │   ├── llm_requests_total: CounterVec
│   │   └── ...
│   └── impl MetricsRegistry
│       ├── fn new() -> Self
│       └── fn export() -> String
│
├── config.rs                       # Configuration structs
│   ├── struct MetricsConfig
│   ├── struct HttpMetricsConfig
│   ├── struct IncidentMetricsConfig
│   └── struct LlmMetricsConfig
│
├── exporter.rs                     # /metrics endpoint
│   ├── fn metrics_router() -> Router
│   └── async fn metrics_handler() -> Response
│
├── server.rs                       # Standalone metrics server
│   └── async fn start_metrics_server(addr: SocketAddr)
│
├── middleware/
│   ├── mod.rs
│   └── http_metrics.rs             # HTTP middleware
│       └── async fn http_metrics_middleware(req, next)
│
├── collectors/
│   ├── mod.rs
│   ├── system.rs                   # System metrics collector
│   │   └── struct SystemMetricsCollector
│   │       ├── fn new() -> Self
│   │       └── async fn start(self)
│   └── custom.rs                   # Custom collectors
│
└── utils.rs                        # Helper utilities
    ├── async fn timed<F>(histogram, labels, operation)
    └── macro instrumented!()
```

---

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    KUBERNETES DEPLOYMENT                             │
└─────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│  incident-manager Deployment (3 replicas)                      │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐│
│  │ Pod 1           │  │ Pod 2           │  │ Pod 3           ││
│  │                 │  │                 │  │                 ││
│  │ App: 8080       │  │ App: 8080       │  │ App: 8080       ││
│  │ Metrics: 9090   │  │ Metrics: 9090   │  │ Metrics: 9090   ││
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘│
└───────────┼────────────────────┼────────────────────┼─────────┘
            │                    │                    │
            │                    │                    │
┌───────────┼────────────────────┼────────────────────┼─────────┐
│           │     Service: incident-manager-metrics   │         │
│           │     Port: 9090                          │         │
│           │     Selector: app=incident-manager      │         │
└───────────┼────────────────────┼────────────────────┼─────────┘
            │                    │                    │
            │ Scrape Every 15s   │                    │
            └────────────────────┼────────────────────┘
                                 │
                                 ▼
┌────────────────────────────────────────────────────────────────┐
│  ServiceMonitor: incident-manager                              │
│                                                                 │
│  spec:                                                         │
│    endpoints:                                                  │
│      - port: metrics                                           │
│        interval: 15s                                           │
│        path: /metrics                                          │
└────────────────────────────┬───────────────────────────────────┘
                             │
                             │ Prometheus Operator watches
                             ▼
┌────────────────────────────────────────────────────────────────┐
│  Prometheus Server                                             │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  Scrape Configs (Auto-generated)                         │ │
│  │                                                           │ │
│  │  - job: incident-manager/incident-manager-metrics/0      │ │
│  │    targets:                                               │ │
│  │      - 10.0.1.23:9090  (pod-1)                          │ │
│  │      - 10.0.1.24:9090  (pod-2)                          │ │
│  │      - 10.0.1.25:9090  (pod-3)                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                 │
│  Storage: PersistentVolume (100GB)                             │
│  Retention: 30 days                                            │
└────────────────────────────┬───────────────────────────────────┘
                             │
                             │ PromQL API
                             ▼
┌────────────────────────────────────────────────────────────────┐
│  Grafana                                                       │
│                                                                 │
│  Data Source: Prometheus                                       │
│  Dashboards:                                                   │
│    - Incident Manager Overview                                 │
│    - HTTP Metrics                                              │
│    - LLM Integration Metrics                                   │
│    - System Health                                             │
└────────────────────────────────────────────────────────────────┘
```

---

## Dashboard Example

```
┌─────────────────────────────────────────────────────────────────────┐
│  INCIDENT MANAGER - OVERVIEW DASHBOARD                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐        │
│  │  Request Rate  │  │  P95 Latency   │  │  Error Rate    │        │
│  │                │  │                │  │                │        │
│  │   142.5 req/s  │  │     23.4 ms    │  │     0.12 %     │        │
│  │                │  │                │  │                │        │
│  │  ▲ +12%        │  │  ▼ -8%         │  │  ▼ -0.03%      │        │
│  └────────────────┘  └────────────────┘  └────────────────┘        │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  HTTP Request Rate by Endpoint                              │   │
│  │                                                              │   │
│  │  /v1/incidents      ████████████████████  (85 req/s)       │   │
│  │  /v1/incidents/:id  ████████  (32 req/s)                   │   │
│  │  /health            ███  (15 req/s)                         │   │
│  │  /metrics           ██  (10 req/s)                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Active Incidents by Severity                               │   │
│  │                                                              │   │
│  │      │                                                       │   │
│  │   50 │     ███                                              │   │
│  │      │     ███                                              │   │
│  │   40 │     ███  ████                                        │   │
│  │      │     ███  ████                                        │   │
│  │   30 │     ███  ████  ██████                               │   │
│  │      │     ███  ████  ██████                               │   │
│  │   20 │ ██  ███  ████  ██████  ████████                     │   │
│  │      │ ██  ███  ████  ██████  ████████                     │   │
│  │   10 │ ██  ███  ████  ██████  ████████  ██████████        │   │
│  │      │ ██  ███  ████  ██████  ████████  ██████████        │   │
│  │    0 └─P0───P1───P2────P3──────P4───────────────────        │   │
│  │         2    5    12     23      35                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  LLM Integration Metrics                                    │   │
│  │                                                              │   │
│  │  Provider: OpenAI                                           │   │
│  │  Requests: 1,234 (last hour)                                │   │
│  │  Tokens: 456,789                                            │   │
│  │  Cost: $13.45                                               │   │
│  │                                                              │   │
│  │  ┌──────────────────────────────────────────────────────┐  │   │
│  │  │  Latency Distribution                                │  │   │
│  │  │                                     /\               │  │   │
│  │  │                                    /  \              │  │   │
│  │  │                          /\       /    \             │  │   │
│  │  │                         /  \     /      \            │  │   │
│  │  │              /\        /    \___/        \___        │  │   │
│  │  │             /  \______/                              │  │   │
│  │  │  __________/                                          │  │   │
│  │  │  P50: 0.8s   P95: 2.3s   P99: 5.1s                  │  │   │
│  │  └──────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  System Health                                              │   │
│  │                                                              │   │
│  │  Memory: 1.2 GB / 4.0 GB  (30%)   ████░░░░░░░░░░░░         │   │
│  │  CPU: 45%                          ███████░░░░░░░░░         │   │
│  │  DB Connections: 23 / 50           ████████░░░░░░░         │   │
│  │  Cache Hit Rate: 87.3%             █████████████░░         │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Quick Start Flowchart

```
START: Add Prometheus Metrics
         │
         ▼
[1] Add dependencies to Cargo.toml
    - prometheus = "0.13"
    - lazy_static = "1.4"
         │
         ▼
[2] Create src/metrics/registry.rs
    - Define MetricsRegistry struct
    - Register all metrics
    - Implement export() method
         │
         ▼
[3] Create src/metrics/middleware/http_metrics.rs
    - Implement http_metrics_middleware()
    - Track request/response metrics
         │
         ▼
[4] Add middleware to router
    - .layer(middleware::from_fn(http_metrics_middleware))
         │
         ▼
[5] Create /metrics endpoint
    - src/metrics/exporter.rs
    - GET /metrics → export()
         │
         ▼
[6] Instrument business logic
    - Incident creation/resolution
    - LLM API calls
    - Background jobs
         │
         ▼
[7] Start metrics server
    - Separate port (9090)
    - spawn(start_metrics_server())
         │
         ▼
[8] Configure Prometheus
    - prometheus.yml
    - scrape_configs
         │
         ▼
[9] Create Grafana dashboards
    - Connect to Prometheus
    - Import dashboards
         │
         ▼
[10] Set up alerts
     - Alert rules
     - Alert Manager
         │
         ▼
      DONE!
```

---

## References

- **Architecture**: [PROMETHEUS_METRICS_ARCHITECTURE.md](./PROMETHEUS_METRICS_ARCHITECTURE.md)
- **Implementation**: [PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md)
- **Quick Reference**: [PROMETHEUS_METRICS_QUICK_REFERENCE.md](./PROMETHEUS_METRICS_QUICK_REFERENCE.md)
