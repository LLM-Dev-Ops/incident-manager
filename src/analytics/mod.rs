//! Enterprise-grade analytics and reporting system
//!
//! This module provides comprehensive analytics, metrics aggregation, and report generation
//! capabilities for incident management data.
//!
//! # Features
//!
//! - **Time-Series Analysis**: Trend analysis over configurable time periods
//! - **Metrics Aggregation**: MTTR, MTTD, incident volumes, SLA compliance
//! - **Report Generation**: Multiple report types (summary, SLA, trends, team performance)
//! - **Export Formats**: JSON, CSV, PDF support
//! - **Dashboard Data**: Real-time metrics for dashboards
//! - **Statistical Analysis**: Percentiles, distributions, correlations
//!
//! # Report Types
//!
//! - **Summary Reports**: Daily/weekly/monthly incident summaries
//! - **SLA Reports**: Compliance tracking and breach analysis
//! - **Trend Reports**: Pattern analysis and forecasting
//! - **Team Performance**: Response times and resolution rates
//! - **Incident Analysis**: Deep-dive into specific incidents or patterns
//!
//! # Example
//!
//! ```no_run
//! use llm_incident_manager::analytics::{AnalyticsEngine, ReportRequest, ReportType};
//! use chrono::{Duration, Utc};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = AnalyticsEngine::new();
//!
//!     let request = ReportRequest {
//!         report_type: ReportType::Summary,
//!         start_date: Utc::now() - Duration::days(7),
//!         end_date: Utc::now(),
//!         filters: Default::default(),
//!     };
//!
//!     let report = engine.generate_report(&request).await?;
//!     println!("Generated report: {}", report.title);
//!
//!     Ok(())
//! }
//! ```

mod aggregation;
mod dashboard;
mod engine;
mod error;
mod export;
mod metrics;
mod reports;
mod statistics;

pub use aggregation::{
    AggregationPeriod, MetricsAggregator, TimeSeriesData, TimeSeriesPoint,
};
pub use dashboard::{DashboardData, DashboardMetrics, DashboardProvider};
pub use engine::{AnalyticsEngine, AnalyticsConfig};
pub use error::{AnalyticsError, AnalyticsResult};
pub use export::{ExportFormat, ReportExporter};
pub use metrics::{
    IncidentMetrics, PerformanceMetrics, SLAMetrics, TeamMetrics, TrendMetrics,
};
pub use reports::{
    Report, ReportFilter, ReportRequest, ReportType, SLAReport, SummaryReport,
    TeamPerformanceReport, TrendReport,
};
pub use statistics::{
    Distribution, Percentiles, StatisticalAnalysis, Statistics, TrendAnalysis,
};
