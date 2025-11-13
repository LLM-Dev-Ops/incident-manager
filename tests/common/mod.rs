//! Common test utilities for metrics testing
//!
//! This module provides helper functions and utilities for testing
//! Prometheus metrics implementation.

use std::collections::HashMap;

/// Helper function to parse Prometheus exposition format
/// Returns a map of metric lines for easy assertion
pub fn parse_prometheus_output(output: &str) -> HashMap<String, Vec<String>> {
    let mut metrics = HashMap::new();
    let mut current_metric = String::new();

    for line in output.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // HELP and TYPE comments
        if line.starts_with("# HELP") || line.starts_with("# TYPE") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                current_metric = parts[2].to_string();
                metrics.entry(current_metric.clone()).or_insert_with(Vec::new).push(line.to_string());
            }
        }
        // Metric values
        else if !line.starts_with('#') && !current_metric.is_empty() {
            metrics.entry(current_metric.clone()).or_insert_with(Vec::new).push(line.to_string());
        }
    }

    metrics
}

/// Helper to validate metric name follows Prometheus conventions
pub fn is_valid_metric_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // First character must be [a-zA-Z_:]
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' && first_char != ':' {
        return false;
    }

    // Remaining characters must be [a-zA-Z0-9_:]
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != ':' {
            return false;
        }
    }

    // Should not start with __
    if name.starts_with("__") {
        return false;
    }

    true
}

/// Helper to validate label name follows Prometheus conventions
pub fn is_valid_label_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Reserved names
    if name.starts_with("__") {
        return false;
    }

    // First character must be [a-zA-Z_]
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }

    // Remaining characters must be [a-zA-Z0-9_]
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return false;
        }
    }

    true
}

/// Helper to validate counter name ends with _total
pub fn is_valid_counter_name(name: &str) -> bool {
    is_valid_metric_name(name) && name.ends_with("_total")
}

/// Extract metric value from a Prometheus output line
/// Example: `metric_name{label1="value1"} 42.5` -> Some(42.5)
pub fn extract_metric_value(line: &str) -> Option<f64> {
    // Split on whitespace, last part should be the value
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // Last part is the value, possibly followed by timestamp
    parts.last()?.parse::<f64>().ok()
}

/// Extract labels from a Prometheus metric line
/// Example: `metric{a="1",b="2"}` -> HashMap{"a" -> "1", "b" -> "2"}
pub fn extract_labels(line: &str) -> HashMap<String, String> {
    let mut labels = HashMap::new();

    // Find the part between { and }
    if let Some(start) = line.find('{') {
        if let Some(end) = line.find('}') {
            let labels_str = &line[start + 1..end];

            // Split by comma and parse key="value" pairs
            for pair in labels_str.split(',') {
                let parts: Vec<&str> = pair.split('=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim().trim_matches('"');
                    labels.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    labels
}

/// Validate that a Prometheus exposition format output is well-formed
pub fn validate_exposition_format(output: &str) -> Result<(), String> {
    let lines: Vec<&str> = output.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // HELP line should be followed by TYPE line for the same metric
        if line.starts_with("# HELP") {
            let help_parts: Vec<&str> = line.split_whitespace().collect();
            if help_parts.len() < 4 {
                return Err(format!("Line {}: Invalid HELP format", i + 1));
            }

            let metric_name = help_parts[2];

            // Next non-empty line should be TYPE for same metric
            if i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                if !next_line.starts_with(&format!("# TYPE {}", metric_name)) {
                    return Err(format!(
                        "Line {}: HELP not followed by TYPE for metric {}",
                        i + 1,
                        metric_name
                    ));
                }
            }
        }

        // TYPE line should have valid metric type
        if line.starts_with("# TYPE") {
            let type_parts: Vec<&str> = line.split_whitespace().collect();
            if type_parts.len() < 4 {
                return Err(format!("Line {}: Invalid TYPE format", i + 1));
            }

            let metric_type = type_parts[3];
            let valid_types = ["counter", "gauge", "histogram", "summary", "untyped"];
            if !valid_types.contains(&metric_type) {
                return Err(format!(
                    "Line {}: Invalid metric type '{}'",
                    i + 1,
                    metric_type
                ));
            }
        }
    }

    Ok(())
}

/// Check if a metric exists in Prometheus output
pub fn metric_exists(output: &str, metric_name: &str) -> bool {
    for line in output.lines() {
        if line.starts_with(&format!("# HELP {}", metric_name))
            || line.starts_with(&format!("# TYPE {}", metric_name))
            || line.starts_with(metric_name)
        {
            return true;
        }
    }
    false
}

/// Count total number of metrics in Prometheus output
pub fn count_metrics(output: &str) -> usize {
    let mut count = 0;
    for line in output.lines() {
        if line.starts_with("# TYPE") {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_metric_names() {
        assert!(is_valid_metric_name("http_requests_total"));
        assert!(is_valid_metric_name("process_cpu_seconds_total"));
        assert!(is_valid_metric_name("node_memory_bytes"));
        assert!(is_valid_metric_name("my_metric_123"));

        assert!(!is_valid_metric_name("123_invalid"));
        assert!(!is_valid_metric_name("invalid-name"));
        assert!(!is_valid_metric_name("invalid.name"));
        assert!(!is_valid_metric_name("__reserved"));
        assert!(!is_valid_metric_name(""));
    }

    #[test]
    fn test_valid_label_names() {
        assert!(is_valid_label_name("method"));
        assert!(is_valid_label_name("status_code"));
        assert!(is_valid_label_name("endpoint"));
        assert!(is_valid_label_name("_internal"));

        assert!(!is_valid_label_name("123_invalid"));
        assert!(!is_valid_label_name("invalid-name"));
        assert!(!is_valid_label_name("__reserved"));
        assert!(!is_valid_label_name(""));
    }

    #[test]
    fn test_valid_counter_names() {
        assert!(is_valid_counter_name("http_requests_total"));
        assert!(is_valid_counter_name("errors_total"));

        assert!(!is_valid_counter_name("http_requests"));
        assert!(!is_valid_counter_name("process_cpu_seconds"));
    }

    #[test]
    fn test_extract_metric_value() {
        assert_eq!(extract_metric_value("metric_name 42.5"), Some(42.5));
        assert_eq!(
            extract_metric_value("metric_name{label=\"value\"} 123"),
            Some(123.0)
        );
        assert_eq!(
            extract_metric_value("metric_name{label=\"value\"} 3.14159 1234567890"),
            Some(1234567890.0)
        );
        assert_eq!(extract_metric_value("invalid"), None);
    }

    #[test]
    fn test_extract_labels() {
        let labels = extract_labels("metric{method=\"GET\",status=\"200\"}");
        assert_eq!(labels.get("method"), Some(&"GET".to_string()));
        assert_eq!(labels.get("status"), Some(&"200".to_string()));

        let labels = extract_labels("metric{single=\"value\"}");
        assert_eq!(labels.get("single"), Some(&"value".to_string()));

        let labels = extract_labels("metric_without_labels");
        assert!(labels.is_empty());
    }

    #[test]
    fn test_validate_exposition_format() {
        let valid = "# HELP test_metric Test metric\n# TYPE test_metric counter\ntest_metric 1";
        assert!(validate_exposition_format(valid).is_ok());

        let invalid = "# HELP test_metric Test metric\ntest_metric 1";
        assert!(validate_exposition_format(invalid).is_err());
    }

    #[test]
    fn test_metric_exists() {
        let output = "# HELP test_metric Test metric\n# TYPE test_metric counter\ntest_metric 1";
        assert!(metric_exists(output, "test_metric"));
        assert!(!metric_exists(output, "other_metric"));
    }

    #[test]
    fn test_count_metrics() {
        let output = "# HELP metric1 First\n# TYPE metric1 counter\nmetric1 1\n# HELP metric2 Second\n# TYPE metric2 gauge\nmetric2 2";
        assert_eq!(count_metrics(output), 2);
    }
}
