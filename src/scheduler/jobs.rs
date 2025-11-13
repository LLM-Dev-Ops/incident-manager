//! Job definitions and management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Unique identifier for a scheduled job
pub type JobId = Uuid;

/// Status of a scheduled job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is scheduled and will run
    Scheduled,
    /// Job is currently running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
    /// Job is paused
    Paused,
    /// Job is cancelled
    Cancelled,
}

/// Metadata about a scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    /// Unique job identifier
    pub id: JobId,

    /// Human-readable job name
    pub name: String,

    /// Job description
    pub description: Option<String>,

    /// Cron expression
    pub schedule: String,

    /// Current job status
    pub status: JobStatus,

    /// When the job was created
    pub created_at: DateTime<Utc>,

    /// When the job was last updated
    pub updated_at: DateTime<Utc>,

    /// When the job last ran
    pub last_run: Option<DateTime<Utc>>,

    /// When the job will next run
    pub next_run: Option<DateTime<Utc>>,

    /// Number of times the job has run
    pub run_count: u64,

    /// Number of successful runs
    pub success_count: u64,

    /// Number of failed runs
    pub failure_count: u64,

    /// Average execution duration in milliseconds
    pub avg_duration_ms: f64,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl JobMetadata {
    pub fn new(name: impl Into<String>, schedule: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            schedule: schedule.into(),
            status: JobStatus::Scheduled,
            created_at: now,
            updated_at: now,
            last_run: None,
            next_run: None,
            run_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_duration_ms: 0.0,
            tags: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn update_execution(&mut self, success: bool, duration_ms: u64) {
        self.run_count += 1;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        // Update average duration using incremental formula
        self.avg_duration_ms = ((self.avg_duration_ms * (self.run_count - 1) as f64)
            + duration_ms as f64)
            / self.run_count as f64;

        self.last_run = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn success_rate(&self) -> f64 {
        if self.run_count == 0 {
            0.0
        } else {
            (self.success_count as f64 / self.run_count as f64) * 100.0
        }
    }
}

/// Context passed to job execution functions
#[derive(Clone)]
pub struct JobContext {
    /// Job metadata
    pub metadata: Arc<tokio::sync::RwLock<JobMetadata>>,

    /// Application state (if needed)
    pub app_state: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl JobContext {
    pub fn new(metadata: JobMetadata) -> Self {
        Self {
            metadata: Arc::new(tokio::sync::RwLock::new(metadata)),
            app_state: None,
        }
    }

    pub fn with_app_state<T: std::any::Any + Send + Sync>(
        mut self,
        state: Arc<T>,
    ) -> Self {
        self.app_state = Some(state);
        self
    }

    pub async fn get_metadata(&self) -> JobMetadata {
        self.metadata.read().await.clone()
    }

    pub async fn update_metadata<F>(&self, f: F)
    where
        F: FnOnce(&mut JobMetadata),
    {
        let mut metadata = self.metadata.write().await;
        f(&mut *metadata);
    }
}

/// A scheduled job
pub struct Job {
    /// Job metadata
    pub metadata: Arc<tokio::sync::RwLock<JobMetadata>>,

    /// Job execution function
    pub execute: Arc<dyn Fn(JobContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> + Send + Sync>,
}

impl Job {
    pub fn new<F, Fut>(metadata: JobMetadata, execute: F) -> Self
    where
        F: Fn(JobContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        Self {
            metadata: Arc::new(tokio::sync::RwLock::new(metadata)),
            execute: Arc::new(move |ctx| Box::pin(execute(ctx))),
        }
    }

    pub async fn get_metadata(&self) -> JobMetadata {
        self.metadata.read().await.clone()
    }

    pub async fn execute(&self, ctx: JobContext) -> Result<(), String> {
        let start = std::time::Instant::now();

        // Update status to Running
        {
            let mut metadata = self.metadata.write().await;
            metadata.status = JobStatus::Running;
            metadata.updated_at = Utc::now();
        }

        // Execute the job
        let result = (self.execute)(ctx.clone()).await;

        // Update execution statistics
        let duration_ms = start.elapsed().as_millis() as u64;
        {
            let mut metadata = self.metadata.write().await;
            metadata.update_execution(result.is_ok(), duration_ms);
            metadata.status = if result.is_ok() {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            };
        }

        result
    }
}
