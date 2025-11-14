use crate::error::{AppError, Result};
use crate::models::{Incident, Notification, NotificationStatus};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

/// Slack notification sender
#[derive(Clone)]
pub struct SlackSender {
    pub(crate) webhook_url: Option<String>,
    pub(crate) bot_token: Option<String>,
    pub(crate) client: Client,
    pub(crate) default_channel: Option<String>,
}

#[derive(Debug, Serialize)]
struct SlackWebhookPayload {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<SlackAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocks: Option<Vec<SlackBlock>>,
}

#[derive(Debug, Serialize)]
struct SlackAttachment {
    color: String,
    title: String,
    text: String,
    fields: Vec<SlackField>,
    footer: String,
    ts: i64,
}

#[derive(Debug, Serialize)]
struct SlackField {
    title: String,
    value: String,
    short: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum SlackBlock {
    #[serde(rename = "header")]
    Header { text: SlackText },
    #[serde(rename = "section")]
    Section {
        text: SlackText,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<SlackText>>,
    },
    #[serde(rename = "divider")]
    Divider {},
    #[serde(rename = "context")]
    Context { elements: Vec<SlackText> },
}

#[derive(Debug, Serialize)]
struct SlackText {
    #[serde(rename = "type")]
    text_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SlackResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

impl SlackSender {
    /// Create a new Slack sender
    pub fn new(
        webhook_url: Option<String>,
        bot_token: Option<String>,
        default_channel: Option<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AppError::Configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            webhook_url,
            bot_token,
            client,
            default_channel,
        })
    }

    /// Send a notification to Slack
    pub async fn send(&self, notification: &mut Notification, incident: &Incident) -> Result<()> {
        // Extract channel and message from notification
        let (channel, message) = match &notification.channel {
            crate::models::NotificationChannel::Slack { channel, message } => {
                (Some(channel.clone()), message.clone())
            }
            _ => {
                return Err(AppError::Validation(
                    "Invalid notification channel type for Slack".to_string(),
                ))
            }
        };

        notification.status = NotificationStatus::Sending;

        // Build rich Slack message
        let payload = self.build_slack_payload(incident, &channel, &message)?;

        // Send via webhook or API
        let result = if let Some(webhook_url) = &self.webhook_url {
            self.send_via_webhook(webhook_url, &payload).await
        } else if let Some(bot_token) = &self.bot_token {
            self.send_via_api(bot_token, &payload).await
        } else {
            Err(AppError::Configuration(
                "No Slack webhook URL or bot token configured".to_string(),
            ))
        };

        match result {
            Ok(_) => {
                notification.status = NotificationStatus::Sent;
                notification.sent_at = Some(Utc::now());
                info!(
                    notification_id = %notification.id,
                    incident_id = %incident.id,
                    channel = ?channel,
                    "Slack notification sent successfully"
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
                    "Failed to send Slack notification"
                );
                Err(e)
            }
        }
    }

    /// Build Slack message payload with rich formatting
    fn build_slack_payload(
        &self,
        incident: &Incident,
        channel: &Option<String>,
        message: &str,
    ) -> Result<SlackWebhookPayload> {
        let severity_color = match incident.severity {
            crate::models::Severity::P0 => "#d00000", // Red
            crate::models::Severity::P1 => "#ff6b35", // Orange
            crate::models::Severity::P2 => "#f7b801", // Yellow
            crate::models::Severity::P3 => "#0077b6", // Blue
            crate::models::Severity::P4 => "#00b4d8", // Light blue
        };

        let severity_emoji = match incident.severity {
            crate::models::Severity::P0 => "🔴",
            crate::models::Severity::P1 => "🟠",
            crate::models::Severity::P2 => "🟡",
            crate::models::Severity::P3 => "🔵",
            crate::models::Severity::P4 => "⚪",
        };

        // Use blocks for modern Slack formatting
        let blocks = vec![
            SlackBlock::Header {
                text: SlackText {
                    text_type: "plain_text".to_string(),
                    text: format!("{} Incident: {}", severity_emoji, incident.title),
                },
            },
            SlackBlock::Section {
                text: SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: message.to_string(),
                },
                fields: None,
            },
            SlackBlock::Section {
                text: SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!("*Description:*\n{}", incident.description),
                },
                fields: Some(vec![
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Severity:*\n{:?}", incident.severity),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*State:*\n{:?}", incident.state),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Type:*\n{:?}", incident.incident_type),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Source:*\n{}", incident.source),
                    },
                ]),
            },
            SlackBlock::Divider {},
            SlackBlock::Context {
                elements: vec![SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!("Incident ID: `{}` | Created: {}", incident.id, incident.created_at.format("%Y-%m-%d %H:%M:%S UTC")),
                }],
            },
        ];

        // Fallback attachment for older clients
        let attachment = SlackAttachment {
            color: severity_color.to_string(),
            title: incident.title.clone(),
            text: incident.description.clone(),
            fields: vec![
                SlackField {
                    title: "Severity".to_string(),
                    value: format!("{:?}", incident.severity),
                    short: true,
                },
                SlackField {
                    title: "State".to_string(),
                    value: format!("{:?}", incident.state),
                    short: true,
                },
                SlackField {
                    title: "Type".to_string(),
                    value: format!("{:?}", incident.incident_type),
                    short: true,
                },
                SlackField {
                    title: "Source".to_string(),
                    value: incident.source.clone(),
                    short: true,
                },
            ],
            footer: "LLM Incident Manager".to_string(),
            ts: incident.created_at.timestamp(),
        };

        Ok(SlackWebhookPayload {
            text: message.to_string(),
            channel: channel.clone().or_else(|| self.default_channel.clone()),
            blocks: Some(blocks),
            attachments: Some(vec![attachment]),
        })
    }

    /// Send notification via webhook
    async fn send_via_webhook(
        &self,
        webhook_url: &str,
        payload: &SlackWebhookPayload,
    ) -> Result<()> {
        let response = self
            .client
            .post(webhook_url)
            .json(payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send Slack webhook: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!(
                "Slack webhook failed with status {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Send notification via Slack API
    async fn send_via_api(&self, bot_token: &str, payload: &SlackWebhookPayload) -> Result<()> {
        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(bot_token)
            .json(payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to call Slack API: {}", e)))?;

        let slack_response: SlackResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse Slack response: {}", e)))?;

        if !slack_response.ok {
            return Err(AppError::Internal(format!(
                "Slack API error: {}",
                slack_response.error.unwrap_or_else(|| "Unknown".to_string())
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};
    use uuid::Uuid;

    #[test]
    fn test_slack_sender_creation() {
        let sender = SlackSender::new(
            Some("https://hooks.slack.com/test".to_string()),
            None,
            Some("#incidents".to_string()),
        );
        assert!(sender.is_ok());
    }

    #[test]
    fn test_build_slack_payload() {
        let sender = SlackSender::new(
            Some("https://hooks.slack.com/test".to_string()),
            None,
            Some("#incidents".to_string()),
        )
        .unwrap();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let payload = sender
            .build_slack_payload(&incident, &Some("#test".to_string()), "Test message")
            .unwrap();

        assert_eq!(payload.text, "Test message");
        assert_eq!(payload.channel, Some("#test".to_string()));
        assert!(payload.blocks.is_some());
        assert!(payload.attachments.is_some());
    }

    #[test]
    fn test_severity_colors() {
        let sender = SlackSender::new(
            Some("https://hooks.slack.com/test".to_string()),
            None,
            None,
        )
        .unwrap();

        let incidents = vec![
            (Severity::P0, "#d00000"),
            (Severity::P1, "#ff6b35"),
            (Severity::P2, "#f7b801"),
            (Severity::P3, "#0077b6"),
            (Severity::P4, "#00b4d8"),
        ];

        for (severity, expected_color) in incidents {
            let incident = Incident::new(
                "test".to_string(),
                "Test".to_string(),
                "Desc".to_string(),
                severity,
                IncidentType::Infrastructure,
            );

            let payload = sender
                .build_slack_payload(&incident, &None, "Test")
                .unwrap();

            let attachment = &payload.attachments.as_ref().unwrap()[0];
            assert_eq!(attachment.color, expected_color);
        }
    }
}
