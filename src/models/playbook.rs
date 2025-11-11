use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::incident::{IncidentType, Severity};

/// Playbook defines automated response workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    /// Triggers that activate this playbook
    pub triggers: PlaybookTriggers,

    /// Variables used in the playbook
    #[serde(default)]
    pub variables: HashMap<String, String>,

    /// Sequence of steps to execute
    pub steps: Vec<PlaybookStep>,

    /// Whether the playbook is enabled
    pub enabled: bool,

    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Playbook {
    /// Check if this playbook should be triggered for an incident
    pub fn matches_incident(&self, severity: &Severity, incident_type: &IncidentType) -> bool {
        let severity_match = self.triggers.severity_trigger.is_empty()
            || self.triggers.severity_trigger.contains(severity);

        let type_match = self.triggers.type_trigger.is_empty()
            || self.triggers.type_trigger.contains(incident_type);

        self.enabled && severity_match && type_match
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookTriggers {
    /// Severity levels that trigger this playbook
    #[serde(default)]
    pub severity_trigger: Vec<Severity>,

    /// Incident types that trigger this playbook
    #[serde(default)]
    pub type_trigger: Vec<IncidentType>,

    /// Source systems that trigger this playbook
    #[serde(default)]
    pub source_trigger: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookStep {
    pub id: String,
    pub step_type: StepType,
    pub description: Option<String>,

    /// Actions to execute in this step
    pub actions: Vec<Action>,

    /// Whether actions should run in parallel
    #[serde(default)]
    pub parallel: bool,

    /// Maximum time to wait for step completion
    pub timeout: Option<String>,

    /// Number of retries on failure
    #[serde(default)]
    pub retry: u32,

    /// Retry backoff strategy
    #[serde(default)]
    pub backoff: BackoffStrategy,

    /// Condition to execute this step
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Notification,
    DataCollection,
    Remediation,
    Escalation,
    Resolution,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub on_success: Option<String>,
    pub on_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    // Notification actions
    Slack,
    Email,
    Pagerduty,
    Webhook,

    // Data collection
    MetricsSnapshot,
    LogsCapture,
    HealthCheck,

    // Remediation actions
    ServiceRestart,
    ServiceRollback,
    ScaleHorizontal,
    ScaleVertical,
    ConfigChange,
    CircuitBreaker,

    // Workflow control
    Wait,
    VerifyResolution,
    CreateWarRoom,
    SchedulePostmortem,

    // Incident management
    IncidentResolve,
    SeverityIncrease,
    SeverityDecrease,

    // Generic
    HttpRequest,
    RunScript,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    Linear,
    Exponential,
    Fixed,
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        BackoffStrategy::Exponential
    }
}

/// Playbook execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookExecution {
    pub id: Uuid,
    pub playbook_id: Uuid,
    pub incident_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub current_step: Option<String>,
    pub step_results: HashMap<String, StepResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub output: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playbook_matching() {
        let playbook = Playbook {
            id: Uuid::new_v4(),
            name: "Critical Infrastructure Response".to_string(),
            version: "1.0.0".to_string(),
            description: "Handles critical infrastructure failures".to_string(),
            owner: "platform-team".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            triggers: PlaybookTriggers {
                severity_trigger: vec![Severity::P0, Severity::P1],
                type_trigger: vec![IncidentType::Infrastructure],
                source_trigger: vec![],
            },
            variables: HashMap::new(),
            steps: vec![],
            enabled: true,
            tags: vec!["critical".to_string()],
        };

        assert!(playbook.matches_incident(&Severity::P0, &IncidentType::Infrastructure));
        assert!(playbook.matches_incident(&Severity::P1, &IncidentType::Infrastructure));
        assert!(!playbook.matches_incident(&Severity::P2, &IncidentType::Infrastructure));
        assert!(!playbook.matches_incident(&Severity::P0, &IncidentType::Application));
    }

    #[test]
    fn test_disabled_playbook() {
        let mut playbook = Playbook {
            id: Uuid::new_v4(),
            name: "Test Playbook".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            owner: "test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            triggers: PlaybookTriggers {
                severity_trigger: vec![Severity::P0],
                type_trigger: vec![],
                source_trigger: vec![],
            },
            variables: HashMap::new(),
            steps: vec![],
            enabled: false, // Disabled
            tags: vec![],
        };

        assert!(!playbook.matches_incident(&Severity::P0, &IncidentType::Infrastructure));

        playbook.enabled = true;
        assert!(playbook.matches_incident(&Severity::P0, &IncidentType::Infrastructure));
    }
}
