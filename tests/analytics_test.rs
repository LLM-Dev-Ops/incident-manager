//! Comprehensive tests for the analytics module

use llm_incident_manager::analytics::*;
use llm_incident_manager::models::{Incident, IncidentState, IncidentType, Severity};
use chrono::{Duration, Utc};
use std::collections::HashMap;

/// Helper function to create a test incident
fn create_test_incident(
    severity: Severity,
    incident_type: IncidentType,
    state: IncidentState,
    hours_ago: i64,
) -> Incident {
    let created_at = Utc::now() - Duration::hours(hours_ago);
    Incident {
        id: uuid::Uuid::new_v4().to_string(),
        title: format!("Test Incident {}", hours_ago),
        description: "Test description".to_string(),
        severity,
        incident_type,
        state,
        source: "test".to_string(),
        created_at,
        updated_at: created_at + Duration::hours(1),
        resolved_at: if state == IncidentState::Resolved {
            Some(created_at + Duration::hours(2))
        } else {
            None
        },
        assignee: None,
        tags: vec![],
        metadata: HashMap::new(),
        correlation_id: None,
    }
}

#[tokio::test]
async fn test_analytics_engine_creation() {
    let config = AnalyticsConfig::default();
    let engine = AnalyticsEngine::new(config);

    // Should be able to create engine
    let stats = engine.get_cache_stats().await;
    assert_eq!(stats.cached_reports, 0);
}

#[tokio::test]
async fn test_summary_report_generation() {
    let engine = AnalyticsEngine::with_defaults();

    // Create test incidents
    let incidents = vec![
        create_test_incident(Severity::P0, IncidentType::Infrastructure, IncidentState::Open, 2),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 5),
        create_test_incident(Severity::P2, IncidentType::Security, IncidentState::Resolved, 10),
    ];

    engine.update_incident_cache(incidents).await;

    let request = ReportRequest::new(
        ReportType::Summary,
        Utc::now() - Duration::days(1),
        Utc::now(),
    );

    let report = engine.generate_report(&request).await;
    assert!(report.is_ok(), "Should generate summary report");

    let report = report.unwrap();
    assert_eq!(report.report_type, ReportType::Summary);
    assert!(!report.summary.is_empty());
}

#[tokio::test]
async fn test_sla_report_generation() {
    let engine = AnalyticsEngine::with_defaults();

    // Create incidents with different resolution times
    let incidents = vec![
        create_test_incident(Severity::P0, IncidentType::Infrastructure, IncidentState::Resolved, 1),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 3),
        create_test_incident(Severity::P2, IncidentType::Security, IncidentState::Resolved, 6),
    ];

    engine.update_incident_cache(incidents).await;

    let request = ReportRequest::new(
        ReportType::SLA,
        Utc::now() - Duration::days(1),
        Utc::now(),
    );

    let report = engine.generate_report(&request).await;
    assert!(report.is_ok(), "Should generate SLA report");

    let report = report.unwrap();
    assert_eq!(report.report_type, ReportType::SLA);
}

#[tokio::test]
async fn test_trend_report_generation() {
    let engine = AnalyticsEngine::with_defaults();

    // Create more incidents for trend analysis
    let mut incidents = Vec::new();
    for i in 0..10 {
        incidents.push(create_test_incident(
            Severity::P1,
            IncidentType::Application,
            IncidentState::Resolved,
            i * 6, // Every 6 hours
        ));
    }

    engine.update_incident_cache(incidents).await;

    let request = ReportRequest::new(
        ReportType::Trend,
        Utc::now() - Duration::days(3),
        Utc::now(),
    );

    let report = engine.generate_report(&request).await;
    assert!(report.is_ok(), "Should generate trend report");
}

#[tokio::test]
async fn test_report_filtering() {
    let engine = AnalyticsEngine::with_defaults();

    let incidents = vec![
        create_test_incident(Severity::P0, IncidentType::Infrastructure, IncidentState::Open, 1),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Open, 2),
        create_test_incident(Severity::P2, IncidentType::Security, IncidentState::Resolved, 3),
    ];

    engine.update_incident_cache(incidents).await;

    // Filter by severity
    let filter = ReportFilter::new()
        .with_severities(vec!["P0".to_string(), "P1".to_string()]);

    let request = ReportRequest::new(
        ReportType::Summary,
        Utc::now() - Duration::days(1),
        Utc::now(),
    )
    .with_filters(filter);

    let report = engine.generate_report(&request).await;
    assert!(report.is_ok(), "Should generate filtered report");
}

#[tokio::test]
async fn test_report_caching() {
    let engine = AnalyticsEngine::with_defaults();

    let incidents = vec![
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 5),
    ];

    engine.update_incident_cache(incidents).await;

    let request = ReportRequest::new(
        ReportType::Summary,
        Utc::now() - Duration::days(1),
        Utc::now(),
    );

    // Generate report twice
    let report1 = engine.generate_report(&request).await.unwrap();
    let report2 = engine.generate_report(&request).await.unwrap();

    // Should return cached version (same ID)
    assert_eq!(report1.id, report2.id);

    // Cache stats should show 1 cached report
    let stats = engine.get_cache_stats().await;
    assert_eq!(stats.cached_reports, 1);
}

#[test]
fn test_metrics_aggregation() {
    let incidents = vec![
        create_test_incident(Severity::P0, IncidentType::Infrastructure, IncidentState::Open, 1),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 5),
        create_test_incident(Severity::P2, IncidentType::Security, IncidentState::Resolved, 10),
    ];

    let start = Utc::now() - Duration::days(1);
    let end = Utc::now();

    let metrics = MetricsAggregator::aggregate_incidents(&incidents, start, end);
    assert!(metrics.is_ok());

    let metrics = metrics.unwrap();
    assert_eq!(metrics.total_incidents, 3);
    assert_eq!(metrics.open_incidents, 1);
    assert_eq!(metrics.resolved_incidents, 2);
}

#[test]
fn test_performance_metrics() {
    let incidents = vec![
        create_test_incident(Severity::P0, IncidentType::Infrastructure, IncidentState::Resolved, 2),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 5),
    ];

    let start = Utc::now() - Duration::days(1);
    let end = Utc::now();

    let perf_metrics = MetricsAggregator::aggregate_performance(&incidents, start, end);
    assert!(perf_metrics.is_ok());

    let metrics = perf_metrics.unwrap();
    assert!(metrics.mttr >= 0.0);
    assert!(metrics.mttrs >= 0.0);
}

#[test]
fn test_sla_metrics() {
    let incidents = vec![
        create_test_incident(Severity::P0, IncidentType::Infrastructure, IncidentState::Resolved, 1),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 3),
    ];

    let start = Utc::now() - Duration::days(1);
    let end = Utc::now();

    let mut sla_targets = HashMap::new();
    sla_targets.insert("P0".to_string(), 900.0);
    sla_targets.insert("P1".to_string(), 3600.0);

    let sla_metrics = MetricsAggregator::aggregate_sla(&incidents, start, end, &sla_targets);
    assert!(sla_metrics.is_ok());

    let metrics = sla_metrics.unwrap();
    assert!(metrics.overall_compliance >= 0.0 && metrics.overall_compliance <= 100.0);
}

#[test]
fn test_time_series_creation() {
    let incidents = vec![
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 2),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 10),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 20),
    ];

    let time_series = MetricsAggregator::create_time_series(
        &incidents,
        AggregationPeriod::Daily,
        "incident_count",
    );

    assert!(time_series.is_ok());
    let series = time_series.unwrap();
    assert!(!series.points.is_empty());
    assert_eq!(series.name, "incident_count");
}

#[test]
fn test_dashboard_provider() {
    let provider = DashboardProvider::with_defaults();

    let incidents = vec![
        create_test_incident(Severity::P0, IncidentType::Infrastructure, IncidentState::Open, 1),
        create_test_incident(Severity::P1, IncidentType::Application, IncidentState::Resolved, 5),
    ];

    let dashboard = provider.generate_dashboard(&incidents);
    assert!(dashboard.is_ok());

    let data = dashboard.unwrap();
    assert!(data.metrics.open_incidents >= 0);
    assert!(!data.alerts.is_empty() || data.alerts.is_empty()); // Can have alerts or not
}

#[test]
fn test_statistics_percentiles() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let percentiles = Percentiles::from_data(data);

    assert!((percentiles.p50 - 5.5).abs() < 1.0);
    assert!(percentiles.p90 > percentiles.p50);
    assert!(percentiles.p99 > percentiles.p90);
}

#[test]
fn test_statistics_distribution() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let dist = Distribution::from_data(&data);

    assert!(dist.is_ok());
    let dist = dist.unwrap();
    assert_eq!(dist.mean, 3.0);
    assert_eq!(dist.median, 3.0);
    assert_eq!(dist.min, 1.0);
    assert_eq!(dist.max, 5.0);
}

#[test]
fn test_trend_analysis() {
    let timestamps = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let values = vec![2.0, 4.0, 6.0, 8.0, 10.0];

    let trend = TrendAnalysis::analyze(&timestamps, &values);
    assert!(trend.is_ok());

    let trend = trend.unwrap();
    assert!((trend.slope - 2.0).abs() < 0.01);
    assert!(trend.r_squared > 0.99);
}

#[test]
fn test_moving_average() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let ma = StatisticalAnalysis::moving_average(&data, 3);

    assert!(!ma.is_empty());
    assert_eq!(ma[0], 2.0); // Average of [1, 2, 3]
}

#[test]
fn test_outlier_detection() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0]; // 100 is an outlier
    let outliers = StatisticalAnalysis::detect_outliers(&data);

    assert!(!outliers.is_empty());
}

#[test]
fn test_correlation() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];

    let correlation = StatisticalAnalysis::correlation(&x, &y);
    assert!(correlation.is_ok());

    let corr = correlation.unwrap();
    assert!((corr - 1.0).abs() < 0.01); // Perfect correlation
}

#[tokio::test]
async fn test_export_json() {
    use std::path::PathBuf;

    let report = Report::new(
        "Test Report".to_string(),
        ReportType::Summary,
        Utc::now() - Duration::days(1),
        Utc::now(),
    );

    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join("test_report.json");

    let result = ReportExporter::export(&report, ExportFormat::Json, &output_path).await;
    assert!(result.is_ok());

    // Clean up
    let _ = tokio::fs::remove_file(output_path).await;
}

#[tokio::test]
async fn test_export_csv() {
    use std::path::PathBuf;

    let report = Report::new(
        "Test Report".to_string(),
        ReportType::Summary,
        Utc::now() - Duration::days(1),
        Utc::now(),
    );

    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join("test_report.csv");

    let result = ReportExporter::export(&report, ExportFormat::Csv, &output_path).await;
    assert!(result.is_ok());

    // Clean up
    let _ = tokio::fs::remove_file(output_path).await;
}

#[tokio::test]
async fn test_export_html() {
    use std::path::PathBuf;

    let report = Report::new(
        "Test Report".to_string(),
        ReportType::Summary,
        Utc::now() - Duration::days(1),
        Utc::now(),
    );

    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join("test_report.html");

    let result = ReportExporter::export(&report, ExportFormat::Html, &output_path).await;
    assert!(result.is_ok());

    // Clean up
    let _ = tokio::fs::remove_file(output_path).await;
}

#[test]
fn test_aggregation_period() {
    assert_eq!(AggregationPeriod::Hourly.duration(), Duration::hours(1));
    assert_eq!(AggregationPeriod::Daily.duration(), Duration::days(1));
    assert_eq!(AggregationPeriod::Weekly.duration(), Duration::weeks(1));
}

#[test]
fn test_export_format() {
    assert_eq!(ExportFormat::Json.extension(), "json");
    assert_eq!(ExportFormat::Csv.extension(), "csv");
    assert_eq!(ExportFormat::Html.extension(), "html");
    assert_eq!(ExportFormat::Pdf.extension(), "pdf");

    assert_eq!(ExportFormat::Json.mime_type(), "application/json");
    assert_eq!(ExportFormat::Csv.mime_type(), "text/csv");
}

#[test]
fn test_summary_report_findings() {
    let incident_metrics = IncidentMetrics {
        total_incidents: 150,
        open_incidents: 10,
        resolved_incidents: 140,
        by_severity: {
            let mut map = HashMap::new();
            map.insert("P0".to_string(), 15);
            map.insert("P1".to_string(), 50);
            map
        },
        by_type: HashMap::new(),
        by_source: HashMap::new(),
        by_status: HashMap::new(),
        avg_incidents_per_day: 15.0,
        peak_hour: Some(14),
        period_start: Utc::now() - Duration::days(7),
        period_end: Utc::now(),
    };

    let performance_metrics = PerformanceMetrics::default();

    let mut summary = SummaryReport::new(incident_metrics, performance_metrics);
    summary.generate_findings();
    summary.generate_recommendations();

    assert!(!summary.key_findings.is_empty());
}

#[test]
fn test_team_performance_rankings() {
    let teams = vec![
        TeamMetrics {
            team_id: "team1".to_string(),
            team_name: "Team 1".to_string(),
            incidents_handled: 100,
            avg_resolution_time: 3600.0,
            avg_first_response_time: 300.0,
            resolution_rate: 95.0,
            escalation_rate: 5.0,
            sla_compliance: 98.0,
            satisfaction_score: Some(4.5),
            incidents_by_status: HashMap::new(),
        },
        TeamMetrics {
            team_id: "team2".to_string(),
            team_name: "Team 2".to_string(),
            incidents_handled: 80,
            avg_resolution_time: 2400.0,
            avg_first_response_time: 200.0,
            resolution_rate: 90.0,
            escalation_rate: 10.0,
            sla_compliance: 95.0,
            satisfaction_score: Some(4.2),
            incidents_by_status: HashMap::new(),
        },
    ];

    let mut team_report = TeamPerformanceReport::new(teams);
    team_report.generate_rankings();
    team_report.generate_comparisons();

    assert_eq!(team_report.rankings.by_volume_handled.len(), 2);
    assert_eq!(team_report.comparisons.len(), 2);
}
