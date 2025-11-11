use crate::config::{StateBackend, StateConfig};
use crate::error::{AppError, Result};
use crate::state::{IncidentStore, InMemoryStore, RedisStore, SledStore};
use std::sync::Arc;

/// Create an incident store based on configuration
pub async fn create_store(config: &StateConfig) -> Result<Arc<dyn IncidentStore>> {
    match config.backend {
        StateBackend::Sled => {
            let path = config
                .path
                .as_ref()
                .ok_or_else(|| {
                    AppError::Configuration(
                        "Sled backend requires 'path' configuration".to_string(),
                    )
                })?;

            tracing::info!(path = ?path, "Initializing Sled storage backend");

            let store = SledStore::new(path)?;
            Ok(Arc::new(store))
        }

        StateBackend::Redis => {
            let redis_url = config.redis_url.as_ref().ok_or_else(|| {
                AppError::Configuration("Redis backend requires 'redis_url' configuration".to_string())
            })?;

            tracing::info!(url = %redis_url, "Initializing Redis storage backend");

            let store = RedisStore::new(redis_url).await?;
            Ok(Arc::new(store))
        }

        StateBackend::RedisCluster => {
            // For now, use standalone Redis with cluster URL
            // Full cluster support would require redis-cluster crate
            let redis_url = if !config.redis_cluster_nodes.is_empty() {
                &config.redis_cluster_nodes[0]
            } else {
                config.redis_url.as_ref().ok_or_else(|| {
                    AppError::Configuration(
                        "RedisCluster backend requires 'redis_cluster_nodes' or 'redis_url' configuration".to_string(),
                    )
                })?
            };

            tracing::info!(url = %redis_url, "Initializing Redis cluster storage backend");
            tracing::warn!("Redis cluster support is currently implemented as standalone connection to first node");

            let store = RedisStore::new(redis_url).await?;
            Ok(Arc::new(store))
        }

        StateBackend::Redb => {
            tracing::warn!("Redb backend is not yet implemented, falling back to in-memory storage");
            Ok(Arc::new(InMemoryStore::new()))
        }
    }
}

/// Create an in-memory store (for testing and development)
pub fn create_in_memory_store() -> Arc<dyn IncidentStore> {
    tracing::info!("Initializing in-memory storage backend");
    Arc::new(InMemoryStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_sled_store() {
        let temp_dir = TempDir::new().unwrap();
        let config = StateConfig {
            backend: StateBackend::Sled,
            path: Some(temp_dir.path().to_path_buf()),
            redis_url: None,
            redis_cluster_nodes: vec![],
            pool_size: 10,
        };

        let store = create_store(&config).await.unwrap();
        // Should be able to use the store
        assert!(store.count_incidents(&Default::default()).await.is_ok());
    }

    #[tokio::test]
    async fn test_create_in_memory_store() {
        let store = create_in_memory_store();
        // Should be able to use the store
        assert!(store.count_incidents(&Default::default()).await.is_ok());
    }

    #[tokio::test]
    async fn test_sled_requires_path() {
        let config = StateConfig {
            backend: StateBackend::Sled,
            path: None,
            redis_url: None,
            redis_cluster_nodes: vec![],
            pool_size: 10,
        };

        let result = create_store(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_redis_requires_url() {
        let config = StateConfig {
            backend: StateBackend::Redis,
            path: None,
            redis_url: None,
            redis_cluster_nodes: vec![],
            pool_size: 10,
        };

        let result = create_store(&config).await;
        assert!(result.is_err());
    }
}
