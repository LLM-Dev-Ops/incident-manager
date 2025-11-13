//! Circuit breaker wrapper for storage operations.

use crate::circuit_breaker::{
    get_circuit_breaker, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerResult,
};
use crate::error::{AppError, Result};
use crate::models::Incident;
use crate::state::{IncidentFilter, IncidentStore};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Wrapper that adds circuit breaker protection to any IncidentStore implementation
pub struct CircuitBreakerStore<S: IncidentStore> {
    inner: Arc<S>,
    breaker: Arc<CircuitBreaker>,
}

impl<S: IncidentStore> CircuitBreakerStore<S> {
    /// Create a new circuit breaker store wrapper
    pub fn new(inner: S, name: impl Into<String>) -> Self {
        let config = CircuitBreakerConfig::for_database();
        let breaker = get_circuit_breaker(name, config);

        Self {
            inner: Arc::new(inner),
            breaker
        }
    }

    /// Create with custom circuit breaker configuration
    pub fn with_config(inner: S, name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let breaker = get_circuit_breaker(name, config);
        Self {
            inner: Arc::new(inner),
            breaker
        }
    }

    /// Get the underlying store
    pub fn inner(&self) -> &Arc<S> {
        &self.inner
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }

    /// Execute a store operation with circuit breaker protection
    async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        self.breaker
            .call(|| Box::pin(async move { f().await.map_err(|e| StoreErrorWrapper(e)) }))
            .await
            .map_err(|e| match e {
                crate::circuit_breaker::CircuitBreakerError::Open(name) => {
                    AppError::Internal(format!("Storage circuit breaker open: {}", name))
                }
                crate::circuit_breaker::CircuitBreakerError::OperationFailed(msg) => {
                    // Extract the original AppError from the wrapper
                    AppError::Database(msg)
                }
                e => AppError::Internal(e.to_string()),
            })
    }
}

#[async_trait]
impl<S: IncidentStore + 'static> IncidentStore for CircuitBreakerStore<S> {
    async fn save_incident(&self, incident: &Incident) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let incident = incident.clone();
        self.execute(move || Box::pin(async move { inner.save_incident(&incident).await }))
            .await
    }

    async fn get_incident(&self, id: &Uuid) -> Result<Option<Incident>> {
        let inner = Arc::clone(&self.inner);
        let id = *id;
        self.execute(move || Box::pin(async move { inner.get_incident(&id).await }))
            .await
    }

    async fn update_incident(&self, incident: &Incident) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let incident = incident.clone();
        self.execute(move || Box::pin(async move { inner.update_incident(&incident).await }))
            .await
    }

    async fn delete_incident(&self, id: &Uuid) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let id = *id;
        self.execute(move || Box::pin(async move { inner.delete_incident(&id).await }))
            .await
    }

    async fn list_incidents(
        &self,
        filter: &IncidentFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Incident>> {
        let inner = Arc::clone(&self.inner);
        let filter = filter.clone();
        self.execute(move || {
            Box::pin(async move { inner.list_incidents(&filter, page, page_size).await })
        })
        .await
    }

    async fn count_incidents(&self, filter: &IncidentFilter) -> Result<u64> {
        let inner = Arc::clone(&self.inner);
        let filter = filter.clone();
        self.execute(move || Box::pin(async move { inner.count_incidents(&filter).await }))
            .await
    }

    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Vec<Incident>> {
        let inner = Arc::clone(&self.inner);
        let fingerprint = fingerprint.to_string();
        self.execute(move || Box::pin(async move { inner.find_by_fingerprint(&fingerprint).await }))
            .await
    }
}

/// Wrapper for AppError to implement std::error::Error
#[derive(Debug)]
struct StoreErrorWrapper(AppError);

impl std::fmt::Display for StoreErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StoreErrorWrapper {}

/// Wrapper for Redis operations with circuit breaker
pub struct CircuitBreakerRedis {
    breaker: Arc<CircuitBreaker>,
}

impl CircuitBreakerRedis {
    /// Create a new Redis wrapper with circuit breaker
    pub fn new(name: impl Into<String>) -> Self {
        let config = CircuitBreakerConfig::for_cache();
        let breaker = get_circuit_breaker(name, config);

        Self { breaker }
    }

    /// Create with custom configuration
    pub fn with_config(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let breaker = get_circuit_breaker(name, config);
        Self { breaker }
    }

    /// Execute a Redis operation with circuit breaker protection
    pub async fn execute<F, T, E>(&self, f: F) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send>>
            + Send,
        E: std::error::Error + Send + 'static,
    {
        self.breaker.call(f).await
    }

    /// Execute with fallback to degraded mode
    pub async fn execute_with_fallback<F, FB, T, E>(
        &self,
        f: F,
        fallback: FB,
    ) -> CircuitBreakerResult<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send>>
            + Send,
        FB: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>> + Send,
        E: std::error::Error + Send + 'static,
    {
        self.breaker.call_with_fallback(f, fallback).await
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, Severity};
    use chrono::Utc;

    // Mock store for testing
    struct MockStore;

    #[async_trait]
    impl IncidentStore for MockStore {
        async fn save_incident(&self, _incident: &Incident) -> Result<()> {
            Ok(())
        }

        async fn get_incident(&self, _id: &Uuid) -> Result<Option<Incident>> {
            Ok(None)
        }

        async fn update_incident(&self, _incident: &Incident) -> Result<()> {
            Ok(())
        }

        async fn delete_incident(&self, _id: &Uuid) -> Result<()> {
            Ok(())
        }

        async fn list_incidents(
            &self,
            _filter: &IncidentFilter,
            _page: u32,
            _page_size: u32,
        ) -> Result<Vec<Incident>> {
            Ok(vec![])
        }

        async fn count_incidents(&self, _filter: &IncidentFilter) -> Result<u64> {
            Ok(0)
        }

        async fn find_by_fingerprint(&self, _fingerprint: &str) -> Result<Vec<Incident>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_store_creation() {
        let mock = MockStore;
        let store = CircuitBreakerStore::new(mock, "test-store");
        assert_eq!(store.breaker().name(), "test-store");
    }

    #[tokio::test]
    async fn test_circuit_breaker_store_get() {
        let mock = MockStore;
        let store = CircuitBreakerStore::new(mock, "test-store-get");
        let id = Uuid::new_v4();

        let result = store.get_incident(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_circuit_breaker_redis_creation() {
        let redis = CircuitBreakerRedis::new("test-redis");
        assert_eq!(redis.breaker().name(), "test-redis");
    }
}
