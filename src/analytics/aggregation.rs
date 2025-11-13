//! Metrics aggregation and time-series data handling

use crate::analytics::error::{AnalyticsError, AnalyticsResult};
use crate::analytics::metrics::{IncidentMetrics, PerformanceMetrics, SLAMetrics, TeamMetrics};
use crate::models::Incident;
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Time period for aggregation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AggregationPeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl AggregationPeriod {
    /// Get the duration of this period
    pub fn duration(&self) -> Duration {
        match self {
            AggregationPeriod::Hourly => Duration::hours(1),
            AggregationPeriod::Daily => Duration::days(1),
            AggregationPeriod::Weekly => Duration::weeks(1),
            AggregationPeriod::Monthly => Duration::days(30),
            AggregationPeriod::Quarterly => Duration::days(90),
            AggregationPeriod::Yearly => Duration::days(365),
        }
    }

    /// Get the number of periods between two dates
    pub fn periods_between(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
        let duration = end.signed_duration_since(start);
        match self {
            AggregationPeriod::Hourly => duration.num_hours(),
            AggregationPeriod::Daily => duration.num_days(),
            AggregationPeriod::Weekly => duration.num_weeks(),
            AggregationPeriod::Monthly => duration.num_days() / 30,
            AggregationPeriod::Quarterly => duration.num_days() / 90,
            AggregationPeriod::Yearly => duration.num_days() / 365,
        }
    }
}

/// A single point in a time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub metadata: HashMap<String, String>,
}

impl TimeSeriesPoint {
    pub fn new(timestamp: DateTime<Utc>, value: f64) -> Self {
        Self {
            timestamp,
            value,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Time series data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesData {
    pub name: String,
    pub period: AggregationPeriod,
    pub points: Vec<TimeSeriesPoint>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

impl TimeSeriesData {
    pub fn new(name: String, period: AggregationPeriod) -> Self {
        Self {
            name,
            period,
            points: Vec::new(),
            start_time: Utc::now(),
            end_time: Utc::now(),
        }
    }

    pub fn add_point(&mut self, point: TimeSeriesPoint) {
        if self.points.is_empty() {
            self.start_time = point.timestamp;
            self.end_time = point.timestamp;
        } else {
            if point.timestamp < self.start_time {
                self.start_time = point.timestamp;
            }
            if point.timestamp > self.end_time {
                self.end_time = point.timestamp;
            }
        }
        self.points.push(point);
    }

    pub fn values(&self) -> Vec<f64> {
        self.points.iter().map(|p| p.value).collect()
    }

    pub fn timestamps(&self) -> Vec<f64> {
        self.points
            .iter()
            .map(|p| p.timestamp.timestamp() as f64)
            .collect()
    }
}

/// Metrics aggregator for incident data
pub struct MetricsAggregator;

impl MetricsAggregator {
    /// Aggregate incident metrics for a time period
    pub fn aggregate_incidents(
        incidents: &[Incident],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AnalyticsResult<IncidentMetrics> {
        if start >= end {
            return Err(AnalyticsError::InvalidDateRange(
                "Start date must be before end date".to_string(),
            ));
        }

        let filtered: Vec<_> = incidents
            .iter()
            .filter(|i| i.created_at >= start && i.created_at <= end)
            .collect();

        let total_incidents = filtered.len() as u64;
        let open_incidents = filtered
            .iter()
            .filter(|i| i.state.to_string() == "open")
            .count() as u64;
        let resolved_incidents = filtered
            .iter()
            .filter(|i| i.state.to_string() == "resolved")
            .count() as u64;

        let mut by_severity = HashMap::new();
        let mut by_type = HashMap::new();
        let mut by_source = HashMap::new();
        let mut by_status = HashMap::new();

        for incident in &filtered {
            *by_severity
                .entry(incident.severity.to_string())
                .or_insert(0u64) += 1;
            *by_type
                .entry(incident.incident_type.to_string())
                .or_insert(0u64) += 1;
            *by_source.entry(incident.source.clone()).or_insert(0u64) += 1;
            *by_status
                .entry(incident.state.to_string())
                .or_insert(0u64) += 1;
        }

        let duration = end.signed_duration_since(start);
        let days = duration.num_days().max(1) as f64;
        let avg_incidents_per_day = total_incidents as f64 / days;

        // Calculate peak hour
        let mut hour_counts: HashMap<u32, u64> = HashMap::new();
        for incident in &filtered {
            *hour_counts.entry(incident.created_at.hour()).or_insert(0) += 1;
        }
        let peak_hour = hour_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hour, _)| hour);

        Ok(IncidentMetrics {
            total_incidents,
            open_incidents,
            resolved_incidents,
            by_severity,
            by_type,
            by_source,
            by_status,
            avg_incidents_per_day,
            peak_hour,
            period_start: start,
            period_end: end,
        })
    }

    /// Aggregate performance metrics
    pub fn aggregate_performance(
        incidents: &[Incident],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AnalyticsResult<PerformanceMetrics> {
        let filtered: Vec<_> = incidents
            .iter()
            .filter(|i| i.created_at >= start && i.created_at <= end)
            .collect();

        if filtered.is_empty() {
            return Err(AnalyticsError::InsufficientData(
                "No incidents in the specified time range".to_string(),
            ));
        }

        // Calculate detection times (for now, use creation to first update as proxy)
        let mut detection_times = Vec::new();
        let mut response_times = Vec::new();
        let mut resolution_times = Vec::new();

        for incident in &filtered {
            // Detection time: assume instant detection for now (would need alert data)
            detection_times.push(0.0);

            // Response time: creation to updated (simplified)
            let response_time = incident
                .updated_at
                .signed_duration_since(incident.created_at)
                .num_seconds() as f64;
            response_times.push(response_time);

            // Resolution time: creation to resolved_at (if resolved)
            if let Some(resolved_at) = incident.resolution.as_ref().map(|r| r.resolved_at) {
                let resolution_time = resolved_at
                    .signed_duration_since(incident.created_at)
                    .num_seconds() as f64;
                resolution_times.push(resolution_time);
            }
        }

        let mttd = detection_times.iter().sum::<f64>() / detection_times.len() as f64;
        let mttr = if !response_times.is_empty() {
            response_times.iter().sum::<f64>() / response_times.len() as f64
        } else {
            0.0
        };
        let mttrs = if !resolution_times.is_empty() {
            resolution_times.iter().sum::<f64>() / resolution_times.len() as f64
        } else {
            0.0
        };

        // Calculate percentiles
        let mut sorted_response = response_times.clone();
        sorted_response.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_response_time = percentile(&sorted_response, 50.0);
        let p90_response_time = percentile(&sorted_response, 90.0);
        let p95_response_time = percentile(&sorted_response, 95.0);
        let p99_response_time = percentile(&sorted_response, 99.0);

        // Resolution time by severity
        let mut resolution_by_severity: HashMap<String, Vec<f64>> = HashMap::new();
        for incident in &filtered {
            if let Some(resolved_at) = incident.resolution.as_ref().map(|r| r.resolved_at) {
                let resolution_time = resolved_at
                    .signed_duration_since(incident.created_at)
                    .num_seconds() as f64;
                resolution_by_severity
                    .entry(incident.severity.to_string())
                    .or_insert_with(Vec::new)
                    .push(resolution_time);
            }
        }

        let resolution_time_by_severity = resolution_by_severity
            .into_iter()
            .map(|(severity, times)| {
                let avg = times.iter().sum::<f64>() / times.len() as f64;
                (severity, avg)
            })
            .collect();

        // Escalation rate (simplified - would need escalation data)
        let escalation_rate = 0.0;

        Ok(PerformanceMetrics {
            mttd,
            mttr,
            mttrs,
            p50_response_time,
            p90_response_time,
            p95_response_time,
            p99_response_time,
            resolution_time_by_severity,
            first_response_time: mttr,
            escalation_rate,
        })
    }

    /// Aggregate SLA metrics
    pub fn aggregate_sla(
        incidents: &[Incident],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        sla_targets: &HashMap<String, f64>,
    ) -> AnalyticsResult<SLAMetrics> {
        let filtered: Vec<_> = incidents
            .iter()
            .filter(|i| i.created_at >= start && i.created_at <= end)
            .collect();

        if filtered.is_empty() {
            return Ok(SLAMetrics::default());
        }

        let mut total_breaches = 0u64;
        let mut breaches_by_severity: HashMap<String, u64> = HashMap::new();
        let mut near_misses = 0u64;
        let mut at_risk_incidents = 0u64;
        let mut breach_times = Vec::new();

        for incident in &filtered {
            let severity_key = incident.severity.to_string();
            let sla_target = sla_targets.get(&severity_key).unwrap_or(&3600.0); // Default 1 hour

            if let Some(resolved_at) = incident.resolution.as_ref().map(|r| r.resolved_at) {
                let resolution_time = resolved_at
                    .signed_duration_since(incident.created_at)
                    .num_seconds() as f64;

                if resolution_time > *sla_target {
                    total_breaches += 1;
                    *breaches_by_severity.entry(severity_key.clone()).or_insert(0) += 1;
                    breach_times.push(resolution_time - sla_target);
                } else if resolution_time > sla_target * 0.9 {
                    near_misses += 1;
                }
            } else {
                // Open incident - check if at risk
                let current_duration = Utc::now()
                    .signed_duration_since(incident.created_at)
                    .num_seconds() as f64;

                if current_duration > sla_target * 0.8 {
                    at_risk_incidents += 1;
                }
            }
        }

        let total_incidents = filtered.len() as u64;
        let compliant_incidents = total_incidents - total_breaches;

        let overall_compliance = if total_incidents > 0 {
            (compliant_incidents as f64 / total_incidents as f64) * 100.0
        } else {
            100.0
        };

        let avg_breach_time = if !breach_times.is_empty() {
            breach_times.iter().sum::<f64>() / breach_times.len() as f64
        } else {
            0.0
        };

        Ok(SLAMetrics {
            overall_compliance,
            response_compliance: overall_compliance, // Simplified
            resolution_compliance: overall_compliance,
            total_breaches,
            breaches_by_severity,
            avg_breach_time,
            near_misses,
            at_risk_incidents,
        })
    }

    /// Create time series data from incidents
    pub fn create_time_series(
        incidents: &[Incident],
        period: AggregationPeriod,
        metric_name: &str,
    ) -> AnalyticsResult<TimeSeriesData> {
        if incidents.is_empty() {
            return Err(AnalyticsError::InsufficientData(
                "No incidents to create time series".to_string(),
            ));
        }

        let start = incidents.iter().map(|i| i.created_at).min().unwrap();
        let end = incidents.iter().map(|i| i.created_at).max().unwrap();

        let mut time_series = TimeSeriesData::new(metric_name.to_string(), period);

        let num_periods = period.periods_between(start, end).max(1);

        for i in 0..num_periods {
            let period_start = start + period.duration() * i as i32;
            let period_end = period_start + period.duration();

            let count = incidents
                .iter()
                .filter(|inc| inc.created_at >= period_start && inc.created_at < period_end)
                .count() as f64;

            time_series.add_point(TimeSeriesPoint::new(period_start, count));
        }

        Ok(time_series)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregation_period_duration() {
        assert_eq!(AggregationPeriod::Hourly.duration(), Duration::hours(1));
        assert_eq!(AggregationPeriod::Daily.duration(), Duration::days(1));
    }

    #[test]
    fn test_time_series_point() {
        let point = TimeSeriesPoint::new(Utc::now(), 42.0)
            .with_metadata("key".to_string(), "value".to_string());

        assert_eq!(point.value, 42.0);
        assert_eq!(point.metadata.get("key").unwrap(), "value");
    }
}
