/// Metrics registry management
///
/// This module provides a centralized registry for managing all metrics
/// and provides utilities for metric lifecycle management.

use super::*;
use std::sync::Arc;

/// Central metrics registry
///
/// This struct provides a high-level API for managing all metrics
/// in the application.
pub struct MetricsRegistry {
    config: Arc<MetricsConfig>,
    runtime_collector: Arc<RuntimeCollector>,
}

impl MetricsRegistry {
    /// Create a new metrics registry with default configuration
    pub fn new() -> Self {
        Self {
            config: Arc::new(MetricsConfig::default()),
            runtime_collector: Arc::new(RuntimeCollector::new()),
        }
    }

    /// Create a new metrics registry with custom configuration
    pub fn with_config(config: MetricsConfig) -> Self {
        Self {
            config: Arc::new(config),
            runtime_collector: Arc::new(RuntimeCollector::new()),
        }
    }

    /// Initialize the registry and start background collection
    ///
    /// This should be called once at application startup.
    pub fn init(&self) -> Result<(), prometheus::Error> {
        // Initialize metrics
        init_metrics()?;

        // Start runtime collector if enabled
        if self.config.include_runtime {
            self.runtime_collector.clone().start_periodic_collection(60);
            tracing::info!("Runtime metrics collector started (60s interval)");
        }

        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &MetricsConfig {
        &self.config
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> String {
        gather_metrics()
    }

    /// Reset all metrics
    ///
    /// WARNING: This is primarily for testing. Use with caution in production.
    #[cfg(test)]
    pub fn reset(&self) {
        // Prometheus metrics cannot be easily reset in Rust
        // This would require re-registering all metrics
        tracing::warn!("Metrics reset requested - not fully implemented");
    }

    /// Get metrics summary
    pub fn summary(&self) -> MetricsSummary {
        let metrics_text = self.export();

        // Parse metrics to count types
        let mut counter_count = 0;
        let mut gauge_count = 0;
        let mut histogram_count = 0;

        for line in metrics_text.lines() {
            if line.starts_with("# TYPE") {
                if line.contains("counter") {
                    counter_count += 1;
                } else if line.contains("gauge") {
                    gauge_count += 1;
                } else if line.contains("histogram") {
                    histogram_count += 1;
                }
            }
        }

        MetricsSummary {
            enabled: self.config.enabled,
            total_metrics: counter_count + gauge_count + histogram_count,
            counter_count,
            gauge_count,
            histogram_count,
            config: self.config.as_ref().clone(),
        }
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of metrics registry state
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSummary {
    pub enabled: bool,
    pub total_metrics: usize,
    pub counter_count: usize,
    pub gauge_count: usize,
    pub histogram_count: usize,
    pub config: MetricsConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = MetricsRegistry::new();
        assert!(registry.config().enabled);
    }

    #[test]
    fn test_registry_with_config() {
        let config = MetricsConfig::disabled();
        let registry = MetricsRegistry::with_config(config);
        assert!(!registry.config().enabled);
    }

    #[test]
    fn test_registry_export() {
        let registry = MetricsRegistry::new();
        let metrics = registry.export();
        assert!(!metrics.is_empty());
        assert!(metrics.contains("llm_incident_manager"));
    }

    #[test]
    fn test_metrics_summary() {
        let registry = MetricsRegistry::new();
        let _ = registry.init(); // May fail if already initialized

        let summary = registry.summary();
        assert!(summary.total_metrics > 0);
        assert!(summary.counter_count > 0);
        assert!(summary.gauge_count > 0);
        assert!(summary.histogram_count > 0);
    }
}
