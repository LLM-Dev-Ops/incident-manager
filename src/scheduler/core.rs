//! Core scheduler service implementation

use super::{
    config::SchedulerConfig, error::{SchedulerError, SchedulerResult}, jobs::{Job, JobContext, JobId, JobMetadata},
    metrics::SCHEDULER_METRICS,
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, JobSchedulerError};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Main scheduler service that manages all scheduled jobs
pub struct SchedulerService {
    /// Configuration
    config: SchedulerConfig,

    /// Underlying tokio-cron-scheduler instance
    scheduler: JobScheduler,

    /// Registered jobs
    jobs: Arc<DashMap<JobId, Arc<Job>>>,

    /// Whether the scheduler is running
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl SchedulerService {
    /// Create a new scheduler service
    pub async fn new(config: SchedulerConfig) -> SchedulerResult<Self> {
        info!("Initializing scheduler service");

        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| SchedulerError::StartupFailed(e.to_string()))?;

        Ok(Self {
            config,
            scheduler,
            jobs: Arc::new(DashMap::new()),
            running: Arc::new(tokio::sync::RwLock::new(false)),
        })
    }

    /// Start the scheduler
    pub async fn start(&mut self) -> SchedulerResult<()> {
        if !self.config.enabled {
            info!("Scheduler is disabled in configuration");
            return Ok(());
        }

        {
            let mut running = self.running.write().await;
            if *running {
                warn!("Scheduler is already running");
                return Ok(());
            }
            *running = true;
        }

        info!("Starting scheduler service");

        self.scheduler
            .start()
            .await
            .map_err(|e| SchedulerError::StartupFailed(e.to_string()))?;

        info!("Scheduler service started successfully");

        Ok(())
    }

    /// Stop the scheduler
    pub async fn shutdown(&mut self) -> SchedulerResult<()> {
        info!("Shutting down scheduler service");

        {
            let mut running = self.running.write().await;
            if !*running {
                warn!("Scheduler is not running");
                return Ok(());
            }
            *running = false;
        }

        self.scheduler
            .shutdown()
            .await
            .map_err(|e| SchedulerError::ShutdownFailed(e.to_string()))?;

        info!("Scheduler service shut down successfully");

        Ok(())
    }

    /// Add a new job to the scheduler
    pub async fn add_job(&self, job: Job) -> SchedulerResult<JobId> {
        let metadata = job.get_metadata().await;
        let job_id = metadata.id;
        let job_name = metadata.name.clone();
        let schedule = metadata.schedule.clone();

        info!(job_id = %job_id, job_name = %job_name, "Adding job to scheduler");

        // Store job in our registry
        let job_arc = Arc::new(job);
        self.jobs.insert(job_id, job_arc.clone());

        // Create tokio-cron-scheduler job
        let job_arc_clone = job_arc.clone();
        let cron_job = tokio_cron_scheduler::Job::new_async(schedule.as_str(), move |_uuid, _l| {
            let job = job_arc_clone.clone();
            Box::pin(async move {
                let metadata = job.get_metadata().await;
                let job_name = metadata.name.clone();
                let job_id = metadata.id;

                debug!(job_id = %job_id, job_name = %job_name, "Executing scheduled job");

                // Record metrics
                SCHEDULER_METRICS.record_execution_start(&job_name);

                let start = std::time::Instant::now();
                let ctx = JobContext::new(metadata);
                let result = job.execute(ctx).await;
                let duration = start.elapsed();

                // Record completion metrics
                SCHEDULER_METRICS.record_execution_complete(
                    &job_name,
                    &job_id.to_string(),
                    result.is_ok(),
                    duration.as_secs_f64(),
                );

                match result {
                    Ok(()) => {
                        info!(
                            job_id = %job_id,
                            job_name = %job_name,
                            duration_ms = duration.as_millis(),
                            "Job executed successfully"
                        );
                    }
                    Err(e) => {
                        error!(
                            job_id = %job_id,
                            job_name = %job_name,
                            error = %e,
                            duration_ms = duration.as_millis(),
                            "Job execution failed"
                        );
                    }
                }
            })
        })
        .map_err(|e: JobSchedulerError| SchedulerError::JobCreationFailed(e.to_string()))?;

        self.scheduler
            .add(cron_job)
            .await
            .map_err(|e| SchedulerError::JobCreationFailed(e.to_string()))?;

        info!(job_id = %job_id, job_name = %job_name, "Job added successfully");

        // Update metrics
        SCHEDULER_METRICS.update_job_count("scheduled", self.jobs.len() as f64);

        Ok(job_id)
    }

    /// Remove a job from the scheduler
    pub async fn remove_job(&self, job_id: &JobId) -> SchedulerResult<()> {
        info!(job_id = %job_id, "Removing job from scheduler");

        self.jobs
            .remove(job_id)
            .ok_or_else(|| SchedulerError::JobNotFound(job_id.to_string()))?;

        // Update metrics
        SCHEDULER_METRICS.update_job_count("scheduled", self.jobs.len() as f64);

        info!(job_id = %job_id, "Job removed successfully");

        Ok(())
    }

    /// Get job metadata
    pub async fn get_job_metadata(&self, job_id: &JobId) -> SchedulerResult<JobMetadata> {
        let job = self
            .jobs
            .get(job_id)
            .ok_or_else(|| SchedulerError::JobNotFound(job_id.to_string()))?;

        Ok(job.get_metadata().await)
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<JobMetadata> {
        let mut jobs = Vec::new();
        for entry in self.jobs.iter() {
            if let Ok(metadata) = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                entry.value().get_metadata(),
            )
            .await
            {
                jobs.push(metadata);
            }
        }
        jobs
    }

    /// Get scheduler statistics
    pub async fn get_stats(&self) -> SchedulerStats {
        let jobs = self.list_jobs().await;

        let total_jobs = jobs.len();
        let running_jobs = jobs
            .iter()
            .filter(|j| j.status == super::jobs::JobStatus::Running)
            .count();
        let paused_jobs = jobs
            .iter()
            .filter(|j| j.status == super::jobs::JobStatus::Paused)
            .count();

        let total_executions: u64 = jobs.iter().map(|j| j.run_count).sum();
        let total_successes: u64 = jobs.iter().map(|j| j.success_count).sum();
        let total_failures: u64 = jobs.iter().map(|j| j.failure_count).sum();

        let success_rate = if total_executions > 0 {
            (total_successes as f64 / total_executions as f64) * 100.0
        } else {
            0.0
        };

        SchedulerStats {
            total_jobs,
            running_jobs,
            paused_jobs,
            total_executions,
            total_successes,
            total_failures,
            success_rate,
        }
    }

    /// Check if scheduler is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Statistics about the scheduler
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchedulerStats {
    pub total_jobs: usize,
    pub running_jobs: usize,
    pub paused_jobs: usize,
    pub total_executions: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub success_rate: f64,
}
