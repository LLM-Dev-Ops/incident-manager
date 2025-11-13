# Circuit Breaker Operations Guide

**Version**: 1.0.0
**Last Updated**: 2025-11-13
**Audience**: Operations, SRE, DevOps Teams

---

## Table of Contents

1. [Overview](#overview)
2. [Monitoring Circuit Breaker Health](#monitoring-circuit-breaker-health)
3. [Dashboard Examples](#dashboard-examples)
4. [Alert Configurations](#alert-configurations)
5. [Manual Circuit Breaker Control](#manual-circuit-breaker-control)
6. [Debugging Common Issues](#debugging-common-issues)
7. [Performance Tuning](#performance-tuning)
8. [Incident Response Playbooks](#incident-response-playbooks)
9. [Best Practices for Operations](#best-practices-for-operations)

---

## Overview

This guide provides operational procedures for monitoring, managing, and troubleshooting circuit breakers in production. Circuit breakers are critical resilience components that prevent cascading failures and protect system resources.

### Quick Reference

| Task | Command | Documentation |
|------|---------|---------------|
| Check all circuit states | `GET /v1/circuit-breakers` | [API](#rest-api-endpoints) |
| Force open circuit | `POST /v1/circuit-breakers/{name}/open` | [Manual Control](#manual-circuit-breaker-control) |
| Reset circuit | `POST /v1/circuit-breakers/{name}/reset` | [Manual Control](#manual-circuit-breaker-control) |
| View metrics | `GET /metrics` | [Metrics](#prometheus-metrics) |
| Health check | `GET /health` | [Health Checks](#health-check-endpoint) |

---

## Monitoring Circuit Breaker Health

### Key Metrics to Monitor

#### 1. Circuit State

**Metric**: `circuit_breaker_state`

```promql
# Current state of all circuit breakers
circuit_breaker_state{name="sentinel"} 0  # 0=closed, 1=open, 2=half-open
```

**What to watch:**
- Circuit staying open for extended periods
- Frequent state transitions (oscillation)
- Multiple circuits open simultaneously

#### 2. Request Metrics

```promql
# Total requests per circuit breaker
rate(circuit_breaker_requests_total{name="sentinel"}[5m])

# Error rate
rate(circuit_breaker_requests_failed{name="sentinel"}[5m]) /
rate(circuit_breaker_requests_total{name="sentinel"}[5m])

# Rejection rate (circuit open)
rate(circuit_breaker_requests_rejected{name="sentinel"}[5m])
```

#### 3. Latency Metrics

```promql
# Average latency
rate(circuit_breaker_request_duration_seconds_sum{name="sentinel"}[5m]) /
rate(circuit_breaker_request_duration_seconds_count{name="sentinel"}[5m])

# P95 latency
histogram_quantile(0.95,
  rate(circuit_breaker_request_duration_seconds_bucket{name="sentinel"}[5m])
)

# P99 latency
histogram_quantile(0.99,
  rate(circuit_breaker_request_duration_seconds_bucket{name="sentinel"}[5m])
)
```

#### 4. State Transition Metrics

```promql
# Number of times circuit opened
circuit_breaker_open_count{name="sentinel"}

# Number of times circuit closed
circuit_breaker_close_count{name="sentinel"}

# Time spent in each state
circuit_breaker_time_in_state_seconds{name="sentinel", state="open"}
```

### REST API Endpoints

#### Get All Circuit Breakers

```bash
curl http://localhost:8080/v1/circuit-breakers
```

**Response:**
```json
{
  "circuit_breakers": [
    {
      "name": "sentinel",
      "state": "closed",
      "failure_count": 0,
      "success_count": 1523,
      "total_requests": 1523,
      "error_rate": 0.0,
      "state_since": "2025-11-13T10:00:00Z",
      "last_opened_at": null,
      "open_count": 0
    },
    {
      "name": "shield",
      "state": "open",
      "failure_count": 5,
      "success_count": 0,
      "total_requests": 5,
      "error_rate": 1.0,
      "state_since": "2025-11-13T10:15:30Z",
      "last_opened_at": "2025-11-13T10:15:30Z",
      "open_count": 3
    }
  ]
}
```

#### Get Specific Circuit Breaker

```bash
curl http://localhost:8080/v1/circuit-breakers/sentinel
```

**Response:**
```json
{
  "name": "sentinel",
  "state": "closed",
  "failure_count": 0,
  "success_count": 1523,
  "total_requests": 1523,
  "consecutive_failures": 0,
  "consecutive_successes": 0,
  "error_rate": 0.0,
  "state_since": "2025-11-13T10:00:00Z",
  "last_opened_at": null,
  "open_count": 0,
  "config": {
    "failure_threshold": 5,
    "success_threshold": 2,
    "timeout_secs": 60,
    "volume_threshold": 10
  }
}
```

#### Health Check Endpoint

```bash
curl http://localhost:8080/v1/circuit-breakers/health
```

**Response:**
```json
{
  "healthy": false,
  "circuit_breakers": {
    "sentinel": "healthy",
    "shield": "unhealthy",
    "edge-agent": "degraded",
    "governance": "healthy"
  },
  "summary": {
    "total": 4,
    "healthy": 2,
    "degraded": 1,
    "unhealthy": 1
  }
}
```

### Prometheus Metrics

#### Available Metrics

```
# Circuit breaker state
circuit_breaker_state{name="sentinel"} 0

# Request counters
circuit_breaker_requests_total{name="sentinel"} 1523
circuit_breaker_requests_successful{name="sentinel"} 1523
circuit_breaker_requests_failed{name="sentinel"} 0
circuit_breaker_requests_rejected{name="sentinel"} 0

# Error rate
circuit_breaker_error_rate{name="sentinel"} 0.0

# State transition counters
circuit_breaker_open_count{name="sentinel"} 0
circuit_breaker_close_count{name="sentinel"} 5
circuit_breaker_half_open_count{name="sentinel"} 5

# Latency histogram
circuit_breaker_request_duration_seconds_bucket{name="sentinel",le="0.05"} 1200
circuit_breaker_request_duration_seconds_bucket{name="sentinel",le="0.1"} 1480
circuit_breaker_request_duration_seconds_bucket{name="sentinel",le="0.5"} 1520
circuit_breaker_request_duration_seconds_bucket{name="sentinel",le="1.0"} 1523
circuit_breaker_request_duration_seconds_sum{name="sentinel"} 76.5
circuit_breaker_request_duration_seconds_count{name="sentinel"} 1523

# Time in each state
circuit_breaker_time_in_state_seconds{name="sentinel",state="closed"} 3600
circuit_breaker_time_in_state_seconds{name="sentinel",state="open"} 0
circuit_breaker_time_in_state_seconds{name="sentinel",state="half_open"} 5
```

---

## Dashboard Examples

### Grafana Dashboard: Circuit Breaker Overview

#### Panel 1: Circuit Breaker States

```json
{
  "title": "Circuit Breaker States",
  "targets": [
    {
      "expr": "circuit_breaker_state",
      "legendFormat": "{{name}}"
    }
  ],
  "type": "stat",
  "options": {
    "graphMode": "none",
    "colorMode": "background",
    "thresholds": {
      "mode": "absolute",
      "steps": [
        {"value": 0, "color": "green"},   // Closed
        {"value": 1, "color": "red"},     // Open
        {"value": 2, "color": "yellow"}   // Half-Open
      ]
    }
  }
}
```

#### Panel 2: Request Rate

```json
{
  "title": "Request Rate (req/s)",
  "targets": [
    {
      "expr": "rate(circuit_breaker_requests_total[5m])",
      "legendFormat": "{{name}}"
    }
  ],
  "type": "graph"
}
```

#### Panel 3: Error Rate

```json
{
  "title": "Error Rate (%)",
  "targets": [
    {
      "expr": "circuit_breaker_error_rate * 100",
      "legendFormat": "{{name}}"
    }
  ],
  "type": "graph",
  "yaxes": [
    {
      "format": "percent",
      "min": 0,
      "max": 100
    }
  ]
}
```

#### Panel 4: Latency Percentiles

```json
{
  "title": "Request Latency",
  "targets": [
    {
      "expr": "histogram_quantile(0.50, rate(circuit_breaker_request_duration_seconds_bucket[5m]))",
      "legendFormat": "p50 {{name}}"
    },
    {
      "expr": "histogram_quantile(0.95, rate(circuit_breaker_request_duration_seconds_bucket[5m]))",
      "legendFormat": "p95 {{name}}"
    },
    {
      "expr": "histogram_quantile(0.99, rate(circuit_breaker_request_duration_seconds_bucket[5m]))",
      "legendFormat": "p99 {{name}}"
    }
  ],
  "type": "graph"
}
```

#### Panel 5: Circuit Opens Over Time

```json
{
  "title": "Circuit Opens (cumulative)",
  "targets": [
    {
      "expr": "circuit_breaker_open_count",
      "legendFormat": "{{name}}"
    }
  ],
  "type": "graph"
}
```

#### Panel 6: Rejected Requests

```json
{
  "title": "Rejected Requests (circuit open)",
  "targets": [
    {
      "expr": "rate(circuit_breaker_requests_rejected[5m])",
      "legendFormat": "{{name}}"
    }
  ],
  "type": "graph"
}
```

### Sample Dashboard JSON

Complete dashboard available at: `/config/grafana/circuit-breaker-dashboard.json`

---

## Alert Configurations

### Prometheus Alert Rules

#### Critical Alerts

```yaml
# circuit-breaker-alerts.yaml
groups:
  - name: circuit_breaker_critical
    interval: 30s
    rules:
      # Alert when circuit breaker opens
      - alert: CircuitBreakerOpen
        expr: circuit_breaker_state == 1
        for: 1m
        labels:
          severity: critical
          component: circuit_breaker
        annotations:
          summary: "Circuit breaker {{ $labels.name }} is open"
          description: "Circuit breaker {{ $labels.name }} has been open for more than 1 minute. External service may be down."
          runbook_url: "https://docs.example.com/runbooks/circuit-breaker-open"

      # Alert when circuit remains open for extended period
      - alert: CircuitBreakerStuckOpen
        expr: circuit_breaker_state == 1
        for: 10m
        labels:
          severity: critical
          component: circuit_breaker
        annotations:
          summary: "Circuit breaker {{ $labels.name }} stuck open"
          description: "Circuit breaker {{ $labels.name }} has been open for more than 10 minutes. Immediate investigation required."

      # Alert when multiple circuits are open
      - alert: MultipleCircuitBreakersOpen
        expr: count(circuit_breaker_state == 1) >= 2
        for: 2m
        labels:
          severity: critical
          component: circuit_breaker
        annotations:
          summary: "Multiple circuit breakers are open"
          description: "{{ $value }} circuit breakers are currently open. System-wide issue suspected."
```

#### Warning Alerts

```yaml
  - name: circuit_breaker_warning
    interval: 30s
    rules:
      # Alert on high error rate before circuit opens
      - alert: CircuitBreakerHighErrorRate
        expr: circuit_breaker_error_rate > 0.3
        for: 3m
        labels:
          severity: warning
          component: circuit_breaker
        annotations:
          summary: "High error rate on {{ $labels.name }}"
          description: "Circuit breaker {{ $labels.name }} has error rate of {{ $value | humanizePercentage }}. Circuit may open soon."

      # Alert when circuit opens frequently
      - alert: CircuitBreakerFlapping
        expr: rate(circuit_breaker_open_count[1h]) > 3
        for: 5m
        labels:
          severity: warning
          component: circuit_breaker
        annotations:
          summary: "Circuit breaker {{ $labels.name }} flapping"
          description: "Circuit breaker {{ $labels.name }} has opened {{ $value }} times in the last hour. Service may be unstable."

      # Alert on high rejection rate
      - alert: CircuitBreakerHighRejectionRate
        expr: rate(circuit_breaker_requests_rejected[5m]) > 10
        for: 2m
        labels:
          severity: warning
          component: circuit_breaker
        annotations:
          summary: "High rejection rate on {{ $labels.name }}"
          description: "Circuit breaker {{ $labels.name }} is rejecting {{ $value }} requests/sec."
```

### PagerDuty Integration

```yaml
# alertmanager.yml
receivers:
  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_KEY'
        description: '{{ .CommonAnnotations.summary }}'
        details:
          circuit_breaker: '{{ .GroupLabels.name }}'
          severity: '{{ .GroupLabels.severity }}'
          description: '{{ .CommonAnnotations.description }}'

route:
  group_by: ['alertname', 'name']
  receiver: 'pagerduty-critical'
  routes:
    - match:
        severity: critical
        component: circuit_breaker
      receiver: 'pagerduty-critical'
      continue: false
```

### Slack Notifications

```yaml
receivers:
  - name: 'slack-alerts'
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK'
        channel: '#circuit-breaker-alerts'
        title: '{{ .CommonAnnotations.summary }}'
        text: |
          *Alert:* {{ .CommonAnnotations.summary }}
          *Details:* {{ .CommonAnnotations.description }}
          *Circuit Breaker:* {{ .GroupLabels.name }}
          *Severity:* {{ .GroupLabels.severity }}
```

---

## Manual Circuit Breaker Control

### Force Open (Maintenance Mode)

```bash
# Open circuit for maintenance
curl -X POST http://localhost:8080/v1/circuit-breakers/sentinel/open \
  -H "Content-Type: application/json" \
  -d '{"reason": "Scheduled maintenance on Sentinel API"}'
```

**Response:**
```json
{
  "success": true,
  "circuit_breaker": "sentinel",
  "previous_state": "closed",
  "current_state": "open",
  "reason": "Scheduled maintenance on Sentinel API",
  "timestamp": "2025-11-13T10:30:00Z"
}
```

**When to use:**
- Scheduled maintenance windows
- Known issues with external service
- Preventing traffic during deployments
- Testing failure scenarios

### Force Close (After Maintenance)

```bash
# Close circuit after maintenance
curl -X POST http://localhost:8080/v1/circuit-breakers/sentinel/close
```

**Response:**
```json
{
  "success": true,
  "circuit_breaker": "sentinel",
  "previous_state": "open",
  "current_state": "closed",
  "timestamp": "2025-11-13T11:00:00Z"
}
```

### Reset Circuit Breaker

```bash
# Reset all counters and state
curl -X POST http://localhost:8080/v1/circuit-breakers/sentinel/reset
```

**Response:**
```json
{
  "success": true,
  "circuit_breaker": "sentinel",
  "state": "closed",
  "counters_reset": true,
  "timestamp": "2025-11-13T11:05:00Z"
}
```

**When to use:**
- After resolving underlying issues
- To clear accumulated failures
- After configuration changes
- Testing recovery behavior

### Bulk Operations

```bash
# Reset all circuit breakers
curl -X POST http://localhost:8080/v1/circuit-breakers/reset-all

# Get status of all circuit breakers
curl http://localhost:8080/v1/circuit-breakers/status
```

---

## Debugging Common Issues

### Issue 1: Circuit Breaker Won't Close

**Symptoms:**
- Circuit remains in open state
- Half-open state keeps returning to open
- Service appears healthy but circuit stays open

**Debugging Steps:**

1. Check if timeout has elapsed:
```bash
curl http://localhost:8080/v1/circuit-breakers/sentinel | jq '.state_since'
```

2. Check service health directly:
```bash
curl https://sentinel-api.example.com/health
```

3. Check half-open test results:
```bash
curl http://localhost:8080/v1/circuit-breakers/sentinel | jq '.consecutive_successes'
```

4. Review logs:
```bash
kubectl logs -l app=incident-manager | grep "circuit_breaker.*sentinel"
```

**Solutions:**

```bash
# If service is healthy, manually close
curl -X POST http://localhost:8080/v1/circuit-breakers/sentinel/close

# If threshold too strict, update config
# Edit config/circuit_breakers.yaml
sentinel:
  success_threshold: 1  # Reduced from 2
  timeout_secs: 30      # Reduced from 60
```

### Issue 2: Circuit Opens Too Frequently

**Symptoms:**
- Circuit opens and closes multiple times per hour
- Flapping between states
- High alert noise

**Debugging Steps:**

1. Check error rate:
```promql
rate(circuit_breaker_requests_failed{name="sentinel"}[5m]) /
rate(circuit_breaker_requests_total{name="sentinel"}[5m])
```

2. Check open frequency:
```promql
rate(circuit_breaker_open_count{name="sentinel"}[1h])
```

3. Check service latency:
```promql
histogram_quantile(0.99,
  rate(circuit_breaker_request_duration_seconds_bucket{name="sentinel"}[5m])
)
```

**Solutions:**

```yaml
# Increase failure threshold
sentinel:
  failure_threshold: 10  # Increased from 5
  volume_threshold: 20   # Increased from 10

# Or increase timeout
sentinel:
  timeout_secs: 120      # Increased from 60
```

### Issue 3: High Rejection Rate

**Symptoms:**
- Many requests rejected
- Circuit frequently open
- User-facing errors

**Debugging Steps:**

1. Check rejection rate:
```promql
rate(circuit_breaker_requests_rejected{name="sentinel"}[5m])
```

2. Check service availability:
```bash
curl https://sentinel-api.example.com/health -w "\nStatus: %{http_code}\n"
```

3. Review service logs:
```bash
kubectl logs -l app=sentinel-api | tail -n 100
```

**Solutions:**

1. If service is down, implement fallback:
```rust
match breaker.call(|| primary_service()).await {
    Ok(result) => Ok(result),
    Err(e) if e.is_circuit_open() => {
        // Use fallback
        fallback_service().await
    }
    Err(e) => Err(e),
}
```

2. If service is overloaded, scale up:
```bash
kubectl scale deployment sentinel-api --replicas=5
```

### Issue 4: Circuit Doesn't Open When It Should

**Symptoms:**
- Service failing but circuit stays closed
- Timeouts consuming resources
- System degradation

**Debugging Steps:**

1. Check if volume threshold is met:
```bash
curl http://localhost:8080/v1/circuit-breakers/sentinel | jq '.total_requests'
```

2. Check current failure count:
```bash
curl http://localhost:8080/v1/circuit-breakers/sentinel | jq '.failure_count'
```

3. Verify errors are being counted:
```bash
# Check error handling in code
# Ensure errors are propagated, not swallowed
```

**Solutions:**

```yaml
# Lower thresholds
sentinel:
  failure_threshold: 3   # Reduced from 5
  volume_threshold: 5    # Reduced from 10
```

---

## Performance Tuning

### Configuration Tuning Guide

| Scenario | Configuration | Rationale |
|----------|--------------|-----------|
| **Critical Service** | `failure_threshold: 3`<br>`timeout: 30s` | Open quickly, recover quickly |
| **Flaky Service** | `failure_threshold: 10`<br>`volume_threshold: 20` | Tolerate intermittent failures |
| **Slow Service** | `timeout: 120s`<br>`half_open_timeout: 60s` | Allow time for slow responses |
| **High-Traffic** | `volume_threshold: 100`<br>`error_threshold_percentage: 60` | Base decisions on larger sample |

### Optimal Settings by Service Type

#### LLM APIs (Sentinel, Shield, etc.)

```yaml
llm_apis:
  failure_threshold: 5
  success_threshold: 2
  timeout_secs: 60
  half_open_timeout_secs: 30
  volume_threshold: 10
  error_threshold_percentage: 50
```

**Rationale:**
- Medium threshold (5 failures) - LLMs can be slow but usually reliable
- 60s timeout - Allow for LLM processing time
- 50% error threshold - Balance between protection and availability

#### Databases

```yaml
databases:
  failure_threshold: 10
  success_threshold: 2
  timeout_secs: 30
  half_open_timeout_secs: 15
  volume_threshold: 20
  error_threshold_percentage: 60
```

**Rationale:**
- Higher threshold - Database connections can be reused
- Shorter timeout - Database queries should be fast
- Higher volume threshold - More requests to evaluate

#### Cache Servers (Redis)

```yaml
cache:
  failure_threshold: 3
  success_threshold: 1
  timeout_secs: 10
  half_open_timeout_secs: 5
  volume_threshold: 5
  error_threshold_percentage: 40
```

**Rationale:**
- Low threshold - Cache failures should open quickly
- Very short timeout - Cache is non-critical
- Low error tolerance - Cache should be highly available

---

## Incident Response Playbooks

### Playbook 1: Single Circuit Breaker Open

**Alert**: `CircuitBreakerOpen`

**Steps:**

1. **Verify Alert** (< 2 min)
   ```bash
   curl http://localhost:8080/v1/circuit-breakers/sentinel
   ```

2. **Check External Service** (< 3 min)
   ```bash
   curl https://sentinel-api.example.com/health
   ```

3. **Review Recent Changes** (< 5 min)
   - Check deployment history
   - Review configuration changes
   - Check external service status page

4. **Decision Point**:
   - If service is healthy: Manually close circuit
   - If service is down: Engage service team
   - If transient: Wait for auto-recovery

5. **Monitor Recovery** (< 10 min)
   ```bash
   watch -n 5 'curl http://localhost:8080/v1/circuit-breakers/sentinel | jq'
   ```

### Playbook 2: Multiple Circuit Breakers Open

**Alert**: `MultipleCircuitBreakersOpen`

**Steps:**

1. **Assess Impact** (< 1 min)
   ```bash
   curl http://localhost:8080/v1/circuit-breakers/health
   ```

2. **Check System Health** (< 2 min)
   - Check incident manager health
   - Check network connectivity
   - Check DNS resolution

3. **Look for Common Cause** (< 5 min)
   - Network partition?
   - DNS issues?
   - Authentication service down?
   - Rate limiting?

4. **Escalate** (< 5 min)
   - Page on-call engineer
   - Create incident ticket
   - Post in incident channel

5. **Implement Workarounds** (< 10 min)
   - Enable fallback modes
   - Route traffic to backup systems
   - Increase timeouts temporarily

### Playbook 3: Circuit Breaker Flapping

**Alert**: `CircuitBreakerFlapping`

**Steps:**

1. **Identify Pattern** (< 5 min)
   ```promql
   circuit_breaker_state{name="sentinel"}[1h]
   ```

2. **Check Service Metrics** (< 5 min)
   - Latency trends
   - Error rates
   - Resource utilization

3. **Adjust Configuration** (< 10 min)
   ```yaml
   sentinel:
     failure_threshold: 10    # Increase
     timeout_secs: 120        # Increase
   ```

4. **Apply Changes**:
   ```bash
   kubectl apply -f config/circuit_breakers.yaml
   kubectl rollout restart deployment/incident-manager
   ```

5. **Monitor** (< 15 min)
   - Watch for continued flapping
   - Check if adjustment helped

---

## Best Practices for Operations

### Daily Operations

1. **Morning Checks**
   - Review circuit breaker dashboard
   - Check for any circuits stuck open overnight
   - Review alert history from previous day

2. **Weekly Reviews**
   - Analyze circuit breaker metrics trends
   - Review configuration appropriateness
   - Check for recurring patterns

3. **Monthly Analysis**
   - Calculate availability metrics
   - Review incident correlation with circuit opens
   - Update runbooks based on learnings

### Change Management

1. **Before Deployment**
   - Review circuit breaker configurations
   - Test circuit breaker behavior in staging
   - Document expected behavior changes

2. **During Deployment**
   - Monitor circuit breaker states
   - Watch for unexpected opens
   - Have rollback plan ready

3. **After Deployment**
   - Verify circuit breaker function
   - Check metrics for anomalies
   - Update documentation

### Documentation

Maintain documentation for:
- Circuit breaker configurations and rationale
- Service dependency map
- Escalation procedures
- Recent incidents and resolutions
- Configuration change history

---

## See Also

- [Circuit Breaker Guide](./CIRCUIT_BREAKER_GUIDE.md) - Architecture and concepts
- [API Reference](./CIRCUIT_BREAKER_API_REFERENCE.md) - Complete API documentation
- [Integration Guide](./CIRCUIT_BREAKER_INTEGRATION_GUIDE.md) - Implementation examples

---

**Document Version**: 1.0.0
**Last Updated**: 2025-11-13
**Status**: Production Ready
