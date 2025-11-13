//! Main analytics engine for report generation

use crate::analytics::aggregation::{AggregationPeriod, MetricsAggregator};
use crate::analytics::error::{AnalyticsError, AnalyticsResult};
use crate::analytics::metrics::TrendMetrics;
use crate::analytics::reports::*;
use crate::analytics::statistics::TrendAnalysis;
use crate::models::Incident;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the analytics engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Default SLA targets by severity (in seconds)
    pub sla_targets: HashMap<String, f64>,

    /// Enable trend analysis
    pub enable_trends: bool,

    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,

    /// Minimum data points for statistical analysis
    pub min_data_points: usize,

    /// Cache TTL for reports (seconds)
    pub cache_ttl: u64,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        let mut sla_targets = HashMap::new();
        sla_targets.insert("P0".to_string(), 900.0); // 15 minutes
        sla_targets.insert("P1".to_string(), 3600.0); // 1 hour
        sla_targets.insert("P2".to_string(), 14400.0); // 4 hours
        sla_targets.insert("P3".to_string(), 86400.0); // 24 hours
        sla_targets.insert("P4".to_string(), 259200.0); // 72 hours

        Self {
            sla_targets,
            enable_trends: true,
            enable_anomaly_detection: true,
            min_data_points: 5,
            cache_ttl: 300, // 5 minutes
        }
    }
}

/// Main analytics engine
pub struct AnalyticsEngine {
    config: AnalyticsConfig,
    incident_cache: Arc<RwLock<Vec<Incident>>>,
    report_cache: Arc<RwLock<HashMap<String, (Report, DateTime<Utc>)>>>,
}

impl AnalyticsEngine {
    /// Create a new analytics engine
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            config,
            incident_cache: Arc::new(RwLock::new(Vec::new())),
            report_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(AnalyticsConfig::default())
    }

    /// Update incident cache
    pub async fn update_incident_cache(&self, incidents: Vec<Incident>) {
        let mut cache = self.incident_cache.write().await;
        *cache = incidents;
    }

    /// Generate a report based on the request
    pub async fn generate_report(&self, request: &ReportRequest) -> AnalyticsResult<Report> {
        // Check cache first
        let cache_key = format!(
            "{:?}-{}-{}",
            request.report_type, request.start_date, request.end_date
        );

        {
            let cache = self.report_cache.read().await;
            if let Some((report, cached_at)) = cache.get(&cache_key) {
                let age = Utc::now()
                    .signed_duration_since(*cached_at)
                    .num_seconds() as u64;
                if age < self.config.cache_ttl {
                    return Ok(report.clone());
                }
            }
        }

        // Get incidents from cache
        let incidents = self.incident_cache.read().await;
        let filtered_incidents = self.filter_incidents(&incidents, request);

        // Generate report based on type
        let report = match request.report_type {
            ReportType::Summary => self.generate_summary_report(&filtered_incidents, request).await?,
            ReportType::SLA => self.generate_sla_report(&filtered_incidents, request).await?,
            ReportType::Trend => self.generate_trend_report(&filtered_incidents, request).await?,
            ReportType::TeamPerformance => {
                self.generate_team_performance_report(&filtered_incidents, request)
                    .await?
            }
            ReportType::IncidentAnalysis => {
                self.generate_incident_analysis_report(&filtered_incidents, request)
                    .await?
            }
        };

        // Update cache
        {
            let mut cache = self.report_cache.write().await;
            cache.insert(cache_key, (report.clone(), Utc::now()));
        }

        Ok(report)
    }

    /// Filter incidents based on request criteria
    fn filter_incidents(&self, incidents: &[Incident], request: &ReportRequest) -> Vec<Incident> {
        incidents
            .iter()
            .filter(|i| {
                // Time range filter
                if i.created_at < request.start_date || i.created_at > request.end_date {
                    return false;
                }

                // Severity filter
                if let Some(ref severities) = request.filters.severities {
                    if !severities.contains(&i.severity.to_string()) {
                        return false;
                    }
                }

                // Type filter
                if let Some(ref types) = request.filters.incident_types {
                    if !types.contains(&i.incident_type.to_string()) {
                        return false;
                    }
                }

                // Source filter
                if let Some(ref sources) = request.filters.sources {
                    if !sources.contains(&i.source) {
                        return false;
                    }
                }

                // Status filter
                if let Some(ref statuses) = request.filters.statuses {
                    if !statuses.contains(&i.state.to_string()) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Generate summary report
    async fn generate_summary_report(
        &self,
        incidents: &[Incident],
        request: &ReportRequest,
    ) -> AnalyticsResult<Report> {
        let incident_metrics = MetricsAggregator::aggregate_incidents(
            incidents,
            request.start_date,
            request.end_date,
        )?;

        let performance_metrics = MetricsAggregator::aggregate_performance(
            incidents,
            request.start_date,
            request.end_date,
        )?;

        let mut summary_report = SummaryReport::new(incident_metrics, performance_metrics);
        summary_report.generate_findings();
        summary_report.generate_recommendations();

        let summary = format!(
            "{} total incidents, {} open, {} resolved. MTTR: {:.1} minutes",
            summary_report.incident_metrics.total_incidents,
            summary_report.incident_metrics.open_incidents,
            summary_report.incident_metrics.resolved_incidents,
            summary_report.performance_metrics.mttr / 60.0
        );

        let data = serde_json::to_value(&summary_report)
            .map_err(|e| AnalyticsError::ReportGenerationFailed(e.to_string()))?;

        Ok(Report::new(
            "Incident Summary Report".to_string(),
            ReportType::Summary,
            request.start_date,
            request.end_date,
        )
        .with_summary(summary)
        .with_data(data))
    }

    /// Generate SLA report
    async fn generate_sla_report(
        &self,
        incidents: &[Incident],
        request: &ReportRequest,
    ) -> AnalyticsResult<Report> {
        let sla_metrics = MetricsAggregator::aggregate_sla(
            incidents,
            request.start_date,
            request.end_date,
            &self.config.sla_targets,
        )?;

        let mut sla_report = SLAReport::new(sla_metrics);
        sla_report.generate_at_risk_summary();

        let summary = format!(
            "{:.1}% SLA compliance, {} breaches",
            sla_report.sla_metrics.overall_compliance, sla_report.sla_metrics.total_breaches
        );

        let data = serde_json::to_value(&sla_report)
            .map_err(|e| AnalyticsError::ReportGenerationFailed(e.to_string()))?;

        Ok(Report::new(
            "SLA Compliance Report".to_string(),
            ReportType::SLA,
            request.start_date,
            request.end_date,
        )
        .with_summary(summary)
        .with_data(data))
    }

    /// Generate trend report
    async fn generate_trend_report(
        &self,
        incidents: &[Incident],
        request: &ReportRequest,
    ) -> AnalyticsResult<Report> {
        if !self.config.enable_trends {
            return Err(AnalyticsError::InvalidConfiguration(
                "Trend analysis is disabled".to_string(),
            ));
        }

        if incidents.len() < self.config.min_data_points {
            return Err(AnalyticsError::InsufficientData(format!(
                "Need at least {} incidents for trend analysis",
                self.config.min_data_points
            )));
        }

        // Create time series
        let time_series = MetricsAggregator::create_time_series(
            incidents,
            AggregationPeriod::Daily,
            "incident_count",
        )?;

        // Perform trend analysis
        let trend_analysis = TrendAnalysis::analyze(
            &time_series.timestamps(),
            &time_series.values(),
        )?;

        let incident_trend = TrendMetrics {
            direction: if trend_analysis.slope > 0.1 {
                crate::analytics::metrics::TrendDirection::Increasing
            } else if trend_analysis.slope < -0.1 {
                crate::analytics::metrics::TrendDirection::Decreasing
            } else {
                crate::analytics::metrics::TrendDirection::Stable
            },
            rate_of_change: trend_analysis.slope,
            confidence: trend_analysis.r_squared,
            forecast: trend_analysis.forecast(Utc::now().timestamp() as f64),
            upper_bound: trend_analysis.forecast(Utc::now().timestamp() as f64) * 1.2,
            lower_bound: trend_analysis.forecast(Utc::now().timestamp() as f64) * 0.8,
            significance: trend_analysis.p_value,
            has_seasonality: false,
            seasonal_period: None,
        };

        let trend_report = TrendReport::new(incident_trend.clone(), incident_trend);

        let summary = format!(
            "Incident trend: {:?}, forecast: {:.0} incidents",
            trend_report.incident_trend.direction, trend_report.incident_trend.forecast
        );

        let data = serde_json::to_value(&trend_report)
            .map_err(|e| AnalyticsError::ReportGenerationFailed(e.to_string()))?;

        Ok(Report::new(
            "Trend Analysis Report".to_string(),
            ReportType::Trend,
            request.start_date,
            request.end_date,
        )
        .with_summary(summary)
        .with_data(data))
    }

    /// Generate team performance report
    async fn generate_team_performance_report(
        &self,
        _incidents: &[Incident],
        request: &ReportRequest,
    ) -> AnalyticsResult<Report> {
        // For now, return a placeholder since we don't have team data in incidents
        let mut team_report = TeamPerformanceReport::new(Vec::new());
        team_report.generate_rankings();
        team_report.generate_comparisons();

        let summary = "Team performance analysis (placeholder)".to_string();

        let data = serde_json::to_value(&team_report)
            .map_err(|e| AnalyticsError::ReportGenerationFailed(e.to_string()))?;

        Ok(Report::new(
            "Team Performance Report".to_string(),
            ReportType::TeamPerformance,
            request.start_date,
            request.end_date,
        )
        .with_summary(summary)
        .with_data(data))
    }

    /// Generate incident analysis report
    async fn generate_incident_analysis_report(
        &self,
        incidents: &[Incident],
        request: &ReportRequest,
    ) -> AnalyticsResult<Report> {
        let incident_metrics = MetricsAggregator::aggregate_incidents(
            incidents,
            request.start_date,
            request.end_date,
        )?;

        let summary = format!(
            "Analyzed {} incidents across {} sources",
            incident_metrics.total_incidents,
            incident_metrics.by_source.len()
        );

        let data = serde_json::to_value(&incident_metrics)
            .map_err(|e| AnalyticsError::ReportGenerationFailed(e.to_string()))?;

        Ok(Report::new(
            "Incident Analysis Report".to_string(),
            ReportType::IncidentAnalysis,
            request.start_date,
            request.end_date,
        )
        .with_summary(summary)
        .with_data(data))
    }

    /// Clear report cache
    pub async fn clear_cache(&self) {
        let mut cache = self.report_cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.report_cache.read().await;
        CacheStats {
            cached_reports: cache.len(),
            total_size_bytes: 0, // Would need to calculate actual size
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub cached_reports: usize,
    pub total_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_config_default() {
        let config = AnalyticsConfig::default();
        assert!(config.enable_trends);
        assert_eq!(config.min_data_points, 5);
        assert!(config.sla_targets.contains_key("P0"));
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = AnalyticsEngine::with_defaults();
        let stats = engine.get_cache_stats().await;
        assert_eq!(stats.cached_reports, 0);
    }
}
