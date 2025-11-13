use crate::correlation::models::{
    Correlation, CorrelationConfig, CorrelationGroup, CorrelationResult, CorrelationType,
    GroupStatus,
};
use crate::correlation::strategy::{
    CombinedStrategy, CorrelationStrategy, FingerprintStrategy, PatternStrategy, SourceStrategy,
    TemporalStrategy, TopologyStrategy,
};
use crate::error::{AppError, Result};
use crate::models::Incident;
use crate::state::IncidentStore;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Main correlation engine that coordinates all correlation activities
pub struct CorrelationEngine {
    /// Configuration
    config: Arc<RwLock<CorrelationConfig>>,

    /// Active correlation groups (group_id -> CorrelationGroup)
    groups: Arc<DashMap<Uuid, CorrelationGroup>>,

    /// Incident to group mapping (incident_id -> group_id)
    incident_to_group: Arc<DashMap<Uuid, Uuid>>,

    /// All correlations detected (correlation_id -> Correlation)
    correlations: Arc<DashMap<Uuid, Correlation>>,

    /// Correlation strategies
    strategies: Vec<Box<dyn CorrelationStrategy>>,

    /// Reference to incident store for fetching incidents
    incident_store: Arc<dyn IncidentStore>,

    /// Engine running state
    running: Arc<RwLock<bool>>,
}

impl CorrelationEngine {
    /// Create a new correlation engine
    pub fn new(config: CorrelationConfig, incident_store: Arc<dyn IncidentStore>) -> Self {
        let strategies = Self::create_strategies(&config);

        Self {
            config: Arc::new(RwLock::new(config)),
            groups: Arc::new(DashMap::new()),
            incident_to_group: Arc::new(DashMap::new()),
            correlations: Arc::new(DashMap::new()),
            strategies,
            incident_store,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Create correlation strategies based on configuration
    fn create_strategies(config: &CorrelationConfig) -> Vec<Box<dyn CorrelationStrategy>> {
        let mut strategies: Vec<Box<dyn CorrelationStrategy>> = Vec::new();

        if config.enable_temporal {
            strategies.push(Box::new(TemporalStrategy::new()));
        }

        if config.enable_pattern {
            strategies.push(Box::new(PatternStrategy::new()));
        }

        if config.enable_source {
            strategies.push(Box::new(SourceStrategy::new()));
        }

        if config.enable_fingerprint {
            strategies.push(Box::new(FingerprintStrategy::new()));
        }

        if config.enable_topology {
            strategies.push(Box::new(TopologyStrategy::new()));
        }

        // If multiple strategies are enabled, add combined strategy
        if strategies.len() > 1 {
            // Create Arc wrappers for the existing strategies
            let arc_strategies: Vec<Arc<dyn CorrelationStrategy>> = vec![
                Arc::new(TemporalStrategy::new()),
                Arc::new(PatternStrategy::new()),
                Arc::new(SourceStrategy::new()),
                Arc::new(FingerprintStrategy::new()),
                Arc::new(TopologyStrategy::new()),
            ];
            let combined = CombinedStrategy::new(arc_strategies);
            strategies.push(Box::new(combined));
        }

        strategies
    }

    /// Start the correlation engine with background monitoring
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(AppError::Internal(
                "Correlation engine already running".to_string(),
            ));
        }
        *running = true;
        drop(running);

        info!("🚀 Starting correlation engine");

        // Spawn background monitor task
        let engine = Arc::new(self.clone_for_task());
        tokio::spawn(async move {
            if let Err(e) = engine.monitor_loop().await {
                error!("Correlation engine monitor loop failed: {}", e);
            }
        });

        Ok(())
    }

    /// Stop the correlation engine
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(AppError::Internal(
                "Correlation engine not running".to_string(),
            ));
        }
        *running = false;

        info!("🛑 Stopping correlation engine");
        Ok(())
    }

    /// Check if engine is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Analyze a new incident for correlations with existing incidents
    pub async fn analyze_incident(&self, incident: &Incident) -> Result<CorrelationResult> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(CorrelationResult::empty());
        }

        let start = Instant::now();
        let mut result = CorrelationResult::empty();

        // Check if incident already belongs to a group
        if self.incident_to_group.contains_key(&incident.id) {
            debug!(
                "Incident {} already in correlation group",
                incident.id
            );
            return Ok(result);
        }

        // Fetch recent incidents for correlation (within temporal window)
        let candidates = self.get_correlation_candidates(incident, &config).await?;

        debug!(
            "Found {} candidate incidents for correlation with {}",
            candidates.len(),
            incident.id
        );

        // Try each strategy to find correlations
        for candidate in &candidates {
            for strategy in &self.strategies {
                match strategy.correlate(incident, candidate, &config).await {
                    Ok(Some(correlation)) => {
                        if correlation.score >= config.min_correlation_score {
                            debug!(
                                "Correlation detected: {} <-> {} (strategy: {}, score: {:.2})",
                                incident.id,
                                candidate.id,
                                strategy.name(),
                                correlation.score
                            );
                            result.correlations.push(correlation);
                        }
                    }
                    Ok(None) => {
                        // No correlation found by this strategy
                    }
                    Err(e) => {
                        warn!(
                            "Strategy {} failed for incidents {} and {}: {}",
                            strategy.name(),
                            incident.id,
                            candidate.id,
                            e
                        );
                    }
                }
            }
        }

        // Process correlations: update or create groups
        if !result.correlations.is_empty() {
            self.process_correlations(incident, &mut result).await?;
        }

        result.processing_time_ms = start.elapsed().as_millis() as u64;

        info!(
            "Incident {} correlation analysis: {} correlations, {} groups affected, {} groups created ({}ms)",
            incident.id,
            result.correlations.len(),
            result.groups_affected.len(),
            result.groups_created.len(),
            result.processing_time_ms
        );

        Ok(result)
    }

    /// Get candidate incidents for correlation analysis
    async fn get_correlation_candidates(
        &self,
        incident: &Incident,
        _config: &CorrelationConfig,
    ) -> Result<Vec<Incident>> {
        // For now, use a simple filter based on temporal window
        // In production, this should use more sophisticated queries
        let filter = crate::state::IncidentFilter {
            severities: Vec::new(),
            states: Vec::new(),
            sources: Vec::new(),
            active_only: false,
        };

        // Fetch incidents
        let candidates = self
            .incident_store
            .list_incidents(&filter, 0, 1000)
            .await?;

        // Filter out the incident itself
        let candidates: Vec<Incident> = candidates
            .into_iter()
            .filter(|c| c.id != incident.id)
            .collect();

        Ok(candidates)
    }

    /// Process detected correlations and update/create groups
    async fn process_correlations(
        &self,
        incident: &Incident,
        result: &mut CorrelationResult,
    ) -> Result<()> {
        let config = self.config.read().await;

        // Store all correlations
        for correlation in &result.correlations {
            self.correlations.insert(correlation.id, correlation.clone());
        }

        // Find if any correlated incidents already belong to groups
        let mut existing_groups: Vec<Uuid> = Vec::new();

        for correlation in &result.correlations {
            for incident_id in &correlation.incident_ids {
                if let Some(group_id) = self.incident_to_group.get(incident_id) {
                    if !existing_groups.contains(&*group_id) {
                        existing_groups.push(*group_id);
                    }
                }
            }
        }

        if existing_groups.is_empty() {
            // No existing groups - create a new one
            let group = self.create_group(incident, result.correlations.clone()).await?;
            result.groups_created.push(group.clone());
            result.groups_affected.push(group.id);
        } else if existing_groups.len() == 1 {
            // Add to existing group
            let group_id = existing_groups[0];
            self.add_to_group(group_id, incident, result.correlations.clone())
                .await?;
            result.groups_affected.push(group_id);
        } else {
            // Multiple groups - merge if auto-merge enabled
            if config.auto_merge_groups {
                let merged_group = self
                    .merge_groups(&existing_groups, incident, result.correlations.clone())
                    .await?;
                result.groups_created.push(merged_group.clone());
                result.groups_affected.push(merged_group.id);
            } else {
                // Add to the largest group
                let largest_group_id = self.find_largest_group(&existing_groups);
                self.add_to_group(largest_group_id, incident, result.correlations.clone())
                    .await?;
                result.groups_affected.push(largest_group_id);
            }
        }

        Ok(())
    }

    /// Create a new correlation group
    async fn create_group(
        &self,
        primary_incident: &Incident,
        correlations: Vec<Correlation>,
    ) -> Result<CorrelationGroup> {
        let mut group = CorrelationGroup::new(primary_incident);

        // Add all correlated incidents
        for correlation in correlations {
            for incident_id in &correlation.incident_ids {
                if *incident_id != primary_incident.id {
                    group.add_incident(*incident_id, correlation.clone());
                    self.incident_to_group.insert(*incident_id, group.id);
                }
            }
        }

        // Map primary incident to group
        self.incident_to_group
            .insert(primary_incident.id, group.id);

        info!(
            "Created correlation group {} with {} incidents",
            group.id,
            group.size()
        );

        self.groups.insert(group.id, group.clone());
        Ok(group)
    }

    /// Add incident to existing group
    async fn add_to_group(
        &self,
        group_id: Uuid,
        incident: &Incident,
        correlations: Vec<Correlation>,
    ) -> Result<()> {
        if let Some(mut group) = self.groups.get_mut(&group_id) {
            for correlation in correlations {
                group.add_incident(incident.id, correlation);
            }

            self.incident_to_group.insert(incident.id, group_id);

            info!(
                "Added incident {} to correlation group {} (size: {})",
                incident.id,
                group_id,
                group.size()
            );

            Ok(())
        } else {
            Err(AppError::NotFound(format!(
                "Correlation group {} not found",
                group_id
            )))
        }
    }

    /// Merge multiple groups into one
    async fn merge_groups(
        &self,
        group_ids: &[Uuid],
        new_incident: &Incident,
        correlations: Vec<Correlation>,
    ) -> Result<CorrelationGroup> {
        // Find the largest group to use as base
        let base_group_id = self.find_largest_group(group_ids);

        let mut merged_group = if let Some(group) = self.groups.get(&base_group_id) {
            group.clone()
        } else {
            return Err(AppError::NotFound(format!(
                "Base group {} not found",
                base_group_id
            )));
        };

        // Merge other groups into the base
        for group_id in group_ids {
            if *group_id == base_group_id {
                continue;
            }

            if let Some(group) = self.groups.get(group_id) {
                // Add all incidents from this group to merged group
                for incident_id in group.all_incident_ids() {
                    if !merged_group.contains_incident(&incident_id) {
                        // Create a merge correlation
                        let merge_correlation = Correlation::new(
                            vec![incident_id, merged_group.primary_incident_id],
                            group.aggregate_score,
                            CorrelationType::Combined,
                            format!("Merged from group {}", group_id),
                        );
                        merged_group.add_incident(incident_id, merge_correlation);
                        self.incident_to_group.insert(incident_id, merged_group.id);
                    }
                }

                // Remove the old group
                self.groups.remove(group_id);
            }
        }

        // Add the new incident
        for correlation in correlations {
            merged_group.add_incident(new_incident.id, correlation);
        }
        self.incident_to_group
            .insert(new_incident.id, merged_group.id);

        info!(
            "Merged {} groups into group {} (size: {})",
            group_ids.len(),
            merged_group.id,
            merged_group.size()
        );

        self.groups.insert(merged_group.id, merged_group.clone());
        Ok(merged_group)
    }

    /// Find the largest group among the given group IDs
    fn find_largest_group(&self, group_ids: &[Uuid]) -> Uuid {
        let mut largest_id = group_ids[0];
        let mut largest_size = 0;

        for group_id in group_ids {
            if let Some(group) = self.groups.get(group_id) {
                if group.size() > largest_size {
                    largest_size = group.size();
                    largest_id = *group_id;
                }
            }
        }

        largest_id
    }

    /// Background monitor loop for periodic correlation analysis
    async fn monitor_loop(&self) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(60)); // Check every minute

        loop {
            ticker.tick().await;

            if !self.is_running().await {
                info!("Correlation engine monitor loop stopping");
                break;
            }

            // Perform periodic maintenance
            if let Err(e) = self.perform_maintenance().await {
                error!("Correlation maintenance failed: {}", e);
            }
        }

        Ok(())
    }

    /// Perform periodic maintenance tasks
    async fn perform_maintenance(&self) -> Result<()> {
        debug!("Performing correlation maintenance");

        let _config = self.config.read().await;

        // Stabilize old groups
        for mut entry in self.groups.iter_mut() {
            let group = entry.value_mut();

            if group.status == GroupStatus::Active {
                let age = chrono::Utc::now()
                    .signed_duration_since(group.updated_at)
                    .num_seconds();

                // Stabilize groups that haven't changed in 1 hour
                if age > 3600 {
                    group.stabilize();
                    debug!("Stabilized correlation group {}", group.id);
                }
            }
        }

        // Clean up old resolved groups (optional)
        let resolved_groups: Vec<Uuid> = self
            .groups
            .iter()
            .filter(|entry| {
                entry.value().status == GroupStatus::Resolved
                    && chrono::Utc::now()
                        .signed_duration_since(entry.value().updated_at)
                        .num_days()
                        > 7
            })
            .map(|entry| entry.key().clone())
            .collect();

        for group_id in resolved_groups {
            if let Some((_, group)) = self.groups.remove(&group_id) {
                // Remove incident mappings
                for incident_id in group.all_incident_ids() {
                    self.incident_to_group.remove(&incident_id);
                }
                info!("Cleaned up old resolved group {}", group_id);
            }
        }

        Ok(())
    }

    /// Get a correlation group by ID
    pub fn get_group(&self, group_id: &Uuid) -> Option<CorrelationGroup> {
        self.groups.get(group_id).map(|g| g.clone())
    }

    /// Get all active correlation groups
    pub fn get_active_groups(&self) -> Vec<CorrelationGroup> {
        self.groups
            .iter()
            .filter(|entry| entry.value().status == GroupStatus::Active)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get correlation group for an incident
    pub fn get_group_for_incident(&self, incident_id: &Uuid) -> Option<CorrelationGroup> {
        if let Some(group_id) = self.incident_to_group.get(incident_id) {
            self.get_group(&group_id)
        } else {
            None
        }
    }

    /// Get all correlations for an incident
    pub fn get_correlations_for_incident(&self, incident_id: &Uuid) -> Vec<Correlation> {
        self.correlations
            .iter()
            .filter(|entry| entry.value().involves_incident(incident_id))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Manually create a correlation between incidents
    pub async fn manual_correlate(
        &self,
        incident_ids: Vec<Uuid>,
        reason: String,
    ) -> Result<Correlation> {
        let _config = self.config.read().await;

        let correlation = Correlation::new(
            incident_ids.clone(),
            1.0, // Manual correlations have perfect score
            CorrelationType::Manual,
            reason,
        );

        // Store correlation
        self.correlations.insert(correlation.id, correlation.clone());

        info!(
            "Manual correlation created: {} incidents",
            incident_ids.len()
        );

        Ok(correlation)
    }

    /// Resolve a correlation group
    pub async fn resolve_group(&self, group_id: &Uuid) -> Result<()> {
        if let Some(mut group) = self.groups.get_mut(group_id) {
            group.resolve();
            info!("Resolved correlation group {}", group_id);
            Ok(())
        } else {
            Err(AppError::NotFound(format!(
                "Correlation group {} not found",
                group_id
            )))
        }
    }

    /// Update correlation configuration
    pub async fn update_config(&self, new_config: CorrelationConfig) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Updated correlation configuration");
        Ok(())
    }

    /// Get correlation statistics
    pub fn get_stats(&self) -> CorrelationStats {
        let total_groups = self.groups.len();
        let active_groups = self
            .groups
            .iter()
            .filter(|e| e.value().status == GroupStatus::Active)
            .count();
        let total_correlations = self.correlations.len();
        let total_mapped_incidents = self.incident_to_group.len();

        CorrelationStats {
            total_groups,
            active_groups,
            total_correlations,
            total_mapped_incidents,
        }
    }

    /// Helper to clone engine for background tasks
    fn clone_for_task(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            groups: Arc::clone(&self.groups),
            incident_to_group: Arc::clone(&self.incident_to_group),
            correlations: Arc::clone(&self.correlations),
            strategies: vec![], // Background task doesn't need strategies
            incident_store: Arc::clone(&self.incident_store),
            running: Arc::clone(&self.running),
        }
    }
}

/// Statistics about the correlation engine
#[derive(Debug, Clone)]
pub struct CorrelationStats {
    pub total_groups: usize,
    pub active_groups: usize,
    pub total_correlations: usize,
    pub total_mapped_incidents: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};
    use crate::state::InMemoryStore;

    fn create_test_incident(source: &str, title: &str, desc: &str) -> Incident {
        Incident::new(
            source.to_string(),
            title.to_string(),
            desc.to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        assert!(!engine.is_running().await);
        assert_eq!(engine.groups.len(), 0);
    }

    #[tokio::test]
    async fn test_engine_start_stop() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        assert!(engine.start().await.is_ok());
        assert!(engine.is_running().await);

        assert!(engine.stop().await.is_ok());
        assert!(!engine.is_running().await);
    }

    #[tokio::test]
    async fn test_analyze_incident_no_candidates() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        let incident = create_test_incident("test", "Test Incident", "Description");

        let result = engine.analyze_incident(&incident).await.unwrap();
        assert_eq!(result.correlations.len(), 0);
    }

    #[tokio::test]
    async fn test_create_group() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        let incident1 = create_test_incident("test", "Test Incident", "Description");
        let incident2 = create_test_incident("test", "Test Incident 2", "Description 2");

        let correlation = Correlation::new(
            vec![incident1.id, incident2.id],
            0.8,
            CorrelationType::Temporal,
            "Close in time".to_string(),
        );

        let group = engine
            .create_group(&incident1, vec![correlation])
            .await
            .unwrap();

        assert_eq!(group.size(), 2);
        assert!(group.contains_incident(&incident1.id));
        assert!(group.contains_incident(&incident2.id));
    }

    #[tokio::test]
    async fn test_get_group_for_incident() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        let incident = create_test_incident("test", "Test Incident", "Description");

        let correlation = Correlation::new(
            vec![incident.id],
            0.8,
            CorrelationType::Temporal,
            "Test".to_string(),
        );

        let group = engine.create_group(&incident, vec![correlation]).await.unwrap();

        let retrieved_group = engine.get_group_for_incident(&incident.id).unwrap();
        assert_eq!(retrieved_group.id, group.id);
    }

    #[tokio::test]
    async fn test_manual_correlate() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        let incident1 = create_test_incident("test", "Test 1", "Desc 1");
        let incident2 = create_test_incident("test", "Test 2", "Desc 2");

        let correlation = engine
            .manual_correlate(
                vec![incident1.id, incident2.id],
                "Manually linked".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(correlation.score, 1.0);
        assert_eq!(correlation.correlation_type, CorrelationType::Manual);
        assert_eq!(correlation.incident_ids.len(), 2);
    }

    #[tokio::test]
    async fn test_resolve_group() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        let incident = create_test_incident("test", "Test", "Desc");
        let correlation = Correlation::new(
            vec![incident.id],
            0.8,
            CorrelationType::Temporal,
            "Test".to_string(),
        );

        let group = engine.create_group(&incident, vec![correlation]).await.unwrap();

        engine.resolve_group(&group.id).await.unwrap();

        let resolved_group = engine.get_group(&group.id).unwrap();
        assert_eq!(resolved_group.status, GroupStatus::Resolved);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        let incident = create_test_incident("test", "Test", "Desc");
        let correlation = Correlation::new(
            vec![incident.id],
            0.8,
            CorrelationType::Temporal,
            "Test".to_string(),
        );

        engine.create_group(&incident, vec![correlation]).await.unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_groups, 1);
        assert_eq!(stats.active_groups, 1);
    }

    #[tokio::test]
    async fn test_merge_groups() {
        let config = CorrelationConfig::default();
        let store = Arc::new(InMemoryStore::new());
        let engine = CorrelationEngine::new(config, store);

        let incident1 = create_test_incident("test", "Test 1", "Desc 1");
        let incident2 = create_test_incident("test", "Test 2", "Desc 2");
        let incident3 = create_test_incident("test", "Test 3", "Desc 3");

        let corr1 = Correlation::new(
            vec![incident1.id],
            0.8,
            CorrelationType::Temporal,
            "Test".to_string(),
        );
        let corr2 = Correlation::new(
            vec![incident2.id],
            0.8,
            CorrelationType::Temporal,
            "Test".to_string(),
        );

        let group1 = engine.create_group(&incident1, vec![corr1]).await.unwrap();
        let group2 = engine.create_group(&incident2, vec![corr2]).await.unwrap();

        let corr3 = Correlation::new(
            vec![incident3.id, incident1.id, incident2.id],
            0.9,
            CorrelationType::Combined,
            "Merge test".to_string(),
        );

        let merged = engine
            .merge_groups(&[group1.id, group2.id], &incident3, vec![corr3])
            .await
            .unwrap();

        assert_eq!(merged.size(), 3);
        assert!(merged.contains_incident(&incident1.id));
        assert!(merged.contains_incident(&incident2.id));
        assert!(merged.contains_incident(&incident3.id));
    }
}
