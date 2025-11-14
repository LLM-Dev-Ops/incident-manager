//! Decorators and wrappers for function-level circuit breaking.

use crate::circuit_breaker::{
    get_circuit_breaker, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerResult,
};
use std::future::Future;
use std::sync::Arc;

/// Trait for wrapping async functions with circuit breaker logic
pub trait CircuitBreakerDecorator {
    /// Execute with circuit breaker protection
    fn with_breaker<F, T, E>(
        &self,
        name: &str,
        f: F,
    ) -> impl Future<Output = CircuitBreakerResult<T>>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        E: std::error::Error + Send + 'static;

    /// Execute with circuit breaker protection and fallback
    fn with_breaker_and_fallback<F, FB, T, E>(
        &self,
        name: &str,
        f: F,
        fallback: FB,
    ) -> impl Future<Output = CircuitBreakerResult<T>>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        FB: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = T> + Send>> + Send,
        E: std::error::Error + Send + 'static;
}

impl CircuitBreakerDecorator for CircuitBreakerConfig {
    async fn with_breaker<F, T, E>(&self, name: &str, f: F) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        E: std::error::Error + Send + 'static,
    {
        let breaker = get_circuit_breaker(name, self.clone());
        breaker.call(f).await
    }

    async fn with_breaker_and_fallback<F, FB, T, E>(
        &self,
        name: &str,
        f: F,
        fallback: FB,
    ) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        FB: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = T> + Send>> + Send,
        E: std::error::Error + Send + 'static,
    {
        let breaker = get_circuit_breaker(name, self.clone());
        breaker.call_with_fallback(f, fallback).await
    }
}

/// Helper function to execute an async operation with circuit breaker protection
///
/// # Example
///
/// ```no_run
/// use llm_incident_manager::circuit_breaker::{with_circuit_breaker, CircuitBreakerConfig};
///
/// async fn my_function() -> Result<String, std::io::Error> {
///     with_circuit_breaker(
///         "my-service",
///         CircuitBreakerConfig::default(),
///         || Box::pin(async {
///             // Your async operation
///             Ok("result".to_string())
///         })
///     ).await
/// }
/// ```
pub async fn with_circuit_breaker<F, T, E>(
    name: &str,
    config: CircuitBreakerConfig,
    f: F,
) -> CircuitBreakerResult<T>
where
    F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
    E: std::error::Error + Send + 'static,
{
    let breaker = get_circuit_breaker(name, config);
    breaker.call(f).await
}

/// Helper function to execute an async operation with circuit breaker and fallback
///
/// # Example
///
/// ```no_run
/// use llm_incident_manager::circuit_breaker::{with_circuit_breaker_and_fallback, CircuitBreakerConfig};
///
/// async fn my_function() -> Result<String, Box<dyn std::error::Error>> {
///     with_circuit_breaker_and_fallback(
///         "my-service",
///         CircuitBreakerConfig::default(),
///         || Box::pin(async {
///             // Your async operation
///             Ok::<String, std::io::Error>("result".to_string())
///         }),
///         || Box::pin(async {
///             // Fallback value
///             "fallback".to_string()
///         })
///     ).await
/// }
/// ```
#[allow(dead_code)]
pub async fn with_circuit_breaker_and_fallback<F, FB, T, E>(
    name: &str,
    config: CircuitBreakerConfig,
    f: F,
    fallback: FB,
) -> CircuitBreakerResult<T>
where
    F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
    FB: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = T> + Send>> + Send,
    E: std::error::Error + Send + 'static,
{
    let breaker = get_circuit_breaker(name, config);
    breaker.call_with_fallback(f, fallback).await
}

/// Wrapper for reqwest HTTP clients with circuit breaker
#[allow(dead_code)]
pub struct CircuitBreakerHttpClient {
    client: reqwest::Client,
    breaker: Arc<CircuitBreaker>,
}

#[allow(dead_code)]
impl CircuitBreakerHttpClient {
    /// Create a new HTTP client with circuit breaker
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            breaker: get_circuit_breaker(name, config),
        }
    }

    /// Create with custom reqwest client
    pub fn with_client(
        client: reqwest::Client,
        name: impl Into<String>,
        config: CircuitBreakerConfig,
    ) -> Self {
        Self {
            client,
            breaker: get_circuit_breaker(name, config),
        }
    }

    /// Get the underlying reqwest client
    pub fn inner(&self) -> &reqwest::Client {
        &self.client
    }

    /// Execute a GET request with circuit breaker protection
    pub async fn get(&self, url: impl reqwest::IntoUrl) -> CircuitBreakerResult<reqwest::Response> {
        let client = self.client.clone();
        let url = url.into_url().map_err(|e| {
            crate::circuit_breaker::CircuitBreakerError::OperationFailed(e.to_string())
        })?;

        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .get(url)
                        .send()
                        .await
                        .map_err(|e| e)
                })
            })
            .await
    }

    /// Execute a POST request with circuit breaker protection
    pub async fn post(&self, url: impl reqwest::IntoUrl) -> CircuitBreakerResult<reqwest::RequestBuilder> {
        let url = url.into_url().map_err(|e| {
            crate::circuit_breaker::CircuitBreakerError::OperationFailed(e.to_string())
        })?;

        Ok(self.client.post(url))
    }

    /// Get the circuit breaker instance
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

/// Wrapper for database operations with circuit breaker
#[allow(dead_code)]
pub struct CircuitBreakerDbWrapper {
    breaker: Arc<CircuitBreaker>,
}

#[allow(dead_code)]
impl CircuitBreakerDbWrapper {
    /// Create a new database wrapper with circuit breaker
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            breaker: get_circuit_breaker(name, CircuitBreakerConfig::for_database()),
        }
    }

    /// Create with custom configuration
    pub fn with_config(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            breaker: get_circuit_breaker(name, config),
        }
    }

    /// Execute a database operation with circuit breaker protection
    pub async fn execute<F, T, E>(&self, f: F) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        E: std::error::Error + Send + 'static,
    {
        self.breaker.call(f).await
    }

    /// Execute with fallback to degraded mode
    pub async fn execute_with_degraded<F, FB, T, E>(
        &self,
        f: F,
        degraded: FB,
    ) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>> + Send,
        FB: FnOnce() -> std::pin::Pin<Box<dyn Future<Output = T> + Send>> + Send,
        E: std::error::Error + Send + 'static,
    {
        self.breaker.call_with_fallback(f, degraded).await
    }

    /// Get the circuit breaker instance
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_circuit_breaker() {
        let config = CircuitBreakerConfig::default();
        let result = with_circuit_breaker("test", config, || {
            Box::pin(async { Ok::<i32, std::io::Error>(42) })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_circuit_breaker_and_fallback() {
        let config = CircuitBreakerConfig::default();
        let result = with_circuit_breaker_and_fallback(
            "test",
            config,
            || Box::pin(async { Ok::<i32, std::io::Error>(42) }),
            || Box::pin(async { 99 }),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_circuit_breaker_http_client() {
        let client = CircuitBreakerHttpClient::new("test-http", CircuitBreakerConfig::for_http_api());
        assert_eq!(client.breaker().name(), "test-http");
    }

    #[tokio::test]
    async fn test_circuit_breaker_db_wrapper() {
        let wrapper = CircuitBreakerDbWrapper::new("test-db");
        assert_eq!(wrapper.breaker().name(), "test-db");

        let result = wrapper
            .execute(|| Box::pin(async { Ok::<String, std::io::Error>("success".to_string()) }))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_db_wrapper_with_degraded() {
        let wrapper = CircuitBreakerDbWrapper::new("test-db-degraded");

        let result = wrapper
            .execute_with_degraded(
                || Box::pin(async { Ok::<String, std::io::Error>("success".to_string()) }),
                || Box::pin(async { "degraded".to_string() }),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }
}
