//! Statistical analysis functions for analytics

use crate::analytics::error::{AnalyticsError, AnalyticsResult};
use std::collections::HashMap;

/// Statistical percentiles
#[derive(Debug, Clone)]
pub struct Percentiles {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

impl Percentiles {
    /// Calculate percentiles from a dataset
    pub fn from_data(mut data: Vec<f64>) -> Self {
        if data.is_empty() {
            return Self {
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99: 0.0,
            };
        }

        data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Self {
            p50: percentile(&data, 50.0),
            p90: percentile(&data, 90.0),
            p95: percentile(&data, 95.0),
            p99: percentile(&data, 99.0),
        }
    }
}

/// Distribution statistics
#[derive(Debug, Clone)]
pub struct Distribution {
    pub mean: f64,
    pub median: f64,
    pub mode: Option<f64>,
    pub std_dev: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

impl Distribution {
    /// Calculate distribution from a dataset
    pub fn from_data(data: &[f64]) -> AnalyticsResult<Self> {
        if data.is_empty() {
            return Err(AnalyticsError::InsufficientData(
                "Cannot calculate distribution from empty dataset".to_string(),
            ));
        }

        let count = data.len();
        let mean = data.iter().sum::<f64>() / count as f64;

        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        } else {
            sorted[count / 2]
        };

        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        let mode = calculate_mode(data);

        Ok(Self {
            mean,
            median,
            mode,
            std_dev,
            variance,
            min: sorted[0],
            max: sorted[count - 1],
            count,
        })
    }
}

/// Statistical analysis results
#[derive(Debug, Clone)]
pub struct Statistics {
    pub distribution: Distribution,
    pub percentiles: Percentiles,
}

impl Statistics {
    /// Analyze a dataset
    pub fn analyze(data: &[f64]) -> AnalyticsResult<Self> {
        Ok(Self {
            distribution: Distribution::from_data(data)?,
            percentiles: Percentiles::from_data(data.to_vec()),
        })
    }
}

/// Trend analysis results
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    /// Linear regression slope
    pub slope: f64,

    /// Linear regression intercept
    pub intercept: f64,

    /// R-squared (coefficient of determination)
    pub r_squared: f64,

    /// Correlation coefficient
    pub correlation: f64,

    /// Trend is statistically significant (p < 0.05)
    pub is_significant: bool,

    /// P-value
    pub p_value: f64,
}

impl TrendAnalysis {
    /// Perform trend analysis on time series data
    pub fn analyze(timestamps: &[f64], values: &[f64]) -> AnalyticsResult<Self> {
        if timestamps.len() != values.len() {
            return Err(AnalyticsError::CalculationError(
                "Timestamps and values must have the same length".to_string(),
            ));
        }

        if timestamps.len() < 2 {
            return Err(AnalyticsError::InsufficientData(
                "Need at least 2 data points for trend analysis".to_string(),
            ));
        }

        let n = timestamps.len() as f64;

        // Calculate means
        let mean_x = timestamps.iter().sum::<f64>() / n;
        let mean_y = values.iter().sum::<f64>() / n;

        // Calculate slope and intercept using linear regression
        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for i in 0..timestamps.len() {
            let x_diff = timestamps[i] - mean_x;
            let y_diff = values[i] - mean_y;
            numerator += x_diff * y_diff;
            denominator += x_diff * x_diff;
        }

        let slope = if denominator != 0.0 {
            numerator / denominator
        } else {
            0.0
        };

        let intercept = mean_y - slope * mean_x;

        // Calculate R-squared
        let mut ss_tot = 0.0;
        let mut ss_res = 0.0;

        for i in 0..timestamps.len() {
            let y_pred = slope * timestamps[i] + intercept;
            ss_tot += (values[i] - mean_y).powi(2);
            ss_res += (values[i] - y_pred).powi(2);
        }

        let r_squared = if ss_tot != 0.0 {
            1.0 - (ss_res / ss_tot)
        } else {
            0.0
        };

        // Calculate correlation coefficient
        let correlation = r_squared.sqrt() * slope.signum();

        // Simple significance test (approximation)
        // For a more accurate test, we'd need to calculate t-statistic
        let p_value = if n > 2.0 {
            let t_stat = correlation * ((n - 2.0) / (1.0 - r_squared)).sqrt();
            // Simplified p-value estimation
            2.0 * (1.0 - normal_cdf(t_stat.abs()))
        } else {
            1.0
        };

        let is_significant = p_value < 0.05;

        Ok(Self {
            slope,
            intercept,
            r_squared,
            correlation,
            is_significant,
            p_value,
        })
    }

    /// Forecast value at a given timestamp
    pub fn forecast(&self, timestamp: f64) -> f64 {
        self.slope * timestamp + self.intercept
    }
}

/// Statistical analysis provider
pub struct StatisticalAnalysis;

impl StatisticalAnalysis {
    /// Calculate percentile from sorted data
    pub fn percentile(sorted_data: &[f64], percentile_value: f64) -> f64 {
        percentile(sorted_data, percentile_value)
    }

    /// Calculate moving average
    pub fn moving_average(data: &[f64], window_size: usize) -> Vec<f64> {
        if data.len() < window_size {
            return vec![];
        }

        let mut result = Vec::with_capacity(data.len() - window_size + 1);

        for i in 0..=(data.len() - window_size) {
            let window = &data[i..i + window_size];
            let avg = window.iter().sum::<f64>() / window_size as f64;
            result.push(avg);
        }

        result
    }

    /// Detect outliers using IQR method
    pub fn detect_outliers(data: &[f64]) -> Vec<usize> {
        if data.len() < 4 {
            return vec![];
        }

        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let q1 = percentile(&sorted, 25.0);
        let q3 = percentile(&sorted, 75.0);
        let iqr = q3 - q1;

        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        data.iter()
            .enumerate()
            .filter(|(_, &value)| value < lower_bound || value > upper_bound)
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Calculate correlation between two datasets
    pub fn correlation(x: &[f64], y: &[f64]) -> AnalyticsResult<f64> {
        if x.len() != y.len() {
            return Err(AnalyticsError::CalculationError(
                "Datasets must have the same length".to_string(),
            ));
        }

        if x.is_empty() {
            return Err(AnalyticsError::InsufficientData(
                "Cannot calculate correlation from empty datasets".to_string(),
            ));
        }

        let n = x.len() as f64;
        let mean_x = x.iter().sum::<f64>() / n;
        let mean_y = y.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut sum_sq_x = 0.0;
        let mut sum_sq_y = 0.0;

        for i in 0..x.len() {
            let x_diff = x[i] - mean_x;
            let y_diff = y[i] - mean_y;
            numerator += x_diff * y_diff;
            sum_sq_x += x_diff * x_diff;
            sum_sq_y += y_diff * y_diff;
        }

        let denominator = (sum_sq_x * sum_sq_y).sqrt();

        Ok(if denominator != 0.0 {
            numerator / denominator
        } else {
            0.0
        })
    }
}

// Helper functions

fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }

    let index = (percentile / 100.0) * (sorted_data.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_data[lower]
    } else {
        let weight = index - lower as f64;
        sorted_data[lower] * (1.0 - weight) + sorted_data[upper] * weight
    }
}

fn calculate_mode(data: &[f64]) -> Option<f64> {
    let mut frequency: HashMap<i64, usize> = HashMap::new();

    // Round to nearest integer for frequency counting
    for &value in data {
        let rounded = (value * 100.0).round() as i64;
        *frequency.entry(rounded).or_insert(0) += 1;
    }

    frequency
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .filter(|(_, count)| *count > 1)
        .map(|(value, _)| value as f64 / 100.0)
}

fn normal_cdf(x: f64) -> f64 {
    // Approximation of the normal cumulative distribution function
    // Using the error function approximation
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989423 * (-x * x / 2.0).exp();
    let prob = d
        * t
        * (0.3193815
            + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));

    if x >= 0.0 {
        1.0 - prob
    } else {
        prob
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentiles() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let percentiles = Percentiles::from_data(data);

        assert!((percentiles.p50 - 5.5).abs() < 0.1);
        assert!((percentiles.p90 - 9.5).abs() < 0.1);
    }

    #[test]
    fn test_distribution() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let dist = Distribution::from_data(&data).unwrap();

        assert_eq!(dist.mean, 3.0);
        assert_eq!(dist.median, 3.0);
        assert_eq!(dist.min, 1.0);
        assert_eq!(dist.max, 5.0);
    }

    #[test]
    fn test_trend_analysis() {
        let timestamps = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let values = vec![2.0, 4.0, 6.0, 8.0, 10.0];

        let trend = TrendAnalysis::analyze(&timestamps, &values).unwrap();

        assert!((trend.slope - 2.0).abs() < 0.01);
        assert!(trend.r_squared > 0.99);
    }
}
