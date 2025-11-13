//! Production-ready Circuit Breaker implementation for fault tolerance.
//!
//! This module provides a comprehensive circuit breaker pattern implementation with:
//! - Thread-safe state management
//! - Prometheus metrics integration
//! - Async/await support
//! - Configurable failure thresholds
//! - Automatic recovery detection
//! - Fallback support
//! - Global registry for circuit breaker management
//!
//! # Circuit Breaker States
//!
//! - **Closed**: Normal operation, requests pass through, failures are counted
//! - **Open**: Fast-fail mode, all requests are rejected immediately
//! - **Half-Open**: Testing recovery, limited requests are allowed to test if the service recovered
//!
//! # Example
//!
//! ```no_run
//! use llm_incident_manager::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = CircuitBreakerConfig::builder()
//!         .failure_threshold(5)
//!         .timeout_duration(std::time::Duration::from_secs(60))
//!         .build()?;
//!
//!     let breaker = CircuitBreaker::new("my-service", config);
//!
//!     // Execute a protected operation
//!     let result = breaker.call(|| async {
//!         // Your async operation here
//!         Ok::<_, std::io::Error>(42)
//!     }).await;
//!
//!     Ok(())
//! }
//! ```

mod config;
mod core;
mod decorators;
mod metrics;
mod middleware;
mod registry;
mod state;

pub use config::{CircuitBreakerConfig, CircuitBreakerConfigBuilder};
pub use core::CircuitBreaker;
pub use decorators::{with_circuit_breaker, CircuitBreakerDecorator};
pub use metrics::{init_circuit_breaker_metrics, CIRCUIT_BREAKER_METRICS};
pub use middleware::CircuitBreakerMiddleware;
pub use registry::{get_circuit_breaker, CircuitBreakerRegistry, GLOBAL_CIRCUIT_BREAKER_REGISTRY};
pub use state::{CircuitBreakerState, StateData, StateTransition};

use crate::error::AppError;

/// Result type for circuit breaker operations
pub type CircuitBreakerResult<T> = std::result::Result<T, CircuitBreakerError>;

/// Errors that can occur in circuit breaker operations
#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError {
    /// Circuit is open and rejecting requests
    #[error("Circuit breaker is open for '{0}'")]
    Open(String),

    /// Configuration is invalid
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    /// Timeout occurred
    #[error("Operation timed out")]
    Timeout,

    /// Circuit breaker not found in registry
    #[error("Circuit breaker '{0}' not found")]
    NotFound(String),
}

impl From<CircuitBreakerError> for AppError {
    fn from(err: CircuitBreakerError) -> Self {
        match err {
            CircuitBreakerError::Open(name) => {
                AppError::Internal(format!("Circuit breaker open: {}", name))
            }
            CircuitBreakerError::InvalidConfig(msg) => AppError::Configuration(msg),
            CircuitBreakerError::OperationFailed(msg) => AppError::Internal(msg),
            CircuitBreakerError::Timeout => AppError::Timeout("Circuit breaker timeout".to_string()),
            CircuitBreakerError::NotFound(name) => {
                AppError::NotFound(format!("Circuit breaker: {}", name))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_error_conversion() {
        let err = CircuitBreakerError::Open("test-service".to_string());
        let app_err: AppError = err.into();
        assert!(matches!(app_err, AppError::Internal(_)));
    }
}
