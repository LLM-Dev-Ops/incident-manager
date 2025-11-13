// Rust WebSocket Client Example
//
// This example demonstrates how to connect to the LLM Incident Manager
// WebSocket API and subscribe to real-time incident updates using Rust.
//
// Add to Cargo.toml:
// ```toml
// [dependencies]
// tokio = { version = "1.35", features = ["full"] }
// tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// futures-util = "0.3"
// anyhow = "1.0"
// ```

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

type WsConnection = WebSocketStream<MaybeTlsStream<TcpStream>>;

// GraphQL WebSocket message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "connection_init")]
    ConnectionInit { payload: serde_json::Value },

    #[serde(rename = "connection_ack")]
    ConnectionAck,

    #[serde(rename = "subscribe")]
    Subscribe {
        id: String,
        payload: SubscriptionPayload,
    },

    #[serde(rename = "next")]
    Next {
        id: String,
        payload: serde_json::Value,
    },

    #[serde(rename = "error")]
    Error {
        id: String,
        payload: Vec<serde_json::Value>,
    },

    #[serde(rename = "complete")]
    Complete { id: String },

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "pong")]
    Pong,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionPayload {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<serde_json::Value>,
}

// Domain types
#[derive(Debug, Deserialize)]
struct Incident {
    id: String,
    title: String,
    description: String,
    severity: String,
    state: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "affectedResources")]
    affected_resources: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct IncidentUpdate {
    #[serde(rename = "updateType")]
    update_type: String,
    #[serde(rename = "incidentId")]
    incident_id: String,
    timestamp: String,
}

/// WebSocket client for incident monitoring
struct IncidentMonitor {
    ws: WsConnection,
}

impl IncidentMonitor {
    /// Connect to the WebSocket server
    async fn connect(url: &str, token: &str) -> Result<Self> {
        let (ws, _) = connect_async(url)
            .await
            .context("Failed to connect to WebSocket")?;

        let mut monitor = Self { ws };

        // Initialize connection with authentication
        monitor
            .send_message(WsMessage::ConnectionInit {
                payload: json!({
                    "Authorization": format!("Bearer {}", token)
                }),
            })
            .await?;

        // Wait for connection acknowledgment
        match monitor.receive_message().await? {
            Some(WsMessage::ConnectionAck) => {
                println!("✅ Connected to incident stream");
                Ok(monitor)
            }
            _ => anyhow::bail!("Expected connection_ack"),
        }
    }

    /// Send a message to the server
    async fn send_message(&mut self, msg: WsMessage) -> Result<()> {
        let json = serde_json::to_string(&msg)?;
        self.ws.send(Message::Text(json)).await?;
        Ok(())
    }

    /// Receive a message from the server
    async fn receive_message(&mut self) -> Result<Option<WsMessage>> {
        if let Some(msg) = self.ws.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let ws_msg: WsMessage = serde_json::from_str(&text)?;
                return Ok(Some(ws_msg));
            }
        }
        Ok(None)
    }

    /// Subscribe to critical incidents
    async fn subscribe_to_critical_incidents(&mut self) -> Result<()> {
        let query = r#"
            subscription {
              criticalIncidents {
                id
                title
                description
                severity
                state
                createdAt
                affectedResources
              }
            }
        "#;

        self.send_message(WsMessage::Subscribe {
            id: "critical-incidents".to_string(),
            payload: SubscriptionPayload {
                query: query.to_string(),
                variables: None,
            },
        })
        .await?;

        println!("Subscribed to critical incidents");
        Ok(())
    }

    /// Subscribe to incident updates
    async fn subscribe_to_incident_updates(&mut self, severities: Vec<&str>) -> Result<()> {
        let query = r#"
            subscription IncidentUpdates($severities: [Severity!]) {
              incidentUpdates(severities: $severities, activeOnly: true) {
                updateType
                incidentId
                timestamp
              }
            }
        "#;

        self.send_message(WsMessage::Subscribe {
            id: "incident-updates".to_string(),
            payload: SubscriptionPayload {
                query: query.to_string(),
                variables: Some(json!({ "severities": severities })),
            },
        })
        .await?;

        println!("Subscribed to incident updates (severities: {:?})", severities);
        Ok(())
    }

    /// Handle incoming messages
    async fn handle_messages(&mut self) -> Result<()> {
        while let Some(msg) = self.receive_message().await? {
            match msg {
                WsMessage::Next { id, payload } => {
                    self.handle_subscription_data(&id, payload)?;
                }
                WsMessage::Error { id, payload } => {
                    eprintln!("❌ Subscription error ({}): {:?}", id, payload);
                }
                WsMessage::Complete { id } => {
                    println!("Subscription completed: {}", id);
                }
                WsMessage::Ping => {
                    self.send_message(WsMessage::Pong).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Handle subscription data based on subscription ID
    fn handle_subscription_data(&self, id: &str, payload: serde_json::Value) -> Result<()> {
        match id {
            "critical-incidents" => {
                if let Some(data) = payload.get("data").and_then(|d| d.get("criticalIncidents")) {
                    let incident: Incident = serde_json::from_value(data.clone())?;
                    self.handle_critical_incident(incident);
                }
            }
            "incident-updates" => {
                if let Some(data) = payload.get("data").and_then(|d| d.get("incidentUpdates")) {
                    let update: IncidentUpdate = serde_json::from_value(data.clone())?;
                    self.handle_incident_update(update);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle critical incident notification
    fn handle_critical_incident(&self, incident: Incident) {
        println!("\n{}", "=".repeat(60));
        println!("🚨 CRITICAL INCIDENT ALERT");
        println!("{}", "=".repeat(60));
        println!("ID:          {}", incident.id);
        println!("Title:       {}", incident.title);
        println!("Severity:    {}", incident.severity);
        println!("State:       {}", incident.state);
        println!("Created:     {}", incident.created_at);
        println!("Affected:    {}", incident.affected_resources.join(", "));
        println!("{}\n", "=".repeat(60));

        // Send notifications
        // send_pagerduty_alert(&incident);
        // send_slack_alert(&incident);
    }

    /// Handle incident update notification
    fn handle_incident_update(&self, update: IncidentUpdate) {
        println!(
            "📢 {}: {} at {}",
            update.update_type, update.incident_id, update.timestamp
        );

        // Update dashboard
        // update_dashboard(&update);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let ws_url = std::env::var("WS_URL").unwrap_or_else(|_| "ws://localhost:8080/graphql/ws".to_string());
    let auth_token = std::env::var("AUTH_TOKEN").unwrap_or_else(|_| "YOUR_JWT_TOKEN".to_string());

    println!("Connecting to {}...", ws_url);

    let mut monitor = IncidentMonitor::connect(&ws_url, &auth_token).await?;

    // Subscribe to critical incidents
    monitor.subscribe_to_critical_incidents().await?;

    // Subscribe to incident updates for P0 and P1
    monitor
        .subscribe_to_incident_updates(vec!["P0", "P1"])
        .await?;

    println!("Monitoring incidents... Press Ctrl+C to stop.");

    // Handle incoming messages
    if let Err(e) = monitor.handle_messages().await {
        eprintln!("Error handling messages: {}", e);
    }

    println!("Disconnected");
    Ok(())
}

// Mock notification functions (implement these based on your needs)
#[allow(dead_code)]
fn send_pagerduty_alert(_incident: &Incident) {
    // Implementation for PagerDuty integration
    println!("Would send to PagerDuty");
}

#[allow(dead_code)]
fn send_slack_alert(_incident: &Incident) {
    // Implementation for Slack integration
    println!("Would send to Slack");
}

#[allow(dead_code)]
fn update_dashboard(_update: &IncidentUpdate) {
    // Implementation for dashboard update
    println!("Would update dashboard");
}
