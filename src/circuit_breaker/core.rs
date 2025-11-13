//! Core circuit breaker implementation with async support.

use crate::circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerResult, CircuitBreakerState,
    StateData, StateTransition,
};
use parking_lot::RwLock;
use std::future::Future;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// A thread-safe, async circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Unique name for this circuit breaker
    name: String,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Internal state
    state: Arc<RwLock<StateData>>,
    /// Number of active requests in half-open state
    half_open_requests: Arc<RwLock<u32>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let name = name.into();
        info!(
            name = %name,
            config = ?config,
            "Creating new circuit breaker"
        );

        Self {
            name,
            config,
            state: Arc::new(RwLock::new(StateData::new())),
            half_open_requests: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the name of this circuit breaker
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current state
    pub fn state(&self) -> CircuitBreakerState {
        self.state.read().state
    }

    /// Get the current configuration
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Execute an async operation protected by the circuit breaker
    pub async fn call<F, T, E>(&self, f: F) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        E: std::error::Error + Send + 'static,
    {
        // Check if we should allow the request
        self.check_and_update_state()?;

        // Track metrics
        super::metrics::CIRCUIT_BREAKER_METRICS
            .calls_total
            .with_label_values(&[&self.name, "allowed"])
            .inc();

        // Execute the operation
        let start = std::time::Instant::now();
        let result = f().await;
        let duration = start.elapsed();

        // Record metrics
        super::metrics::CIRCUIT_BREAKER_METRICS
            .call_duration
            .with_label_values(&[&self.name])
            .observe(duration.as_secs_f64());

        // Handle the result
        match result {
            Ok(value) => {
                self.on_success();
                super::metrics::CIRCUIT_BREAKER_METRICS
                    .successful_calls
                    .with_label_values(&[&self.name])
                    .inc();
                Ok(value)
            }
            Err(err) => {
                self.on_failure();
                super::metrics::CIRCUIT_BREAKER_METRICS
                    .failed_calls
                    .with_label_values(&[&self.name])
                    .inc();
                Err(CircuitBreakerError::OperationFailed(err.to_string()))
            }
        }
    }

    /// Execute an async operation with a fallback
    pub async fn call_with_fallback<F, FB, T, E>(
        &self,
        f: F,
        fallback: FB,
    ) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        FB: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = T> + Send>> + Send,
        E: std::error::Error + Send + 'static,
    {
        match self.call(f).await {
            Ok(value) => Ok(value),
            Err(CircuitBreakerError::Open(_)) => {
                debug!(
                    name = %self.name,
                    "Circuit breaker open, using fallback"
                );
                Ok(fallback().await)
            }
            Err(err) => Err(err),
        }
    }

    /// Check state and update if necessary
    fn check_and_update_state(&self) -> CircuitBreakerResult<()> {
        let mut state = self.state.write();

        // Check if we should transition from Open to HalfOpen
        if state.should_attempt_reset(self.config.timeout_duration) {
            let transition = state.transition_to(CircuitBreakerState::HalfOpen);
            self.log_transition(&transition);
            *self.half_open_requests.write() = 0;
        }

        // Check current state
        match state.state {
            CircuitBreakerState::Closed => Ok(()),
            CircuitBreakerState::Open => {
                super::metrics::CIRCUIT_BREAKER_METRICS
                    .rejected_calls
                    .with_label_values(&[&self.name])
                    .inc();
                Err(CircuitBreakerError::Open(self.name.clone()))
            }
            CircuitBreakerState::HalfOpen => {
                let mut half_open_count = self.half_open_requests.write();
                if *half_open_count >= self.config.half_open_max_requests {
                    super::metrics::CIRCUIT_BREAKER_METRICS
                        .rejected_calls
                        .with_label_values(&[&self.name])
                        .inc();
                    Err(CircuitBreakerError::Open(self.name.clone()))
                } else {
                    *half_open_count += 1;
                    Ok(())
                }
            }
        }
    }

    /// Handle successful operation
    fn on_success(&self) {
        let mut state = self.state.write();
        state.record_success();

        debug!(
            name = %self.name,
            current_state = %state.state,
            consecutive_successes = state.consecutive_successes,
            "Operation succeeded"
        );

        // Check if we should transition to Closed from HalfOpen
        if state.state == CircuitBreakerState::HalfOpen
            && state.consecutive_successes >= self.config.success_threshold
        {
            let transition = state.transition_to(CircuitBreakerState::Closed);
            self.log_transition(&transition);
            *self.half_open_requests.write() = 0;
        }
    }

    /// Handle failed operation
    fn on_failure(&self) {
        let mut state = self.state.write();
        state.record_failure();

        warn!(
            name = %self.name,
            current_state = %state.state,
            consecutive_failures = state.consecutive_failures,
            "Operation failed"
        );

        // Check if we should open the circuit
        if state.state == CircuitBreakerState::Closed
            && state.consecutive_failures >= self.config.failure_threshold
        {
            let transition = state.transition_to(CircuitBreakerState::Open);
            self.log_transition(&transition);
        } else if state.state == CircuitBreakerState::HalfOpen {
            // Any failure in half-open state reopens the circuit
            let transition = state.transition_to(CircuitBreakerState::Open);
            self.log_transition(&transition);
            *self.half_open_requests.write() = 0;
        }
    }

    /// Log and record state transition
    fn log_transition(&self, transition: &StateTransition) {
        info!(
            name = %self.name,
            from = %transition.from,
            to = %transition.to,
            reason = %transition.reason,
            "Circuit breaker state transition"
        );

        // Update metrics
        super::metrics::CIRCUIT_BREAKER_METRICS
            .state
            .with_label_values(&[&self.name])
            .set(transition.to.to_metric_value());

        super::metrics::CIRCUIT_BREAKER_METRICS
            .state_transitions
            .with_label_values(&[&self.name, &transition.from.to_string(), &transition.to.to_string()])
            .inc();
    }

    /// Get statistics for this circuit breaker
    pub fn stats(&self) -> CircuitBreakerStats {
        let state = self.state.read();
        CircuitBreakerStats {
            name: self.name.clone(),
            state: state.state,
            consecutive_failures: state.consecutive_failures,
            consecutive_successes: state.consecutive_successes,
            transition_count: state.transition_count,
            last_state_change: state.last_state_change,
        }
    }

    /// Manually reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut state = self.state.write();
        if state.state != CircuitBreakerState::Closed {
            let transition = state.transition_to(CircuitBreakerState::Closed);
            self.log_transition(&transition);
            *self.half_open_requests.write() = 0;
        }
    }

    /// Force the circuit breaker to open state
    pub fn force_open(&self) {
        let mut state = self.state.write();
        if state.state != CircuitBreakerState::Open {
            let transition = state.transition_to(CircuitBreakerState::Open);
            self.log_transition(&transition);
        }
    }
}

/// Statistics for a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub name: String,
    pub state: CircuitBreakerState,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub transition_count: u64,
    pub last_state_change: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(3)
            .build()
            .unwrap();
        let breaker = CircuitBreaker::new("test", config);

        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_successful_call() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test", config);

        let result = breaker
            .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_failed_call() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test", config);

        let result = breaker
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "test error",
                    ))
                })
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(3)
            .build()
            .unwrap();
        let breaker = CircuitBreaker::new("test", config);

        // Trigger failures
        for _ in 0..3 {
            let _ = breaker
                .call(|| {
                    Box::pin(async {
                        Err::<i32, std::io::Error>(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "test",
                        ))
                    })
                })
                .await;
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Open);

        // Next call should be rejected
        let result = breaker
            .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
            .await;

        assert!(matches!(result, Err(CircuitBreakerError::Open(_))));
    }

    #[tokio::test]
    async fn test_fallback_on_open_circuit() {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(2)
            .build()
            .unwrap();
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker
                .call(|| {
                    Box::pin(async {
                        Err::<i32, std::io::Error>(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "test",
                        ))
                    })
                })
                .await;
        }

        // Use fallback
        let result = breaker
            .call_with_fallback(
                || Box::pin(async { Ok::<i32, std::io::Error>(42) }),
                || Box::pin(async { 99 }),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 99);
    }

    #[tokio::test]
    async fn test_manual_reset() {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(2)
            .build()
            .unwrap();
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker
                .call(|| {
                    Box::pin(async {
                        Err::<i32, std::io::Error>(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "test",
                        ))
                    })
                })
                .await;
        }

        assert_eq!(breaker.state(), CircuitBreakerState::Open);

        // Reset
        breaker.reset();
        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_stats() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test", config);

        let stats = breaker.stats();
        assert_eq!(stats.name, "test");
        assert_eq!(stats.state, CircuitBreakerState::Closed);
        assert_eq!(stats.consecutive_failures, 0);
    }
}
