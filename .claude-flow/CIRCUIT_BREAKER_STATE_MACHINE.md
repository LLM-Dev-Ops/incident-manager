# Circuit Breaker State Machine Design

## Overview

This document specifies the enterprise-grade circuit breaker state machine for the LLM Incident Manager. The circuit breaker pattern prevents cascading failures in distributed systems by detecting failures and preventing requests to unhealthy services.

## State Machine Architecture

### States

```
┌─────────────────────────────────────────────────────────────────┐
│                    Circuit Breaker States                        │
└─────────────────────────────────────────────────────────────────┘

┌──────────────┐
│    CLOSED    │  ◄─── Normal operation
│              │       All requests pass through
│  ✓ Healthy   │       Success/failure counted
└──────┬───────┘
       │
       │ Failure threshold exceeded
       ▼
┌──────────────┐
│     OPEN     │  ◄─── Circuit tripped
│              │       All requests fail fast
│  ✗ Unhealthy │       Waiting for timeout
└──────┬───────┘
       │
       │ Timeout elapsed
       ▼
┌──────────────┐
│  HALF-OPEN   │  ◄─── Recovery testing
│              │       Limited requests allowed
│  ? Testing   │       Success/failure monitored
└──────┬───────┘
       │
       ├─────────► Success threshold met ───┐
       │                                     │
       └─────────► Failure detected         │
                         │                  │
                         ▼                  │
                   ┌──────────┐            │
                   │   OPEN   │            │
                   └──────────┘            │
                                            │
                                            ▼
                                      ┌──────────┐
                                      │  CLOSED  │
                                      └──────────┘
```

### State Definitions

#### 1. CLOSED State
- **Purpose**: Normal operation
- **Behavior**:
  - All requests pass through to the protected service
  - Track success and failure counts
  - Monitor failure rate in sliding window
- **Transition Out**:
  - To OPEN: When failure threshold is exceeded
- **Metrics**:
  - Request count
  - Success count
  - Failure count
  - Failure rate percentage

#### 2. OPEN State
- **Purpose**: Prevent requests to failing service
- **Behavior**:
  - Immediately fail all requests without calling service
  - Return cached response or fallback value
  - Start timeout timer
  - Log circuit open events
- **Transition Out**:
  - To HALF-OPEN: When timeout duration elapses
- **Metrics**:
  - Time in open state
  - Rejected request count
  - Fallback invocation count

#### 3. HALF-OPEN State
- **Purpose**: Test if service has recovered
- **Behavior**:
  - Allow limited number of trial requests (probe requests)
  - Track success/failure of trial requests
  - Block additional requests beyond probe limit
- **Transition Out**:
  - To CLOSED: When consecutive success threshold met
  - To OPEN: When any failure occurs (configurable)
- **Metrics**:
  - Probe request count
  - Probe success count
  - Probe failure count

## State Transition Conditions

### CLOSED → OPEN

**Consecutive Failure Threshold**:
```rust
if consecutive_failures >= config.consecutive_failure_threshold {
    transition_to_open()
}
```

**Failure Rate Threshold (Sliding Window)**:
```rust
let failure_rate = failures_in_window / total_requests_in_window;
if failure_rate >= config.failure_rate_threshold
   && total_requests_in_window >= config.minimum_requests {
    transition_to_open()
}
```

**Slow Call Threshold**:
```rust
let slow_call_rate = slow_calls_in_window / total_requests_in_window;
if slow_call_rate >= config.slow_call_rate_threshold
   && total_requests_in_window >= config.minimum_requests {
    transition_to_open()
}
```

### OPEN → HALF-OPEN

**Time-Based**:
```rust
if time_since_opened >= config.open_timeout {
    transition_to_half_open()
}
```

**Exponential Backoff** (optional):
```rust
let backoff_time = config.initial_open_timeout * config.backoff_multiplier.powi(attempt);
if time_since_opened >= backoff_time {
    transition_to_half_open()
}
```

### HALF-OPEN → CLOSED

**Consecutive Success**:
```rust
if consecutive_successes >= config.half_open_success_threshold {
    transition_to_closed()
    reset_counters()
}
```

**Success Rate**:
```rust
let success_rate = successes / total_probes;
if success_rate >= config.half_open_success_rate
   && total_probes >= config.half_open_minimum_probes {
    transition_to_closed()
    reset_counters()
}
```

### HALF-OPEN → OPEN

**Immediate Failure** (strict mode):
```rust
if any_failure && config.half_open_strict_mode {
    transition_to_open()
    increment_attempt_counter()
}
```

**Failure Threshold**:
```rust
if failures >= config.half_open_failure_threshold {
    transition_to_open()
    increment_attempt_counter()
}
```

## Configuration Parameters

### Core Thresholds

```rust
pub struct CircuitBreakerConfig {
    // CLOSED → OPEN conditions
    /// Consecutive failures before opening (default: 5)
    pub consecutive_failure_threshold: u32,

    /// Failure rate threshold 0.0-1.0 (default: 0.5 = 50%)
    pub failure_rate_threshold: f64,

    /// Slow call rate threshold 0.0-1.0 (default: 0.5 = 50%)
    pub slow_call_rate_threshold: f64,

    /// Slow call duration threshold (default: 5s)
    pub slow_call_duration_threshold: Duration,

    /// Minimum requests before evaluating rate (default: 10)
    pub minimum_requests: u32,

    /// Sliding window size in seconds (default: 60)
    pub sliding_window_size: Duration,

    /// Window type: Count-based or Time-based
    pub window_type: WindowType,

    // OPEN → HALF-OPEN conditions
    /// Duration to wait in OPEN before HALF-OPEN (default: 30s)
    pub open_timeout: Duration,

    /// Enable exponential backoff for open timeout
    pub enable_exponential_backoff: bool,

    /// Backoff multiplier (default: 2.0)
    pub backoff_multiplier: f64,

    /// Maximum backoff duration (default: 5 minutes)
    pub max_backoff_duration: Duration,

    // HALF-OPEN → CLOSED/OPEN conditions
    /// Consecutive successes to close circuit (default: 3)
    pub half_open_success_threshold: u32,

    /// Maximum concurrent probe requests (default: 3)
    pub half_open_max_concurrent: u32,

    /// Failure threshold in half-open (default: 1)
    pub half_open_failure_threshold: u32,

    /// Strict mode: any failure returns to OPEN
    pub half_open_strict_mode: bool,

    /// Success rate required to close (default: 1.0 = 100%)
    pub half_open_success_rate: f64,

    /// Minimum probes before evaluating success rate (default: 3)
    pub half_open_minimum_probes: u32,
}
```

### Window Types

```rust
pub enum WindowType {
    /// Count-based sliding window (last N requests)
    CountBased {
        /// Number of requests to track
        size: usize,
    },

    /// Time-based sliding window (requests in last N seconds)
    TimeBased {
        /// Duration of the window
        duration: Duration,
    },
}
```

## State Management

### Thread-Safe State

```rust
use std::sync::Arc;
use parking_lot::RwLock;

pub struct CircuitBreakerState {
    /// Current state
    state: CircuitState,

    /// State entered at timestamp
    state_entered_at: Instant,

    /// Counters for current window
    counters: WindowCounters,

    /// Number of state transitions (CLOSED→OPEN)
    trip_count: u64,

    /// Current backoff attempt number
    backoff_attempt: u32,

    /// Last state transition time
    last_transition_at: Instant,
}

impl CircuitBreakerState {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            state: CircuitState::Closed,
            state_entered_at: Instant::now(),
            counters: WindowCounters::new(),
            trip_count: 0,
            backoff_attempt: 0,
            last_transition_at: Instant::now(),
        }))
    }
}
```

### Window Counters

```rust
pub struct WindowCounters {
    /// Total requests in window
    total: u64,

    /// Successful requests
    successes: u64,

    /// Failed requests
    failures: u64,

    /// Consecutive failures (resets on success)
    consecutive_failures: u32,

    /// Consecutive successes (resets on failure)
    consecutive_successes: u32,

    /// Slow calls (above threshold)
    slow_calls: u64,

    /// Time-series data for time-based window
    request_history: VecDeque<RequestRecord>,
}

pub struct RequestRecord {
    timestamp: Instant,
    success: bool,
    duration: Duration,
}
```

## State Transition Events

### Event Types

```rust
pub enum CircuitBreakerEvent {
    /// Circuit transitioned to a new state
    StateTransition {
        from: CircuitState,
        to: CircuitState,
        reason: TransitionReason,
        timestamp: Instant,
    },

    /// Request was executed
    RequestExecuted {
        state: CircuitState,
        success: bool,
        duration: Duration,
        timestamp: Instant,
    },

    /// Request was rejected (circuit open)
    RequestRejected {
        reason: String,
        timestamp: Instant,
    },

    /// Fallback was invoked
    FallbackExecuted {
        reason: String,
        timestamp: Instant,
    },

    /// Circuit was reset manually
    ManualReset {
        timestamp: Instant,
    },
}

pub enum TransitionReason {
    ConsecutiveFailures { count: u32 },
    FailureRateExceeded { rate: f64 },
    SlowCallRateExceeded { rate: f64 },
    TimeoutElapsed,
    SuccessThresholdMet { count: u32 },
    FailureInHalfOpen,
    ManualOverride,
}
```

### Event Callbacks

```rust
pub type EventCallback = Arc<dyn Fn(CircuitBreakerEvent) + Send + Sync>;

pub struct CircuitBreaker {
    // ... other fields ...

    /// Event callbacks for observability
    event_callbacks: Vec<EventCallback>,
}

impl CircuitBreaker {
    /// Register an event callback
    pub fn on_event<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(CircuitBreakerEvent) + Send + Sync + 'static,
    {
        self.event_callbacks.push(Arc::new(callback));
        self
    }

    /// Emit an event to all registered callbacks
    fn emit_event(&self, event: CircuitBreakerEvent) {
        for callback in &self.event_callbacks {
            callback(event.clone());
        }
    }
}
```

## Default Configurations

### Conservative (Production Default)

```rust
impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            // Conservative thresholds
            consecutive_failure_threshold: 5,
            failure_rate_threshold: 0.50,
            slow_call_rate_threshold: 0.50,
            slow_call_duration_threshold: Duration::from_secs(5),
            minimum_requests: 10,
            sliding_window_size: Duration::from_secs(60),
            window_type: WindowType::TimeBased {
                duration: Duration::from_secs(60),
            },

            // Moderate recovery time
            open_timeout: Duration::from_secs(30),
            enable_exponential_backoff: true,
            backoff_multiplier: 2.0,
            max_backoff_duration: Duration::from_secs(300),

            // Strict half-open recovery
            half_open_success_threshold: 3,
            half_open_max_concurrent: 3,
            half_open_failure_threshold: 1,
            half_open_strict_mode: false,
            half_open_success_rate: 0.80,
            half_open_minimum_probes: 3,
        }
    }
}
```

### Aggressive (Fast Fail)

```rust
pub fn aggressive_config() -> CircuitBreakerConfig {
    CircuitBreakerConfig {
        consecutive_failure_threshold: 3,
        failure_rate_threshold: 0.30,
        slow_call_rate_threshold: 0.30,
        slow_call_duration_threshold: Duration::from_secs(2),
        minimum_requests: 5,
        open_timeout: Duration::from_secs(10),
        half_open_success_threshold: 5,
        half_open_strict_mode: true,
        ..Default::default()
    }
}
```

### Lenient (Graceful Degradation)

```rust
pub fn lenient_config() -> CircuitBreakerConfig {
    CircuitBreakerConfig {
        consecutive_failure_threshold: 10,
        failure_rate_threshold: 0.70,
        slow_call_rate_threshold: 0.70,
        slow_call_duration_threshold: Duration::from_secs(10),
        minimum_requests: 20,
        open_timeout: Duration::from_secs(60),
        half_open_success_threshold: 2,
        half_open_strict_mode: false,
        half_open_success_rate: 0.60,
        ..Default::default()
    }
}
```

## Metrics Collection

### State Metrics

```rust
pub struct CircuitBreakerMetrics {
    /// Current state (0=closed, 1=open, 2=half-open)
    pub state: AtomicU8,

    /// Total requests processed
    pub requests_total: AtomicU64,

    /// Successful requests
    pub requests_successful: AtomicU64,

    /// Failed requests
    pub requests_failed: AtomicU64,

    /// Rejected requests (circuit open)
    pub requests_rejected: AtomicU64,

    /// Total state transitions
    pub state_transitions_total: AtomicU64,

    /// Time spent in each state (milliseconds)
    pub state_duration_ms: [AtomicU64; 3], // [closed, open, half-open]

    /// Fallback executions
    pub fallback_executions: AtomicU64,

    /// Current failure rate (percentage)
    pub failure_rate_percent: AtomicU64,

    /// Current slow call rate (percentage)
    pub slow_call_rate_percent: AtomicU64,
}
```

### Prometheus Metrics

```
# Circuit breaker state (0=closed, 1=open, 2=half-open)
circuit_breaker_state{name="sentinel_client"} 0

# Total requests by result
circuit_breaker_requests_total{name="sentinel_client",result="success"} 1000
circuit_breaker_requests_total{name="sentinel_client",result="failure"} 50
circuit_breaker_requests_total{name="sentinel_client",result="rejected"} 20

# State transitions
circuit_breaker_transitions_total{name="sentinel_client",from="closed",to="open"} 3
circuit_breaker_transitions_total{name="sentinel_client",from="open",to="half_open"} 3
circuit_breaker_transitions_total{name="sentinel_client",from="half_open",to="closed"} 2

# Time in each state (milliseconds)
circuit_breaker_state_duration_milliseconds{name="sentinel_client",state="closed"} 95000
circuit_breaker_state_duration_milliseconds{name="sentinel_client",state="open"} 3000
circuit_breaker_state_duration_milliseconds{name="sentinel_client",state="half_open"} 2000

# Rates
circuit_breaker_failure_rate{name="sentinel_client"} 0.05
circuit_breaker_slow_call_rate{name="sentinel_client"} 0.02
```

## State Persistence (Optional)

For critical services, circuit breaker state can be persisted:

```rust
pub trait StateStore: Send + Sync {
    async fn save_state(&self, name: &str, state: &CircuitBreakerState) -> Result<()>;
    async fn load_state(&self, name: &str) -> Result<Option<CircuitBreakerState>>;
    async fn delete_state(&self, name: &str) -> Result<()>;
}

// Redis implementation
pub struct RedisStateStore {
    client: redis::Client,
    ttl: Duration,
}

impl StateStore for RedisStateStore {
    async fn save_state(&self, name: &str, state: &CircuitBreakerState) -> Result<()> {
        let key = format!("circuit_breaker:state:{}", name);
        let serialized = bincode::serialize(state)?;

        let mut conn = self.client.get_async_connection().await?;
        conn.set_ex(&key, serialized, self.ttl.as_secs() as usize).await?;

        Ok(())
    }

    // ... other implementations
}
```

## Testing Considerations

### State Machine Test Cases

1. **Normal Operation**: CLOSED state with successful requests
2. **Failure Detection**: CLOSED → OPEN on threshold breach
3. **Fast Fail**: OPEN state rejecting requests
4. **Recovery Test**: OPEN → HALF-OPEN after timeout
5. **Successful Recovery**: HALF-OPEN → CLOSED after success threshold
6. **Failed Recovery**: HALF-OPEN → OPEN on failure
7. **Exponential Backoff**: Multiple OPEN→HALF-OPEN→OPEN cycles
8. **Concurrent Access**: Thread-safety under load
9. **Manual Reset**: Force state transitions
10. **Event Emission**: Verify callbacks are invoked

### Performance Benchmarks

- State transitions: < 100 nanoseconds
- Request execution overhead: < 1 microsecond
- Concurrent throughput: > 100,000 requests/second
- Memory overhead: < 1KB per circuit breaker instance

## Related Documents

- [Circuit Breaker Core Architecture](./CIRCUIT_BREAKER_CORE_ARCHITECTURE.md)
- [Failure Detection Strategies](./CIRCUIT_BREAKER_FAILURE_DETECTION.md)
- [Configuration Schema](./CIRCUIT_BREAKER_CONFIGURATION_SCHEMA.md)
- [Integration Patterns](./CIRCUIT_BREAKER_INTEGRATION_PATTERNS.md)
