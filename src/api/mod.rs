pub mod handlers;
pub mod routes;

pub use routes::*;

use crate::processing::IncidentProcessor;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub processor: Arc<IncidentProcessor>,
}

impl AppState {
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        Self { processor }
    }
}
