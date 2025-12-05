pub mod circuit_breaker_wrappers;
pub mod common;
pub mod edge_agent;
pub mod governance;
pub mod sentinel;
pub mod shield;
pub mod upstream;

pub use circuit_breaker_wrappers::{
    EdgeAgentClientWithBreaker, GovernanceClientWithBreaker, SentinelClientWithBreaker,
    ShieldClientWithBreaker,
};

pub use common::{
    ConnectionConfig, ConnectionState, Credentials, HealthCheck, HealthStatus,
    IntegrationError, IntegrationResult, LLMClient, RetryPolicy,
};

pub use edge_agent::{EdgeAgentClient, EdgeInferenceHandler, ResourceAwarePrioritizer};

pub use sentinel::{
    AlertCategory, AlertHandler, AlertSeverity, SentinelAlert, SentinelClient,
};

pub use shield::{SecurityEventHandler, ShieldClient};

pub use governance::{
    AuditEntry, AuditReport, AuditReportType, ComplianceData, ComplianceEventHandler,
    ComplianceFramework, ComplianceRequest, ComplianceResponse, DataClassification,
    GovernanceClient, GovernanceMetrics, PolicyEngine, PolicyViolation, ViolationSeverity,
};

// Upstream LLM-Dev-Ops ecosystem adapters (Phase 2B)
pub use upstream::{
    // Sentinel adapter
    SentinelCoreAdapter, UpstreamAnomalyEvent, UpstreamThreatSignal, UpstreamDriftEvent,
    DriftType, ThreatType,
    // Observatory adapter
    ObservatoryCoreAdapter, UpstreamTelemetryStream, UpstreamTraceSpan, UpstreamStructuredEvent,
    TelemetryMetrics, TraceStatus, EventSeverity,
    // Governance adapter
    GovernanceDashboardAdapter, UpstreamComplianceMetadata, UpstreamAuditReference,
    UpstreamEscalationRule, EnforcementLevel,
    // Analytics adapter
    AnalyticsHubAdapter, UpstreamStatisticalBaseline, UpstreamOutlierDetection,
    UpstreamLongTailAnalytics, OutlierType, OutlierSeverity,
    // Policy adapter
    PolicyEngineAdapter, PolicyEvaluationRequest, PolicyDecision, DecisionType,
    EnforcementConstraints, FallbackMode,
};
