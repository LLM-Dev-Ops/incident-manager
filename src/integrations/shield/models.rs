use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security event requiring threat analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_id: String,
    pub event_type: SecurityEventType,
    pub source: String,
    pub description: String,
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub severity: EventSeverity,
}

/// Types of security events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityEventType {
    UnauthorizedAccess,
    DataExfiltration,
    MaliciousPayload,
    AnomalousPattern,
    PolicyViolation,
    PrivilegeEscalation,
    SuspiciousActivity,
    Other(String),
}

/// Severity levels for events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for EventSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSeverity::Low => write!(f, "LOW"),
            EventSeverity::Medium => write!(f, "MEDIUM"),
            EventSeverity::High => write!(f, "HIGH"),
            EventSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Result of threat analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAnalysis {
    pub analysis_id: String,
    pub threat_type: ThreatType,
    pub confidence_score: f64,
    pub severity_level: ThreatSeverity,
    pub indicators: Vec<String>,
    pub affected_systems: Vec<String>,
    pub description: String,
    pub details: HashMap<String, String>,
    pub analyzed_at: DateTime<Utc>,
}

/// Types of identified threats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreatType {
    Malware,
    RansomwareAttack,
    DataBreach,
    DenialOfService,
    InsiderThreat,
    PhishingAttempt,
    BruteForce,
    SqlInjection,
    CrossSiteScripting,
    ZeroDayExploit,
    AdvancedPersistentThreat,
    Unknown,
}

/// Threat severity classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ThreatSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatSeverity::Low => write!(f, "LOW"),
            ThreatSeverity::Medium => write!(f, "MEDIUM"),
            ThreatSeverity::High => write!(f, "HIGH"),
            ThreatSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub assessment_id: String,
    pub risk_level: RiskLevel,
    pub risk_score: f64,
    pub risk_factors: Vec<RiskFactor>,
    pub recommendations: Vec<String>,
    pub assessed_at: DateTime<Utc>,
}

/// Overall risk level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Individual risk factor contributing to overall risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub description: String,
    pub impact_score: f64,
    pub likelihood: f64,
}

/// Mitigation plan for addressing threats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationPlan {
    pub plan_id: String,
    pub steps: Vec<MitigationStep>,
    pub estimated_duration_minutes: i32,
    pub priority: MitigationPriority,
    pub required_resources: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Priority level for mitigation actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MitigationPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for MitigationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MitigationPriority::Low => write!(f, "LOW"),
            MitigationPriority::Medium => write!(f, "MEDIUM"),
            MitigationPriority::High => write!(f, "HIGH"),
            MitigationPriority::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Individual step in a mitigation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationStep {
    pub order: i32,
    pub action: String,
    pub description: String,
    pub automation_available: bool,
    pub dependencies: Vec<String>,
}

/// Security pattern recognition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPattern {
    pub pattern_id: String,
    pub is_known_pattern: bool,
    pub pattern_name: String,
    pub confidence: f64,
    pub similar_patterns: Vec<String>,
    pub recommendation: String,
}

/// Request for analyzing security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeSecurityEventRequest {
    pub event: SecurityEvent,
}

/// Request for risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessRiskRequest {
    pub context_id: String,
    pub event_ids: Vec<String>,
    pub assessment_type: String,
    pub parameters: HashMap<String, String>,
}

/// Request for mitigation plan generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateMitigationRequest {
    pub threat_id: String,
    pub threat_type: ThreatType,
    pub severity: ThreatSeverity,
    pub affected_systems: Vec<String>,
    pub context: HashMap<String, String>,
}

/// Request for security pattern reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSecurityPatternRequest {
    pub pattern_type: String,
    pub event_ids: Vec<String>,
    pub description: String,
    pub attributes: HashMap<String, String>,
}


// Type aliases for Shield responses
pub type ValidationResult = ThreatAnalysis;
pub type ThreatScanResult = ThreatAnalysis;

