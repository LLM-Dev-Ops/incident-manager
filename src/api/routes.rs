use crate::api::{handlers, AppState};
use crate::execution::middleware::execution_context_middleware;
use axum::{
    middleware,
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
        // Metrics endpoint
        .route("/metrics", get(handlers::metrics))
        // Alert ingestion
        .route("/v1/alerts", post(handlers::submit_alert))
        // Incident management
        .route("/v1/incidents", post(handlers::create_incident))
        .route("/v1/incidents", get(handlers::list_incidents))
        .route("/v1/incidents/:id", get(handlers::get_incident))
        .route("/v1/incidents/:id", put(handlers::update_incident))
        .route("/v1/incidents/:id/resolve", post(handlers::resolve_incident))
        // Add WebSocket endpoint if WebSocket is enabled
        .route(
            "/ws",
            get(|ws: axum::extract::ws::WebSocketUpgrade,
                axum::extract::State(app_state): axum::extract::State<AppState>,
                connect_info: axum::extract::ConnectInfo<std::net::SocketAddr>| async move {
                if let Some(ws_state) = &app_state.websocket {
                    crate::websocket::websocket_handler(
                        ws,
                        axum::extract::State(ws_state.clone()),
                        connect_info
                    ).await
                } else {
                    axum::response::Response::builder()
                        .status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
                        .body(axum::body::Body::from("WebSocket not enabled"))
                        .unwrap()
                }
            })
        )
        // Add state after all routes
        .with_state(state)
        // Middleware stack (applied bottom-to-top: execution context -> trace -> cors)
        .layer(middleware::from_fn(execution_context_middleware))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true)),
        )
        .layer(CorsLayer::permissive())
}
