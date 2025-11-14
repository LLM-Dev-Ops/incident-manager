//! WebSocket server implementation
//!
//! This module implements the WebSocket server using Axum's WebSocket support.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, State, WebSocketUpgrade,
    },
    response::Response,
};
use chrono::Utc;
use futures::stream::StreamExt;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use super::{
    connection::MessageWriter,
    messages::{ClientMessage, ServerMessage},
    session::Session,
    WebSocketState,
};

/// WebSocket endpoint handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebSocketState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    info!(remote_addr = %addr, "WebSocket connection request");

    ws.on_upgrade(move |socket| handle_socket(socket, state, addr))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<WebSocketState>, addr: SocketAddr) {
    // Split socket into sender and receiver
    let (sender, mut receiver) = socket.split();
    let mut writer = MessageWriter::new(sender);

    // Create session
    let session = Session::new();
    let session_id = session.id.clone();

    info!(session_id = %session_id, remote_addr = %addr, "WebSocket session started");

    // Register connection
    let (connection, mut message_rx) = state
        .connections
        .register(session, Some(addr.to_string()));

    // Send welcome message
    let welcome = ServerMessage::Welcome {
        session_id: session_id.clone(),
        server_time: Utc::now(),
    };

    if let Err(e) = writer.send(welcome).await {
        error!(session_id = %session_id, error = ?e, "Failed to send welcome message");
        return;
    }

    // Spawn message sender task
    let session_id_clone = session_id.clone();
    let sender_handle = tokio::spawn(async move {
        while let Some(message) = message_rx.recv().await {
            if let Err(e) = writer.send(message).await {
                error!(session_id = %session_id_clone, error = ?e, "Failed to send message");
                break;
            }
        }

        // Close the writer when done
        let _ = writer.close().await;
    });

    // Spawn heartbeat task
    let connection_clone = connection.clone();
    let heartbeat_interval = state.config.heartbeat_interval_secs;
    let heartbeat_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(heartbeat_interval));
        loop {
            ticker.tick().await;

            let pong = ServerMessage::Pong {
                timestamp: Utc::now(),
            };

            if connection_clone.send(pong).is_err() {
                debug!("Heartbeat failed, connection closed");
                break;
            }
        }
    });

    // Process incoming messages
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                error!(session_id = %session_id, error = ?e, "WebSocket error");
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                debug!(session_id = %session_id, "Received text message");

                if let Err(e) = handle_client_message(&text, &connection, &state).await {
                    error!(session_id = %session_id, error = ?e, "Failed to handle message");

                    let error_msg = ServerMessage::Error {
                        code: "INVALID_MESSAGE".to_string(),
                        message: format!("Failed to process message: {}", e),
                    };

                    let _ = connection.send(error_msg);
                }
            }
            Message::Binary(_) => {
                warn!(session_id = %session_id, "Received binary message (not supported)");
                let error_msg = ServerMessage::Error {
                    code: "UNSUPPORTED".to_string(),
                    message: "Binary messages are not supported".to_string(),
                };
                let _ = connection.send(error_msg);
            }
            Message::Ping(_) => {
                debug!(session_id = %session_id, "Received ping");
                // Axum automatically handles pong responses
            }
            Message::Pong(_) => {
                debug!(session_id = %session_id, "Received pong");
                connection.session.write().touch();
            }
            Message::Close(_) => {
                info!(session_id = %session_id, "Client closed connection");
                break;
            }
        }
    }

    // Cleanup
    info!(session_id = %session_id, "WebSocket session ended");
    sender_handle.abort();
    heartbeat_handle.abort();
    state.connections.unregister(&session_id);
}

/// Handle a client message
async fn handle_client_message(
    text: &str,
    connection: &Arc<super::connection::Connection>,
    _state: &Arc<WebSocketState>,
) -> Result<(), String> {
    let message: ClientMessage = serde_json::from_str(text)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let session_id = connection.session_id();

    match message {
        ClientMessage::Subscribe {
            subscription_id,
            filters,
        } => {
            info!(
                session_id = %session_id,
                subscription_id = %subscription_id,
                "Client subscribing to events"
            );

            connection.session.write().subscribe(subscription_id.clone(), filters.clone());

            let response = ServerMessage::Subscribed {
                subscription_id,
                filters,
            };
            connection.send(response).map_err(|e| e.to_string())?;
        }
        ClientMessage::Unsubscribe { subscription_id } => {
            info!(
                session_id = %session_id,
                subscription_id = %subscription_id,
                "Client unsubscribing from events"
            );

            let removed = connection.session.write().unsubscribe(&subscription_id);

            if removed {
                let response = ServerMessage::Unsubscribed { subscription_id };
                connection.send(response).map_err(|e| e.to_string())?;
            } else {
                let error = ServerMessage::Error {
                    code: "NOT_FOUND".to_string(),
                    message: format!("Subscription {} not found", subscription_id),
                };
                connection.send(error).map_err(|e| e.to_string())?;
            }
        }
        ClientMessage::Ping { timestamp } => {
            debug!(session_id = %session_id, "Client ping");
            connection.session.write().touch();

            let response = ServerMessage::Pong { timestamp };
            connection.send(response).map_err(|e| e.to_string())?;
        }
        ClientMessage::Ack { message_id } => {
            debug!(session_id = %session_id, message_id = %message_id, "Message acknowledged");
            connection.session.write().touch();
        }
    }

    Ok(())
}

/// Periodic cleanup task for expired sessions
pub async fn cleanup_task(state: Arc<WebSocketState>) {
    let mut ticker = interval(Duration::from_secs(state.config.cleanup_interval_secs));

    loop {
        ticker.tick().await;

        debug!("Running WebSocket cleanup task");
        state.connections.cleanup_expired(state.config.session_timeout_secs as i64).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::WebSocketConfig;

    #[test]
    fn test_client_message_parsing() {
        let json = r#"{"type":"subscribe","subscription_id":"sub1","filters":{"event_types":[],"severities":[],"states":[],"sources":[],"affected_resources":[],"labels":{},"incident_ids":[]}}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();

        match msg {
            ClientMessage::Subscribe { subscription_id, .. } => {
                assert_eq!(subscription_id, "sub1");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Error {
            code: "TEST_ERROR".to_string(),
            message: "This is a test".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("TEST_ERROR"));
    }

    #[tokio::test]
    async fn test_websocket_state_creation() {
        let config = WebSocketConfig::default();
        let state = WebSocketState::new(config);

        assert_eq!(state.connections.connection_count(), 0);
    }
}
