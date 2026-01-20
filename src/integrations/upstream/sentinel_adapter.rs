//! LLM-Sentinel Core Adapter
//!
//! Thin runtime adapter for consuming anomaly flags, threat signals, and drift events
//! from the upstream llm-sentinel-core crate. Translates sentinel types to internal
//! incident-manager types without modifying existing alerting logic.
//!
//! This adapter provides type-safe consumption of sentinel events and converts them
//! to incident-manager's internal representation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import internal types
use crate::integrations::sentinel::{AlertCategory, AlertSeverity, SentinelAlert};

/// Adapter for consuming events from llm-sentinel-core
#[derive(Debug, Clone)]
pub struct SentinelCoreAdapter {
    /// Source identifier for tracing
    source_id: String,
    /// Enable debug logging
    debug_enabled: bool,
}

impl Default for SentinelCoreAdapter {
    fn default() -> Self {
        Self::new("llm-sentinel-core")
    }
}

impl SentinelCoreAdapter {
    /// Create a new sentinel core adapter
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            debug_enabled: false,
        }
    }

    /// Enable debug mode
    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.debug_enabled = enabled;
        self
    }

    /// Convert upstream anomaly event to internal SentinelAlert
    ///
    /// This is the primary consumption point for anomaly flags from sentinel-core.
    /// Does not modify alerting algorithms - pure type translation.
    pub fn convert_anomaly_event(&self, event: &UpstreamAnomalyEvent) -> SentinelAlert {
        SentinelAlert {
            id: event.id.clone(),
            timestamp: event.timestamp,
            severity: self.convert_severity(&event.severity),
            category: self.convert_anomaly_type_to_category(&event.anomaly_type),
            title: self.generate_alert_title(&event.anomaly_type, &event.service_id),
            description: event.description.clone().unwrap_or_default(),
            source: self.source_id.clone(),
            affected_resources: vec![event.service_id.clone(), event.model_id.clone()],
            metadata: event.metadata.clone(),
            anomaly_score: Some(event.confidence),
            recommended_actions: self.generate_recommendations(&event.anomaly_type),
        }
    }

    /// Extract threat signal from anomaly event if security-related
    pub fn extract_threat_signal(&self, event: &UpstreamAnomalyEvent) -> Option<UpstreamThreatSignal> {
        // Only extract threat signals from security-related anomalies
        if event.anomaly_type != "SecurityThreat" {
            return None;
        }

        Some(UpstreamThreatSignal {
            id: format!("threat-{}", event.id),
            timestamp: event.timestamp,
            threat_type: ThreatType::Unknown,
            confidence: event.confidence,
            source_service: event.service_id.clone(),
            indicators: Vec::new(),
            metadata: HashMap::new(),
        })
    }

    /// Extract drift events from anomaly events
    pub fn extract_drift_event(&self, event: &UpstreamAnomalyEvent) -> Option<UpstreamDriftEvent> {
        let drift_type = match event.anomaly_type.as_str() {
            "InputDrift" => Some(DriftType::Input),
            "OutputDrift" => Some(DriftType::Output),
            "ConceptDrift" => Some(DriftType::Concept),
            "EmbeddingDrift" => Some(DriftType::Embedding),
            _ => None,
        }?;

        Some(UpstreamDriftEvent {
            id: format!("drift-{}", event.id),
            timestamp: event.timestamp,
            drift_type,
            magnitude: event.confidence,
            baseline_reference: event.metadata.get("baseline_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            affected_model: event.model_id.clone(),
            detection_method: event.detection_method.clone(),
            metadata: HashMap::new(),
        })
    }

    /// Batch convert multiple anomaly events
    pub fn convert_anomaly_batch(&self, events: &[UpstreamAnomalyEvent]) -> Vec<SentinelAlert> {
        events.iter().map(|e| self.convert_anomaly_event(e)).collect()
    }

    // --- Private helper methods ---

    fn convert_severity(&self, severity: &str) -> AlertSeverity {
        match severity.to_lowercase().as_str() {
            "critical" => AlertSeverity::Critical,
            "high" => AlertSeverity::High,
            "medium" => AlertSeverity::Medium,
            "low" => AlertSeverity::Low,
            _ => AlertSeverity::Info,
        }
    }

    fn convert_anomaly_type_to_category(&self, anomaly_type: &str) -> AlertCategory {
        match anomaly_type {
            "InputDrift" | "OutputDrift" | "ConceptDrift" | "EmbeddingDrift" => AlertCategory::ModelDrift,
            "LatencySpike" | "ThroughputDegradation" => AlertCategory::PerformanceDegradation,
            "ErrorRateIncrease" | "QualityDegradation" | "Hallucination" => AlertCategory::DataQuality,
            "SecurityThreat" => AlertCategory::SecurityThreat,
            "TokenUsageSpike" | "CostAnomaly" => AlertCategory::ResourceExhaustion,
            _ => AlertCategory::Other,
        }
    }

    fn generate_alert_title(&self, anomaly_type: &str, service_id: &str) -> String {
        format!("{} detected in {}", anomaly_type, service_id)
    }

    fn generate_recommendations(&self, anomaly_type: &str) -> Vec<String> {
        match anomaly_type {
            "InputDrift" | "OutputDrift" => vec![
                "Review recent input data distributions".to_string(),
                "Check for data pipeline changes".to_string(),
                "Consider model retraining if drift persists".to_string(),
            ],
            "SecurityThreat" => vec![
                "Investigate potential security incident".to_string(),
                "Review access logs and audit trails".to_string(),
                "Escalate to security team if confirmed".to_string(),
            ],
            "LatencySpike" => vec![
                "Check infrastructure health".to_string(),
                "Review recent deployments".to_string(),
                "Monitor resource utilization".to_string(),
            ],
            _ => vec!["Review anomaly details and investigate root cause".to_string()],
        }
    }
}

/// Upstream anomaly event from sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamAnomalyEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub anomaly_type: String,
    pub severity: String,
    pub confidence: f64,
    pub service_id: String,
    pub model_id: String,
    pub description: Option<String>,
    pub detection_method: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl UpstreamAnomalyEvent {
    // NOTE: from_sentinel requires ecosystem feature (llm_sentinel_core)
    // Uncomment when ecosystem dependencies are available
    // pub fn from_sentinel(event: &llm_sentinel_core::events::AnomalyEvent) -> Self { ... }
}

/// Threat signal extracted from sentinel telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamThreatSignal {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub threat_type: ThreatType,
    pub confidence: f64,
    pub source_service: String,
    pub indicators: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Classification of threat types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThreatType {
    PromptInjection,
    Jailbreak,
    DataExfiltration,
    ModelInversion,
    AdversarialInput,
    Unknown,
}

/// Drift event from sentinel anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamDriftEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub drift_type: DriftType,
    pub magnitude: f64,
    pub baseline_reference: Option<String>,
    pub affected_model: String,
    pub detection_method: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of drift detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    Input,
    Output,
    Concept,
    Embedding,
    Distribution,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = SentinelCoreAdapter::new("test-source");
        assert_eq!(adapter.source_id, "test-source");
    }

    #[test]
    fn test_default_adapter() {
        let adapter = SentinelCoreAdapter::default();
        assert_eq!(adapter.source_id, "llm-sentinel-core");
    }

    #[test]
    fn test_severity_conversion() {
        let adapter = SentinelCoreAdapter::default();
        assert!(matches!(adapter.convert_severity("Critical"), AlertSeverity::Critical));
        assert!(matches!(adapter.convert_severity("low"), AlertSeverity::Low));
    }

    #[test]
    fn test_drift_type_category() {
        let adapter = SentinelCoreAdapter::default();
        assert!(matches!(
            adapter.convert_anomaly_type_to_category("InputDrift"),
            AlertCategory::ModelDrift
        ));
        assert!(matches!(
            adapter.convert_anomaly_type_to_category("SecurityThreat"),
            AlertCategory::SecurityThreat
        ));
    }
}
