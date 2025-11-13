//! WebSocket event types and schemas
//!
//! This module provides type-safe event definitions for the WebSocket system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use super::messages::{Event, EventType};

/// Internal event envelope for broadcasting
#[derive(Debug, Clone)]
pub struct EventEnvelope {
    /// Unique event ID
    pub id: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event payload
    pub event: Event,
    /// Event priority (higher = more important)
    pub priority: EventPriority,
}

impl EventEnvelope {
    /// Create a new event envelope
    pub fn new(event: Event) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            priority: EventPriority::from_event(&event),
            event,
        }
    }

    /// Create a high-priority event envelope
    pub fn high_priority(event: Event) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            priority: EventPriority::High,
            event,
        }
    }
}

/// Event priority for delivery ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl EventPriority {
    /// Determine priority from event content
    pub fn from_event(event: &Event) -> Self {
        use crate::models::incident::Severity;

        match event {
            // Critical events
            Event::IncidentCreated { incident } if incident.severity == Severity::P0 => {
                EventPriority::Critical
            }
            Event::Escalated {
                to_severity: Severity::P0,
                ..
            } => EventPriority::Critical,

            // High priority events
            Event::IncidentCreated { incident } if incident.severity == Severity::P1 => {
                EventPriority::High
            }
            Event::Escalated { .. } => EventPriority::High,
            Event::AlertReceived { alert } if alert.severity == Severity::P0 => EventPriority::High,

            // Normal priority
            Event::IncidentCreated { .. }
            | Event::IncidentUpdated { .. }
            | Event::AlertReceived { .. }
            | Event::PlaybookStarted { .. }
            | Event::NotificationSent { .. } => EventPriority::Normal,

            // Low priority
            Event::CommentAdded { .. }
            | Event::PlaybookActionExecuted { .. }
            | Event::SystemEvent { .. } => EventPriority::Low,

            // Default to normal
            _ => EventPriority::Normal,
        }
    }
}

/// Event statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventStats {
    pub total_events: u64,
    pub events_by_type: std::collections::HashMap<String, u64>,
    pub last_event_time: Option<DateTime<Utc>>,
}

impl EventStats {
    /// Record a new event
    pub fn record_event(&mut self, event_type: EventType) {
        self.total_events += 1;
        *self
            .events_by_type
            .entry(format!("{:?}", event_type))
            .or_insert(0) += 1;
        self.last_event_time = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::incident::{Incident, IncidentType, Severity};

    #[test]
    fn test_event_priority() {
        let p0_incident = Incident::new(
            "test".to_string(),
            "Critical Issue".to_string(),
            "Description".to_string(),
            Severity::P0,
            IncidentType::Security,
        );

        let event = Event::IncidentCreated {
            incident: p0_incident,
        };
        assert_eq!(EventPriority::from_event(&event), EventPriority::Critical);

        let comment_event = Event::CommentAdded {
            incident_id: Uuid::new_v4(),
            author: "user".to_string(),
            comment: "test".to_string(),
            timestamp: Utc::now(),
        };
        assert_eq!(
            EventPriority::from_event(&comment_event),
            EventPriority::Low
        );
    }

    #[test]
    fn test_event_envelope_creation() {
        let event = Event::SystemEvent {
            category: "test".to_string(),
            message: "test message".to_string(),
            metadata: Default::default(),
        };

        let envelope = EventEnvelope::new(event);
        assert!(!envelope.id.is_empty());
        assert_eq!(envelope.priority, EventPriority::Low);
    }

    #[test]
    fn test_event_stats() {
        let mut stats = EventStats::default();
        assert_eq!(stats.total_events, 0);

        stats.record_event(EventType::IncidentCreated);
        stats.record_event(EventType::IncidentCreated);
        stats.record_event(EventType::AlertReceived);

        assert_eq!(stats.total_events, 3);
        assert_eq!(
            *stats.events_by_type.get("IncidentCreated").unwrap(),
            2
        );
        assert!(stats.last_event_time.is_some());
    }
}
