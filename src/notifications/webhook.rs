use crate::error::{AppError, Result};
use crate::models::{Incident, Notification, NotificationStatus};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

/// Webhook notification sender
#[derive(Clone)]
pub struct WebhookSender {
    pub(crate) client: Client,
    pub(crate) timeout_secs: u64,
}

#[derive(Debug, Serialize)]
struct WebhookPayload {
    event_type: String,
    timestamp: String,
    incident: IncidentWebhookData,
    notification_id: String,
}

#[derive(Debug, Serialize)]
struct IncidentWebhookData {
    id: String,
    title: String,
    description: String,
    severity: String,
    state: String,
    incident_type: String,
    source: String,
    created_at: String,
    updated_at: String,
    affected_resources: Vec<String>,
    assignees: Vec<String>,
    labels: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<ResolutionWebhookData>,
}

#[derive(Debug, Serialize)]
struct ResolutionWebhookData {
    resolved_by: String,
    resolved_at: String,
    method: String,
    notes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_cause: Option<String>,
}

impl WebhookSender {
    /// Create a new webhook sender
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| AppError::Configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            timeout_secs,
        })
    }

    /// Send a notification via webhook
    pub async fn send(&self, notification: &mut Notification, incident: &Incident) -> Result<()> {
        // Extract webhook details from notification
        let (url, custom_payload) = match &notification.channel {
            crate::models::NotificationChannel::Webhook { url, payload } => {
                (url.clone(), Some(payload.clone()))
            }
            _ => {
                return Err(AppError::Validation(
                    "Invalid notification channel type for Webhook".to_string(),
                ))
            }
        };

        notification.status = NotificationStatus::Sending;

        // Build payload - use custom if provided, otherwise use default
        let payload_json = if let Some(custom) = custom_payload {
            custom
        } else {
            let default_payload = self.build_default_payload(notification, incident)?;
            serde_json::to_value(&default_payload)
                .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {}", e)))?
        };

        // Send webhook
        let result = self.send_webhook(&url, &payload_json).await;

        match result {
            Ok(response_body) => {
                notification.status = NotificationStatus::Sent;
                notification.sent_at = Some(Utc::now());
                info!(
                    notification_id = %notification.id,
                    incident_id = %incident.id,
                    url = %url,
                    response_length = response_body.len(),
                    "Webhook notification sent successfully"
                );
                Ok(())
            }
            Err(e) => {
                notification.status = NotificationStatus::Failed;
                notification.error = Some(e.to_string());
                error!(
                    notification_id = %notification.id,
                    incident_id = %incident.id,
                    url = %url,
                    error = %e,
                    "Failed to send webhook notification"
                );
                Err(e)
            }
        }
    }

    /// Build default webhook payload
    fn build_default_payload(
        &self,
        notification: &Notification,
        incident: &Incident,
    ) -> Result<WebhookPayload> {
        let event_type = match incident.state {
            crate::models::IncidentState::Detected => "incident.detected",
            crate::models::IncidentState::Triaged => "incident.triaged",
            crate::models::IncidentState::Investigating => "incident.investigating",
            crate::models::IncidentState::Remediating => "incident.remediating",
            crate::models::IncidentState::Resolved => "incident.resolved",
            crate::models::IncidentState::Closed => "incident.closed",
        };

        let resolution_data = incident.resolution.as_ref().map(|res| ResolutionWebhookData {
            resolved_by: res.resolved_by.clone(),
            resolved_at: res.resolved_at.to_rfc3339(),
            method: format!("{:?}", res.resolution_method),
            notes: res.notes.clone(),
            root_cause: res.root_cause.clone(),
        });

        Ok(WebhookPayload {
            event_type: event_type.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            incident: IncidentWebhookData {
                id: incident.id.to_string(),
                title: incident.title.clone(),
                description: incident.description.clone(),
                severity: format!("{:?}", incident.severity),
                state: format!("{:?}", incident.state),
                incident_type: format!("{:?}", incident.incident_type),
                source: incident.source.clone(),
                created_at: incident.created_at.to_rfc3339(),
                updated_at: incident.updated_at.to_rfc3339(),
                affected_resources: incident.affected_resources.clone(),
                assignees: incident.assignees.clone(),
                labels: incident.labels.clone(),
                resolution: resolution_data,
            },
            notification_id: notification.id.to_string(),
        })
    }

    /// Send HTTP POST request to webhook URL
    async fn send_webhook(&self, url: &str, payload: &serde_json::Value) -> Result<String> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "llm-incident-manager/1.0")
            .json(payload)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::Timeout(format!(
                        "Webhook request timed out after {} seconds",
                        self.timeout_secs
                    ))
                } else if e.is_connect() {
                    AppError::Internal(format!("Failed to connect to webhook URL: {}", e))
                } else {
                    AppError::Internal(format!("Webhook request failed: {}", e))
                }
            })?;

        let status = response.status();

        // Read response body
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::new());

        // Check for success status codes (2xx)
        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "Webhook returned non-success status {}: {}",
                status,
                if body.is_empty() {
                    "No response body"
                } else {
                    &body
                }
            )));
        }

        Ok(body)
    }

    /// Send a custom webhook with arbitrary payload
    pub async fn send_custom(
        &self,
        url: &str,
        payload: &serde_json::Value,
        headers: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<String> {
        let mut request = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "llm-incident-manager/1.0");

        // Add custom headers if provided
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                request = request.header(key, value);
            }
        }

        let response = request
            .json(payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Custom webhook failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::new());

        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "Custom webhook returned status {}: {}",
                status, body
            )));
        }

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, NotificationChannel, Severity};
    use uuid::Uuid;

    #[test]
    fn test_webhook_sender_creation() {
        let sender = WebhookSender::new(10);
        assert!(sender.is_ok());
    }

    #[test]
    fn test_build_default_payload() {
        let sender = WebhookSender::new(10).unwrap();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let notification = Notification {
            id: Uuid::new_v4(),
            incident_id: incident.id,
            channel: NotificationChannel::Webhook {
                url: "https://example.com/webhook".to_string(),
                payload: serde_json::json!({}),
            },
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            error: None,
        };

        let payload = sender.build_default_payload(&notification, &incident).unwrap();

        assert_eq!(payload.event_type, "incident.detected");
        assert_eq!(payload.incident.title, "Test Incident");
        assert_eq!(payload.incident.severity, "P1");
        assert_eq!(payload.incident.source, "test-source");
    }

    #[test]
    fn test_event_type_mapping() {
        let sender = WebhookSender::new(10).unwrap();

        let test_cases = vec![
            (IncidentState::Detected, "incident.detected"),
            (IncidentState::Triaged, "incident.triaged"),
            (IncidentState::Investigating, "incident.investigating"),
            (IncidentState::Remediating, "incident.remediating"),
            (IncidentState::Resolved, "incident.resolved"),
            (IncidentState::Closed, "incident.closed"),
        ];

        for (state, expected_event) in test_cases {
            let mut incident = Incident::new(
                "test".to_string(),
                "Test".to_string(),
                "Desc".to_string(),
                Severity::P1,
                IncidentType::Infrastructure,
            );
            incident.state = state;

            let notification = Notification {
                id: Uuid::new_v4(),
                incident_id: incident.id,
                channel: NotificationChannel::Webhook {
                    url: "https://example.com".to_string(),
                    payload: serde_json::json!({}),
                },
                created_at: Utc::now(),
                sent_at: None,
                status: NotificationStatus::Pending,
                retry_count: 0,
                error: None,
            };

            let payload = sender.build_default_payload(&notification, &incident).unwrap();
            assert_eq!(payload.event_type, expected_event);
        }
    }

    #[test]
    fn test_payload_serialization() {
        let sender = WebhookSender::new(10).unwrap();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P0,
            IncidentType::Security,
        );

        let notification = Notification {
            id: Uuid::new_v4(),
            incident_id: incident.id,
            channel: NotificationChannel::Webhook {
                url: "https://example.com/webhook".to_string(),
                payload: serde_json::json!({}),
            },
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            error: None,
        };

        let payload = sender.build_default_payload(&notification, &incident).unwrap();
        let json = serde_json::to_value(&payload).unwrap();

        assert!(json.get("event_type").is_some());
        assert!(json.get("timestamp").is_some());
        assert!(json.get("incident").is_some());
        assert!(json.get("notification_id").is_some());
    }
}
