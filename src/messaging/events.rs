//! Event types for message queue

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Incident event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IncidentEvent {
    /// Incident created
    Created {
        incident_id: String,
        severity: String,
        incident_type: String,
        title: String,
    },

    /// Incident state changed
    StateChanged {
        incident_id: String,
        old_state: String,
        new_state: String,
    },

    /// Incident assigned
    Assigned {
        incident_id: String,
        assignee: String,
    },

    /// Incident resolved
    Resolved {
        incident_id: String,
        resolution_time_secs: u64,
    },

    /// Incident escalated
    Escalated {
        incident_id: String,
        escalation_level: u32,
    },

    /// Incident comment added
    CommentAdded {
        incident_id: String,
        comment_id: String,
        author: String,
    },

    /// Playbook started
    PlaybookStarted {
        incident_id: String,
        playbook_id: String,
    },

    /// Playbook completed
    PlaybookCompleted {
        incident_id: String,
        playbook_id: String,
        success: bool,
    },

    /// Alert correlated
    AlertCorrelated {
        incident_id: String,
        alert_id: String,
        correlation_score: f64,
    },
}

impl IncidentEvent {
    /// Get the incident ID from any event
    pub fn incident_id(&self) -> &str {
        match self {
            IncidentEvent::Created { incident_id, .. }
            | IncidentEvent::StateChanged { incident_id, .. }
            | IncidentEvent::Assigned { incident_id, .. }
            | IncidentEvent::Resolved { incident_id, .. }
            | IncidentEvent::Escalated { incident_id, .. }
            | IncidentEvent::CommentAdded { incident_id, .. }
            | IncidentEvent::PlaybookStarted { incident_id, .. }
            | IncidentEvent::PlaybookCompleted { incident_id, .. }
            | IncidentEvent::AlertCorrelated { incident_id, .. } => incident_id,
        }
    }

    /// Get the event type as a string
    pub fn event_type(&self) -> &'static str {
        match self {
            IncidentEvent::Created { .. } => "Created",
            IncidentEvent::StateChanged { .. } => "StateChanged",
            IncidentEvent::Assigned { .. } => "Assigned",
            IncidentEvent::Resolved { .. } => "Resolved",
            IncidentEvent::Escalated { .. } => "Escalated",
            IncidentEvent::CommentAdded { .. } => "CommentAdded",
            IncidentEvent::PlaybookStarted { .. } => "PlaybookStarted",
            IncidentEvent::PlaybookCompleted { .. } => "PlaybookCompleted",
            IncidentEvent::AlertCorrelated { .. } => "AlertCorrelated",
        }
    }
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Message ID
    pub message_id: String,

    /// Correlation ID
    pub correlation_id: Option<String>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Source service
    pub source: String,

    /// Custom headers
    pub headers: HashMap<String, String>,
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: None,
            timestamp: Utc::now(),
            source: "llm-incident-manager".to_string(),
            headers: HashMap::new(),
        }
    }
}

/// Message envelope wrapping the event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    /// Message metadata
    pub metadata: MessageMetadata,

    /// Message payload
    pub payload: T,
}

impl<T> MessageEnvelope<T> {
    /// Create a new message envelope
    pub fn new(payload: T) -> Self {
        Self {
            metadata: MessageMetadata::default(),
            payload,
        }
    }

    /// Create with correlation ID
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.metadata.correlation_id = Some(correlation_id);
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.metadata.headers.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_event_id() {
        let event = IncidentEvent::Created {
            incident_id: "inc-001".to_string(),
            severity: "P0".to_string(),
            incident_type: "Infrastructure".to_string(),
            title: "Test".to_string(),
        };

        assert_eq!(event.incident_id(), "inc-001");
        assert_eq!(event.event_type(), "Created");
    }

    #[test]
    fn test_message_envelope() {
        let event = IncidentEvent::Resolved {
            incident_id: "inc-002".to_string(),
            resolution_time_secs: 3600,
        };

        let envelope = MessageEnvelope::new(event)
            .with_correlation_id("corr-123".to_string())
            .with_header("priority".to_string(), "high".to_string());

        assert_eq!(envelope.metadata.correlation_id.as_ref().unwrap(), "corr-123");
        assert_eq!(envelope.metadata.headers.get("priority").unwrap(), "high");
    }
}
