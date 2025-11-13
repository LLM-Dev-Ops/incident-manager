//! Configuration for the scheduler module

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the scheduler service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Whether the scheduler is enabled
    pub enabled: bool,

    /// Timezone for cron expressions (e.g., "UTC", "America/New_York")
    pub timezone: String,

    /// Maximum number of concurrent job executions
    pub max_concurrent_jobs: usize,

    /// Job execution timeout
    pub job_timeout: Duration,

    /// Enable persistent job storage (requires database)
    pub persistent_storage: bool,

    /// Number of job execution history entries to keep
    pub history_retention_count: usize,

    /// Predefined jobs configuration
    pub jobs: JobsConfig,
}

/// Configuration for predefined scheduled jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobsConfig {
    /// Cleanup old incidents job
    pub cleanup_old_incidents: JobConfig,

    /// Generate daily reports job
    pub generate_daily_reports: JobConfig,

    /// Monitor stale incidents job
    pub monitor_stale_incidents: JobConfig,

    /// Refresh correlation rules job
    pub refresh_correlation_rules: JobConfig,

    /// Sync external systems job
    pub sync_external_systems: JobConfig,

    /// Update ML models job
    pub update_ml_models: JobConfig,
}

/// Configuration for a single scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    /// Whether this job is enabled
    pub enabled: bool,

    /// Cron expression for scheduling
    pub schedule: String,

    /// Job-specific configuration as JSON
    #[serde(default)]
    pub config: serde_json::Value,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timezone: "UTC".to_string(),
            max_concurrent_jobs: 10,
            job_timeout: Duration::from_secs(300), // 5 minutes
            persistent_storage: false,
            history_retention_count: 100,
            jobs: JobsConfig::default(),
        }
    }
}

impl Default for JobsConfig {
    fn default() -> Self {
        Self {
            cleanup_old_incidents: JobConfig {
                enabled: true,
                schedule: "0 2 * * *".to_string(), // Daily at 2 AM
                config: serde_json::json!({
                    "retention_days": 90,
                    "batch_size": 100
                }),
            },
            generate_daily_reports: JobConfig {
                enabled: true,
                schedule: "0 8 * * *".to_string(), // Daily at 8 AM
                config: serde_json::json!({
                    "report_types": ["summary", "sla", "trends"]
                }),
            },
            monitor_stale_incidents: JobConfig {
                enabled: true,
                schedule: "*/15 * * * *".to_string(), // Every 15 minutes
                config: serde_json::json!({
                    "stale_threshold_hours": 24,
                    "escalate": true
                }),
            },
            refresh_correlation_rules: JobConfig {
                enabled: true,
                schedule: "0 */6 * * *".to_string(), // Every 6 hours
                config: serde_json::json!({
                    "rebuild_index": false
                }),
            },
            sync_external_systems: JobConfig {
                enabled: true,
                schedule: "*/30 * * * *".to_string(), // Every 30 minutes
                config: serde_json::json!({
                    "systems": ["jira", "servicenow", "pagerduty"]
                }),
            },
            update_ml_models: JobConfig {
                enabled: true,
                schedule: "0 0 * * 0".to_string(), // Weekly on Sunday at midnight
                config: serde_json::json!({
                    "models": ["severity_classifier", "correlation_detector"],
                    "min_training_samples": 1000
                }),
            },
        }
    }
}

/// Builder for SchedulerConfig
pub struct SchedulerConfigBuilder {
    config: SchedulerConfig,
}

impl SchedulerConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: SchedulerConfig::default(),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.config.timezone = timezone.into();
        self
    }

    pub fn max_concurrent_jobs(mut self, max: usize) -> Self {
        self.config.max_concurrent_jobs = max;
        self
    }

    pub fn job_timeout(mut self, timeout: Duration) -> Self {
        self.config.job_timeout = timeout;
        self
    }

    pub fn persistent_storage(mut self, enabled: bool) -> Self {
        self.config.persistent_storage = enabled;
        self
    }

    pub fn build(self) -> SchedulerConfig {
        self.config
    }
}

impl Default for SchedulerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
