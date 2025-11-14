use crate::enrichment::models::{
    EnrichedContext, EnrichmentConfig, EnrichmentResult, HistoricalContext, OnCallEngineer,
    ServiceContext, ServiceDependency, ServiceStatus, SimilarIncident, TeamContext,
};
use crate::error::{AppError, Result};
use crate::models::Incident;
use crate::state::{IncidentFilter, IncidentStore};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tracing::warn;

/// Trait for context enrichers
#[async_trait]
pub trait Enricher: Send + Sync + 'static {
    /// Get enricher name
    fn name(&self) -> &str;

    /// Enrich incident with additional context
    async fn enrich(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        config: &EnrichmentConfig,
    ) -> EnrichmentResult;

    /// Check if enricher is enabled for given config
    fn is_enabled(&self, config: &EnrichmentConfig) -> bool;

    /// Get enricher priority (lower = higher priority)
    fn priority(&self) -> u32 {
        100
    }
}

/// Historical enricher - enriches with similar past incidents
pub struct HistoricalEnricher {
    incident_store: Arc<dyn IncidentStore>,
}

impl HistoricalEnricher {
    pub fn new(incident_store: Arc<dyn IncidentStore>) -> Self {
        Self { incident_store }
    }

    /// Calculate similarity between two incidents
    fn calculate_similarity(incident1: &Incident, incident2: &Incident) -> f64 {
        let mut score = 0.0;
        let mut components = 0.0;

        // Title similarity (Jaccard)
        let title_sim = Self::jaccard_similarity(&incident1.title, &incident2.title);
        score += title_sim * 0.4;
        components += 0.4;

        // Description similarity
        let desc_sim = Self::jaccard_similarity(&incident1.description, &incident2.description);
        score += desc_sim * 0.3;
        components += 0.3;

        // Source match
        if incident1.source == incident2.source {
            score += 0.15;
        }
        components += 0.15;

        // Type match
        if incident1.incident_type == incident2.incident_type {
            score += 0.15;
        }
        components += 0.15;

        score / components
    }

    fn jaccard_similarity(s1: &str, s2: &str) -> f64 {
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();
        let words1: std::collections::HashSet<_> =
            s1_lower.split_whitespace().collect();
        let words2: std::collections::HashSet<_> =
            s2_lower.split_whitespace().collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }
}

#[async_trait]
impl Enricher for HistoricalEnricher {
    fn name(&self) -> &str {
        "historical"
    }

    async fn enrich(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        config: &EnrichmentConfig,
    ) -> EnrichmentResult {
        let start = Instant::now();

        // Fetch historical incidents
        let lookback = chrono::Duration::seconds(config.historical_lookback_secs as i64);
        let cutoff = chrono::Utc::now() - lookback;

        // IncidentFilter only has: states, severities, sources, active_only
        let filter = IncidentFilter::default();

        let historical_incidents = match self.incident_store.list_incidents(&filter, 0, 1000).await
        {
            Ok(incidents) => incidents,
            Err(e) => {
                return EnrichmentResult::failure(
                    self.name().to_string(),
                    start.elapsed().as_millis() as u64,
                    format!("Failed to fetch historical incidents: {}", e),
                );
            }
        };

        // Calculate similarities
        let mut similar: Vec<(Incident, f64)> = historical_incidents
            .into_iter()
            .filter(|hist| hist.id != incident.id && hist.created_at >= cutoff) // Exclude self and filter by cutoff
            .map(|hist| {
                let similarity = Self::calculate_similarity(incident, &hist);
                (hist, similarity)
            })
            .filter(|(_, sim)| *sim >= config.similarity_threshold)
            .collect();

        // Sort by similarity (highest first)
        similar.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Take top 10
        similar.truncate(10);

        // Build similar incidents
        let similar_incidents: Vec<SimilarIncident> = similar
            .into_iter()
            .map(|(inc, score)| {
                let resolution_time = inc.resolution.as_ref()
                    .map(|r| (r.resolved_at - inc.created_at).num_seconds() as u64);

                SimilarIncident {
                    incident_id: inc.id,
                    similarity_score: score,
                    title: inc.title.clone(),
                    resolution: inc.resolution.as_ref().map(|r| r.notes.clone()),
                    resolution_time,
                    occurred_at: inc.created_at,
                }
            })
            .collect();

        // Calculate statistics
        let avg_resolution_time = if !similar_incidents.is_empty() {
            let total: u64 = similar_incidents
                .iter()
                .filter_map(|s| s.resolution_time)
                .sum();
            let count = similar_incidents
                .iter()
                .filter(|s| s.resolution_time.is_some())
                .count();
            if count > 0 {
                Some(total / count as u64)
            } else {
                None
            }
        } else {
            None
        };

        let last_occurrence = similar_incidents
            .first()
            .map(|s| s.occurred_at);

        let recurrence_rate = if similar_incidents.len() > 1 {
            similar_incidents.len() as f64 / 10.0 // Normalized to top 10
        } else {
            0.0
        };

        context.historical = Some(HistoricalContext {
            similar_incidents,
            common_patterns: vec![], // TODO: Extract patterns
            avg_resolution_time,
            common_resolution_method: None, // TODO: Determine common method
            recurrence_rate,
            last_occurrence,
        });

        EnrichmentResult::success(self.name().to_string(), start.elapsed().as_millis() as u64)
    }

    fn is_enabled(&self, config: &EnrichmentConfig) -> bool {
        config.enable_historical
    }

    fn priority(&self) -> u32 {
        10 // High priority
    }
}

/// Service enricher - enriches with service catalog data
pub struct ServiceEnricher {
    // In production, this would connect to a service catalog API/CMDB
    // For now, we'll use mock data
}

impl ServiceEnricher {
    pub fn new() -> Self {
        Self {}
    }

    /// Mock service lookup (in production, this would call external API)
    async fn lookup_service(&self, incident: &Incident) -> Option<ServiceContext> {
        // Extract service name from source or title
        let service_name = Self::extract_service_name(incident);

        if service_name.is_empty() {
            return None;
        }

        // Mock service data
        Some(ServiceContext {
            service_name: service_name.clone(),
            service_id: Some(format!("svc-{}", &service_name[..4.min(service_name.len())])),
            service_status: ServiceStatus::Healthy, // Would be fetched from monitoring
            owner: Some("platform-team".to_string()),
            tier: Some("P1".to_string()),
            dependencies: vec![
                ServiceDependency {
                    service_name: "database".to_string(),
                    dependency_type: crate::enrichment::models::DependencyType::Database,
                    status: ServiceStatus::Healthy,
                },
            ],
            recent_changes: vec![],
            health_score: Some(0.95),
            sla_target: Some(300), // 5 minutes
            service_url: Some(format!("https://service-catalog.example.com/{}", service_name)),
        })
    }

    fn extract_service_name(incident: &Incident) -> String {
        // Try to extract from source first
        if !incident.source.is_empty() {
            return incident.source.clone();
        }

        // Try to extract from title (look for patterns like "service-name:")
        let title_lower = incident.title.to_lowercase();
        for word in title_lower.split_whitespace() {
            if word.contains("service") || word.contains("api") || word.contains("db") {
                return word.to_string();
            }
        }

        "unknown".to_string()
    }
}

#[async_trait]
impl Enricher for ServiceEnricher {
    fn name(&self) -> &str {
        "service"
    }

    async fn enrich(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        _config: &EnrichmentConfig,
    ) -> EnrichmentResult {
        let start = Instant::now();

        match self.lookup_service(incident).await {
            Some(service_context) => {
                context.service = Some(service_context);
                EnrichmentResult::success(self.name().to_string(), start.elapsed().as_millis() as u64)
            }
            None => {
                EnrichmentResult::failure(
                    self.name().to_string(),
                    start.elapsed().as_millis() as u64,
                    "Service not found".to_string(),
                )
            }
        }
    }

    fn is_enabled(&self, config: &EnrichmentConfig) -> bool {
        config.enable_service
    }

    fn priority(&self) -> u32 {
        20
    }
}

/// Team enricher - enriches with team and on-call information
pub struct TeamEnricher {
    // In production, would connect to PagerDuty, Opsgenie, or similar
}

impl TeamEnricher {
    pub fn new() -> Self {
        Self {}
    }

    /// Mock team lookup
    async fn lookup_team(&self, incident: &Incident) -> Option<TeamContext> {
        let team_name = Self::determine_team(incident);

        // Mock team data
        Some(TeamContext {
            primary_team: team_name.clone(),
            on_call: vec![
                OnCallEngineer {
                    name: "Alice Johnson".to_string(),
                    email: "alice@example.com".to_string(),
                    phone: Some("+1-555-0100".to_string()),
                    role: "Primary".to_string(),
                    shift_start: chrono::Utc::now() - chrono::Duration::hours(2),
                    shift_end: chrono::Utc::now() + chrono::Duration::hours(6),
                },
                OnCallEngineer {
                    name: "Bob Smith".to_string(),
                    email: "bob@example.com".to_string(),
                    phone: Some("+1-555-0101".to_string()),
                    role: "Secondary".to_string(),
                    shift_start: chrono::Utc::now() - chrono::Duration::hours(2),
                    shift_end: chrono::Utc::now() + chrono::Duration::hours(6),
                },
            ],
            expertise: vec!["kubernetes".to_string(), "databases".to_string()],
            timezone: Some("America/Los_Angeles".to_string()),
            slack_channel: Some(format!("#{}-incidents", team_name)),
            escalation_policy: Some("follow-the-sun".to_string()),
            avg_response_time: Some(180), // 3 minutes
        })
    }

    fn determine_team(incident: &Incident) -> String {
        // Simple heuristic - in production, would use service catalog mapping
        match incident.incident_type {
            crate::models::IncidentType::Infrastructure => "platform-team".to_string(),
            crate::models::IncidentType::Application => "app-team".to_string(),
            crate::models::IncidentType::Security => "security-team".to_string(),
            crate::models::IncidentType::Performance => "sre-team".to_string(),
            crate::models::IncidentType::Data => "data-team".to_string(),
            crate::models::IncidentType::Availability => "sre-team".to_string(),
            crate::models::IncidentType::Compliance => "compliance-team".to_string(),
            crate::models::IncidentType::Unknown => "ops-team".to_string(),
        }
    }
}

#[async_trait]
impl Enricher for TeamEnricher {
    fn name(&self) -> &str {
        "team"
    }

    async fn enrich(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        _config: &EnrichmentConfig,
    ) -> EnrichmentResult {
        let start = Instant::now();

        match self.lookup_team(incident).await {
            Some(team_context) => {
                context.team = Some(team_context);
                EnrichmentResult::success(self.name().to_string(), start.elapsed().as_millis() as u64)
            }
            None => {
                EnrichmentResult::failure(
                    self.name().to_string(),
                    start.elapsed().as_millis() as u64,
                    "Team not found".to_string(),
                )
            }
        }
    }

    fn is_enabled(&self, config: &EnrichmentConfig) -> bool {
        config.enable_team
    }

    fn priority(&self) -> u32 {
        30
    }
}

/// External API enricher base
pub struct ExternalApiEnricher {
    name: String,
    api_url: String,
    timeout_secs: u64,
}

impl ExternalApiEnricher {
    pub fn new(name: String, api_url: String, timeout_secs: u64) -> Self {
        Self {
            name,
            api_url,
            timeout_secs,
        }
    }

    /// Make HTTP request to external API
    async fn call_api(&self, endpoint: &str) -> Result<serde_json::Value> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {}", e)))?;

        let url = format!("{}/{}", self.api_url, endpoint);

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!(
                "API returned error: {}",
                response.status()
            )));
        }

        let body = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse response: {}", e)))?;

        Ok(body)
    }
}

#[async_trait]
impl Enricher for ExternalApiEnricher {
    fn name(&self) -> &str {
        &self.name
    }

    async fn enrich(
        &self,
        incident: &Incident,
        context: &mut EnrichedContext,
        _config: &EnrichmentConfig,
    ) -> EnrichmentResult {
        let start = Instant::now();

        // Example: fetch additional data from external API
        let endpoint = format!("incidents/{}", incident.id);

        match self.call_api(&endpoint).await {
            Ok(data) => {
                // Store in metadata
                if let Some(obj) = data.as_object() {
                    for (key, value) in obj {
                        context.add_metadata(
                            format!("external_{}", key),
                            value.to_string(),
                        );
                    }
                }
                EnrichmentResult::success(self.name().to_string(), start.elapsed().as_millis() as u64)
            }
            Err(e) => {
                warn!("External API enricher failed: {}", e);
                EnrichmentResult::failure(
                    self.name().to_string(),
                    start.elapsed().as_millis() as u64,
                    e.to_string(),
                )
            }
        }
    }

    fn is_enabled(&self, _config: &EnrichmentConfig) -> bool {
        true // Controlled by presence in config
    }

    fn priority(&self) -> u32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};
    use crate::state::InMemoryStore;

    fn create_test_incident(title: &str, description: &str, source: &str) -> Incident {
        Incident::new(
            source.to_string(),
            title.to_string(),
            description.to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[test]
    fn test_jaccard_similarity() {
        let s1 = "database connection timeout error";
        let s2 = "database connection timeout";

        let sim = HistoricalEnricher::jaccard_similarity(s1, s2);
        assert!(sim > 0.7 && sim <= 1.0);
    }

    #[test]
    fn test_calculate_similarity() {
        let inc1 = create_test_incident(
            "Database timeout",
            "Connection failed",
            "db-monitor",
        );
        let inc2 = create_test_incident(
            "Database connection timeout",
            "Connection to DB failed",
            "db-monitor",
        );

        let similarity = HistoricalEnricher::calculate_similarity(&inc1, &inc2);
        assert!(similarity > 0.6);
    }

    #[tokio::test]
    async fn test_historical_enricher() {
        let store = Arc::new(InMemoryStore::new());

        // Add historical incidents
        let hist1 = create_test_incident(
            "Database timeout",
            "Connection failed",
            "db-monitor",
        );
        store.save_incident(&hist1).await.unwrap();

        let enricher = HistoricalEnricher::new(store);
        let incident = create_test_incident(
            "Database connection error",
            "Failed to connect",
            "db-monitor",
        );

        let mut context = EnrichedContext::new(incident.id);
        let config = EnrichmentConfig::default();

        let result = enricher.enrich(&incident, &mut context, &config).await;

        assert!(result.success);
        assert!(context.historical.is_some());
    }

    #[tokio::test]
    async fn test_service_enricher() {
        let enricher = ServiceEnricher::new();
        let incident = create_test_incident(
            "API service error",
            "500 error",
            "api-service",
        );

        let mut context = EnrichedContext::new(incident.id);
        let config = EnrichmentConfig::default();

        let result = enricher.enrich(&incident, &mut context, &config).await;

        assert!(result.success);
        assert!(context.service.is_some());

        let service = context.service.unwrap();
        assert!(!service.service_name.is_empty());
    }

    #[tokio::test]
    async fn test_team_enricher() {
        let enricher = TeamEnricher::new();
        let incident = create_test_incident(
            "Infrastructure issue",
            "Server down",
            "monitoring",
        );

        let mut context = EnrichedContext::new(incident.id);
        let config = EnrichmentConfig::default();

        let result = enricher.enrich(&incident, &mut context, &config).await;

        assert!(result.success);
        assert!(context.team.is_some());

        let team = context.team.unwrap();
        assert!(!team.on_call.is_empty());
        assert_eq!(team.primary_team, "platform-team");
    }

    #[test]
    fn test_enricher_priority() {
        let store = Arc::new(InMemoryStore::new());
        let hist_enricher = HistoricalEnricher::new(store);
        let service_enricher = ServiceEnricher::new();
        let team_enricher = TeamEnricher::new();

        assert!(hist_enricher.priority() < service_enricher.priority());
        assert!(service_enricher.priority() < team_enricher.priority());
    }
}
