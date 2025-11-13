//! Circuit breaker configuration with builder pattern.

use crate::circuit_breaker::CircuitBreakerError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for a circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,

    /// Number of consecutive successes in half-open state before closing
    pub success_threshold: u32,

    /// Duration to wait before attempting to reset an open circuit
    pub timeout_duration: Duration,

    /// Maximum number of concurrent requests allowed in half-open state
    pub half_open_max_requests: u32,

    /// Whether to include timeout errors as failures
    pub count_timeout_as_failure: bool,

    /// Minimum number of requests before failure rate is calculated
    pub minimum_request_threshold: u32,
}

impl CircuitBreakerConfig {
    /// Create a new builder for CircuitBreakerConfig
    pub fn builder() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::default()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), CircuitBreakerError> {
        if self.failure_threshold == 0 {
            return Err(CircuitBreakerError::InvalidConfig(
                "failure_threshold must be greater than 0".to_string(),
            ));
        }

        if self.success_threshold == 0 {
            return Err(CircuitBreakerError::InvalidConfig(
                "success_threshold must be greater than 0".to_string(),
            ));
        }

        if self.timeout_duration.is_zero() {
            return Err(CircuitBreakerError::InvalidConfig(
                "timeout_duration must be greater than 0".to_string(),
            ));
        }

        if self.half_open_max_requests == 0 {
            return Err(CircuitBreakerError::InvalidConfig(
                "half_open_max_requests must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(60),
            half_open_max_requests: 3,
            count_timeout_as_failure: true,
            minimum_request_threshold: 10,
        }
    }
}

/// Builder for CircuitBreakerConfig with fluent API
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfigBuilder {
    failure_threshold: Option<u32>,
    success_threshold: Option<u32>,
    timeout_duration: Option<Duration>,
    half_open_max_requests: Option<u32>,
    count_timeout_as_failure: Option<bool>,
    minimum_request_threshold: Option<u32>,
}

impl Default for CircuitBreakerConfigBuilder {
    fn default() -> Self {
        Self {
            failure_threshold: None,
            success_threshold: None,
            timeout_duration: None,
            half_open_max_requests: None,
            count_timeout_as_failure: None,
            minimum_request_threshold: None,
        }
    }
}

impl CircuitBreakerConfigBuilder {
    /// Set the failure threshold
    pub fn failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = Some(threshold);
        self
    }

    /// Set the success threshold
    pub fn success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = Some(threshold);
        self
    }

    /// Set the timeout duration
    pub fn timeout_duration(mut self, duration: Duration) -> Self {
        self.timeout_duration = Some(duration);
        self
    }

    /// Set the maximum requests in half-open state
    pub fn half_open_max_requests(mut self, max: u32) -> Self {
        self.half_open_max_requests = Some(max);
        self
    }

    /// Set whether to count timeouts as failures
    pub fn count_timeout_as_failure(mut self, count: bool) -> Self {
        self.count_timeout_as_failure = Some(count);
        self
    }

    /// Set the minimum request threshold
    pub fn minimum_request_threshold(mut self, threshold: u32) -> Self {
        self.minimum_request_threshold = Some(threshold);
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<CircuitBreakerConfig, CircuitBreakerError> {
        let default = CircuitBreakerConfig::default();

        let config = CircuitBreakerConfig {
            failure_threshold: self.failure_threshold.unwrap_or(default.failure_threshold),
            success_threshold: self.success_threshold.unwrap_or(default.success_threshold),
            timeout_duration: self.timeout_duration.unwrap_or(default.timeout_duration),
            half_open_max_requests: self
                .half_open_max_requests
                .unwrap_or(default.half_open_max_requests),
            count_timeout_as_failure: self
                .count_timeout_as_failure
                .unwrap_or(default.count_timeout_as_failure),
            minimum_request_threshold: self
                .minimum_request_threshold
                .unwrap_or(default.minimum_request_threshold),
        };

        config.validate()?;
        Ok(config)
    }
}

/// Predefined configurations for common use cases
impl CircuitBreakerConfig {
    /// Configuration for external HTTP APIs (moderate tolerance)
    pub fn for_http_api() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(30),
            half_open_max_requests: 3,
            count_timeout_as_failure: true,
            minimum_request_threshold: 10,
        }
    }

    /// Configuration for LLM services (high tolerance, longer timeout)
    pub fn for_llm_service() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(120),
            half_open_max_requests: 2,
            count_timeout_as_failure: true,
            minimum_request_threshold: 5,
        }
    }

    /// Configuration for database operations (low tolerance)
    pub fn for_database() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 3,
            timeout_duration: Duration::from_secs(10),
            half_open_max_requests: 5,
            count_timeout_as_failure: true,
            minimum_request_threshold: 20,
        }
    }

    /// Configuration for notification services (high tolerance)
    pub fn for_notifications() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(60),
            half_open_max_requests: 3,
            count_timeout_as_failure: false,
            minimum_request_threshold: 15,
        }
    }

    /// Configuration for Redis/cache operations (moderate tolerance)
    pub fn for_cache() -> Self {
        Self {
            failure_threshold: 8,
            success_threshold: 3,
            timeout_duration: Duration::from_secs(20),
            half_open_max_requests: 4,
            count_timeout_as_failure: true,
            minimum_request_threshold: 15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_builder() {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(10)
            .success_threshold(3)
            .timeout_duration(Duration::from_secs(30))
            .build()
            .unwrap();

        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.timeout_duration, Duration::from_secs(30));
    }

    #[test]
    fn test_builder_with_defaults() {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(7)
            .build()
            .unwrap();

        assert_eq!(config.failure_threshold, 7);
        assert_eq!(config.success_threshold, 2); // default
    }

    #[test]
    fn test_invalid_config_zero_failure_threshold() {
        let result = CircuitBreakerConfig::builder()
            .failure_threshold(0)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_zero_success_threshold() {
        let result = CircuitBreakerConfig::builder()
            .success_threshold(0)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_predefined_configs() {
        assert!(CircuitBreakerConfig::for_http_api().validate().is_ok());
        assert!(CircuitBreakerConfig::for_llm_service().validate().is_ok());
        assert!(CircuitBreakerConfig::for_database().validate().is_ok());
        assert!(CircuitBreakerConfig::for_notifications().validate().is_ok());
        assert!(CircuitBreakerConfig::for_cache().validate().is_ok());
    }

    #[test]
    fn test_llm_service_config() {
        let config = CircuitBreakerConfig::for_llm_service();
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.timeout_duration, Duration::from_secs(120));
    }
}
