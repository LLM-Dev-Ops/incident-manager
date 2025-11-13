use thiserror::Error;

/// Integration-specific error types
#[derive(Error, Debug)]
pub enum IntegrationError {
    /// Connection errors
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request failed
    #[error("Request failed: {0}")]
    RequestFailed(String),

    /// Invalid response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Timeout error
    #[error("Operation timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },

    /// Retry exhausted
    #[error("Retry exhausted for {operation} after {attempts} attempts")]
    RetryExhausted { operation: String, attempts: u32 },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntegrationError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            IntegrationError::Connection(_)
                | IntegrationError::Network(_)
                | IntegrationError::Timeout { .. }
        )
    }
}

/// Result type for integration operations
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// Convert from tonic::Status to IntegrationError
impl From<tonic::Status> for IntegrationError {
    fn from(status: tonic::Status) -> Self {
        use tonic::Code;
        match status.code() {
            Code::Unauthenticated => {
                IntegrationError::Authentication(status.message().to_string())
            }
            Code::Unavailable => IntegrationError::Connection(status.message().to_string()),
            Code::DeadlineExceeded => IntegrationError::Timeout { timeout_secs: 0 },
            Code::ResourceExhausted => IntegrationError::RateLimit(status.message().to_string()),
            _ => IntegrationError::RequestFailed(status.message().to_string()),
        }
    }
}

/// Convert IntegrationError to AppError
impl From<IntegrationError> for crate::error::AppError {
    fn from(err: IntegrationError) -> Self {
        match err {
            IntegrationError::Connection(msg) => crate::error::AppError::Network(msg),
            IntegrationError::RequestFailed(msg) => crate::error::AppError::Integration {
                integration_source: "LLM".to_string(),
                message: msg,
            },
            IntegrationError::InvalidResponse(msg) => crate::error::AppError::Integration {
                integration_source: "LLM".to_string(),
                message: msg,
            },
            IntegrationError::Timeout { timeout_secs } => {
                crate::error::AppError::Timeout(format!("Timeout after {} seconds", timeout_secs))
            }
            IntegrationError::RetryExhausted { operation, attempts } => {
                crate::error::AppError::Integration {
                    integration_source: "LLM".to_string(),
                    message: format!("Retry exhausted for {} after {} attempts", operation, attempts),
                }
            }
            IntegrationError::Configuration(msg) => crate::error::AppError::Configuration(msg),
            IntegrationError::Authentication(msg) => crate::error::AppError::Authentication(msg),
            IntegrationError::RateLimit(msg) => crate::error::AppError::Integration {
                integration_source: "LLM".to_string(),
                message: format!("Rate limit: {}", msg),
            },
            IntegrationError::Network(msg) => crate::error::AppError::Network(msg),
            IntegrationError::Serialization(msg) => crate::error::AppError::Serialization(msg),
            IntegrationError::Internal(msg) => crate::error::AppError::Internal(msg),
        }
    }
}
