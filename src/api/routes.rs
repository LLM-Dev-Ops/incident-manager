use crate::api::{handlers, AppState};
use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

/// Build the main API router
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(handlers::health_check))
        .route("/health/live", get(handlers::health_check))
        .route("/health/ready", get(handlers::health_check))
        // Alert ingestion
        .route("/v1/alerts", post(handlers::submit_alert))
        // Incident management
        .route("/v1/incidents", post(handlers::create_incident))
        .route("/v1/incidents", get(handlers::list_incidents))
        .route("/v1/incidents/:id", get(handlers::get_incident))
        .route("/v1/incidents/:id", put(handlers::update_incident))
        .route("/v1/incidents/:id/resolve", post(handlers::resolve_incident))
        // Add state
        .with_state(state)
        // Add middleware
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true)),
        )
        .layer(CorsLayer::permissive())
}
