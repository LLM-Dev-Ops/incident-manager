//! WebSocket connection management
//!
//! This module manages active WebSocket connections and provides a registry
//! for tracking and broadcasting to connected clients.

use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::{stream::SplitSink, SinkExt};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::{
    events::EventEnvelope,
    messages::{Event, ServerMessage, SubscriptionFilters},
    session::Session,
};

/// Maximum pending messages per connection before dropping
#[allow(dead_code)]
const MAX_PENDING_MESSAGES: usize = 1000;

/// Connection handle for sending messages
pub struct Connection {
    /// Session information
    pub session: Arc<RwLock<Session>>,
    /// Channel for sending messages to this connection
    tx: mpsc::UnboundedSender<ServerMessage>,
    /// Connection metadata
    remote_addr: Option<String>,
}

impl Connection {
    /// Create a new connection
    pub fn new(session: Session, remote_addr: Option<String>) -> (Self, mpsc::UnboundedReceiver<ServerMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                session: Arc::new(RwLock::new(session)),
                tx,
                remote_addr,
            },
            rx,
        )
    }

    /// Send a message to this connection
    pub fn send(&self, message: ServerMessage) -> Result<(), ConnectionError> {
        self.tx
            .send(message)
            .map_err(|_| ConnectionError::SendFailed)
    }

    /// Get session ID
    pub fn session_id(&self) -> String {
        self.session.read().id.clone()
    }

    /// Check if this connection should receive an event
    pub fn should_receive_event(&self, event: &Event, filters: &SubscriptionFilters) -> bool {
        // Check event type filter
        if !filters.matches_event_type(&event.event_type()) {
            return false;
        }

        // Check incident-based filters
        if let Some(incident) = event.incident() {
            return filters.matches_incident(incident);
        }

        // Check alert-based filters
        if let Some(alert) = event.alert() {
            return filters.matches_alert(alert);
        }

        // For non-incident/alert events, just check event type
        true
    }

    /// Get remote address
    pub fn remote_addr(&self) -> Option<&str> {
        self.remote_addr.as_deref()
    }
}

/// Connection manager
pub struct ConnectionManager {
    /// Active connections indexed by session ID
    connections: Arc<DashMap<String, Arc<Connection>>>,
    /// Connection statistics
    stats: Arc<RwLock<ConnectionStats>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
        }
    }

    /// Register a new connection
    pub fn register(
        &self,
        session: Session,
        remote_addr: Option<String>,
    ) -> (Arc<Connection>, mpsc::UnboundedReceiver<ServerMessage>) {
        let session_id = session.id.clone();
        let (connection, rx) = Connection::new(session, remote_addr);
        let connection = Arc::new(connection);

        self.connections.insert(session_id.clone(), connection.clone());
        self.stats.write().total_connections += 1;
        self.stats.write().active_connections = self.connections.len() as u64;

        info!(session_id = %session_id, "WebSocket connection registered");
        (connection, rx)
    }

    /// Unregister a connection
    pub fn unregister(&self, session_id: &str) {
        if self.connections.remove(session_id).is_some() {
            self.stats.write().active_connections = self.connections.len() as u64;
            info!(session_id = %session_id, "WebSocket connection unregistered");
        }
    }

    /// Get connection by session ID
    pub fn get(&self, session_id: &str) -> Option<Arc<Connection>> {
        self.connections.get(session_id).map(|e| e.value().clone())
    }

    /// Get number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get all connection session IDs
    pub fn session_ids(&self) -> Vec<String> {
        self.connections.iter().map(|e| e.key().clone()).collect()
    }

    /// Broadcast event to all matching connections
    pub async fn broadcast_event(&self, envelope: EventEnvelope) {
        let event_type = envelope.event.event_type();
        let mut delivered = 0;
        let mut filtered = 0;

        for entry in self.connections.iter() {
            let connection = entry.value();
            let session = connection.session.read();

            // Check if session has any subscriptions interested in this event type
            if !session.interested_event_types().contains(&event_type) {
                filtered += 1;
                continue;
            }

            // Check filters for each subscription
            let mut should_send = false;
            for subscription in session.subscriptions.values() {
                if connection.should_receive_event(&envelope.event, &subscription.filters) {
                    should_send = true;
                    break;
                }
            }

            if !should_send {
                filtered += 1;
                continue;
            }

            // Send event to connection
            drop(session); // Release lock before sending
            let message = ServerMessage::Event {
                message_id: envelope.id.clone(),
                event: envelope.event.clone(),
                timestamp: envelope.timestamp,
            };

            if let Err(e) = connection.send(message) {
                warn!(
                    session_id = %connection.session_id(),
                    error = ?e,
                    "Failed to send event to connection"
                );
            } else {
                delivered += 1;
            }
        }

        debug!(
            event_type = ?event_type,
            delivered = delivered,
            filtered = filtered,
            "Event broadcast completed"
        );

        self.stats.write().total_events_broadcast += 1;
        self.stats.write().total_events_delivered += delivered;
    }

    /// Send event to specific session
    pub async fn send_to_session(&self, session_id: &str, event: Event) -> Result<(), ConnectionError> {
        let connection = self
            .get(session_id)
            .ok_or(ConnectionError::NotFound)?;

        let envelope = EventEnvelope::new(event);
        let message = ServerMessage::Event {
            message_id: envelope.id,
            event: envelope.event,
            timestamp: envelope.timestamp,
        };

        connection.send(message)
    }

    /// Get statistics
    pub fn stats(&self) -> ConnectionStats {
        self.stats.read().clone()
    }

    /// Cleanup expired sessions
    pub async fn cleanup_expired(&self, timeout_secs: i64) {
        let mut expired = Vec::new();

        for entry in self.connections.iter() {
            let session = entry.value().session.read();
            if session.is_expired(timeout_secs) {
                expired.push(session.id.clone());
            }
        }

        for session_id in expired {
            info!(session_id = %session_id, "Cleaning up expired session");
            self.unregister(&session_id);
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_events_broadcast: u64,
    pub total_events_delivered: u64,
}

/// Connection errors
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Failed to send message to connection")]
    SendFailed,
    #[error("Connection not found")]
    NotFound,
}

/// WebSocket message writer
pub struct MessageWriter {
    sink: SplitSink<WebSocket, Message>,
}

impl MessageWriter {
    /// Create a new message writer
    pub fn new(sink: SplitSink<WebSocket, Message>) -> Self {
        Self { sink }
    }

    /// Send a server message
    pub async fn send(&mut self, message: ServerMessage) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(&message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        self.sink
            .send(Message::Text(json))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
    }

    /// Send a ping
    pub async fn send_ping(&mut self) -> Result<(), std::io::Error> {
        self.sink
            .send(Message::Ping(vec![]))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<(), std::io::Error> {
        self.sink
            .send(Message::Close(None))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::incident::{Incident, IncidentType, Severity};

    #[test]
    fn test_connection_manager_creation() {
        let manager = ConnectionManager::new();
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn test_connection_registration() {
        let manager = ConnectionManager::new();
        let session = Session::new();
        let session_id = session.id.clone();

        let (_conn, _rx) = manager.register(session, Some("127.0.0.1:8080".to_string()));
        assert_eq!(manager.connection_count(), 1);

        assert!(manager.get(&session_id).is_some());

        manager.unregister(&session_id);
        assert_eq!(manager.connection_count(), 0);
        assert!(manager.get(&session_id).is_none());
    }

    #[test]
    fn test_connection_should_receive_event() {
        let session = Session::new();
        let (conn, _rx) = Connection::new(session, None);

        let incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Application,
        );

        let event = Event::IncidentCreated {
            incident: incident.clone(),
        };

        // Empty filters match all
        let filters = SubscriptionFilters::default();
        assert!(conn.should_receive_event(&event, &filters));

        // Severity filter
        let mut filters = SubscriptionFilters::default();
        filters.severities = vec![Severity::P0];
        assert!(!conn.should_receive_event(&event, &filters));

        filters.severities = vec![Severity::P1, Severity::P2];
        assert!(conn.should_receive_event(&event, &filters));
    }

    #[tokio::test]
    async fn test_connection_send() {
        let session = Session::new();
        let (conn, mut rx) = Connection::new(session, None);

        let message = ServerMessage::Welcome {
            session_id: "test".to_string(),
            server_time: chrono::Utc::now(),
        };

        conn.send(message.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            ServerMessage::Welcome { session_id, .. } => {
                assert_eq!(session_id, "test");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_stats_tracking() {
        let manager = ConnectionManager::new();
        let stats = manager.stats();
        assert_eq!(stats.active_connections, 0);

        let session = Session::new();
        let session_id = session.id.clone();
        let (_conn, _rx) = manager.register(session, None);

        let stats = manager.stats();
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.total_connections, 1);

        manager.unregister(&session_id);
        let stats = manager.stats();
        assert_eq!(stats.active_connections, 0);
    }
}
