use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use super::incident::{IncidentType, Severity};

/// Represents an alert from an external source
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Alert {
    /// Unique alert identifier
    pub id: Uuid,

    /// External alert ID (from source system)
    pub external_id: String,

    /// Alert source (e.g., "llm-sentinel", "llm-shield")
    #[validate(length(min = 1, max = 255))]
    pub source: String,

    /// Timestamp when alert was generated
    pub timestamp: DateTime<Utc>,

    /// Timestamp when alert was received
    pub received_at: DateTime<Utc>,

    /// Alert severity
    pub severity: Severity,

    /// Alert type
    pub alert_type: IncidentType,

    /// Alert title
    #[validate(length(min = 1, max = 500))]
    pub title: String,

    /// Alert description
    pub description: String,

    /// Labels/tags
    pub labels: HashMap<String, String>,

    /// Affected services/resources
    pub affected_services: Vec<String>,

    /// Runbook URL
    pub runbook_url: Option<String>,

    /// Annotations (additional metadata)
    pub annotations: HashMap<String, String>,

    /// Generated incident ID (if converted to incident)
    pub incident_id: Option<Uuid>,

    /// Whether this alert was deduplicated
    pub deduplicated: bool,

    /// Parent alert ID if this is a duplicate
    pub parent_alert_id: Option<Uuid>,
}

impl Alert {
    /// Create a new alert
    pub fn new(
        external_id: String,
        source: String,
        title: String,
        description: String,
        severity: Severity,
        alert_type: IncidentType,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            external_id,
            source,
            timestamp: now,
            received_at: now,
            severity,
            alert_type,
            title,
            description,
            labels: HashMap::new(),
            affected_services: Vec::new(),
            runbook_url: None,
            annotations: HashMap::new(),
            incident_id: None,
            deduplicated: false,
            parent_alert_id: None,
        }
    }

    /// Convert alert to incident (consuming the alert)
    pub fn to_incident(&self) -> super::incident::Incident {
        use super::incident::Incident;

        let mut incident = Incident::new(
            self.source.clone(),
            self.title.clone(),
            self.description.clone(),
            self.severity,
            self.alert_type.clone(),
        );

        incident.affected_resources = self.affected_services.clone();
        incident.labels = self.labels.clone();

        // Add runbook URL to labels if present
        if let Some(runbook) = &self.runbook_url {
            incident.labels.insert("runbook_url".to_string(), runbook.clone());
        }

        // Copy annotations to labels
        for (key, value) in &self.annotations {
            incident.labels.insert(format!("annotation_{}", key), value.clone());
        }

        incident
    }

    /// Generate fingerprint for deduplication
    pub fn generate_fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.source.as_bytes());
        hasher.update(self.alert_type.to_string().as_bytes());
        hasher.update(self.title.as_bytes());

        // Include affected services
        for service in &self.affected_services {
            hasher.update(service.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Check if alert is urgent
    pub fn is_urgent(&self) -> bool {
        self.severity.is_urgent()
    }
}

/// Alert acknowledgment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AckStatus {
    Accepted,
    Duplicate,
    RateLimited,
    Rejected,
}

/// Alert acknowledgment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertAck {
    pub alert_id: Uuid,
    pub incident_id: Option<Uuid>,
    pub status: AckStatus,
    pub message: String,
    pub received_at: DateTime<Utc>,
}

impl AlertAck {
    pub fn accepted(alert_id: Uuid, incident_id: Uuid) -> Self {
        Self {
            alert_id,
            incident_id: Some(incident_id),
            status: AckStatus::Accepted,
            message: "Alert accepted and incident created".to_string(),
            received_at: Utc::now(),
        }
    }

    pub fn duplicate(alert_id: Uuid, incident_id: Uuid) -> Self {
        Self {
            alert_id,
            incident_id: Some(incident_id),
            status: AckStatus::Duplicate,
            message: "Alert is a duplicate of existing incident".to_string(),
            received_at: Utc::now(),
        }
    }

    pub fn rate_limited(alert_id: Uuid) -> Self {
        Self {
            alert_id,
            incident_id: None,
            status: AckStatus::RateLimited,
            message: "Alert rejected due to rate limiting".to_string(),
            received_at: Utc::now(),
        }
    }

    pub fn rejected(alert_id: Uuid, reason: String) -> Self {
        Self {
            alert_id,
            incident_id: None,
            status: AckStatus::Rejected,
            message: format!("Alert rejected: {}", reason),
            received_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            "ext-123".to_string(),
            "llm-sentinel".to_string(),
            "High CPU Usage".to_string(),
            "CPU usage exceeded 90%".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        assert_eq!(alert.source, "llm-sentinel");
        assert!(!alert.deduplicated);
        assert!(alert.incident_id.is_none());
        assert!(alert.is_urgent());
    }

    #[test]
    fn test_alert_to_incident_conversion() {
        let mut alert = Alert::new(
            "ext-456".to_string(),
            "llm-shield".to_string(),
            "Security Violation".to_string(),
            "Unauthorized access attempt".to_string(),
            Severity::P0,
            IncidentType::Security,
        );

        alert.affected_services.push("api-service".to_string());
        alert.runbook_url = Some("https://runbooks.example.com/security".to_string());
        alert.labels.insert("environment".to_string(), "production".to_string());

        let incident = alert.to_incident();

        assert_eq!(incident.source, alert.source);
        assert_eq!(incident.title, alert.title);
        assert_eq!(incident.severity, alert.severity);
        assert_eq!(incident.affected_resources, alert.affected_services);
        assert!(incident.labels.contains_key("runbook_url"));
    }

    #[test]
    fn test_fingerprint_consistency() {
        let alert1 = Alert::new(
            "ext-1".to_string(),
            "source-a".to_string(),
            "Alert Title".to_string(),
            "Description 1".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        let alert2 = Alert::new(
            "ext-2".to_string(),
            "source-a".to_string(),
            "Alert Title".to_string(),
            "Description 2".to_string(), // Different description
            Severity::P1,                // Different severity
            IncidentType::Application,
        );

        // Same fingerprint despite different descriptions/severities
        assert_eq!(alert1.generate_fingerprint(), alert2.generate_fingerprint());
    }

    #[test]
    fn test_ack_statuses() {
        let alert_id = Uuid::new_v4();
        let incident_id = Uuid::new_v4();

        let ack_accepted = AlertAck::accepted(alert_id, incident_id);
        assert_eq!(ack_accepted.status, AckStatus::Accepted);
        assert!(ack_accepted.incident_id.is_some());

        let ack_dup = AlertAck::duplicate(alert_id, incident_id);
        assert_eq!(ack_dup.status, AckStatus::Duplicate);

        let ack_limited = AlertAck::rate_limited(alert_id);
        assert_eq!(ack_limited.status, AckStatus::RateLimited);
        assert!(ack_limited.incident_id.is_none());

        let ack_rejected = AlertAck::rejected(alert_id, "Invalid format".to_string());
        assert_eq!(ack_rejected.status, AckStatus::Rejected);
        assert!(ack_rejected.message.contains("Invalid format"));
    }
}
