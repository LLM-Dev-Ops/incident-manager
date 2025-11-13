//! WebSocket metrics
//!
//! This module provides Prometheus metrics for WebSocket monitoring.

use lazy_static::lazy_static;
use prometheus::{
    register_gauge, register_histogram, register_int_counter, register_int_gauge, Gauge,
    Histogram, IntCounter, IntGauge,
};

lazy_static! {
    /// Active WebSocket connections
    pub static ref WS_ACTIVE_CONNECTIONS: IntGauge = register_int_gauge!(
        "websocket_active_connections",
        "Number of active WebSocket connections"
    )
    .unwrap();

    /// Total WebSocket connections
    pub static ref WS_TOTAL_CONNECTIONS: IntCounter = register_int_counter!(
        "websocket_total_connections",
        "Total number of WebSocket connections established"
    )
    .unwrap();

    /// Active subscriptions
    pub static ref WS_ACTIVE_SUBSCRIPTIONS: IntGauge = register_int_gauge!(
        "websocket_active_subscriptions",
        "Number of active event subscriptions"
    )
    .unwrap();

    /// Messages sent to clients
    pub static ref WS_MESSAGES_SENT: IntCounter = register_int_counter!(
        "websocket_messages_sent_total",
        "Total number of messages sent to WebSocket clients"
    )
    .unwrap();

    /// Messages received from clients
    pub static ref WS_MESSAGES_RECEIVED: IntCounter = register_int_counter!(
        "websocket_messages_received_total",
        "Total number of messages received from WebSocket clients"
    )
    .unwrap();

    /// Events broadcast
    pub static ref WS_EVENTS_BROADCAST: IntCounter = register_int_counter!(
        "websocket_events_broadcast_total",
        "Total number of events broadcast to WebSocket clients"
    )
    .unwrap();

    /// Events delivered
    pub static ref WS_EVENTS_DELIVERED: IntCounter = register_int_counter!(
        "websocket_events_delivered_total",
        "Total number of events successfully delivered to WebSocket clients"
    )
    .unwrap();

    /// Connection errors
    pub static ref WS_CONNECTION_ERRORS: IntCounter = register_int_counter!(
        "websocket_connection_errors_total",
        "Total number of WebSocket connection errors"
    )
    .unwrap();

    /// Message send errors
    pub static ref WS_SEND_ERRORS: IntCounter = register_int_counter!(
        "websocket_send_errors_total",
        "Total number of errors sending messages to clients"
    )
    .unwrap();

    /// Session duration histogram
    pub static ref WS_SESSION_DURATION: Histogram = register_histogram!(
        "websocket_session_duration_seconds",
        "Duration of WebSocket sessions in seconds"
    )
    .unwrap();

    /// Message latency histogram
    pub static ref WS_MESSAGE_LATENCY: Histogram = register_histogram!(
        "websocket_message_latency_seconds",
        "Latency of WebSocket message delivery in seconds"
    )
    .unwrap();

    /// Broadcast channel usage
    pub static ref WS_BROADCAST_CHANNEL_USAGE: Gauge = register_gauge!(
        "websocket_broadcast_channel_usage_ratio",
        "Usage ratio of the broadcast channel (0.0 to 1.0)"
    )
    .unwrap();
}

/// Record a new connection
pub fn record_connection() {
    WS_ACTIVE_CONNECTIONS.inc();
    WS_TOTAL_CONNECTIONS.inc();
}

/// Record a disconnection
pub fn record_disconnection(duration_secs: f64) {
    WS_ACTIVE_CONNECTIONS.dec();
    WS_SESSION_DURATION.observe(duration_secs);
}

/// Record a subscription
pub fn record_subscription() {
    WS_ACTIVE_SUBSCRIPTIONS.inc();
}

/// Record an unsubscription
pub fn record_unsubscription() {
    WS_ACTIVE_SUBSCRIPTIONS.dec();
}

/// Record a message sent
pub fn record_message_sent() {
    WS_MESSAGES_SENT.inc();
}

/// Record a message received
pub fn record_message_received() {
    WS_MESSAGES_RECEIVED.inc();
}

/// Record an event broadcast
pub fn record_event_broadcast() {
    WS_EVENTS_BROADCAST.inc();
}

/// Record an event delivered
pub fn record_event_delivered() {
    WS_EVENTS_DELIVERED.inc();
}

/// Record a connection error
pub fn record_connection_error() {
    WS_CONNECTION_ERRORS.inc();
}

/// Record a send error
pub fn record_send_error() {
    WS_SEND_ERRORS.inc();
}

/// Record message latency
pub fn record_message_latency(latency_secs: f64) {
    WS_MESSAGE_LATENCY.observe(latency_secs);
}

/// Update broadcast channel usage
pub fn update_broadcast_channel_usage(used: usize, capacity: usize) {
    if capacity > 0 {
        let ratio = used as f64 / capacity as f64;
        WS_BROADCAST_CHANNEL_USAGE.set(ratio);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        // Test connection metrics
        let before = WS_ACTIVE_CONNECTIONS.get();
        record_connection();
        assert_eq!(WS_ACTIVE_CONNECTIONS.get(), before + 1);

        record_disconnection(10.5);
        assert_eq!(WS_ACTIVE_CONNECTIONS.get(), before);

        // Test subscription metrics
        let before_sub = WS_ACTIVE_SUBSCRIPTIONS.get();
        record_subscription();
        assert_eq!(WS_ACTIVE_SUBSCRIPTIONS.get(), before_sub + 1);

        record_unsubscription();
        assert_eq!(WS_ACTIVE_SUBSCRIPTIONS.get(), before_sub);

        // Test message metrics
        record_message_sent();
        record_message_received();
        record_event_broadcast();
        record_event_delivered();

        // Test error metrics
        record_connection_error();
        record_send_error();

        // Test latency
        record_message_latency(0.001);

        // Test channel usage
        update_broadcast_channel_usage(50, 100);
        assert_eq!(WS_BROADCAST_CHANNEL_USAGE.get(), 0.5);
    }
}
