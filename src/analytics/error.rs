//! Error types for analytics operations

use crate::error::AppError;

/// Result type for analytics operations
pub type AnalyticsResult<T> = std::result::Result<T, AnalyticsError>;

/// Errors that can occur in analytics operations
#[derive(Debug, thiserror::Error)]
pub enum AnalyticsError {
    /// Invalid date range
    #[error("Invalid date range: {0}")]
    InvalidDateRange(String),

    /// Insufficient data for analysis
    #[error("Insufficient data for analysis: {0}")]
    InsufficientData(String),

    /// Report generation failed
    #[error("Report generation failed: {0}")]
    ReportGenerationFailed(String),

    /// Export failed
    #[error("Export failed: {0}")]
    ExportFailed(String),

    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Calculation error
    #[error("Calculation error: {0}")]
    CalculationError(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

impl From<AnalyticsError> for AppError {
    fn from(err: AnalyticsError) -> Self {
        match err {
            AnalyticsError::InvalidDateRange(msg)
            | AnalyticsError::InvalidConfiguration(msg) => AppError::Configuration(msg),
            AnalyticsError::DatabaseError(msg) => AppError::Database(msg),
            _ => AppError::Internal(err.to_string()),
        }
    }
}
