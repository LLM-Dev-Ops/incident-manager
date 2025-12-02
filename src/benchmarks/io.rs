//! I/O utilities for benchmark results
//!
//! This module handles reading and writing benchmark results to the
//! canonical output directories.

use crate::benchmarks::BenchmarkResult;
use chrono::Utc;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Default output directory for benchmark results
pub const OUTPUT_DIR: &str = "benchmarks/output";

/// Default directory for raw JSON results
pub const RAW_DIR: &str = "benchmarks/output/raw";

/// Write a single benchmark result to the raw output directory
pub fn write_result(result: &BenchmarkResult) -> std::io::Result<PathBuf> {
    write_result_to_dir(result, RAW_DIR)
}

/// Write a single benchmark result to a specific directory
pub fn write_result_to_dir(result: &BenchmarkResult, dir: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;

    let filename = format!(
        "{}_{}.json",
        result.target_id.replace(['/', ':', ' '], "_"),
        result.timestamp.format("%Y%m%d_%H%M%S")
    );
    let path = dir.join(&filename);

    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, result)?;
    writer.flush()?;

    Ok(path)
}

/// Write multiple benchmark results to the raw output directory
pub fn write_results(results: &[BenchmarkResult]) -> std::io::Result<Vec<PathBuf>> {
    results.iter().map(write_result).collect()
}

/// Write all results to a single combined JSON file
pub fn write_combined_results(results: &[BenchmarkResult]) -> std::io::Result<PathBuf> {
    write_combined_results_to_dir(results, OUTPUT_DIR)
}

/// Write all results to a single combined JSON file in a specific directory
pub fn write_combined_results_to_dir(
    results: &[BenchmarkResult],
    dir: impl AsRef<Path>,
) -> std::io::Result<PathBuf> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;

    let filename = format!("benchmark_results_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));
    let path = dir.join(&filename);

    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, results)?;
    writer.flush()?;

    Ok(path)
}

/// Read a benchmark result from a JSON file
pub fn read_result(path: impl AsRef<Path>) -> std::io::Result<BenchmarkResult> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Read all benchmark results from the raw output directory
pub fn read_all_results() -> std::io::Result<Vec<BenchmarkResult>> {
    read_all_results_from_dir(RAW_DIR)
}

/// Read all benchmark results from a specific directory
pub fn read_all_results_from_dir(dir: impl AsRef<Path>) -> std::io::Result<Vec<BenchmarkResult>> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            match read_result(&path) {
                Ok(result) => results.push(result),
                Err(e) => {
                    eprintln!("Warning: Failed to read {:?}: {}", path, e);
                }
            }
        }
    }

    // Sort by timestamp, newest first
    results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(results)
}

/// Get the latest result for a specific target
pub fn get_latest_result(target_id: &str) -> std::io::Result<Option<BenchmarkResult>> {
    get_latest_result_from_dir(target_id, RAW_DIR)
}

/// Get the latest result for a specific target from a specific directory
pub fn get_latest_result_from_dir(
    target_id: &str,
    dir: impl AsRef<Path>,
) -> std::io::Result<Option<BenchmarkResult>> {
    let results = read_all_results_from_dir(dir)?;
    Ok(results.into_iter().find(|r| r.target_id == target_id))
}

/// Clean up old benchmark results, keeping only the most recent N results per target
pub fn cleanup_old_results(keep_count: usize) -> std::io::Result<usize> {
    cleanup_old_results_in_dir(keep_count, RAW_DIR)
}

/// Clean up old benchmark results in a specific directory
pub fn cleanup_old_results_in_dir(keep_count: usize, dir: impl AsRef<Path>) -> std::io::Result<usize> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(0);
    }

    let results = read_all_results_from_dir(dir)?;

    // Group by target_id
    let mut by_target: std::collections::HashMap<String, Vec<BenchmarkResult>> =
        std::collections::HashMap::new();
    for result in results {
        by_target
            .entry(result.target_id.clone())
            .or_default()
            .push(result);
    }

    let mut removed = 0;
    for (target_id, mut target_results) in by_target {
        // Sort by timestamp, newest first
        target_results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Remove old results beyond keep_count
        for result in target_results.iter().skip(keep_count) {
            let filename = format!(
                "{}_{}.json",
                target_id.replace(['/', ':', ' '], "_"),
                result.timestamp.format("%Y%m%d_%H%M%S")
            );
            let path = dir.join(&filename);
            if path.exists() {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_and_read_result() {
        let dir = tempdir().unwrap();
        let result = BenchmarkResult::new(
            "test-io",
            serde_json::json!({ "value": 42 }),
        );

        let path = write_result_to_dir(&result, dir.path()).unwrap();
        assert!(path.exists());

        let read_back = read_result(&path).unwrap();
        assert_eq!(read_back.target_id, "test-io");
    }

    #[test]
    fn test_read_all_results() {
        let dir = tempdir().unwrap();

        // Write multiple results
        for i in 0..3 {
            let result = BenchmarkResult::new(
                format!("target-{}", i),
                serde_json::json!({ "index": i }),
            );
            write_result_to_dir(&result, dir.path()).unwrap();
        }

        let results = read_all_results_from_dir(dir.path()).unwrap();
        assert_eq!(results.len(), 3);
    }
}
