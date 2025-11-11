use crate::config::NotificationConfig;
use crate::error::{AppError, Result};
use crate::models::{Incident, Notification, NotificationChannel, NotificationStatus};
use crate::notifications::{EmailSender, PagerDutySender, SlackSender, WebhookSender};
use crate::state::IncidentStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Notification service that dispatches notifications to various channels
pub struct NotificationService {
    config: NotificationConfig,
    slack_sender: Option<SlackSender>,
    email_sender: Option<EmailSender>,
    pagerduty_sender: Option<PagerDutySender>,
    webhook_sender: WebhookSender,
    store: Arc<dyn IncidentStore>,
    notification_tx: mpsc::Sender<Notification>,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(config: NotificationConfig, store: Arc<dyn IncidentStore>) -> Result<Self> {
        // Initialize Slack sender if enabled
        let slack_sender = if config.slack_enabled {
            let webhook_url = config
                .slack_webhook_env
                .as_ref()
                .and_then(|env_var| std::env::var(env_var).ok());

            let bot_token = config
                .slack_bot_token_env
                .as_ref()
                .and_then(|env_var| std::env::var(env_var).ok());

            if webhook_url.is_some() || bot_token.is_some() {
                Some(SlackSender::new(
                    webhook_url,
                    bot_token,
                    config.slack_default_channel.clone(),
                )?)
            } else {
                warn!("Slack notifications enabled but no webhook URL or bot token configured");
                None
            }
        } else {
            None
        };

        // Initialize Email sender if enabled
        let email_sender = if config.email_enabled {
            if let (Some(server), Some(from)) = (config.smtp_server.as_ref(), config.email_from.as_ref()) {
                let username = config
                    .smtp_username_env
                    .as_ref()
                    .and_then(|env_var| std::env::var(env_var).ok());

                let password = config
                    .smtp_password_env
                    .as_ref()
                    .and_then(|env_var| std::env::var(env_var).ok());

                Some(EmailSender::new(
                    server.clone(),
                    config.smtp_port,
                    username,
                    password,
                    from.clone(),
                    config.email_from_name.clone(),
                    config.smtp_use_tls,
                )?)
            } else {
                warn!("Email notifications enabled but SMTP server or from address not configured");
                None
            }
        } else {
            None
        };

        // Initialize PagerDuty sender if enabled
        let pagerduty_sender = if config.pagerduty_enabled {
            let integration_key = config
                .pagerduty_integration_key_env
                .as_ref()
                .and_then(|env_var| std::env::var(env_var).ok());

            if let Some(key) = integration_key {
                Some(PagerDutySender::new(key, config.pagerduty_api_url.clone())?)
            } else {
                warn!("PagerDuty notifications enabled but no integration key configured");
                None
            }
        } else {
            None
        };

        // Initialize Webhook sender
        let webhook_sender = WebhookSender::new(config.webhook_timeout_secs)?;

        // Create notification queue channel
        let (notification_tx, notification_rx) = mpsc::channel(config.queue_size);

        let service = Self {
            config: config.clone(),
            slack_sender,
            email_sender,
            pagerduty_sender,
            webhook_sender,
            store,
            notification_tx,
        };

        // Spawn worker threads
        for worker_id in 0..config.worker_threads {
            service.spawn_worker(worker_id, notification_rx.clone());
        }

        info!(
            slack_enabled = service.slack_sender.is_some(),
            email_enabled = service.email_sender.is_some(),
            pagerduty_enabled = service.pagerduty_sender.is_some(),
            webhook_enabled = config.webhook_enabled,
            workers = config.worker_threads,
            queue_size = config.queue_size,
            "Notification service initialized"
        );

        Ok(service)
    }

    /// Queue a notification for sending
    pub async fn queue_notification(&self, notification: Notification) -> Result<()> {
        self.notification_tx
            .send(notification)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to queue notification: {}", e)))?;

        Ok(())
    }

    /// Create and queue notifications for an incident based on channels
    pub async fn notify_incident(
        &self,
        incident: &Incident,
        channels: Vec<NotificationChannel>,
        message: &str,
    ) -> Result<Vec<Uuid>> {
        let mut notification_ids = Vec::new();

        for channel in channels {
            let notification = Notification {
                id: Uuid::new_v4(),
                incident_id: incident.id,
                channel: channel.clone(),
                created_at: chrono::Utc::now(),
                sent_at: None,
                status: NotificationStatus::Pending,
                retry_count: 0,
                error: None,
            };

            notification_ids.push(notification.id);
            self.queue_notification(notification).await?;
        }

        info!(
            incident_id = %incident.id,
            notification_count = notification_ids.len(),
            "Queued notifications for incident"
        );

        Ok(notification_ids)
    }

    /// Notify incident detected
    pub async fn notify_incident_detected(&self, incident: &Incident) -> Result<Vec<Uuid>> {
        let mut channels = Vec::new();

        // Add Slack notification if enabled
        if self.slack_sender.is_some() {
            let channel = self.config.slack_default_channel.clone()
                .unwrap_or_else(|| "#incidents".to_string());

            channels.push(NotificationChannel::Slack {
                channel: channel.clone(),
                message: format!(
                    "🚨 New {} incident detected: {}",
                    format!("{:?}", incident.severity).to_uppercase(),
                    incident.title
                ),
            });
        }

        // Add PagerDuty notification for high severity incidents
        if self.pagerduty_sender.is_some()
            && matches!(
                incident.severity,
                crate::models::Severity::P0 | crate::models::Severity::P1
            )
        {
            channels.push(NotificationChannel::Pagerduty {
                service_key: String::new(), // Will use default integration key
                incident_key: incident.id.to_string(),
            });
        }

        self.notify_incident(incident, channels, "Incident detected").await
    }

    /// Notify incident resolved
    pub async fn notify_incident_resolved(&self, incident: &Incident) -> Result<Vec<Uuid>> {
        let mut channels = Vec::new();

        // Add Slack notification if enabled
        if self.slack_sender.is_some() {
            let channel = self.config.slack_default_channel.clone()
                .unwrap_or_else(|| "#incidents".to_string());

            channels.push(NotificationChannel::Slack {
                channel: channel.clone(),
                message: format!("✅ Incident resolved: {}", incident.title),
            });
        }

        // Resolve in PagerDuty
        if self.pagerduty_sender.is_some() {
            channels.push(NotificationChannel::Pagerduty {
                service_key: String::new(),
                incident_key: incident.id.to_string(),
            });
        }

        self.notify_incident(incident, channels, "Incident resolved").await
    }

    /// Spawn a notification worker
    fn spawn_worker(&self, worker_id: usize, mut notification_rx: mpsc::Receiver<Notification>) {
        let slack_sender = self.slack_sender.clone();
        let email_sender = self.email_sender.clone();
        let pagerduty_sender = self.pagerduty_sender.clone();
        let webhook_sender = self.webhook_sender.clone();
        let store = self.store.clone();
        let max_retries = self.config.max_retries;
        let retry_backoff = self.config.retry_backoff_secs;

        tokio::spawn(async move {
            info!(worker_id, "Notification worker started");

            while let Some(mut notification) = notification_rx.recv().await {
                // Get incident from store
                let incident = match store.get_incident(&notification.incident_id).await {
                    Ok(Some(inc)) => inc,
                    Ok(None) => {
                        error!(
                            notification_id = %notification.id,
                            incident_id = %notification.incident_id,
                            "Incident not found for notification"
                        );
                        continue;
                    }
                    Err(e) => {
                        error!(
                            notification_id = %notification.id,
                            incident_id = %notification.incident_id,
                            error = %e,
                            "Failed to fetch incident"
                        );
                        continue;
                    }
                };

                // Try to send notification with retries
                let mut attempts = 0;
                let mut success = false;

                while attempts <= max_retries && !success {
                    if attempts > 0 {
                        // Exponential backoff
                        let delay = retry_backoff * 2_u64.pow(attempts - 1);
                        info!(
                            notification_id = %notification.id,
                            attempt = attempts + 1,
                            delay_secs = delay,
                            "Retrying notification"
                        );
                        sleep(Duration::from_secs(delay)).await;
                    }

                    notification.retry_count = attempts;

                    // Send based on channel type
                    let result = match &notification.channel {
                        NotificationChannel::Slack { .. } => {
                            if let Some(ref sender) = slack_sender {
                                sender.send(&mut notification, &incident).await
                            } else {
                                Err(AppError::Configuration(
                                    "Slack sender not configured".to_string(),
                                ))
                            }
                        }
                        NotificationChannel::Email { .. } => {
                            if let Some(ref sender) = email_sender {
                                sender.send(&mut notification, &incident).await
                            } else {
                                Err(AppError::Configuration(
                                    "Email sender not configured".to_string(),
                                ))
                            }
                        }
                        NotificationChannel::Pagerduty { .. } => {
                            if let Some(ref sender) = pagerduty_sender {
                                sender.send(&mut notification, &incident).await
                            } else {
                                Err(AppError::Configuration(
                                    "PagerDuty sender not configured".to_string(),
                                ))
                            }
                        }
                        NotificationChannel::Webhook { .. } => {
                            webhook_sender.send(&mut notification, &incident).await
                        }
                        NotificationChannel::Custom { .. } => {
                            warn!("Custom notification channels not yet implemented");
                            Err(AppError::Configuration(
                                "Custom channels not implemented".to_string(),
                            ))
                        }
                    };

                    match result {
                        Ok(_) => {
                            success = true;
                            info!(
                                notification_id = %notification.id,
                                worker_id,
                                attempts = attempts + 1,
                                "Notification sent successfully"
                            );
                        }
                        Err(e) => {
                            error!(
                                notification_id = %notification.id,
                                worker_id,
                                attempts = attempts + 1,
                                error = %e,
                                "Failed to send notification"
                            );
                        }
                    }

                    attempts += 1;
                }

                if !success {
                    notification.status = NotificationStatus::Failed;
                    error!(
                        notification_id = %notification.id,
                        worker_id,
                        total_attempts = attempts,
                        "Notification failed after all retries"
                    );
                }
            }

            info!(worker_id, "Notification worker stopped");
        });
    }

    /// Get notification statistics
    pub fn get_stats(&self) -> NotificationStats {
        NotificationStats {
            slack_enabled: self.slack_sender.is_some(),
            email_enabled: self.email_sender.is_some(),
            pagerduty_enabled: self.pagerduty_sender.is_some(),
            webhook_enabled: self.config.webhook_enabled,
            queue_capacity: self.config.queue_size,
            worker_count: self.config.worker_threads,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationStats {
    pub slack_enabled: bool,
    pub email_enabled: bool,
    pub pagerduty_enabled: bool,
    pub webhook_enabled: bool,
    pub queue_capacity: usize,
    pub worker_count: usize,
}

// Make senders cloneable for worker threads
impl Clone for SlackSender {
    fn clone(&self) -> Self {
        Self {
            webhook_url: self.webhook_url.clone(),
            bot_token: self.bot_token.clone(),
            client: self.client.clone(),
            default_channel: self.default_channel.clone(),
        }
    }
}

impl Clone for EmailSender {
    fn clone(&self) -> Self {
        Self {
            smtp_server: self.smtp_server.clone(),
            smtp_port: self.smtp_port,
            smtp_username: self.smtp_username.clone(),
            smtp_password: self.smtp_password.clone(),
            from_email: self.from_email.clone(),
            from_name: self.from_name.clone(),
            use_tls: self.use_tls,
        }
    }
}

impl Clone for PagerDutySender {
    fn clone(&self) -> Self {
        Self {
            integration_key: self.integration_key.clone(),
            api_url: self.api_url.clone(),
            client: self.client.clone(),
        }
    }
}

impl Clone for WebhookSender {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            timeout_secs: self.timeout_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};
    use crate::state::InMemoryStore;

    fn create_test_config() -> NotificationConfig {
        NotificationConfig {
            slack_enabled: false,
            slack_webhook_env: None,
            slack_bot_token_env: None,
            slack_default_channel: Some("#test".to_string()),
            email_enabled: false,
            smtp_server: None,
            smtp_port: 587,
            smtp_use_tls: true,
            smtp_username_env: None,
            smtp_password_env: None,
            email_from: None,
            email_from_name: None,
            pagerduty_enabled: false,
            pagerduty_api_token_env: None,
            pagerduty_integration_key_env: None,
            pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
            webhook_enabled: true,
            default_webhook_url: None,
            webhook_timeout_secs: 10,
            max_retries: 3,
            retry_backoff_secs: 5,
            queue_size: 1000,
            worker_threads: 2,
        }
    }

    #[tokio::test]
    async fn test_notification_service_creation() {
        let config = create_test_config();
        let store = Arc::new(InMemoryStore::new());

        let service = NotificationService::new(config, store);
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_queue_notification() {
        let config = create_test_config();
        let store = Arc::new(InMemoryStore::new());
        let service = NotificationService::new(config, store).unwrap();

        let incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let notification = Notification {
            id: Uuid::new_v4(),
            incident_id: incident.id,
            channel: NotificationChannel::Webhook {
                url: "https://example.com".to_string(),
                payload: serde_json::json!({}),
            },
            created_at: chrono::Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            error: None,
        };

        let result = service.queue_notification(notification).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_notification_stats() {
        let config = create_test_config();
        let store = Arc::new(InMemoryStore::new());
        let service = NotificationService::new(config, store).unwrap();

        let stats = service.get_stats();
        assert!(!stats.slack_enabled);
        assert!(!stats.email_enabled);
        assert!(!stats.pagerduty_enabled);
        assert!(stats.webhook_enabled);
        assert_eq!(stats.queue_capacity, 1000);
        assert_eq!(stats.worker_count, 2);
    }
}
