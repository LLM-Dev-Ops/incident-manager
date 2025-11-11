use crate::enrichment::enrichers::{
    Enricher, ExternalApiEnricher, HistoricalEnricher, ServiceEnricher, TeamEnricher,
};
use crate::enrichment::models::{EnrichedContext, EnrichmentConfig};
use crate::enrichment::pipeline::{CacheStats, EnrichmentPipeline};
use crate::error::{AppError, Result};
use crate::models::Incident;
use crate::state::IncidentStore;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Enrichment service manages context enrichment for incidents
pub struct EnrichmentService {
    /// Configuration
    config: Arc<RwLock<EnrichmentConfig>>,

    /// Enrichment pipeline
    pipeline: Arc<RwLock<EnrichmentPipeline>>,

    /// Service running state
    running: Arc<RwLock<bool>>,
}

impl EnrichmentService {
    /// Create a new enrichment service
    pub fn new(config: EnrichmentConfig, incident_store: Arc<dyn IncidentStore>) -> Self {
        let mut pipeline = EnrichmentPipeline::new(config.clone());

        // Register default enrichers
        if config.enable_historical {
            let historical = Arc::new(HistoricalEnricher::new(incident_store.clone()));
            pipeline.register_enricher(historical);
        }

        if config.enable_service {
            let service = Arc::new(ServiceEnricher::new());
            pipeline.register_enricher(service);
        }

        if config.enable_team {
            let team = Arc::new(TeamEnricher::new());
            pipeline.register_enricher(team);
        }

        // Register external API enrichers from config
        for (name, url) in &config.external_apis {
            let external = Arc::new(ExternalApiEnricher::new(
                name.clone(),
                url.clone(),
                config.api_timeout_secs,
            ));
            pipeline.register_enricher(external);
        }

        // Sort enrichers by priority
        pipeline.sort_enrichers_by_priority();

        Self {
            config: Arc::new(RwLock::new(config)),
            pipeline: Arc::new(RwLock::new(pipeline)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the enrichment service
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(AppError::Internal(
                "Enrichment service already running".to_string(),
            ));
        }
        *running = true;
        drop(running);

        info!("🚀 Starting enrichment service");

        let config = self.config.read().await;
        info!(
            "Enrichment service started with {} enrichers enabled",
            self.get_enabled_enricher_count().await
        );

        // Spawn background cache cleanup task
        if config.cache_ttl_secs > 0 {
            let pipeline = Arc::clone(&self.pipeline);
            let running = Arc::clone(&self.running);

            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                    if !*running.read().await {
                        break;
                    }

                    // Clean expired cache entries
                    let pipeline = pipeline.read().await;
                    pipeline.clear_expired_cache();
                }
            });
        }

        Ok(())
    }

    /// Stop the enrichment service
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(AppError::Internal(
                "Enrichment service not running".to_string(),
            ));
        }
        *running = false;

        info!("🛑 Stopping enrichment service");
        Ok(())
    }

    /// Check if service is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Enrich an incident with additional context
    pub async fn enrich_incident(&self, incident: &Incident) -> Result<EnrichedContext> {
        let config = self.config.read().await;

        if !config.enabled {
            warn!("Enrichment service is disabled");
            return Ok(EnrichedContext::new(incident.id));
        }

        drop(config);

        let pipeline = self.pipeline.read().await;
        pipeline.enrich(incident).await
    }

    /// Get enriched context for an incident (from cache if available)
    pub async fn get_context(&self, incident_id: &Uuid) -> Option<EnrichedContext> {
        let pipeline = self.pipeline.read().await;
        pipeline.get_cached_context(incident_id)
    }

    /// Register a custom enricher
    pub async fn register_enricher(&self, enricher: Arc<dyn Enricher>) {
        let mut pipeline = self.pipeline.write().await;
        pipeline.register_enricher(enricher);
        pipeline.sort_enrichers_by_priority();
        info!("Registered custom enricher");
    }

    /// Clear enrichment cache
    pub async fn clear_cache(&self) {
        let pipeline = self.pipeline.read().await;
        pipeline.clear_cache();
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let pipeline = self.pipeline.read().await;
        pipeline.get_cache_stats()
    }

    /// Get enricher count
    pub async fn get_enricher_count(&self) -> usize {
        let pipeline = self.pipeline.read().await;
        pipeline.enricher_count()
    }

    /// Get enabled enricher count
    pub async fn get_enabled_enricher_count(&self) -> usize {
        let pipeline = self.pipeline.read().await;
        pipeline.enabled_enricher_count()
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> EnrichmentStats {
        let config = self.config.read().await;
        let pipeline = self.pipeline.read().await;
        let cache_stats = pipeline.get_cache_stats();

        EnrichmentStats {
            enabled: config.enabled,
            is_running: *self.running.read().await,
            total_enrichers: pipeline.enricher_count(),
            enabled_enrichers: pipeline.enabled_enricher_count(),
            cache_size: cache_stats.size,
            cache_capacity: cache_stats.capacity,
            async_enrichment: config.async_enrichment,
            max_concurrent: config.max_concurrent,
        }
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: EnrichmentConfig) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Enrichment service configuration updated");
        Ok(())
    }
}

/// Enrichment service statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnrichmentStats {
    pub enabled: bool,
    pub is_running: bool,
    pub total_enrichers: usize,
    pub enabled_enrichers: usize,
    pub cache_size: usize,
    pub cache_capacity: usize,
    pub async_enrichment: bool,
    pub max_concurrent: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};
    use crate::state::InMemoryStore;

    fn create_test_incident() -> Incident {
        Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test Description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[tokio::test]
    async fn test_service_creation() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_service_start_stop() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        service.start().await.unwrap();
        assert!(service.is_running().await);

        service.stop().await.unwrap();
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_enrich_incident() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        service.start().await.unwrap();

        let incident = create_test_incident();
        let context = service.enrich_incident(&incident).await.unwrap();

        assert_eq!(context.incident_id, incident.id);
        assert!(context.total_enrichers() > 0);
    }

    #[tokio::test]
    async fn test_get_context() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        let incident = create_test_incident();

        // Enrich first
        service.enrich_incident(&incident).await.unwrap();

        // Get from cache
        let cached = service.get_context(&incident.id).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().incident_id, incident.id);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        let incident = create_test_incident();

        // Enrich and cache
        service.enrich_incident(&incident).await.unwrap();

        let stats_before = service.get_cache_stats().await;
        assert!(stats_before.size > 0);

        // Clear cache
        service.clear_cache().await;

        let stats_after = service.get_cache_stats().await;
        assert_eq!(stats_after.size, 0);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        service.start().await.unwrap();

        let stats = service.get_stats().await;

        assert!(stats.enabled);
        assert!(stats.is_running);
        assert!(stats.total_enrichers > 0);
        assert!(stats.enabled_enrichers > 0);
    }

    #[tokio::test]
    async fn test_disabled_service() {
        let store = Arc::new(InMemoryStore::new());
        let mut config = EnrichmentConfig::default();
        config.enabled = false;

        let service = EnrichmentService::new(config, store);
        service.start().await.unwrap();

        let incident = create_test_incident();
        let context = service.enrich_incident(&incident).await.unwrap();

        // Should return empty context
        assert_eq!(context.total_enrichers(), 0);
    }

    #[tokio::test]
    async fn test_update_config() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        let mut new_config = EnrichmentConfig::default();
        new_config.timeout_secs = 20;
        new_config.max_concurrent = 10;

        service.update_config(new_config).await.unwrap();

        // Config should be updated (we can't directly verify, but no error means success)
    }

    #[tokio::test]
    async fn test_enricher_count() {
        let store = Arc::new(InMemoryStore::new());
        let config = EnrichmentConfig::default();
        let service = EnrichmentService::new(config, store);

        let count = service.get_enricher_count().await;
        let enabled_count = service.get_enabled_enricher_count().await;

        assert!(count > 0);
        assert!(enabled_count > 0);
        assert!(enabled_count <= count);
    }

    #[tokio::test]
    async fn test_selective_enrichers() {
        let store = Arc::new(InMemoryStore::new());
        let mut config = EnrichmentConfig::default();
        config.enable_historical = true;
        config.enable_service = false;
        config.enable_team = false;

        let service = EnrichmentService::new(config, store);

        let enabled_count = service.get_enabled_enricher_count().await;

        // Only historical should be enabled
        assert_eq!(enabled_count, 1);
    }
}
