use chrono::Utc;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::models::{SentinelAlert, SeverityPredictionRequest};
use crate::integrations::common::IntegrationResult;
use crate::models::{Alert, Incident, TimelineEvent, EventType};

/// Handler for processing Sentinel alerts
pub struct AlertHandler {
    /// Enable automatic incident creation
    auto_create_incidents: bool,
    /// Minimum severity for auto-creation
    min_severity_for_auto_create: crate::models::Severity,
}

impl Default for AlertHandler {
    fn default() -> Self {
        Self {
            auto_create_incidents: true,
            min_severity_for_auto_create: crate::models::Severity::P2,
        }
    }
}

impl AlertHandler {
    /// Create a new alert handler
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to automatically create incidents
    pub fn with_auto_create(mut self, enabled: bool) -> Self {
        self.auto_create_incidents = enabled;
        self
    }

    /// Set minimum severity for auto-creation
    pub fn with_min_severity(mut self, severity: crate::models::Severity) -> Self {
        self.min_severity_for_auto_create = severity;
        self
    }

    /// Process a Sentinel alert
    pub fn process_alert(&self, sentinel_alert: &SentinelAlert) -> IntegrationResult<Alert> {
        debug!(
            alert_id = %sentinel_alert.id,
            title = %sentinel_alert.title,
            severity = ?sentinel_alert.severity,
            "Processing Sentinel alert"
        );

        // Convert Sentinel alert to internal Alert
        let mut alert = sentinel_alert.to_alert();

        // Add metadata from Sentinel
        for (key, value) in &sentinel_alert.metadata {
            if let Some(str_value) = value.as_str() {
                alert.annotations.insert(key.clone(), str_value.to_string());
            } else {
                alert.annotations.insert(key.clone(), value.to_string());
            }
        }

        // Add anomaly score if present
        if let Some(score) = sentinel_alert.anomaly_score {
            alert.annotations.insert("anomaly_score".to_string(), score.to_string());
        }

        // Add recommended actions
        if !sentinel_alert.recommended_actions.is_empty() {
            let actions = sentinel_alert.recommended_actions.join("; ");
            alert.annotations.insert("recommended_actions".to_string(), actions);
        }

        // Add affected resources
        alert.affected_services = sentinel_alert.affected_resources.clone();

        info!(
            alert_id = %alert.id,
            external_id = %alert.external_id,
            "Processed Sentinel alert"
        );

        Ok(alert)
    }

    /// Determine if an alert should trigger incident creation
    pub fn should_create_incident(&self, alert: &Alert) -> bool {
        if !self.auto_create_incidents {
            debug!("Auto-create incidents is disabled");
            return false;
        }

        let should_create = alert.severity <= self.min_severity_for_auto_create;
        
        if should_create {
            debug!(
                alert_id = %alert.id,
                severity = ?alert.severity,
                min_severity = ?self.min_severity_for_auto_create,
                "Alert meets criteria for incident creation"
            );
        } else {
            debug!(
                alert_id = %alert.id,
                severity = ?alert.severity,
                min_severity = ?self.min_severity_for_auto_create,
                "Alert does not meet criteria for incident creation"
            );
        }

        should_create
    }

    /// Create an incident from an alert
    pub fn create_incident_from_alert(&self, alert: &Alert) -> Incident {
        debug!(alert_id = %alert.id, "Creating incident from alert");

        let mut incident = Incident::new(
            alert.source.clone(),
            alert.title.clone(),
            alert.description.clone(),
            alert.severity,
            alert.alert_type.clone(),
        );

        // Set affected resources
        incident.affected_resources = alert.affected_services.clone();

        // Copy labels
        incident.labels = alert.labels.clone();

        // Add alert metadata to timeline
        incident.add_timeline_event(TimelineEvent {
            timestamp: Utc::now(),
            event_type: EventType::AlertReceived,
            actor: "sentinel-handler".to_string(),
            description: format!("Incident created from Sentinel alert {}", alert.external_id),
            metadata: HashMap::from([
                ("alert_id".to_string(), alert.id.to_string()),
                ("external_id".to_string(), alert.external_id.clone()),
            ]),
        });

        // Add runbook URL if available
        if let Some(runbook) = &alert.runbook_url {
            incident.labels.insert("runbook_url".to_string(), runbook.clone());
        }

        info!(
            incident_id = %incident.id,
            alert_id = %alert.id,
            "Created incident from alert"
        );

        incident
    }

    /// Enrich alert with additional context
    pub fn enrich_alert(&self, alert: &mut Alert, context: HashMap<String, String>) {
        debug!(alert_id = %alert.id, "Enriching alert with context");

        for (key, value) in context {
            alert.annotations.insert(key, value);
        }
    }

    /// Build severity prediction request from alert
    pub fn build_severity_prediction_request(
        &self,
        alert: &Alert,
    ) -> SeverityPredictionRequest {
        let mut context = HashMap::new();
        
        // Add alert metadata to context
        for (key, value) in &alert.annotations {
            context.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        // Add affected services
        if !alert.affected_services.is_empty() {
            context.insert(
                "affected_services".to_string(),
                serde_json::Value::Array(
                    alert.affected_services
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            );
        }

        SeverityPredictionRequest {
            description: alert.description.clone(),
            incident_type: format!("{:?}", alert.alert_type),
            context,
            historical_severity: Some(format!("{:?}", alert.severity)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};
    use std::collections::HashMap;

    fn create_test_sentinel_alert() -> SentinelAlert {
        use super::super::models::{AlertCategory, AlertSeverity};
        
        SentinelAlert {
            id: "sentinel-123".to_string(),
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
        }
    }

    #[test]
    fn test_alert_handler_creation() {
        let handler = AlertHandler::new();
        assert!(handler.auto_create_incidents);
        assert_eq!(handler.min_severity_for_auto_create, Severity::P2);
    }

    #[test]
    fn test_alert_handler_with_config() {
        let handler = AlertHandler::new()
            .with_auto_create(false)
            .with_min_severity(Severity::P0);

        assert!(!handler.auto_create_incidents);
        assert_eq!(handler.min_severity_for_auto_create, Severity::P0);
    }

    #[test]
    fn test_process_alert() {
        let handler = AlertHandler::new();
        let sentinel_alert = create_test_sentinel_alert();

        let result = handler.process_alert(&sentinel_alert);
        assert!(result.is_ok());

        let alert = result.unwrap();
        assert_eq!(alert.external_id, "sentinel-123");
        assert_eq!(alert.source, "llm-sentinel");
        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.severity, Severity::P1);
        assert!(alert.annotations.contains_key("anomaly_score"));
    }

    #[test]
    fn test_should_create_incident() {
        let handler = AlertHandler::new()
            .with_auto_create(true)
            .with_min_severity(Severity::P2);

        // Should create for P0, P1, P2
        let alert = Alert::new(
            "test".to_string(),
            "source".to_string(),
            "title".to_string(),
            "desc".to_string(),
            Severity::P1,
            IncidentType::Application,
        );
        assert!(handler.should_create_incident(&alert));

        // Should not create for P3, P4
        let alert = Alert::new(
            "test".to_string(),
            "source".to_string(),
            "title".to_string(),
            "desc".to_string(),
            Severity::P3,
            IncidentType::Application,
        );
        assert!(!handler.should_create_incident(&alert));
    }

    #[test]
    fn test_should_not_create_incident_when_disabled() {
        let handler = AlertHandler::new().with_auto_create(false);

        let alert = Alert::new(
            "test".to_string(),
            "source".to_string(),
            "title".to_string(),
            "desc".to_string(),
            Severity::P0,
            IncidentType::Application,
        );
        assert!(!handler.should_create_incident(&alert));
    }

    #[test]
    fn test_create_incident_from_alert() {
        let handler = AlertHandler::new();
        let sentinel_alert = create_test_sentinel_alert();
        let alert = handler.process_alert(&sentinel_alert).unwrap();

        let incident = handler.create_incident_from_alert(&alert);
        
        assert_eq!(incident.title, alert.title);
        assert_eq!(incident.description, alert.description);
        assert_eq!(incident.severity, alert.severity);
        assert_eq!(incident.incident_type, alert.alert_type);
        assert_eq!(incident.affected_resources, alert.affected_services);
        assert!(!incident.timeline.is_empty());
    }

    #[test]
    fn test_enrich_alert() {
        let handler = AlertHandler::new();
        let sentinel_alert = create_test_sentinel_alert();
        let mut alert = handler.process_alert(&sentinel_alert).unwrap();

        let mut context = HashMap::new();
        context.insert("region".to_string(), "us-east-1".to_string());
        context.insert("environment".to_string(), "production".to_string());

        handler.enrich_alert(&mut alert, context);

        assert_eq!(alert.annotations.get("region"), Some(&"us-east-1".to_string()));
        assert_eq!(alert.annotations.get("environment"), Some(&"production".to_string()));
    }

    #[test]
    fn test_build_severity_prediction_request() {
        let handler = AlertHandler::new();
        let sentinel_alert = create_test_sentinel_alert();
        let alert = handler.process_alert(&sentinel_alert).unwrap();

        let request = handler.build_severity_prediction_request(&alert);

        assert_eq!(request.description, alert.description);
        assert!(request.historical_severity.is_some());
        assert!(!request.context.is_empty());
    }
}
