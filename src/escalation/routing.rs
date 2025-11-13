use crate::error::{AppError, Result};
use crate::models::policy::{ConditionOperator, RoutingAction, RoutingRule, RuleCondition};
use crate::models::Incident;
use crate::playbooks::PlaybookService;
use dashmap::DashMap;
use regex::Regex;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Evaluates routing rules and executes actions
pub struct RoutingRuleEvaluator {
    /// Routing rules indexed by priority (higher priority first)
    rules: Arc<DashMap<Uuid, RoutingRule>>,

    /// Optional playbook service for ApplyPlaybook actions
    playbook_service: Option<Arc<PlaybookService>>,
}

impl RoutingRuleEvaluator {
    pub fn new(playbook_service: Option<Arc<PlaybookService>>) -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
            playbook_service,
        }
    }

    /// Register a routing rule
    pub fn register_rule(&self, rule: RoutingRule) -> Result<()> {
        if rule.conditions.is_empty() {
            return Err(AppError::Validation(
                "Routing rule must have at least one condition".to_string(),
            ));
        }

        if rule.actions.is_empty() {
            return Err(AppError::Validation(
                "Routing rule must have at least one action".to_string(),
            ));
        }

        tracing::info!(
            rule_id = %rule.id,
            rule_name = %rule.name,
            priority = rule.priority,
            "Registered routing rule"
        );

        self.rules.insert(rule.id, rule);
        Ok(())
    }

    /// Remove a routing rule
    pub fn remove_rule(&self, rule_id: &Uuid) -> Option<RoutingRule> {
        self.rules.remove(rule_id).map(|(_, rule)| rule)
    }

    /// Get a routing rule by ID
    pub fn get_rule(&self, rule_id: &Uuid) -> Option<RoutingRule> {
        self.rules.get(rule_id).map(|e| e.value().clone())
    }

    /// List all routing rules
    pub fn list_rules(&self) -> Vec<RoutingRule> {
        let mut rules: Vec<_> = self.rules.iter().map(|e| e.value().clone()).collect();
        // Sort by priority (higher first)
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }

    /// Evaluate all matching rules for an incident and return recommended actions
    pub fn evaluate_incident(&self, incident: &Incident) -> Vec<RoutingRuleMatch> {
        let mut matches = Vec::new();

        // Get rules sorted by priority
        let rules = self.list_rules();

        for rule in rules {
            if !rule.enabled {
                continue;
            }

            if self.evaluate_rule(&rule, incident) {
                matches.push(RoutingRuleMatch {
                    rule_id: rule.id,
                    rule_name: rule.name.clone(),
                    priority: rule.priority,
                    actions: rule.actions.clone(),
                });

                tracing::info!(
                    incident_id = %incident.id,
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Routing rule matched"
                );
            }
        }

        matches
    }

    /// Evaluate a single rule against an incident
    fn evaluate_rule(&self, rule: &RoutingRule, incident: &Incident) -> bool {
        // All conditions must be true (AND logic)
        for condition in &rule.conditions {
            if !self.evaluate_condition(condition, incident) {
                return false;
            }
        }
        true
    }

    /// Evaluate a single condition
    fn evaluate_condition(&self, condition: &RuleCondition, incident: &Incident) -> bool {
        let incident_value = self.get_incident_field_value(&condition.field, incident);

        match &condition.operator {
            ConditionOperator::Equals => {
                self.compare_values(&incident_value, &condition.value, |a, b| a == b)
            }
            ConditionOperator::NotEquals => {
                self.compare_values(&incident_value, &condition.value, |a, b| a != b)
            }
            ConditionOperator::Contains => self.check_contains(&incident_value, &condition.value),
            ConditionOperator::NotContains => {
                !self.check_contains(&incident_value, &condition.value)
            }
            ConditionOperator::GreaterThan => {
                self.compare_numbers(&incident_value, &condition.value, |a, b| a > b)
            }
            ConditionOperator::LessThan => {
                self.compare_numbers(&incident_value, &condition.value, |a, b| a < b)
            }
            ConditionOperator::In => self.check_in(&incident_value, &condition.value),
            ConditionOperator::NotIn => !self.check_in(&incident_value, &condition.value),
            ConditionOperator::Matches => self.check_regex(&incident_value, &condition.value),
        }
    }

    /// Get the value of a field from an incident
    fn get_incident_field_value(&self, field: &str, incident: &Incident) -> JsonValue {
        match field {
            "id" => JsonValue::String(incident.id.to_string()),
            "source" => JsonValue::String(incident.source.clone()),
            "title" => JsonValue::String(incident.title.clone()),
            "description" => JsonValue::String(incident.description.clone()),
            "severity" => JsonValue::String(format!("{:?}", incident.severity)),
            "state" => JsonValue::String(format!("{:?}", incident.state)),
            "incident_type" => JsonValue::String(format!("{:?}", incident.incident_type)),
            "priority_score" => JsonValue::Number(serde_json::Number::from(incident.severity.priority())),
            "assignees" => JsonValue::Array(
                incident
                    .assignees
                    .iter()
                    .map(|a| JsonValue::String(a.clone()))
                    .collect(),
            ),
            _ => {
                // Check labels
                if let Some(stripped) = field.strip_prefix("labels.") {
                    if let Some(value) = incident.labels.get(stripped) {
                        return JsonValue::String(value.clone());
                    }
                }
                JsonValue::Null
            }
        }
    }

    /// Compare two values with a comparison function
    fn compare_values<F>(&self, a: &JsonValue, b: &JsonValue, cmp: F) -> bool
    where
        F: Fn(&JsonValue, &JsonValue) -> bool,
    {
        cmp(a, b)
    }

    /// Compare numeric values
    fn compare_numbers<F>(&self, a: &JsonValue, b: &JsonValue, cmp: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        match (a.as_f64(), b.as_f64()) {
            (Some(av), Some(bv)) => cmp(av, bv),
            _ => false,
        }
    }

    /// Check if value contains substring
    fn check_contains(&self, haystack: &JsonValue, needle: &JsonValue) -> bool {
        match (haystack.as_str(), needle.as_str()) {
            (Some(h), Some(n)) => h.contains(n),
            _ => false,
        }
    }

    /// Check if value is in array
    fn check_in(&self, value: &JsonValue, array: &JsonValue) -> bool {
        if let Some(arr) = array.as_array() {
            arr.contains(value)
        } else {
            false
        }
    }

    /// Check if value matches regex pattern
    fn check_regex(&self, value: &JsonValue, pattern: &JsonValue) -> bool {
        match (value.as_str(), pattern.as_str()) {
            (Some(v), Some(p)) => {
                if let Ok(regex) = Regex::new(p) {
                    regex.is_match(v)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Apply actions from matched rules to an incident
    pub async fn apply_actions(
        &self,
        incident: &Incident,
        matches: &[RoutingRuleMatch],
    ) -> Result<RoutingActionResult> {
        let mut result = RoutingActionResult {
            actions_applied: 0,
            actions_failed: 0,
            suggested_assignees: Vec::new(),
            suggested_labels: HashMap::new(),
            suggested_severity: None,
            notifications: Vec::new(),
            playbooks_to_execute: Vec::new(),
            suppress_for_minutes: None,
        };

        for routing_match in matches {
            for action in &routing_match.actions {
                match self.apply_action(action, incident, &mut result).await {
                    Ok(()) => result.actions_applied += 1,
                    Err(e) => {
                        result.actions_failed += 1;
                        tracing::error!(
                            incident_id = %incident.id,
                            action = ?action,
                            error = %e,
                            "Failed to apply routing action"
                        );
                    }
                }
            }
        }

        Ok(result)
    }

    /// Apply a single action
    async fn apply_action(
        &self,
        action: &RoutingAction,
        incident: &Incident,
        result: &mut RoutingActionResult,
    ) -> Result<()> {
        match action {
            RoutingAction::Notify { channels } => {
                result.notifications.extend(channels.clone());
                tracing::info!(
                    incident_id = %incident.id,
                    channels = ?channels,
                    "Routing action: Notify"
                );
                Ok(())
            }
            RoutingAction::Assign { assignees } => {
                result.suggested_assignees.extend(assignees.clone());
                tracing::info!(
                    incident_id = %incident.id,
                    assignees = ?assignees,
                    "Routing action: Assign"
                );
                Ok(())
            }
            RoutingAction::ApplyPlaybook { playbook_id } => {
                result.playbooks_to_execute.push(*playbook_id);
                tracing::info!(
                    incident_id = %incident.id,
                    playbook_id = %playbook_id,
                    "Routing action: Apply playbook"
                );
                Ok(())
            }
            RoutingAction::AddLabels { labels } => {
                result.suggested_labels.extend(labels.clone());
                tracing::info!(
                    incident_id = %incident.id,
                    labels = ?labels,
                    "Routing action: Add labels"
                );
                Ok(())
            }
            RoutingAction::SetSeverity { severity } => {
                result.suggested_severity = Some(*severity);
                tracing::info!(
                    incident_id = %incident.id,
                    severity = ?severity,
                    "Routing action: Set severity"
                );
                Ok(())
            }
            RoutingAction::Suppress { duration_minutes } => {
                result.suppress_for_minutes = Some(*duration_minutes);
                tracing::info!(
                    incident_id = %incident.id,
                    duration = duration_minutes,
                    "Routing action: Suppress"
                );
                Ok(())
            }
        }
    }

    /// Get statistics about routing rules
    pub fn get_stats(&self) -> RoutingStats {
        let total = self.rules.len();
        let enabled = self.rules.iter().filter(|e| e.value().enabled).count();

        RoutingStats {
            total_rules: total,
            enabled_rules: enabled,
        }
    }
}

/// A matched routing rule
#[derive(Debug, Clone)]
pub struct RoutingRuleMatch {
    pub rule_id: Uuid,
    pub rule_name: String,
    pub priority: u32,
    pub actions: Vec<RoutingAction>,
}

/// Result of applying routing actions
#[derive(Debug, Clone)]
pub struct RoutingActionResult {
    pub actions_applied: usize,
    pub actions_failed: usize,
    pub suggested_assignees: Vec<String>,
    pub suggested_labels: HashMap<String, String>,
    pub suggested_severity: Option<crate::models::Severity>,
    pub notifications: Vec<String>,
    pub playbooks_to_execute: Vec<Uuid>,
    pub suppress_for_minutes: Option<u32>,
}

/// Routing statistics
#[derive(Debug, Clone)]
pub struct RoutingStats {
    pub total_rules: usize,
    pub enabled_rules: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};

    fn create_test_incident() -> Incident {
        let mut incident = Incident::new(
            "monitoring".to_string(),
            "High CPU Usage".to_string(),
            "CPU usage exceeded 90%".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );
        incident.labels.insert("team".to_string(), "platform".to_string());
        incident
    }

    #[tokio::test]
    async fn test_register_rule() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "Security Alerts".to_string(),
            priority: 100,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "incident_type".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("Security".to_string()),
            }],
            actions: vec![RoutingAction::Notify {
                channels: vec!["#security".to_string()],
            }],
        };

        let rule_id = rule.id;
        evaluator.register_rule(rule).unwrap();

        let retrieved = evaluator.get_rule(&rule_id);
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_evaluate_equals_condition() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "Infrastructure Incidents".to_string(),
            priority: 50,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "incident_type".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("Infrastructure".to_string()),
            }],
            actions: vec![RoutingAction::Assign {
                assignees: vec!["platform-team@example.com".to_string()],
            }],
        };

        evaluator.register_rule(rule).unwrap();

        let incident = create_test_incident();
        let matches = evaluator.evaluate_incident(&incident);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule_name, "Infrastructure Incidents");
    }

    #[tokio::test]
    async fn test_evaluate_contains_condition() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "CPU Alerts".to_string(),
            priority: 50,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "title".to_string(),
                operator: ConditionOperator::Contains,
                value: JsonValue::String("CPU".to_string()),
            }],
            actions: vec![RoutingAction::Notify {
                channels: vec!["#ops".to_string()],
            }],
        };

        evaluator.register_rule(rule).unwrap();

        let incident = create_test_incident();
        let matches = evaluator.evaluate_incident(&incident);

        assert_eq!(matches.len(), 1);
    }

    #[tokio::test]
    async fn test_evaluate_severity_condition() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "Critical P1 Incidents".to_string(),
            priority: 100,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "severity".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("P1".to_string()),
            }],
            actions: vec![RoutingAction::Notify {
                channels: vec!["#incidents".to_string()],
            }],
        };

        evaluator.register_rule(rule).unwrap();

        let incident = create_test_incident();
        let matches = evaluator.evaluate_incident(&incident);

        assert_eq!(matches.len(), 1);
    }

    #[tokio::test]
    async fn test_evaluate_label_condition() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "Platform Team Incidents".to_string(),
            priority: 50,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "labels.team".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("platform".to_string()),
            }],
            actions: vec![RoutingAction::Assign {
                assignees: vec!["platform@example.com".to_string()],
            }],
        };

        evaluator.register_rule(rule).unwrap();

        let incident = create_test_incident();
        let matches = evaluator.evaluate_incident(&incident);

        assert_eq!(matches.len(), 1);
    }

    #[tokio::test]
    async fn test_multiple_conditions() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "Critical Infrastructure".to_string(),
            priority: 100,
            enabled: true,
            conditions: vec![
                RuleCondition {
                    field: "incident_type".to_string(),
                    operator: ConditionOperator::Equals,
                    value: JsonValue::String("Infrastructure".to_string()),
                },
                RuleCondition {
                    field: "severity".to_string(),
                    operator: ConditionOperator::Equals,
                    value: JsonValue::String("P1".to_string()),
                },
            ],
            actions: vec![RoutingAction::Notify {
                channels: vec!["#critical".to_string()],
            }],
        };

        evaluator.register_rule(rule).unwrap();

        let incident = create_test_incident();
        let matches = evaluator.evaluate_incident(&incident);

        assert_eq!(matches.len(), 1);
    }

    #[tokio::test]
    async fn test_disabled_rule_not_evaluated() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "Disabled Rule".to_string(),
            priority: 50,
            enabled: false, // Disabled
            conditions: vec![RuleCondition {
                field: "incident_type".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("Infrastructure".to_string()),
            }],
            actions: vec![],
        };

        evaluator.register_rule(rule).unwrap();

        let incident = create_test_incident();
        let matches = evaluator.evaluate_incident(&incident);

        assert_eq!(matches.len(), 0);
    }

    #[tokio::test]
    async fn test_apply_actions() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let matches = vec![RoutingRuleMatch {
            rule_id: Uuid::new_v4(),
            rule_name: "Test Rule".to_string(),
            priority: 100,
            actions: vec![
                RoutingAction::Notify {
                    channels: vec!["#ops".to_string()],
                },
                RoutingAction::Assign {
                    assignees: vec!["oncall@example.com".to_string()],
                },
                RoutingAction::AddLabels {
                    labels: {
                        let mut labels = HashMap::new();
                        labels.insert("automated".to_string(), "true".to_string());
                        labels
                    },
                },
            ],
        }];

        let incident = create_test_incident();
        let result = evaluator.apply_actions(&incident, &matches).await.unwrap();

        assert_eq!(result.actions_applied, 3);
        assert_eq!(result.notifications.len(), 1);
        assert_eq!(result.suggested_assignees.len(), 1);
        assert_eq!(result.suggested_labels.len(), 1);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let evaluator = RoutingRuleEvaluator::new(None);

        // Register rules with different priorities
        let rule1 = RoutingRule {
            id: Uuid::new_v4(),
            name: "Low Priority".to_string(),
            priority: 10,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "incident_type".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("Infrastructure".to_string()),
            }],
            actions: vec![],
        };

        let rule2 = RoutingRule {
            id: Uuid::new_v4(),
            name: "High Priority".to_string(),
            priority: 100,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "incident_type".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("Infrastructure".to_string()),
            }],
            actions: vec![],
        };

        evaluator.register_rule(rule1).unwrap();
        evaluator.register_rule(rule2).unwrap();

        let incident = create_test_incident();
        let matches = evaluator.evaluate_incident(&incident);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].rule_name, "High Priority"); // Higher priority first
        assert_eq!(matches[1].rule_name, "Low Priority");
    }

    #[test]
    fn test_get_stats() {
        let evaluator = RoutingRuleEvaluator::new(None);

        let rule1 = RoutingRule {
            id: Uuid::new_v4(),
            name: "Rule 1".to_string(),
            priority: 100,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "severity".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("P0".to_string()),
            }],
            actions: vec![],
        };

        let rule2 = RoutingRule {
            id: Uuid::new_v4(),
            name: "Rule 2".to_string(),
            priority: 50,
            enabled: false,
            conditions: vec![RuleCondition {
                field: "severity".to_string(),
                operator: ConditionOperator::Equals,
                value: JsonValue::String("P1".to_string()),
            }],
            actions: vec![],
        };

        evaluator.register_rule(rule1).unwrap();
        evaluator.register_rule(rule2).unwrap();

        let stats = evaluator.get_stats();
        assert_eq!(stats.total_rules, 2);
        assert_eq!(stats.enabled_rules, 1);
    }
}
