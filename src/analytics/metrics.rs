//! Core metrics structures for analytics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Incident-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentMetrics {
    /// Total number of incidents
    pub total_incidents: u64,

    /// Open incidents
    pub open_incidents: u64,

    /// Resolved incidents
    pub resolved_incidents: u64,

    /// Incidents by severity
    pub by_severity: HashMap<String, u64>,

    /// Incidents by type
    pub by_type: HashMap<String, u64>,

    /// Incidents by source
    pub by_source: HashMap<String, u64>,

    /// Incidents by status
    pub by_status: HashMap<String, u64>,

    /// Average incidents per day
    pub avg_incidents_per_day: f64,

    /// Peak incident hour
    pub peak_hour: Option<u32>,

    /// Time period
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl Default for IncidentMetrics {
    fn default() -> Self {
        Self {
            total_incidents: 0,
            open_incidents: 0,
            resolved_incidents: 0,
            by_severity: HashMap::new(),
            by_type: HashMap::new(),
            by_source: HashMap::new(),
            by_status: HashMap::new(),
            avg_incidents_per_day: 0.0,
            peak_hour: None,
            period_start: Utc::now(),
            period_end: Utc::now(),
        }
    }
}

/// Performance metrics (MTTR, MTTD, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Mean Time To Detect (seconds)
    pub mttd: f64,

    /// Mean Time To Respond (seconds)
    pub mttr: f64,

    /// Mean Time To Resolve (seconds)
    pub mttrs: f64,

    /// P50 response time (seconds)
    pub p50_response_time: f64,

    /// P90 response time (seconds)
    pub p90_response_time: f64,

    /// P95 response time (seconds)
    pub p95_response_time: f64,

    /// P99 response time (seconds)
    pub p99_response_time: f64,

    /// Average resolution time by severity
    pub resolution_time_by_severity: HashMap<String, f64>,

    /// First response time (seconds)
    pub first_response_time: f64,

    /// Escalation rate (percentage)
    pub escalation_rate: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            mttd: 0.0,
            mttr: 0.0,
            mttrs: 0.0,
            p50_response_time: 0.0,
            p90_response_time: 0.0,
            p95_response_time: 0.0,
            p99_response_time: 0.0,
            resolution_time_by_severity: HashMap::new(),
            first_response_time: 0.0,
            escalation_rate: 0.0,
        }
    }
}

/// SLA compliance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAMetrics {
    /// Overall SLA compliance rate (percentage)
    pub overall_compliance: f64,

    /// Response SLA compliance
    pub response_compliance: f64,

    /// Resolution SLA compliance
    pub resolution_compliance: f64,

    /// SLA breaches count
    pub total_breaches: u64,

    /// Breaches by severity
    pub breaches_by_severity: HashMap<String, u64>,

    /// Average breach time (seconds)
    pub avg_breach_time: f64,

    /// Near-miss incidents (within 10% of SLA)
    pub near_misses: u64,

    /// At-risk incidents (currently open, approaching SLA)
    pub at_risk_incidents: u64,
}

impl Default for SLAMetrics {
    fn default() -> Self {
        Self {
            overall_compliance: 100.0,
            response_compliance: 100.0,
            resolution_compliance: 100.0,
            total_breaches: 0,
            breaches_by_severity: HashMap::new(),
            avg_breach_time: 0.0,
            near_misses: 0,
            at_risk_incidents: 0,
        }
    }
}

/// Team performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMetrics {
    /// Team or individual identifier
    pub team_id: String,

    /// Team name
    pub team_name: String,

    /// Total incidents handled
    pub incidents_handled: u64,

    /// Average resolution time (seconds)
    pub avg_resolution_time: f64,

    /// First response time (seconds)
    pub avg_first_response_time: f64,

    /// Resolution rate (percentage)
    pub resolution_rate: f64,

    /// Escalation rate (percentage)
    pub escalation_rate: f64,

    /// SLA compliance (percentage)
    pub sla_compliance: f64,

    /// Customer satisfaction score (1-5)
    pub satisfaction_score: Option<f64>,

    /// Incidents by status
    pub incidents_by_status: HashMap<String, u64>,
}

/// Trend metrics for pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendMetrics {
    /// Trend direction: increasing, decreasing, stable
    pub direction: TrendDirection,

    /// Rate of change (percentage per period)
    pub rate_of_change: f64,

    /// Confidence in trend (0-1)
    pub confidence: f64,

    /// Forecasted value for next period
    pub forecast: f64,

    /// Upper confidence bound
    pub upper_bound: f64,

    /// Lower confidence bound
    pub lower_bound: f64,

    /// Trend significance (p-value)
    pub significance: f64,

    /// Seasonality detected
    pub has_seasonality: bool,

    /// Seasonal period (if detected)
    pub seasonal_period: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

impl Default for TrendMetrics {
    fn default() -> Self {
        Self {
            direction: TrendDirection::Stable,
            rate_of_change: 0.0,
            confidence: 0.0,
            forecast: 0.0,
            upper_bound: 0.0,
            lower_bound: 0.0,
            significance: 1.0,
            has_seasonality: false,
            seasonal_period: None,
        }
    }
}
