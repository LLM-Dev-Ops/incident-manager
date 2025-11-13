pub mod circuit_breaker_wrappers;
pub mod common;
pub mod edge_agent;
pub mod governance;
pub mod sentinel;
pub mod shield;

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
