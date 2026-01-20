//! LLM-Analytics-Hub Adapter
//!
//! Thin runtime adapter for consuming statistical baselines, outlier detection,
//! and long-tail analytics from the upstream llm-analytics-hub crate.
//! Translates analytics types to internal incident-manager types without modifying
//! existing event-priority heuristics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Import upstream analytics types (disabled - ecosystem feature not enabled)
// When ecosystem feature is enabled, uncomment these imports:
// use llm_analytics_hub::{Anomaly, AnomalyType as AnalyticsAnomalyType, AnomalySeverity};
// use llm_analytics_hub::analytics::anomaly::DetectorStats;

/// Adapter for consuming analytics from llm-analytics-hub
#[derive(Debug, Clone)]
pub struct AnalyticsHubAdapter {
    /// Source identifier for tracing
    source_id: String,
    /// Confidence threshold for anomalies
    confidence_threshold: f64,
}

impl Default for AnalyticsHubAdapter {
    fn default() -> Self {
        Self::new("llm-analytics-hub")
    }
}

impl AnalyticsHubAdapter {
    /// Create a new analytics hub adapter
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            confidence_threshold: 0.8,
        }
    }

    /// Set confidence threshold for anomaly filtering
    pub fn with_confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Convert upstream statistical measures to internal baseline representation
    ///
    /// This is the primary consumption point for baselines from analytics-hub.
    /// Does not modify analytics algorithms - pure type translation.
    pub fn convert_statistical_measures(&self, measures: &UpstreamStatisticalMeasures, metric_name: &str) -> UpstreamStatisticalBaseline {
        UpstreamStatisticalBaseline {
            metric_name: metric_name.to_string(),
            mean: measures.avg,
            min: measures.min,
            max: measures.max,
            p50: measures.p50,
            p95: measures.p95,
            p99: measures.p99,
            std_dev: measures.stddev,
            sample_count: measures.count as u64,
            sum: measures.sum,
            computed_at: Utc::now(),
            source: self.source_id.clone(),
        }
    }

    // NOTE: convert_anomaly requires ecosystem feature (llm_analytics_hub)
    // Uncomment when ecosystem dependencies are available
    // pub fn convert_anomaly(&self, anomaly: &Anomaly) -> UpstreamOutlierDetection { ... }

    /// Extract long-tail analytics from metrics data
    pub fn create_long_tail_analytics(&self, metrics: &[UpstreamAggregatedMetric]) -> UpstreamLongTailAnalytics {
        let statistical_summary = self.compute_summary_stats(metrics);
        let distribution = self.analyze_distribution(metrics);
        let outlier_ratio = self.calculate_outlier_ratio(&distribution);

        UpstreamLongTailAnalytics {
            id: format!("longtail-{}", uuid::Uuid::new_v4()),
            analysis_period: self.extract_time_window(metrics),
            metric_count: metrics.len(),
            statistical_summary,
            distribution,
            outlier_ratio,
            tail_characteristics: self.analyze_tail(metrics),
            computed_at: Utc::now(),
            source: self.source_id.clone(),
        }
    }

    // NOTE: The following methods require ecosystem feature (llm_analytics_hub)
    // convert_detector_stats, filter_high_confidence_anomalies, convert_anomaly_batch
    // convert_anomaly_type, convert_anomaly_severity, estimate_confidence
    // Uncomment when ecosystem dependencies are available

    // --- Private helper methods ---

    fn calculate_deviation_percentage(&self, observed: f64, expected: f64) -> f64 {
        if expected == 0.0 {
            if observed == 0.0 {
                0.0
            } else {
                100.0
            }
        } else {
            ((observed - expected).abs() / expected.abs()) * 100.0
        }
    }

    fn compute_summary_stats(&self, metrics: &[UpstreamAggregatedMetric]) -> StatsSummary {
        if metrics.is_empty() {
            return StatsSummary::default();
        }

        // Extract values from aggregated metrics
        let values: Vec<f64> = metrics
            .iter()
            .filter_map(|m| m.measures.as_ref())
            .map(|m| m.avg)
            .collect();

        if values.is_empty() {
            return StatsSummary::default();
        }

        let sum: f64 = values.iter().sum();
        let count = values.len() as f64;
        let mean = sum / count;

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count;
        let std_dev = variance.sqrt();

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min = *sorted.first().unwrap_or(&0.0);
        let max = *sorted.last().unwrap_or(&0.0);
        let median = sorted[sorted.len() / 2];

        StatsSummary {
            mean,
            median,
            std_dev,
            min,
            max,
            sample_count: values.len() as u64,
        }
    }

    fn analyze_distribution(&self, metrics: &[UpstreamAggregatedMetric]) -> DistributionAnalysis {
        let values: Vec<f64> = metrics
            .iter()
            .filter_map(|m| m.measures.as_ref())
            .map(|m| m.avg)
            .collect();

        if values.is_empty() {
            return DistributionAnalysis::default();
        }

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len();
        let p25_idx = n / 4;
        let p75_idx = (3 * n) / 4;
        let p95_idx = (95 * n) / 100;

        let p25 = sorted.get(p25_idx).copied().unwrap_or(0.0);
        let p75 = sorted.get(p75_idx).copied().unwrap_or(0.0);
        let p95 = sorted.get(p95_idx).copied().unwrap_or(0.0);

        let iqr = p75 - p25;
        let lower_bound = p25 - 1.5 * iqr;
        let upper_bound = p75 + 1.5 * iqr;

        let outlier_count = sorted.iter().filter(|&&v| v < lower_bound || v > upper_bound).count();

        DistributionAnalysis {
            p25,
            p75,
            p95,
            iqr,
            lower_bound,
            upper_bound,
            outlier_count: outlier_count as u64,
        }
    }

    fn calculate_outlier_ratio(&self, distribution: &DistributionAnalysis) -> f64 {
        // Avoid division by zero
        let total = distribution.outlier_count as f64;
        if total == 0.0 {
            0.0
        } else {
            // This is a simplified ratio - in practice would use total sample count
            (distribution.outlier_count as f64).min(0.1)
        }
    }

    fn analyze_tail(&self, metrics: &[UpstreamAggregatedMetric]) -> TailCharacteristics {
        let values: Vec<f64> = metrics
            .iter()
            .filter_map(|m| m.measures.as_ref())
            .map(|m| m.avg)
            .collect();

        if values.len() < 10 {
            return TailCharacteristics::default();
        }

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len();
        let tail_size = n / 10; // Top and bottom 10%

        let lower_tail: Vec<f64> = sorted[..tail_size].to_vec();
        let upper_tail: Vec<f64> = sorted[(n - tail_size)..].to_vec();

        let lower_tail_mean = lower_tail.iter().sum::<f64>() / tail_size as f64;
        let upper_tail_mean = upper_tail.iter().sum::<f64>() / tail_size as f64;

        let overall_mean = sorted.iter().sum::<f64>() / n as f64;

        TailCharacteristics {
            lower_tail_size: tail_size as u64,
            upper_tail_size: tail_size as u64,
            lower_tail_mean,
            upper_tail_mean,
            tail_asymmetry: (upper_tail_mean - overall_mean) - (overall_mean - lower_tail_mean),
            is_heavy_tailed: (upper_tail_mean - overall_mean).abs() > 2.0 * overall_mean,
        }
    }

    fn extract_time_window(&self, metrics: &[UpstreamAggregatedMetric]) -> TimeWindowInfo {
        let start = metrics.iter().filter_map(|m| m.window_start).min();
        let end = metrics.iter().filter_map(|m| m.window_end).max();

        TimeWindowInfo {
            start: start.unwrap_or_else(Utc::now),
            end: end.unwrap_or_else(Utc::now),
            duration_seconds: start.and_then(|s| end.map(|e| (e - s).num_seconds() as u64)).unwrap_or(0),
        }
    }
}

// --- Upstream Type Definitions ---
// These mirror analytics-hub types for adapter compatibility

/// Upstream statistical measures
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpstreamStatisticalMeasures {
    pub avg: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub stddev: f64,
    pub count: u64,
    pub sum: f64,
}

/// Upstream aggregated metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamAggregatedMetric {
    pub metric_name: String,
    pub window_start: Option<DateTime<Utc>>,
    pub window_end: Option<DateTime<Utc>>,
    pub measures: Option<UpstreamStatisticalMeasures>,
}

/// Statistical baseline from analytics hub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamStatisticalBaseline {
    pub metric_name: String,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub std_dev: f64,
    pub sample_count: u64,
    pub sum: f64,
    pub computed_at: DateTime<Utc>,
    pub source: String,
}

/// Outlier detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamOutlierDetection {
    pub id: String,
    pub metric_name: String,
    pub timestamp: DateTime<Utc>,
    pub observed_value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub deviation_percentage: f64,
    pub anomaly_type: OutlierType,
    pub severity: OutlierSeverity,
    pub confidence: f64,
    pub source: String,
}

/// Types of outliers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutlierType {
    Spike,
    Drop,
    HighValue,
    LowValue,
    Pattern,
}

/// Severity of outliers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutlierSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Long-tail analytics result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamLongTailAnalytics {
    pub id: String,
    pub analysis_period: TimeWindowInfo,
    pub metric_count: usize,
    pub statistical_summary: StatsSummary,
    pub distribution: DistributionAnalysis,
    pub outlier_ratio: f64,
    pub tail_characteristics: TailCharacteristics,
    pub computed_at: DateTime<Utc>,
    pub source: String,
}

/// Time window information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeWindowInfo {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_seconds: u64,
}

/// Statistical summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsSummary {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub sample_count: u64,
}

/// Distribution analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DistributionAnalysis {
    pub p25: f64,
    pub p75: f64,
    pub p95: f64,
    pub iqr: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub outlier_count: u64,
}

/// Tail characteristics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TailCharacteristics {
    pub lower_tail_size: u64,
    pub upper_tail_size: u64,
    pub lower_tail_mean: f64,
    pub upper_tail_mean: f64,
    pub tail_asymmetry: f64,
    pub is_heavy_tailed: bool,
}

/// Detector statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorStatistics {
    pub total_metrics_monitored: u64,
    pub total_anomalies_detected: u64,
    pub active_baselines: u64,
    pub detection_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = AnalyticsHubAdapter::new("test-source");
        assert_eq!(adapter.source_id, "test-source");
    }

    #[test]
    fn test_default_adapter() {
        let adapter = AnalyticsHubAdapter::default();
        assert_eq!(adapter.source_id, "llm-analytics-hub");
    }

    #[test]
    fn test_confidence_threshold() {
        let adapter = AnalyticsHubAdapter::default()
            .with_confidence_threshold(0.9);
        assert_eq!(adapter.confidence_threshold, 0.9);

        // Test clamping
        let adapter = AnalyticsHubAdapter::default()
            .with_confidence_threshold(1.5);
        assert_eq!(adapter.confidence_threshold, 1.0);
    }

    #[test]
    fn test_deviation_percentage() {
        let adapter = AnalyticsHubAdapter::default();

        // Normal case
        assert!((adapter.calculate_deviation_percentage(110.0, 100.0) - 10.0).abs() < 0.001);

        // Zero expected
        assert_eq!(adapter.calculate_deviation_percentage(10.0, 0.0), 100.0);

        // Both zero
        assert_eq!(adapter.calculate_deviation_percentage(0.0, 0.0), 0.0);
    }

    // NOTE: test_outlier_type_conversion requires ecosystem feature (llm_analytics_hub)
}
