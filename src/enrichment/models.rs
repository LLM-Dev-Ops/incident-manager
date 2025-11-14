use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for context enrichment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentConfig {
    /// Enable enrichment
    pub enabled: bool,

    /// Enable historical enrichment
    pub enable_historical: bool,

    /// Enable service enrichment
    pub enable_service: bool,

    /// Enable team enrichment
    pub enable_team: bool,

    /// Enable metric enrichment
    pub enable_metrics: bool,

    /// Enable log enrichment
    pub enable_logs: bool,

    /// Enrichment timeout (seconds)
    pub timeout_secs: u64,

    /// Maximum concurrent enrichers
    pub max_concurrent: usize,

    /// Cache TTL (seconds)
    pub cache_ttl_secs: u64,

    /// Retry attempts for failed enrichments
    pub retry_attempts: usize,

    /// Historical lookback window (seconds)
    pub historical_lookback_secs: u64,

    /// Similar incident threshold (0.0 - 1.0)
    pub similarity_threshold: f64,

    /// External API endpoints
    pub external_apis: HashMap<String, String>,

    /// API timeouts (seconds)
    pub api_timeout_secs: u64,

    /// Enable async enrichment
    pub async_enrichment: bool,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_historical: true,
            enable_service: true,
            enable_team: true,
            enable_metrics: false,
            enable_logs: false,
            timeout_secs: 10,
            max_concurrent: 5,
            cache_ttl_secs: 300,
            retry_attempts: 3,
            historical_lookback_secs: 2592000, // 30 days
            similarity_threshold: 0.7,
            external_apis: HashMap::new(),
            api_timeout_secs: 5,
            async_enrichment: true,
        }
    }
}

/// Enriched context for an incident
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnrichedContext {
    /// Incident ID this context belongs to
    pub incident_id: Uuid,

    /// Historical context
    pub historical: Option<HistoricalContext>,

    /// Service context
    pub service: Option<ServiceContext>,

    /// Team context
    pub team: Option<TeamContext>,

    /// Metrics context
    pub metrics: Option<MetricsContext>,

    /// Log context
    pub logs: Option<LogContext>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Enrichment timestamp
    pub enriched_at: DateTime<Utc>,

    /// Enrichment duration (milliseconds)
    pub enrichment_duration_ms: u64,

    /// Enrichers that ran successfully
    pub successful_enrichers: Vec<String>,

    /// Enrichers that failed
    pub failed_enrichers: Vec<String>,
}

impl EnrichedContext {
    /// Create a new enriched context
    pub fn new(incident_id: Uuid) -> Self {
        Self {
            incident_id,
            enriched_at: Utc::now(),
            ..Default::default()
        }
    }

    /// Check if context is complete (all enrichers ran)
    pub fn is_complete(&self) -> bool {
        self.failed_enrichers.is_empty()
    }

    /// Get total enrichers count
    pub fn total_enrichers(&self) -> usize {
        self.successful_enrichers.len() + self.failed_enrichers.len()
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_enrichers() == 0 {
            return 0.0;
        }
        self.successful_enrichers.len() as f64 / self.total_enrichers() as f64
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

/// Historical context from similar past incidents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalContext {
    /// Similar incidents from the past
    pub similar_incidents: Vec<SimilarIncident>,

    /// Common patterns
    pub common_patterns: Vec<String>,

    /// Average resolution time (seconds)
    pub avg_resolution_time: Option<u64>,

    /// Most common resolution method
    pub common_resolution_method: Option<String>,

    /// Recurrence rate (0.0 - 1.0)
    pub recurrence_rate: f64,

    /// Last occurrence timestamp
    pub last_occurrence: Option<DateTime<Utc>>,
}

/// Similar incident from history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarIncident {
    /// Incident ID
    pub incident_id: Uuid,

    /// Similarity score (0.0 - 1.0)
    pub similarity_score: f64,

    /// Title
    pub title: String,

    /// How it was resolved
    pub resolution: Option<String>,

    /// Resolution time (seconds)
    pub resolution_time: Option<u64>,

    /// Occurred at
    pub occurred_at: DateTime<Utc>,
}

/// Service context from service catalog/CMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceContext {
    /// Service name
    pub service_name: String,

    /// Service ID
    pub service_id: Option<String>,

    /// Service status
    pub service_status: ServiceStatus,

    /// Service owner/team
    pub owner: Option<String>,

    /// Service tier (P0, P1, etc.)
    pub tier: Option<String>,

    /// Dependencies
    pub dependencies: Vec<ServiceDependency>,

    /// Recent changes
    pub recent_changes: Vec<ServiceChange>,

    /// Service health score (0.0 - 1.0)
    pub health_score: Option<f64>,

    /// SLA target (seconds)
    pub sla_target: Option<u64>,

    /// Service URL
    pub service_url: Option<String>,
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Down,
    Maintenance,
    Unknown,
}

/// Service dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependency {
    pub service_name: String,
    pub dependency_type: DependencyType,
    pub status: ServiceStatus,
}

/// Dependency type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Upstream,
    Downstream,
    Database,
    Cache,
    Queue,
    External,
}

/// Service change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceChange {
    pub change_id: String,
    pub description: String,
    pub deployed_at: DateTime<Utc>,
    pub deployed_by: String,
}

/// Team context for incident response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamContext {
    /// Primary team responsible
    pub primary_team: String,

    /// On-call engineers
    pub on_call: Vec<OnCallEngineer>,

    /// Team expertise areas
    pub expertise: Vec<String>,

    /// Team timezone
    pub timezone: Option<String>,

    /// Team slack channel
    pub slack_channel: Option<String>,

    /// Team escalation policy
    pub escalation_policy: Option<String>,

    /// Average response time (seconds)
    pub avg_response_time: Option<u64>,
}

/// On-call engineer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallEngineer {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
    pub shift_start: DateTime<Utc>,
    pub shift_end: DateTime<Utc>,
}

/// Metrics context from monitoring systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsContext {
    /// Relevant metrics
    pub metrics: Vec<Metric>,

    /// Anomalies detected
    pub anomalies: Vec<Anomaly>,

    /// Metric dashboard URL
    pub dashboard_url: Option<String>,
}

/// Metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    pub threshold: Option<f64>,
    pub is_anomalous: bool,
}

/// Anomaly detected in metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub metric_name: String,
    pub severity: AnomalySeverity,
    pub description: String,
    pub detected_at: DateTime<Utc>,
}

/// Anomaly severity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Log context from logging systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
    /// Relevant log entries
    pub log_entries: Vec<LogEntry>,

    /// Error patterns found
    pub error_patterns: Vec<String>,

    /// Log query URL
    pub query_url: Option<String>,

    /// Total errors in time window
    pub total_errors: usize,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
    pub correlation_id: Option<String>,
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Enrichment result for a single enricher
#[derive(Debug, Clone)]
pub struct EnrichmentResult {
    /// Enricher name
    pub enricher_name: String,

    /// Success status
    pub success: bool,

    /// Duration (milliseconds)
    pub duration_ms: u64,

    /// Error message if failed
    pub error: Option<String>,
}

impl EnrichmentResult {
    pub fn success(enricher_name: String, duration_ms: u64) -> Self {
        Self {
            enricher_name,
            success: true,
            duration_ms,
            error: None,
        }
    }

    pub fn failure(enricher_name: String, duration_ms: u64, error: String) -> Self {
        Self {
            enricher_name,
            success: false,
            duration_ms,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enriched_context_creation() {
        let incident_id = Uuid::new_v4();
        let context = EnrichedContext::new(incident_id);

        assert_eq!(context.incident_id, incident_id);
        assert!(context.successful_enrichers.is_empty());
        assert!(context.failed_enrichers.is_empty());
    }

    #[test]
    fn test_enriched_context_success_rate() {
        let mut context = EnrichedContext::new(Uuid::new_v4());

        context.successful_enrichers.push("enricher1".to_string());
        context.successful_enrichers.push("enricher2".to_string());
        context.failed_enrichers.push("enricher3".to_string());

        assert_eq!(context.total_enrichers(), 3);
        assert_eq!(context.success_rate(), 2.0 / 3.0);
        assert!(!context.is_complete());
    }

    #[test]
    fn test_enriched_context_complete() {
        let mut context = EnrichedContext::new(Uuid::new_v4());

        context.successful_enrichers.push("enricher1".to_string());
        context.successful_enrichers.push("enricher2".to_string());

        assert!(context.is_complete());
        assert_eq!(context.success_rate(), 1.0);
    }

    #[test]
    fn test_enrichment_result_success() {
        let result = EnrichmentResult::success("test_enricher".to_string(), 100);

        assert!(result.success);
        assert_eq!(result.enricher_name, "test_enricher");
        assert_eq!(result.duration_ms, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_enrichment_result_failure() {
        let result = EnrichmentResult::failure(
            "test_enricher".to_string(),
            50,
            "Connection timeout".to_string(),
        );

        assert!(!result.success);
        assert_eq!(result.error, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_enrichment_config_default() {
        let config = EnrichmentConfig::default();

        assert!(config.enabled);
        assert!(config.enable_historical);
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.similarity_threshold, 0.7);
    }

    #[test]
    fn test_service_status() {
        let status = ServiceStatus::Healthy;
        assert_eq!(status, ServiceStatus::Healthy);

        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""healthy""#);
    }

    #[test]
    fn test_add_metadata() {
        let mut context = EnrichedContext::new(Uuid::new_v4());
        context.add_metadata("key1".to_string(), "value1".to_string());
        context.add_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(context.metadata.len(), 2);
        assert_eq!(context.metadata.get("key1"), Some(&"value1".to_string()));
    }
}
