//! WebSocket message protocol
//!
//! This module defines the message types used for WebSocket communication between
//! clients and the server. Messages are JSON-encoded and support bidirectional
//! communication for real-time incident management updates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{
    alert::Alert,
    incident::{Incident, IncidentState, Severity},
};

/// Message sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to specific event types
    Subscribe {
        subscription_id: String,
        filters: SubscriptionFilters,
    },
    /// Unsubscribe from events
    Unsubscribe { subscription_id: String },
    /// Ping to keep connection alive
    Ping { timestamp: DateTime<Utc> },
    /// Acknowledge receipt of server message
    Ack { message_id: String },
}

/// Message sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Welcome message on connection
    Welcome {
        session_id: String,
        server_time: DateTime<Utc>,
    },
    /// Subscription confirmed
    Subscribed {
        subscription_id: String,
        filters: SubscriptionFilters,
    },
    /// Unsubscription confirmed
    Unsubscribed { subscription_id: String },
    /// Pong response to ping
    Pong { timestamp: DateTime<Utc> },
    /// Event notification
    Event {
        message_id: String,
        event: Event,
        timestamp: DateTime<Utc>,
    },
    /// Error message
    Error { code: String, message: String },
    /// Connection is being closed
    Closing { reason: String },
}

/// Subscription filters to control which events are delivered
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubscriptionFilters {
    /// Filter by event types (empty = all events)
    #[serde(default)]
    pub event_types: Vec<EventType>,
    /// Filter by incident severities (empty = all severities)
    #[serde(default)]
    pub severities: Vec<Severity>,
    /// Filter by incident states (empty = all states)
    #[serde(default)]
    pub states: Vec<IncidentState>,
    /// Filter by sources (empty = all sources)
    #[serde(default)]
    pub sources: Vec<String>,
    /// Filter by affected resources (empty = all resources)
    #[serde(default)]
    pub affected_resources: Vec<String>,
    /// Filter by labels (must match all specified labels)
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
    /// Filter by incident IDs (empty = all incidents)
    #[serde(default)]
    pub incident_ids: Vec<Uuid>,
}

impl SubscriptionFilters {
    /// Check if an incident matches these filters
    pub fn matches_incident(&self, incident: &Incident) -> bool {
        // Check severities
        if !self.severities.is_empty() && !self.severities.contains(&incident.severity) {
            return false;
        }

        // Check states
        if !self.states.is_empty() && !self.states.contains(&incident.state) {
            return false;
        }

        // Check sources
        if !self.sources.is_empty() && !self.sources.contains(&incident.source) {
            return false;
        }

        // Check affected resources
        if !self.affected_resources.is_empty() {
            let has_match = self
                .affected_resources
                .iter()
                .any(|r| incident.affected_resources.contains(r));
            if !has_match {
                return false;
            }
        }

        // Check labels (must match all specified labels)
        for (key, value) in &self.labels {
            if incident.labels.get(key) != Some(value) {
                return false;
            }
        }

        // Check incident IDs
        if !self.incident_ids.is_empty() && !self.incident_ids.contains(&incident.id) {
            return false;
        }

        true
    }

    /// Check if an alert matches these filters
    pub fn matches_alert(&self, alert: &Alert) -> bool {
        // Check severities
        if !self.severities.is_empty() && !self.severities.contains(&alert.severity) {
            return false;
        }

        // Check sources
        if !self.sources.is_empty() && !self.sources.contains(&alert.source) {
            return false;
        }

        // Check affected resources
        if !self.affected_resources.is_empty() {
            let has_match = self
                .affected_resources
                .iter()
                .any(|r| alert.affected_services.contains(r));
            if !has_match {
                return false;
            }
        }

        // Check labels
        for (key, value) in &self.labels {
            if alert.labels.get(key) != Some(value) {
                return false;
            }
        }

        true
    }

    /// Check if an event type matches these filters
    pub fn matches_event_type(&self, event_type: &EventType) -> bool {
        self.event_types.is_empty() || self.event_types.contains(event_type)
    }
}

/// Type of event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// New incident created
    IncidentCreated,
    /// Incident state changed
    IncidentUpdated,
    /// Incident resolved
    IncidentResolved,
    /// Incident closed
    IncidentClosed,
    /// New alert received
    AlertReceived,
    /// Alert converted to incident
    AlertConverted,
    /// Escalation triggered
    Escalated,
    /// Playbook started
    PlaybookStarted,
    /// Playbook action executed
    PlaybookActionExecuted,
    /// Playbook completed
    PlaybookCompleted,
    /// Notification sent
    NotificationSent,
    /// Assignment changed
    AssignmentChanged,
    /// Comment added
    CommentAdded,
    /// System event
    SystemEvent,
}

/// Event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum Event {
    IncidentCreated {
        incident: Incident,
    },
    IncidentUpdated {
        incident: Incident,
        previous_state: Option<IncidentState>,
    },
    IncidentResolved {
        incident: Incident,
    },
    IncidentClosed {
        incident: Incident,
    },
    AlertReceived {
        alert: Alert,
    },
    AlertConverted {
        alert: Alert,
        incident_id: Uuid,
    },
    Escalated {
        incident_id: Uuid,
        from_severity: Severity,
        to_severity: Severity,
        reason: String,
    },
    PlaybookStarted {
        incident_id: Uuid,
        playbook_id: Uuid,
        playbook_name: String,
    },
    PlaybookActionExecuted {
        incident_id: Uuid,
        playbook_id: Uuid,
        action_name: String,
        success: bool,
        message: String,
    },
    PlaybookCompleted {
        incident_id: Uuid,
        playbook_id: Uuid,
        success: bool,
        actions_executed: usize,
    },
    NotificationSent {
        incident_id: Uuid,
        channel: String,
        success: bool,
    },
    AssignmentChanged {
        incident_id: Uuid,
        assignees: Vec<String>,
    },
    CommentAdded {
        incident_id: Uuid,
        author: String,
        comment: String,
        timestamp: DateTime<Utc>,
    },
    SystemEvent {
        category: String,
        message: String,
        metadata: std::collections::HashMap<String, String>,
    },
}

impl Event {
    /// Get the event type
    pub fn event_type(&self) -> EventType {
        match self {
            Event::IncidentCreated { .. } => EventType::IncidentCreated,
            Event::IncidentUpdated { .. } => EventType::IncidentUpdated,
            Event::IncidentResolved { .. } => EventType::IncidentResolved,
            Event::IncidentClosed { .. } => EventType::IncidentClosed,
            Event::AlertReceived { .. } => EventType::AlertReceived,
            Event::AlertConverted { .. } => EventType::AlertConverted,
            Event::Escalated { .. } => EventType::Escalated,
            Event::PlaybookStarted { .. } => EventType::PlaybookStarted,
            Event::PlaybookActionExecuted { .. } => EventType::PlaybookActionExecuted,
            Event::PlaybookCompleted { .. } => EventType::PlaybookCompleted,
            Event::NotificationSent { .. } => EventType::NotificationSent,
            Event::AssignmentChanged { .. } => EventType::AssignmentChanged,
            Event::CommentAdded { .. } => EventType::CommentAdded,
            Event::SystemEvent { .. } => EventType::SystemEvent,
        }
    }

    /// Get the incident associated with this event (if any)
    pub fn incident(&self) -> Option<&Incident> {
        match self {
            Event::IncidentCreated { incident }
            | Event::IncidentUpdated { incident, .. }
            | Event::IncidentResolved { incident }
            | Event::IncidentClosed { incident } => Some(incident),
            _ => None,
        }
    }

    /// Get the alert associated with this event (if any)
    pub fn alert(&self) -> Option<&Alert> {
        match self {
            Event::AlertReceived { alert } | Event::AlertConverted { alert, .. } => Some(alert),
            _ => None,
        }
    }

    /// Get the incident ID associated with this event (if any)
    pub fn incident_id(&self) -> Option<Uuid> {
        match self {
            Event::IncidentCreated { incident }
            | Event::IncidentUpdated { incident, .. }
            | Event::IncidentResolved { incident }
            | Event::IncidentClosed { incident } => Some(incident.id),
            Event::AlertConverted { incident_id, .. }
            | Event::Escalated { incident_id, .. }
            | Event::PlaybookStarted { incident_id, .. }
            | Event::PlaybookActionExecuted { incident_id, .. }
            | Event::PlaybookCompleted { incident_id, .. }
            | Event::NotificationSent { incident_id, .. }
            | Event::AssignmentChanged { incident_id, .. }
            | Event::CommentAdded { incident_id, .. } => Some(*incident_id),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::incident::{Incident, IncidentType};

    #[test]
    fn test_subscription_filters_severity() {
        let incident = Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Application,
        );

        let mut filters = SubscriptionFilters::default();
        assert!(filters.matches_incident(&incident)); // Empty filters match all

        filters.severities = vec![Severity::P0, Severity::P1];
        assert!(filters.matches_incident(&incident));

        filters.severities = vec![Severity::P2, Severity::P3];
        assert!(!filters.matches_incident(&incident));
    }

    #[test]
    fn test_subscription_filters_labels() {
        let mut incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P2,
            IncidentType::Infrastructure,
        );
        incident
            .labels
            .insert("environment".to_string(), "production".to_string());
        incident
            .labels
            .insert("team".to_string(), "platform".to_string());

        let mut filters = SubscriptionFilters::default();
        filters
            .labels
            .insert("environment".to_string(), "production".to_string());
        assert!(filters.matches_incident(&incident));

        filters
            .labels
            .insert("team".to_string(), "platform".to_string());
        assert!(filters.matches_incident(&incident));

        filters
            .labels
            .insert("region".to_string(), "us-east-1".to_string());
        assert!(!filters.matches_incident(&incident)); // Missing label
    }

    #[test]
    fn test_event_type_extraction() {
        let incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Security,
        );

        let event = Event::IncidentCreated {
            incident: incident.clone(),
        };
        assert_eq!(event.event_type(), EventType::IncidentCreated);
        assert!(event.incident().is_some());
        assert_eq!(event.incident_id(), Some(incident.id));
    }

    #[test]
    fn test_message_serialization() {
        let msg = ServerMessage::Welcome {
            session_id: "test-session".to_string(),
            server_time: Utc::now(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("welcome"));
        assert!(json.contains("session_id"));
    }
}
