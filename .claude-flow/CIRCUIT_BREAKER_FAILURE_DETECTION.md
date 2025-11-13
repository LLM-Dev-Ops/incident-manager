# Circuit Breaker Failure Detection Strategies

## Overview

This document defines comprehensive failure detection strategies for the circuit breaker system. Accurate failure detection is critical for making intelligent decisions about when to open, close, or test a circuit.

## Failure Detection Strategies

### 1. Consecutive Failure Counting

The simplest and most common strategy: count failures in a row.

#### Implementation

```rust
pub struct ConsecutiveFailureDetector {
    threshold: u32,
    count: AtomicU32,
}

impl ConsecutiveFailureDetector {
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold,
            count: AtomicU32::new(0),
        }
    }

    pub fn record_success(&self) {
        self.count.store(0, Ordering::Release);
    }

    pub fn record_failure(&self) -> bool {
        let count = self.count.fetch_add(1, Ordering::AcqRel) + 1;
        count >= self.threshold
    }

    pub fn should_trip(&self) -> bool {
        self.count.load(Ordering::Acquire) >= self.threshold
    }

    pub fn reset(&self) {
        self.count.store(0, Ordering::Release);
    }
}
```

#### Configuration

```rust
pub struct ConsecutiveFailureConfig {
    /// Number of consecutive failures before opening (default: 5)
    pub threshold: u32,
}
```

#### Use Cases

- **Best for**: Services with binary success/failure
- **Pros**: Simple, predictable, low overhead
- **Cons**: Doesn't account for success/failure ratio

#### Example

```
Request sequence: S S F F F F F
                       ↑ threshold=5
                       Opens here
```

### 2. Failure Rate Percentage (Sliding Window)

Track failure rate over a sliding window of requests or time.

#### Implementation - Count-Based Window

```rust
use std::collections::VecDeque;

pub struct CountBasedWindow {
    window_size: usize,
    requests: VecDeque<RequestOutcome>,
    failure_threshold: f64,
    minimum_requests: usize,
}

#[derive(Debug, Clone, Copy)]
struct RequestOutcome {
    success: bool,
    timestamp: Instant,
}

impl CountBasedWindow {
    pub fn new(window_size: usize, failure_threshold: f64, minimum_requests: usize) -> Self {
        Self {
            window_size,
            requests: VecDeque::with_capacity(window_size),
            failure_threshold,
            minimum_requests,
        }
    }

    pub fn record_request(&mut self, success: bool) -> bool {
        // Add new request
        self.requests.push_back(RequestOutcome {
            success,
            timestamp: Instant::now(),
        });

        // Maintain window size
        while self.requests.len() > self.window_size {
            self.requests.pop_front();
        }

        self.should_trip()
    }

    fn should_trip(&self) -> bool {
        // Need minimum requests for statistical significance
        if self.requests.len() < self.minimum_requests {
            return false;
        }

        let failures = self.requests.iter().filter(|r| !r.success).count();
        let failure_rate = failures as f64 / self.requests.len() as f64;

        failure_rate >= self.failure_threshold
    }

    pub fn failure_rate(&self) -> f64 {
        if self.requests.is_empty() {
            return 0.0;
        }

        let failures = self.requests.iter().filter(|r| !r.success).count();
        failures as f64 / self.requests.len() as f64
    }

    pub fn reset(&mut self) {
        self.requests.clear();
    }
}
```

#### Implementation - Time-Based Window

```rust
use std::time::Duration;

pub struct TimeBasedWindow {
    window_duration: Duration,
    requests: VecDeque<RequestOutcome>,
    failure_threshold: f64,
    minimum_requests: usize,
}

impl TimeBasedWindow {
    pub fn new(
        window_duration: Duration,
        failure_threshold: f64,
        minimum_requests: usize,
    ) -> Self {
        Self {
            window_duration,
            requests: VecDeque::new(),
            failure_threshold,
            minimum_requests,
        }
    }

    pub fn record_request(&mut self, success: bool) -> bool {
        let now = Instant::now();

        // Add new request
        self.requests.push_back(RequestOutcome {
            success,
            timestamp: now,
        });

        // Remove expired requests
        let cutoff = now - self.window_duration;
        while let Some(first) = self.requests.front() {
            if first.timestamp < cutoff {
                self.requests.pop_front();
            } else {
                break;
            }
        }

        self.should_trip()
    }

    fn should_trip(&self) -> bool {
        if self.requests.len() < self.minimum_requests {
            return false;
        }

        let failures = self.requests.iter().filter(|r| !r.success).count();
        let failure_rate = failures as f64 / self.requests.len() as f64;

        failure_rate >= self.failure_threshold
    }

    pub fn failure_rate(&self) -> f64 {
        if self.requests.is_empty() {
            return 0.0;
        }

        let failures = self.requests.iter().filter(|r| !r.success).count();
        failures as f64 / self.requests.len() as f64
    }

    pub fn reset(&mut self) {
        self.requests.clear();
    }
}
```

#### Configuration

```rust
pub struct FailureRateConfig {
    /// Failure rate threshold 0.0-1.0 (default: 0.5)
    pub threshold: f64,

    /// Minimum requests before evaluating (default: 10)
    pub minimum_requests: usize,

    /// Window type and size
    pub window: WindowConfig,
}

pub enum WindowConfig {
    /// Last N requests
    CountBased { size: usize },

    /// Requests in last N seconds
    TimeBased { duration: Duration },
}
```

#### Use Cases

- **Best for**: High-traffic services with variable load
- **Pros**: Statistical significance, handles bursts
- **Cons**: More complex, requires tuning

#### Example

```
Time-based window (60s):
[-------- 60s --------]
S S F S F F S F F F  ← Current
        ↑ 50% failure rate → Opens
```

### 3. Timeout-Based Failures

Classify slow responses as failures.

#### Implementation

```rust
pub struct TimeoutDetector {
    timeout_threshold: Duration,
    treat_as_failure: bool,
}

impl TimeoutDetector {
    pub fn new(timeout_threshold: Duration, treat_as_failure: bool) -> Self {
        Self {
            timeout_threshold,
            treat_as_failure,
        }
    }

    pub async fn execute_with_timeout<F, T, E>(
        &self,
        operation: F,
    ) -> Result<T, TimeoutError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        match tokio::time::timeout(self.timeout_threshold, operation).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(TimeoutError::OperationFailed(e)),
            Err(_) => Err(TimeoutError::Timeout),
        }
    }

    pub fn is_failure(&self, error: &TimeoutError<impl std::error::Error>) -> bool {
        match error {
            TimeoutError::Timeout => self.treat_as_failure,
            TimeoutError::OperationFailed(_) => true,
        }
    }
}

pub enum TimeoutError<E> {
    Timeout,
    OperationFailed(E),
}
```

#### Configuration

```rust
pub struct TimeoutConfig {
    /// Request timeout duration (default: 30s)
    pub timeout: Duration,

    /// Treat timeouts as failures (default: true)
    pub count_as_failure: bool,

    /// Timeout threshold for "slow call" detection (default: 5s)
    pub slow_call_threshold: Duration,
}
```

### 4. Slow Call Detection (Latency-Based)

Detect degraded service performance by tracking slow responses.

#### Implementation

```rust
pub struct SlowCallDetector {
    slow_threshold: Duration,
    slow_call_rate_threshold: f64,
    minimum_requests: usize,
    window: TimeBasedWindow,
}

impl SlowCallDetector {
    pub fn new(
        slow_threshold: Duration,
        slow_call_rate_threshold: f64,
        window_duration: Duration,
        minimum_requests: usize,
    ) -> Self {
        Self {
            slow_threshold,
            slow_call_rate_threshold,
            minimum_requests,
            window: TimeBasedWindow::new(window_duration),
        }
    }

    pub fn record_request(&mut self, duration: Duration) -> bool {
        let is_slow = duration >= self.slow_threshold;
        self.window.record(is_slow);

        self.should_trip()
    }

    fn should_trip(&self) -> bool {
        if self.window.total() < self.minimum_requests {
            return false;
        }

        let slow_rate = self.window.slow_count() as f64 / self.window.total() as f64;
        slow_rate >= self.slow_call_rate_threshold
    }

    pub fn slow_call_rate(&self) -> f64 {
        if self.window.total() == 0 {
            return 0.0;
        }

        self.window.slow_count() as f64 / self.window.total() as f64
    }
}

struct SlowCallWindow {
    duration: Duration,
    calls: VecDeque<SlowCallRecord>,
}

struct SlowCallRecord {
    timestamp: Instant,
    is_slow: bool,
}

impl SlowCallWindow {
    fn record(&mut self, is_slow: bool) {
        let now = Instant::now();
        self.calls.push_back(SlowCallRecord {
            timestamp: now,
            is_slow,
        });

        // Remove expired records
        let cutoff = now - self.duration;
        while let Some(first) = self.calls.front() {
            if first.timestamp < cutoff {
                self.calls.pop_front();
            } else {
                break;
            }
        }
    }

    fn total(&self) -> usize {
        self.calls.len()
    }

    fn slow_count(&self) -> usize {
        self.calls.iter().filter(|r| r.is_slow).count()
    }
}
```

#### Configuration

```rust
pub struct SlowCallConfig {
    /// Duration threshold for "slow" calls (default: 5s)
    pub slow_call_threshold: Duration,

    /// Slow call rate threshold 0.0-1.0 (default: 0.5)
    pub slow_call_rate_threshold: f64,

    /// Minimum requests before evaluating (default: 10)
    pub minimum_requests: usize,

    /// Window duration (default: 60s)
    pub window_duration: Duration,
}
```

#### Use Cases

- **Best for**: Detecting gradual performance degradation
- **Pros**: Catches degraded performance before total failure
- **Cons**: Requires careful threshold tuning

### 5. Custom Failure Predicates

Allow custom logic to determine if a response is a failure.

#### Implementation

```rust
pub type FailurePredicate<T, E> = Arc<dyn Fn(&Result<T, E>) -> bool + Send + Sync>;

pub struct CustomFailureDetector<T, E> {
    predicate: FailurePredicate<T, E>,
}

impl<T, E> CustomFailureDetector<T, E> {
    pub fn new(predicate: FailurePredicate<T, E>) -> Self {
        Self { predicate }
    }

    pub fn is_failure(&self, result: &Result<T, E>) -> bool {
        (self.predicate)(result)
    }
}

// Example: HTTP status code based failure detection
impl CustomFailureDetector<reqwest::Response, reqwest::Error> {
    pub fn http_status_predicate() -> FailurePredicate<reqwest::Response, reqwest::Error> {
        Arc::new(|result| match result {
            Ok(response) => {
                let status = response.status();
                // Treat 5xx as failure, but not 4xx (client errors)
                status.is_server_error()
            }
            Err(_) => true,
        })
    }

    pub fn custom_error_predicate<F>(f: F) -> FailurePredicate<reqwest::Response, reqwest::Error>
    where
        F: Fn(&reqwest::Error) -> bool + Send + Sync + 'static,
    {
        Arc::new(move |result| match result {
            Ok(_) => false,
            Err(e) => f(e),
        })
    }
}
```

#### Configuration

```rust
pub struct CustomPredicateConfig<T, E> {
    /// Custom predicate function
    pub predicate: FailurePredicate<T, E>,

    /// Optional: Combine with other detectors
    pub combine_with: Option<CombineStrategy>,
}

pub enum CombineStrategy {
    /// Custom OR other detectors
    Or,
    /// Custom AND other detectors
    And,
    /// Custom predicate only
    Only,
}
```

#### Use Cases

- **Best for**: Domain-specific failure criteria
- **Pros**: Maximum flexibility
- **Cons**: Requires custom implementation

#### Examples

```rust
// Example 1: HTTP status codes
let predicate = Arc::new(|result: &Result<Response, reqwest::Error>| {
    match result {
        Ok(response) => response.status().is_server_error(),
        Err(_) => true,
    }
});

// Example 2: Business logic errors
let predicate = Arc::new(|result: &Result<ApiResponse, ApiError>| {
    match result {
        Ok(response) => response.error_code.is_some(),
        Err(e) => !e.is_retryable(),
    }
});

// Example 3: Specific error types
let predicate = Arc::new(|result: &Result<Data, MyError>| {
    match result {
        Ok(_) => false,
        Err(MyError::Timeout) => true,
        Err(MyError::ConnectionRefused) => true,
        Err(MyError::InvalidInput(_)) => false, // Not a circuit breaker failure
        _ => true,
    }
});
```

## Composite Detection Strategy

Combine multiple strategies for comprehensive failure detection.

### Implementation

```rust
pub struct CompositeFailureDetector {
    consecutive_detector: Option<ConsecutiveFailureDetector>,
    failure_rate_detector: Option<FailureRateDetector>,
    slow_call_detector: Option<SlowCallDetector>,
    strategy: DetectionStrategy,
}

pub enum DetectionStrategy {
    /// Trip if ANY detector triggers
    Any,
    /// Trip if ALL detectors trigger
    All,
    /// Trip based on weighted voting
    Weighted { weights: Vec<f64> },
}

impl CompositeFailureDetector {
    pub fn new(strategy: DetectionStrategy) -> Self {
        Self {
            consecutive_detector: None,
            failure_rate_detector: None,
            slow_call_detector: None,
            strategy,
        }
    }

    pub fn with_consecutive_failures(mut self, threshold: u32) -> Self {
        self.consecutive_detector = Some(ConsecutiveFailureDetector::new(threshold));
        self
    }

    pub fn with_failure_rate(
        mut self,
        threshold: f64,
        window_size: usize,
        min_requests: usize,
    ) -> Self {
        self.failure_rate_detector = Some(FailureRateDetector::new(
            threshold,
            window_size,
            min_requests,
        ));
        self
    }

    pub fn with_slow_call_detection(
        mut self,
        slow_threshold: Duration,
        rate_threshold: f64,
        window_duration: Duration,
        min_requests: usize,
    ) -> Self {
        self.slow_call_detector = Some(SlowCallDetector::new(
            slow_threshold,
            rate_threshold,
            window_duration,
            min_requests,
        ));
        self
    }

    pub fn should_trip(
        &self,
        success: bool,
        duration: Duration,
    ) -> bool {
        let mut signals = Vec::new();

        // Check consecutive failures
        if let Some(detector) = &self.consecutive_detector {
            signals.push(if success {
                detector.record_success();
                false
            } else {
                detector.record_failure()
            });
        }

        // Check failure rate
        if let Some(detector) = &self.failure_rate_detector {
            signals.push(detector.record_request(success));
        }

        // Check slow calls
        if let Some(detector) = &self.slow_call_detector {
            signals.push(detector.record_request(duration));
        }

        // Evaluate strategy
        match self.strategy {
            DetectionStrategy::Any => signals.iter().any(|&s| s),
            DetectionStrategy::All => signals.iter().all(|&s| s),
            DetectionStrategy::Weighted { ref weights } => {
                let total: f64 = signals
                    .iter()
                    .zip(weights.iter())
                    .filter(|(&signal, _)| signal)
                    .map(|(_, &weight)| weight)
                    .sum();

                total >= 1.0
            }
        }
    }
}
```

### Configuration

```rust
pub struct CompositeDetectionConfig {
    /// Enable consecutive failure detection
    pub consecutive: Option<ConsecutiveFailureConfig>,

    /// Enable failure rate detection
    pub failure_rate: Option<FailureRateConfig>,

    /// Enable slow call detection
    pub slow_call: Option<SlowCallConfig>,

    /// Detection strategy
    pub strategy: DetectionStrategy,
}

impl Default for CompositeDetectionConfig {
    fn default() -> Self {
        Self {
            consecutive: Some(ConsecutiveFailureConfig { threshold: 5 }),
            failure_rate: Some(FailureRateConfig {
                threshold: 0.5,
                minimum_requests: 10,
                window: WindowConfig::TimeBased {
                    duration: Duration::from_secs(60),
                },
            }),
            slow_call: Some(SlowCallConfig {
                slow_call_threshold: Duration::from_secs(5),
                slow_call_rate_threshold: 0.5,
                minimum_requests: 10,
                window_duration: Duration::from_secs(60),
            }),
            strategy: DetectionStrategy::Any,
        }
    }
}
```

## Error Classification

### Retryable vs Non-Retryable Errors

```rust
pub trait ErrorClassifier: Send + Sync {
    type Error: std::error::Error;

    /// Determine if an error should be counted as a failure
    fn is_failure(&self, error: &Self::Error) -> bool;

    /// Determine if the error is retryable
    fn is_retryable(&self, error: &Self::Error) -> bool;

    /// Determine if the error indicates a service issue
    fn is_service_error(&self, error: &Self::Error) -> bool;
}

// HTTP error classifier
pub struct HttpErrorClassifier;

impl ErrorClassifier for HttpErrorClassifier {
    type Error = reqwest::Error;

    fn is_failure(&self, error: &Self::Error) -> bool {
        // Network errors are failures
        if error.is_connect() || error.is_timeout() {
            return true;
        }

        // 5xx status codes are failures
        if let Some(status) = error.status() {
            return status.is_server_error();
        }

        true
    }

    fn is_retryable(&self, error: &Self::Error) -> bool {
        // Connection errors are retryable
        if error.is_connect() || error.is_timeout() {
            return true;
        }

        // 503 Service Unavailable is retryable
        if let Some(status) = error.status() {
            return status == reqwest::StatusCode::SERVICE_UNAVAILABLE
                || status == reqwest::StatusCode::TOO_MANY_REQUESTS;
        }

        false
    }

    fn is_service_error(&self, error: &Self::Error) -> bool {
        // 5xx errors indicate service issues
        if let Some(status) = error.status() {
            return status.is_server_error();
        }

        // Connection errors indicate service issues
        error.is_connect()
    }
}
```

## Performance Optimization

### Lock-Free Failure Tracking

```rust
use std::sync::atomic::{AtomicU64, Ordering};

pub struct LockFreeFailureTracker {
    // Packed: upper 32 bits = successes, lower 32 bits = failures
    counters: AtomicU64,
    threshold: u32,
}

impl LockFreeFailureTracker {
    pub fn new(threshold: u32) -> Self {
        Self {
            counters: AtomicU64::new(0),
            threshold,
        }
    }

    pub fn record_success(&self) {
        // Increment upper 32 bits, reset lower 32 bits
        loop {
            let current = self.counters.load(Ordering::Acquire);
            let successes = (current >> 32) as u32;
            let new_value = ((successes.wrapping_add(1) as u64) << 32) | 0;

            if self
                .counters
                .compare_exchange_weak(current, new_value, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }

    pub fn record_failure(&self) -> bool {
        // Increment lower 32 bits
        let current = self.counters.fetch_add(1, Ordering::AcqRel);
        let failures = (current & 0xFFFFFFFF) as u32 + 1;

        failures >= self.threshold
    }

    pub fn get_counts(&self) -> (u32, u32) {
        let value = self.counters.load(Ordering::Acquire);
        let successes = (value >> 32) as u32;
        let failures = (value & 0xFFFFFFFF) as u32;
        (successes, failures)
    }
}
```

## Testing Strategies

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consecutive_failures() {
        let detector = ConsecutiveFailureDetector::new(3);

        assert!(!detector.record_failure()); // 1
        assert!(!detector.record_failure()); // 2
        assert!(detector.record_failure());  // 3 - should trip

        detector.record_success();
        assert!(!detector.should_trip());
    }

    #[test]
    fn test_failure_rate_window() {
        let mut detector = CountBasedWindow::new(10, 0.5, 5);

        // Add 10 requests: 3 failures, 7 successes = 30% failure rate
        for _ in 0..7 {
            assert!(!detector.record_request(true));
        }
        for _ in 0..3 {
            assert!(!detector.record_request(false)); // 30% < 50%
        }

        // Add 3 more failures: 6 failures, 10 total = 60% failure rate
        for _ in 0..3 {
            detector.record_request(false);
        }

        assert!(detector.should_trip()); // 60% > 50%
    }

    #[tokio::test]
    async fn test_timeout_detection() {
        let detector = TimeoutDetector::new(Duration::from_millis(100), true);

        // Fast operation succeeds
        let result = detector
            .execute_with_timeout(async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<_, String>(())
            })
            .await;

        assert!(result.is_ok());

        // Slow operation times out
        let result = detector
            .execute_with_timeout(async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok::<_, String>(())
            })
            .await;

        assert!(matches!(result, Err(TimeoutError::Timeout)));
    }
}
```

## Related Documents

- [State Machine Design](./CIRCUIT_BREAKER_STATE_MACHINE.md)
- [Core Architecture](./CIRCUIT_BREAKER_CORE_ARCHITECTURE.md)
- [Configuration Schema](./CIRCUIT_BREAKER_CONFIGURATION_SCHEMA.md)
- [Testing Strategy](./CIRCUIT_BREAKER_TESTING_STRATEGY.md)
