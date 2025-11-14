/// Custom metric collectors for specialized metrics
///
/// This module provides custom collectors for metrics that don't fit
/// the standard counter/gauge/histogram pattern, such as:
/// - Runtime statistics (memory, GC, etc.)
/// - System resource usage
/// - Custom aggregations

use super::*;
use std::sync::Arc;
use std::time::SystemTime;

/// Runtime metrics collector
///
/// Collects runtime statistics like memory usage, thread count, etc.
pub struct RuntimeCollector {
    start_time: SystemTime,
}

impl RuntimeCollector {
    /// Create a new runtime collector
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
        }
    }

    /// Update runtime metrics
    ///
    /// This should be called periodically (e.g., every 60 seconds)
    /// to update runtime statistics.
    pub fn collect(&self) {
        // Update uptime
        if let Ok(duration) = self.start_time.elapsed() {
            UPTIME_SECONDS.set(duration.as_secs_f64());
        }

        // Note: Rust doesn't have built-in APIs for memory/thread stats like Node.js
        // In production, you'd integrate with system-specific APIs or crates like:
        // - sysinfo crate for system metrics
        // - jemalloc/mimalloc stats for memory metrics
        // - tokio-metrics for async runtime metrics
    }

    /// Start periodic collection
    ///
    /// Spawns a background task that collects metrics every interval.
    pub fn start_periodic_collection(self: Arc<Self>, interval_secs: u64) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                self.collect();
            }
        });
    }
}

impl Default for RuntimeCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for common metric operations
pub mod helpers {
    use super::*;

    /// Record a storage operation
    pub fn record_storage_operation(
        operation: &str,
        backend: &str,
        duration_secs: f64,
    ) {
        STORAGE_OPERATIONS_TOTAL
            .with_label_values(&[operation, backend])
            .inc();

        STORAGE_OPERATION_DURATION_SECONDS
            .with_label_values(&[operation, backend])
            .observe(duration_secs);
    }

    /// Record a notification
    pub fn record_notification(channel: &str, success: bool, duration_secs: f64) {
        let status = if success { "success" } else { "error" };

        NOTIFICATIONS_TOTAL
            .with_label_values(&[channel, status])
            .inc();

        NOTIFICATION_DURATION_SECONDS
            .with_label_values(&[channel])
            .observe(duration_secs);
    }

    /// Record incident deduplication
    pub fn record_deduplication() {
        INCIDENTS_DEDUPLICATED_TOTAL.inc();
    }

    /// Record incident correlation
    pub fn record_correlation() {
        INCIDENTS_CORRELATED_TOTAL.inc();
    }

    /// Record incident escalation
    pub fn record_escalation(level: &str) {
        INCIDENTS_ESCALATED_TOTAL
            .with_label_values(&[level])
            .inc();
    }

    /// Update notification queue size
    pub fn update_notification_queue_size(channel: &str, size: f64) {
        NOTIFICATION_QUEUE_SIZE
            .with_label_values(&[channel])
            .set(size);
    }

    /// Update storage size
    pub fn update_storage_size(backend: &str, size_bytes: f64) {
        STORAGE_SIZE_BYTES
            .with_label_values(&[backend])
            .set(size_bytes);
    }

    /// Record a generic error
    pub fn record_error(component: &str, error_type: &str) {
        ERRORS_TOTAL
            .with_label_values(&[component, error_type])
            .inc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::helpers::*;

    #[test]
    fn test_runtime_collector() {
        let collector = RuntimeCollector::new();
        collector.collect();

        let metrics = gather_metrics();
        assert!(metrics.contains("uptime_seconds"));
    }

    #[test]
    fn test_record_storage_operation() {
        record_storage_operation("read", "sled", 0.005);

        let metrics = gather_metrics();
        assert!(metrics.contains("storage_operations_total"));
        assert!(metrics.contains("storage_operation_duration_seconds"));
    }

    #[test]
    fn test_record_notification() {
        record_notification("slack", true, 0.5);

        let metrics = gather_metrics();
        assert!(metrics.contains("notifications_total"));
    }

    #[test]
    fn test_record_deduplication() {
        record_deduplication();

        let metrics = gather_metrics();
        assert!(metrics.contains("incidents_deduplicated_total"));
    }

    #[test]
    fn test_record_escalation() {
        record_escalation("level2");

        let metrics = gather_metrics();
        assert!(metrics.contains("incidents_escalated_total"));
    }

    #[test]
    fn test_update_queue_size() {
        update_notification_queue_size("email", 42.0);

        let metrics = gather_metrics();
        assert!(metrics.contains("notification_queue_size"));
    }
}
