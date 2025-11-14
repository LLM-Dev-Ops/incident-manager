//! HTTP middleware for circuit breaker integration.

use crate::circuit_breaker::{get_circuit_breaker, CircuitBreakerConfig, CircuitBreakerError};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

/// Middleware layer for circuit breaker
#[derive(Clone)]
#[allow(dead_code)]
pub struct CircuitBreakerLayer {
    name: String,
    config: CircuitBreakerConfig,
}

#[allow(dead_code)]
impl CircuitBreakerLayer {
    /// Create a new circuit breaker layer
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// Create for HTTP API calls
    pub fn for_http_api(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::for_http_api())
    }

    /// Create for LLM service calls
    pub fn for_llm_service(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::for_llm_service())
    }
}

impl<S> Layer<S> for CircuitBreakerLayer {
    type Service = CircuitBreakerMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let breaker = get_circuit_breaker(&self.name, self.config.clone());
        CircuitBreakerMiddleware {
            inner,
            breaker,
        }
    }
}

/// Middleware service that applies circuit breaker logic
#[derive(Clone)]
pub struct CircuitBreakerMiddleware<S> {
    inner: S,
    breaker: std::sync::Arc<crate::circuit_breaker::CircuitBreaker>,
}

impl<S> CircuitBreakerMiddleware<S> {
    /// Create a new middleware instance
    pub fn new(inner: S, name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let breaker = get_circuit_breaker(name, config);
        Self { inner, breaker }
    }
}

impl<S> Service<Request<Body>> for CircuitBreakerMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let breaker = self.breaker.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Try to execute the request through the circuit breaker
            let result = breaker
                .call(|| {
                    Box::pin(async move {
                        let response = inner.call(req).await;
                        match response {
                            Ok(resp) => {
                                // Check if response indicates a failure (5xx status codes)
                                if resp.status().is_server_error() {
                                    Err(CircuitBreakerError::OperationFailed(
                                        format!("Server error: {}", resp.status()),
                                    ))
                                } else {
                                    Ok(resp)
                                }
                            }
                            Err(err) => Err(CircuitBreakerError::OperationFailed(
                                format!("Request failed: {:?}", err.into()),
                            )),
                        }
                    })
                })
                .await;

            match result {
                Ok(response) => Ok(response),
                Err(CircuitBreakerError::Open(name)) => {
                    warn!(circuit_breaker = %name, "Circuit breaker is open, rejecting request");
                    Ok(CircuitBreakerOpenResponse::new(name).into_response())
                }
                Err(err) => {
                    warn!(error = %err, "Circuit breaker operation failed");
                    Ok(CircuitBreakerErrorResponse::new(err).into_response())
                }
            }
        })
    }
}

/// Response when circuit breaker is open
struct CircuitBreakerOpenResponse {
    breaker_name: String,
}

impl CircuitBreakerOpenResponse {
    fn new(breaker_name: String) -> Self {
        Self { breaker_name }
    }
}

impl IntoResponse for CircuitBreakerOpenResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": {
                "code": "CIRCUIT_BREAKER_OPEN",
                "message": format!("Circuit breaker '{}' is open. Service temporarily unavailable.", self.breaker_name),
                "status": 503
            }
        });

        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(body),
        )
            .into_response()
    }
}

/// Response when circuit breaker operation fails
struct CircuitBreakerErrorResponse {
    error: CircuitBreakerError,
}

impl CircuitBreakerErrorResponse {
    fn new(error: CircuitBreakerError) -> Self {
        Self { error }
    }
}

impl IntoResponse for CircuitBreakerErrorResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": {
                "code": "CIRCUIT_BREAKER_ERROR",
                "message": self.error.to_string(),
                "status": 502
            }
        });

        (StatusCode::BAD_GATEWAY, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_circuit_breaker_layer_creation() {
        let config = CircuitBreakerConfig::default();
        let layer = CircuitBreakerLayer::new("test", config);
        assert_eq!(layer.name, "test");
    }

    #[tokio::test]
    async fn test_circuit_breaker_layer_presets() {
        let _http_layer = CircuitBreakerLayer::for_http_api("http-test");
        let _llm_layer = CircuitBreakerLayer::for_llm_service("llm-test");
        let _default_layer = CircuitBreakerLayer::with_defaults("default-test");
    }
}
