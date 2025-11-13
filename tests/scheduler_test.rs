//! Comprehensive tests for the scheduler module

use llm_incident_manager::scheduler::{
    Job, JobContext, JobMetadata, SchedulerConfig, SchedulerService,
};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

#[tokio::test]
async fn test_scheduler_creation() {
    let config = SchedulerConfig::default();
    let result = SchedulerService::new(config).await;
    assert!(result.is_ok(), "Should create scheduler successfully");
}

#[tokio::test]
async fn test_scheduler_start_stop() {
    let config = SchedulerConfig::default();
    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    // Start scheduler
    let result = scheduler.start().await;
    assert!(result.is_ok(), "Should start scheduler successfully");
    assert!(scheduler.is_running().await, "Scheduler should be running");

    // Stop scheduler
    let result = scheduler.shutdown().await;
    assert!(result.is_ok(), "Should stop scheduler successfully");
    assert!(!scheduler.is_running().await, "Scheduler should not be running");
}

#[tokio::test]
async fn test_add_simple_job() {
    let config = SchedulerConfig::default();
    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    // Create a simple job
    let metadata = JobMetadata::new("test_job", "*/5 * * * * *"); // Every 5 seconds
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let job = Job::new(metadata, move |_ctx| {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let job_id = scheduler
        .add_job(job)
        .await
        .expect("Failed to add job");

    // Wait a bit to allow job to run
    tokio::time::sleep(Duration::from_secs(6)).await;

    // Job should have run at least once
    let count = counter.load(Ordering::SeqCst);
    assert!(count >= 1, "Job should have run at least once, got {}", count);

    // Clean up
    scheduler.remove_job(&job_id).await.expect("Failed to remove job");
    scheduler.shutdown().await.expect("Failed to stop scheduler");
}

#[tokio::test]
async fn test_job_metadata_updates() {
    let config = SchedulerConfig::default();
    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    let metadata = JobMetadata::new("metadata_test", "*/2 * * * * *"); // Every 2 seconds
    let job = Job::new(metadata, |_ctx| async { Ok(()) });

    let job_id = scheduler.add_job(job).await.expect("Failed to add job");

    // Wait for job to run
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check metadata was updated
    let metadata = scheduler
        .get_job_metadata(&job_id)
        .await
        .expect("Failed to get metadata");

    assert!(metadata.run_count >= 1, "Run count should be at least 1");
    assert!(metadata.last_run.is_some(), "Last run should be set");

    scheduler.shutdown().await.expect("Failed to stop scheduler");
}

#[tokio::test]
async fn test_list_jobs() {
    let config = SchedulerConfig::default();
    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    // Add multiple jobs
    let job1 = Job::new(
        JobMetadata::new("job1", "0 0 * * * *"),
        |_ctx| async { Ok(()) },
    );
    let job2 = Job::new(
        JobMetadata::new("job2", "0 0 * * * *"),
        |_ctx| async { Ok(()) },
    );

    scheduler.add_job(job1).await.expect("Failed to add job1");
    scheduler.add_job(job2).await.expect("Failed to add job2");

    // List jobs
    let jobs = scheduler.list_jobs().await;
    assert_eq!(jobs.len(), 2, "Should have 2 jobs");

    scheduler.shutdown().await.expect("Failed to stop scheduler");
}

#[tokio::test]
async fn test_scheduler_stats() {
    let config = SchedulerConfig::default();
    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    // Add a job
    let job = Job::new(
        JobMetadata::new("stats_job", "*/2 * * * * *"),
        |_ctx| async { Ok(()) },
    );
    scheduler.add_job(job).await.expect("Failed to add job");

    // Wait for job to run
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Get stats
    let stats = scheduler.get_stats().await;
    assert_eq!(stats.total_jobs, 1, "Should have 1 job");
    assert!(stats.total_executions >= 1, "Should have at least 1 execution");

    scheduler.shutdown().await.expect("Failed to stop scheduler");
}

#[tokio::test]
async fn test_job_with_context() {
    let config = SchedulerConfig::default();
    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    let metadata = JobMetadata::new("context_job", "*/2 * * * * *")
        .with_description("Test job with context")
        .with_tags(vec!["test".to_string(), "context".to_string()]);

    let job = Job::new(metadata, |ctx| async move {
        let metadata = ctx.get_metadata().await;
        assert_eq!(metadata.name, "context_job");
        assert_eq!(metadata.description, Some("Test job with context".to_string()));
        assert_eq!(metadata.tags.len(), 2);
        Ok(())
    });

    scheduler.add_job(job).await.expect("Failed to add job");

    // Wait for job to run
    tokio::time::sleep(Duration::from_secs(3)).await;

    scheduler.shutdown().await.expect("Failed to stop scheduler");
}

#[tokio::test]
async fn test_job_failure_tracking() {
    let config = SchedulerConfig::default();
    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let metadata = JobMetadata::new("failing_job", "*/2 * * * * *");
    let job = Job::new(metadata, move |_ctx| {
        let counter = counter_clone.clone();
        async move {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err("Intentional failure".to_string())
            } else {
                Ok(())
            }
        }
    });

    let job_id = scheduler.add_job(job).await.expect("Failed to add job");

    // Wait for multiple runs
    tokio::time::sleep(Duration::from_secs(7)).await;

    let metadata = scheduler
        .get_job_metadata(&job_id)
        .await
        .expect("Failed to get metadata");

    assert!(metadata.failure_count >= 2, "Should have at least 2 failures");
    assert!(metadata.success_count >= 1, "Should have at least 1 success");

    scheduler.shutdown().await.expect("Failed to stop scheduler");
}

#[tokio::test]
async fn test_disabled_scheduler() {
    let config = SchedulerConfig {
        enabled: false,
        ..Default::default()
    };

    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    // Starting should succeed but do nothing
    let result = scheduler.start().await;
    assert!(result.is_ok(), "Should handle disabled scheduler gracefully");
}

#[tokio::test]
async fn test_concurrent_jobs() {
    let config = SchedulerConfig {
        max_concurrent_jobs: 5,
        ..Default::default()
    };

    let mut scheduler = SchedulerService::new(config)
        .await
        .expect("Failed to create scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    // Add multiple jobs that run at the same time
    for i in 0..3 {
        let metadata = JobMetadata::new(format!("concurrent_job_{}", i), "*/2 * * * * *");
        let job = Job::new(metadata, move |_ctx| async move {
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(())
        });
        scheduler.add_job(job).await.expect("Failed to add job");
    }

    // Wait for jobs to run
    tokio::time::sleep(Duration::from_secs(3)).await;

    let stats = scheduler.get_stats().await;
    assert_eq!(stats.total_jobs, 3, "Should have 3 jobs");
    assert!(stats.total_executions >= 3, "Should have executed all jobs");

    scheduler.shutdown().await.expect("Failed to stop scheduler");
}
