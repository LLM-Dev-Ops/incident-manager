//! WebSocket streaming module
//!
//! This module provides real-time event streaming via WebSockets for the
//! LLM Incident Manager. It supports:
//! - Real-time incident and alert notifications
//! - Filtered event subscriptions
//! - Session management and authentication
//! - Backpressure handling
//! - Metrics and observability
//!
//! # Architecture
//!
//! The WebSocket system is built on several key components:
//!
//! - **Messages**: Protocol definitions for client-server communication
//! - **Events**: Type-safe event definitions and envelopes
//! - **Session**: Session lifecycle and subscription tracking
//! - **Connection**: Connection registry and management
//! - **Broadcaster**: Event publishing and distribution
//! - **Server**: WebSocket endpoint and connection handling
//! - **Handlers**: Integration hooks for system events
//! - **Metrics**: Prometheus metrics for monitoring
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use llm_incident_manager::websocket::{WebSocketState, WebSocketConfig};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = WebSocketConfig::default();
//!     let state = Arc::new(WebSocketState::new(config));
//!
//!     // Use state.broadcaster to publish events
//!     // Use state.handlers for event integration
//!     // Use state.connections for connection management
//! }
//! ```

pub mod broadcaster;
pub mod connection;
pub mod events;
pub mod handlers;
pub mod messages;
pub mod metrics;
pub mod server;
pub mod session;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use broadcaster::EventBroadcaster;
pub use connection::ConnectionManager;
pub use handlers::EventHandlers;
pub use messages::{ClientMessage, Event, EventType, ServerMessage, SubscriptionFilters};
pub use server::{cleanup_task, websocket_handler};
pub use session::Session;

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Maximum number of pending messages per connection
    pub max_pending_messages: usize,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
    /// Cleanup interval in seconds
    pub cleanup_interval_secs: u64,
    /// Broadcast channel capacity
    pub broadcast_capacity: usize,
    /// Enable message compression
    pub enable_compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_pending_messages: 1000,
            heartbeat_interval_secs: 30,
            session_timeout_secs: 300, // 5 minutes
            cleanup_interval_secs: 60,
            broadcast_capacity: 10000,
            enable_compression: true,
        }
    }
}

/// WebSocket state shared across connections
pub struct WebSocketState {
    /// Configuration
    pub config: WebSocketConfig,
    /// Connection manager
    pub connections: Arc<ConnectionManager>,
    /// Event broadcaster
    pub broadcaster: Arc<EventBroadcaster>,
    /// Event handlers
    pub handlers: EventHandlers,
}

impl WebSocketState {
    /// Create a new WebSocket state
    pub fn new(config: WebSocketConfig) -> Self {
        let connections = Arc::new(ConnectionManager::new());
        let broadcaster = Arc::new(EventBroadcaster::new(
            connections.clone(),
            config.broadcast_capacity,
        ));
        let handlers = EventHandlers::new(broadcaster.clone());

        Self {
            config,
            connections,
            broadcaster,
            handlers,
        }
    }

    /// Get connection statistics
    pub fn connection_stats(&self) -> connection::ConnectionStats {
        self.connections.stats()
    }

    /// Get event statistics
    pub fn event_stats(&self) -> events::EventStats {
        self.broadcaster.stats()
    }

    /// Get active connection count
    pub fn active_connections(&self) -> usize {
        self.connections.connection_count()
    }

    /// Get active subscriber count
    pub fn active_subscribers(&self) -> usize {
        self.broadcaster.subscriber_count()
    }
}

/// Builder for WebSocket state
pub struct WebSocketStateBuilder {
    config: WebSocketConfig,
}

impl WebSocketStateBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: WebSocketConfig::default(),
        }
    }

    /// Set maximum pending messages
    pub fn max_pending_messages(mut self, max: usize) -> Self {
        self.config.max_pending_messages = max;
        self
    }

    /// Set heartbeat interval
    pub fn heartbeat_interval_secs(mut self, secs: u64) -> Self {
        self.config.heartbeat_interval_secs = secs;
        self
    }

    /// Set session timeout
    pub fn session_timeout_secs(mut self, secs: u64) -> Self {
        self.config.session_timeout_secs = secs;
        self
    }

    /// Set cleanup interval
    pub fn cleanup_interval_secs(mut self, secs: u64) -> Self {
        self.config.cleanup_interval_secs = secs;
        self
    }

    /// Set broadcast capacity
    pub fn broadcast_capacity(mut self, capacity: usize) -> Self {
        self.config.broadcast_capacity = capacity;
        self
    }

    /// Enable or disable compression
    pub fn enable_compression(mut self, enable: bool) -> Self {
        self.config.enable_compression = enable;
        self
    }

    /// Build the WebSocket state
    pub fn build(self) -> WebSocketState {
        WebSocketState::new(self.config)
    }
}

impl Default for WebSocketStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.max_pending_messages, 1000);
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert_eq!(config.session_timeout_secs, 300);
        assert_eq!(config.cleanup_interval_secs, 60);
        assert_eq!(config.broadcast_capacity, 10000);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_state_creation() {
        let config = WebSocketConfig::default();
        let state = WebSocketState::new(config);

        assert_eq!(state.active_connections(), 0);
        assert_eq!(state.active_subscribers(), 0);
    }

    #[test]
    fn test_builder() {
        let state = WebSocketStateBuilder::new()
            .max_pending_messages(500)
            .heartbeat_interval_secs(60)
            .session_timeout_secs(600)
            .cleanup_interval_secs(120)
            .broadcast_capacity(5000)
            .enable_compression(false)
            .build();

        assert_eq!(state.config.max_pending_messages, 500);
        assert_eq!(state.config.heartbeat_interval_secs, 60);
        assert_eq!(state.config.session_timeout_secs, 600);
        assert_eq!(state.config.cleanup_interval_secs, 120);
        assert_eq!(state.config.broadcast_capacity, 5000);
        assert!(!state.config.enable_compression);
    }

    #[test]
    fn test_state_stats() {
        let state = WebSocketState::new(WebSocketConfig::default());

        let conn_stats = state.connection_stats();
        assert_eq!(conn_stats.active_connections, 0);

        let event_stats = state.event_stats();
        assert_eq!(event_stats.total_events, 0);
    }
}
