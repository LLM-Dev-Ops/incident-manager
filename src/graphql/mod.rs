//! GraphQL API module
//!
//! Production-ready GraphQL server with:
//! - Type-safe schema and resolvers
//! - Query, Mutation, and Subscription support
//! - DataLoaders for N+1 query prevention
//! - Pagination and filtering
//! - Real-time subscriptions via WebSocket
//! - Metrics and tracing integration

pub mod context;
pub mod dataloaders;
pub mod metrics;
pub mod mutations;
pub mod queries;
pub mod schema;
pub mod subscriptions;
pub mod types;

pub use context::GraphQLContext;
pub use schema::{build_schema, GraphQLSchema};

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Json, Router,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::processing::IncidentProcessor;

#[derive(Clone)]
struct GraphQLState {
    schema: GraphQLSchema,
    processor: Arc<IncidentProcessor>,
}

#[derive(Debug, Deserialize)]
struct GraphQLRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct GraphQLResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    errors: Vec<serde_json::Value>,
}

/// Build GraphQL routes for Axum
///
/// Returns a router with:
/// - POST /graphql - GraphQL endpoint
/// - GET /graphql/playground - GraphQL Playground UI
pub fn graphql_routes(processor: Arc<IncidentProcessor>) -> Router {
    let schema = build_schema();
    let state = GraphQLState { schema, processor };

    Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        // TODO: Add WebSocket subscriptions - requires async_graphql_axum version alignment
        // .route("/graphql/ws", get(graphql_subscription_handler))
        .with_state(state)
}

/// GraphQL query/mutation handler
async fn graphql_handler(
    State(state): State<GraphQLState>,
    Json(req): Json<GraphQLRequest>,
) -> Json<GraphQLResponse> {
    let ctx = GraphQLContext::new(state.processor.clone());

    let mut request = async_graphql::Request::new(req.query);

    if let Some(op_name) = req.operation_name {
        request = request.operation_name(op_name);
    }

    if let Some(vars) = req.variables {
        if let Ok(vars) = serde_json::from_value(vars) {
            request = request.variables(vars);
        }
    }

    request = request.data(ctx);

    let response = state.schema.execute(request).await;

    Json(GraphQLResponse {
        data: Some(serde_json::to_value(&response.data).unwrap_or_default()),
        errors: response.errors.into_iter()
            .map(|e| serde_json::to_value(&e).unwrap_or_default())
            .collect(),
    })
}

/// GraphQL Playground UI handler
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_compiles() {
        let schema = build_schema();
        let sdl = schema.sdl();

        // Verify key types exist in schema
        assert!(sdl.contains("type Incident"));
        assert!(sdl.contains("type Alert"));
        assert!(sdl.contains("type Playbook"));
        assert!(sdl.contains("type Query"));
        assert!(sdl.contains("type Mutation"));
        assert!(sdl.contains("type Subscription"));
    }

    #[test]
    fn test_schema_has_queries() {
        let schema = build_schema();
        let sdl = schema.sdl();

        assert!(sdl.contains("incident("));
        assert!(sdl.contains("incidents("));
        assert!(sdl.contains("activeIncidents("));
        assert!(sdl.contains("criticalIncidents("));
    }

    #[test]
    fn test_schema_has_mutations() {
        let schema = build_schema();
        let sdl = schema.sdl();

        assert!(sdl.contains("createIncident("));
        assert!(sdl.contains("updateIncident("));
        assert!(sdl.contains("resolveIncident("));
        assert!(sdl.contains("submitAlert("));
    }

    #[test]
    fn test_schema_has_subscriptions() {
        let schema = build_schema();
        let sdl = schema.sdl();

        assert!(sdl.contains("incidentUpdates("));
        assert!(sdl.contains("newIncidents("));
        assert!(sdl.contains("criticalIncidents:"));
    }
}
