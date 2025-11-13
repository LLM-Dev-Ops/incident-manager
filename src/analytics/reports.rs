//! Report generation and types

use crate::analytics::metrics::{
    IncidentMetrics, PerformanceMetrics, SLAMetrics, TeamMetrics, TrendMetrics,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of report to generate
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReportType {
    Summary,
    SLA,
    Trend,
    TeamPerformance,
    IncidentAnalysis,
}

/// Filter criteria for reports
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportFilter {
    pub severities: Option<Vec<String>>,
    pub incident_types: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
    pub statuses: Option<Vec<String>>,
    pub teams: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

impl ReportFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_severities(mut self, severities: Vec<String>) -> Self {
        self.severities = Some(severities);
        self
    }

    pub fn with_incident_types(mut self, types: Vec<String>) -> Self {
        self.incident_types = Some(types);
        self
    }

    pub fn with_sources(mut self, sources: Vec<String>) -> Self {
        self.sources = Some(sources);
        self
    }

    pub fn with_statuses(mut self, statuses: Vec<String>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    pub fn with_teams(mut self, teams: Vec<String>) -> Self {
        self.teams = Some(teams);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }
}

/// Request for generating a report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub report_type: ReportType,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub filters: ReportFilter,
}

impl ReportRequest {
    pub fn new(
        report_type: ReportType,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Self {
        Self {
            report_type,
            start_date,
            end_date,
            filters: ReportFilter::default(),
        }
    }

    pub fn with_filters(mut self, filters: ReportFilter) -> Self {
        self.filters = filters;
        self
    }
}

/// Base report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub title: String,
    pub report_type: ReportType,
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary: String,
    pub data: serde_json::Value,
}

impl Report {
    pub fn new(
        title: String,
        report_type: ReportType,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            report_type,
            generated_at: Utc::now(),
            period_start,
            period_end,
            summary: String::new(),
            data: serde_json::Value::Null,
        }
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

/// Summary report with high-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryReport {
    pub incident_metrics: IncidentMetrics,
    pub performance_metrics: PerformanceMetrics,
    pub key_findings: Vec<String>,
    pub recommendations: Vec<String>,
}

impl SummaryReport {
    pub fn new(
        incident_metrics: IncidentMetrics,
        performance_metrics: PerformanceMetrics,
    ) -> Self {
        Self {
            incident_metrics,
            performance_metrics,
            key_findings: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn with_key_findings(mut self, findings: Vec<String>) -> Self {
        self.key_findings = findings;
        self
    }

    pub fn with_recommendations(mut self, recommendations: Vec<String>) -> Self {
        self.recommendations = recommendations;
        self
    }

    /// Generate key findings based on metrics
    pub fn generate_findings(&mut self) {
        let mut findings = Vec::new();

        // Analyze incident volume
        if self.incident_metrics.total_incidents > 100 {
            findings.push(format!(
                "High incident volume: {} incidents in the period",
                self.incident_metrics.total_incidents
            ));
        }

        // Analyze severity distribution
        if let Some(critical_count) = self.incident_metrics.by_severity.get("P0") {
            if *critical_count > 10 {
                findings.push(format!(
                    "Critical incidents require attention: {} P0 incidents",
                    critical_count
                ));
            }
        }

        // Analyze performance
        if self.performance_metrics.mttr > 3600.0 {
            findings.push(format!(
                "Mean time to respond is high: {:.1} minutes",
                self.performance_metrics.mttr / 60.0
            ));
        }

        if self.performance_metrics.mttrs > 7200.0 {
            findings.push(format!(
                "Mean time to resolve is high: {:.1} hours",
                self.performance_metrics.mttrs / 3600.0
            ));
        }

        self.key_findings = findings;
    }

    /// Generate recommendations based on findings
    pub fn generate_recommendations(&mut self) {
        let mut recommendations = Vec::new();

        // Based on MTTR
        if self.performance_metrics.mttr > 3600.0 {
            recommendations.push(
                "Consider implementing automated response workflows to reduce MTTR".to_string(),
            );
        }

        // Based on incident volume
        if self.incident_metrics.total_incidents > 100
            && self.incident_metrics.avg_incidents_per_day > 10.0
        {
            recommendations.push(
                "High incident rate detected - review monitoring thresholds to reduce noise"
                    .to_string(),
            );
        }

        // Based on escalation rate
        if self.performance_metrics.escalation_rate > 20.0 {
            recommendations.push(
                "High escalation rate - consider additional training or runbook improvements"
                    .to_string(),
            );
        }

        self.recommendations = recommendations;
    }
}

/// SLA compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAReport {
    pub sla_metrics: SLAMetrics,
    pub compliance_trend: Vec<ComplianceDataPoint>,
    pub breach_details: Vec<BreachDetail>,
    pub at_risk_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceDataPoint {
    pub timestamp: DateTime<Utc>,
    pub compliance_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreachDetail {
    pub incident_id: String,
    pub severity: String,
    pub breach_duration: f64,
    pub occurred_at: DateTime<Utc>,
}

impl SLAReport {
    pub fn new(sla_metrics: SLAMetrics) -> Self {
        Self {
            sla_metrics,
            compliance_trend: Vec::new(),
            breach_details: Vec::new(),
            at_risk_summary: String::new(),
        }
    }

    pub fn with_compliance_trend(mut self, trend: Vec<ComplianceDataPoint>) -> Self {
        self.compliance_trend = trend;
        self
    }

    pub fn with_breach_details(mut self, details: Vec<BreachDetail>) -> Self {
        self.breach_details = details;
        self
    }

    pub fn generate_at_risk_summary(&mut self) {
        let at_risk = self.sla_metrics.at_risk_incidents;
        let near_misses = self.sla_metrics.near_misses;

        self.at_risk_summary = format!(
            "{} incidents currently at risk of SLA breach, {} near-miss incidents in the period",
            at_risk, near_misses
        );
    }
}

/// Trend analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendReport {
    pub incident_trend: TrendMetrics,
    pub performance_trend: TrendMetrics,
    pub forecasts: HashMap<String, Forecast>,
    pub anomalies: Vec<Anomaly>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    pub metric_name: String,
    pub next_period_value: f64,
    pub confidence_interval: (f64, f64),
    pub trend_direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub timestamp: DateTime<Utc>,
    pub metric_name: String,
    pub expected_value: f64,
    pub actual_value: f64,
    pub deviation: f64,
}

impl TrendReport {
    pub fn new(incident_trend: TrendMetrics, performance_trend: TrendMetrics) -> Self {
        Self {
            incident_trend,
            performance_trend,
            forecasts: HashMap::new(),
            anomalies: Vec::new(),
        }
    }

    pub fn add_forecast(&mut self, metric: String, forecast: Forecast) {
        self.forecasts.insert(metric, forecast);
    }

    pub fn add_anomaly(&mut self, anomaly: Anomaly) {
        self.anomalies.push(anomaly);
    }
}

/// Team performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamPerformanceReport {
    pub teams: Vec<TeamMetrics>,
    pub rankings: TeamRankings,
    pub comparisons: Vec<TeamComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRankings {
    pub by_resolution_time: Vec<(String, f64)>,
    pub by_sla_compliance: Vec<(String, f64)>,
    pub by_volume_handled: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamComparison {
    pub team_id: String,
    pub team_name: String,
    pub vs_average: HashMap<String, f64>,
}

impl TeamPerformanceReport {
    pub fn new(teams: Vec<TeamMetrics>) -> Self {
        Self {
            teams,
            rankings: TeamRankings {
                by_resolution_time: Vec::new(),
                by_sla_compliance: Vec::new(),
                by_volume_handled: Vec::new(),
            },
            comparisons: Vec::new(),
        }
    }

    pub fn generate_rankings(&mut self) {
        // Rank by resolution time (lower is better)
        let mut by_resolution: Vec<_> = self
            .teams
            .iter()
            .map(|t| (t.team_name.clone(), t.avg_resolution_time))
            .collect();
        by_resolution.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        self.rankings.by_resolution_time = by_resolution;

        // Rank by SLA compliance (higher is better)
        let mut by_sla: Vec<_> = self
            .teams
            .iter()
            .map(|t| (t.team_name.clone(), t.sla_compliance))
            .collect();
        by_sla.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        self.rankings.by_sla_compliance = by_sla;

        // Rank by volume handled (higher shows more activity)
        let mut by_volume: Vec<_> = self
            .teams
            .iter()
            .map(|t| (t.team_name.clone(), t.incidents_handled))
            .collect();
        by_volume.sort_by(|a, b| b.1.cmp(&a.1));
        self.rankings.by_volume_handled = by_volume;
    }

    pub fn generate_comparisons(&mut self) {
        if self.teams.is_empty() {
            return;
        }

        // Calculate averages
        let avg_resolution_time: f64 =
            self.teams.iter().map(|t| t.avg_resolution_time).sum::<f64>() / self.teams.len() as f64;
        let avg_sla_compliance: f64 =
            self.teams.iter().map(|t| t.sla_compliance).sum::<f64>() / self.teams.len() as f64;
        let avg_first_response: f64 = self
            .teams
            .iter()
            .map(|t| t.avg_first_response_time)
            .sum::<f64>()
            / self.teams.len() as f64;

        let mut comparisons = Vec::new();

        for team in &self.teams {
            let mut vs_average = HashMap::new();

            vs_average.insert(
                "resolution_time".to_string(),
                ((team.avg_resolution_time - avg_resolution_time) / avg_resolution_time) * 100.0,
            );
            vs_average.insert(
                "sla_compliance".to_string(),
                ((team.sla_compliance - avg_sla_compliance) / avg_sla_compliance) * 100.0,
            );
            vs_average.insert(
                "first_response_time".to_string(),
                ((team.avg_first_response_time - avg_first_response) / avg_first_response) * 100.0,
            );

            comparisons.push(TeamComparison {
                team_id: team.team_id.clone(),
                team_name: team.team_name.clone(),
                vs_average,
            });
        }

        self.comparisons = comparisons;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_creation() {
        let report = Report::new(
            "Test Report".to_string(),
            ReportType::Summary,
            Utc::now(),
            Utc::now(),
        );

        assert_eq!(report.title, "Test Report");
        assert_eq!(report.report_type, ReportType::Summary);
    }

    #[test]
    fn test_report_filter() {
        let filter = ReportFilter::new()
            .with_severities(vec!["P0".to_string()])
            .with_sources(vec!["monitoring".to_string()]);

        assert_eq!(filter.severities.unwrap().len(), 1);
        assert_eq!(filter.sources.unwrap().len(), 1);
    }
}
