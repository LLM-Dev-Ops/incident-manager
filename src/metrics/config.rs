/// Configuration for Prometheus metrics exporter
///
/// This module provides configuration options for controlling metrics collection,
/// export format, and behavior.

use serde::{Deserialize, Serialize};

/// Configuration for metrics collection and export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Endpoint path for metrics export
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    /// Port for metrics server (if separate from main HTTP server)
    pub metrics_port: Option<u16>,

    /// Include runtime metrics (memory, CPU, etc.)
    #[serde(default = "default_include_runtime")]
    pub include_runtime: bool,

    /// Include detailed request metrics
    #[serde(default = "default_include_request_details")]
    pub include_request_details: bool,

    /// Maximum number of unique label combinations
    /// Protects against cardinality explosion
    #[serde(default = "default_max_label_cardinality")]
    pub max_label_cardinality: usize,

    /// Sample rate for high-frequency metrics (0.0 to 1.0)
    /// 1.0 = sample all, 0.1 = sample 10%
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,

    /// Enable histogram metrics (can be expensive)
    #[serde(default = "default_enable_histograms")]
    pub enable_histograms: bool,

    /// Custom metric labels to add to all metrics
    #[serde(default)]
    pub global_labels: Vec<(String, String)>,

    /// Paths to exclude from HTTP metrics
    #[serde(default)]
    pub excluded_paths: Vec<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            endpoint: default_endpoint(),
            metrics_port: None,
            include_runtime: default_include_runtime(),
            include_request_details: default_include_request_details(),
            max_label_cardinality: default_max_label_cardinality(),
            sample_rate: default_sample_rate(),
            enable_histograms: default_enable_histograms(),
            global_labels: Vec::new(),
            excluded_paths: vec!["/health".to_string(), "/metrics".to_string()],
        }
    }
}

impl MetricsConfig {
    /// Create a new metrics configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration with metrics enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Create a configuration with metrics disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Set the metrics endpoint path
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Set a separate metrics port
    pub fn with_metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = Some(port);
        self
    }

    /// Enable runtime metrics
    pub fn with_runtime_metrics(mut self, enabled: bool) -> Self {
        self.include_runtime = enabled;
        self
    }

    /// Enable detailed request metrics
    pub fn with_request_details(mut self, enabled: bool) -> Self {
        self.include_request_details = enabled;
        self
    }

    /// Set maximum label cardinality
    pub fn with_max_cardinality(mut self, max: usize) -> Self {
        self.max_label_cardinality = max;
        self
    }

    /// Set sampling rate
    pub fn with_sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Add a global label to all metrics
    pub fn with_global_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.global_labels.push((key.into(), value.into()));
        self
    }

    /// Exclude a path from HTTP metrics
    pub fn exclude_path(mut self, path: impl Into<String>) -> Self {
        self.excluded_paths.push(path.into());
        self
    }

    /// Check if a path should be excluded from metrics
    pub fn is_path_excluded(&self, path: &str) -> bool {
        self.excluded_paths.iter().any(|excluded| {
            // Support exact match and prefix match
            path == excluded || path.starts_with(&format!("{}/", excluded))
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.sample_rate < 0.0 || self.sample_rate > 1.0 {
            return Err("sample_rate must be between 0.0 and 1.0".to_string());
        }

        if self.max_label_cardinality == 0 {
            return Err("max_label_cardinality must be greater than 0".to_string());
        }

        if !self.endpoint.starts_with('/') {
            return Err("endpoint must start with '/'".to_string());
        }

        Ok(())
    }
}

// Default value functions for serde
fn default_enabled() -> bool {
    true
}

fn default_endpoint() -> String {
    "/metrics".to_string()
}

fn default_include_runtime() -> bool {
    true
}

fn default_include_request_details() -> bool {
    true
}

fn default_max_label_cardinality() -> usize {
    10_000
}

fn default_sample_rate() -> f64 {
    1.0
}

fn default_enable_histograms() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.endpoint, "/metrics");
        assert!(config.include_runtime);
        assert_eq!(config.sample_rate, 1.0);
    }

    #[test]
    fn test_config_builder() {
        let config = MetricsConfig::new()
            .with_endpoint("/custom-metrics")
            .with_metrics_port(9090)
            .with_sample_rate(0.5)
            .with_global_label("env", "production");

        assert_eq!(config.endpoint, "/custom-metrics");
        assert_eq!(config.metrics_port, Some(9090));
        assert_eq!(config.sample_rate, 0.5);
        assert_eq!(config.global_labels.len(), 1);
    }

    #[test]
    fn test_path_exclusion() {
        let config = MetricsConfig::default();
        assert!(config.is_path_excluded("/health"));
        assert!(config.is_path_excluded("/metrics"));
        assert!(!config.is_path_excluded("/api/v1/incidents"));
    }

    #[test]
    fn test_validation() {
        let valid_config = MetricsConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_sample = MetricsConfig {
            sample_rate: 1.5,
            ..Default::default()
        };
        assert!(invalid_sample.validate().is_err());

        let invalid_cardinality = MetricsConfig {
            max_label_cardinality: 0,
            ..Default::default()
        };
        assert!(invalid_cardinality.validate().is_err());

        let invalid_endpoint = MetricsConfig {
            endpoint: "metrics".to_string(), // Missing leading slash
            ..Default::default()
        };
        assert!(invalid_endpoint.validate().is_err());
    }

    #[test]
    fn test_sample_rate_clamping() {
        let config = MetricsConfig::new()
            .with_sample_rate(2.0); // Over max
        assert_eq!(config.sample_rate, 1.0);

        let config = MetricsConfig::new()
            .with_sample_rate(-0.5); // Under min
        assert_eq!(config.sample_rate, 0.0);
    }
}
