//! GraphQL types for playbooks

use async_graphql::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models;
use super::common::DateTimeScalar;

/// Playbook object type
#[derive(Clone)]
pub struct Playbook(pub models::Playbook);

#[Object]
impl Playbook {
    async fn id(&self) -> &Uuid {
        &self.0.id
    }

    async fn name(&self) -> &str {
        &self.0.name
    }

    async fn version(&self) -> &str {
        &self.0.version
    }

    async fn description(&self) -> &str {
        &self.0.description
    }

    async fn owner(&self) -> &str {
        &self.0.owner
    }

    async fn created_at(&self) -> DateTimeScalar {
        self.0.created_at.into()
    }

    async fn updated_at(&self) -> DateTimeScalar {
        self.0.updated_at.into()
    }

    async fn triggers(&self) -> PlaybookTriggers {
        PlaybookTriggers(self.0.triggers.clone())
    }

    async fn variables(&self) -> Vec<super::incident::Label> {
        self.0
            .variables
            .iter()
            .map(|(k, v)| super::incident::Label {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }

    async fn steps(&self) -> Vec<PlaybookStep> {
        self.0.steps.iter().map(|s| PlaybookStep(s.clone())).collect()
    }

    async fn enabled(&self) -> bool {
        self.0.enabled
    }

    async fn tags(&self) -> &[String] {
        &self.0.tags
    }
}

/// Playbook triggers
#[derive(Clone)]
pub struct PlaybookTriggers(pub models::PlaybookTriggers);

#[Object]
impl PlaybookTriggers {
    async fn severity_trigger(&self) -> Vec<super::incident::Severity> {
        self.0
            .severity_trigger
            .iter()
            .map(|s| super::incident::Severity::from(*s))
            .collect()
    }

    async fn type_trigger(&self) -> Vec<super::incident::IncidentType> {
        self.0
            .type_trigger
            .iter()
            .map(|t| super::incident::IncidentType::from(t.clone()))
            .collect()
    }

    async fn source_trigger(&self) -> &[String] {
        &self.0.source_trigger
    }
}

/// Playbook step
#[derive(Clone)]
pub struct PlaybookStep(pub models::PlaybookStep);

#[Object]
impl PlaybookStep {
    async fn id(&self) -> &str {
        &self.0.id
    }

    async fn step_type(&self) -> StepType {
        StepType::from(self.0.step_type.clone())
    }

    async fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    async fn actions(&self) -> Vec<Action> {
        self.0.actions.iter().map(|a| Action(a.clone())).collect()
    }

    async fn parallel(&self) -> bool {
        self.0.parallel
    }

    async fn timeout(&self) -> Option<&str> {
        self.0.timeout.as_deref()
    }

    async fn retry(&self) -> u32 {
        self.0.retry
    }

    async fn backoff(&self) -> BackoffStrategy {
        BackoffStrategy::from(self.0.backoff.clone())
    }

    async fn condition(&self) -> Option<&str> {
        self.0.condition.as_deref()
    }
}

/// Step type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum StepType {
    Notification,
    DataCollection,
    Remediation,
    Escalation,
    Resolution,
    Custom,
}

impl From<models::StepType> for StepType {
    fn from(step_type: models::StepType) -> Self {
        match step_type {
            models::StepType::Notification => StepType::Notification,
            models::StepType::DataCollection => StepType::DataCollection,
            models::StepType::Remediation => StepType::Remediation,
            models::StepType::Escalation => StepType::Escalation,
            models::StepType::Resolution => StepType::Resolution,
            models::StepType::Custom => StepType::Custom,
        }
    }
}

/// Action
#[derive(Clone)]
pub struct Action(pub models::Action);

#[Object]
impl Action {
    async fn action_type(&self) -> ActionType {
        ActionType::from(self.0.action_type.clone())
    }

    async fn parameters(&self) -> HashMap<String, serde_json::Value> {
        self.0.parameters.clone()
    }

    async fn on_success(&self) -> Option<&str> {
        self.0.on_success.as_deref()
    }

    async fn on_failure(&self) -> Option<&str> {
        self.0.on_failure.as_deref()
    }
}

/// Action type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
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

impl From<models::ActionType> for ActionType {
    fn from(action_type: models::ActionType) -> Self {
        match action_type {
            models::ActionType::Slack => ActionType::Slack,
            models::ActionType::Email => ActionType::Email,
            models::ActionType::Pagerduty => ActionType::Pagerduty,
            models::ActionType::Webhook => ActionType::Webhook,
            models::ActionType::MetricsSnapshot => ActionType::MetricsSnapshot,
            models::ActionType::LogsCapture => ActionType::LogsCapture,
            models::ActionType::HealthCheck => ActionType::HealthCheck,
            models::ActionType::ServiceRestart => ActionType::ServiceRestart,
            models::ActionType::ServiceRollback => ActionType::ServiceRollback,
            models::ActionType::ScaleHorizontal => ActionType::ScaleHorizontal,
            models::ActionType::ScaleVertical => ActionType::ScaleVertical,
            models::ActionType::ConfigChange => ActionType::ConfigChange,
            models::ActionType::CircuitBreaker => ActionType::CircuitBreaker,
            models::ActionType::Wait => ActionType::Wait,
            models::ActionType::VerifyResolution => ActionType::VerifyResolution,
            models::ActionType::CreateWarRoom => ActionType::CreateWarRoom,
            models::ActionType::SchedulePostmortem => ActionType::SchedulePostmortem,
            models::ActionType::IncidentResolve => ActionType::IncidentResolve,
            models::ActionType::SeverityIncrease => ActionType::SeverityIncrease,
            models::ActionType::SeverityDecrease => ActionType::SeverityDecrease,
            models::ActionType::HttpRequest => ActionType::HttpRequest,
            models::ActionType::RunScript => ActionType::RunScript,
        }
    }
}

/// Backoff strategy enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum BackoffStrategy {
    Linear,
    Exponential,
    Fixed,
}

impl From<models::BackoffStrategy> for BackoffStrategy {
    fn from(strategy: models::BackoffStrategy) -> Self {
        match strategy {
            models::BackoffStrategy::Linear => BackoffStrategy::Linear,
            models::BackoffStrategy::Exponential => BackoffStrategy::Exponential,
            models::BackoffStrategy::Fixed => BackoffStrategy::Fixed,
        }
    }
}

/// Playbook execution
#[derive(Clone)]
pub struct PlaybookExecution(pub models::PlaybookExecution);

#[Object]
impl PlaybookExecution {
    async fn id(&self) -> &Uuid {
        &self.0.id
    }

    async fn playbook_id(&self) -> &Uuid {
        &self.0.playbook_id
    }

    async fn incident_id(&self) -> &Uuid {
        &self.0.incident_id
    }

    async fn started_at(&self) -> DateTimeScalar {
        self.0.started_at.into()
    }

    async fn completed_at(&self) -> Option<DateTimeScalar> {
        self.0.completed_at.map(|dt| dt.into())
    }

    async fn status(&self) -> ExecutionStatus {
        ExecutionStatus::from(self.0.status.clone())
    }

    async fn current_step(&self) -> Option<&str> {
        self.0.current_step.as_deref()
    }

    async fn step_results(&self) -> Vec<StepResultEntry> {
        self.0
            .step_results
            .iter()
            .map(|(step_id, result)| StepResultEntry {
                step_id: step_id.clone(),
                result: StepResult(result.clone()),
            })
            .collect()
    }

    async fn error(&self) -> Option<&str> {
        self.0.error.as_deref()
    }
}

/// Execution status enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl From<models::ExecutionStatus> for ExecutionStatus {
    fn from(status: models::ExecutionStatus) -> Self {
        match status {
            models::ExecutionStatus::Running => ExecutionStatus::Running,
            models::ExecutionStatus::Completed => ExecutionStatus::Completed,
            models::ExecutionStatus::Failed => ExecutionStatus::Failed,
            models::ExecutionStatus::Cancelled => ExecutionStatus::Cancelled,
        }
    }
}

/// Step result entry
#[derive(SimpleObject)]
pub struct StepResultEntry {
    pub step_id: String,
    pub result: StepResult,
}

/// Step result
#[derive(Clone)]
pub struct StepResult(pub models::StepResult);

#[Object]
impl StepResult {
    async fn step_id(&self) -> &str {
        &self.0.step_id
    }

    async fn started_at(&self) -> DateTimeScalar {
        self.0.started_at.into()
    }

    async fn completed_at(&self) -> Option<DateTimeScalar> {
        self.0.completed_at.map(|dt| dt.into())
    }

    async fn status(&self) -> ExecutionStatus {
        ExecutionStatus::from(self.0.status.clone())
    }

    async fn output(&self) -> HashMap<String, serde_json::Value> {
        self.0.output.clone()
    }

    async fn error(&self) -> Option<&str> {
        self.0.error.as_deref()
    }
}
