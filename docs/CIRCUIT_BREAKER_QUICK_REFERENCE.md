# Circuit Breaker Quick Reference

## Quick Start

### 1. Basic Usage

```rust
use llm_incident_manager::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

let config = CircuitBreakerConfig::default();
let breaker = CircuitBreaker::new("my-service", config);

let result = breaker.call(|| Box::pin(async {
    // Your async operation
    Ok::<String, std::io::Error>("success".to_string())
})).await?;
```

### 2. With Fallback

```rust
let result = breaker.call_with_fallback(
    || Box::pin(async { api_call().await }),
    || Box::pin(async { cached_value() })
).await?;
```

### 3. LLM Service Integration

```rust
use llm_incident_manager::integrations::SentinelClientWithBreaker;

let client = SentinelClient::new(config, credentials)?;
let protected = SentinelClientWithBreaker::new(client);
let alerts = protected.fetch_alerts(Some(10)).await?;
```

### 4. Database Integration

```rust
use llm_incident_manager::state::CircuitBreakerStore;

let store = create_store(&config).await?;
let protected = CircuitBreakerStore::new(store, "db");
let incident = protected.get_incident(&id).await?;
```

### 5. Notification Integration

```rust
use llm_incident_manager::notifications::SlackSenderWithBreaker;

let sender = SlackSender::new(config);
let protected = SlackSenderWithBreaker::new(sender);
protected.send(&incident).await?;
```

## Predefined Configurations

```rust
// HTTP APIs (30s timeout, 5 failures)
CircuitBreakerConfig::for_http_api()

// LLM Services (120s timeout, 3 failures)
CircuitBreakerConfig::for_llm_service()

// Database (10s timeout, 10 failures)
CircuitBreakerConfig::for_database()

// Notifications (60s timeout, 10 failures)
CircuitBreakerConfig::for_notifications()

// Cache/Redis (20s timeout, 8 failures)
CircuitBreakerConfig::for_cache()
```

## Custom Configuration

```rust
let config = CircuitBreakerConfig::builder()
    .failure_threshold(5)           // Failures before opening
    .success_threshold(2)            // Successes to close
    .timeout_duration(Duration::from_secs(60))
    .half_open_max_requests(3)      // Max requests in half-open
    .build()?;
```

## State Management

```rust
// Get current state
let state = breaker.state(); // Closed | Open | HalfOpen

// Get statistics
let stats = breaker.stats();
println!("Failures: {}, Successes: {}",
    stats.consecutive_failures,
    stats.consecutive_successes);

// Manual control
breaker.reset();       // Force to Closed
breaker.force_open();  // Force to Open
```

## Registry Operations

```rust
use llm_incident_manager::circuit_breaker::GLOBAL_CIRCUIT_BREAKER_REGISTRY;

// List all circuit breakers
let names = GLOBAL_CIRCUIT_BREAKER_REGISTRY.list_names();

// Health check
let health = GLOBAL_CIRCUIT_BREAKER_REGISTRY.health_check();

// Reset all
GLOBAL_CIRCUIT_BREAKER_REGISTRY.reset_all();

// Check for issues
if GLOBAL_CIRCUIT_BREAKER_REGISTRY.has_open_circuits() {
    // Handle degraded state
}
```

## Metrics

All circuit breakers automatically export Prometheus metrics:

- `circuit_breaker_state` - Current state (0/1/2)
- `circuit_breaker_calls_total` - Total calls
- `circuit_breaker_successful_calls_total` - Successes
- `circuit_breaker_failed_calls_total` - Failures
- `circuit_breaker_rejected_calls_total` - Rejected (open)
- `circuit_breaker_call_duration_seconds` - Duration histogram
- `circuit_breaker_state_transitions_total` - State changes

## Integration Patterns

### Pattern 1: Wrap Existing Client

```rust
let client = ExternalClient::new(config);
let protected = ClientWithBreaker::new(client);
// Use protected instead of client
```

### Pattern 2: Function Decorator

```rust
use llm_incident_manager::circuit_breaker::with_circuit_breaker;

let result = with_circuit_breaker("service", config, || {
    Box::pin(async { do_something().await })
}).await?;
```

### Pattern 3: HTTP Middleware

```rust
use llm_incident_manager::circuit_breaker::CircuitBreakerLayer;

let app = Router::new()
    .route("/api", get(handler))
    .layer(CircuitBreakerLayer::for_http_api("external"));
```

### Pattern 4: Store Wrapper

```rust
let store = create_store(&config).await?;
let protected = CircuitBreakerStore::new(store, "incidents");
// Use protected for all store operations
```

## Common Issues

### Issue: Circuit Opens Too Quickly
**Solution**: Increase `failure_threshold` or `timeout_duration`

```rust
let config = CircuitBreakerConfig::builder()
    .failure_threshold(10)  // Increase tolerance
    .timeout_duration(Duration::from_secs(120))  // Longer recovery
    .build()?;
```

### Issue: Circuit Stays Open Too Long
**Solution**: Decrease `timeout_duration`

```rust
let config = CircuitBreakerConfig::builder()
    .timeout_duration(Duration::from_secs(30))  // Faster recovery attempts
    .build()?;
```

### Issue: Too Many Rejected Requests in Half-Open
**Solution**: Increase `half_open_max_requests`

```rust
let config = CircuitBreakerConfig::builder()
    .half_open_max_requests(5)  // Allow more test requests
    .build()?;
```

## Error Handling

```rust
match result {
    Ok(value) => { /* Success */ },
    Err(CircuitBreakerError::Open(name)) => {
        // Circuit is open - use fallback
        log::warn!("Circuit {} is open", name);
    },
    Err(CircuitBreakerError::OperationFailed(msg)) => {
        // Operation failed but circuit still closed
        log::error!("Operation failed: {}", msg);
    },
    Err(e) => {
        // Other errors
        log::error!("Circuit breaker error: {}", e);
    }
}
```

## Testing

```rust
#[tokio::test]
async fn test_circuit_breaker() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .build()
        .unwrap();

    let breaker = CircuitBreaker::new("test", config);

    // Should succeed
    let result = breaker.call(|| Box::pin(async {
        Ok::<i32, std::io::Error>(42)
    })).await;
    assert!(result.is_ok());

    // Cause failures
    for _ in 0..2 {
        let _ = breaker.call(|| Box::pin(async {
            Err::<i32, std::io::Error>(
                std::io::Error::new(std::io::ErrorKind::Other, "test")
            )
        })).await;
    }

    // Should be open
    assert_eq!(breaker.state(), CircuitBreakerState::Open);
}
```

## Monitoring Queries

### Prometheus Queries

```promql
# Circuit breaker states
llm_incident_manager_circuit_breaker_state

# Open circuits
llm_incident_manager_circuit_breaker_state{state="1"}

# Failure rate
rate(llm_incident_manager_circuit_breaker_failed_calls_total[5m])

# Rejection rate
rate(llm_incident_manager_circuit_breaker_rejected_calls_total[5m])

# 99th percentile latency
histogram_quantile(0.99,
  rate(llm_incident_manager_circuit_breaker_call_duration_seconds_bucket[5m])
)
```

### Alert Rules

```yaml
groups:
  - name: circuit_breaker
    rules:
      - alert: CircuitBreakerOpen
        expr: llm_incident_manager_circuit_breaker_state == 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Circuit breaker {{ $labels.name }} is open"

      - alert: HighRejectionRate
        expr: rate(llm_incident_manager_circuit_breaker_rejected_calls_total[5m]) > 10
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High rejection rate on {{ $labels.name }}"
```

## Cheat Sheet

| Operation | Command |
|-----------|---------|
| Create breaker | `CircuitBreaker::new("name", config)` |
| Execute protected call | `breaker.call(\|\| Box::pin(async { ... })).await` |
| Execute with fallback | `breaker.call_with_fallback(primary, fallback).await` |
| Check state | `breaker.state()` |
| Get stats | `breaker.stats()` |
| Reset breaker | `breaker.reset()` |
| Force open | `breaker.force_open()` |
| Get from registry | `get_circuit_breaker("name", config)` |
| Health check | `GLOBAL_CIRCUIT_BREAKER_REGISTRY.health_check()` |
| List all | `GLOBAL_CIRCUIT_BREAKER_REGISTRY.list_names()` |

## See Also

- [Full Implementation Guide](CIRCUIT_BREAKER_IMPLEMENTATION.md)
- [Prometheus Metrics Documentation](../src/circuit_breaker/metrics.rs)
- [Configuration Examples](../src/circuit_breaker/config.rs)
