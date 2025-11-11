use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::incident::Severity;

/// Escalation policy defines how and when to escalate incidents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    /// Escalation levels
    pub levels: Vec<EscalationLevel>,

    /// Repeat escalation if unacknowledged
    pub repeat: Option<RepeatConfig>,

    /// Which severities this policy applies to
    pub severity_filter: Vec<Severity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    /// Level number (0-indexed)
    pub level: u32,

    /// Delay before escalating to this level
    pub delay_minutes: u32,

    /// Targets to notify at this level
    pub targets: Vec<EscalationTarget>,

    /// Stop escalation if acknowledged at this level
    #[serde(default = "default_true")]
    pub stop_on_ack: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EscalationTarget {
    User { email: String },
    Team { team_id: String },
    Schedule { schedule_id: String },
    Webhook { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatConfig {
    /// Maximum number of times to repeat
    pub max_repeats: u32,

    /// Interval between repeats (minutes)
    pub interval_minutes: u32,
}

/// Notification routing rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: Uuid,
    pub name: String,
    pub priority: u32,
    pub enabled: bool,

    /// Conditions that must be met for this rule to apply
    pub conditions: Vec<RuleCondition>,

    /// Actions to take when conditions are met
    pub actions: Vec<RoutingAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    In,
    NotIn,
    Matches, // Regex match
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RoutingAction {
    Notify {
        channels: Vec<String>,
    },
    Assign {
        assignees: Vec<String>,
    },
    ApplyPlaybook {
        playbook_id: Uuid,
    },
    AddLabels {
        labels: HashMap<String, String>,
    },
    SetSeverity {
        severity: Severity,
    },
    Suppress {
        duration_minutes: u32,
    },
}

/// On-call schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallSchedule {
    pub id: Uuid,
    pub name: String,
    pub timezone: String,

    /// Schedule layers (primary, secondary, etc.)
    pub layers: Vec<ScheduleLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleLayer {
    pub name: String,
    pub users: Vec<String>,

    /// Rotation strategy
    pub rotation: RotationStrategy,

    /// When this layer is active
    pub restrictions: Option<TimeRestrictions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RotationStrategy {
    Daily { handoff_hour: u32 },
    Weekly { handoff_day: String, handoff_hour: u32 },
    Custom { duration_hours: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    /// Days of week (0 = Sunday, 6 = Saturday)
    pub days_of_week: Vec<u8>,

    /// Start hour (0-23)
    pub start_hour: u32,

    /// End hour (0-23)
    pub end_hour: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escalation_policy_creation() {
        let policy = EscalationPolicy {
            id: Uuid::new_v4(),
            name: "Standard Escalation".to_string(),
            description: "Standard escalation path".to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            levels: vec![
                EscalationLevel {
                    level: 0,
                    delay_minutes: 0,
                    targets: vec![EscalationTarget::User {
                        email: "oncall@example.com".to_string(),
                    }],
                    stop_on_ack: true,
                },
                EscalationLevel {
                    level: 1,
                    delay_minutes: 5,
                    targets: vec![EscalationTarget::Team {
                        team_id: "platform-team".to_string(),
                    }],
                    stop_on_ack: true,
                },
            ],
            repeat: Some(RepeatConfig {
                max_repeats: 3,
                interval_minutes: 15,
            }),
            severity_filter: vec![Severity::P0, Severity::P1],
        };

        assert_eq!(policy.levels.len(), 2);
        assert_eq!(policy.levels[0].delay_minutes, 0);
        assert_eq!(policy.levels[1].delay_minutes, 5);
        assert!(policy.repeat.is_some());
    }

    #[test]
    fn test_routing_rule() {
        let rule = RoutingRule {
            id: Uuid::new_v4(),
            name: "Route Security Incidents".to_string(),
            priority: 100,
            enabled: true,
            conditions: vec![RuleCondition {
                field: "incident_type".to_string(),
                operator: ConditionOperator::Equals,
                value: serde_json::json!("Security"),
            }],
            actions: vec![
                RoutingAction::Notify {
                    channels: vec!["#security-team".to_string()],
                },
                RoutingAction::Assign {
                    assignees: vec!["security-oncall@example.com".to_string()],
                },
            ],
        };

        assert_eq!(rule.conditions.len(), 1);
        assert_eq!(rule.actions.len(), 2);
        assert_eq!(rule.conditions[0].operator, ConditionOperator::Equals);
    }
}
