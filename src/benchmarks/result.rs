//! Canonical benchmark result types for LLM Incident Manager
//!
//! This module provides the standardized BenchmarkResult struct used across
//! all 25 benchmark-target repositories for consistent performance measurement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Canonical benchmark result structure.
///
/// This struct represents the standardized output format for all benchmarks
/// in the LLM-Dev-Ops benchmark target ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique identifier for the benchmark target
    pub target_id: String,

    /// Benchmark metrics as flexible JSON value
    /// Can contain any metric structure appropriate for the target
    pub metrics: serde_json::Value,

    /// Timestamp when the benchmark was executed
    pub timestamp: DateTime<Utc>,
}

impl BenchmarkResult {
    /// Create a new BenchmarkResult with the current timestamp
    pub fn new(target_id: impl Into<String>, metrics: serde_json::Value) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp: Utc::now(),
        }
    }

    /// Create a BenchmarkResult with a specific timestamp
    pub fn with_timestamp(
        target_id: impl Into<String>,
        metrics: serde_json::Value,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp,
        }
    }

    /// Check if this result indicates success (no error field in metrics)
    pub fn is_success(&self) -> bool {
        !self.metrics.get("error").is_some_and(|e| !e.is_null())
    }

    /// Get a specific metric value by key
    pub fn get_metric(&self, key: &str) -> Option<&serde_json::Value> {
        self.metrics.get(key)
    }

    /// Get duration in milliseconds if present
    pub fn duration_ms(&self) -> Option<f64> {
        self.metrics
            .get("duration_ms")
            .and_then(|v| v.as_f64())
    }

    /// Get throughput if present
    pub fn throughput(&self) -> Option<f64> {
        self.metrics
            .get("throughput")
            .and_then(|v| v.as_f64())
    }
}

/// Builder for constructing BenchmarkResult with common metrics
#[derive(Debug, Default)]
pub struct BenchmarkResultBuilder {
    target_id: String,
    metrics: serde_json::Map<String, serde_json::Value>,
    timestamp: Option<DateTime<Utc>>,
}

impl BenchmarkResultBuilder {
    /// Create a new builder for the given target
    pub fn new(target_id: impl Into<String>) -> Self {
        Self {
            target_id: target_id.into(),
            metrics: serde_json::Map::new(),
            timestamp: None,
        }
    }

    /// Set the duration in milliseconds
    pub fn duration_ms(mut self, duration: f64) -> Self {
        self.metrics.insert("duration_ms".to_string(), serde_json::json!(duration));
        self
    }

    /// Set the throughput (operations per second)
    pub fn throughput(mut self, ops_per_sec: f64) -> Self {
        self.metrics.insert("throughput".to_string(), serde_json::json!(ops_per_sec));
        self
    }

    /// Set the number of iterations
    pub fn iterations(mut self, count: u64) -> Self {
        self.metrics.insert("iterations".to_string(), serde_json::json!(count));
        self
    }

    /// Set the mean latency
    pub fn mean_latency_ms(mut self, latency: f64) -> Self {
        self.metrics.insert("mean_latency_ms".to_string(), serde_json::json!(latency));
        self
    }

    /// Set the p50 latency
    pub fn p50_latency_ms(mut self, latency: f64) -> Self {
        self.metrics.insert("p50_latency_ms".to_string(), serde_json::json!(latency));
        self
    }

    /// Set the p95 latency
    pub fn p95_latency_ms(mut self, latency: f64) -> Self {
        self.metrics.insert("p95_latency_ms".to_string(), serde_json::json!(latency));
        self
    }

    /// Set the p99 latency
    pub fn p99_latency_ms(mut self, latency: f64) -> Self {
        self.metrics.insert("p99_latency_ms".to_string(), serde_json::json!(latency));
        self
    }

    /// Set an error message
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.metrics.insert("error".to_string(), serde_json::json!(message.into()));
        self
    }

    /// Add a custom metric
    pub fn metric(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }

    /// Set a specific timestamp
    pub fn timestamp(mut self, ts: DateTime<Utc>) -> Self {
        self.timestamp = Some(ts);
        self
    }

    /// Build the BenchmarkResult
    pub fn build(self) -> BenchmarkResult {
        BenchmarkResult {
            target_id: self.target_id,
            metrics: serde_json::Value::Object(self.metrics),
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_new() {
        let result = BenchmarkResult::new(
            "test-target",
            serde_json::json!({
                "duration_ms": 100.5,
                "throughput": 1000.0
            }),
        );

        assert_eq!(result.target_id, "test-target");
        assert_eq!(result.duration_ms(), Some(100.5));
        assert_eq!(result.throughput(), Some(1000.0));
        assert!(result.is_success());
    }

    #[test]
    fn test_benchmark_result_builder() {
        let result = BenchmarkResultBuilder::new("builder-test")
            .duration_ms(50.0)
            .throughput(2000.0)
            .iterations(1000)
            .mean_latency_ms(0.05)
            .p95_latency_ms(0.1)
            .p99_latency_ms(0.2)
            .build();

        assert_eq!(result.target_id, "builder-test");
        assert_eq!(result.duration_ms(), Some(50.0));
        assert_eq!(result.throughput(), Some(2000.0));
        assert!(result.is_success());
    }

    #[test]
    fn test_benchmark_result_with_error() {
        let result = BenchmarkResultBuilder::new("error-test")
            .error("Something went wrong")
            .build();

        assert!(!result.is_success());
    }
}
