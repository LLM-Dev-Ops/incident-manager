//! Prometheus metrics for the scheduler module

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec,
};

/// Scheduler metrics collection
pub struct SchedulerMetrics {
    /// Number of scheduled jobs
    pub jobs_total: GaugeVec,

    /// Number of job executions
    pub executions_total: CounterVec,

    /// Number of successful job executions
    pub executions_success: CounterVec,

    /// Number of failed job executions
    pub executions_failed: CounterVec,

    /// Job execution duration in seconds
    pub execution_duration: HistogramVec,

    /// Number of currently running jobs
    pub running_jobs: GaugeVec,

    /// Last execution timestamp (Unix timestamp)
    pub last_execution: GaugeVec,
}

impl SchedulerMetrics {
    pub fn new() -> Self {
        Self {
            jobs_total: register_gauge_vec!(
                "scheduler_jobs_total",
                "Total number of scheduled jobs",
                &["status"]
            )
            .unwrap(),

            executions_total: register_counter_vec!(
                "scheduler_executions_total",
                "Total number of job executions",
                &["job_name", "job_id"]
            )
            .unwrap(),

            executions_success: register_counter_vec!(
                "scheduler_executions_success_total",
                "Total number of successful job executions",
                &["job_name", "job_id"]
            )
            .unwrap(),

            executions_failed: register_counter_vec!(
                "scheduler_executions_failed_total",
                "Total number of failed job executions",
                &["job_name", "job_id"]
            )
            .unwrap(),

            execution_duration: register_histogram_vec!(
                "scheduler_execution_duration_seconds",
                "Job execution duration in seconds",
                &["job_name", "job_id"],
                vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0]
            )
            .unwrap(),

            running_jobs: register_gauge_vec!(
                "scheduler_running_jobs",
                "Number of currently running jobs",
                &["job_name"]
            )
            .unwrap(),

            last_execution: register_gauge_vec!(
                "scheduler_last_execution_timestamp",
                "Unix timestamp of last job execution",
                &["job_name", "job_id"]
            )
            .unwrap(),
        }
    }

    /// Record job execution start
    pub fn record_execution_start(&self, job_name: &str) {
        self.running_jobs.with_label_values(&[job_name]).inc();
    }

    /// Record job execution completion
    pub fn record_execution_complete(
        &self,
        job_name: &str,
        job_id: &str,
        success: bool,
        duration_secs: f64,
    ) {
        self.running_jobs.with_label_values(&[job_name]).dec();

        self.executions_total
            .with_label_values(&[job_name, job_id])
            .inc();

        if success {
            self.executions_success
                .with_label_values(&[job_name, job_id])
                .inc();
        } else {
            self.executions_failed
                .with_label_values(&[job_name, job_id])
                .inc();
        }

        self.execution_duration
            .with_label_values(&[job_name, job_id])
            .observe(duration_secs);

        self.last_execution
            .with_label_values(&[job_name, job_id])
            .set(chrono::Utc::now().timestamp() as f64);
    }

    /// Update job count by status
    pub fn update_job_count(&self, status: &str, count: f64) {
        self.jobs_total.with_label_values(&[status]).set(count);
    }
}

impl Default for SchedulerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    /// Global scheduler metrics instance
    pub static ref SCHEDULER_METRICS: SchedulerMetrics = SchedulerMetrics::new();
}

/// Initialize scheduler metrics (idempotent)
pub fn init_scheduler_metrics() {
    lazy_static::initialize(&SCHEDULER_METRICS);
}
