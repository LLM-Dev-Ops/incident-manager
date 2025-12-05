//! Upstream LLM-Dev-Ops Ecosystem Adapters (Phase 2B)
//!
//! This module provides thin runtime consume-from adapters for upstream
//! LLM-Dev-Ops ecosystem repositories. These adapters translate upstream
//! types to incident-manager internal types without modifying existing
//! alerting algorithms, escalation-chain logic, or event-priority heuristics.
//!
//! # Integrated Upstream Modules
//!
//! - **LLM-Sentinel**: Anomaly flags, threat signals, drift events
//! - **LLM-Observatory**: Telemetry streams, traces, structured events
//! - **LLM-Governance-Dashboard**: Compliance metadata, audit trail references
//! - **LLM-Analytics-Hub**: Statistical baselines, outlier detection
//! - **LLM-Policy-Engine**: Real-time policy decisions, enforcement constraints

pub mod sentinel_adapter;
pub mod observatory_adapter;
pub mod governance_adapter;
pub mod analytics_adapter;
pub mod policy_adapter;

// Re-export adapter types for convenience
pub use sentinel_adapter::{
    SentinelCoreAdapter, UpstreamAnomalyEvent, UpstreamThreatSignal, UpstreamDriftEvent,
    DriftType, ThreatType,
};
pub use observatory_adapter::{
    ObservatoryCoreAdapter, UpstreamTelemetryStream, UpstreamTraceSpan, UpstreamStructuredEvent,
    TelemetryMetrics, TraceStatus, EventSeverity,
};
pub use governance_adapter::{
    GovernanceDashboardAdapter, UpstreamComplianceMetadata, UpstreamAuditReference,
    UpstreamEscalationRule, EnforcementLevel,
};
pub use analytics_adapter::{
    AnalyticsHubAdapter, UpstreamStatisticalBaseline, UpstreamOutlierDetection,
    UpstreamLongTailAnalytics, OutlierType, OutlierSeverity,
};
pub use policy_adapter::{
    PolicyEngineAdapter, PolicyEvaluationRequest, PolicyDecision, DecisionType,
    EnforcementConstraints, FallbackMode,
};
