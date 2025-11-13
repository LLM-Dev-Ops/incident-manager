# Prometheus Metrics - Documentation Index

Welcome to the comprehensive Prometheus metrics documentation for the LLM Incident Manager. This documentation suite provides everything needed to design, implement, and operate enterprise-grade observability.

---

## Documentation Suite Overview

This documentation is organized into four complementary documents:

### 1. Architecture Document
**File**: [PROMETHEUS_METRICS_ARCHITECTURE.md](./PROMETHEUS_METRICS_ARCHITECTURE.md)
**Size**: ~50KB, 1,685 lines
**Purpose**: Comprehensive architectural design and specifications

**Contents**:
- Complete metrics taxonomy (HTTP, Incident, LLM, Jobs, System)
- Architecture design patterns and component organization
- Prometheus naming conventions and best practices
- Integration strategy and graceful degradation
- Configuration schema (TOML + Rust)
- Implementation patterns with code examples
- Dashboard templates and alert rules

**When to use**:
- Designing the metrics system
- Understanding the overall architecture
- Making architectural decisions
- Reference for metric definitions

---

### 2. Implementation Guide
**File**: [PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md)
**Size**: ~32KB, 1,314 lines
**Purpose**: Step-by-step implementation instructions with complete code

**Contents**:
- Quick start guide
- Phase-by-phase implementation (5 phases)
- Complete code examples for all components
- Testing guide (unit tests, integration tests, load tests)
- Deployment configurations (Kubernetes, Docker Compose, Prometheus)
- Troubleshooting guide with solutions

**When to use**:
- Implementing metrics in your application
- Following step-by-step tutorial
- Looking for code examples
- Debugging implementation issues

---

### 3. Quick Reference
**File**: [PROMETHEUS_METRICS_QUICK_REFERENCE.md](./PROMETHEUS_METRICS_QUICK_REFERENCE.md)
**Size**: ~9KB, 503 lines
**Purpose**: Fast lookup reference for common operations

**Contents**:
- Common PromQL queries by category
- Metric recording patterns
- Configuration snippets
- Alert rule examples
- Dashboard query examples
- Troubleshooting quick tips
- Naming conventions cheatsheet

**When to use**:
- Quick lookup of metric queries
- Finding alert rule examples
- Need code snippet fast
- Runtime troubleshooting

---

### 4. Visual Guide
**File**: [PROMETHEUS_METRICS_VISUAL_GUIDE.md](./PROMETHEUS_METRICS_VISUAL_GUIDE.md)
**Size**: ~52KB, 770 lines
**Purpose**: Visual architecture diagrams and flowcharts

**Contents**:
- System architecture diagram
- Metrics collection flow
- Incident lifecycle with metrics
- LLM integration flow
- Export and scraping flow
- Module organization tree
- Deployment architecture
- Dashboard examples
- Quick start flowchart

**When to use**:
- Understanding system flows
- Visualizing architecture
- Onboarding new team members
- Presenting to stakeholders

---

## Getting Started

### For Architects

Start here:
1. [Architecture Document](./PROMETHEUS_METRICS_ARCHITECTURE.md) - Read sections 1-3
2. [Visual Guide](./PROMETHEUS_METRICS_VISUAL_GUIDE.md) - Review diagrams
3. [Architecture Document](./PROMETHEUS_METRICS_ARCHITECTURE.md) - Read sections 4-9

### For Developers

Start here:
1. [Implementation Guide](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md) - Quick Start
2. [Implementation Guide](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md) - Phase 1-5
3. [Quick Reference](./PROMETHEUS_METRICS_QUICK_REFERENCE.md) - Bookmark for later
4. [Visual Guide](./PROMETHEUS_METRICS_VISUAL_GUIDE.md) - Understand flows

### For Operators

Start here:
1. [Quick Reference](./PROMETHEUS_METRICS_QUICK_REFERENCE.md) - Read all sections
2. [Architecture Document](./PROMETHEUS_METRICS_ARCHITECTURE.md) - Section 8 (Best Practices)
3. [Implementation Guide](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md) - Section 6 (Troubleshooting)

---

## Key Features

### Enterprise-Grade Design
- Production-ready patterns
- Scalable architecture
- Performance optimized (sub-millisecond overhead)
- Graceful degradation

### Comprehensive Coverage
- HTTP request metrics
- Incident lifecycle metrics
- LLM integration metrics (with cost tracking)
- Background job metrics
- System health metrics

### Developer-Friendly
- Non-invasive integration
- Minimal code changes
- Clear examples
- Strong typing (Rust)

### Operations-Ready
- Prometheus integration
- Grafana dashboards
- Alert rules included
- Troubleshooting guide

---

## Metrics Categories

### HTTP Metrics
- Request rate
- Latency (P50, P95, P99)
- Error rate
- In-flight requests
- Request/response sizes

**Key Metrics**: `http_requests_total`, `http_request_duration_seconds`

### Incident Metrics
- Creation rate
- Active incidents
- Resolution time (MTTR)
- Acknowledgment time (MTTA)
- Deduplication effectiveness

**Key Metrics**: `incidents_total`, `incidents_active`, `incident_resolution_duration_seconds`

### LLM Metrics
- Request rate by provider
- Latency by model
- Token consumption
- Cost tracking
- Error rates by type

**Key Metrics**: `llm_requests_total`, `llm_tokens_total`, `llm_cost_dollars`

### Job Metrics
- Execution rate
- Queue depth
- Success/failure rate
- Execution time

**Key Metrics**: `jobs_total`, `jobs_queue_depth`, `jobs_duration_seconds`

### System Metrics
- Memory usage
- CPU utilization
- Database connections
- Cache hit rate
- Message queue lag

**Key Metrics**: `system_memory_bytes`, `database_connections_active`, `cache_hits_total`

---

## Implementation Phases

### Phase 1: Core Registry (Day 1)
- Create metrics module structure
- Implement MetricsRegistry
- Register core metrics
- Add unit tests

**Deliverable**: Working metrics registry

### Phase 2: HTTP Middleware (Day 2)
- Implement HTTP metrics middleware
- Add to application router
- Create /metrics endpoint
- Test HTTP metrics

**Deliverable**: Automatic HTTP metrics collection

### Phase 3: Metrics Endpoint (Day 2)
- Create metrics exporter
- Start metrics server (port 9090)
- Configure Prometheus scraping
- Verify metrics export

**Deliverable**: Prometheus can scrape metrics

### Phase 4: Business Logic (Day 3-4)
- Instrument incident creation/resolution
- Add LLM client metrics
- Track background jobs
- Add custom metrics

**Deliverable**: Full business metrics coverage

### Phase 5: System Metrics (Day 5)
- Implement system collector
- Start background collector
- Add custom collectors
- Performance optimization

**Deliverable**: Complete observability

---

## Quick Examples

### Recording a Counter
```rust
METRICS_REGISTRY.incidents_total
    .with_label_values(&["P0", "sentinel", "performance"])
    .inc();
```

### Recording a Histogram
```rust
let duration = start.elapsed().as_secs_f64();
METRICS_REGISTRY.http_request_duration_seconds
    .with_label_values(&["GET", "/incidents", "200"])
    .observe(duration);
```

### Querying in PromQL
```promql
# Request rate
rate(http_requests_total[5m])

# P95 latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Active P0 incidents
incidents_active{severity="P0"}
```

---

## Configuration

### Minimal Configuration
```toml
[metrics]
enabled = true
listen_address = "0.0.0.0:9090"
```

### Production Configuration
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
cost_per_1k_tokens_prompt = 0.03
cost_per_1k_tokens_completion = 0.06

[metrics.system]
enabled = true
collection_interval_seconds = 15
```

---

## Common Queries

### Request Rate
```promql
sum(rate(http_requests_total[5m])) by (path)
```

### Error Rate
```promql
100 * rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])
```

### P95 Latency
```promql
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

### MTTR
```promql
histogram_quantile(0.5, rate(incident_resolution_duration_seconds_bucket[1h])) / 60
```

### LLM Cost per Hour
```promql
rate(llm_cost_dollars[1h]) * 3600
```

---

## Alert Examples

### High Error Rate
```yaml
- alert: HighErrorRate
  expr: |
    100 * rate(http_requests_total{status=~"5.."}[5m])
    / rate(http_requests_total[5m]) > 5
  for: 5m
  labels:
    severity: warning
```

### Critical Incidents Active
```yaml
- alert: CriticalIncidents
  expr: incidents_active{severity="P0"} > 0
  for: 1m
  labels:
    severity: critical
```

### LLM High Error Rate
```yaml
- alert: LLMHighErrorRate
  expr: |
    100 * rate(llm_errors_total[5m])
    / rate(llm_requests_total[5m]) > 10
  for: 5m
  labels:
    severity: warning
```

---

## Deployment

### Docker
```yaml
services:
  incident-manager:
    ports:
      - "8080:8080"  # Application
      - "9090:9090"  # Metrics

  prometheus:
    image: prom/prometheus
    ports:
      - "9091:9090"
```

### Kubernetes
```yaml
apiVersion: v1
kind: Service
metadata:
  name: incident-manager-metrics
spec:
  ports:
    - port: 9090
      name: metrics
```

---

## Best Practices

### Do's
- ✅ Use histograms for latencies
- ✅ Use counters for event counts
- ✅ Use gauges for current values
- ✅ Keep label cardinality low
- ✅ Use descriptive metric names
- ✅ Follow Prometheus naming conventions

### Don'ts
- ❌ Use high-cardinality labels (UUIDs, timestamps)
- ❌ Create metrics dynamically at runtime
- ❌ Expose sensitive data in labels
- ❌ Use metrics for debugging (use logs)
- ❌ Create duplicate metrics

---

## Troubleshooting

### No Metrics Appearing
```bash
# Check metrics endpoint
curl http://localhost:9090/metrics

# Verify Prometheus
curl http://prometheus:9090/api/v1/targets
```

### High Memory Usage
```bash
# Check label cardinality
curl http://localhost:9090/metrics | \
  grep -o 'label="[^"]*"' | sort | uniq -c | sort -nr
```

### Metrics Not Updating
```promql
# Check Prometheus is scraping
up{job="incident-manager"}
```

---

## Performance

### Metrics Collection Overhead
- Counter increment: ~50 nanoseconds
- Histogram observation: ~200 nanoseconds
- Metric export: ~1-5 milliseconds

### Memory Usage
- Empty registry: ~1 MB
- 100 metrics: ~5 MB
- 1000 metrics: ~20 MB

### Prometheus Scraping
- Recommended interval: 15 seconds
- Scrape duration: <100ms
- Maximum series: 1-2 million per instance

---

## Testing

### Unit Tests
```rust
#[test]
fn test_metric() {
    METRICS_REGISTRY.my_metric
        .with_label_values(&["label"])
        .inc();

    let export = METRICS_REGISTRY.export().unwrap();
    assert!(export.contains("my_metric"));
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_metrics_endpoint() {
    let app = build_test_app();
    let client = TestClient::new(app);

    let metrics = client.get("/metrics").await.text();
    assert!(metrics.contains("http_requests_total"));
}
```

---

## Support & Resources

### Documentation
- [Architecture](./PROMETHEUS_METRICS_ARCHITECTURE.md)
- [Implementation](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md)
- [Quick Reference](./PROMETHEUS_METRICS_QUICK_REFERENCE.md)
- [Visual Guide](./PROMETHEUS_METRICS_VISUAL_GUIDE.md)

### External Resources
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [PromQL Guide](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Grafana Documentation](https://grafana.com/docs/)

### Community
- GitHub Issues: Report bugs or request features
- Stack Overflow: Tag `prometheus` and `rust`
- Prometheus Slack: `#prometheus-users`

---

## Document Map

```
PROMETHEUS_METRICS_INDEX.md (this file)
    │
    ├── PROMETHEUS_METRICS_ARCHITECTURE.md
    │   ├── Section 1: Overview
    │   ├── Section 2: Metrics Taxonomy
    │   ├── Section 3: Architecture Design
    │   ├── Section 4: Naming Conventions
    │   ├── Section 5: Integration Strategy
    │   ├── Section 6: Configuration Schema
    │   ├── Section 7: Implementation Patterns
    │   └── Section 8: Best Practices
    │
    ├── PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md
    │   ├── Section 1: Quick Start
    │   ├── Section 2: Step-by-Step Implementation
    │   │   ├── Phase 1: Core Registry
    │   │   ├── Phase 2: HTTP Middleware
    │   │   ├── Phase 3: Metrics Endpoint
    │   │   ├── Phase 4: Business Logic
    │   │   └── Phase 5: System Metrics
    │   ├── Section 3: Code Examples
    │   ├── Section 4: Testing Guide
    │   ├── Section 5: Deployment Guide
    │   └── Section 6: Troubleshooting
    │
    ├── PROMETHEUS_METRICS_QUICK_REFERENCE.md
    │   ├── Metric Categories
    │   ├── Common Patterns
    │   ├── Configuration
    │   ├── Prometheus Config
    │   ├── Alert Rules
    │   ├── Dashboard Queries
    │   ├── Troubleshooting
    │   └── Quick Links
    │
    └── PROMETHEUS_METRICS_VISUAL_GUIDE.md
        ├── System Architecture Diagram
        ├── Metrics Collection Flow
        ├── Incident Lifecycle Diagram
        ├── LLM Integration Flow
        ├── Export & Scraping Flow
        ├── Module Organization
        ├── Deployment Architecture
        ├── Dashboard Example
        └── Quick Start Flowchart
```

---

## Next Steps

### For New Implementations
1. Read [Architecture Document](./PROMETHEUS_METRICS_ARCHITECTURE.md) (Sections 1-3)
2. Follow [Implementation Guide](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md) (Phases 1-5)
3. Bookmark [Quick Reference](./PROMETHEUS_METRICS_QUICK_REFERENCE.md)
4. Review [Visual Guide](./PROMETHEUS_METRICS_VISUAL_GUIDE.md) for understanding

### For Existing Implementations
1. Audit current metrics against [Architecture Document](./PROMETHEUS_METRICS_ARCHITECTURE.md)
2. Check [Best Practices](./PROMETHEUS_METRICS_ARCHITECTURE.md#8-best-practices)
3. Use [Quick Reference](./PROMETHEUS_METRICS_QUICK_REFERENCE.md) for queries
4. Set up alerts from [Architecture Document](./PROMETHEUS_METRICS_ARCHITECTURE.md#alerting-rules)

### For Operations Teams
1. Review [Quick Reference](./PROMETHEUS_METRICS_QUICK_REFERENCE.md)
2. Configure Prometheus from [Implementation Guide](./PROMETHEUS_METRICS_IMPLEMENTATION_GUIDE.md#5-deployment-guide)
3. Create dashboards using [Visual Guide](./PROMETHEUS_METRICS_VISUAL_GUIDE.md#dashboard-example)
4. Set up alerts from [Quick Reference](./PROMETHEUS_METRICS_QUICK_REFERENCE.md#alert-rules)

---

## Version Information

**Documentation Version**: 1.0.0
**Last Updated**: 2025-11-12
**Rust Version**: 1.75+
**Prometheus Client**: 0.13+

---

## License

This documentation is part of the LLM Incident Manager project.

---

## Feedback

Found an issue or have a suggestion? Please open an issue or submit a pull request.
