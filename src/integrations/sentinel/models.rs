use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{IncidentType, Severity};

/// Alert received from Sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelAlert {
    /// Alert ID from Sentinel
    pub id: String,
    /// Timestamp when alert was generated
    pub timestamp: DateTime<Utc>,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert category
    pub category: AlertCategory,
    /// Alert title
    pub title: String,
    /// Alert description
    pub description: String,
    /// Source model or service
    pub source: String,
    /// Affected resources
    pub affected_resources: Vec<String>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Anomaly score (0.0 to 1.0)
    pub anomaly_score: Option<f64>,
    /// Recommended actions
    pub recommended_actions: Vec<String>,
}

/// Sentinel alert severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl From<AlertSeverity> for Severity {
    fn from(severity: AlertSeverity) -> Self {
        match severity {
            AlertSeverity::Critical => Severity::P0,
            AlertSeverity::High => Severity::P1,
            AlertSeverity::Medium => Severity::P2,
            AlertSeverity::Low => Severity::P3,
            AlertSeverity::Info => Severity::P4,
        }
    }
}

/// Alert category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertCategory {
    ModelDrift,
    DataQuality,
    PerformanceDegradation,
    SecurityThreat,
    ResourceExhaustion,
    ComplianceViolation,
    Other,
}

impl From<AlertCategory> for IncidentType {
    fn from(category: AlertCategory) -> Self {
        match category {
            AlertCategory::ModelDrift => IncidentType::Application,
            AlertCategory::DataQuality => IncidentType::Data,
            AlertCategory::PerformanceDegradation => IncidentType::Performance,
            AlertCategory::SecurityThreat => IncidentType::Security,
            AlertCategory::ResourceExhaustion => IncidentType::Infrastructure,
            AlertCategory::ComplianceViolation => IncidentType::Compliance,
            AlertCategory::Other => IncidentType::Unknown,
        }
    }
}

/// Anomaly analysis result from Sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAnalysis {
    /// Analysis ID
    pub id: String,
    /// Timestamp of analysis
    pub timestamp: DateTime<Utc>,
    /// Anomaly detected
    pub is_anomalous: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Anomaly type
    pub anomaly_type: Option<String>,
    /// Contributing factors
    pub factors: Vec<AnomalyFactor>,
    /// Recommended severity
    pub recommended_severity: AlertSeverity,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Contributing factor to an anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFactor {
    /// Factor name
    pub name: String,
    /// Factor value
    pub value: f64,
    /// Contribution percentage
    pub contribution: f64,
    /// Description
    pub description: String,
}

/// Severity prediction request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityPredictionRequest {
    /// Incident description
    pub description: String,
    /// Incident type
    pub incident_type: String,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
    /// Historical data
    pub historical_severity: Option<String>,
}

/// Severity prediction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityPrediction {
    /// Predicted severity
    pub predicted_severity: AlertSeverity,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Reasoning
    pub reasoning: String,
    /// Alternative severities with probabilities
    pub alternatives: Vec<SeverityAlternative>,
}

/// Alternative severity prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityAlternative {
    /// Severity level
    pub severity: AlertSeverity,
    /// Probability (0.0 to 1.0)
    pub probability: f64,
}

/// Sentinel API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelResponse<T> {
    /// Success status
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message if failed
    pub error: Option<String>,
    /// Request ID for tracing
    pub request_id: Option<String>,
}

impl<T> SentinelResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: Some(Uuid::new_v4().to_string()),
        }
    }

    /// Create an error response
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            request_id: Some(Uuid::new_v4().to_string()),
        }
    }

    /// Check if response is successful
    pub fn is_success(&self) -> bool {
        self.success && self.data.is_some()
    }

    /// Get data or error
    pub fn into_result(self) -> Result<T, String> {
        if self.success {
            self.data.ok_or_else(|| "No data in successful response".to_string())
        } else {
            Err(self.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }
}

impl SentinelAlert {
    /// Convert to internal Alert model
    pub fn to_alert(&self) -> crate::models::Alert {
        crate::models::Alert::new(
            self.id.clone(),
            "llm-sentinel".to_string(),
            self.title.clone(),
            self.description.clone(),
            self.severity.into(),
            self.category.into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_conversion() {
        assert_eq!(Severity::from(AlertSeverity::Critical), Severity::P0);
        assert_eq!(Severity::from(AlertSeverity::High), Severity::P1);
        assert_eq!(Severity::from(AlertSeverity::Medium), Severity::P2);
        assert_eq!(Severity::from(AlertSeverity::Low), Severity::P3);
        assert_eq!(Severity::from(AlertSeverity::Info), Severity::P4);
    }

    #[test]
    fn test_category_conversion() {
        assert_eq!(IncidentType::from(AlertCategory::ModelDrift), IncidentType::Application);
        assert_eq!(IncidentType::from(AlertCategory::DataQuality), IncidentType::Data);
        assert_eq!(
            IncidentType::from(AlertCategory::PerformanceDegradation),
            IncidentType::Performance
        );
    }

    #[test]
    fn test_sentinel_response_success() {
        let response = SentinelResponse::success("test data");
        assert!(response.is_success());
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_sentinel_response_error() {
        let response: SentinelResponse<String> = SentinelResponse::error("test error".to_string());
        assert!(!response.is_success());
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }

    #[test]
    fn test_sentinel_response_into_result() {
        let response = SentinelResponse::success(42);
        assert_eq!(response.into_result(), Ok(42));

        let response: SentinelResponse<i32> = SentinelResponse::error("error".to_string());
        assert!(response.into_result().is_err());
    }

    #[test]
    fn test_sentinel_alert_to_alert() {
        let sentinel_alert = SentinelAlert {
            id: "test-123".to_string(),
            timestamp: Utc::now(),
            severity: AlertSeverity::High,
            category: AlertCategory::ModelDrift,
            title: "Test Alert".to_string(),
            description: "Test description".to_string(),
            source: "test-model".to_string(),
            affected_resources: vec!["resource1".to_string()],
            metadata: HashMap::new(),
            anomaly_score: Some(0.95),
            recommended_actions: vec!["Action 1".to_string()],
        };

        let alert = sentinel_alert.to_alert();
        assert_eq!(alert.external_id, "test-123");
        assert_eq!(alert.source, "llm-sentinel");
        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.severity, Severity::P1);
    }
}
