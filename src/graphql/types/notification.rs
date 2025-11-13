//! GraphQL types for notifications

use async_graphql::*;
use uuid::Uuid;

use crate::models;
use super::common::DateTimeScalar;

/// Notification object type
#[derive(Clone)]
pub struct Notification(pub models::Notification);

#[Object]
impl Notification {
    async fn id(&self) -> &Uuid {
        &self.0.id
    }

    async fn incident_id(&self) -> &Uuid {
        &self.0.incident_id
    }

    async fn channel(&self) -> NotificationChannel {
        NotificationChannel::from(self.0.channel.clone())
    }

    async fn created_at(&self) -> DateTimeScalar {
        self.0.created_at.into()
    }

    async fn sent_at(&self) -> Option<DateTimeScalar> {
        self.0.sent_at.map(|dt| dt.into())
    }

    async fn status(&self) -> NotificationStatus {
        NotificationStatus::from(self.0.status.clone())
    }

    async fn retry_count(&self) -> u32 {
        self.0.retry_count
    }

    async fn error(&self) -> Option<&str> {
        self.0.error.as_deref()
    }
}

/// Notification channel union
#[derive(Union, Clone)]
pub enum NotificationChannel {
    Slack(SlackChannel),
    Email(EmailChannel),
    Webhook(WebhookChannel),
    Pagerduty(PagerdutyChannel),
}

impl From<models::NotificationChannel> for NotificationChannel {
    fn from(channel: models::NotificationChannel) -> Self {
        match channel {
            models::NotificationChannel::Slack { channel, message } => {
                NotificationChannel::Slack(SlackChannel { channel, message })
            }
            models::NotificationChannel::Email { to, subject, body } => {
                NotificationChannel::Email(EmailChannel { to, subject, body })
            }
            models::NotificationChannel::Webhook { url, payload } => {
                NotificationChannel::Webhook(WebhookChannel {
                    url,
                    payload: serde_json::to_string(&payload).unwrap_or_default(),
                })
            }
            models::NotificationChannel::Pagerduty {
                service_key,
                incident_key,
            } => NotificationChannel::Pagerduty(PagerdutyChannel {
                service_key,
                incident_key,
            }),
            models::NotificationChannel::Custom { handler, config } => {
                // For custom channels, represent as a webhook with JSON payload
                NotificationChannel::Webhook(WebhookChannel {
                    url: format!("custom://{}", handler),
                    payload: serde_json::to_string(&config).unwrap_or_default(),
                })
            }
        }
    }
}

/// Slack notification channel
#[derive(SimpleObject, Clone)]
pub struct SlackChannel {
    pub channel: String,
    pub message: String,
}

/// Email notification channel
#[derive(SimpleObject, Clone)]
pub struct EmailChannel {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
}

/// Webhook notification channel
#[derive(SimpleObject, Clone)]
pub struct WebhookChannel {
    pub url: String,
    pub payload: String,
}

/// PagerDuty notification channel
#[derive(SimpleObject, Clone)]
pub struct PagerdutyChannel {
    pub service_key: String,
    pub incident_key: String,
}

/// Notification status enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum NotificationStatus {
    Pending,
    Sending,
    Sent,
    Failed,
    Cancelled,
}

impl From<models::NotificationStatus> for NotificationStatus {
    fn from(status: models::NotificationStatus) -> Self {
        match status {
            models::NotificationStatus::Pending => NotificationStatus::Pending,
            models::NotificationStatus::Sending => NotificationStatus::Sending,
            models::NotificationStatus::Sent => NotificationStatus::Sent,
            models::NotificationStatus::Failed => NotificationStatus::Failed,
            models::NotificationStatus::Cancelled => NotificationStatus::Cancelled,
        }
    }
}
