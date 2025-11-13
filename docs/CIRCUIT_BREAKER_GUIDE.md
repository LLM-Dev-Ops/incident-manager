# Circuit Breaker Pattern - Comprehensive Guide

**Version**: 1.0.0
**Status**: Implementation Guide
**Last Updated**: 2025-11-13

---

## Table of Contents

1. [Introduction](#introduction)
2. [What is a Circuit Breaker](#what-is-a-circuit-breaker)
3. [Why Use Circuit Breakers](#why-use-circuit-breakers)
4. [Architecture Overview](#architecture-overview)
5. [State Machine Explanation](#state-machine-explanation)
6. [Configuration Guide](#configuration-guide)
7. [Integration Examples](#integration-examples)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)
10. [Performance Considerations](#performance-considerations)

---

## Introduction

The Circuit Breaker pattern is a critical resilience mechanism in the LLM Incident Manager system that prevents cascading failures when external services (like LLM providers) become unavailable or degraded. This guide provides comprehensive documentation on understanding, implementing, and operating circuit breakers in production.

### Key Benefits

- **Prevent Cascading Failures**: Stop problems in one service from spreading
- **Fast Failure**: Fail quickly instead of waiting for timeouts
- **Automatic Recovery**: Self-healing when services recover
- **Resource Protection**: Prevent resource exhaustion from failing calls
- **Observability**: Real-time monitoring of service health

---

## What is a Circuit Breaker

A circuit breaker is a design pattern used in software development to detect failures and encapsulate the logic of preventing a failure from constantly recurring during maintenance, temporary external system failure, or unexpected system difficulties.

### Real-World Analogy

Think of an electrical circuit breaker in your home:
- **Closed Circuit (Normal)**: Electricity flows, everything works
- **Open Circuit (Tripped)**: Breaker detects problem, stops electricity flow
- **Testing (Half-Open)**: Carefully tests if problem is resolved

Software circuit breakers work the same way:
- **Closed**: Requests flow to the external service normally
- **Open**: All requests fail immediately without calling the service
- **Half-Open**: Test requests are allowed to check if service recovered

### The Problem Circuit Breakers Solve

```
WITHOUT Circuit Breaker:
Client → [Long Wait] → Failed Service → Timeout → Retry → Timeout → ...
         (30 seconds)                    (30 seconds)    (30 seconds)
Result: 90+ seconds of waiting, resource exhaustion

WITH Circuit Breaker:
Client → Circuit Breaker (Open) → Immediate Failure → Fallback Response
         (<1ms)
Result: Instant failure, resources preserved, system stays responsive
```

---

## Why Use Circuit Breakers

### 1. Prevent Cascading Failures

When one service fails, without circuit breakers, the failure cascades:

```
LLM Provider Down
    ↓
Thread Pool Exhausted (waiting on timeouts)
    ↓
API Server Unresponsive
    ↓
Load Balancer Marks Server Unhealthy
    ↓
Other Servers Overloaded
    ↓
Complete System Outage
```

With circuit breakers, the failure is contained:

```
LLM Provider Down
    ↓
Circuit Breaker Opens (after 5 failures)
    ↓
Fast Fail (no resource consumption)
    ↓
System Continues Operating (degraded mode)
    ↓
Automatic Recovery When Service Returns
```

### 2. Resource Protection

**Without Circuit Breaker:**
- Threads blocked waiting for timeouts
- Connection pool exhausted
- Memory consumption grows
- CPU wasted on failed attempts
- Database connections held unnecessarily

**With Circuit Breaker:**
- Immediate failure response
- No thread blocking
- Minimal resource usage
- CPU available for successful operations
- Resources freed for other operations

### 3. Faster Failure Detection

```
Traditional Retry Logic:
Attempt 1: 30s timeout
Attempt 2: 30s timeout (after 1s backoff)
Attempt 3: 30s timeout (after 2s backoff)
Total: ~93 seconds to fail

Circuit Breaker:
Detect pattern after 5 failures
Open circuit
Subsequent failures: <1ms
Total: Milliseconds to fail
```

### 4. Automatic Recovery

Circuit breakers automatically test service recovery:
- No manual intervention required
- Gradual traffic restoration
- Protection against premature recovery
- Configurable recovery strategies

---

## Architecture Overview

### High-Level Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                     Incident Manager System                     │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Application Layer                            │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐         │  │
│  │  │ Sentinel   │  │   Shield   │  │ Edge-Agent │         │  │
│  │  │ Client     │  │   Client   │  │   Client   │         │  │
│  │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘         │  │
│  └────────┼───────────────┼───────────────┼────────────────┘  │
│           │               │               │                    │
│  ┌────────┼───────────────┼───────────────┼────────────────┐  │
│  │        ▼               ▼               ▼                 │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │         Circuit Breaker Registry                 │   │  │
│  │  │  ┌────────────┐  ┌────────────┐  ┌────────────┐ │   │  │
│  │  │  │ Sentinel CB│  │ Shield CB  │  │ Edge CB    │ │   │  │
│  │  │  │ (Closed)   │  │ (Open)     │  │ (Half-Open)│ │   │  │
│  │  │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘ │   │  │
│  │  └────────┼───────────────┼───────────────┼────────┘   │  │
│  │           │               │               │             │  │
│  │  ┌────────▼───────────────▼───────────────▼─────────┐  │  │
│  │  │         State Machine & Metrics                   │  │  │
│  │  │  • Success/Failure Tracking                       │  │  │
│  │  │  • Threshold Monitoring                           │  │  │
│  │  │  • Timeout Management                             │  │  │
│  │  │  • Recovery Testing                               │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│           │               │               │                    │
│  ┌────────▼───────────────▼───────────────▼────────────────┐  │
│  │               External Services Layer                    │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐        │  │
│  │  │ Sentinel   │  │   Shield   │  │ Edge-Agent │        │  │
│  │  │ LLM API    │  │  LLM API   │  │  LLM API   │        │  │
│  │  │ (Healthy)  │  │ (Down)     │  │ (Degraded) │        │  │
│  │  └────────────┘  └────────────┘  └────────────┘        │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

### Component Interaction Flow

```
┌─────────┐
│ Request │
└────┬────┘
     │
     ▼
┌────────────────────┐
│ Circuit Breaker    │
│ Check State        │
└────┬───────────────┘
     │
     ├─── [CLOSED] ──────┐
     │                   │
     ├─── [OPEN] ────────┼──→ Fail Fast (Return Error)
     │                   │
     ├─── [HALF-OPEN] ───┤
     │                   │
     ▼                   ▼
┌────────────────┐  ┌──────────────────┐
│ Execute Call   │  │ Check Timeout    │
└────┬───────────┘  └──────────────────┘
     │
     ├─── Success ──────┐
     │                  │
     ├─── Failure ──────┤
     │                  │
     ▼                  ▼
┌────────────────────────────┐
│ Update Circuit State       │
│ • Record Result            │
│ • Update Counters          │
│ • Check Thresholds         │
│ • Transition State         │
└────────────────────────────┘
```

---

## State Machine Explanation

### Circuit Breaker States

The circuit breaker operates as a finite state machine with three states:

#### 1. CLOSED State (Normal Operation)

**Behavior:**
- All requests are allowed to pass through
- Success and failure counts are tracked
- When failure threshold is exceeded → transition to OPEN

```
┌──────────────────────────────────┐
│         CLOSED STATE             │
│                                  │
│  Current Status: ✓ Operational   │
│  Requests Allowed: Yes           │
│  Failures: 3 / 5 threshold       │
│  Last Success: 2s ago            │
│                                  │
│  ┌────────┐                      │
│  │Request │──→ [Execute] ──→ ✓   │
│  └────────┘                      │
└──────────────────────────────────┘
         │
         │ 5+ failures in window
         │
         ▼
┌──────────────────────────────────┐
│          OPEN STATE              │
└──────────────────────────────────┘
```

**Configuration Parameters:**
```rust
failure_threshold: 5        // Open after 5 failures
volume_threshold: 10        // Minimum requests before evaluation
error_rate_threshold: 0.5   // Open if 50%+ requests fail
```

**Monitoring Metrics:**
```
circuit_breaker_state{name="sentinel", state="closed"} = 1
circuit_breaker_failures{name="sentinel"} = 3
circuit_breaker_successes{name="sentinel"} = 47
circuit_breaker_requests_total{name="sentinel"} = 50
```

#### 2. OPEN State (Circuit Tripped)

**Behavior:**
- All requests fail immediately without calling the service
- No resources consumed (no network calls, no timeouts)
- Timeout timer starts (e.g., 60 seconds)
- When timeout expires → transition to HALF-OPEN

```
┌──────────────────────────────────┐
│          OPEN STATE              │
│                                  │
│  Current Status: ✗ Tripped       │
│  Requests Allowed: No            │
│  Timeout: 45s remaining          │
│  Opened At: 2025-11-13 10:30:15  │
│                                  │
│  ┌────────┐                      │
│  │Request │──→ [Reject] ──→ ✗    │
│  └────────┘     (instant)        │
└──────────────────────────────────┘
         │
         │ Timeout expires (60s)
         │
         ▼
┌──────────────────────────────────┐
│       HALF-OPEN STATE            │
└──────────────────────────────────┘
```

**Configuration Parameters:**
```rust
timeout: Duration::from_secs(60)     // Wait 60s before testing
reset_timeout: Duration::from_secs(120)  // Alternative timeout strategy
```

**Monitoring Metrics:**
```
circuit_breaker_state{name="sentinel", state="open"} = 1
circuit_breaker_rejected_requests{name="sentinel"} = 1234
circuit_breaker_open_duration_seconds{name="sentinel"} = 45
```

**Error Response:**
```rust
Err(IntegrationError::CircuitOpen {
    breaker_name: "sentinel",
    opened_at: "2025-11-13T10:30:15Z",
    retry_after_secs: 45,
})
```

#### 3. HALF-OPEN State (Testing Recovery)

**Behavior:**
- Limited number of test requests are allowed through
- Service is being tested for recovery
- Success → transition to CLOSED
- Failure → transition back to OPEN

```
┌──────────────────────────────────┐
│       HALF-OPEN STATE            │
│                                  │
│  Current Status: ? Testing       │
│  Test Requests: 2 / 3            │
│  Successes: 2                    │
│  Required: 2 consecutive         │
│                                  │
│  ┌────────┐                      │
│  │Request │──→ [Test] ──→ ?      │
│  └────────┘     (limited)        │
└──────────────────────────────────┘
         │
         ├─── 2 successes ──────────┐
         │                          │
         │                          ▼
         │                ┌──────────────────┐
         │                │  CLOSED STATE    │
         │                │  (Recovered)     │
         │                └──────────────────┘
         │
         ├─── 1 failure ────────────┐
         │                          │
         │                          ▼
         │                ┌──────────────────┐
         │                │   OPEN STATE     │
         │                │  (Still failing) │
         │                └──────────────────┘
```

**Configuration Parameters:**
```rust
success_threshold: 2           // Need 2 successes to close
half_open_max_requests: 3      // Allow max 3 test requests
half_open_timeout: Duration::from_secs(30)  // Test for 30s max
```

**Monitoring Metrics:**
```
circuit_breaker_state{name="sentinel", state="half_open"} = 1
circuit_breaker_half_open_successes{name="sentinel"} = 2
circuit_breaker_half_open_failures{name="sentinel"} = 0
```

### Complete State Transition Diagram

```
                    ┌─────────────────────────────────┐
                    │                                 │
                    │    INITIALIZATION               │
                    │                                 │
                    └───────────────┬─────────────────┘
                                    │
                                    │ Start
                                    │
                                    ▼
              ┌──────────────────────────────────────────┐
              │                                          │
              │            CLOSED STATE                  │
              │         (Normal Operation)               │
              │                                          │
              │  • Allow all requests                    │
              │  • Track success/failure                 │
              │  • Monitor thresholds                    │
              │                                          │
              └────────┬─────────────────────────┬───────┘
                       │                         │
                       │                         │ Self-loop on
                       │                         │ success or
                       │                         │ below threshold
                       │                         │
                       │ Failure threshold       └────┐
                       │ exceeded                     │
                       │ (e.g., 5 failures in 10 requests)
                       │                              │
                       ▼                              │
              ┌──────────────────────────────────────────┐
              │                                          │
              │             OPEN STATE                   │
              │           (Circuit Tripped)              │
              │                                          │
              │  • Reject all requests (fast fail)       │
              │  • No external calls made                │
              │  • Start timeout timer                   │
              │  • Wait for recovery period              │
              │                                          │
              └────────┬─────────────────────────────────┘
                       │
                       │ Timeout expires
                       │ (e.g., after 60 seconds)
                       │
                       ▼
              ┌──────────────────────────────────────────┐
              │                                          │
              │          HALF-OPEN STATE                 │
              │       (Testing Recovery)                 │
              │                                          │
              │  • Allow limited test requests           │
              │  • Evaluate service health               │
              │  • Make recovery decision                │
              │                                          │
              └───┬──────────────────────────────────┬───┘
                  │                                  │
                  │                                  │
                  │ Success threshold met            │ Any failure
                  │ (e.g., 2 consecutive successes)  │ detected
                  │                                  │
                  ▼                                  │
           ┌─────────────┐                          │
           │   CLOSED    │                          │
           │  (Recovered)│                          │
           └─────────────┘                          │
                                                     │
                                                     ▼
                                              ┌─────────────┐
                                              │    OPEN     │
                                              │ (Still Down)│
                                              └─────────────┘
```

### State Transition Conditions

| From State | To State   | Condition | Action |
|------------|------------|-----------|--------|
| CLOSED     | OPEN       | `failures >= failure_threshold` within time window | Start timeout timer, log event, emit metric |
| CLOSED     | CLOSED     | `failures < failure_threshold` | Continue normal operation |
| OPEN       | HALF-OPEN  | Timeout expired | Allow limited test requests |
| OPEN       | OPEN       | Timeout not expired | Continue rejecting requests |
| HALF-OPEN  | CLOSED     | `successes >= success_threshold` | Reset counters, log recovery |
| HALF-OPEN  | OPEN       | Any failure | Restart timeout timer, increment open count |
| HALF-OPEN  | HALF-OPEN  | Test in progress | Continue testing |

### Timing Diagrams

#### Normal Operation (CLOSED → CLOSED)

```
Time    Circuit State    Requests         Result
────────────────────────────────────────────────────
0ms     CLOSED          Request 1   →    Success ✓
10ms    CLOSED          Request 2   →    Success ✓
20ms    CLOSED          Request 3   →    Success ✓
30ms    CLOSED          Request 4   →    Success ✓
40ms    CLOSED          Request 5   →    Success ✓

Stats: 5/5 success, Circuit remains CLOSED
```

#### Failure Detection (CLOSED → OPEN)

```
Time    Circuit State    Requests         Result          Failures
──────────────────────────────────────────────────────────────────
0ms     CLOSED          Request 1   →    Success ✓       0/5
100ms   CLOSED          Request 2   →    Failure ✗       1/5
200ms   CLOSED          Request 3   →    Failure ✗       2/5
300ms   CLOSED          Request 4   →    Failure ✗       3/5
400ms   CLOSED          Request 5   →    Failure ✗       4/5
500ms   CLOSED          Request 6   →    Failure ✗       5/5
501ms   OPEN            Request 7   →    Rejected (fast) 5/5
502ms   OPEN            Request 8   →    Rejected (fast) 5/5

Circuit opens at 501ms, Timer started for 60s
```

#### Recovery Testing (OPEN → HALF-OPEN → CLOSED)

```
Time     Circuit State    Requests         Result          Notes
────────────────────────────────────────────────────────────────────
0s       OPEN            Request 1   →    Rejected       Timeout active
30s      OPEN            Request 2   →    Rejected       Timeout active
60s      HALF-OPEN       Request 3   →    Success ✓      Test 1/2
61s      HALF-OPEN       Request 4   →    Success ✓      Test 2/2
62s      CLOSED          Request 5   →    Success ✓      Recovered!
63s      CLOSED          Request 6   →    Success ✓      Normal ops
```

#### Failed Recovery (OPEN → HALF-OPEN → OPEN)

```
Time     Circuit State    Requests         Result          Notes
────────────────────────────────────────────────────────────────────
0s       OPEN            Request 1   →    Rejected       Timeout active
60s      HALF-OPEN       Request 2   →    Success ✓      Test 1/2
61s      HALF-OPEN       Request 3   →    Failure ✗      Still down!
62s      OPEN            Request 4   →    Rejected       Back to OPEN
122s     HALF-OPEN       Request 5   →    Success ✓      Test again 1/2
123s     HALF-OPEN       Request 6   →    Success ✓      Test 2/2
124s     CLOSED          Request 7   →    Success ✓      Recovered!
```

---

## Configuration Guide

### Basic Configuration

```rust
use std::time::Duration;

// Create a circuit breaker with default settings
let circuit_breaker = CircuitBreaker::new("sentinel-api")
    .build();

// Configuration with custom settings
let circuit_breaker = CircuitBreaker::new("sentinel-api")
    .failure_threshold(5)
    .success_threshold(2)
    .timeout(Duration::from_secs(60))
    .volume_threshold(10)
    .build();
```

### YAML Configuration

```yaml
# config/circuit_breakers.yaml
circuit_breakers:
  sentinel:
    name: "sentinel-api"
    enabled: true
    failure_threshold: 5
    success_threshold: 2
    timeout_secs: 60
    half_open_timeout_secs: 30
    volume_threshold: 10
    error_threshold_percentage: 50

  shield:
    name: "shield-api"
    enabled: true
    failure_threshold: 3
    success_threshold: 2
    timeout_secs: 120
    half_open_timeout_secs: 30
    volume_threshold: 5
    error_threshold_percentage: 60

  edge_agent:
    name: "edge-agent-api"
    enabled: true
    failure_threshold: 10
    success_threshold: 3
    timeout_secs: 30
    half_open_timeout_secs: 15
    volume_threshold: 20
    error_threshold_percentage: 40
```

### Environment Variables

```bash
# Override circuit breaker settings via environment variables

# Sentinel Circuit Breaker
SENTINEL_CB_ENABLED=true
SENTINEL_CB_FAILURE_THRESHOLD=5
SENTINEL_CB_SUCCESS_THRESHOLD=2
SENTINEL_CB_TIMEOUT_SECS=60
SENTINEL_CB_VOLUME_THRESHOLD=10

# Shield Circuit Breaker
SHIELD_CB_ENABLED=true
SHIELD_CB_FAILURE_THRESHOLD=3
SHIELD_CB_TIMEOUT_SECS=120

# Global Circuit Breaker Defaults
CB_DEFAULT_FAILURE_THRESHOLD=5
CB_DEFAULT_SUCCESS_THRESHOLD=2
CB_DEFAULT_TIMEOUT_SECS=60
```

### Advanced Configuration Options

```rust
use llm_incident_manager::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, RecoveryStrategy
};

let config = CircuitBreakerConfig {
    // Basic thresholds
    failure_threshold: 5,
    success_threshold: 2,
    volume_threshold: 10,

    // Timing
    timeout: Duration::from_secs(60),
    half_open_timeout: Duration::from_secs(30),

    // Error rate based triggering
    error_threshold_percentage: 50.0,
    error_window_duration: Duration::from_secs(60),

    // Recovery strategy
    recovery_strategy: RecoveryStrategy::LinearBackoff {
        initial_timeout: Duration::from_secs(60),
        max_timeout: Duration::from_secs(300),
        increment: Duration::from_secs(60),
    },

    // Behavior options
    fail_fast_on_open: true,
    half_open_max_requests: 3,
    reset_on_success: true,

    // Observability
    emit_metrics: true,
    metrics_prefix: "llm_client",
    log_state_changes: true,
    log_level: tracing::Level::WARN,
};

let circuit_breaker = CircuitBreaker::with_config("sentinel-api", config);
```

### Configuration Parameters Reference

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `failure_threshold` | `u32` | `5` | Number of failures before opening circuit |
| `success_threshold` | `u32` | `2` | Number of successes needed to close circuit |
| `timeout` | `Duration` | `60s` | Time to wait before entering half-open state |
| `volume_threshold` | `u32` | `10` | Minimum requests before evaluation |
| `error_threshold_percentage` | `f64` | `50.0` | Error rate % that triggers circuit open |
| `half_open_timeout` | `Duration` | `30s` | Max time to stay in half-open state |
| `half_open_max_requests` | `u32` | `3` | Max test requests in half-open state |
| `fail_fast_on_open` | `bool` | `true` | Immediately fail when circuit is open |
| `reset_on_success` | `bool` | `true` | Reset failure count on success |

---

## Integration Examples

### Example 1: Basic Integration with Sentinel Client

```rust
use llm_incident_manager::{
    circuit_breaker::CircuitBreaker,
    integrations::sentinel::SentinelClient,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create circuit breaker
    let circuit_breaker = Arc::new(
        CircuitBreaker::new("sentinel")
            .failure_threshold(5)
            .timeout(Duration::from_secs(60))
            .build()
    );

    // Create Sentinel client
    let mut sentinel = SentinelClient::new(
        ConnectionConfig::new("https://sentinel-api.example.com".to_string()),
        Credentials::api_key("your-api-key")
    )?;

    sentinel.connect().await?;

    // Execute request through circuit breaker
    let result = circuit_breaker.call(|| async {
        sentinel.fetch_alerts(Some(10)).await
    }).await;

    match result {
        Ok(alerts) => {
            println!("Fetched {} alerts", alerts.len());
        }
        Err(e) if e.is_circuit_open() => {
            println!("Circuit breaker is open, service unavailable");
            // Use cached data or fallback logic
        }
        Err(e) => {
            println!("Request failed: {}", e);
        }
    }

    Ok(())
}
```

### Example 2: Integration with Retry Logic

```rust
use llm_incident_manager::{
    circuit_breaker::CircuitBreaker,
    integrations::common::{RetryPolicy, retry_with_backoff},
};

async fn fetch_with_resilience() -> IntegrationResult<Vec<SentinelAlert>> {
    let circuit_breaker = CircuitBreaker::new("sentinel").build();
    let retry_policy = RetryPolicy::default();

    // First check circuit breaker
    circuit_breaker.call(|| async {
        // Then apply retry logic
        retry_with_backoff(
            "fetch_alerts",
            &retry_policy,
            || async {
                sentinel.fetch_alerts(Some(10)).await
            }
        ).await
    }).await
}
```

### Example 3: Multiple Circuit Breakers (Registry Pattern)

```rust
use llm_incident_manager::circuit_breaker::CircuitBreakerRegistry;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry
    let mut registry = CircuitBreakerRegistry::new();

    // Register circuit breakers for different services
    registry.register("sentinel", CircuitBreaker::new("sentinel").build());
    registry.register("shield", CircuitBreaker::new("shield").build());
    registry.register("edge-agent", CircuitBreaker::new("edge-agent").build());

    // Get circuit breaker for specific service
    let sentinel_cb = registry.get("sentinel").unwrap();

    // Execute request
    let result = sentinel_cb.call(|| async {
        sentinel_client.fetch_alerts(Some(10)).await
    }).await;

    // Check health of all circuit breakers
    let health = registry.health_check();
    println!("Circuit breakers health: {:?}", health);

    Ok(())
}
```

### Example 4: Graceful Degradation

```rust
use llm_incident_manager::circuit_breaker::{CircuitBreaker, CircuitState};

async fn analyze_with_fallback(
    circuit_breaker: &CircuitBreaker,
    sentinel: &SentinelClient,
    data: serde_json::Value,
) -> Result<AnomalyAnalysis, AppError> {
    // Try primary service through circuit breaker
    match circuit_breaker.call(|| async {
        sentinel.analyze_anomaly(data.clone()).await
    }).await {
        Ok(analysis) => Ok(analysis),
        Err(e) if e.is_circuit_open() => {
            // Circuit is open, use fallback
            warn!("Circuit breaker open, using fallback analysis");

            // Option 1: Use cached result
            if let Some(cached) = get_cached_analysis(&data) {
                return Ok(cached);
            }

            // Option 2: Use simple rule-based analysis
            Ok(simple_rule_based_analysis(data))
        }
        Err(e) => {
            error!("Analysis failed: {}", e);
            Err(AppError::from(e))
        }
    }
}

fn simple_rule_based_analysis(data: serde_json::Value) -> AnomalyAnalysis {
    // Simple threshold-based analysis as fallback
    AnomalyAnalysis {
        is_anomalous: false, // Conservative default
        confidence: 0.5,
        anomaly_type: "unknown".to_string(),
        details: Some("Fallback analysis - circuit breaker open".to_string()),
    }
}
```

### Example 5: Manual Circuit Control

```rust
use llm_incident_manager::circuit_breaker::CircuitBreaker;

async fn manual_circuit_control() {
    let circuit_breaker = CircuitBreaker::new("sentinel").build();

    // Manually open circuit (for maintenance)
    circuit_breaker.force_open("Scheduled maintenance").await;

    // All requests will now fail fast
    let result = circuit_breaker.call(|| async {
        sentinel.fetch_alerts(Some(10)).await
    }).await;
    assert!(result.unwrap_err().is_circuit_open());

    // Manually close circuit (after maintenance)
    circuit_breaker.force_close().await;

    // Reset circuit to initial state
    circuit_breaker.reset().await;
}
```

---

## Best Practices

### 1. Threshold Configuration

**DO:**
- Start with conservative thresholds (e.g., 5 failures)
- Adjust based on observed failure patterns
- Set different thresholds for different services
- Consider service SLA when setting timeouts

**DON'T:**
- Use very low thresholds (e.g., 1-2 failures) - too sensitive
- Use very high thresholds (e.g., 50+ failures) - defeats purpose
- Use same configuration for all services
- Set timeout shorter than typical request duration

**Recommended Starting Values:**

| Service Type | Failure Threshold | Timeout | Volume Threshold |
|--------------|-------------------|---------|------------------|
| Critical (LLM APIs) | 3-5 | 60s | 5 |
| Standard APIs | 5-10 | 60-120s | 10 |
| Non-critical | 10-20 | 30-60s | 20 |
| High-traffic | 20-50 | 30s | 50 |

### 2. Monitoring and Alerting

**Essential Metrics to Monitor:**
```
- circuit_breaker_state
- circuit_breaker_open_count
- circuit_breaker_half_open_count
- circuit_breaker_rejected_requests_total
- circuit_breaker_request_duration_seconds
- circuit_breaker_failure_rate
```

**Alert on:**
- Circuit opens (immediate notification)
- Circuit remains open > 5 minutes (escalation)
- Circuit opens > 3 times in 1 hour (pattern alert)
- High failure rate before circuit opens (early warning)

### 3. Graceful Degradation

Always implement fallback strategies:

```rust
// Good: Multiple fallback levels
let result = circuit_breaker.call(|| primary_service()).await
    .or_else(|| secondary_service())
    .or_else(|| cached_result())
    .or_else(|| default_value());

// Bad: No fallback
let result = circuit_breaker.call(|| primary_service()).await
    .expect("This will panic if circuit opens!");
```

### 4. Service Dependency Management

**Independent Circuit Breakers:**
- One circuit breaker per external service
- Isolate failures to specific services
- Prevent cascading circuit opens

```rust
// Good: Separate circuit breakers
let sentinel_cb = CircuitBreaker::new("sentinel").build();
let shield_cb = CircuitBreaker::new("shield").build();

// Bad: Shared circuit breaker
let shared_cb = CircuitBreaker::new("all-services").build();
```

### 5. Testing

**Unit Tests:**
```rust
#[tokio::test]
async fn test_circuit_opens_on_failures() {
    let cb = CircuitBreaker::new("test").failure_threshold(3).build();

    // Simulate failures
    for _ in 0..3 {
        let _ = cb.call(|| async { Err::<(), _>("failure") }).await;
    }

    // Circuit should be open
    assert_eq!(cb.state().await, CircuitState::Open);
}
```

**Integration Tests:**
```rust
#[tokio::test]
async fn test_circuit_recovery() {
    let cb = CircuitBreaker::new("test")
        .failure_threshold(2)
        .timeout(Duration::from_millis(100))
        .build();

    // Open circuit
    let _ = cb.call(|| async { Err::<(), _>("failure") }).await;
    let _ = cb.call(|| async { Err::<(), _>("failure") }).await;
    assert_eq!(cb.state().await, CircuitState::Open);

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should be half-open
    assert_eq!(cb.state().await, CircuitState::HalfOpen);

    // Successful requests should close circuit
    let _ = cb.call(|| async { Ok::<(), &str>(()) }).await;
    let _ = cb.call(|| async { Ok::<(), &str>(()) }).await;
    assert_eq!(cb.state().await, CircuitState::Closed);
}
```

### 6. Documentation

Always document:
- Why specific threshold values were chosen
- Expected behavior during outages
- Fallback strategies
- Recovery procedures
- Metrics and alerts

---

## Troubleshooting

### Problem: Circuit Breaker Opens Too Frequently

**Symptoms:**
- Circuit opens multiple times per hour
- Many fast-fail errors in logs
- Service appears unstable

**Possible Causes:**
1. Threshold too low for normal failure rate
2. Transient network issues
3. Service genuinely unstable
4. Timeout set too short

**Solutions:**
```rust
// Increase failure threshold
let cb = CircuitBreaker::new("sentinel")
    .failure_threshold(10) // was 5
    .build();

// Increase volume threshold (require more samples)
let cb = CircuitBreaker::new("sentinel")
    .volume_threshold(20) // was 10
    .build();

// Use error rate instead of absolute count
let cb = CircuitBreaker::new("sentinel")
    .error_threshold_percentage(60.0) // 60% error rate
    .volume_threshold(10)
    .build();
```

### Problem: Circuit Doesn't Open When It Should

**Symptoms:**
- Service failing but circuit stays closed
- Timeouts continue to consume resources
- System degradation not prevented

**Possible Causes:**
1. Volume threshold not met
2. Failure threshold too high
3. Errors not being counted correctly
4. Circuit breaker not in request path

**Solutions:**
```rust
// Lower volume threshold
let cb = CircuitBreaker::new("sentinel")
    .volume_threshold(5) // was 10
    .build();

// Lower failure threshold
let cb = CircuitBreaker::new("sentinel")
    .failure_threshold(3) // was 5
    .build();

// Verify error counting
cb.call(|| async {
    match service.call().await {
        Ok(result) => Ok(result),
        Err(e) => {
            // Ensure error is counted
            error!("Service call failed: {}", e);
            Err(e) // Must propagate error
        }
    }
}).await
```

### Problem: Circuit Remains Open Too Long

**Symptoms:**
- Circuit doesn't transition to half-open
- Service recovered but circuit still open
- Manual intervention required

**Possible Causes:**
1. Timeout set too long
2. Half-open state failing immediately
3. Success threshold too high

**Solutions:**
```rust
// Reduce timeout
let cb = CircuitBreaker::new("sentinel")
    .timeout(Duration::from_secs(30)) // was 60
    .build();

// Reduce success threshold
let cb = CircuitBreaker::new("sentinel")
    .success_threshold(1) // was 2
    .build();

// Manual circuit reset
cb.reset().await;
```

### Problem: Half-Open State Oscillation

**Symptoms:**
- Circuit keeps transitioning between half-open and open
- Never successfully closes
- Logs show alternating success/failure

**Possible Causes:**
1. Service intermittently failing
2. Success threshold too high
3. Half-open timeout too short

**Solutions:**
```rust
// Increase half-open max requests
let cb = CircuitBreaker::new("sentinel")
    .half_open_max_requests(5) // was 3
    .build();

// Increase half-open timeout
let cb = CircuitBreaker::new("sentinel")
    .half_open_timeout(Duration::from_secs(60)) // was 30
    .build();

// Use exponential backoff strategy
let cb = CircuitBreaker::new("sentinel")
    .recovery_strategy(RecoveryStrategy::ExponentialBackoff {
        initial_timeout: Duration::from_secs(60),
        max_timeout: Duration::from_secs(300),
        multiplier: 2.0,
    })
    .build();
```

### Debugging Commands

```bash
# Check circuit breaker state via API
curl http://localhost:8080/v1/circuit-breakers

# Check specific circuit breaker
curl http://localhost:8080/v1/circuit-breakers/sentinel

# Get metrics
curl http://localhost:9090/metrics | grep circuit_breaker

# Force circuit open (for testing)
curl -X POST http://localhost:8080/v1/circuit-breakers/sentinel/open

# Force circuit closed (after maintenance)
curl -X POST http://localhost:8080/v1/circuit-breakers/sentinel/close

# Reset circuit breaker
curl -X POST http://localhost:8080/v1/circuit-breakers/sentinel/reset
```

---

## Performance Considerations

### Memory Usage

Each circuit breaker maintains:
- State information (~100 bytes)
- Failure/success counters (~16 bytes)
- Timing information (~24 bytes)
- Request history (configurable)

**Total per circuit breaker: ~200-500 bytes**

For 100 circuit breakers: ~20-50 KB total memory

### CPU Overhead

Circuit breaker operations:
- State check: < 100ns (lock-free atomic read)
- Success recording: < 200ns (atomic increment)
- Failure recording: < 500ns (atomic increment + state check)
- State transition: < 1μs (mutex lock + state update)

**Total overhead: < 0.1% for typical workloads**

### Latency Impact

- **Closed state**: ~100ns overhead
- **Open state**: ~50ns overhead (immediate reject)
- **Half-open state**: ~200ns overhead

**Recommendation:** Circuit breakers add negligible latency (<1ms) compared to network calls (100ms+)

### Throughput Impact

Circuit breakers are designed for high throughput:
- Lock-free atomic operations for common path
- No blocking in hot path
- Minimal memory allocations

**Expected throughput: > 1M requests/sec per circuit breaker**

### Scaling Guidelines

| Concurrent Requests | Circuit Breakers | Memory | CPU |
|---------------------|------------------|--------|-----|
| 1,000 | 10 | < 1 MB | < 1% |
| 10,000 | 50 | < 5 MB | < 2% |
| 100,000 | 100 | < 10 MB | < 5% |

---

## Conclusion

Circuit breakers are essential for building resilient distributed systems. They provide:

1. **Fast failure** - Stop wasting resources on failing services
2. **Automatic recovery** - Self-healing when services recover
3. **System protection** - Prevent cascading failures
4. **Observability** - Real-time visibility into service health

**Key Takeaways:**

- Start with conservative thresholds and tune based on metrics
- Always implement fallback strategies
- Monitor circuit breaker state and transitions
- Test circuit breaker behavior under failure conditions
- Document configuration decisions and recovery procedures

For more information:
- [Circuit Breaker API Reference](./CIRCUIT_BREAKER_API_REFERENCE.md)
- [Integration Guide](./CIRCUIT_BREAKER_INTEGRATION_GUIDE.md)
- [Operations Guide](./CIRCUIT_BREAKER_OPERATIONS_GUIDE.md)

---

**Document Version**: 1.0.0
**Last Updated**: 2025-11-13
**Authors**: LLM Incident Manager Team
**Status**: Production Ready
