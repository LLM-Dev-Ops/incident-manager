//! Dashboard data providers for real-time metrics

use crate::analytics::aggregation::MetricsAggregator;
use crate::analytics::error::{AnalyticsError, AnalyticsResult};
use crate::analytics::metrics::{IncidentMetrics, PerformanceMetrics, SLAMetrics};
use crate::models::Incident;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Real-time dashboard metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    /// Current open incidents count
    pub open_incidents: u64,

    /// Critical (P0) incidents count
    pub critical_incidents: u64,

    /// Incidents opened in last hour
    pub last_hour_incidents: u64,

    /// Incidents opened in last 24 hours
    pub last_day_incidents: u64,

    /// Average MTTR (last 24 hours)
    pub mttr_24h: f64,

    /// Average MTTD (last 24 hours)
    pub mttd_24h: f64,

    /// SLA compliance (last 7 days)
    pub sla_compliance_7d: f64,

    /// At-risk incidents (approaching SLA)
    pub at_risk_incidents: u64,

    /// Recent incident rate (incidents per hour, last 24h)
    pub incident_rate: f64,

    /// Top incident sources (last 24 hours)
    pub top_sources: Vec<(String, u64)>,

    /// Severity distribution (last 24 hours)
    pub severity_distribution: HashMap<String, u64>,

    /// Status distribution (current)
    pub status_distribution: HashMap<String, u64>,
}

impl Default for DashboardMetrics {
    fn default() -> Self {
        Self {
            open_incidents: 0,
            critical_incidents: 0,
            last_hour_incidents: 0,
            last_day_incidents: 0,
            mttr_24h: 0.0,
            mttd_24h: 0.0,
            sla_compliance_7d: 100.0,
            at_risk_incidents: 0,
            incident_rate: 0.0,
            top_sources: Vec::new(),
            severity_distribution: HashMap::new(),
            status_distribution: HashMap::new(),
        }
    }
}

/// Complete dashboard data including trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Current metrics
    pub metrics: DashboardMetrics,

    /// Historical metrics for comparison
    pub historical: HistoricalMetrics,

    /// Alerts and notifications
    pub alerts: Vec<DashboardAlert>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Historical metrics for trend comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMetrics {
    /// Incident counts by hour (last 24 hours)
    pub hourly_counts: Vec<HourlyCount>,

    /// Incident counts by day (last 7 days)
    pub daily_counts: Vec<DailyCount>,

    /// MTTR trend (last 7 days)
    pub mttr_trend: Vec<MetricPoint>,

    /// SLA compliance trend (last 7 days)
    pub sla_trend: Vec<MetricPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyCount {
    pub hour: DateTime<Utc>,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCount {
    pub day: DateTime<Utc>,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Dashboard alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub severity: AlertSeverity,
    pub message: String,
    pub metric: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Dashboard data provider
pub struct DashboardProvider {
    sla_targets: HashMap<String, f64>,
}

impl DashboardProvider {
    /// Create a new dashboard provider
    pub fn new(sla_targets: HashMap<String, f64>) -> Self {
        Self { sla_targets }
    }

    /// Create with default SLA targets
    pub fn with_defaults() -> Self {
        let mut sla_targets = HashMap::new();
        sla_targets.insert("P0".to_string(), 900.0); // 15 minutes
        sla_targets.insert("P1".to_string(), 3600.0); // 1 hour
        sla_targets.insert("P2".to_string(), 14400.0); // 4 hours
        sla_targets.insert("P3".to_string(), 86400.0); // 24 hours
        sla_targets.insert("P4".to_string(), 259200.0); // 72 hours

        Self::new(sla_targets)
    }

    /// Generate complete dashboard data
    pub fn generate_dashboard(&self, incidents: &[Incident]) -> AnalyticsResult<DashboardData> {
        let metrics = self.generate_metrics(incidents)?;
        let historical = self.generate_historical(incidents)?;
        let alerts = self.generate_alerts(&metrics, incidents);

        Ok(DashboardData {
            metrics,
            historical,
            alerts,
            updated_at: Utc::now(),
        })
    }

    /// Generate current metrics
    fn generate_metrics(&self, incidents: &[Incident]) -> AnalyticsResult<DashboardMetrics> {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let one_day_ago = now - Duration::days(1);
        let seven_days_ago = now - Duration::days(7);

        // Count open incidents
        let open_incidents = incidents
            .iter()
            .filter(|i| i.state.to_string() == "open")
            .count() as u64;

        // Count critical incidents
        let critical_incidents = incidents
            .iter()
            .filter(|i| {
                i.state.to_string() == "open" && i.severity.to_string() == "P0"
            })
            .count() as u64;

        // Last hour incidents
        let last_hour_incidents = incidents
            .iter()
            .filter(|i| i.created_at > one_hour_ago)
            .count() as u64;

        // Last day incidents
        let last_day_incidents = incidents
            .iter()
            .filter(|i| i.created_at > one_day_ago)
            .count() as u64;

        // Calculate MTTR and MTTD for last 24 hours
        let last_day_incidents_vec: Vec<_> = incidents
            .iter()
            .filter(|i| i.created_at > one_day_ago)
            .cloned()
            .collect();

        let (mttr_24h, mttd_24h) = if !last_day_incidents_vec.is_empty() {
            let perf_metrics =
                MetricsAggregator::aggregate_performance(&last_day_incidents_vec, one_day_ago, now)
                    .unwrap_or_default();
            (perf_metrics.mttr, perf_metrics.mttd)
        } else {
            (0.0, 0.0)
        };

        // Calculate SLA compliance for last 7 days
        let last_week_incidents: Vec<_> = incidents
            .iter()
            .filter(|i| i.created_at > seven_days_ago)
            .cloned()
            .collect();

        let sla_compliance_7d = if !last_week_incidents.is_empty() {
            let sla_metrics = MetricsAggregator::aggregate_sla(
                &last_week_incidents,
                seven_days_ago,
                now,
                &self.sla_targets,
            )
            .unwrap_or_default();
            sla_metrics.overall_compliance
        } else {
            100.0
        };

        // At-risk incidents
        let at_risk_incidents = incidents
            .iter()
            .filter(|i| {
                if i.state.to_string() != "open" {
                    return false;
                }

                let severity_key = i.severity.to_string();
                let sla_target = self.sla_targets.get(&severity_key).unwrap_or(&3600.0);

                let current_duration = now
                    .signed_duration_since(i.created_at)
                    .num_seconds() as f64;

                current_duration > sla_target * 0.8
            })
            .count() as u64;

        // Incident rate (per hour)
        let incident_rate = if last_day_incidents > 0 {
            last_day_incidents as f64 / 24.0
        } else {
            0.0
        };

        // Top sources
        let mut source_counts: HashMap<String, u64> = HashMap::new();
        for incident in incidents.iter().filter(|i| i.created_at > one_day_ago) {
            *source_counts.entry(incident.source.clone()).or_insert(0) += 1;
        }

        let mut top_sources: Vec<_> = source_counts.into_iter().collect();
        top_sources.sort_by(|a, b| b.1.cmp(&a.1));
        top_sources.truncate(5);

        // Severity distribution
        let mut severity_distribution: HashMap<String, u64> = HashMap::new();
        for incident in incidents.iter().filter(|i| i.created_at > one_day_ago) {
            *severity_distribution
                .entry(incident.severity.to_string())
                .or_insert(0) += 1;
        }

        // Status distribution
        let mut status_distribution: HashMap<String, u64> = HashMap::new();
        for incident in incidents {
            *status_distribution
                .entry(incident.state.to_string())
                .or_insert(0) += 1;
        }

        Ok(DashboardMetrics {
            open_incidents,
            critical_incidents,
            last_hour_incidents,
            last_day_incidents,
            mttr_24h,
            mttd_24h,
            sla_compliance_7d,
            at_risk_incidents,
            incident_rate,
            top_sources,
            severity_distribution,
            status_distribution,
        })
    }

    /// Generate historical metrics
    fn generate_historical(&self, incidents: &[Incident]) -> AnalyticsResult<HistoricalMetrics> {
        let now = Utc::now();
        let one_day_ago = now - Duration::days(1);
        let seven_days_ago = now - Duration::days(7);

        // Hourly counts (last 24 hours)
        let mut hourly_counts = Vec::new();
        for i in 0..24 {
            let hour_start = one_day_ago + Duration::hours(i);
            let hour_end = hour_start + Duration::hours(1);

            let count = incidents
                .iter()
                .filter(|inc| inc.created_at >= hour_start && inc.created_at < hour_end)
                .count() as u64;

            hourly_counts.push(HourlyCount {
                hour: hour_start,
                count,
            });
        }

        // Daily counts (last 7 days)
        let mut daily_counts = Vec::new();
        for i in 0..7 {
            let day_start = seven_days_ago + Duration::days(i);
            let day_end = day_start + Duration::days(1);

            let count = incidents
                .iter()
                .filter(|inc| inc.created_at >= day_start && inc.created_at < day_end)
                .count() as u64;

            daily_counts.push(DailyCount {
                day: day_start,
                count,
            });
        }

        // MTTR trend (last 7 days)
        let mut mttr_trend = Vec::new();
        for i in 0..7 {
            let day_start = seven_days_ago + Duration::days(i);
            let day_end = day_start + Duration::days(1);

            let day_incidents: Vec<_> = incidents
                .iter()
                .filter(|inc| inc.created_at >= day_start && inc.created_at < day_end)
                .cloned()
                .collect();

            let mttr = if !day_incidents.is_empty() {
                MetricsAggregator::aggregate_performance(&day_incidents, day_start, day_end)
                    .map(|m| m.mttr)
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            mttr_trend.push(MetricPoint {
                timestamp: day_start,
                value: mttr,
            });
        }

        // SLA trend (last 7 days)
        let mut sla_trend = Vec::new();
        for i in 0..7 {
            let day_start = seven_days_ago + Duration::days(i);
            let day_end = day_start + Duration::days(1);

            let day_incidents: Vec<_> = incidents
                .iter()
                .filter(|inc| inc.created_at >= day_start && inc.created_at < day_end)
                .cloned()
                .collect();

            let compliance = if !day_incidents.is_empty() {
                MetricsAggregator::aggregate_sla(
                    &day_incidents,
                    day_start,
                    day_end,
                    &self.sla_targets,
                )
                .map(|m| m.overall_compliance)
                .unwrap_or(100.0)
            } else {
                100.0
            };

            sla_trend.push(MetricPoint {
                timestamp: day_start,
                value: compliance,
            });
        }

        Ok(HistoricalMetrics {
            hourly_counts,
            daily_counts,
            mttr_trend,
            sla_trend,
        })
    }

    /// Generate dashboard alerts
    fn generate_alerts(
        &self,
        metrics: &DashboardMetrics,
        _incidents: &[Incident],
    ) -> Vec<DashboardAlert> {
        let mut alerts = Vec::new();

        // Critical incidents alert
        if metrics.critical_incidents > 0 {
            alerts.push(DashboardAlert {
                severity: AlertSeverity::Critical,
                message: format!("{} critical (P0) incidents require immediate attention", metrics.critical_incidents),
                metric: "critical_incidents".to_string(),
                created_at: Utc::now(),
            });
        }

        // At-risk incidents alert
        if metrics.at_risk_incidents > 5 {
            alerts.push(DashboardAlert {
                severity: AlertSeverity::Warning,
                message: format!("{} incidents are approaching SLA breach", metrics.at_risk_incidents),
                metric: "at_risk_incidents".to_string(),
                created_at: Utc::now(),
            });
        }

        // High incident rate alert
        if metrics.incident_rate > 10.0 {
            alerts.push(DashboardAlert {
                severity: AlertSeverity::Warning,
                message: format!("High incident rate: {:.1} incidents per hour", metrics.incident_rate),
                metric: "incident_rate".to_string(),
                created_at: Utc::now(),
            });
        }

        // SLA compliance alert
        if metrics.sla_compliance_7d < 95.0 {
            alerts.push(DashboardAlert {
                severity: AlertSeverity::Warning,
                message: format!("SLA compliance below target: {:.1}%", metrics.sla_compliance_7d),
                metric: "sla_compliance".to_string(),
                created_at: Utc::now(),
            });
        }

        // High MTTR alert
        if metrics.mttr_24h > 3600.0 {
            alerts.push(DashboardAlert {
                severity: AlertSeverity::Info,
                message: format!("MTTR is elevated: {:.1} minutes", metrics.mttr_24h / 60.0),
                metric: "mttr".to_string(),
                created_at: Utc::now(),
            });
        }

        alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_provider_creation() {
        let provider = DashboardProvider::with_defaults();
        assert!(provider.sla_targets.contains_key("P0"));
    }

    #[test]
    fn test_dashboard_metrics_default() {
        let metrics = DashboardMetrics::default();
        assert_eq!(metrics.open_incidents, 0);
        assert_eq!(metrics.sla_compliance_7d, 100.0);
    }
}
