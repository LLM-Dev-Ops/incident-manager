# Circuit Breaker Fallback Mechanisms

## Overview

This document defines comprehensive fallback strategies for circuit breaker implementations in the LLM Incident Manager. Fallback mechanisms provide graceful degradation when services are unavailable or circuit breakers are open.

## Fallback Strategies

### 1. Static Value Fallback

Return a predefined static value when the circuit is open.

#### Implementation

```rust
pub struct StaticFallback<T> {
    value: T,
}

impl<T: Clone> StaticFallback<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn get(&self) -> T {
        self.value.clone()
    }
}

// Usage with circuit breaker
impl CircuitBreaker {
    pub async fn call_with_static_fallback<F, Fut, T, E>(
        &self,
        operation: F,
        fallback_value: T,
    ) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        T: Clone,
        E: std::error::Error,
    {
        match self.call(operation).await {
            Ok(value) => Ok(value),
            Err(CircuitBreakerError::CircuitOpen) => {
                self.metrics.record_fallback();
                Ok(fallback_value)
            }
            Err(e) => Err(e),
        }
    }
}
```

#### Usage Example

```rust
// Return empty list when Sentinel is unavailable
let alerts = circuit_breaker
    .call_with_static_fallback(
        || sentinel_client.fetch_alerts(None),
        Vec::new(), // Empty list as fallback
    )
    .await?;

// Return default configuration
let config = circuit_breaker
    .call_with_static_fallback(
        || fetch_remote_config(),
        AppConfig::default(),
    )
    .await?;
```

### 2. Cache-Based Fallback

Return cached values when the circuit is open.

#### Implementation

```rust
use moka::future::Cache;
use std::hash::Hash;

pub struct CacheFallback<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    cache: Cache<K, V>,
    ttl: Duration,
}

impl<K, V> CacheFallback<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();

        Self { cache, ttl }
    }

    pub async fn get_or_fetch<F, Fut, E>(
        &self,
        key: K,
        fetch: F,
    ) -> Result<V, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, E>>,
    {
        // Try cache first
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        // Fetch and cache on success
        let value = fetch().await?;
        self.cache.insert(key, value.clone()).await;
        Ok(value)
    }

    pub async fn get_cached(&self, key: &K) -> Option<V> {
        self.cache.get(key).await
    }

    pub async fn invalidate(&self, key: &K) {
        self.cache.invalidate(key).await;
    }
}

// Circuit breaker integration
impl CircuitBreaker {
    pub async fn call_with_cache<F, Fut, K, V, E>(
        &self,
        cache: &CacheFallback<K, V>,
        cache_key: K,
        operation: F,
    ) -> Result<V, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, E>>,
        K: Hash + Eq + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
        E: std::error::Error,
    {
        match self.call(operation).await {
            Ok(value) => {
                // Cache the successful result
                cache.cache.insert(cache_key, value.clone()).await;
                Ok(value)
            }
            Err(CircuitBreakerError::CircuitOpen) => {
                // Try cache fallback
                if let Some(cached) = cache.get_cached(&cache_key).await {
                    self.metrics.record_fallback();
                    tracing::warn!("Circuit open, using cached value");
                    Ok(cached)
                } else {
                    Err(CircuitBreakerError::CircuitOpen)
                }
            }
            Err(e) => Err(e),
        }
    }
}
```

#### Usage Example

```rust
// Create cache for Sentinel alerts
let alert_cache = Arc::new(CacheFallback::new(
    1000,                       // max 1000 entries
    Duration::from_secs(300),   // 5 minute TTL
));

// Fetch with cache fallback
let cache_key = "recent_alerts";
let alerts = circuit_breaker
    .call_with_cache(
        &alert_cache,
        cache_key,
        || sentinel_client.fetch_alerts(Some(10)),
    )
    .await?;
```

### 3. Function-Based Fallback

Execute a custom fallback function when the circuit is open.

#### Implementation

```rust
pub type FallbackFn<T> = Arc<dyn Fn() -> T + Send + Sync>;
pub type AsyncFallbackFn<T> = Arc<dyn Fn() -> BoxFuture<'static, T> + Send + Sync>;

impl CircuitBreaker {
    pub async fn call_with_fallback<F, Fut, Fb, T, E>(
        &self,
        operation: F,
        fallback: Fb,
    ) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        Fb: FnOnce() -> T,
        E: std::error::Error,
    {
        match self.call(operation).await {
            Ok(value) => Ok(value),
            Err(CircuitBreakerError::CircuitOpen) => {
                self.metrics.record_fallback();
                self.emit_event(CircuitBreakerEvent::FallbackExecuted {
                    reason: "Circuit open".to_string(),
                    timestamp: Instant::now(),
                });
                Ok(fallback())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn call_with_async_fallback<F, Fut, Fb, FbFut, T, E>(
        &self,
        operation: F,
        fallback: Fb,
    ) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        Fb: FnOnce() -> FbFut,
        FbFut: Future<Output = T>,
        E: std::error::Error,
    {
        match self.call(operation).await {
            Ok(value) => Ok(value),
            Err(CircuitBreakerError::CircuitOpen) => {
                self.metrics.record_fallback();
                Ok(fallback().await)
            }
            Err(e) => Err(e),
        }
    }
}
```

#### Usage Example

```rust
// Sync fallback function
let data = circuit_breaker
    .call_with_fallback(
        || fetch_from_primary(),
        || {
            tracing::warn!("Using degraded mode");
            get_default_data()
        },
    )
    .await?;

// Async fallback function
let data = circuit_breaker
    .call_with_async_fallback(
        || fetch_from_primary(),
        || async {
            // Fetch from secondary source
            fetch_from_backup().await.unwrap_or_default()
        },
    )
    .await?;
```

### 4. Multi-Tier Fallback Chain

Attempt multiple fallback strategies in sequence.

#### Implementation

```rust
pub struct FallbackChain<T> {
    strategies: Vec<Box<dyn FallbackStrategy<T>>>,
}

#[async_trait]
pub trait FallbackStrategy<T>: Send + Sync {
    async fn execute(&self) -> Option<T>;
    fn name(&self) -> &str;
}

impl<T> FallbackChain<T>
where
    T: Send + Sync,
{
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    pub fn add_strategy(mut self, strategy: Box<dyn FallbackStrategy<T>>) -> Self {
        self.strategies.push(strategy);
        self
    }

    pub async fn execute(&self) -> Option<T> {
        for strategy in &self.strategies {
            tracing::debug!(strategy = strategy.name(), "Trying fallback strategy");

            if let Some(value) = strategy.execute().await {
                tracing::info!(
                    strategy = strategy.name(),
                    "Fallback strategy succeeded"
                );
                return Some(value);
            }
        }

        tracing::warn!("All fallback strategies failed");
        None
    }
}

// Example strategies
pub struct CacheStrategy<K, V> {
    cache: Arc<CacheFallback<K, V>>,
    key: K,
}

#[async_trait]
impl<K, V> FallbackStrategy<V> for CacheStrategy<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn execute(&self) -> Option<V> {
        self.cache.get_cached(&self.key).await
    }

    fn name(&self) -> &str {
        "cache"
    }
}

pub struct SecondaryServiceStrategy {
    client: Arc<SecondaryClient>,
}

#[async_trait]
impl FallbackStrategy<Vec<Alert>> for SecondaryServiceStrategy {
    async fn execute(&self) -> Option<Vec<Alert>> {
        self.client.fetch_alerts().await.ok()
    }

    fn name(&self) -> &str {
        "secondary_service"
    }
}

pub struct DefaultValueStrategy<T> {
    value: T,
}

#[async_trait]
impl<T: Clone + Send + Sync + 'static> FallbackStrategy<T> for DefaultValueStrategy<T> {
    async fn execute(&self) -> Option<T> {
        Some(self.value.clone())
    }

    fn name(&self) -> &str {
        "default_value"
    }
}
```

#### Usage Example

```rust
// Build fallback chain
let fallback_chain = FallbackChain::new()
    .add_strategy(Box::new(CacheStrategy {
        cache: alert_cache.clone(),
        key: "recent_alerts".to_string(),
    }))
    .add_strategy(Box::new(SecondaryServiceStrategy {
        client: backup_sentinel_client.clone(),
    }))
    .add_strategy(Box::new(DefaultValueStrategy {
        value: Vec::new(),
    }));

// Use with circuit breaker
let alerts = match circuit_breaker.call(|| primary_client.fetch_alerts(None)).await {
    Ok(alerts) => alerts,
    Err(CircuitBreakerError::CircuitOpen) => {
        fallback_chain
            .execute()
            .await
            .expect("Fallback chain exhausted")
    }
    Err(e) => return Err(e),
};
```

### 5. Stale Data Fallback

Return stale cached data with staleness indicator.

#### Implementation

```rust
#[derive(Clone)]
pub struct StaleValue<T> {
    pub value: T,
    pub cached_at: Instant,
    pub is_stale: bool,
}

impl<T> StaleValue<T> {
    pub fn fresh(value: T) -> Self {
        Self {
            value,
            cached_at: Instant::now(),
            is_stale: false,
        }
    }

    pub fn stale(value: T, cached_at: Instant) -> Self {
        Self {
            value,
            cached_at,
            is_stale: true,
        }
    }

    pub fn age(&self) -> Duration {
        self.cached_at.elapsed()
    }
}

pub struct StaleCache<K, V>
where
    K: Hash + Eq,
{
    cache: DashMap<K, (V, Instant)>,
    fresh_ttl: Duration,
    stale_ttl: Duration,
}

impl<K, V> StaleCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(fresh_ttl: Duration, stale_ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            fresh_ttl,
            stale_ttl,
        }
    }

    pub fn insert(&self, key: K, value: V) {
        self.cache.insert(key, (value, Instant::now()));
    }

    pub fn get(&self, key: &K) -> Option<StaleValue<V>> {
        self.cache.get(key).map(|entry| {
            let (value, cached_at) = entry.value();
            let age = cached_at.elapsed();

            if age < self.fresh_ttl {
                StaleValue::fresh(value.clone())
            } else if age < self.stale_ttl {
                StaleValue::stale(value.clone(), *cached_at)
            } else {
                // Remove expired entry
                drop(entry);
                self.cache.remove(key);
                return None;
            }
        })?
    }
}

impl CircuitBreaker {
    pub async fn call_with_stale_cache<F, Fut, K, V, E>(
        &self,
        cache: &StaleCache<K, V>,
        key: K,
        operation: F,
    ) -> Result<StaleValue<V>, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, E>>,
        K: Hash + Eq + Clone,
        V: Clone,
        E: std::error::Error,
    {
        match self.call(operation).await {
            Ok(value) => {
                // Cache fresh value
                cache.insert(key, value.clone());
                Ok(StaleValue::fresh(value))
            }
            Err(CircuitBreakerError::CircuitOpen) => {
                // Try stale cache
                if let Some(stale_value) = cache.get(&key) {
                    if stale_value.is_stale {
                        tracing::warn!(
                            age_secs = stale_value.age().as_secs(),
                            "Returning stale cached value"
                        );
                    }
                    self.metrics.record_fallback();
                    Ok(stale_value)
                } else {
                    Err(CircuitBreakerError::CircuitOpen)
                }
            }
            Err(e) => Err(e),
        }
    }
}
```

#### Usage Example

```rust
let stale_cache = Arc::new(StaleCache::new(
    Duration::from_secs(60),    // Fresh for 1 minute
    Duration::from_secs(3600),  // Accept stale up to 1 hour
));

let result = circuit_breaker
    .call_with_stale_cache(
        &stale_cache,
        "alerts",
        || sentinel_client.fetch_alerts(None),
    )
    .await?;

if result.is_stale {
    // Display warning to user
    warn!("Displaying stale data from {} seconds ago", result.age().as_secs());
}

process_alerts(result.value);
```

### 6. Graceful Degradation Patterns

#### Read-Only Mode

```rust
pub struct DegradedService<T> {
    service: T,
    circuit_breaker: Arc<CircuitBreaker>,
    read_only_mode: Arc<AtomicBool>,
}

impl<T> DegradedService<T> {
    pub fn new(service: T, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            service,
            circuit_breaker,
            read_only_mode: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_read_only(&self) -> bool {
        self.circuit_breaker.is_open() || self.read_only_mode.load(Ordering::Acquire)
    }

    pub fn enable_read_only(&self) {
        self.read_only_mode.store(true, Ordering::Release);
    }

    pub fn disable_read_only(&self) {
        self.read_only_mode.store(false, Ordering::Release);
    }
}

// Example: Database service
impl DegradedService<DatabasePool> {
    pub async fn execute_query<T>(&self, query: &str) -> Result<T, ServiceError> {
        if self.is_read_only() {
            return Err(ServiceError::ReadOnlyMode);
        }

        self.circuit_breaker
            .call(|| self.service.execute(query))
            .await
            .map_err(|e| e.into())
    }

    pub async fn read_query<T>(&self, query: &str) -> Result<T, ServiceError> {
        // Reads allowed even in degraded mode
        self.service
            .execute_readonly(query)
            .await
            .map_err(|e| e.into())
    }
}
```

#### Feature Flagging

```rust
pub struct FeatureFallback {
    features: DashMap<String, bool>,
    circuit_breakers: DashMap<String, Arc<CircuitBreaker>>,
}

impl FeatureFallback {
    pub fn new() -> Self {
        Self {
            features: DashMap::new(),
            circuit_breakers: DashMap::new(),
        }
    }

    pub fn register_feature(&self, name: String, circuit_breaker: Arc<CircuitBreaker>) {
        self.circuit_breakers.insert(name.clone(), circuit_breaker);
        self.features.insert(name, true);
    }

    pub fn is_enabled(&self, feature: &str) -> bool {
        // Check if circuit is open
        if let Some(cb) = self.circuit_breakers.get(feature) {
            if cb.is_open() {
                return false;
            }
        }

        // Check feature flag
        self.features
            .get(feature)
            .map(|v| *v.value())
            .unwrap_or(false)
    }

    pub async fn execute_if_enabled<F, Fut, T, Fb>(
        &self,
        feature: &str,
        operation: F,
        fallback: Fb,
    ) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
        Fb: FnOnce() -> T,
    {
        if self.is_enabled(feature) {
            operation().await
        } else {
            tracing::info!(feature = feature, "Feature disabled, using fallback");
            fallback()
        }
    }
}
```

#### Usage Example

```rust
let features = Arc::new(FeatureFallback::new());

// Register ML classification feature
features.register_feature(
    "ml_classification".to_string(),
    ml_circuit_breaker,
);

// Use with fallback
let severity = features
    .execute_if_enabled(
        "ml_classification",
        || async {
            ml_service.predict_severity(&incident).await.unwrap()
        },
        || {
            // Fallback to rule-based classification
            rule_based_classifier.classify(&incident)
        },
    )
    .await;
```

## Fallback Best Practices

### 1. Fallback Metrics

Track fallback usage for monitoring:

```rust
pub struct FallbackMetrics {
    pub fallback_executions: AtomicU64,
    pub fallback_successes: AtomicU64,
    pub fallback_failures: AtomicU64,
    pub last_fallback_at: Arc<RwLock<Option<Instant>>>,
}

impl FallbackMetrics {
    pub fn record_fallback_execution(&self, success: bool) {
        self.fallback_executions.fetch_add(1, Ordering::Relaxed);

        if success {
            self.fallback_successes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.fallback_failures.fetch_add(1, Ordering::Relaxed);
        }

        *self.last_fallback_at.write() = Some(Instant::now());
    }
}
```

### 2. Fallback Testing

Ensure fallbacks work correctly:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_fallback() {
        let cache = CacheFallback::new(100, Duration::from_secs(60));
        let cb = CircuitBreaker::new("test".to_string(), config);

        // Prime the cache
        let result = cb
            .call_with_cache(&cache, "key", || async { Ok::<_, String>("value") })
            .await
            .unwrap();

        assert_eq!(result, "value");

        // Force circuit open
        cb.force_open();

        // Should return cached value
        let result = cb
            .call_with_cache(&cache, "key", || async { Ok::<_, String>("new") })
            .await
            .unwrap();

        assert_eq!(result, "value"); // Old cached value
    }

    #[tokio::test]
    async fn test_fallback_chain() {
        let chain = FallbackChain::new()
            .add_strategy(Box::new(CacheStrategy { /* ... */ }))
            .add_strategy(Box::new(DefaultValueStrategy {
                value: Vec::new(),
            }));

        let result = chain.execute().await;
        assert!(result.is_some());
    }
}
```

### 3. Fallback Documentation

Document fallback behavior in client APIs:

```rust
impl SentinelClient {
    /// Fetch alerts with automatic fallback to cache
    ///
    /// # Fallback Behavior
    ///
    /// When the primary service is unavailable (circuit open):
    /// 1. Returns cached alerts if available (up to 5 minutes old)
    /// 2. Returns empty vec if no cache available
    ///
    /// # Returns
    ///
    /// - `Ok(StaleValue<Vec<Alert>>)` with `is_stale = false` for fresh data
    /// - `Ok(StaleValue<Vec<Alert>>)` with `is_stale = true` for cached data
    /// - `Err(...)` if both primary and fallback fail
    pub async fn fetch_alerts_with_fallback(
        &self,
        limit: Option<usize>,
    ) -> Result<StaleValue<Vec<Alert>>, Error> {
        // Implementation
    }
}
```

## Related Documents

- [Core Architecture](./CIRCUIT_BREAKER_CORE_ARCHITECTURE.md)
- [Integration Patterns](./CIRCUIT_BREAKER_INTEGRATION_PATTERNS.md)
- [Configuration Schema](./CIRCUIT_BREAKER_CONFIGURATION_SCHEMA.md)
- [Monitoring & Observability](./CIRCUIT_BREAKER_MONITORING.md)
