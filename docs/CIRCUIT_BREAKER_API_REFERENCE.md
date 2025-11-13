# Circuit Breaker API Reference

**Version**: 1.0.0
**Last Updated**: 2025-11-13

---

## Table of Contents

1. [Overview](#overview)
2. [Core Types](#core-types)
3. [CircuitBreaker API](#circuitbreaker-api)
4. [CircuitBreakerRegistry API](#circuitbreakerregistry-api)
5. [Configuration Types](#configuration-types)
6. [Error Types](#error-types)
7. [Metrics API](#metrics-api)
8. [Usage Examples](#usage-examples)

---

## Overview

This document provides complete API reference documentation for the Circuit Breaker implementation in the LLM Incident Manager system. All APIs are designed for production use with a focus on type safety, performance, and observability.

### Module Structure

```
llm_incident_manager::circuit_breaker
├── CircuitBreaker           // Core circuit breaker implementation
├── CircuitBreakerRegistry   // Registry for managing multiple breakers
├── CircuitBreakerConfig     // Configuration structures
├── CircuitState             // State enumeration
├── CircuitBreakerError      // Error types
├── CircuitBreakerMetrics    // Metrics collection
└── RecoveryStrategy         // Recovery behavior configuration
```

---

## Core Types

### CircuitState

Represents the current state of a circuit breaker.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed, requests are allowed
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, testing recovery
    HalfOpen,
}
```

**Methods:**

```rust
impl CircuitState {
    /// Check if circuit is closed
    pub fn is_closed(&self) -> bool;

    /// Check if circuit is open
    pub fn is_open(&self) -> bool;

    /// Check if circuit is half-open
    pub fn is_half_open(&self) -> bool;

    /// Get human-readable state name
    pub fn as_str(&self) -> &'static str;
}
```

**Example:**

```rust
use llm_incident_manager::circuit_breaker::CircuitState;

let state = CircuitState::Closed;
assert!(state.is_closed());
assert_eq!(state.as_str(), "closed");
```

### CircuitBreakerInfo

Contains current circuit breaker status information.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerInfo {
    /// Breaker name
    pub name: String,
    /// Current state
    pub state: CircuitState,
    /// Number of failures in current window
    pub failure_count: u32,
    /// Number of successes in current window
    pub success_count: u32,
    /// Total requests in current window
    pub total_requests: u32,
    /// Number of consecutive failures (in open state)
    pub consecutive_failures: u32,
    /// Number of consecutive successes (in half-open state)
    pub consecutive_successes: u32,
    /// Timestamp when current state started
    pub state_since: chrono::DateTime<chrono::Utc>,
    /// Timestamp when circuit last opened
    pub last_opened_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of times circuit has opened
    pub open_count: u32,
    /// Current error rate (0.0 - 1.0)
    pub error_rate: f64,
}
```

**Example:**

```rust
let info = circuit_breaker.info().await;
println!("Circuit: {} - State: {:?}", info.name, info.state);
println!("Failures: {} / {}", info.failure_count, info.total_requests);
println!("Error rate: {:.2}%", info.error_rate * 100.0);
```

---

## CircuitBreaker API

### Constructor and Builder

#### `CircuitBreaker::new`

Creates a new circuit breaker with a given name.

```rust
pub fn new(name: impl Into<String>) -> CircuitBreakerBuilder
```

**Parameters:**
- `name`: Unique identifier for the circuit breaker

**Returns:** `CircuitBreakerBuilder` for configuration

**Example:**

```rust
let breaker = CircuitBreaker::new("sentinel-api")
    .failure_threshold(5)
    .timeout(Duration::from_secs(60))
    .build();
```

#### `CircuitBreaker::with_config`

Creates a circuit breaker with a complete configuration.

```rust
pub fn with_config(
    name: impl Into<String>,
    config: CircuitBreakerConfig
) -> Self
```

**Parameters:**
- `name`: Unique identifier
- `config`: Full configuration structure

**Returns:** Configured `CircuitBreaker`

**Example:**

```rust
let config = CircuitBreakerConfig {
    failure_threshold: 5,
    success_threshold: 2,
    timeout: Duration::from_secs(60),
    ..Default::default()
};

let breaker = CircuitBreaker::with_config("sentinel-api", config);
```

### CircuitBreakerBuilder

#### Builder Methods

```rust
impl CircuitBreakerBuilder {
    /// Set failure threshold (default: 5)
    pub fn failure_threshold(mut self, threshold: u32) -> Self;

    /// Set success threshold (default: 2)
    pub fn success_threshold(mut self, threshold: u32) -> Self;

    /// Set timeout duration (default: 60s)
    pub fn timeout(mut self, duration: Duration) -> Self;

    /// Set half-open timeout (default: 30s)
    pub fn half_open_timeout(mut self, duration: Duration) -> Self;

    /// Set volume threshold (default: 10)
    pub fn volume_threshold(mut self, threshold: u32) -> Self;

    /// Set error threshold percentage (default: 50.0)
    pub fn error_threshold_percentage(mut self, percentage: f64) -> Self;

    /// Set maximum test requests in half-open state (default: 3)
    pub fn half_open_max_requests(mut self, max: u32) -> Self;

    /// Set recovery strategy (default: FixedTimeout)
    pub fn recovery_strategy(mut self, strategy: RecoveryStrategy) -> Self;

    /// Enable/disable metrics emission (default: true)
    pub fn emit_metrics(mut self, enable: bool) -> Self;

    /// Set metrics prefix (default: "circuit_breaker")
    pub fn metrics_prefix(mut self, prefix: impl Into<String>) -> Self;

    /// Build the circuit breaker
    pub fn build(self) -> CircuitBreaker;
}
```

**Example:**

```rust
let breaker = CircuitBreaker::new("custom-service")
    .failure_threshold(10)
    .success_threshold(3)
    .timeout(Duration::from_secs(120))
    .error_threshold_percentage(60.0)
    .recovery_strategy(RecoveryStrategy::ExponentialBackoff {
        initial_timeout: Duration::from_secs(60),
        max_timeout: Duration::from_secs(300),
        multiplier: 2.0,
    })
    .build();
```

### Core Methods

#### `call`

Executes an async operation through the circuit breaker.

```rust
pub async fn call<F, Fut, T, E>(
    &self,
    operation: F
) -> Result<T, CircuitBreakerError<E>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
```

**Parameters:**
- `operation`: Async function to execute

**Returns:**
- `Ok(T)` on success
- `Err(CircuitBreakerError<E>)` on failure

**Error Types:**
- `CircuitBreakerError::Open`: Circuit is open, request rejected
- `CircuitBreakerError::Timeout`: Operation timed out
- `CircuitBreakerError::OperationFailed(E)`: Operation failed with error E

**Example:**

```rust
let result = breaker.call(|| async {
    sentinel_client.fetch_alerts(Some(10)).await
}).await;

match result {
    Ok(alerts) => println!("Success: {} alerts", alerts.len()),
    Err(CircuitBreakerError::Open { .. }) => {
        println!("Circuit open, using fallback");
    }
    Err(CircuitBreakerError::OperationFailed(e)) => {
        println!("Operation failed: {}", e);
    }
    Err(e) => println!("Other error: {}", e),
}
```

#### `call_with_timeout`

Executes an operation with a custom timeout.

```rust
pub async fn call_with_timeout<F, Fut, T, E>(
    &self,
    operation: F,
    timeout: Duration,
) -> Result<T, CircuitBreakerError<E>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
```

**Example:**

```rust
let result = breaker.call_with_timeout(
    || async {
        long_running_operation().await
    },
    Duration::from_secs(30)
).await;
```

#### `try_call`

Attempts to execute an operation, returns immediately if circuit is open.

```rust
pub async fn try_call<F, Fut, T, E>(
    &self,
    operation: F
) -> Option<Result<T, CircuitBreakerError<E>>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
```

**Returns:**
- `Some(Ok(T))`: Success
- `Some(Err(E))`: Operation failed
- `None`: Circuit is open, request not attempted

**Example:**

```rust
match breaker.try_call(|| async {
    service.call().await
}).await {
    Some(Ok(result)) => println!("Success: {:?}", result),
    Some(Err(e)) => println!("Failed: {:?}", e),
    None => println!("Circuit open, not attempted"),
}
```

### State Query Methods

#### `state`

Gets the current circuit breaker state.

```rust
pub async fn state(&self) -> CircuitState
```

**Example:**

```rust
let state = breaker.state().await;
println!("Current state: {:?}", state);
```

#### `info`

Gets comprehensive circuit breaker information.

```rust
pub async fn info(&self) -> CircuitBreakerInfo
```

**Example:**

```rust
let info = breaker.info().await;
println!("State: {:?}", info.state);
println!("Failures: {} / {}", info.failure_count, info.total_requests);
println!("Error rate: {:.2}%", info.error_rate * 100.0);
```

#### `is_closed` / `is_open` / `is_half_open`

Quick state checks.

```rust
pub async fn is_closed(&self) -> bool;
pub async fn is_open(&self) -> bool;
pub async fn is_half_open(&self) -> bool;
```

**Example:**

```rust
if breaker.is_open().await {
    println!("Circuit is open, using fallback");
    return fallback_response();
}
```

#### `health_check`

Performs a health check of the circuit breaker.

```rust
pub async fn health_check(&self) -> HealthStatus
```

**Returns:**
- `HealthStatus::Healthy`: Circuit closed, operating normally
- `HealthStatus::Degraded`: Circuit half-open, recovering
- `HealthStatus::Unhealthy`: Circuit open, service unavailable

**Example:**

```rust
match breaker.health_check().await {
    HealthStatus::Healthy => println!("All systems operational"),
    HealthStatus::Degraded => println!("Service recovering"),
    HealthStatus::Unhealthy => println!("Service unavailable"),
}
```

### Manual Control Methods

#### `force_open`

Manually opens the circuit breaker.

```rust
pub async fn force_open(&self, reason: impl Into<String>)
```

**Use cases:**
- Scheduled maintenance
- Manual intervention during incidents
- Testing failure scenarios

**Example:**

```rust
// Open circuit for maintenance
breaker.force_open("Scheduled maintenance window").await;

// Perform maintenance...

// Close when done
breaker.force_close().await;
```

#### `force_close`

Manually closes the circuit breaker.

```rust
pub async fn force_close(&self)
```

**Example:**

```rust
breaker.force_close().await;
```

#### `reset`

Resets the circuit breaker to initial state.

```rust
pub async fn reset(&self)
```

**Resets:**
- State to Closed
- All counters to zero
- Error rates
- Timers

**Example:**

```rust
// Reset after resolving issues
breaker.reset().await;
```

### Metrics Methods

#### `get_metrics`

Gets current metrics snapshot.

```rust
pub async fn get_metrics(&self) -> CircuitBreakerMetrics
```

**Returns:** `CircuitBreakerMetrics` struct with:
- Request counts (total, success, failure)
- Error rates
- Latency statistics
- State transition counts

**Example:**

```rust
let metrics = breaker.get_metrics().await;
println!("Total requests: {}", metrics.total_requests);
println!("Success rate: {:.2}%", metrics.success_rate * 100.0);
println!("Average latency: {}ms", metrics.avg_latency_ms);
```

---

## CircuitBreakerRegistry API

Registry for managing multiple circuit breakers.

### Constructor

#### `new`

Creates a new circuit breaker registry.

```rust
pub fn new() -> Self
```

**Example:**

```rust
let mut registry = CircuitBreakerRegistry::new();
```

### Registration Methods

#### `register`

Registers a circuit breaker with the registry.

```rust
pub fn register(
    &mut self,
    name: impl Into<String>,
    breaker: CircuitBreaker
) -> Result<(), RegistryError>
```

**Errors:**
- `RegistryError::DuplicateName`: Circuit breaker with same name already exists

**Example:**

```rust
let mut registry = CircuitBreakerRegistry::new();

registry.register("sentinel", CircuitBreaker::new("sentinel").build())?;
registry.register("shield", CircuitBreaker::new("shield").build())?;
registry.register("edge-agent", CircuitBreaker::new("edge-agent").build())?;
```

#### `register_or_replace`

Registers a circuit breaker, replacing any existing one with the same name.

```rust
pub fn register_or_replace(
    &mut self,
    name: impl Into<String>,
    breaker: CircuitBreaker
)
```

**Example:**

```rust
// Always succeeds, replacing if exists
registry.register_or_replace("sentinel", new_breaker);
```

#### `unregister`

Removes a circuit breaker from the registry.

```rust
pub fn unregister(&mut self, name: &str) -> Option<CircuitBreaker>
```

**Returns:**
- `Some(CircuitBreaker)`: Removed breaker
- `None`: No breaker with that name

**Example:**

```rust
if let Some(removed) = registry.unregister("sentinel") {
    println!("Removed circuit breaker: {}", removed.name());
}
```

### Access Methods

#### `get`

Gets a reference to a circuit breaker.

```rust
pub fn get(&self, name: &str) -> Option<&CircuitBreaker>
```

**Example:**

```rust
if let Some(breaker) = registry.get("sentinel") {
    let state = breaker.state().await;
    println!("Sentinel circuit state: {:?}", state);
}
```

#### `get_mut`

Gets a mutable reference to a circuit breaker.

```rust
pub fn get_mut(&mut self, name: &str) -> Option<&mut CircuitBreaker>
```

**Example:**

```rust
if let Some(breaker) = registry.get_mut("sentinel") {
    breaker.reset().await;
}
```

#### `list`

Lists all registered circuit breaker names.

```rust
pub fn list(&self) -> Vec<String>
```

**Example:**

```rust
let names = registry.list();
println!("Registered breakers: {:?}", names);
// Output: ["sentinel", "shield", "edge-agent"]
```

### Bulk Operations

#### `reset_all`

Resets all circuit breakers in the registry.

```rust
pub async fn reset_all(&self)
```

**Example:**

```rust
// Reset all breakers after maintenance
registry.reset_all().await;
```

#### `health_check_all`

Performs health check on all circuit breakers.

```rust
pub async fn health_check_all(&self) -> HashMap<String, HealthStatus>
```

**Example:**

```rust
let health = registry.health_check_all().await;
for (name, status) in health {
    println!("{}: {:?}", name, status);
}
```

#### `get_all_info`

Gets information for all circuit breakers.

```rust
pub async fn get_all_info(&self) -> HashMap<String, CircuitBreakerInfo>
```

**Example:**

```rust
let all_info = registry.get_all_info().await;
for (name, info) in all_info {
    println!("{}: state={:?}, error_rate={:.2}%",
        name, info.state, info.error_rate * 100.0);
}
```

---

## Configuration Types

### CircuitBreakerConfig

Complete configuration structure for a circuit breaker.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,

    /// Number of successes needed to close circuit from half-open
    pub success_threshold: u32,

    /// Minimum requests before circuit breaker evaluates state
    pub volume_threshold: u32,

    /// Error rate percentage that triggers circuit open (0.0 - 100.0)
    pub error_threshold_percentage: f64,

    /// Duration to wait before transitioning from open to half-open
    pub timeout: Duration,

    /// Maximum duration to stay in half-open state
    pub half_open_timeout: Duration,

    /// Maximum number of test requests in half-open state
    pub half_open_max_requests: u32,

    /// Recovery strategy configuration
    pub recovery_strategy: RecoveryStrategy,

    /// Whether to fail fast when circuit is open
    pub fail_fast_on_open: bool,

    /// Whether to reset failure count on success
    pub reset_on_success: bool,

    /// Whether to emit metrics
    pub emit_metrics: bool,

    /// Metrics prefix
    pub metrics_prefix: String,

    /// Whether to log state changes
    pub log_state_changes: bool,

    /// Log level for state changes
    pub log_level: tracing::Level,
}
```

**Default Configuration:**

```rust
impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            volume_threshold: 10,
            error_threshold_percentage: 50.0,
            timeout: Duration::from_secs(60),
            half_open_timeout: Duration::from_secs(30),
            half_open_max_requests: 3,
            recovery_strategy: RecoveryStrategy::FixedTimeout,
            fail_fast_on_open: true,
            reset_on_success: true,
            emit_metrics: true,
            metrics_prefix: "circuit_breaker".to_string(),
            log_state_changes: true,
            log_level: tracing::Level::WARN,
        }
    }
}
```

### RecoveryStrategy

Defines how the circuit breaker recovers after opening.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Fixed timeout, always same duration
    FixedTimeout,

    /// Linear backoff: timeout increases linearly
    LinearBackoff {
        initial_timeout: Duration,
        max_timeout: Duration,
        increment: Duration,
    },

    /// Exponential backoff: timeout doubles each time
    ExponentialBackoff {
        initial_timeout: Duration,
        max_timeout: Duration,
        multiplier: f64,
    },

    /// Adaptive: adjusts based on error rate
    Adaptive {
        min_timeout: Duration,
        max_timeout: Duration,
    },
}
```

**Examples:**

```rust
// Fixed timeout (default)
let strategy = RecoveryStrategy::FixedTimeout;

// Linear backoff: 60s, 120s, 180s, ...
let strategy = RecoveryStrategy::LinearBackoff {
    initial_timeout: Duration::from_secs(60),
    max_timeout: Duration::from_secs(300),
    increment: Duration::from_secs(60),
};

// Exponential backoff: 60s, 120s, 240s, ...
let strategy = RecoveryStrategy::ExponentialBackoff {
    initial_timeout: Duration::from_secs(60),
    max_timeout: Duration::from_secs(300),
    multiplier: 2.0,
};

// Adaptive: adjusts based on service health
let strategy = RecoveryStrategy::Adaptive {
    min_timeout: Duration::from_secs(30),
    max_timeout: Duration::from_secs(300),
};
```

---

## Error Types

### CircuitBreakerError

Main error type for circuit breaker operations.

```rust
#[derive(Error, Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request rejected
    #[error("Circuit breaker '{name}' is open (opened at: {opened_at}, retry after: {retry_after_secs}s)")]
    Open {
        name: String,
        opened_at: chrono::DateTime<chrono::Utc>,
        retry_after_secs: u64,
    },

    /// Operation timed out
    #[error("Operation timed out after {timeout_secs} seconds")]
    Timeout {
        timeout_secs: u64,
    },

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(E),

    /// Circuit breaker internal error
    #[error("Circuit breaker internal error: {0}")]
    Internal(String),
}
```

**Methods:**

```rust
impl<E> CircuitBreakerError<E> {
    /// Check if error is due to open circuit
    pub fn is_circuit_open(&self) -> bool;

    /// Check if error is a timeout
    pub fn is_timeout(&self) -> bool;

    /// Check if error is an operation failure
    pub fn is_operation_failed(&self) -> bool;

    /// Get retry-after duration for open circuit
    pub fn retry_after(&self) -> Option<Duration>;
}
```

**Example:**

```rust
match breaker.call(|| async { service.call().await }).await {
    Ok(result) => Ok(result),
    Err(e) if e.is_circuit_open() => {
        // Circuit is open
        if let Some(retry_after) = e.retry_after() {
            println!("Retry after {:?}", retry_after);
        }
        Err(e)
    }
    Err(e) => Err(e),
}
```

---

## Metrics API

### CircuitBreakerMetrics

Metrics structure for monitoring circuit breaker behavior.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerMetrics {
    /// Breaker name
    pub name: String,

    /// Current state
    pub state: CircuitState,

    /// Total requests
    pub total_requests: u64,

    /// Successful requests
    pub successful_requests: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Rejected requests (circuit open)
    pub rejected_requests: u64,

    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,

    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,

    /// Number of times circuit opened
    pub open_count: u32,

    /// Number of times circuit closed
    pub close_count: u32,

    /// Average request latency (milliseconds)
    pub avg_latency_ms: f64,

    /// p95 latency (milliseconds)
    pub p95_latency_ms: f64,

    /// p99 latency (milliseconds)
    pub p99_latency_ms: f64,

    /// Total time spent in each state (seconds)
    pub time_in_closed_secs: f64,
    pub time_in_open_secs: f64,
    pub time_in_half_open_secs: f64,

    /// Timestamp of last state change
    pub last_state_change: chrono::DateTime<chrono::Utc>,
}
```

### Prometheus Metrics

Circuit breakers automatically export Prometheus metrics:

```
# Circuit breaker state (0=closed, 1=open, 2=half-open)
circuit_breaker_state{name="sentinel"} 0

# Total requests
circuit_breaker_requests_total{name="sentinel"} 1000

# Successful requests
circuit_breaker_requests_successful{name="sentinel"} 950

# Failed requests
circuit_breaker_requests_failed{name="sentinel"} 50

# Rejected requests (circuit open)
circuit_breaker_requests_rejected{name="sentinel"} 0

# Error rate (0.0 - 1.0)
circuit_breaker_error_rate{name="sentinel"} 0.05

# Number of times circuit opened
circuit_breaker_open_count{name="sentinel"} 0

# Request duration histogram
circuit_breaker_request_duration_seconds_bucket{name="sentinel",le="0.1"} 800
circuit_breaker_request_duration_seconds_bucket{name="sentinel",le="0.5"} 950
circuit_breaker_request_duration_seconds_bucket{name="sentinel",le="1.0"} 1000

# Time in each state (seconds)
circuit_breaker_time_in_state_seconds{name="sentinel",state="closed"} 3600
circuit_breaker_time_in_state_seconds{name="sentinel",state="open"} 0
circuit_breaker_time_in_state_seconds{name="sentinel",state="half_open"} 0
```

---

## Usage Examples

### Example 1: Basic Usage

```rust
use llm_incident_manager::circuit_breaker::CircuitBreaker;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create circuit breaker
    let breaker = CircuitBreaker::new("my-service")
        .failure_threshold(5)
        .timeout(Duration::from_secs(60))
        .build();

    // Execute request through circuit breaker
    let result = breaker.call(|| async {
        external_api_call().await
    }).await?;

    println!("Result: {:?}", result);
    Ok(())
}
```

### Example 2: Error Handling

```rust
use llm_incident_manager::circuit_breaker::{CircuitBreaker, CircuitBreakerError};

async fn fetch_with_fallback(breaker: &CircuitBreaker) -> Result<Data, AppError> {
    match breaker.call(|| async {
        api_client.fetch_data().await
    }).await {
        Ok(data) => Ok(data),
        Err(CircuitBreakerError::Open { retry_after_secs, .. }) => {
            warn!("Circuit open, using cache (retry after {}s)", retry_after_secs);
            cache.get_data().ok_or(AppError::NoData)
        }
        Err(CircuitBreakerError::Timeout { .. }) => {
            warn!("Request timed out, using stale data");
            cache.get_stale_data().ok_or(AppError::NoData)
        }
        Err(CircuitBreakerError::OperationFailed(e)) => {
            error!("Operation failed: {}", e);
            Err(AppError::from(e))
        }
        Err(e) => Err(AppError::from(e)),
    }
}
```

### Example 3: Monitoring

```rust
use llm_incident_manager::circuit_breaker::CircuitBreaker;

async fn monitor_circuit_breaker(breaker: &CircuitBreaker) {
    // Get current state
    let state = breaker.state().await;
    println!("Current state: {:?}", state);

    // Get detailed info
    let info = breaker.info().await;
    println!("Failures: {} / {}", info.failure_count, info.total_requests);
    println!("Error rate: {:.2}%", info.error_rate * 100.0);

    // Get metrics
    let metrics = breaker.get_metrics().await;
    println!("Total requests: {}", metrics.total_requests);
    println!("Success rate: {:.2}%", metrics.success_rate * 100.0);
    println!("Average latency: {:.2}ms", metrics.avg_latency_ms);

    // Health check
    let health = breaker.health_check().await;
    println!("Health: {:?}", health);
}
```

### Example 4: Registry Management

```rust
use llm_incident_manager::circuit_breaker::{CircuitBreakerRegistry, CircuitBreaker};

async fn setup_registry() -> CircuitBreakerRegistry {
    let mut registry = CircuitBreakerRegistry::new();

    // Register multiple breakers
    registry.register("sentinel", CircuitBreaker::new("sentinel").build()).unwrap();
    registry.register("shield", CircuitBreaker::new("shield").build()).unwrap();
    registry.register("edge-agent", CircuitBreaker::new("edge-agent").build()).unwrap();

    // Check health of all breakers
    let health = registry.health_check_all().await;
    for (name, status) in health {
        println!("{}: {:?}", name, status);
    }

    registry
}

async fn use_registry(registry: &CircuitBreakerRegistry) {
    if let Some(breaker) = registry.get("sentinel") {
        let result = breaker.call(|| async {
            sentinel_api_call().await
        }).await;
    }
}
```

---

## See Also

- [Circuit Breaker Guide](./CIRCUIT_BREAKER_GUIDE.md) - Comprehensive implementation guide
- [Integration Guide](./CIRCUIT_BREAKER_INTEGRATION_GUIDE.md) - Service integration examples
- [Operations Guide](./CIRCUIT_BREAKER_OPERATIONS_GUIDE.md) - Operations and monitoring

---

**Document Version**: 1.0.0
**Last Updated**: 2025-11-13
**Status**: Production Ready
