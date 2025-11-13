use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Notification to be sent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub channel: NotificationChannel,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub status: NotificationStatus,
    pub retry_count: u32,
    pub error: Option<String>,
}

impl Notification {
    /// Create a new notification
    pub fn new(
        incident_id: Uuid,
        channel: NotificationChannel,
        recipient: String,
        subject: String,
        body: String,
    ) -> Self {
        let channel = match channel {
            NotificationChannel::Email { .. } => NotificationChannel::Email {
                to: vec![recipient],
                subject,
                body,
            },
            NotificationChannel::Webhook { .. } => NotificationChannel::Webhook {
                url: recipient,
                payload: serde_json::json!({
                    "subject": subject,
                    "body": body,
                }),
            },
            NotificationChannel::Slack { .. } => NotificationChannel::Slack {
                channel: recipient,
                message: format!("{}\n\n{}", subject, body),
            },
            _ => channel,
        };

        Self {
            id: Uuid::new_v4(),
            incident_id,
            channel,
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationChannel {
    Slack { channel: String, message: String },
    Email { to: Vec<String>, subject: String, body: String },
    Webhook { url: String, payload: serde_json::Value },
    Pagerduty { service_key: String, incident_key: String },
    Custom { handler: String, config: HashMap<String, String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    Pending,
    Sending,
    Sent,
    Failed,
    Cancelled,
}

/// Notification template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub name: String,
    pub channel_type: String,
    pub template: String,
    pub variables: Vec<String>,
}

impl NotificationTemplate {
    /// Render template with provided variables
    pub fn render(&self, vars: &HashMap<String, String>) -> Result<String, String> {
        let mut result = self.template.clone();

        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Check for any unreplaced variables
        if result.contains("{{") {
            return Err("Template contains unreplaced variables".to_string());
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = Notification {
            id: Uuid::new_v4(),
            incident_id: Uuid::new_v4(),
            channel: NotificationChannel::Slack {
                channel: "#incidents".to_string(),
                message: "New incident created".to_string(),
            },
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            error: None,
        };

        assert_eq!(notification.status, NotificationStatus::Pending);
        assert!(notification.sent_at.is_none());
        assert_eq!(notification.retry_count, 0);
    }

    #[test]
    fn test_template_rendering() {
        let template = NotificationTemplate {
            id: Uuid::new_v4(),
            name: "Incident Alert".to_string(),
            channel_type: "slack".to_string(),
            template: "New incident: {{title}} - Severity: {{severity}}".to_string(),
            variables: vec!["title".to_string(), "severity".to_string()],
        };

        let mut vars = HashMap::new();
        vars.insert("title".to_string(), "API Down".to_string());
        vars.insert("severity".to_string(), "P0".to_string());

        let result = template.render(&vars).unwrap();
        assert_eq!(result, "New incident: API Down - Severity: P0");
    }

    #[test]
    fn test_template_missing_variables() {
        let template = NotificationTemplate {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            channel_type: "slack".to_string(),
            template: "Value: {{missing}}".to_string(),
            variables: vec!["missing".to_string()],
        };

        let vars = HashMap::new();
        let result = template.render(&vars);
        assert!(result.is_err());
    }
}
