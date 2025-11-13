//! Global registry for managing circuit breakers.

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tracing::{debug, info};

/// Global registry for all circuit breakers in the application
pub struct CircuitBreakerRegistry {
    /// Map of circuit breaker name to instance
    breakers: DashMap<String, Arc<CircuitBreaker>>,
}

impl CircuitBreakerRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            breakers: DashMap::new(),
        }
    }

    /// Get or create a circuit breaker with the given name and config
    pub fn get_or_create(
        &self,
        name: impl Into<String>,
        config: CircuitBreakerConfig,
    ) -> Arc<CircuitBreaker> {
        let name = name.into();

        self.breakers
            .entry(name.clone())
            .or_insert_with(|| {
                info!(name = %name, "Creating new circuit breaker in registry");
                Arc::new(CircuitBreaker::new(name.clone(), config))
            })
            .clone()
    }

    /// Get an existing circuit breaker by name
    pub fn get(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        self.breakers.get(name).map(|entry| entry.value().clone())
    }

    /// Register a pre-configured circuit breaker
    pub fn register(&self, breaker: CircuitBreaker) -> Arc<CircuitBreaker> {
        let name = breaker.name().to_string();
        let arc_breaker = Arc::new(breaker);

        self.breakers.insert(name.clone(), arc_breaker.clone());
        info!(name = %name, "Registered circuit breaker");

        arc_breaker
    }

    /// Remove a circuit breaker from the registry
    pub fn remove(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        debug!(name = %name, "Removing circuit breaker from registry");
        self.breakers.remove(name).map(|(_, breaker)| breaker)
    }

    /// Get all circuit breaker names
    pub fn list_names(&self) -> Vec<String> {
        self.breakers.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get statistics for all circuit breakers
    pub fn get_all_stats(&self) -> Vec<super::core::CircuitBreakerStats> {
        self.breakers
            .iter()
            .map(|entry| entry.value().stats())
            .collect()
    }

    /// Reset all circuit breakers to closed state
    pub fn reset_all(&self) {
        info!("Resetting all circuit breakers");
        for entry in self.breakers.iter() {
            entry.value().reset();
        }
    }

    /// Get count of circuit breakers in each state
    pub fn get_state_counts(&self) -> StateCount {
        let mut counts = StateCount::default();

        for entry in self.breakers.iter() {
            match entry.value().state() {
                CircuitBreakerState::Closed => counts.closed += 1,
                CircuitBreakerState::Open => counts.open += 1,
                CircuitBreakerState::HalfOpen => counts.half_open += 1,
            }
        }

        counts
    }

    /// Check if any circuit breakers are open
    pub fn has_open_circuits(&self) -> bool {
        self.breakers
            .iter()
            .any(|entry| entry.value().state() == CircuitBreakerState::Open)
    }

    /// Get health check information
    pub fn health_check(&self) -> RegistryHealth {
        let state_counts = self.get_state_counts();
        let total = state_counts.total();

        RegistryHealth {
            total_breakers: total,
            closed: state_counts.closed,
            open: state_counts.open,
            half_open: state_counts.half_open,
            healthy: state_counts.open == 0,
        }
    }

    /// Clear all circuit breakers from the registry
    pub fn clear(&self) {
        info!("Clearing all circuit breakers from registry");
        self.breakers.clear();
    }

    /// Get the total number of circuit breakers
    pub fn len(&self) -> usize {
        self.breakers.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.breakers.is_empty()
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Count of circuit breakers in each state
#[derive(Debug, Clone, Default)]
pub struct StateCount {
    pub closed: usize,
    pub open: usize,
    pub half_open: usize,
}

impl StateCount {
    pub fn total(&self) -> usize {
        self.closed + self.open + self.half_open
    }
}

/// Health information for the circuit breaker registry
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryHealth {
    pub total_breakers: usize,
    pub closed: usize,
    pub open: usize,
    pub half_open: usize,
    pub healthy: bool,
}

/// Global circuit breaker registry instance
pub static GLOBAL_CIRCUIT_BREAKER_REGISTRY: Lazy<CircuitBreakerRegistry> =
    Lazy::new(CircuitBreakerRegistry::new);

/// Helper function to get or create a circuit breaker from the global registry
pub fn get_circuit_breaker(
    name: impl Into<String>,
    config: CircuitBreakerConfig,
) -> Arc<CircuitBreaker> {
    GLOBAL_CIRCUIT_BREAKER_REGISTRY.get_or_create(name, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = CircuitBreakerRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_get_or_create() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        let breaker1 = registry.get_or_create("test", config.clone());
        assert_eq!(breaker1.name(), "test");
        assert_eq!(registry.len(), 1);

        // Getting again should return the same instance
        let breaker2 = registry.get_or_create("test", config);
        assert_eq!(Arc::ptr_eq(&breaker1, &breaker2), true);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_get() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        assert!(registry.get("nonexistent").is_none());

        registry.get_or_create("test", config);
        let breaker = registry.get("test");
        assert!(breaker.is_some());
        assert_eq!(breaker.unwrap().name(), "test");
    }

    #[test]
    fn test_register() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("custom", config);

        let registered = registry.register(breaker);
        assert_eq!(registered.name(), "custom");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_remove() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        registry.get_or_create("test", config);
        assert_eq!(registry.len(), 1);

        let removed = registry.remove("test");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);

        let not_found = registry.remove("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_list_names() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        registry.get_or_create("test1", config.clone());
        registry.get_or_create("test2", config.clone());
        registry.get_or_create("test3", config);

        let names = registry.list_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"test1".to_string()));
        assert!(names.contains(&"test2".to_string()));
        assert!(names.contains(&"test3".to_string()));
    }

    #[test]
    fn test_reset_all() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(2)
            .build()
            .unwrap();

        let breaker = registry.get_or_create("test", config);
        breaker.force_open();
        assert_eq!(breaker.state(), CircuitBreakerState::Open);

        registry.reset_all();
        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_state_counts() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        let b1 = registry.get_or_create("test1", config.clone());
        let b2 = registry.get_or_create("test2", config.clone());
        let b3 = registry.get_or_create("test3", config);

        b2.force_open();

        let counts = registry.get_state_counts();
        assert_eq!(counts.closed, 2);
        assert_eq!(counts.open, 1);
        assert_eq!(counts.half_open, 0);
        assert_eq!(counts.total(), 3);
    }

    #[test]
    fn test_has_open_circuits() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        let breaker = registry.get_or_create("test", config);
        assert!(!registry.has_open_circuits());

        breaker.force_open();
        assert!(registry.has_open_circuits());
    }

    #[test]
    fn test_health_check() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        let health = registry.health_check();
        assert_eq!(health.total_breakers, 0);
        assert!(health.healthy);

        let b1 = registry.get_or_create("test1", config.clone());
        let b2 = registry.get_or_create("test2", config);

        let health = registry.health_check();
        assert_eq!(health.total_breakers, 2);
        assert!(health.healthy);

        b1.force_open();
        let health = registry.health_check();
        assert!(!health.healthy);
        assert_eq!(health.open, 1);
    }

    #[test]
    fn test_clear() {
        let registry = CircuitBreakerRegistry::new();
        let config = CircuitBreakerConfig::default();

        registry.get_or_create("test1", config.clone());
        registry.get_or_create("test2", config);
        assert_eq!(registry.len(), 2);

        registry.clear();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }
}
