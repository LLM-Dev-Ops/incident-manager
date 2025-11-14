//! Event broadcasting system
//!
//! This module provides a pub-sub event broadcasting system for distributing
//! events to WebSocket connections.

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info};

use super::{
    connection::ConnectionManager,
    events::{EventEnvelope, EventStats},
    messages::Event,
};

/// Event broadcaster
pub struct EventBroadcaster {
    /// Broadcast channel for events
    tx: broadcast::Sender<EventEnvelope>,
    /// Connection manager
    connections: Arc<ConnectionManager>,
    /// Event statistics
    stats: Arc<RwLock<EventStats>>,
    /// Channel capacity
    capacity: usize,
}

impl EventBroadcaster {
    /// Create a new event broadcaster
    pub fn new(connections: Arc<ConnectionManager>, capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            connections,
            stats: Arc::new(RwLock::new(EventStats::default())),
            capacity,
        }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: Event) {
        let event_type = event.event_type();
        let envelope = EventEnvelope::new(event);

        debug!(
            event_type = ?event_type,
            priority = ?envelope.priority,
            "Publishing event"
        );

        // Update statistics
        self.stats.write().record_event(event_type);

        // Broadcast via channel (for potential multiple processors)
        if let Err(e) = self.tx.send(envelope.clone()) {
            error!(error = ?e, "Failed to send event to broadcast channel");
        }

        // Broadcast to WebSocket connections
        self.connections.broadcast_event(envelope).await;
    }

    /// Publish a high-priority event
    pub async fn publish_high_priority(&self, event: Event) {
        let event_type = event.event_type();
        let envelope = EventEnvelope::high_priority(event);

        info!(
            event_type = ?event_type,
            priority = ?envelope.priority,
            "Publishing high-priority event"
        );

        self.stats.write().record_event(event_type);

        if let Err(e) = self.tx.send(envelope.clone()) {
            error!(error = ?e, "Failed to send high-priority event to broadcast channel");
        }

        self.connections.broadcast_event(envelope).await;
    }

    /// Subscribe to events (for internal consumers)
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.tx.subscribe()
    }

    /// Get event statistics
    pub fn stats(&self) -> EventStats {
        self.stats.read().clone()
    }

    /// Get channel capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

/// Event publisher trait for services to implement
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish an event
    async fn publish_event(&self, event: Event);

    /// Publish a high-priority event
    async fn publish_high_priority_event(&self, event: Event);
}

#[async_trait::async_trait]
impl EventPublisher for EventBroadcaster {
    async fn publish_event(&self, event: Event) {
        self.publish(event).await;
    }

    async fn publish_high_priority_event(&self, event: Event) {
        self.publish_high_priority(event).await;
    }
}

/// Convenience functions for common event publications
impl EventBroadcaster {
    /// Publish incident created event
    pub async fn incident_created(&self, incident: crate::models::incident::Incident) {
        let is_critical = incident.is_critical();
        let event = Event::IncidentCreated { incident };

        if is_critical {
            self.publish_high_priority(event).await;
        } else {
            self.publish(event).await;
        }
    }

    /// Publish incident updated event
    pub async fn incident_updated(
        &self,
        incident: crate::models::incident::Incident,
        previous_state: Option<crate::models::incident::IncidentState>,
    ) {
        let event = Event::IncidentUpdated {
            incident,
            previous_state,
        };
        self.publish(event).await;
    }

    /// Publish incident resolved event
    pub async fn incident_resolved(&self, incident: crate::models::incident::Incident) {
        let event = Event::IncidentResolved { incident };
        self.publish(event).await;
    }

    /// Publish alert received event
    pub async fn alert_received(&self, alert: crate::models::alert::Alert) {
        let is_urgent = alert.is_urgent();
        let event = Event::AlertReceived { alert };

        if is_urgent {
            self.publish_high_priority(event).await;
        } else {
            self.publish(event).await;
        }
    }

    /// Publish escalation event
    pub async fn escalated(
        &self,
        incident_id: uuid::Uuid,
        from_severity: crate::models::incident::Severity,
        to_severity: crate::models::incident::Severity,
        reason: String,
    ) {
        let event = Event::Escalated {
            incident_id,
            from_severity,
            to_severity,
            reason,
        };
        self.publish_high_priority(event).await;
    }

    /// Publish playbook started event
    pub async fn playbook_started(
        &self,
        incident_id: uuid::Uuid,
        playbook_id: uuid::Uuid,
        playbook_name: String,
    ) {
        let event = Event::PlaybookStarted {
            incident_id,
            playbook_id,
            playbook_name,
        };
        self.publish(event).await;
    }

    /// Publish playbook completed event
    pub async fn playbook_completed(
        &self,
        incident_id: uuid::Uuid,
        playbook_id: uuid::Uuid,
        success: bool,
        actions_executed: usize,
    ) {
        let event = Event::PlaybookCompleted {
            incident_id,
            playbook_id,
            success,
            actions_executed,
        };
        self.publish(event).await;
    }

    /// Publish notification sent event
    pub async fn notification_sent(
        &self,
        incident_id: uuid::Uuid,
        channel: String,
        success: bool,
    ) {
        let event = Event::NotificationSent {
            incident_id,
            channel,
            success,
        };
        self.publish(event).await;
    }

    /// Publish system event
    pub async fn system_event(
        &self,
        category: String,
        message: String,
        metadata: std::collections::HashMap<String, String>,
    ) {
        let event = Event::SystemEvent {
            category,
            message,
            metadata,
        };
        self.publish(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::incident::{Incident, IncidentType, Severity};

    #[tokio::test]
    async fn test_broadcaster_creation() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = EventBroadcaster::new(manager, 100);

        assert_eq!(broadcaster.capacity(), 100);
        assert_eq!(broadcaster.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_event_publishing() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = EventBroadcaster::new(manager, 100);

        let incident = Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        broadcaster.incident_created(incident).await;

        let stats = broadcaster.stats();
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_high_priority_publishing() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = EventBroadcaster::new(manager, 100);

        let incident = Incident::new(
            "test".to_string(),
            "Critical Issue".to_string(),
            "Description".to_string(),
            Severity::P0,
            IncidentType::Security,
        );

        // P0 incidents should be published as high priority
        broadcaster.incident_created(incident).await;

        let stats = broadcaster.stats();
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_subscription() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = EventBroadcaster::new(manager, 100);

        let mut rx = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 1);

        // Publish event
        let event = Event::SystemEvent {
            category: "test".to_string(),
            message: "test message".to_string(),
            metadata: Default::default(),
        };

        broadcaster.publish(event).await;

        // Receive event
        let envelope = rx.recv().await.unwrap();
        match envelope.event {
            Event::SystemEvent { message, .. } => {
                assert_eq!(message, "test message");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let manager = Arc::new(ConnectionManager::new());
        let broadcaster = EventBroadcaster::new(manager, 100);

        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 2);

        let event = Event::SystemEvent {
            category: "test".to_string(),
            message: "broadcast test".to_string(),
            metadata: Default::default(),
        };

        broadcaster.publish(event).await;

        // Both receivers should get the event
        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();

        assert_eq!(e1.id, e2.id);
    }
}
