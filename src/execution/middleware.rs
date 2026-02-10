use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use super::context::ExecutionContext;

/// Paths that are excluded from execution context enforcement.
/// These are infrastructure endpoints that don't participate in the agentics execution graph.
const EXCLUDED_PATHS: &[&str] = &["/health", "/health/live", "/health/ready", "/metrics", "/ws"];

/// Axum middleware that extracts execution context from request headers.
///
/// For `/v1/*` paths:
/// - Requires `X-Execution-Id` and `X-Parent-Span-Id` headers
/// - Returns 400 if either header is missing or invalid
/// - Creates an `ExecutionContext` and inserts it into request extensions
///
/// For excluded paths (health, metrics, ws):
/// - Passes through without requiring headers
pub async fn execution_context_middleware(mut req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();

    // Skip excluded paths
    if EXCLUDED_PATHS.iter().any(|p| path == *p) {
        return next.run(req).await;
    }

    // Only enforce execution context on /v1/* API paths
    if !path.starts_with("/v1/") {
        return next.run(req).await;
    }

    match extract_execution_headers(req.headers()) {
        Ok((execution_id, parent_span_id)) => {
            let ctx = ExecutionContext::new(execution_id, parent_span_id);
            req.extensions_mut().insert(ctx);
            next.run(req).await
        }
        Err(error_msg) => {
            tracing::warn!(path = %path, error = %error_msg, "Rejected request: missing execution context");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": {
                        "code": "EXECUTION_VIOLATION",
                        "message": error_msg,
                        "status": 400
                    }
                })),
            )
                .into_response()
        }
    }
}

/// Extract and validate execution headers from an HTTP request.
fn extract_execution_headers(headers: &HeaderMap) -> Result<(Uuid, Uuid), String> {
    let execution_id_str = headers
        .get("x-execution-id")
        .ok_or("Missing required header: X-Execution-Id")?
        .to_str()
        .map_err(|_| "Invalid X-Execution-Id header: not valid UTF-8")?;

    let parent_span_id_str = headers
        .get("x-parent-span-id")
        .ok_or("Missing required header: X-Parent-Span-Id")?
        .to_str()
        .map_err(|_| "Invalid X-Parent-Span-Id header: not valid UTF-8")?;

    let execution_id = Uuid::parse_str(execution_id_str)
        .map_err(|_| "X-Execution-Id must be a valid UUID")?;

    let parent_span_id = Uuid::parse_str(parent_span_id_str)
        .map_err(|_| "X-Parent-Span-Id must be a valid UUID")?;

    Ok((execution_id, parent_span_id))
}

/// Extract execution context from gRPC request metadata.
///
/// Used by gRPC service implementations to extract the same execution context
/// headers that the HTTP middleware handles.
pub fn extract_execution_context_from_grpc_metadata(
    metadata: &tonic::metadata::MetadataMap,
) -> Result<ExecutionContext, tonic::Status> {
    let execution_id_str = metadata
        .get("x-execution-id")
        .ok_or_else(|| tonic::Status::invalid_argument("Missing required metadata: x-execution-id"))?
        .to_str()
        .map_err(|_| tonic::Status::invalid_argument("Invalid x-execution-id: not valid UTF-8"))?;

    let parent_span_id_str = metadata
        .get("x-parent-span-id")
        .ok_or_else(|| {
            tonic::Status::invalid_argument("Missing required metadata: x-parent-span-id")
        })?
        .to_str()
        .map_err(|_| {
            tonic::Status::invalid_argument("Invalid x-parent-span-id: not valid UTF-8")
        })?;

    let execution_id = Uuid::parse_str(execution_id_str)
        .map_err(|_| tonic::Status::invalid_argument("x-execution-id must be a valid UUID"))?;

    let parent_span_id = Uuid::parse_str(parent_span_id_str)
        .map_err(|_| tonic::Status::invalid_argument("x-parent-span-id must be a valid UUID"))?;

    Ok(ExecutionContext::new(execution_id, parent_span_id))
}

/// Attach an ExecutionGraph to gRPC response metadata as a JSON-encoded binary value.
pub fn attach_execution_graph_to_grpc_response<T>(
    response: &mut tonic::Response<T>,
    graph: &crate::execution::types::ExecutionGraph,
) {
    if let Ok(graph_json) = serde_json::to_string(graph) {
        if let Ok(val) = graph_json.parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>()
        {
            response
                .metadata_mut()
                .insert("x-execution-graph", val);
        }
    }
}
