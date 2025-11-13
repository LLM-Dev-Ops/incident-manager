# Metrics and Observability Guide

## Table of Contents
- [Overview](#overview)
- [Architecture](#architecture)
- [Available Metrics](#available-metrics)
- [Configuration](#configuration)
- [Integration with Prometheus](#integration-with-prometheus)
- [Integration with Grafana](#integration-with-grafana)
- [Best Practices](#best-practices)
- [Examples](#examples)

## Overview

The LLM Incident Manager provides comprehensive metrics collection and exposure for monitoring system performance, health, and business operations. The metrics system is built on a custom IntegrationMetrics framework that tracks performance across all LLM integrations (Sentinel, Shield, Edge-Agent, Governance) and core system components.

### Key Features

- **Real-time Metrics**: Track requests, latency, success rates, and errors in real-time
- **Thread-Safe Operations**: Atomic operations ensure accurate metrics in concurrent environments
- **Zero-Copy Snapshots**: Efficient metric snapshots without blocking operations
- **Extensible Design**: Easy to add custom metrics for new integrations
- **Prometheus-Ready**: Metrics can be exported in Prometheus format (port 9090 by default)
- **Low Overhead**: Minimal performance impact using atomic operations

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Sentinel │  │  Shield  │  │   Edge   │  │Governance│   │
│  │  Client  │  │  Client  │  │  Agent   │  │  Client  │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
│       │             │              │              │          │
│       └─────────────┴──────────────┴──────────────┘         │
│                          │                                    │
└──────────────────────────┼────────────────────────────────────┘
                           │
                           ▼
         ┌─────────────────────────────────┐
         │   IntegrationMetrics System     │
         │  ┌─────────────────────────┐   │
         │  │  AtomicU64 Counters     │   │
         │  │  - total_requests       │   │
         │  │  - successful_requests  │   │
         │  │  - failed_requests      │   │
         │  │  - total_latency_ms     │   │
         │  └─────────────────────────┘   │
         └───────────────┬─────────────────┘
                         │
                         ▼
         ┌─────────────────────────────────┐
         │    Metrics Exporter (9090)      │
         │   - Prometheus Format           │
         │   - JSON API                    │
         │   - Health Checks               │
         └───────────────┬─────────────────┘
                         │
                         ▼
         ┌─────────────────────────────────┐
         │    Monitoring Stack             │
         │  ┌───────────┐  ┌────────────┐ │
         │  │Prometheus │→ │  Grafana   │ │
         │  └───────────┘  └────────────┘ │
         └─────────────────────────────────┘
```

### Metrics Collection Flow

1. **Request Initiation**: Client makes a request to an LLM service
2. **Start Timer**: Record request start time
3. **Execute Request**: Perform the actual operation
4. **Record Result**: Update atomic counters based on success/failure
5. **Calculate Latency**: Measure elapsed time
6. **Update Metrics**: Atomically increment counters and latency
7. **Expose Metrics**: Make metrics available via HTTP endpoint

## Available Metrics

### Integration Metrics

Each LLM integration (Sentinel, Shield, Edge-Agent, Governance) exposes the following metrics:

#### Request Counters

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `llm_integration_requests_total` | Counter | Total number of requests made to the integration | `integration`, `method` |
| `llm_integration_requests_successful` | Counter | Number of successful requests | `integration`, `method` |
| `llm_integration_requests_failed` | Counter | Number of failed requests | `integration`, `method`, `error_type` |

#### Latency Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `llm_integration_latency_milliseconds_total` | Counter | Cumulative latency in milliseconds | `integration`, `method` |
| `llm_integration_latency_milliseconds_average` | Gauge | Average latency per request | `integration`, `method` |
| `llm_integration_latency_milliseconds_p95` | Gauge | 95th percentile latency | `integration`, `method` |
| `llm_integration_latency_milliseconds_p99` | Gauge | 99th percentile latency | `integration`, `method` |

#### Rate Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `llm_integration_success_rate_percent` | Gauge | Percentage of successful requests (0-100) | `integration` |
| `llm_integration_error_rate_percent` | Gauge | Percentage of failed requests (0-100) | `integration` |
| `llm_integration_requests_per_second` | Gauge | Request rate per second | `integration` |

#### Health Metrics

| Metric Name | Type | Description | Labels |
|------------|------|-------------|--------|
| `llm_integration_last_request_timestamp` | Gauge | Unix timestamp of last request | `integration` |
| `llm_integration_health_status` | Gauge | Health status (1=healthy, 0=unhealthy) | `integration` |

### Core System Metrics

#### Incident Processing

| Metric Name | Type | Description |
|------------|------|-------------|
| `incident_manager_alerts_processed_total` | Counter | Total alerts processed |
| `incident_manager_incidents_created_total` | Counter | Total incidents created |
| `incident_manager_incidents_resolved_total` | Counter | Total incidents resolved |
| `incident_manager_processing_duration_seconds` | Histogram | Time to process alerts |
| `incident_manager_deduplication_hits_total` | Counter | Duplicate alerts detected |

#### Escalation Engine

| Metric Name | Type | Description |
|------------|------|-------------|
| `incident_manager_escalations_triggered_total` | Counter | Total escalations triggered |
| `incident_manager_escalation_level` | Gauge | Current escalation level by incident |
| `incident_manager_escalation_duration_seconds` | Histogram | Time spent in escalation |

#### Enrichment Service

| Metric Name | Type | Description |
|------------|------|-------------|
| `incident_manager_enrichment_duration_seconds` | Histogram | Time to enrich incidents |
| `incident_manager_enrichment_cache_hits_total` | Counter | Cache hits during enrichment |
| `incident_manager_enrichment_cache_misses_total` | Counter | Cache misses during enrichment |
| `incident_manager_enrichment_cache_hit_rate` | Gauge | Cache hit rate percentage |

#### Correlation Engine

| Metric Name | Type | Description |
|------------|------|-------------|
| `incident_manager_correlation_groups_created_total` | Counter | Correlation groups created |
| `incident_manager_correlation_analysis_duration_seconds` | Histogram | Time to analyze correlations |
| `incident_manager_correlation_group_size` | Histogram | Size distribution of correlation groups |

#### ML Classification

| Metric Name | Type | Description |
|------------|------|-------------|
| `incident_manager_ml_predictions_total` | Counter | Total ML predictions made |
| `incident_manager_ml_prediction_confidence` | Histogram | Confidence score distribution |
| `incident_manager_ml_model_accuracy` | Gauge | Current model accuracy |
| `incident_manager_ml_training_duration_seconds` | Histogram | Time to train models |

#### Notification Service

| Metric Name | Type | Description |
|------------|------|-------------|
| `incident_manager_notifications_sent_total` | Counter | Total notifications sent by channel |
| `incident_manager_notifications_failed_total` | Counter | Failed notifications by channel |
| `incident_manager_notification_queue_size` | Gauge | Current queue size |
| `incident_manager_notification_delivery_duration_seconds` | Histogram | Time to deliver notifications |

### System Resource Metrics

| Metric Name | Type | Description |
|------------|------|-------------|
| `incident_manager_memory_usage_bytes` | Gauge | Current memory usage |
| `incident_manager_cpu_usage_percent` | Gauge | Current CPU usage |
| `incident_manager_goroutines` | Gauge | Number of active goroutines |
| `incident_manager_storage_operations_total` | Counter | Storage operations performed |
| `incident_manager_storage_operation_duration_seconds` | Histogram | Storage operation latency |

## Configuration

### Environment Variables

```bash
# Metrics server configuration
LLM_IM__SERVER__METRICS_PORT=9090
LLM_IM__OBSERVABILITY__PROMETHEUS_ENABLED=true

# Log level (affects metrics overhead)
LLM_IM__OBSERVABILITY__LOG_LEVEL=info

# Optional: OpenTelemetry integration
LLM_IM__OBSERVABILITY__OTLP_ENABLED=true
LLM_IM__OBSERVABILITY__OTLP_ENDPOINT=http://localhost:4317
```

### Configuration File (config.yaml)

```yaml
server:
  host: "0.0.0.0"
  http_port: 8080
  grpc_port: 9000
  metrics_port: 9090  # Prometheus metrics endpoint

observability:
  log_level: "info"
  json_logs: false
  prometheus_enabled: true
  otlp_enabled: false
  otlp_endpoint: null
  service_name: "llm-incident-manager"
```

### Programmatic Configuration

```rust
use llm_incident_manager::config::Config;

let mut config = Config::load()?;

// Enable Prometheus metrics
config.observability.prometheus_enabled = true;
config.server.metrics_port = 9090;

// Configure service name for metrics
config.observability.service_name = "llm-im-prod-us-east-1";
```

## Integration with Prometheus

### Prometheus Configuration

Add the following to your `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    cluster: 'llm-production'
    region: 'us-east-1'

scrape_configs:
  - job_name: 'llm-incident-manager'
    static_configs:
      - targets: ['localhost:9090']
        labels:
          service: 'incident-manager'
          environment: 'production'

    # Scrape metrics every 10 seconds
    scrape_interval: 10s
    scrape_timeout: 5s

    # Optional: Filter metrics
    metric_relabel_configs:
      - source_labels: [__name__]
        regex: 'llm_integration_.*|incident_manager_.*'
        action: keep

  # Multi-instance setup with service discovery
  - job_name: 'llm-incident-manager-cluster'
    kubernetes_sd_configs:
      - role: pod
        namespaces:
          names:
            - incident-manager

    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: keep
        regex: llm-incident-manager

      - source_labels: [__meta_kubernetes_pod_ip]
        action: replace
        target_label: __address__
        replacement: $1:9090

      - source_labels: [__meta_kubernetes_pod_name]
        action: replace
        target_label: instance
```

### Docker Compose Setup

```yaml
version: '3.8'

services:
  incident-manager:
    image: llm-incident-manager:latest
    ports:
      - "8080:8080"  # HTTP API
      - "9000:9000"  # gRPC API
      - "9090:9090"  # Metrics
    environment:
      - LLM_IM__OBSERVABILITY__PROMETHEUS_ENABLED=true
      - LLM_IM__SERVER__METRICS_PORT=9090
    networks:
      - monitoring

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
      - '--web.console.libraries=/usr/share/prometheus/console_libraries'
      - '--web.console.templates=/usr/share/prometheus/consoles'
    networks:
      - monitoring

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources
    networks:
      - monitoring

networks:
  monitoring:
    driver: bridge

volumes:
  prometheus-data:
  grafana-data:
```

### Kubernetes Deployment

```yaml
apiVersion: v1
kind: Service
metadata:
  name: incident-manager-metrics
  labels:
    app: llm-incident-manager
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "9090"
    prometheus.io/path: "/metrics"
spec:
  selector:
    app: llm-incident-manager
  ports:
    - name: metrics
      port: 9090
      targetPort: 9090
---
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: incident-manager-monitor
  labels:
    app: llm-incident-manager
spec:
  selector:
    matchLabels:
      app: llm-incident-manager
  endpoints:
    - port: metrics
      interval: 15s
      path: /metrics
```

## Integration with Grafana

### Datasource Configuration

1. Add Prometheus as a datasource in Grafana
2. Configure the datasource:

```yaml
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    jsonData:
      timeInterval: "15s"
      queryTimeout: "60s"
      httpMethod: POST
    editable: false
```

### Dashboard Templates

#### Overview Dashboard

```json
{
  "dashboard": {
    "title": "LLM Incident Manager - Overview",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "sum(rate(llm_integration_requests_total[5m])) by (integration)"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Success Rate",
        "targets": [
          {
            "expr": "100 * sum(rate(llm_integration_requests_successful[5m])) / sum(rate(llm_integration_requests_total[5m]))"
          }
        ],
        "type": "gauge"
      },
      {
        "title": "Average Latency",
        "targets": [
          {
            "expr": "llm_integration_latency_milliseconds_average"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
```

### Pre-built Dashboards

The following Grafana dashboards are available in `grafana/dashboards/`:

1. **System Overview** (`overview.json`)
   - Request rates across all integrations
   - Success/error rates
   - Average latency trends
   - System health indicators

2. **Integration Performance** (`integrations.json`)
   - Per-integration request metrics
   - Latency percentiles (p50, p95, p99)
   - Error breakdown by type
   - Request/response size distributions

3. **Incident Processing** (`incidents.json`)
   - Alerts processed per minute
   - Incident creation rate
   - Processing pipeline latency
   - Deduplication statistics

4. **Enrichment & Correlation** (`enrichment.json`)
   - Enrichment cache hit rates
   - Correlation group formation
   - Historical lookup performance
   - Service catalog queries

5. **ML Classification** (`ml-classification.json`)
   - Prediction accuracy over time
   - Confidence score distributions
   - Model training metrics
   - Feature importance

6. **Escalation & Notifications** (`escalation.json`)
   - Active escalations
   - Escalation level progression
   - Notification delivery success rates
   - Channel-specific metrics

## Best Practices

### Metric Naming Conventions

1. **Use Consistent Prefixes**: All metrics start with `llm_integration_` or `incident_manager_`
2. **Include Units**: Append units to metric names (e.g., `_seconds`, `_bytes`, `_percent`)
3. **Use Descriptive Names**: Metrics should be self-documenting
4. **Follow Prometheus Guidelines**: Use snake_case and include type suffix

### Label Usage

1. **Use Labels Sparingly**: Each unique label combination creates a new time series
2. **Avoid High Cardinality**: Don't use user IDs, timestamps, or random values as labels
3. **Common Labels**:
   - `integration`: sentinel, shield, edge-agent, governance
   - `method`: API method or operation name
   - `status`: success, failure, timeout
   - `environment`: production, staging, development

### Performance Considerations

1. **Metric Collection Overhead**: Metrics use atomic operations with minimal overhead (<1μs per operation)
2. **Memory Usage**: Each time series uses ~3KB of memory
3. **Cardinality Control**: Limit unique label combinations to prevent memory bloat
4. **Scrape Interval**: Balance freshness vs. load (15-30s recommended)

### Recording Rules

Create Prometheus recording rules for frequently-used aggregations:

```yaml
groups:
  - name: incident_manager_rules
    interval: 30s
    rules:
      # Request rate per integration
      - record: llm_integration:requests:rate5m
        expr: rate(llm_integration_requests_total[5m])

      # Success rate percentage
      - record: llm_integration:success_rate:percent
        expr: |
          100 * sum by (integration) (
            rate(llm_integration_requests_successful[5m])
          ) / sum by (integration) (
            rate(llm_integration_requests_total[5m])
          )

      # Average latency by integration
      - record: llm_integration:latency:avg_ms
        expr: |
          llm_integration_latency_milliseconds_total
          / llm_integration_requests_total
```

## Examples

### Querying Metrics

#### Get Request Rate by Integration

```promql
rate(llm_integration_requests_total[5m])
```

#### Calculate Success Rate

```promql
100 * (
  sum(llm_integration_requests_successful)
  /
  sum(llm_integration_requests_total)
)
```

#### Find Slow Integrations

```promql
llm_integration_latency_milliseconds_average > 1000
```

#### Monitor Error Rates

```promql
rate(llm_integration_requests_failed[5m]) > 0.01
```

### Custom Metric Collection

Add custom metrics to your integration:

```rust
use crate::integrations::common::IntegrationMetrics;
use std::sync::Arc;
use std::time::Instant;

pub struct MyCustomClient {
    metrics: Arc<IntegrationMetrics>,
}

impl MyCustomClient {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(IntegrationMetrics::new("my-custom-integration")),
        }
    }

    pub async fn make_request(&self) -> Result<Response, Error> {
        let start = Instant::now();

        match self.execute_request().await {
            Ok(response) => {
                let latency = start.elapsed().as_millis() as u64;
                self.metrics.record_success(latency);
                Ok(response)
            }
            Err(error) => {
                let latency = start.elapsed().as_millis() as u64;
                self.metrics.record_failure(latency);
                Err(error)
            }
        }
    }

    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }
}
```

### Accessing Metrics via HTTP

```bash
# Get all metrics in Prometheus format
curl http://localhost:9090/metrics

# Get metrics for specific integration
curl http://localhost:9090/metrics | grep sentinel

# Get metrics as JSON
curl http://localhost:8080/v1/metrics/integrations

# Get health status
curl http://localhost:8080/health
```

### Alert Rules

Create alerting rules in Prometheus:

```yaml
groups:
  - name: incident_manager_alerts
    rules:
      # High error rate alert
      - alert: HighErrorRate
        expr: |
          (
            rate(llm_integration_requests_failed[5m])
            /
            rate(llm_integration_requests_total[5m])
          ) > 0.05
        for: 5m
        labels:
          severity: warning
          component: integration
        annotations:
          summary: "High error rate on {{ $labels.integration }}"
          description: "Error rate is {{ $value | humanizePercentage }} on {{ $labels.integration }}"

      # High latency alert
      - alert: HighLatency
        expr: llm_integration_latency_milliseconds_p95 > 2000
        for: 10m
        labels:
          severity: warning
          component: integration
        annotations:
          summary: "High latency on {{ $labels.integration }}"
          description: "P95 latency is {{ $value }}ms on {{ $labels.integration }}"

      # Integration down alert
      - alert: IntegrationDown
        expr: |
          (time() - llm_integration_last_request_timestamp) > 300
          and
          llm_integration_health_status == 0
        for: 5m
        labels:
          severity: critical
          component: integration
        annotations:
          summary: "Integration {{ $labels.integration }} is down"
          description: "No requests in the last 5 minutes"

      # Processing queue backed up
      - alert: ProcessingQueueBackedUp
        expr: incident_manager_processing_queue_size > 1000
        for: 5m
        labels:
          severity: warning
          component: processing
        annotations:
          summary: "Processing queue is backed up"
          description: "Queue size is {{ $value }}, may indicate processing issues"
```

## Next Steps

- See [METRICS_IMPLEMENTATION.md](./METRICS_IMPLEMENTATION.md) for technical implementation details
- See [METRICS_OPERATIONAL_RUNBOOK.md](./METRICS_OPERATIONAL_RUNBOOK.md) for troubleshooting and operations
- Explore pre-built Grafana dashboards in `grafana/dashboards/`
- Review alert examples in `prometheus/alerts/`

## Related Documentation

- [LLM Integration Architecture](./LLM_CLIENT_ARCHITECTURE.md)
- [System Architecture](./ARCHITECTURE.md)
- [Deployment Guide](./deployment-guide.md)
- [Observability Best Practices](./TECHNICAL_DECISIONS.md#observability)
