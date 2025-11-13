//! Error types for the scheduler module

use crate::error::AppError;

/// Result type for scheduler operations
pub type SchedulerResult<T> = std::result::Result<T, SchedulerError>;

/// Errors that can occur in scheduler operations
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    /// Scheduler failed to start
    #[error("Failed to start scheduler: {0}")]
    StartupFailed(String),

    /// Scheduler failed to shutdown
    #[error("Failed to shutdown scheduler: {0}")]
    ShutdownFailed(String),

    /// Job creation failed
    #[error("Failed to create job: {0}")]
    JobCreationFailed(String),

    /// Job not found
    #[error("Job not found: {0}")]
    JobNotFound(String),

    /// Job execution failed
    #[error("Job execution failed: {0}")]
    JobExecutionFailed(String),

    /// Invalid cron expression
    #[error("Invalid cron expression: {0}")]
    InvalidCronExpression(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Internal scheduler error
    #[error("Internal scheduler error: {0}")]
    InternalError(String),

    /// Job already exists
    #[error("Job already exists: {0}")]
    JobAlreadyExists(String),
}

impl From<SchedulerError> for AppError {
    fn from(err: SchedulerError) -> Self {
        match err {
            SchedulerError::JobNotFound(msg) => AppError::NotFound(msg),
            SchedulerError::InvalidCronExpression(msg) | SchedulerError::ConfigurationError(msg) => {
                AppError::Configuration(msg)
            }
            _ => AppError::Internal(err.to_string()),
        }
    }
}

impl From<tokio_cron_scheduler::JobSchedulerError> for SchedulerError {
    fn from(err: tokio_cron_scheduler::JobSchedulerError) -> Self {
        SchedulerError::InternalError(format!("tokio-cron-scheduler error: {}", err))
    }
}
