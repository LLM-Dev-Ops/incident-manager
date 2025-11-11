use moka::future::Cache;
use std::hash::Hash;
use std::time::Duration;

/// Generic cache wrapper using Moka
#[derive(Clone)]
pub struct AppCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    cache: Cache<K, V>,
}

impl<K, V> AppCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();

        Self { cache }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        self.cache.get(key).await
    }

    pub async fn insert(&self, key: K, value: V) {
        self.cache.insert(key, value).await;
    }

    pub async fn invalidate(&self, key: &K) {
        self.cache.invalidate(key).await;
    }

    pub async fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }

    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = AppCache::new(100, Duration::from_secs(60));

        cache.insert("key1".to_string(), "value1".to_string()).await;

        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));

        cache.invalidate(&"key1".to_string()).await;
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache = AppCache::new(100, Duration::from_millis(100));

        cache.insert("key".to_string(), "value".to_string()).await;

        // Value should be present immediately
        assert!(cache.get(&"key".to_string()).await.is_some());

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Value should be expired
        assert!(cache.get(&"key".to_string()).await.is_none());
    }
}
