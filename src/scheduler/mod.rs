//! Enterprise-grade scheduler and cron job management
//!
//! This module provides comprehensive scheduled task execution using tokio-cron-scheduler.
//!
//! # Features
//!
//! - **Cron Expression Support**: Standard cron syntax for flexible scheduling
//! - **Job Management**: Add, remove, pause, and resume jobs dynamically
//! - **Metrics Integration**: Prometheus metrics for job execution tracking
//! - **Error Handling**: Comprehensive error types and recovery
//! - **Observability**: Full logging and tracing integration
//! - **Thread Safety**: Safe concurrent job execution
//!
//! # Example
//!
//! ```no_run
//! use llm_incident_manager::scheduler::{SchedulerService, SchedulerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SchedulerConfig::default();
//!     let mut scheduler = SchedulerService::new(config).await?;
//!
//!     scheduler.start().await?;
//!
//!     // Scheduler runs in background
//!     tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
//!
//!     scheduler.shutdown().await?;
//!     Ok(())
//! }
//! ```

mod config;
mod core;
mod error;
mod jobs;
mod metrics;
mod tasks;

pub use config::{SchedulerConfig, SchedulerConfigBuilder};
pub use core::SchedulerService;
pub use error::{SchedulerError, SchedulerResult};
pub use jobs::{Job, JobContext, JobId, JobMetadata, JobStatus};
pub use metrics::{init_scheduler_metrics, SCHEDULER_METRICS};
pub use tasks::{
    cleanup_old_incidents, generate_daily_reports, monitor_stale_incidents,
    refresh_correlation_rules, sync_external_systems, update_ml_models,
};
