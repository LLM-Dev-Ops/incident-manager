use tracing::{info, warn};

use super::client::ShieldClient;
use super::models::*;
use crate::error::Result;
use crate::models::incident::Incident;

/// Security event handler for Shield integration
pub struct SecurityEventHandler {
    client: ShieldClient,
}

impl SecurityEventHandler {
    /// Create a new security event handler
    pub fn new(client: ShieldClient) -> Self {
        Self { client }
    }

    /// Handle a security incident by analyzing threats and generating mitigation
    pub async fn handle_security_incident(
        &self,
        incident: &Incident,
    ) -> Result<SecurityIncidentResponse> {
        info!(
            incident_id = %incident.id,
            "Handling security incident with Shield"
        );

        // Convert incident to security event
        let security_event = SecurityEvent {
            event_id: incident.id.to_string(),
            event_type: SecurityEventType::SuspiciousActivity,
            source: "IncidentManager".to_string(),
            description: incident.description.clone(),
            metadata: incident.labels.clone(),
            timestamp: incident.created_at,
            severity: match incident.severity {
                crate::models::incident::Severity::P0 => EventSeverity::Critical,
                crate::models::incident::Severity::P1 => EventSeverity::High,
                crate::models::incident::Severity::P2 => EventSeverity::Medium,
                crate::models::incident::Severity::P3 => EventSeverity::Low,
                crate::models::incident::Severity::P4 => EventSeverity::Low,
            },
        };

        // Analyze the threat
        let threat_analysis = self.client.analyze_security_event(security_event).await?;

        info!(
            incident_id = %incident.id,
            threat_type = ?threat_analysis.threat_type,
            confidence = threat_analysis.confidence_score,
            "Threat analysis completed"
        );

        // Generate mitigation plan if threat is significant
        let mitigation_plan = if threat_analysis.confidence_score >= 0.7 {
            match self
                .client
                .generate_mitigation_plan(GenerateMitigationRequest {
                    threat_id: threat_analysis.analysis_id.clone(),
                    threat_type: threat_analysis.threat_type.clone(),
                    severity: threat_analysis.severity_level.clone(),
                    affected_systems: threat_analysis.affected_systems.clone(),
                    context: threat_analysis.details.clone(),
                })
                .await
            {
                Ok(plan) => Some(plan),
                Err(e) => {
                    warn!(error = %e, "Failed to generate mitigation plan");
                    None
                }
            }
        } else {
            None
        };

        Ok(SecurityIncidentResponse {
            incident_id: incident.id.to_string(),
            threat_analysis,
            mitigation_plan,
        })
    }

    /// Track security patterns across multiple events
    pub async fn track_security_pattern(
        &self,
        event_ids: Vec<String>,
        pattern_type: String,
        description: String,
    ) -> Result<SecurityPattern> {
        info!(
            pattern_type = %pattern_type,
            num_events = event_ids.len(),
            "Tracking security pattern"
        );

        self.client
            .report_security_pattern(ReportSecurityPatternRequest {
                pattern_type,
                event_ids,
                description,
                attributes: Default::default(),
            })
            .await
    }
}

/// Response from handling a security incident
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityIncidentResponse {
    pub incident_id: String,
    pub threat_analysis: ThreatAnalysis,
    pub mitigation_plan: Option<MitigationPlan>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_incident() -> Incident {
        Incident::new(
            "test".to_string(),
            "Test Security Incident".to_string(),
            "Suspicious activity detected".to_string(),
            crate::models::incident::Severity::P1,
            crate::models::incident::IncidentType::Security,
        )
    }

    #[test]
    fn test_incident_conversion() {
        let incident = create_test_incident();
        assert_eq!(incident.severity, crate::models::incident::Severity::P1);
    }
}
