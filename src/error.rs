use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Application error types
#[derive(Error, Debug)]
pub enum AppError {
    /// Database errors
    #[error("Database error: {0}")]
    Database(String),

    /// Not found errors
    #[error("Not found: {0}")]
    NotFound(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Authorization errors
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Rate limit errors
    #[error("Rate limit exceeded")]
    RateLimit,

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Internal server errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// Integration errors
    #[error("Integration error ({integration_source}): {message}")]
    Integration { integration_source: String, message: String },

    /// Processing errors
    #[error("Processing error: {0}")]
    Processing(String),

    /// Invalid state transition
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    /// Execution context violation (missing or invalid execution headers)
    #[error("Execution violation: {0}")]
    ExecutionViolation(String),
}

impl AppError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Authentication(_) => StatusCode::UNAUTHORIZED,
            AppError::Authorization(_) => StatusCode::FORBIDDEN,
            AppError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            AppError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            AppError::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Network(_) => StatusCode::BAD_GATEWAY,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Integration { .. } => StatusCode::BAD_GATEWAY,
            AppError::Processing(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidStateTransition(_) => StatusCode::CONFLICT,
            AppError::ExecutionViolation(_) => StatusCode::BAD_REQUEST,
        }
    }

    /// Get error code string
    pub fn error_code(&self) -> &str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::Authentication(_) => "AUTHENTICATION_ERROR",
            AppError::Authorization(_) => "AUTHORIZATION_ERROR",
            AppError::RateLimit => "RATE_LIMIT_EXCEEDED",
            AppError::Timeout(_) => "TIMEOUT",
            AppError::Configuration(_) => "CONFIGURATION_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Io(_) => "IO_ERROR",
            AppError::Serialization(_) => "SERIALIZATION_ERROR",
            AppError::Network(_) => "NETWORK_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
            AppError::Integration { .. } => "INTEGRATION_ERROR",
            AppError::Processing(_) => "PROCESSING_ERROR",
            AppError::InvalidStateTransition(_) => "INVALID_STATE_TRANSITION",
            AppError::ExecutionViolation(_) => "EXECUTION_VIOLATION",
        }
    }
}

/// Convert AppError to HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();

        tracing::error!(
            error_code = error_code,
            status_code = status.as_u16(),
            message = %message,
            "Request error"
        );

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
                "status": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}

/// Conversion from serde_json::Error
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

/// Conversion from serde_yaml::Error
impl From<serde_yaml::Error> for AppError {
    fn from(err: serde_yaml::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

/// Conversion from validator::ValidationErrors
impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::Validation(err.to_string())
    }
}

/// Conversion from config::ConfigError
impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        AppError::Configuration(err.to_string())
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AppError::NotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::Validation("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Authentication("test".to_string()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::RateLimit.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            AppError::NotFound("test".to_string()).error_code(),
            "NOT_FOUND"
        );
        assert_eq!(
            AppError::Validation("test".to_string()).error_code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(AppError::RateLimit.error_code(), "RATE_LIMIT_EXCEEDED");
    }
}
