use crate::error::{AppError, Result};
use crate::models::{Incident, Notification, NotificationStatus};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

/// PagerDuty notification sender
#[derive(Clone)]
pub struct PagerDutySender {
    pub(crate) integration_key: String,
    pub(crate) api_url: String,
    pub(crate) client: Client,
}

#[derive(Debug, Serialize)]
struct PagerDutyEvent {
    routing_key: String,
    event_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dedup_key: Option<String>,
    payload: PagerDutyPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    links: Option<Vec<PagerDutyLink>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<PagerDutyImage>>,
}

#[derive(Debug, Serialize)]
struct PagerDutyPayload {
    summary: String,
    source: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_details: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct PagerDutyLink {
    href: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct PagerDutyImage {
    src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PagerDutyResponse {
    status: String,
    message: String,
    #[serde(default)]
    dedup_key: Option<String>,
    #[serde(default)]
    errors: Option<Vec<String>>,
}

impl PagerDutySender {
    /// Create a new PagerDuty sender
    pub fn new(integration_key: String, api_url: String) -> Result<Self> {
        if integration_key.is_empty() {
            return Err(AppError::Configuration(
                "PagerDuty integration key cannot be empty".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AppError::Configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            integration_key,
            api_url,
            client,
        })
    }

    /// Send a notification to PagerDuty
    pub async fn send(&self, notification: &mut Notification, incident: &Incident) -> Result<()> {
        // Extract PagerDuty details from notification
        let (service_key, incident_key) = match &notification.channel {
            crate::models::NotificationChannel::Pagerduty {
                service_key,
                incident_key,
            } => (service_key.clone(), incident_key.clone()),
            _ => {
                return Err(AppError::Validation(
                    "Invalid notification channel type for PagerDuty".to_string(),
                ))
            }
        };

        notification.status = NotificationStatus::Sending;

        // Build PagerDuty event
        let event = self.build_pagerduty_event(incident, &service_key, &incident_key)?;

        // Send to PagerDuty
        let result = self.send_event(&event).await;

        match result {
            Ok(dedup_key) => {
                notification.status = NotificationStatus::Sent;
                notification.sent_at = Some(Utc::now());
                info!(
                    notification_id = %notification.id,
                    incident_id = %incident.id,
                    dedup_key = ?dedup_key,
                    "PagerDuty notification sent successfully"
                );
                Ok(())
            }
            Err(e) => {
                notification.status = NotificationStatus::Failed;
                notification.error = Some(e.to_string());
                error!(
                    notification_id = %notification.id,
                    incident_id = %incident.id,
                    error = %e,
                    "Failed to send PagerDuty notification"
                );
                Err(e)
            }
        }
    }

    /// Build PagerDuty event from incident
    fn build_pagerduty_event(
        &self,
        incident: &Incident,
        service_key: &str,
        incident_key: &str,
    ) -> Result<PagerDutyEvent> {
        // Map severity to PagerDuty severity
        let pd_severity = match incident.severity {
            crate::models::Severity::P0 => "critical",
            crate::models::Severity::P1 => "error",
            crate::models::Severity::P2 => "warning",
            crate::models::Severity::P3 => "info",
            crate::models::Severity::P4 => "info",
        };

        // Determine event action based on incident state
        let event_action = match incident.state {
            crate::models::IncidentState::Resolved | crate::models::IncidentState::Closed => {
                "resolve"
            }
            crate::models::IncidentState::Investigating
            | crate::models::IncidentState::Remediating => "acknowledge",
            _ => "trigger",
        };

        // Build custom details
        let mut custom_details = HashMap::new();
        custom_details.insert(
            "incident_id".to_string(),
            serde_json::json!(incident.id.to_string()),
        );
        custom_details.insert(
            "incident_type".to_string(),
            serde_json::json!(format!("{:?}", incident.incident_type)),
        );
        custom_details.insert(
            "state".to_string(),
            serde_json::json!(format!("{:?}", incident.state)),
        );
        custom_details.insert(
            "description".to_string(),
            serde_json::json!(incident.description),
        );
        custom_details.insert(
            "affected_resources".to_string(),
            serde_json::json!(incident.affected_resources),
        );
        custom_details.insert(
            "assignees".to_string(),
            serde_json::json!(incident.assignees),
        );
        custom_details.insert(
            "labels".to_string(),
            serde_json::json!(incident.labels),
        );

        // Add resolution info if resolved
        if let Some(resolution) = &incident.resolution {
            custom_details.insert(
                "resolution_method".to_string(),
                serde_json::json!(format!("{:?}", resolution.resolution_method)),
            );
            custom_details.insert(
                "resolved_by".to_string(),
                serde_json::json!(&resolution.resolved_by),
            );
            custom_details.insert(
                "resolution_notes".to_string(),
                serde_json::json!(&resolution.notes),
            );
            if let Some(root_cause) = &resolution.root_cause {
                custom_details.insert("root_cause".to_string(), serde_json::json!(root_cause));
            }
        }

        Ok(PagerDutyEvent {
            routing_key: if service_key.is_empty() {
                self.integration_key.clone()
            } else {
                service_key.to_string()
            },
            event_action: event_action.to_string(),
            dedup_key: Some(if incident_key.is_empty() {
                incident.id.to_string()
            } else {
                incident_key.to_string()
            }),
            payload: PagerDutyPayload {
                summary: incident.title.clone(),
                source: incident.source.clone(),
                severity: pd_severity.to_string(),
                timestamp: Some(incident.created_at.to_rfc3339()),
                component: None,
                group: Some(format!("{:?}", incident.incident_type)),
                class: Some(format!("{:?}", incident.severity)),
                custom_details: Some(custom_details),
            },
            links: None,
            images: None,
        })
    }

    /// Send event to PagerDuty Events API v2
    async fn send_event(&self, event: &PagerDutyEvent) -> Result<Option<String>> {
        let response = self
            .client
            .post(&self.api_url)
            .header("Content-Type", "application/json")
            .json(event)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send PagerDuty event: {}", e)))?;

        let status = response.status();

        let pd_response: PagerDutyResponse = response.json().await.map_err(|e| {
            AppError::Internal(format!("Failed to parse PagerDuty response: {}", e))
        })?;

        if !status.is_success() {
            let error_msg = if let Some(errors) = pd_response.errors {
                errors.join(", ")
            } else {
                pd_response.message.clone()
            };

            return Err(AppError::Internal(format!(
                "PagerDuty API error ({}): {}",
                status, error_msg
            )));
        }

        // Check response status
        match pd_response.status.as_str() {
            "success" => Ok(pd_response.dedup_key),
            _ => Err(AppError::Internal(format!(
                "PagerDuty returned non-success status: {} - {}",
                pd_response.status, pd_response.message
            ))),
        }
    }

    /// Trigger a new incident in PagerDuty
    pub async fn trigger_incident(&self, incident: &Incident) -> Result<String> {
        let event = PagerDutyEvent {
            routing_key: self.integration_key.clone(),
            event_action: "trigger".to_string(),
            dedup_key: Some(incident.id.to_string()),
            payload: PagerDutyPayload {
                summary: incident.title.clone(),
                source: incident.source.clone(),
                severity: match incident.severity {
                    crate::models::Severity::P0 => "critical",
                    crate::models::Severity::P1 => "error",
                    crate::models::Severity::P2 => "warning",
                    _ => "info",
                }
                .to_string(),
                timestamp: Some(incident.created_at.to_rfc3339()),
                component: None,
                group: Some(format!("{:?}", incident.incident_type)),
                class: Some(format!("{:?}", incident.severity)),
                custom_details: None,
            },
            links: None,
            images: None,
        };

        let dedup_key = self.send_event(&event).await?;
        Ok(dedup_key.unwrap_or_else(|| incident.id.to_string()))
    }

    /// Acknowledge an incident in PagerDuty
    pub async fn acknowledge_incident(&self, incident_id: &str) -> Result<()> {
        let event = PagerDutyEvent {
            routing_key: self.integration_key.clone(),
            event_action: "acknowledge".to_string(),
            dedup_key: Some(incident_id.to_string()),
            payload: PagerDutyPayload {
                summary: "Incident acknowledged".to_string(),
                source: "llm-incident-manager".to_string(),
                severity: "info".to_string(),
                timestamp: None,
                component: None,
                group: None,
                class: None,
                custom_details: None,
            },
            links: None,
            images: None,
        };

        self.send_event(&event).await?;
        Ok(())
    }

    /// Resolve an incident in PagerDuty
    pub async fn resolve_incident(&self, incident_id: &str) -> Result<()> {
        let event = PagerDutyEvent {
            routing_key: self.integration_key.clone(),
            event_action: "resolve".to_string(),
            dedup_key: Some(incident_id.to_string()),
            payload: PagerDutyPayload {
                summary: "Incident resolved".to_string(),
                source: "llm-incident-manager".to_string(),
                severity: "info".to_string(),
                timestamp: None,
                component: None,
                group: None,
                class: None,
                custom_details: None,
            },
            links: None,
            images: None,
        };

        self.send_event(&event).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};

    #[test]
    fn test_pagerduty_sender_creation() {
        let sender = PagerDutySender::new(
            "test-integration-key".to_string(),
            "https://events.pagerduty.com/v2/enqueue".to_string(),
        );
        assert!(sender.is_ok());
    }

    #[test]
    fn test_pagerduty_sender_validation() {
        let result = PagerDutySender::new(
            "".to_string(),
            "https://events.pagerduty.com/v2/enqueue".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_build_pagerduty_event() {
        let sender = PagerDutySender::new(
            "test-key".to_string(),
            "https://events.pagerduty.com/v2/enqueue".to_string(),
        )
        .unwrap();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P0,
            IncidentType::Infrastructure,
        );

        let event = sender
            .build_pagerduty_event(&incident, "service-key", "incident-key")
            .unwrap();

        assert_eq!(event.routing_key, "service-key");
        assert_eq!(event.event_action, "trigger");
        assert_eq!(event.dedup_key, Some("incident-key".to_string()));
        assert_eq!(event.payload.summary, "Test Incident");
        assert_eq!(event.payload.severity, "critical");
        assert_eq!(event.payload.source, "test-source");
    }

    #[test]
    fn test_severity_mapping() {
        let sender = PagerDutySender::new(
            "test-key".to_string(),
            "https://events.pagerduty.com/v2/enqueue".to_string(),
        )
        .unwrap();

        let test_cases = vec![
            (Severity::P0, "critical"),
            (Severity::P1, "error"),
            (Severity::P2, "warning"),
            (Severity::P3, "info"),
            (Severity::P4, "info"),
        ];

        for (severity, expected_pd_severity) in test_cases {
            let incident = Incident::new(
                "test".to_string(),
                "Test".to_string(),
                "Desc".to_string(),
                severity,
                IncidentType::Infrastructure,
            );

            let event = sender
                .build_pagerduty_event(&incident, "", "")
                .unwrap();

            assert_eq!(event.payload.severity, expected_pd_severity);
        }
    }

    #[test]
    fn test_event_action_by_state() {
        let sender = PagerDutySender::new(
            "test-key".to_string(),
            "https://events.pagerduty.com/v2/enqueue".to_string(),
        )
        .unwrap();

        let test_cases = vec![
            (IncidentState::Detected, "trigger"),
            (IncidentState::Triaged, "trigger"),
            (IncidentState::Investigating, "acknowledge"),
            (IncidentState::Remediating, "acknowledge"),
            (IncidentState::Resolved, "resolve"),
            (IncidentState::Closed, "resolve"),
        ];

        for (state, expected_action) in test_cases {
            let mut incident = Incident::new(
                "test".to_string(),
                "Test".to_string(),
                "Desc".to_string(),
                Severity::P1,
                IncidentType::Infrastructure,
            );
            incident.state = state;

            let event = sender
                .build_pagerduty_event(&incident, "", "")
                .unwrap();

            assert_eq!(event.event_action, expected_action);
        }
    }
}
