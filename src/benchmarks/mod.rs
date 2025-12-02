//! Canonical Benchmark Module for LLM Incident Manager
//!
//! This module provides the standardized benchmarking infrastructure used across
//! all 25 benchmark-target repositories in the LLM-Dev-Ops ecosystem.
//!
//! # Architecture
//!
//! The benchmark system follows the canonical interface:
//! - `BenchmarkResult`: Standard result struct with target_id, metrics, and timestamp
//! - `BenchTarget` trait: Interface for benchmarkable components (in adapters module)
//! - `run_all_benchmarks()`: Main entrypoint returning Vec<BenchmarkResult>
//!
//! # Usage
//!
//! ```rust,ignore
//! use llm_incident_manager::benchmarks;
//!
//! // Run all registered benchmarks
//! let results = benchmarks::run_all_benchmarks().await;
//!
//! // Write results to canonical output directories
//! benchmarks::io::write_results(&results)?;
//!
//! // Generate markdown summary
//! benchmarks::markdown::write_summary(&results)?;
//! ```

pub mod io;
pub mod markdown;
pub mod result;

pub use result::{BenchmarkResult, BenchmarkResultBuilder};

use crate::adapters::{all_targets, BenchTarget};
use std::time::Instant;

/// Run all registered benchmarks and return their results.
///
/// This is the main entrypoint for the canonical benchmark interface.
/// It discovers all registered benchmark targets via the adapters module
/// and executes each one, collecting results.
///
/// # Returns
///
/// A vector of `BenchmarkResult` containing metrics for each benchmark target.
///
/// # Example
///
/// ```rust,ignore
/// use llm_incident_manager::benchmarks;
///
/// #[tokio::main]
/// async fn main() {
///     let results = benchmarks::run_all_benchmarks().await;
///     println!("Executed {} benchmarks", results.len());
///
///     for result in &results {
///         println!("{}: {:?}", result.target_id, result.metrics);
///     }
/// }
/// ```
pub async fn run_all_benchmarks() -> Vec<BenchmarkResult> {
    let targets = all_targets();
    let mut results = Vec::with_capacity(targets.len());

    println!("Running {} benchmark targets...", targets.len());
    println!();

    for target in targets {
        let id = target.id();
        print!("  Running {}... ", id);

        let start = Instant::now();
        let result = target.run().await;
        let elapsed = start.elapsed();

        if result.is_success() {
            println!("OK ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
        } else {
            println!("FAILED ({:.2}ms)", elapsed.as_secs_f64() * 1000.0);
            if let Some(error) = result.metrics.get("error") {
                println!("    Error: {}", error);
            }
        }

        results.push(result);
    }

    println!();
    println!("Completed {} benchmarks", results.len());

    let successful = results.iter().filter(|r| r.is_success()).count();
    let failed = results.len() - successful;
    println!("  Successful: {}", successful);
    println!("  Failed: {}", failed);

    results
}

/// Run benchmarks and write results to canonical output locations.
///
/// This convenience function runs all benchmarks and writes results to:
/// - Individual JSON files in `benchmarks/output/raw/`
/// - A combined summary markdown file at `benchmarks/output/summary.md`
///
/// # Returns
///
/// The vector of benchmark results.
///
/// # Errors
///
/// Returns an error if writing to the filesystem fails.
pub async fn run_and_save_benchmarks() -> std::io::Result<Vec<BenchmarkResult>> {
    let results = run_all_benchmarks().await;

    // Write individual results
    io::write_results(&results)?;

    // Write markdown summary
    markdown::write_summary(&results)?;

    // Write combined JSON
    io::write_combined_results(&results)?;

    println!();
    println!("Results written to:");
    println!("  - {}", io::RAW_DIR);
    println!("  - {}", markdown::SUMMARY_PATH);

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_all_benchmarks() {
        let results = run_all_benchmarks().await;
        // Should have at least one benchmark target registered
        assert!(!results.is_empty(), "Should have registered benchmark targets");

        // All results should have valid target IDs
        for result in &results {
            assert!(!result.target_id.is_empty());
        }
    }
}
