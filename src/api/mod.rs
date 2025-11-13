pub mod handlers;
pub mod routes;

pub use routes::*;

use crate::{processing::IncidentProcessor, websocket::WebSocketState};
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub processor: Arc<IncidentProcessor>,
    pub websocket: Option<Arc<WebSocketState>>,
}

impl AppState {
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        Self {
            processor,
            websocket: None,
        }
    }

    /// Set the WebSocket state
    pub fn with_websocket(mut self, websocket: Arc<WebSocketState>) -> Self {
        self.websocket = Some(websocket);
        self
    }
}
