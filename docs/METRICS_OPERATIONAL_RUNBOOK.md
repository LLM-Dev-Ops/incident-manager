# Metrics Operational Runbook

## Table of Contents
- [Overview](#overview)
- [Accessing Metrics](#accessing-metrics)
- [Monitoring Dashboard Setup](#monitoring-dashboard-setup)
- [Common Issues and Solutions](#common-issues-and-solutions)
- [Performance Tuning](#performance-tuning)
- [Alert Configuration](#alert-configuration)
- [Troubleshooting Guide](#troubleshooting-guide)
- [Maintenance Procedures](#maintenance-procedures)

## Overview

This runbook provides operational procedures for managing, monitoring, and troubleshooting the metrics system in the LLM Incident Manager. It covers common scenarios, alert responses, and maintenance tasks.

### Prerequisites

- Access to Prometheus/Grafana dashboards
- SSH/kubectl access to application servers
- Familiarity with PromQL query language
- Understanding of the metrics catalog (see METRICS_GUIDE.md)

## Accessing Metrics

### Direct HTTP Access

#### Prometheus Format
```bash
# Get all metrics
curl http://localhost:9090/metrics

# Get metrics for specific integration
curl http://localhost:9090/metrics | grep sentinel

# Get metrics with authentication
curl -H "Authorization: Bearer $TOKEN" https://metrics.example.com/metrics
```

#### JSON Format
```bash
# Get metrics as JSON
curl http://localhost:8080/v1/metrics/integrations

# Pretty print
curl http://localhost:8080/v1/metrics/integrations | jq .

# Get specific integration
curl http://localhost:8080/v1/metrics/integrations/sentinel | jq .
```

### Using Prometheus CLI

```bash
# Query instant value
promtool query instant http://localhost:9091 \
  'llm_integration_requests_total{integration="sentinel"}'

# Query range
promtool query range http://localhost:9091 \
  --start 2024-01-01T00:00:00Z \
  --end 2024-01-01T01:00:00Z \
  'rate(llm_integration_requests_total[5m])'

# Check metric metadata
promtool query metadata http://localhost:9091 \
  llm_integration_requests_total
```

### Using kubectl (Kubernetes)

```bash
# Port-forward to metrics endpoint
kubectl port-forward svc/incident-manager 9090:9090

# In another terminal
curl http://localhost:9090/metrics

# Get metrics from specific pod
kubectl exec -it incident-manager-pod -- \
  wget -O- http://localhost:9090/metrics
```

## Monitoring Dashboard Setup

### Grafana Quick Start

#### 1. Import Pre-built Dashboards

```bash
# Download dashboard definitions
curl -o dashboards.zip \
  https://github.com/llm-devops/llm-incident-manager/dashboards.zip

unzip dashboards.zip -d /etc/grafana/provisioning/dashboards/
```

#### 2. Configure Datasource

Create `/etc/grafana/provisioning/datasources/prometheus.yaml`:

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
    editable: false
```

#### 3. Key Dashboards

| Dashboard | Purpose | Update Frequency |
|-----------|---------|------------------|
| System Overview | High-level health and performance | Real-time |
| Integration Performance | Per-integration deep dive | 15s |
| Incident Processing | Processing pipeline metrics | 30s |
| Alerts & SLOs | SLA compliance and alerts | 1m |
| Cost & Resource Usage | Resource consumption tracking | 5m |

### Essential Panels

#### Request Rate Panel
```json
{
  "title": "Request Rate by Integration",
  "targets": [
    {
      "expr": "sum(rate(llm_integration_requests_total[5m])) by (integration)",
      "legendFormat": "{{integration}}"
    }
  ],
  "yAxis": {
    "label": "Requests/sec"
  }
}
```

#### Error Rate Panel
```json
{
  "title": "Error Rate %",
  "targets": [
    {
      "expr": "100 * (sum(rate(llm_integration_requests_failed[5m])) / sum(rate(llm_integration_requests_total[5m])))"
    }
  ],
  "thresholds": [
    {"value": 1, "color": "yellow"},
    {"value": 5, "color": "red"}
  ]
}
```

#### Latency Heatmap
```json
{
  "title": "Latency Distribution",
  "targets": [
    {
      "expr": "histogram_quantile(0.95, rate(llm_integration_latency_milliseconds_bucket[5m]))",
      "legendFormat": "p95"
    },
    {
      "expr": "histogram_quantile(0.99, rate(llm_integration_latency_milliseconds_bucket[5m]))",
      "legendFormat": "p99"
    }
  ]
}
```

## Common Issues and Solutions

### Issue 1: Metrics Not Updating

#### Symptoms
- Metrics values frozen
- `last_request_timestamp` not changing
- Grafana shows "No data"

#### Diagnosis
```bash
# Check if metrics endpoint is accessible
curl http://localhost:9090/metrics

# Check application logs
kubectl logs incident-manager-pod | grep -i metrics

# Verify Prometheus is scraping
curl http://prometheus:9091/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job=="llm-incident-manager")'
```

#### Solutions

**1. Application Not Running**
```bash
# Check pod status
kubectl get pods -l app=incident-manager

# Restart if needed
kubectl rollout restart deployment/incident-manager
```

**2. Metrics Port Not Exposed**
```bash
# Verify port configuration
kubectl get svc incident-manager -o yaml | grep 9090

# Check environment variable
kubectl exec incident-manager-pod -- env | grep METRICS_PORT
```

**3. Prometheus Scrape Failure**
```yaml
# Check prometheus.yml configuration
scrape_configs:
  - job_name: 'llm-incident-manager'
    static_configs:
      - targets: ['incident-manager:9090']  # Correct service name

# Reload Prometheus
curl -X POST http://prometheus:9091/-/reload
```

**4. Network Policy Blocking**
```bash
# Test network connectivity
kubectl exec prometheus-pod -- \
  wget -O- http://incident-manager:9090/metrics

# Check network policies
kubectl get networkpolicy -n incident-manager
```

### Issue 2: High Latency Metrics

#### Symptoms
- `llm_integration_latency_milliseconds_average` > 2000ms
- Alert: `HighLatency` firing
- User complaints about slow responses

#### Diagnosis
```promql
# Check latency by integration
llm_integration_latency_milliseconds_average

# Compare to historical baseline
rate(llm_integration_latency_milliseconds_total[5m])
/ rate(llm_integration_requests_total[5m])

# Identify worst performing integration
topk(1, llm_integration_latency_milliseconds_average)
```

#### Solutions

**1. External Service Degradation**
```bash
# Check health of LLM services
curl http://sentinel-service/health
curl http://shield-service/health
curl http://edge-agent-service/health
curl http://governance-service/health

# Review integration logs
kubectl logs incident-manager-pod | grep "Request failed"
```

**2. Network Issues**
```bash
# Test network latency
kubectl exec incident-manager-pod -- \
  ping -c 5 sentinel-service

# Check DNS resolution
kubectl exec incident-manager-pod -- \
  nslookup sentinel-service

# Verify service mesh (if using Istio/Linkerd)
kubectl get virtualservice,destinationrule
```

**3. Resource Constraints**
```bash
# Check CPU/Memory usage
kubectl top pod incident-manager-pod

# Review resource limits
kubectl describe pod incident-manager-pod | grep -A 5 "Limits:"

# Increase resources if needed
kubectl edit deployment incident-manager
```

**4. Connection Pool Exhaustion**
```bash
# Check connection pool metrics
curl http://localhost:9090/metrics | grep connection_pool

# Increase pool size in config
LLM_IM__INTEGRATIONS__MAX_CONNECTIONS=50
```

### Issue 3: High Error Rate

#### Symptoms
- `llm_integration_requests_failed` increasing
- `success_rate_percent` < 95%
- Alert: `HighErrorRate` firing

#### Diagnosis
```promql
# Calculate error rate
rate(llm_integration_requests_failed[5m])
/ rate(llm_integration_requests_total[5m])

# Group by integration
sum(rate(llm_integration_requests_failed[5m])) by (integration)

# Check error types in logs
kubectl logs incident-manager-pod | grep -i error | tail -100
```

#### Solutions

**1. Authentication Failures**
```bash
# Verify API keys/tokens
kubectl get secret llm-credentials -o yaml

# Check if tokens are expired
kubectl logs incident-manager-pod | grep "401\|403"

# Rotate credentials
kubectl create secret generic llm-credentials \
  --from-literal=sentinel-token=$NEW_TOKEN \
  --dry-run=client -o yaml | kubectl apply -f -
```

**2. Rate Limiting**
```bash
# Check for 429 errors
kubectl logs incident-manager-pod | grep "429\|rate limit"

# Review retry configuration
kubectl get configmap incident-manager-config -o yaml

# Adjust retry policy
# Edit config to increase backoff intervals
```

**3. Service Unavailability**
```bash
# Check target service health
for service in sentinel shield edge-agent governance; do
  echo "Checking $service..."
  kubectl get pods -l app=$service
  kubectl logs -l app=$service --tail=10
done

# Check circuit breaker status
curl http://localhost:8080/v1/health/circuit-breakers
```

**4. Timeout Issues**
```bash
# Check timeout configuration
kubectl exec incident-manager-pod -- env | grep TIMEOUT

# Increase timeout if needed
LLM_IM__INTEGRATIONS__SENTINEL__TIMEOUT_SECS=30
```

### Issue 4: Memory Leak

#### Symptoms
- `incident_manager_memory_usage_bytes` continuously increasing
- OOMKilled pod restarts
- Slow metric queries

#### Diagnosis
```bash
# Check memory usage trend
kubectl top pod incident-manager-pod

# Get heap profile (if profiling enabled)
curl http://localhost:8080/debug/pprof/heap > heap.prof

# Analyze with pprof
go tool pprof heap.prof
```

#### Solutions

**1. Metric Cardinality Explosion**
```promql
# Count unique time series
count(llm_integration_requests_total)

# Check label cardinality
count by (__name__, integration) ({__name__=~"llm_integration.*"})
```

Fix:
```rust
// Remove high-cardinality labels
// BAD: Using user_id as label
metrics.record_with_labels(vec![("user_id", user_id)]);

// GOOD: Use fixed set of labels
metrics.record_with_labels(vec![("integration", "sentinel")]);
```

**2. Unbounded Caches**
```bash
# Check cache sizes
curl http://localhost:8080/v1/metrics/caches

# Configure cache TTL and size limits
LLM_IM__ENRICHMENT__CACHE_TTL_SECS=300
LLM_IM__ENRICHMENT__CACHE_MAX_SIZE=10000
```

**3. Resource Limits**
```yaml
# Increase memory limits
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
      - name: incident-manager
        resources:
          limits:
            memory: "2Gi"
          requests:
            memory: "1Gi"
```

### Issue 5: Missing Metrics

#### Symptoms
- Expected metrics not appearing in Prometheus
- Queries return "no data"
- Gaps in metric timeseries

#### Diagnosis
```bash
# List all available metrics
curl http://localhost:9090/metrics | grep "# TYPE"

# Check Prometheus targets
curl http://prometheus:9091/api/v1/targets

# Review application logs
kubectl logs incident-manager-pod | grep "metrics\|prometheus"
```

#### Solutions

**1. Metrics Not Enabled**
```bash
# Check configuration
kubectl exec incident-manager-pod -- env | grep PROMETHEUS_ENABLED

# Enable in config
LLM_IM__OBSERVABILITY__PROMETHEUS_ENABLED=true
```

**2. Scrape Interval Too Long**
```yaml
# Reduce scrape interval in prometheus.yml
scrape_configs:
  - job_name: 'llm-incident-manager'
    scrape_interval: 15s  # Was 60s
```

**3. Metric Name Mismatch**
```bash
# Search for similar metrics
curl http://localhost:9090/metrics | grep -i "integration.*request"

# Check metric registration in code
grep -r "register.*metric" src/
```

## Performance Tuning

### Optimizing Scrape Performance

#### 1. Reduce Metric Cardinality
```rust
// Before: High cardinality
metrics.with_label("endpoint", full_url);  // Unique per request

// After: Low cardinality
metrics.with_label("endpoint_type", endpoint_type);  // Few values
```

#### 2. Adjust Scrape Interval
```yaml
# For high-frequency metrics
scrape_configs:
  - job_name: 'critical-metrics'
    scrape_interval: 10s
    metric_relabel_configs:
      - source_labels: [__name__]
        regex: 'llm_integration_requests.*|incident_manager_alerts.*'
        action: keep

  # For low-frequency metrics
  - job_name: 'background-metrics'
    scrape_interval: 60s
    metric_relabel_configs:
      - source_labels: [__name__]
        regex: 'incident_manager_cache.*|incident_manager_storage.*'
        action: keep
```

#### 3. Enable Compression
```yaml
# In prometheus.yml
scrape_configs:
  - job_name: 'llm-incident-manager'
    compression: gzip
```

### Optimizing Query Performance

#### 1. Use Recording Rules
```yaml
groups:
  - name: incident_manager_rules
    interval: 30s
    rules:
      # Pre-calculate expensive queries
      - record: llm_integration:request_rate:5m
        expr: rate(llm_integration_requests_total[5m])

      - record: llm_integration:error_rate:5m
        expr: rate(llm_integration_requests_failed[5m])

      - record: llm_integration:success_percent:5m
        expr: |
          100 * (
            sum by (integration) (rate(llm_integration_requests_successful[5m]))
            / sum by (integration) (rate(llm_integration_requests_total[5m]))
          )
```

#### 2. Optimize Range Queries
```promql
# Slow: Large range with short interval
rate(llm_integration_requests_total[1h])[5m:10s]

# Fast: Match range to resolution
rate(llm_integration_requests_total[5m])
```

#### 3. Use Aggregation
```promql
# Slow: Process all series
llm_integration_requests_total

# Fast: Aggregate early
sum by (integration) (llm_integration_requests_total)
```

### Resource Optimization

#### 1. Prometheus Storage
```yaml
# Configure retention
storage:
  tsdb:
    retention.time: 15d
    retention.size: 50GB

# Configure compaction
compaction:
  block_range: 2h
```

#### 2. Application Memory
```rust
// Use bounded collections
use std::collections::VecDeque;

const MAX_HISTORY: usize = 1000;

let mut history: VecDeque<MetricsSnapshot> = VecDeque::with_capacity(MAX_HISTORY);

// Maintain size limit
if history.len() >= MAX_HISTORY {
    history.pop_front();
}
history.push_back(snapshot);
```

## Alert Configuration

### Critical Alerts

#### 1. Service Down
```yaml
- alert: ServiceDown
  expr: up{job="llm-incident-manager"} == 0
  for: 1m
  labels:
    severity: critical
    team: platform
  annotations:
    summary: "LLM Incident Manager is down"
    description: "Service has been down for more than 1 minute"
    runbook: "https://wiki.example.com/runbooks/service-down"
```

**Response Procedure**:
1. Check pod status: `kubectl get pods`
2. Review recent logs: `kubectl logs --previous`
3. Check recent deployments: `kubectl rollout history`
4. If crashed, check OOM or errors: `kubectl describe pod`
5. Rollback if needed: `kubectl rollout undo`

#### 2. High Error Rate
```yaml
- alert: HighErrorRate
  expr: |
    (
      rate(llm_integration_requests_failed[5m])
      / rate(llm_integration_requests_total[5m])
    ) > 0.05
  for: 5m
  labels:
    severity: warning
    team: platform
  annotations:
    summary: "High error rate on {{ $labels.integration }}"
    description: "Error rate is {{ $value | humanizePercentage }}"
```

**Response Procedure**:
1. Identify failing integration from alert labels
2. Check integration service health
3. Review error logs for patterns
4. Check authentication/authorization
5. Verify network connectivity
6. Escalate to integration team if external issue

#### 3. High Latency
```yaml
- alert: HighLatency
  expr: llm_integration_latency_milliseconds_p95 > 2000
  for: 10m
  labels:
    severity: warning
    team: platform
  annotations:
    summary: "High latency on {{ $labels.integration }}"
    description: "P95 latency is {{ $value }}ms"
```

**Response Procedure**:
1. Check network latency to integration
2. Review CPU/memory usage
3. Check for resource contention
4. Review recent configuration changes
5. Scale up if load-related
6. Contact integration provider if external

### Warning Alerts

#### 4. Processing Queue Backed Up
```yaml
- alert: ProcessingQueueBackedUp
  expr: incident_manager_processing_queue_size > 1000
  for: 5m
  labels:
    severity: warning
    team: platform
  annotations:
    summary: "Processing queue is backed up"
    description: "Queue size is {{ $value }}"
```

#### 5. Cache Hit Rate Low
```yaml
- alert: CacheHitRateLow
  expr: incident_manager_enrichment_cache_hit_rate < 0.5
  for: 15m
  labels:
    severity: info
    team: platform
  annotations:
    summary: "Cache hit rate is low"
    description: "Hit rate is {{ $value | humanizePercentage }}"
```

#### 6. Memory Usage High
```yaml
- alert: MemoryUsageHigh
  expr: |
    (
      container_memory_usage_bytes{pod=~"incident-manager.*"}
      / container_spec_memory_limit_bytes{pod=~"incident-manager.*"}
    ) > 0.8
  for: 10m
  labels:
    severity: warning
    team: platform
  annotations:
    summary: "Memory usage is high"
    description: "Usage is {{ $value | humanizePercentage }} of limit"
```

### Alert Routing

```yaml
# alertmanager.yml
route:
  receiver: 'default'
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h

  routes:
    # Critical alerts - page immediately
    - match:
        severity: critical
      receiver: 'pagerduty-critical'
      continue: true

    # Warning alerts - Slack notification
    - match:
        severity: warning
      receiver: 'slack-warnings'
      continue: true

    # Info alerts - logged only
    - match:
        severity: info
      receiver: 'slack-info'

receivers:
  - name: 'default'
    webhook_configs:
      - url: 'http://incident-manager:8080/v1/alerts/webhook'

  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: '$PAGERDUTY_SERVICE_KEY'
        severity: 'critical'

  - name: 'slack-warnings'
    slack_configs:
      - api_url: '$SLACK_WEBHOOK_URL'
        channel: '#platform-alerts'
        title: 'Warning: {{ .GroupLabels.alertname }}'

  - name: 'slack-info'
    slack_configs:
      - api_url: '$SLACK_WEBHOOK_URL'
        channel: '#platform-info'
```

## Troubleshooting Guide

### Debugging Checklist

#### Phase 1: Verify Basics
- [ ] Is the application running?
- [ ] Is the metrics endpoint accessible?
- [ ] Is Prometheus scraping successfully?
- [ ] Are metrics being generated?

#### Phase 2: Check Configuration
- [ ] Is `prometheus_enabled = true`?
- [ ] Is metrics port correct (default 9090)?
- [ ] Are integration clients initialized?
- [ ] Are metrics being recorded in code?

#### Phase 3: Network Connectivity
- [ ] Can Prometheus reach metrics endpoint?
- [ ] Are firewalls/network policies blocking?
- [ ] Is DNS resolving correctly?
- [ ] Are service meshes configured properly?

#### Phase 4: Data Validation
- [ ] Are metric values reasonable?
- [ ] Is timestamp current?
- [ ] Are labels correct?
- [ ] Is cardinality within limits?

### Debug Commands

```bash
# 1. Check application health
curl http://localhost:8080/health

# 2. Verify metrics endpoint
curl http://localhost:9090/metrics | head -20

# 3. Count metrics
curl -s http://localhost:9090/metrics | grep "# TYPE" | wc -l

# 4. Check specific metric
curl -s http://localhost:9090/metrics | grep llm_integration_requests_total

# 5. Verify Prometheus scraping
curl http://prometheus:9091/api/v1/targets | jq '.data.activeTargets[] | {job:.labels.job, health:.health}'

# 6. Query Prometheus directly
curl -G http://prometheus:9091/api/v1/query \
  --data-urlencode 'query=llm_integration_requests_total' | jq .

# 7. Check metric metadata
curl http://prometheus:9091/api/v1/metadata?metric=llm_integration_requests_total | jq .

# 8. Review application logs
kubectl logs -f incident-manager-pod | grep -i "metric\|prometheus"

# 9. Check for errors
kubectl logs incident-manager-pod | grep -i error | tail -50

# 10. Inspect pod events
kubectl describe pod incident-manager-pod | grep -A 10 Events:
```

## Maintenance Procedures

### Daily Tasks

#### 1. Health Check
```bash
#!/bin/bash
# daily-health-check.sh

# Check service availability
if ! curl -sf http://localhost:8080/health > /dev/null; then
  echo "CRITICAL: Service health check failed"
  exit 1
fi

# Check error rate
error_rate=$(curl -sG http://prometheus:9091/api/v1/query \
  --data-urlencode 'query=rate(llm_integration_requests_failed[5m]) / rate(llm_integration_requests_total[5m])' \
  | jq -r '.data.result[0].value[1]')

if (( $(echo "$error_rate > 0.05" | bc -l) )); then
  echo "WARNING: Error rate is ${error_rate}"
fi

echo "Health check passed"
```

#### 2. Metric Validation
```bash
#!/bin/bash
# validate-metrics.sh

# Expected metrics
expected_metrics=(
  "llm_integration_requests_total"
  "llm_integration_success_rate_percent"
  "incident_manager_alerts_processed_total"
)

# Check each metric exists
for metric in "${expected_metrics[@]}"; do
  if ! curl -s http://localhost:9090/metrics | grep -q "^$metric"; then
    echo "WARNING: Missing metric: $metric"
  fi
done
```

### Weekly Tasks

#### 1. Review Alert History
```promql
# Count alert firings per alert
count_over_time(ALERTS{alertstate="firing"}[7d]) by (alertname)

# Alert duration
avg_over_time(ALERTS_FOR_STATE{alertstate="firing"}[7d]) by (alertname)
```

#### 2. Capacity Planning
```promql
# Trend request rate
predict_linear(
  rate(llm_integration_requests_total[7d])[7d:1h],
  7*24*3600
)

# Trend memory usage
predict_linear(
  incident_manager_memory_usage_bytes[7d],
  7*24*3600
)
```

#### 3. Performance Review
```bash
#!/bin/bash
# weekly-performance-review.sh

echo "=== Weekly Performance Report ==="
echo ""

# Average latency
echo "Average Latency (7d):"
curl -sG http://prometheus:9091/api/v1/query \
  --data-urlencode 'query=avg_over_time(llm_integration_latency_milliseconds_average[7d])' \
  | jq -r '.data.result[] | "\(.metric.integration): \(.value[1])ms"'

echo ""

# Success rate
echo "Success Rate (7d):"
curl -sG http://prometheus:9091/api/v1/query \
  --data-urlencode 'query=avg_over_time(llm_integration_success_rate_percent[7d])' \
  | jq -r '.data.result[] | "\(.metric.integration): \(.value[1])%"'
```

### Monthly Tasks

#### 1. Metrics Cleanup
```bash
#!/bin/bash
# monthly-cleanup.sh

# Remove stale metrics (not updated in 30 days)
# Configure Prometheus retention
curl -X POST http://prometheus:9091/api/v1/admin/tsdb/clean_tombstones

# Compact data
curl -X POST http://prometheus:9091/api/v1/admin/tsdb/snapshot
```

#### 2. Update Dashboards
- Review and update Grafana dashboards
- Add new metrics as features are added
- Remove deprecated metrics
- Update alert thresholds based on trends

#### 3. Documentation Review
- Update this runbook with new procedures
- Document any recurring issues and solutions
- Update contact information
- Review and update SLOs

## Related Documentation

- [METRICS_GUIDE.md](./METRICS_GUIDE.md) - Metrics catalog and configuration
- [METRICS_IMPLEMENTATION.md](./METRICS_IMPLEMENTATION.md) - Technical implementation
- [deployment-guide.md](./deployment-guide.md) - Deployment procedures
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture
