//! Error types for messaging operations

use crate::error::AppError;

/// Result type for messaging operations
pub type MessagingResult<T> = std::result::Result<T, MessagingError>;

/// Errors that can occur during messaging operations
#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Publish failed
    #[error("Publish failed: {0}")]
    PublishFailed(String),

    /// Subscribe failed
    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),

    /// Consume failed
    #[error("Consume failed: {0}")]
    ConsumeFailed(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Backend not available
    #[error("Backend not available: {0}")]
    BackendUnavailable(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
}

impl From<serde_json::Error> for MessagingError {
    fn from(err: serde_json::Error) -> Self {
        MessagingError::SerializationError(err.to_string())
    }
}

impl From<MessagingError> for AppError {
    fn from(err: MessagingError) -> Self {
        match err {
            MessagingError::ConfigurationError(msg) => AppError::Configuration(msg),
            _ => AppError::Internal(err.to_string()),
        }
    }
}
