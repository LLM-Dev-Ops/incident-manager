//! Benchmark Adapters Module for LLM Incident Manager
//!
//! This module provides the canonical `BenchTarget` trait and implements
//! benchmark adapters for key incident management components.
//!
//! # Architecture
//!
//! The adapter pattern allows existing components to be benchmarked without
//! modifying their core logic. Each adapter wraps a component and provides:
//! - A unique identifier via `id()`
//! - A benchmark implementation via `run()`
//!
//! # Available Targets
//!
//! - `DeduplicationBenchTarget`: Alert deduplication performance
//! - `EscalationBenchTarget`: Escalation workflow latency
//! - `CorrelationBenchTarget`: Incident correlation throughput
//! - `CircuitBreakerBenchTarget`: Circuit breaker overhead
//! - `ProcessorBenchTarget`: Alert processing pipeline
//! - `RoutingBenchTarget`: On-call routing evaluation

mod targets;

pub use targets::*;

use crate::benchmarks::BenchmarkResult;
use async_trait::async_trait;

/// Canonical trait for benchmarkable components.
///
/// This trait defines the interface that all benchmark targets must implement
/// to participate in the canonical benchmark system used across all 25
/// benchmark-target repositories.
///
/// # Implementation Notes
///
/// - `id()` should return a unique, stable identifier for the benchmark
/// - `run()` should execute the benchmark and return a `BenchmarkResult`
/// - Implementations should be stateless or manage their own state
///
/// # Example
///
/// ```rust,ignore
/// use llm_incident_manager::adapters::BenchTarget;
/// use llm_incident_manager::benchmarks::BenchmarkResult;
/// use async_trait::async_trait;
///
/// struct MyBenchTarget;
///
/// #[async_trait]
/// impl BenchTarget for MyBenchTarget {
///     fn id(&self) -> String {
///         "my-custom-target".to_string()
///     }
///
///     async fn run(&self) -> BenchmarkResult {
///         // Perform benchmark operations
///         BenchmarkResult::new(
///             self.id(),
///             serde_json::json!({
///                 "duration_ms": 100.0,
///                 "iterations": 1000
///             })
///         )
///     }
/// }
/// ```
#[async_trait]
pub trait BenchTarget: Send + Sync {
    /// Returns the unique identifier for this benchmark target.
    ///
    /// The ID should be:
    /// - Unique across all registered targets
    /// - Stable across runs (same ID for same target)
    /// - Descriptive of what is being benchmarked
    ///
    /// Convention: Use kebab-case, e.g., "alert-deduplication"
    fn id(&self) -> String;

    /// Executes the benchmark and returns the result.
    ///
    /// This method should:
    /// - Set up any necessary test data
    /// - Execute the benchmark operation multiple times
    /// - Measure timing and other relevant metrics
    /// - Clean up any test data
    /// - Return a `BenchmarkResult` with the collected metrics
    async fn run(&self) -> BenchmarkResult;
}

/// Returns a vector of all registered benchmark targets.
///
/// This is the registry function that provides all benchmarkable components
/// to the `run_all_benchmarks()` entrypoint.
///
/// # Adding New Targets
///
/// To add a new benchmark target:
/// 1. Create a struct implementing `BenchTarget`
/// 2. Add it to the `targets` module
/// 3. Add an instance to this function's return vector
///
/// # Example
///
/// ```rust,ignore
/// use llm_incident_manager::adapters::all_targets;
///
/// let targets = all_targets();
/// for target in &targets {
///     println!("Registered: {}", target.id());
/// }
/// ```
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    vec![
        // Core Processing Pipeline
        Box::new(DeduplicationBenchTarget::new()),
        Box::new(ProcessorBenchTarget::new()),

        // Escalation & Routing
        Box::new(EscalationBenchTarget::new()),
        Box::new(RoutingBenchTarget::new()),

        // Correlation
        Box::new(CorrelationBenchTarget::new()),

        // Resilience Patterns
        Box::new(CircuitBreakerBenchTarget::new()),

        // Metrics System
        Box::new(MetricsBenchTarget::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_targets_registered() {
        let targets = all_targets();
        assert!(!targets.is_empty(), "Should have registered targets");

        // Check for unique IDs
        let ids: Vec<_> = targets.iter().map(|t| t.id()).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len(), "All target IDs should be unique");
    }

    #[tokio::test]
    async fn test_all_targets_can_run() {
        let targets = all_targets();
        for target in targets {
            let result = target.run().await;
            assert!(!result.target_id.is_empty());
        }
    }
}
