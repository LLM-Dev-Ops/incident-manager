//! LLM-Governance-Dashboard Adapter
//!
//! Thin runtime adapter for consuming compliance metadata, audit trail references,
//! and escalation governance rules from the upstream llm-governance-common crate.
//! Translates governance types to internal incident-manager types without modifying
//! existing escalation-chain logic.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Import internal types
use crate::integrations::governance::{
    AuditEntry, ComplianceFramework, PolicyViolation, ViolationSeverity,
};

/// Adapter for consuming governance data from llm-governance-common
#[derive(Debug, Clone)]
pub struct GovernanceDashboardAdapter {
    /// Source identifier for tracing
    source_id: String,
    /// Organization context
    org_id: Option<String>,
}

impl Default for GovernanceDashboardAdapter {
    fn default() -> Self {
        Self::new("llm-governance-dashboard")
    }
}

impl GovernanceDashboardAdapter {
    /// Create a new governance dashboard adapter
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            org_id: None,
        }
    }

    /// Set organization context
    pub fn with_org(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Convert upstream audit log to internal AuditEntry
    ///
    /// This is the primary consumption point for audit trails from governance-dashboard.
    /// Does not modify audit logic - pure type translation.
    pub fn convert_audit_log(&self, log: &UpstreamAuditLog) -> AuditEntry {
        let mut entry = AuditEntry::new(
            log.id,
            log.action.clone(),
            log.user_id.map(|u| u.to_string()).unwrap_or_else(|| "system".to_string()),
        );

        // Add metadata from audit log
        entry = entry.with_detail("resource".to_string(), serde_json::json!(log.resource));
        entry = entry.with_detail("metadata".to_string(), log.metadata.clone());

        if let Some(ip) = &log.ip_address {
            entry = entry.with_detail("ip_address".to_string(), serde_json::json!(ip));
        }

        if let Some(ua) = &log.user_agent {
            entry = entry.with_detail("user_agent".to_string(), serde_json::json!(ua));
        }

        entry
    }

    /// Convert upstream policy to internal escalation rule representation
    pub fn convert_policy_to_escalation_rule(&self, policy: &UpstreamPolicy) -> UpstreamEscalationRule {
        UpstreamEscalationRule {
            id: policy.id.to_string(),
            name: policy.name.clone(),
            description: policy.description.clone(),
            is_active: policy.is_active,
            rules: policy.rules.iter().map(|r| self.convert_rule(r)).collect(),
            priority: self.infer_priority(&policy.rules),
            created_by: policy.created_by.to_string(),
            created_at: policy.created_at,
            updated_at: policy.updated_at,
            source: self.source_id.clone(),
        }
    }

    /// Convert upstream policy violation to internal PolicyViolation
    pub fn convert_violation(&self, violation: &UpstreamPolicyViolation) -> PolicyViolation {
        PolicyViolation::new(
            self.infer_framework(&violation.policy_id),
            violation.policy_id.clone(),
            format!("Policy Violation: {}", violation.id),
            self.convert_violation_severity(&violation.severity),
            violation.description.clone(),
            self.generate_remediation(&violation.severity),
        )
    }

    /// Extract compliance metadata from policy
    pub fn extract_compliance_metadata(&self, policy: &UpstreamPolicy) -> UpstreamComplianceMetadata {
        UpstreamComplianceMetadata {
            policy_id: policy.id.to_string(),
            policy_name: policy.name.clone(),
            frameworks: self.detect_frameworks(&policy.rules),
            enforcement_level: self.infer_enforcement_level(&policy.rules),
            data_classifications: self.extract_data_classifications(&policy.rules),
            retention_period_days: self.extract_retention_period(&policy.rules),
            audit_required: self.is_audit_required(&policy.rules),
            last_reviewed: policy.updated_at,
            source: self.source_id.clone(),
        }
    }

    /// Create audit reference from audit log
    pub fn create_audit_reference(&self, log: &UpstreamAuditLog) -> UpstreamAuditReference {
        UpstreamAuditReference {
            audit_id: log.id.to_string(),
            timestamp: log.created_at,
            action: log.action.clone(),
            resource: log.resource.clone(),
            actor: log.user_id.map(|u| u.to_string()),
            checksum: self.compute_reference_checksum(log),
            immutable: true, // Audit logs are immutable
            source: self.source_id.clone(),
        }
    }

    /// Batch convert multiple audit logs
    pub fn convert_audit_batch(&self, logs: &[UpstreamAuditLog]) -> Vec<AuditEntry> {
        logs.iter().map(|l| self.convert_audit_log(l)).collect()
    }

    // --- Private helper methods ---

    fn convert_rule(&self, rule: &UpstreamPolicyRule) -> EscalationRuleCondition {
        EscalationRuleCondition {
            rule_type: rule.rule_type.clone(),
            condition: rule.condition.clone(),
            action: rule.action.clone(),
            parameters: rule.parameters.clone(),
        }
    }

    fn convert_violation_severity(&self, severity: &UpstreamViolationSeverity) -> ViolationSeverity {
        match severity {
            UpstreamViolationSeverity::Low => ViolationSeverity::Low,
            UpstreamViolationSeverity::Medium => ViolationSeverity::Medium,
            UpstreamViolationSeverity::High => ViolationSeverity::High,
            UpstreamViolationSeverity::Critical => ViolationSeverity::Critical,
        }
    }

    fn infer_framework(&self, policy_id: &str) -> ComplianceFramework {
        let id_lower = policy_id.to_lowercase();
        if id_lower.contains("gdpr") {
            ComplianceFramework::GDPR
        } else if id_lower.contains("hipaa") {
            ComplianceFramework::HIPAA
        } else if id_lower.contains("soc2") {
            ComplianceFramework::SOC2
        } else if id_lower.contains("pci") {
            ComplianceFramework::PCI
        } else if id_lower.contains("iso27001") {
            ComplianceFramework::ISO27001
        } else {
            ComplianceFramework::Custom("general".to_string())
        }
    }

    fn infer_priority(&self, rules: &[UpstreamPolicyRule]) -> u32 {
        // Higher priority for blocking or approval-required rules
        let has_blocking = rules.iter().any(|r| r.action == "block");
        let has_approval = rules.iter().any(|r| r.action == "require_approval");

        if has_blocking {
            1
        } else if has_approval {
            2
        } else {
            3
        }
    }

    fn detect_frameworks(&self, rules: &[UpstreamPolicyRule]) -> Vec<String> {
        let mut frameworks = Vec::new();

        for rule in rules {
            if let Some(fw) = rule.parameters.get("framework") {
                if let Some(fw_str) = fw.as_str() {
                    if !frameworks.contains(&fw_str.to_string()) {
                        frameworks.push(fw_str.to_string());
                    }
                }
            }
        }

        if frameworks.is_empty() {
            frameworks.push("general".to_string());
        }

        frameworks
    }

    fn infer_enforcement_level(&self, rules: &[UpstreamPolicyRule]) -> EnforcementLevel {
        if rules.iter().any(|r| r.action == "block") {
            EnforcementLevel::Strict
        } else if rules.iter().any(|r| r.action == "warn") {
            EnforcementLevel::Warning
        } else {
            EnforcementLevel::Monitor
        }
    }

    fn extract_data_classifications(&self, rules: &[UpstreamPolicyRule]) -> Vec<String> {
        rules.iter()
            .filter_map(|r| r.parameters.get("data_classification"))
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect()
    }

    fn extract_retention_period(&self, rules: &[UpstreamPolicyRule]) -> Option<u32> {
        rules.iter()
            .filter_map(|r| r.parameters.get("retention_days"))
            .filter_map(|v| v.as_u64())
            .map(|v| v as u32)
            .next()
    }

    fn is_audit_required(&self, rules: &[UpstreamPolicyRule]) -> bool {
        rules.iter().any(|r| {
            r.parameters.get("audit_required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
    }

    fn generate_remediation(&self, severity: &UpstreamViolationSeverity) -> String {
        match severity {
            UpstreamViolationSeverity::Critical => "Immediate action required. Escalate to security team.".to_string(),
            UpstreamViolationSeverity::High => "High priority remediation needed within 24 hours.".to_string(),
            UpstreamViolationSeverity::Medium => "Address within standard SLA timeframe.".to_string(),
            UpstreamViolationSeverity::Low => "Review and address in next maintenance window.".to_string(),
        }
    }

    fn compute_reference_checksum(&self, log: &UpstreamAuditLog) -> String {
        use sha2::{Sha256, Digest};
        let data = format!("{}:{}:{}:{:?}",
            log.id, log.action, log.resource, log.created_at);
        let hash = Sha256::digest(data.as_bytes());
        format!("{:x}", hash)
    }
}

// --- Upstream Type Definitions ---
// These mirror the governance-dashboard types for adapter compatibility

/// Upstream audit log from governance dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamAuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource: String,
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Upstream policy from governance dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub rules: Vec<UpstreamPolicyRule>,
    pub is_active: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upstream policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamPolicyRule {
    pub id: String,
    pub rule_type: String,
    pub condition: String,
    pub action: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Upstream policy violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamPolicyViolation {
    pub id: String,
    pub policy_id: String,
    pub user_id: String,
    pub severity: UpstreamViolationSeverity,
    pub description: String,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

/// Upstream violation severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Compliance metadata from governance dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamComplianceMetadata {
    pub policy_id: String,
    pub policy_name: String,
    pub frameworks: Vec<String>,
    pub enforcement_level: EnforcementLevel,
    pub data_classifications: Vec<String>,
    pub retention_period_days: Option<u32>,
    pub audit_required: bool,
    pub last_reviewed: DateTime<Utc>,
    pub source: String,
}

/// Enforcement level for policies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementLevel {
    Strict,
    Warning,
    Monitor,
}

/// Audit trail reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamAuditReference {
    pub audit_id: String,
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub resource: String,
    pub actor: Option<String>,
    pub checksum: String,
    pub immutable: bool,
    pub source: String,
}

/// Escalation governance rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamEscalationRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub rules: Vec<EscalationRuleCondition>,
    pub priority: u32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,
}

/// Condition within an escalation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRuleCondition {
    pub rule_type: String,
    pub condition: String,
    pub action: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = GovernanceDashboardAdapter::new("test-source");
        assert_eq!(adapter.source_id, "test-source");
    }

    #[test]
    fn test_default_adapter() {
        let adapter = GovernanceDashboardAdapter::default();
        assert_eq!(adapter.source_id, "llm-governance-dashboard");
    }

    #[test]
    fn test_framework_inference() {
        let adapter = GovernanceDashboardAdapter::default();
        assert!(matches!(adapter.infer_framework("GDPR-001"), ComplianceFramework::GDPR));
        assert!(matches!(adapter.infer_framework("hipaa_policy"), ComplianceFramework::HIPAA));
        assert!(matches!(adapter.infer_framework("custom_policy"), ComplianceFramework::Custom(_)));
    }

    #[test]
    fn test_severity_conversion() {
        let adapter = GovernanceDashboardAdapter::default();
        assert_eq!(
            adapter.convert_violation_severity(&UpstreamViolationSeverity::Critical),
            ViolationSeverity::Critical
        );
        assert_eq!(
            adapter.convert_violation_severity(&UpstreamViolationSeverity::Low),
            ViolationSeverity::Low
        );
    }
}
