use crate::error::{AppError, Result};
use crate::models::{Action, ActionType, NotificationChannel};
use crate::notifications::NotificationService;
use crate::playbooks::ExecutionContext;
use crate::state::IncidentStore;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

/// Action executor trait
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult>;
}

/// Result of action execution
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub success: bool,
    pub output: HashMap<String, JsonValue>,
    pub error: Option<String>,
}

impl ActionResult {
    pub fn success(output: HashMap<String, JsonValue>) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            output: HashMap::new(),
            error: Some(error),
        }
    }
}

/// Registry of action executors
pub struct ActionExecutorRegistry {
    executors: HashMap<ActionType, Arc<dyn ActionExecutor>>,
}

impl ActionExecutorRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }

    /// Register an executor for an action type
    pub fn register(&mut self, action_type: ActionType, executor: Arc<dyn ActionExecutor>) {
        self.executors.insert(action_type, executor);
    }

    /// Execute an action
    pub async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        if let Some(executor) = self.executors.get(&action.action_type) {
            executor.execute(action, context).await
        } else {
            Err(AppError::Configuration(format!(
                "No executor registered for action type: {:?}",
                action.action_type
            )))
        }
    }
}

/// Create default action executor registry with all standard executors
pub fn create_default_registry(
    notification_service: Option<Arc<NotificationService>>,
    store: Arc<dyn IncidentStore>,
) -> ActionExecutorRegistry {
    let mut registry = ActionExecutorRegistry::new();

    // Notification actions
    if let Some(notif_service) = notification_service {
        registry.register(
            ActionType::Slack,
            Arc::new(SlackActionExecutor::new(notif_service.clone())),
        );
        registry.register(
            ActionType::Email,
            Arc::new(EmailActionExecutor::new(notif_service.clone())),
        );
        registry.register(
            ActionType::Pagerduty,
            Arc::new(PagerdutyActionExecutor::new(notif_service.clone())),
        );
    }

    registry.register(ActionType::Webhook, Arc::new(WebhookActionExecutor::new()));

    // Workflow control
    registry.register(ActionType::Wait, Arc::new(WaitActionExecutor::new()));

    // Incident management
    registry.register(
        ActionType::IncidentResolve,
        Arc::new(IncidentResolveActionExecutor::new(store.clone())),
    );
    registry.register(
        ActionType::SeverityIncrease,
        Arc::new(SeverityChangeActionExecutor::new(store.clone(), true)),
    );
    registry.register(
        ActionType::SeverityDecrease,
        Arc::new(SeverityChangeActionExecutor::new(store, false)),
    );

    // Generic actions
    registry.register(ActionType::HttpRequest, Arc::new(HttpRequestActionExecutor::new()));

    registry
}

// ==================== Notification Action Executors ====================

/// Slack notification executor
struct SlackActionExecutor {
    notification_service: Arc<NotificationService>,
}

impl SlackActionExecutor {
    fn new(notification_service: Arc<NotificationService>) -> Self {
        Self { notification_service }
    }
}

#[async_trait]
impl ActionExecutor for SlackActionExecutor {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        let params = context.substitute_parameters(&action.parameters);

        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("'channel' parameter required".to_string()))?;

        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("'message' parameter required".to_string()))?;

        let incident = context.incident();
        let notification_channel = NotificationChannel::Slack {
            channel: channel.to_string(),
            message: message.to_string(),
        };

        match self
            .notification_service
            .notify_incident(incident, vec![notification_channel], message)
            .await
        {
            Ok(notification_ids) => {
                let mut output = HashMap::new();
                output.insert(
                    "notification_ids".to_string(),
                    JsonValue::Array(
                        notification_ids
                            .iter()
                            .map(|id| JsonValue::String(id.to_string()))
                            .collect(),
                    ),
                );
                Ok(ActionResult::success(output))
            }
            Err(e) => Ok(ActionResult::failure(format!("Slack notification failed: {}", e))),
        }
    }
}

/// Email notification executor
struct EmailActionExecutor {
    notification_service: Arc<NotificationService>,
}

impl EmailActionExecutor {
    fn new(notification_service: Arc<NotificationService>) -> Self {
        Self { notification_service }
    }
}

#[async_trait]
impl ActionExecutor for EmailActionExecutor {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        let params = context.substitute_parameters(&action.parameters);

        let to = params
            .get("to")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .ok_or_else(|| AppError::Validation("'to' parameter required as array".to_string()))?;

        let subject = params
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Incident Alert");

        let body = params
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("Incident notification");

        let incident = context.incident();
        let notification_channel = NotificationChannel::Email {
            to,
            subject: subject.to_string(),
            body: body.to_string(),
        };

        match self
            .notification_service
            .notify_incident(incident, vec![notification_channel], subject)
            .await
        {
            Ok(notification_ids) => {
                let mut output = HashMap::new();
                output.insert(
                    "notification_ids".to_string(),
                    JsonValue::Array(
                        notification_ids
                            .iter()
                            .map(|id| JsonValue::String(id.to_string()))
                            .collect(),
                    ),
                );
                Ok(ActionResult::success(output))
            }
            Err(e) => Ok(ActionResult::failure(format!("Email notification failed: {}", e))),
        }
    }
}

/// PagerDuty notification executor
struct PagerdutyActionExecutor {
    notification_service: Arc<NotificationService>,
}

impl PagerdutyActionExecutor {
    fn new(notification_service: Arc<NotificationService>) -> Self {
        Self { notification_service }
    }
}

#[async_trait]
impl ActionExecutor for PagerdutyActionExecutor {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        let params = context.substitute_parameters(&action.parameters);

        let service_key = params
            .get("service_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let incident = context.incident();
        let notification_channel = NotificationChannel::Pagerduty {
            service_key: service_key.to_string(),
            incident_key: incident.id.to_string(),
        };

        match self
            .notification_service
            .notify_incident(incident, vec![notification_channel], "PagerDuty alert")
            .await
        {
            Ok(notification_ids) => {
                let mut output = HashMap::new();
                output.insert(
                    "notification_ids".to_string(),
                    JsonValue::Array(
                        notification_ids
                            .iter()
                            .map(|id| JsonValue::String(id.to_string()))
                            .collect(),
                    ),
                );
                Ok(ActionResult::success(output))
            }
            Err(e) => Ok(ActionResult::failure(format!("PagerDuty notification failed: {}", e))),
        }
    }
}

// ==================== Webhook Action Executor ====================

struct WebhookActionExecutor {
    client: Client,
}

impl WebhookActionExecutor {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl ActionExecutor for WebhookActionExecutor {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        let params = context.substitute_parameters(&action.parameters);

        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("'url' parameter required".to_string()))?;

        let payload = params.get("payload").cloned().unwrap_or_else(|| {
            JsonValue::Object(serde_json::Map::new())
        });

        match self.client.post(url).json(&payload).send().await {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                let mut output = HashMap::new();
                output.insert("status_code".to_string(), JsonValue::Number(status.as_u16().into()));
                output.insert("response_body".to_string(), JsonValue::String(body.clone()));

                if status.is_success() {
                    Ok(ActionResult::success(output))
                } else {
                    Ok(ActionResult::failure(format!(
                        "Webhook returned non-success status {}: {}",
                        status, body
                    )))
                }
            }
            Err(e) => Ok(ActionResult::failure(format!("Webhook request failed: {}", e))),
        }
    }
}

// ==================== Workflow Control Executors ====================

struct WaitActionExecutor;

impl WaitActionExecutor {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ActionExecutor for WaitActionExecutor {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        let params = context.substitute_parameters(&action.parameters);

        let duration_secs = params
            .get("duration")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        info!("Waiting for {} seconds", duration_secs);
        sleep(Duration::from_secs(duration_secs)).await;

        let mut output = HashMap::new();
        output.insert("waited_seconds".to_string(), JsonValue::Number(duration_secs.into()));
        Ok(ActionResult::success(output))
    }
}

// ==================== Incident Management Executors ====================

struct IncidentResolveActionExecutor {
    store: Arc<dyn IncidentStore>,
}

impl IncidentResolveActionExecutor {
    fn new(store: Arc<dyn IncidentStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ActionExecutor for IncidentResolveActionExecutor {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        let params = context.substitute_parameters(&action.parameters);

        let notes = params
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("Resolved by playbook");

        let mut incident = context.incident().clone();
        incident.resolve(
            "playbook-engine".to_string(),
            crate::models::ResolutionMethod::Automated,
            notes.to_string(),
            params.get("root_cause").and_then(|v| v.as_str()).map(|s| s.to_string()),
        );

        self.store.update_incident(&incident).await?;

        let mut output = HashMap::new();
        output.insert("incident_resolved".to_string(), JsonValue::Bool(true));
        output.insert("incident_id".to_string(), JsonValue::String(incident.id.to_string()));
        Ok(ActionResult::success(output))
    }
}

struct SeverityChangeActionExecutor {
    store: Arc<dyn IncidentStore>,
    increase: bool,
}

impl SeverityChangeActionExecutor {
    fn new(store: Arc<dyn IncidentStore>, increase: bool) -> Self {
        Self { store, increase }
    }
}

#[async_trait]
impl ActionExecutor for SeverityChangeActionExecutor {
    async fn execute(&self, _action: &Action, _context: &mut ExecutionContext) -> Result<ActionResult> {
        let mut incident = _context.incident().clone();

        let old_severity = incident.severity.clone();
        let new_severity = if self.increase {
            match incident.severity {
                crate::models::Severity::P4 => crate::models::Severity::P3,
                crate::models::Severity::P3 => crate::models::Severity::P2,
                crate::models::Severity::P2 => crate::models::Severity::P1,
                crate::models::Severity::P1 => crate::models::Severity::P0,
                crate::models::Severity::P0 => crate::models::Severity::P0, // Already max
            }
        } else {
            match incident.severity {
                crate::models::Severity::P0 => crate::models::Severity::P1,
                crate::models::Severity::P1 => crate::models::Severity::P2,
                crate::models::Severity::P2 => crate::models::Severity::P3,
                crate::models::Severity::P3 => crate::models::Severity::P4,
                crate::models::Severity::P4 => crate::models::Severity::P4, // Already min
            }
        };

        if old_severity != new_severity {
            incident.severity = new_severity.clone();
            incident.add_timeline_event(crate::models::TimelineEvent {
                timestamp: chrono::Utc::now(),
                event_type: crate::models::EventType::SeverityChanged,
                actor: "playbook-engine".to_string(),
                description: format!("Severity changed from {:?} to {:?}", old_severity, new_severity),
                metadata: HashMap::new(),
            });

            self.store.update_incident(&incident).await?;
        }

        let mut output = HashMap::new();
        output.insert("old_severity".to_string(), JsonValue::String(format!("{:?}", old_severity)));
        output.insert("new_severity".to_string(), JsonValue::String(format!("{:?}", new_severity)));
        output.insert("changed".to_string(), JsonValue::Bool(old_severity != new_severity));
        Ok(ActionResult::success(output))
    }
}

// ==================== Generic HTTP Request Executor ====================

struct HttpRequestActionExecutor {
    client: Client,
}

impl HttpRequestActionExecutor {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl ActionExecutor for HttpRequestActionExecutor {
    async fn execute(&self, action: &Action, context: &mut ExecutionContext) -> Result<ActionResult> {
        let params = context.substitute_parameters(&action.parameters);

        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("'url' parameter required".to_string()))?;

        let method = params
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let mut request = match method.as_str() {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            "PATCH" => self.client.patch(url),
            _ => return Err(AppError::Validation(format!("Unsupported HTTP method: {}", method))),
        };

        // Add headers if provided
        if let Some(headers) = params.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in headers {
                if let Some(val_str) = value.as_str() {
                    request = request.header(key, val_str);
                }
            }
        }

        // Add body if provided
        if let Some(body) = params.get("body") {
            request = request.json(body);
        }

        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                let mut output = HashMap::new();
                output.insert("status_code".to_string(), JsonValue::Number(status.as_u16().into()));
                output.insert("response_body".to_string(), JsonValue::String(body.clone()));

                if status.is_success() {
                    Ok(ActionResult::success(output))
                } else {
                    Ok(ActionResult::failure(format!(
                        "HTTP request returned status {}: {}",
                        status, body
                    )))
                }
            }
            Err(e) => Ok(ActionResult::failure(format!("HTTP request failed: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Incident, IncidentType, Severity};
    use crate::state::InMemoryStore;

    fn create_test_incident() -> Incident {
        Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[tokio::test]
    async fn test_wait_action() {
        let incident = create_test_incident();
        let mut context = ExecutionContext::new(incident);

        let mut params = HashMap::new();
        params.insert("duration".to_string(), JsonValue::Number(1.into()));

        let action = Action {
            action_type: ActionType::Wait,
            parameters: params,
            on_success: None,
            on_failure: None,
        };

        let executor = WaitActionExecutor::new();
        let result = executor.execute(&action, &mut context).await.unwrap();

        assert!(result.success);
        assert_eq!(
            result.output.get("waited_seconds").unwrap().as_u64().unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_incident_resolve_action() {
        let incident = create_test_incident();
        let mut context = ExecutionContext::new(incident.clone());

        let store = Arc::new(InMemoryStore::new());
        store.save_incident(&incident).await.unwrap();

        let mut params = HashMap::new();
        params.insert("notes".to_string(), JsonValue::String("Test resolution".to_string()));

        let action = Action {
            action_type: ActionType::IncidentResolve,
            parameters: params,
            on_success: None,
            on_failure: None,
        };

        let executor = IncidentResolveActionExecutor::new(store.clone());
        let result = executor.execute(&action, &mut context).await.unwrap();

        assert!(result.success);
        assert_eq!(
            result.output.get("incident_resolved").unwrap().as_bool().unwrap(),
            true
        );

        // Verify incident was resolved
        let updated = store.get_incident(&incident.id).await.unwrap().unwrap();
        assert!(updated.resolution.is_some());
    }
}
