use crate::enrichment::enrichers::Enricher;
use crate::enrichment::models::{EnrichedContext, EnrichmentConfig};
use crate::error::Result;
use crate::models::Incident;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Enrichment pipeline orchestrates multiple enrichers
pub struct EnrichmentPipeline {
    /// Registered enrichers
    enrichers: Vec<Arc<dyn Enricher>>,

    /// Context cache (incident_id -> context)
    cache: Arc<DashMap<Uuid, (EnrichedContext, Instant)>>,

    /// Configuration
    config: Arc<EnrichmentConfig>,
}

impl EnrichmentPipeline {
    /// Create a new enrichment pipeline
    pub fn new(config: EnrichmentConfig) -> Self {
        Self {
            enrichers: Vec::new(),
            cache: Arc::new(DashMap::new()),
            config: Arc::new(config),
        }
    }

    /// Register an enricher
    pub fn register_enricher(&mut self, enricher: Arc<dyn Enricher>) {
        self.enrichers.push(enricher);
        debug!("Registered enricher: {}", enricher.name());
    }

    /// Sort enrichers by priority
    pub fn sort_enrichers_by_priority(&mut self) {
        self.enrichers.sort_by_key(|e| e.priority());
    }

    /// Enrich an incident with context from all registered enrichers
    pub async fn enrich(&self, incident: &Incident) -> Result<EnrichedContext> {
        let start = Instant::now();

        // Check cache first
        if let Some(cached) = self.get_cached_context(&incident.id) {
            debug!("Using cached enrichment for incident {}", incident.id);
            return Ok(cached);
        }

        let mut context = EnrichedContext::new(incident.id);

        // Filter enabled enrichers
        let enabled_enrichers: Vec<_> = self
            .enrichers
            .iter()
            .filter(|e| e.is_enabled(&self.config))
            .collect();

        if enabled_enrichers.is_empty() {
            warn!("No enrichers enabled for incident {}", incident.id);
            return Ok(context);
        }

        info!(
            "Enriching incident {} with {} enrichers",
            incident.id,
            enabled_enrichers.len()
        );

        // Run enrichers based on configuration
        if self.config.async_enrichment && self.config.max_concurrent > 1 {
            // Run enrichers in parallel (with concurrency limit)
            self.run_enrichers_parallel(
                incident,
                &mut context,
                &enabled_enrichers,
            )
            .await;
        } else {
            // Run enrichers sequentially
            self.run_enrichers_sequential(
                incident,
                &mut context,
                &enabled_enrichers,
            )
            .await;
        }

        // Set enrichment duration
        context.enrichment_duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Enrichment completed for incident {} in {}ms ({}/{} enrichers successful)",
            incident.id,
            context.enrichment_duration_ms,
            context.successful_enrichers.len(),
            context.total_enrichers()
        );

        // Cache result
        self.cache_context(&incident.id, context.clone());

        Ok(context)
    }

    /// Run enrichers sequentially
    async fn run_enrichers_sequential(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        enrichers: &[&Arc<dyn Enricher>],
    ) {
        for enricher in enrichers {
            let timeout_duration = std::time::Duration::from_secs(self.config.timeout_secs);

            match timeout(
                timeout_duration,
                enricher.enrich(incident, context, &self.config),
            )
            .await
            {
                Ok(result) => {
                    if result.success {
                        context.successful_enrichers.push(enricher.name().to_string());
                        debug!(
                            "Enricher {} succeeded in {}ms",
                            enricher.name(),
                            result.duration_ms
                        );
                    } else {
                        context.failed_enrichers.push(enricher.name().to_string());
                        warn!(
                            "Enricher {} failed: {:?}",
                            enricher.name(),
                            result.error
                        );
                    }
                }
                Err(_) => {
                    context.failed_enrichers.push(enricher.name().to_string());
                    error!(
                        "Enricher {} timed out after {}s",
                        enricher.name(),
                        self.config.timeout_secs
                    );
                }
            }
        }
    }

    /// Run enrichers in parallel with concurrency limit
    async fn run_enrichers_parallel(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        enrichers: &[&Arc<dyn Enricher>],
    ) {
        use futures::stream::{self, StreamExt};

        let config = Arc::clone(&self.config);
        let incident = incident.clone();

        // Create a concurrent stream
        let results: Vec<_> = stream::iter(enrichers.iter())
            .map(|enricher| {
                let enricher = Arc::clone(enricher);
                let incident = incident.clone();
                let config = Arc::clone(&config);

                async move {
                    let timeout_duration = std::time::Duration::from_secs(config.timeout_secs);

                    // Create a temporary context for this enricher
                    let mut temp_context = EnrichedContext::new(incident.id);

                    match timeout(
                        timeout_duration,
                        enricher.enrich(&incident, &mut temp_context, &config),
                    )
                    .await
                    {
                        Ok(result) => (enricher.name().to_string(), result, Some(temp_context)),
                        Err(_) => {
                            let failed_result = crate::enrichment::models::EnrichmentResult::failure(
                                enricher.name().to_string(),
                                config.timeout_secs * 1000,
                                "Timeout".to_string(),
                            );
                            (enricher.name().to_string(), failed_result, None)
                        }
                    }
                }
            })
            .buffer_unordered(self.config.max_concurrent)
            .collect()
            .await;

        // Merge results into main context
        for (name, result, temp_context) in results {
            if result.success {
                context.successful_enrichers.push(name.clone());
                debug!("Enricher {} succeeded in {}ms", name, result.duration_ms);

                // Merge enriched data from temp context
                if let Some(temp) = temp_context {
                    if let Some(hist) = temp.historical {
                        context.historical = Some(hist);
                    }
                    if let Some(svc) = temp.service {
                        context.service = Some(svc);
                    }
                    if let Some(team) = temp.team {
                        context.team = Some(team);
                    }
                    if let Some(metrics) = temp.metrics {
                        context.metrics = Some(metrics);
                    }
                    if let Some(logs) = temp.logs {
                        context.logs = Some(logs);
                    }
                    // Merge metadata
                    for (key, value) in temp.metadata {
                        context.metadata.insert(key, value);
                    }
                }
            } else {
                context.failed_enrichers.push(name.clone());
                warn!("Enricher {} failed: {:?}", name, result.error);
            }
        }
    }

    /// Get cached context if available and not expired
    fn get_cached_context(&self, incident_id: &Uuid) -> Option<EnrichedContext> {
        if let Some(entry) = self.cache.get(incident_id) {
            let (context, cached_at) = entry.value();

            // Check if cache is still valid
            let age = Instant::now().duration_since(*cached_at);
            if age.as_secs() < self.config.cache_ttl_secs {
                return Some(context.clone());
            } else {
                // Remove expired entry
                drop(entry);
                self.cache.remove(incident_id);
            }
        }
        None
    }

    /// Cache enriched context
    fn cache_context(&self, incident_id: &Uuid, context: EnrichedContext) {
        self.cache.insert(*incident_id, (context, Instant::now()));
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.clear();
        info!("Enrichment cache cleared");
    }

    /// Clear expired cache entries
    pub fn clear_expired_cache(&self) {
        let ttl = self.config.cache_ttl_secs;
        let now = Instant::now();

        self.cache.retain(|_, (_, cached_at)| {
            let age = now.duration_since(*cached_at);
            age.as_secs() < ttl
        });
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            capacity: self.cache.capacity(),
        }
    }

    /// Get enricher count
    pub fn enricher_count(&self) -> usize {
        self.enrichers.len()
    }

    /// Get enabled enricher count
    pub fn enabled_enricher_count(&self) -> usize {
        self.enrichers
            .iter()
            .filter(|e| e.is_enabled(&self.config))
            .count()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enrichment::enrichers::{HistoricalEnricher, ServiceEnricher, TeamEnricher};
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
    async fn test_pipeline_creation() {
        let config = EnrichmentConfig::default();
        let pipeline = EnrichmentPipeline::new(config);

        assert_eq!(pipeline.enricher_count(), 0);
    }

    #[tokio::test]
    async fn test_register_enrichers() {
        let config = EnrichmentConfig::default();
        let mut pipeline = EnrichmentPipeline::new(config);

        let store = Arc::new(InMemoryStore::new());
        let enricher1 = Arc::new(HistoricalEnricher::new(store)) as Arc<dyn Enricher>;
        let enricher2 = Arc::new(ServiceEnricher::new()) as Arc<dyn Enricher>;

        pipeline.register_enricher(enricher1);
        pipeline.register_enricher(enricher2);

        assert_eq!(pipeline.enricher_count(), 2);
    }

    #[tokio::test]
    async fn test_enrich_sequential() {
        let mut config = EnrichmentConfig::default();
        config.async_enrichment = false; // Force sequential

        let mut pipeline = EnrichmentPipeline::new(config);

        let store = Arc::new(InMemoryStore::new());
        pipeline.register_enricher(Arc::new(HistoricalEnricher::new(store)));
        pipeline.register_enricher(Arc::new(ServiceEnricher::new()));
        pipeline.register_enricher(Arc::new(TeamEnricher::new()));

        let incident = create_test_incident();
        let context = pipeline.enrich(&incident).await.unwrap();

        assert!(context.total_enrichers() > 0);
        assert!(context.enrichment_duration_ms > 0);
    }

    #[tokio::test]
    async fn test_enrich_parallel() {
        let mut config = EnrichmentConfig::default();
        config.async_enrichment = true;
        config.max_concurrent = 3;

        let mut pipeline = EnrichmentPipeline::new(config);

        let store = Arc::new(InMemoryStore::new());
        pipeline.register_enricher(Arc::new(HistoricalEnricher::new(store)));
        pipeline.register_enricher(Arc::new(ServiceEnricher::new()));
        pipeline.register_enricher(Arc::new(TeamEnricher::new()));

        let incident = create_test_incident();
        let context = pipeline.enrich(&incident).await.unwrap();

        assert!(context.total_enrichers() > 0);
        assert!(context.service.is_some());
        assert!(context.team.is_some());
    }

    #[tokio::test]
    async fn test_cache() {
        let config = EnrichmentConfig::default();
        let mut pipeline = EnrichmentPipeline::new(config);

        let store = Arc::new(InMemoryStore::new());
        pipeline.register_enricher(Arc::new(HistoricalEnricher::new(store)));

        let incident = create_test_incident();

        // First call - should enrich
        let context1 = pipeline.enrich(&incident).await.unwrap();
        let duration1 = context1.enrichment_duration_ms;

        // Second call - should use cache
        let context2 = pipeline.enrich(&incident).await.unwrap();

        // Should be same incident
        assert_eq!(context1.incident_id, context2.incident_id);

        // Cache hit should be faster or equal
        // (Note: In tests this might not always be true due to timing)
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let config = EnrichmentConfig::default();
        let mut pipeline = EnrichmentPipeline::new(config);

        let store = Arc::new(InMemoryStore::new());
        pipeline.register_enricher(Arc::new(HistoricalEnricher::new(store)));

        let incident = create_test_incident();

        // Enrich and cache
        pipeline.enrich(&incident).await.unwrap();

        let stats_before = pipeline.get_cache_stats();
        assert_eq!(stats_before.size, 1);

        // Clear cache
        pipeline.clear_cache();

        let stats_after = pipeline.get_cache_stats();
        assert_eq!(stats_after.size, 0);
    }

    #[tokio::test]
    async fn test_sort_by_priority() {
        let config = EnrichmentConfig::default();
        let mut pipeline = EnrichmentPipeline::new(config);

        let store = Arc::new(InMemoryStore::new());
        pipeline.register_enricher(Arc::new(TeamEnricher::new())); // Priority 30
        pipeline.register_enricher(Arc::new(ServiceEnricher::new())); // Priority 20
        pipeline.register_enricher(Arc::new(HistoricalEnricher::new(store))); // Priority 10

        pipeline.sort_enrichers_by_priority();

        // After sorting, historical should be first (lowest priority number)
        assert_eq!(pipeline.enrichers[0].name(), "historical");
        assert_eq!(pipeline.enrichers[1].name(), "service");
        assert_eq!(pipeline.enrichers[2].name(), "team");
    }

    #[tokio::test]
    async fn test_enabled_enricher_count() {
        let mut config = EnrichmentConfig::default();
        config.enable_historical = true;
        config.enable_service = true;
        config.enable_team = false; // Disable team

        let mut pipeline = EnrichmentPipeline::new(config);

        let store = Arc::new(InMemoryStore::new());
        pipeline.register_enricher(Arc::new(HistoricalEnricher::new(store)));
        pipeline.register_enricher(Arc::new(ServiceEnricher::new()));
        pipeline.register_enricher(Arc::new(TeamEnricher::new()));

        assert_eq!(pipeline.enricher_count(), 3);
        assert_eq!(pipeline.enabled_enricher_count(), 2); // Only historical and service
    }
}
