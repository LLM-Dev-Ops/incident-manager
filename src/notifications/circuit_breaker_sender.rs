//! Circuit breaker wrappers for notification senders.

use crate::circuit_breaker::{
    get_circuit_breaker, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerResult,
};
use crate::error::Result;
use crate::models::Incident;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for sending notifications
#[async_trait]
pub trait NotificationSender: Send + Sync {
    /// Send a notification for an incident
    async fn send(&self, incident: &Incident) -> Result<()>;

    /// Send a custom message
    async fn send_message(&self, message: &str) -> Result<()>;
}

/// Wrapper that adds circuit breaker protection to notification senders
pub struct CircuitBreakerNotificationSender<S: NotificationSender> {
    inner: Arc<S>,
    breaker: Arc<CircuitBreaker>,
}

impl<S: NotificationSender + 'static> CircuitBreakerNotificationSender<S> {
    /// Create a new circuit breaker notification sender wrapper
    pub fn new(inner: S, name: impl Into<String>) -> Self {
        let config = CircuitBreakerConfig::for_notifications();
        let breaker = get_circuit_breaker(name, config);

        Self {
            inner: Arc::new(inner),
            breaker
        }
    }

    /// Create with custom circuit breaker configuration
    pub fn with_config(inner: S, name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let breaker = get_circuit_breaker(name, config);
        Self {
            inner: Arc::new(inner),
            breaker
        }
    }

    /// Get the underlying sender
    pub fn inner(&self) -> &Arc<S> {
        &self.inner
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

#[async_trait]
impl<S: NotificationSender + 'static> NotificationSender for CircuitBreakerNotificationSender<S> {
    async fn send(&self, incident: &Incident) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let incident = incident.clone();

        self.breaker
            .call(|| {
                Box::pin(async move {
                    inner
                        .send(&incident)
                        .await
                        .map_err(|e| NotificationErrorWrapper(e))
                })
            })
            .await
            .map_err(|e| match e {
                crate::circuit_breaker::CircuitBreakerError::Open(name) => {
                    crate::error::AppError::Internal(format!(
                        "Notification circuit breaker open: {}",
                        name
                    ))
                }
                crate::circuit_breaker::CircuitBreakerError::OperationFailed(msg) => {
                    crate::error::AppError::Internal(msg)
                }
                e => crate::error::AppError::Internal(e.to_string()),
            })
    }

    async fn send_message(&self, message: &str) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let message = message.to_string();

        self.breaker
            .call(|| {
                Box::pin(async move {
                    inner
                        .send_message(&message)
                        .await
                        .map_err(|e| NotificationErrorWrapper(e))
                })
            })
            .await
            .map_err(|e| match e {
                crate::circuit_breaker::CircuitBreakerError::Open(name) => {
                    crate::error::AppError::Internal(format!(
                        "Notification circuit breaker open: {}",
                        name
                    ))
                }
                crate::circuit_breaker::CircuitBreakerError::OperationFailed(msg) => {
                    crate::error::AppError::Internal(msg)
                }
                e => crate::error::AppError::Internal(e.to_string()),
            })
    }
}

/// Wrapper for notification errors to implement std::error::Error
#[derive(Debug)]
struct NotificationErrorWrapper(crate::error::AppError);

impl std::fmt::Display for NotificationErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NotificationErrorWrapper {}

/// Slack sender with circuit breaker
pub struct SlackSenderWithBreaker {
    sender: Arc<crate::notifications::SlackSender>,
    breaker: Arc<CircuitBreaker>,
}

impl SlackSenderWithBreaker {
    /// Create a new Slack sender with circuit breaker
    pub fn new(sender: crate::notifications::SlackSender) -> Self {
        let config = CircuitBreakerConfig::for_notifications();
        let breaker = get_circuit_breaker("slack-notifications", config);

        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        sender: crate::notifications::SlackSender,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("slack-notifications", config);
        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Send a notification with circuit breaker protection
    /// Note: This method takes ownership of the notification and returns it updated
    pub async fn send(&self, mut notification: crate::models::Notification, incident: &Incident) -> CircuitBreakerResult<crate::models::Notification> {
        let sender = Arc::clone(&self.sender);
        let incident = incident.clone();

        let result = self.breaker
            .call(|| {
                let sender = Arc::clone(&sender);
                let incident = incident.clone();
                let mut notif_clone = notification.clone();
                Box::pin(async move {
                    sender
                        .send(&mut notif_clone, &incident)
                        .await
                        .map(|_| notif_clone)
                        .map_err(|e| NotificationErrorWrapper(e))
                })
            })
            .await;

        match result {
            Ok(updated_notification) => {
                notification = updated_notification;
                Ok(notification)
            }
            Err(e) => {
                notification.status = crate::models::NotificationStatus::Failed;
                notification.error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

/// Email sender with circuit breaker
pub struct EmailSenderWithBreaker {
    sender: Arc<crate::notifications::EmailSender>,
    breaker: Arc<CircuitBreaker>,
}

impl EmailSenderWithBreaker {
    /// Create a new Email sender with circuit breaker
    pub fn new(sender: crate::notifications::EmailSender) -> Self {
        let config = CircuitBreakerConfig::for_notifications();
        let breaker = get_circuit_breaker("email-notifications", config);

        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        sender: crate::notifications::EmailSender,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("email-notifications", config);
        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Send a notification with circuit breaker protection
    /// Note: This method takes ownership of the notification and returns it updated
    pub async fn send(&self, mut notification: crate::models::Notification, incident: &Incident) -> CircuitBreakerResult<crate::models::Notification> {
        let sender = Arc::clone(&self.sender);
        let incident = incident.clone();

        let result = self.breaker
            .call(|| {
                let sender = Arc::clone(&sender);
                let incident = incident.clone();
                let mut notif_clone = notification.clone();
                Box::pin(async move {
                    sender
                        .send(&mut notif_clone, &incident)
                        .await
                        .map(|_| notif_clone)
                        .map_err(|e| NotificationErrorWrapper(e))
                })
            })
            .await;

        match result {
            Ok(updated_notification) => {
                notification = updated_notification;
                Ok(notification)
            }
            Err(e) => {
                notification.status = crate::models::NotificationStatus::Failed;
                notification.error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

/// PagerDuty sender with circuit breaker
pub struct PagerDutySenderWithBreaker {
    sender: Arc<crate::notifications::PagerDutySender>,
    breaker: Arc<CircuitBreaker>,
}

impl PagerDutySenderWithBreaker {
    /// Create a new PagerDuty sender with circuit breaker
    pub fn new(sender: crate::notifications::PagerDutySender) -> Self {
        let config = CircuitBreakerConfig::for_notifications();
        let breaker = get_circuit_breaker("pagerduty-notifications", config);

        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        sender: crate::notifications::PagerDutySender,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("pagerduty-notifications", config);
        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Send a notification with circuit breaker protection
    /// Note: This method takes ownership of the notification and returns it updated
    pub async fn send(&self, mut notification: crate::models::Notification, incident: &Incident) -> CircuitBreakerResult<crate::models::Notification> {
        let sender = Arc::clone(&self.sender);
        let incident = incident.clone();

        let result = self.breaker
            .call(|| {
                let sender = Arc::clone(&sender);
                let incident = incident.clone();
                let mut notif_clone = notification.clone();
                Box::pin(async move {
                    sender
                        .send(&mut notif_clone, &incident)
                        .await
                        .map(|_| notif_clone)
                        .map_err(|e| NotificationErrorWrapper(e))
                })
            })
            .await;

        match result {
            Ok(updated_notification) => {
                notification = updated_notification;
                Ok(notification)
            }
            Err(e) => {
                notification.status = crate::models::NotificationStatus::Failed;
                notification.error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

/// Webhook sender with circuit breaker
pub struct WebhookSenderWithBreaker {
    sender: Arc<crate::notifications::WebhookSender>,
    breaker: Arc<CircuitBreaker>,
}

impl WebhookSenderWithBreaker {
    /// Create a new Webhook sender with circuit breaker
    pub fn new(sender: crate::notifications::WebhookSender) -> Self {
        let config = CircuitBreakerConfig::for_notifications();
        let breaker = get_circuit_breaker("webhook-notifications", config);

        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        sender: crate::notifications::WebhookSender,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("webhook-notifications", config);
        Self {
            sender: Arc::new(sender),
            breaker
        }
    }

    /// Send a notification with circuit breaker protection
    /// Note: This method takes ownership of the notification and returns it updated
    pub async fn send(&self, mut notification: crate::models::Notification, incident: &Incident) -> CircuitBreakerResult<crate::models::Notification> {
        let sender = Arc::clone(&self.sender);
        let incident = incident.clone();

        let result = self.breaker
            .call(|| {
                let sender = Arc::clone(&sender);
                let incident = incident.clone();
                let mut notif_clone = notification.clone();
                Box::pin(async move {
                    sender
                        .send(&mut notif_clone, &incident)
                        .await
                        .map(|_| notif_clone)
                        .map_err(|e| NotificationErrorWrapper(e))
                })
            })
            .await;

        match result {
            Ok(updated_notification) => {
                notification = updated_notification;
                Ok(notification)
            }
            Err(e) => {
                notification.status = crate::models::NotificationStatus::Failed;
                notification.error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full tests require actual notification sender instances
    // These would typically be tested with mocks in a real implementation

    #[test]
    fn test_notification_sender_creation() {
        // Test that the types compile and can be created
        // Actual senders would require configuration
    }
}
