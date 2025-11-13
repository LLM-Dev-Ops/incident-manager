//! Circuit breaker state machine implementation.
//!
//! This module handles state transitions and state-specific behavior.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The current state of a circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Circuit is closed - requests are allowed through, failures are counted
    Closed,
    /// Circuit is open - all requests are rejected, waiting for timeout
    Open,
    /// Circuit is half-open - testing recovery with limited requests
    HalfOpen,
}

impl CircuitBreakerState {
    /// Convert state to numeric value for Prometheus gauge
    pub fn to_metric_value(&self) -> f64 {
        match self {
            CircuitBreakerState::Closed => 0.0,
            CircuitBreakerState::Open => 1.0,
            CircuitBreakerState::HalfOpen => 2.0,
        }
    }

    /// Check if requests should be allowed in this state
    pub fn allows_requests(&self) -> bool {
        matches!(self, CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen)
    }

    /// Check if this is a failure-counting state
    pub fn counts_failures(&self) -> bool {
        matches!(self, CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen)
    }
}

impl fmt::Display for CircuitBreakerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitBreakerState::Closed => write!(f, "closed"),
            CircuitBreakerState::Open => write!(f, "open"),
            CircuitBreakerState::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Represents a state transition in the circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Previous state
    pub from: CircuitBreakerState,
    /// New state
    pub to: CircuitBreakerState,
    /// When the transition occurred
    pub timestamp: DateTime<Utc>,
    /// Reason for the transition
    pub reason: String,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(from: CircuitBreakerState, to: CircuitBreakerState, reason: String) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason,
        }
    }
}

/// Internal state data for the circuit breaker
#[derive(Debug, Clone)]
pub struct StateData {
    /// Current state
    pub state: CircuitBreakerState,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Number of consecutive successes
    pub consecutive_successes: u32,
    /// When the state was last changed
    pub last_state_change: DateTime<Utc>,
    /// When the circuit was opened (if in Open state)
    pub opened_at: Option<DateTime<Utc>>,
    /// Total number of state transitions
    pub transition_count: u64,
}

impl StateData {
    /// Create new state data in Closed state
    pub fn new() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_state_change: Utc::now(),
            opened_at: None,
            transition_count: 0,
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.consecutive_successes += 1;
    }

    /// Record a failed request
    pub fn record_failure(&mut self) {
        self.consecutive_successes = 0;
        self.consecutive_failures += 1;
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: CircuitBreakerState) -> StateTransition {
        let transition = StateTransition::new(
            self.state,
            new_state,
            self.transition_reason(new_state),
        );

        self.state = new_state;
        self.last_state_change = Utc::now();
        self.transition_count += 1;

        if new_state == CircuitBreakerState::Open {
            self.opened_at = Some(Utc::now());
        } else if new_state == CircuitBreakerState::Closed {
            self.opened_at = None;
            self.consecutive_failures = 0;
            self.consecutive_successes = 0;
        }

        transition
    }

    /// Get a human-readable reason for the state transition
    fn transition_reason(&self, new_state: CircuitBreakerState) -> String {
        match (self.state, new_state) {
            (CircuitBreakerState::Closed, CircuitBreakerState::Open) => {
                format!("Failure threshold exceeded ({} consecutive failures)", self.consecutive_failures)
            }
            (CircuitBreakerState::Open, CircuitBreakerState::HalfOpen) => {
                "Timeout period elapsed, testing recovery".to_string()
            }
            (CircuitBreakerState::HalfOpen, CircuitBreakerState::Closed) => {
                format!("Recovery successful ({} consecutive successes)", self.consecutive_successes)
            }
            (CircuitBreakerState::HalfOpen, CircuitBreakerState::Open) => {
                "Recovery test failed".to_string()
            }
            _ => format!("Transitioned from {} to {}", self.state, new_state),
        }
    }

    /// Check if enough time has passed to transition from Open to HalfOpen
    pub fn should_attempt_reset(&self, timeout_duration: std::time::Duration) -> bool {
        if self.state != CircuitBreakerState::Open {
            return false;
        }

        if let Some(opened_at) = self.opened_at {
            let elapsed = Utc::now().signed_duration_since(opened_at);
            elapsed.num_milliseconds() as u64 >= timeout_duration.as_millis() as u64
        } else {
            false
        }
    }
}

impl Default for StateData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_state_metric_values() {
        assert_eq!(CircuitBreakerState::Closed.to_metric_value(), 0.0);
        assert_eq!(CircuitBreakerState::Open.to_metric_value(), 1.0);
        assert_eq!(CircuitBreakerState::HalfOpen.to_metric_value(), 2.0);
    }

    #[test]
    fn test_state_allows_requests() {
        assert!(CircuitBreakerState::Closed.allows_requests());
        assert!(!CircuitBreakerState::Open.allows_requests());
        assert!(CircuitBreakerState::HalfOpen.allows_requests());
    }

    #[test]
    fn test_state_data_success() {
        let mut data = StateData::new();
        assert_eq!(data.consecutive_failures, 0);
        assert_eq!(data.consecutive_successes, 0);

        data.record_success();
        assert_eq!(data.consecutive_successes, 1);
        assert_eq!(data.consecutive_failures, 0);

        data.record_success();
        assert_eq!(data.consecutive_successes, 2);
    }

    #[test]
    fn test_state_data_failure() {
        let mut data = StateData::new();
        data.record_failure();
        assert_eq!(data.consecutive_failures, 1);
        assert_eq!(data.consecutive_successes, 0);

        data.record_failure();
        assert_eq!(data.consecutive_failures, 2);
    }

    #[test]
    fn test_state_transition() {
        let mut data = StateData::new();
        assert_eq!(data.state, CircuitBreakerState::Closed);
        assert_eq!(data.transition_count, 0);

        let transition = data.transition_to(CircuitBreakerState::Open);
        assert_eq!(transition.from, CircuitBreakerState::Closed);
        assert_eq!(transition.to, CircuitBreakerState::Open);
        assert_eq!(data.state, CircuitBreakerState::Open);
        assert_eq!(data.transition_count, 1);
        assert!(data.opened_at.is_some());
    }

    #[test]
    fn test_should_attempt_reset() {
        let mut data = StateData::new();
        data.transition_to(CircuitBreakerState::Open);

        // Should not reset immediately
        assert!(!data.should_attempt_reset(std::time::Duration::from_millis(100)));

        // Wait for timeout
        sleep(std::time::Duration::from_millis(150));
        assert!(data.should_attempt_reset(std::time::Duration::from_millis(100)));
    }

    #[test]
    fn test_reset_clears_counters() {
        let mut data = StateData::new();
        data.record_failure();
        data.record_failure();
        data.transition_to(CircuitBreakerState::Open);

        data.transition_to(CircuitBreakerState::Closed);
        assert_eq!(data.consecutive_failures, 0);
        assert_eq!(data.consecutive_successes, 0);
        assert!(data.opened_at.is_none());
    }
}
