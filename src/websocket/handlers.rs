//! Event handlers for WebSocket system
//!
//! This module provides handlers that hook into various system events
//! and publish them to the WebSocket broadcaster.

use std::sync::Arc;
use uuid::Uuid;

use crate::models::{
    alert::Alert,
    incident::{Incident, IncidentState, Severity},
};

use super::{broadcaster::EventBroadcaster, messages::Event};

/// Event handler for incident lifecycle events
#[derive(Clone)]
pub struct IncidentEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl IncidentEventHandler {
    /// Create a new incident event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle incident created event
    pub async fn on_incident_created(&self, incident: Incident) {
        self.broadcaster.incident_created(incident).await;
    }

    /// Handle incident updated event
    pub async fn on_incident_updated(&self, incident: Incident, previous_state: Option<IncidentState>) {
        self.broadcaster.incident_updated(incident, previous_state).await;
    }

    /// Handle incident resolved event
    pub async fn on_incident_resolved(&self, incident: Incident) {
        self.broadcaster.incident_resolved(incident).await;
    }

    /// Handle incident closed event
    pub async fn on_incident_closed(&self, incident: Incident) {
        let event = Event::IncidentClosed { incident };
        self.broadcaster.publish(event).await;
    }
}

/// Event handler for alert events
#[derive(Clone)]
pub struct AlertEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl AlertEventHandler {
    /// Create a new alert event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle alert received event
    pub async fn on_alert_received(&self, alert: Alert) {
        self.broadcaster.alert_received(alert).await;
    }

    /// Handle alert converted to incident
    pub async fn on_alert_converted(&self, alert: Alert, incident_id: Uuid) {
        let event = Event::AlertConverted { alert, incident_id };
        self.broadcaster.publish(event).await;
    }
}

/// Event handler for escalation events
#[derive(Clone)]
pub struct EscalationEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl EscalationEventHandler {
    /// Create a new escalation event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle escalation event
    pub async fn on_escalated(
        &self,
        incident_id: Uuid,
        from_severity: Severity,
        to_severity: Severity,
        reason: String,
    ) {
        self.broadcaster
            .escalated(incident_id, from_severity, to_severity, reason)
            .await;
    }
}

/// Event handler for playbook events
#[derive(Clone)]
pub struct PlaybookEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl PlaybookEventHandler {
    /// Create a new playbook event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle playbook started event
    pub async fn on_playbook_started(
        &self,
        incident_id: Uuid,
        playbook_id: Uuid,
        playbook_name: String,
    ) {
        self.broadcaster
            .playbook_started(incident_id, playbook_id, playbook_name)
            .await;
    }

    /// Handle playbook action executed event
    pub async fn on_playbook_action_executed(
        &self,
        incident_id: Uuid,
        playbook_id: Uuid,
        action_name: String,
        success: bool,
        message: String,
    ) {
        let event = Event::PlaybookActionExecuted {
            incident_id,
            playbook_id,
            action_name,
            success,
            message,
        };
        self.broadcaster.publish(event).await;
    }

    /// Handle playbook completed event
    pub async fn on_playbook_completed(
        &self,
        incident_id: Uuid,
        playbook_id: Uuid,
        success: bool,
        actions_executed: usize,
    ) {
        self.broadcaster
            .playbook_completed(incident_id, playbook_id, success, actions_executed)
            .await;
    }
}

/// Event handler for notification events
#[derive(Clone)]
pub struct NotificationEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl NotificationEventHandler {
    /// Create a new notification event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle notification sent event
    pub async fn on_notification_sent(&self, incident_id: Uuid, channel: String, success: bool) {
        self.broadcaster
            .notification_sent(incident_id, channel, success)
            .await;
    }
}

/// Event handler for assignment changes
#[derive(Clone)]
pub struct AssignmentEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl AssignmentEventHandler {
    /// Create a new assignment event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle assignment changed event
    pub async fn on_assignment_changed(&self, incident_id: Uuid, assignees: Vec<String>) {
        let event = Event::AssignmentChanged {
            incident_id,
            assignees,
        };
        self.broadcaster.publish(event).await;
    }
}

/// Event handler for comments
#[derive(Clone)]
pub struct CommentEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl CommentEventHandler {
    /// Create a new comment event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle comment added event
    pub async fn on_comment_added(
        &self,
        incident_id: Uuid,
        author: String,
        comment: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) {
        let event = Event::CommentAdded {
            incident_id,
            author,
            comment,
            timestamp,
        };
        self.broadcaster.publish(event).await;
    }
}

/// Event handler for system events
#[derive(Clone)]
pub struct SystemEventHandler {
    broadcaster: Arc<EventBroadcaster>,
}

impl SystemEventHandler {
    /// Create a new system event handler
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }

    /// Handle system event
    pub async fn on_system_event(
        &self,
        category: String,
        message: String,
        metadata: std::collections::HashMap<String, String>,
    ) {
        self.broadcaster
            .system_event(category, message, metadata)
            .await;
    }
}

/// Aggregate event handler that includes all event types
#[derive(Clone)]
pub struct EventHandlers {
    pub incidents: IncidentEventHandler,
    pub alerts: AlertEventHandler,
    pub escalations: EscalationEventHandler,
    pub playbooks: PlaybookEventHandler,
    pub notifications: NotificationEventHandler,
    pub assignments: AssignmentEventHandler,
    pub comments: CommentEventHandler,
    pub system: SystemEventHandler,
}

impl EventHandlers {
    /// Create a new event handlers collection
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self {
            incidents: IncidentEventHandler::new(broadcaster.clone()),
            alerts: AlertEventHandler::new(broadcaster.clone()),
            escalations: EscalationEventHandler::new(broadcaster.clone()),
            playbooks: PlaybookEventHandler::new(broadcaster.clone()),
            notifications: NotificationEventHandler::new(broadcaster.clone()),
            assignments: AssignmentEventHandler::new(broadcaster.clone()),
            comments: CommentEventHandler::new(broadcaster.clone()),
            system: SystemEventHandler::new(broadcaster),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::incident::{IncidentType, Severity},
        websocket::connection::ConnectionManager,
    };

    #[tokio::test]
    async fn test_incident_event_handler() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = Arc::new(EventBroadcaster::new(manager, 100));
        let handler = IncidentEventHandler::new(broadcaster.clone());

        let incident = Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        handler.on_incident_created(incident).await;

        let stats = broadcaster.stats();
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_alert_event_handler() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = Arc::new(EventBroadcaster::new(manager, 100));
        let handler = AlertEventHandler::new(broadcaster.clone());

        let alert = Alert::new(
            "ext-123".to_string(),
            "test-source".to_string(),
            "Test Alert".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Security,
        );

        handler.on_alert_received(alert).await;

        let stats = broadcaster.stats();
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_event_handlers_creation() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = Arc::new(EventBroadcaster::new(manager, 100));
        let handlers = EventHandlers::new(broadcaster);

        // Test that all handlers are created
        let incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P3,
            IncidentType::Infrastructure,
        );

        handlers.incidents.on_incident_created(incident).await;
    }
}
