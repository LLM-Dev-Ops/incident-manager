//! LLM-Policy-Engine Adapter (Stub)
//!
//! Thin runtime adapter for consuming real-time policy decisions and enforcement
//! constraints from the upstream llm-policy-engine crate.
//!
//! NOTE: The upstream policy-engine crate currently has compile errors in the
//! policy-engine-benchmarks package. This adapter provides the interface and
//! stub implementations that will be activated once the upstream is fixed.
//!
//! The dependency is marked as optional in Cargo.toml to allow the main build
//! to succeed while the upstream issue is being resolved.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Adapter for consuming policy decisions from llm-policy-engine
///
/// This adapter is designed to integrate with the llm-policy-engine once
/// the upstream compile issues are resolved. Currently provides stub
/// implementations for interface compatibility.
#[derive(Debug, Clone)]
pub struct PolicyEngineAdapter {
    /// Source identifier for tracing
    source_id: String,
    /// Default policy decision when upstream is unavailable
    fallback_mode: FallbackMode,
    /// Enable strict enforcement
    strict_mode: bool,
}

impl Default for PolicyEngineAdapter {
    fn default() -> Self {
        Self::new("llm-policy-engine")
    }
}

impl PolicyEngineAdapter {
    /// Create a new policy engine adapter
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            fallback_mode: FallbackMode::Warn,
            strict_mode: false,
        }
    }

    /// Set fallback mode for when upstream is unavailable
    pub fn with_fallback_mode(mut self, mode: FallbackMode) -> Self {
        self.fallback_mode = mode;
        self
    }

    /// Enable strict enforcement mode
    pub fn with_strict_mode(mut self, enabled: bool) -> Self {
        self.strict_mode = enabled;
        self
    }

    /// Check if upstream policy engine is available
    ///
    /// Currently returns false as the upstream has compile errors.
    /// Will be updated when upstream is fixed.
    pub fn is_upstream_available(&self) -> bool {
        // TODO: Implement actual health check when upstream is fixed
        #[cfg(feature = "policy-engine-benchmarks")]
        {
            true
        }
        #[cfg(not(feature = "policy-engine-benchmarks"))]
        {
            false
        }
    }

    /// Evaluate a policy decision request
    ///
    /// When upstream is unavailable, returns a fallback decision based on
    /// the configured fallback mode.
    pub fn evaluate_policy(&self, request: &PolicyEvaluationRequest) -> PolicyDecision {
        if self.is_upstream_available() {
            // TODO: Call upstream policy engine when available
            self.evaluate_with_upstream(request)
        } else {
            self.fallback_decision(request)
        }
    }

    /// Get enforcement constraints for an operation
    pub fn get_enforcement_constraints(&self, operation: &str) -> EnforcementConstraints {
        if self.is_upstream_available() {
            // TODO: Query upstream for constraints
            self.default_constraints(operation)
        } else {
            self.default_constraints(operation)
        }
    }

    /// Batch evaluate multiple policy requests
    pub fn evaluate_batch(&self, requests: &[PolicyEvaluationRequest]) -> Vec<PolicyDecision> {
        requests.iter().map(|r| self.evaluate_policy(r)).collect()
    }

    // --- Private helper methods ---

    fn evaluate_with_upstream(&self, request: &PolicyEvaluationRequest) -> PolicyDecision {
        // Placeholder for upstream integration
        // This will be replaced with actual upstream calls when available
        PolicyDecision {
            request_id: request.request_id.clone(),
            decision: DecisionType::Allow,
            reasoning: "Upstream policy engine integration pending".to_string(),
            constraints: Vec::new(),
            evaluated_at: Utc::now(),
            source: self.source_id.clone(),
            is_fallback: false,
        }
    }

    fn fallback_decision(&self, request: &PolicyEvaluationRequest) -> PolicyDecision {
        let (decision, reasoning) = match self.fallback_mode {
            FallbackMode::Allow => (
                DecisionType::Allow,
                "Fallback: Allowing operation (upstream unavailable)".to_string(),
            ),
            FallbackMode::Deny => (
                DecisionType::Deny,
                "Fallback: Denying operation (upstream unavailable)".to_string(),
            ),
            FallbackMode::Warn => (
                DecisionType::Warn,
                "Fallback: Warning issued (upstream unavailable)".to_string(),
            ),
            FallbackMode::Audit => (
                DecisionType::Allow,
                "Fallback: Allowing with audit log (upstream unavailable)".to_string(),
            ),
        };

        PolicyDecision {
            request_id: request.request_id.clone(),
            decision,
            reasoning,
            constraints: Vec::new(),
            evaluated_at: Utc::now(),
            source: self.source_id.clone(),
            is_fallback: true,
        }
    }

    fn default_constraints(&self, operation: &str) -> EnforcementConstraints {
        EnforcementConstraints {
            operation: operation.to_string(),
            rate_limit: Some(RateLimit {
                requests_per_minute: 60,
                burst_limit: 10,
            }),
            token_limit: Some(TokenLimit {
                max_input_tokens: 4096,
                max_output_tokens: 4096,
                max_total_tokens: 8192,
            }),
            content_restrictions: Vec::new(),
            required_approvals: Vec::new(),
            audit_level: AuditLevel::Standard,
            source: self.source_id.clone(),
        }
    }
}

/// Fallback mode when upstream is unavailable
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackMode {
    /// Allow operations with warning
    Allow,
    /// Deny all operations
    Deny,
    /// Allow with warning logged
    Warn,
    /// Allow with full audit trail
    Audit,
}

/// Policy evaluation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationRequest {
    pub request_id: String,
    pub operation: String,
    pub resource: String,
    pub actor: String,
    pub context: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl PolicyEvaluationRequest {
    pub fn new(operation: impl Into<String>, resource: impl Into<String>, actor: impl Into<String>) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            operation: operation.into(),
            resource: resource.into(),
            actor: actor.into(),
            context: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

/// Policy decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub request_id: String,
    pub decision: DecisionType,
    pub reasoning: String,
    pub constraints: Vec<String>,
    pub evaluated_at: DateTime<Utc>,
    pub source: String,
    pub is_fallback: bool,
}

impl PolicyDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self.decision, DecisionType::Allow)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self.decision, DecisionType::Deny)
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self.decision, DecisionType::RequireApproval)
    }
}

/// Type of policy decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    Allow,
    Deny,
    Warn,
    RequireApproval,
    Modify,
}

/// Enforcement constraints for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementConstraints {
    pub operation: String,
    pub rate_limit: Option<RateLimit>,
    pub token_limit: Option<TokenLimit>,
    pub content_restrictions: Vec<ContentRestriction>,
    pub required_approvals: Vec<String>,
    pub audit_level: AuditLevel,
    pub source: String,
}

/// Rate limiting constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub burst_limit: u32,
}

/// Token usage constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLimit {
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub max_total_tokens: u32,
}

/// Content restriction rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRestriction {
    pub rule_id: String,
    pub rule_type: ContentRuleType,
    pub pattern: Option<String>,
    pub action: RestrictionAction,
}

/// Type of content restriction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentRuleType {
    Regex,
    Keyword,
    Category,
    Custom,
}

/// Action to take on content restriction match
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestrictionAction {
    Block,
    Redact,
    Warn,
    Log,
}

/// Audit level for operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditLevel {
    None,
    Minimal,
    Standard,
    Detailed,
    Full,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = PolicyEngineAdapter::new("test-source");
        assert_eq!(adapter.source_id, "test-source");
    }

    #[test]
    fn test_default_adapter() {
        let adapter = PolicyEngineAdapter::default();
        assert_eq!(adapter.source_id, "llm-policy-engine");
        assert_eq!(adapter.fallback_mode, FallbackMode::Warn);
    }

    #[test]
    fn test_fallback_mode() {
        let adapter = PolicyEngineAdapter::default()
            .with_fallback_mode(FallbackMode::Deny);
        assert_eq!(adapter.fallback_mode, FallbackMode::Deny);
    }

    #[test]
    fn test_policy_request_creation() {
        let request = PolicyEvaluationRequest::new("create", "incident", "user-123")
            .with_context("severity", serde_json::json!("high"));

        assert_eq!(request.operation, "create");
        assert_eq!(request.resource, "incident");
        assert!(request.context.contains_key("severity"));
    }

    #[test]
    fn test_fallback_decision() {
        let adapter = PolicyEngineAdapter::default()
            .with_fallback_mode(FallbackMode::Deny);

        let request = PolicyEvaluationRequest::new("delete", "incident", "user-456");
        let decision = adapter.evaluate_policy(&request);

        assert!(decision.is_denied());
        assert!(decision.is_fallback);
    }

    #[test]
    fn test_decision_helpers() {
        let decision = PolicyDecision {
            request_id: "test".to_string(),
            decision: DecisionType::Allow,
            reasoning: "Test".to_string(),
            constraints: Vec::new(),
            evaluated_at: Utc::now(),
            source: "test".to_string(),
            is_fallback: false,
        };

        assert!(decision.is_allowed());
        assert!(!decision.is_denied());
        assert!(!decision.requires_approval());
    }

    #[test]
    fn test_default_constraints() {
        let adapter = PolicyEngineAdapter::default();
        let constraints = adapter.get_enforcement_constraints("create_incident");

        assert_eq!(constraints.operation, "create_incident");
        assert!(constraints.rate_limit.is_some());
        assert!(constraints.token_limit.is_some());
    }
}
