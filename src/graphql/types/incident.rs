//! GraphQL types for incidents

use async_graphql::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models;
use super::common::{DateTimeScalar, PageInfo, SortOrder};
use crate::graphql::context::GraphQLContext;

/// Incident object type
#[derive(Clone)]
pub struct Incident(pub models::Incident);

#[Object]
impl Incident {
    /// Unique identifier
    async fn id(&self) -> &Uuid {
        &self.0.id
    }

    /// Creation timestamp
    async fn created_at(&self) -> DateTimeScalar {
        self.0.created_at.into()
    }

    /// Last update timestamp
    async fn updated_at(&self) -> DateTimeScalar {
        self.0.updated_at.into()
    }

    /// Current state
    async fn state(&self) -> IncidentState {
        IncidentState::from(self.0.state.clone())
    }

    /// Severity level
    async fn severity(&self) -> Severity {
        Severity::from(self.0.severity)
    }

    /// Incident type
    async fn incident_type(&self) -> IncidentType {
        IncidentType::from(self.0.incident_type.clone())
    }

    /// Source system
    async fn source(&self) -> &str {
        &self.0.source
    }

    /// Human-readable title
    async fn title(&self) -> &str {
        &self.0.title
    }

    /// Detailed description
    async fn description(&self) -> &str {
        &self.0.description
    }

    /// Affected services/resources
    async fn affected_resources(&self) -> &[String] {
        &self.0.affected_resources
    }

    /// Custom labels
    async fn labels(&self) -> Vec<Label> {
        self.0
            .labels
            .iter()
            .map(|(k, v)| Label {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }

    /// Related incidents
    async fn related_incidents(&self, ctx: &Context<'_>) -> Result<Vec<Incident>> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Use DataLoader to batch load related incidents
        let incidents = gql_ctx
            .related_incidents_loader
            .load_one(self.0.id)
            .await
            .map_err(|e| Error::new(format!("Failed to load related incidents: {}", e)))?
            .unwrap_or_default();

        Ok(incidents.into_iter().map(Incident).collect())
    }

    /// Current playbook (if any)
    async fn active_playbook(&self, ctx: &Context<'_>) -> Result<Option<super::Playbook>> {
        if let Some(playbook_id) = self.0.active_playbook {
            let gql_ctx = ctx.data::<GraphQLContext>()?;

            let playbook = gql_ctx
                .playbook_loader
                .load_one(playbook_id)
                .await
                .map_err(|e| Error::new(format!("Failed to load playbook: {}", e)))?;

            Ok(playbook.map(super::Playbook))
        } else {
            Ok(None)
        }
    }

    /// Resolution details
    async fn resolution(&self) -> Option<Resolution> {
        self.0.resolution.clone().map(Resolution)
    }

    /// Timeline of events
    async fn timeline(&self) -> Vec<TimelineEvent> {
        self.0.timeline.iter().map(|e| TimelineEvent(e.clone())).collect()
    }

    /// Assignees
    async fn assignees(&self) -> &[String] {
        &self.0.assignees
    }

    /// Fingerprint for deduplication
    async fn fingerprint(&self) -> Option<&str> {
        self.0.fingerprint.as_deref()
    }

    /// Correlation score
    async fn correlation_score(&self) -> Option<f64> {
        self.0.correlation_score
    }

    /// Check if incident is active
    async fn is_active(&self) -> bool {
        self.0.is_active()
    }

    /// Check if incident is critical
    async fn is_critical(&self) -> bool {
        self.0.is_critical()
    }
}

/// Incident state enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum IncidentState {
    Detected,
    Triaged,
    Investigating,
    Remediating,
    Resolved,
    Closed,
}

impl From<models::IncidentState> for IncidentState {
    fn from(state: models::IncidentState) -> Self {
        match state {
            models::IncidentState::Detected => IncidentState::Detected,
            models::IncidentState::Triaged => IncidentState::Triaged,
            models::IncidentState::Investigating => IncidentState::Investigating,
            models::IncidentState::Remediating => IncidentState::Remediating,
            models::IncidentState::Resolved => IncidentState::Resolved,
            models::IncidentState::Closed => IncidentState::Closed,
        }
    }
}

impl From<IncidentState> for models::IncidentState {
    fn from(state: IncidentState) -> Self {
        match state {
            IncidentState::Detected => models::IncidentState::Detected,
            IncidentState::Triaged => models::IncidentState::Triaged,
            IncidentState::Investigating => models::IncidentState::Investigating,
            IncidentState::Remediating => models::IncidentState::Remediating,
            IncidentState::Resolved => models::IncidentState::Resolved,
            IncidentState::Closed => models::IncidentState::Closed,
        }
    }
}

/// Severity enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum Severity {
    /// Critical - immediate action
    P0,
    /// High - < 1 hour
    P1,
    /// Medium - < 24 hours
    P2,
    /// Low - < 1 week
    P3,
    /// Informational
    P4,
}

impl From<models::Severity> for Severity {
    fn from(severity: models::Severity) -> Self {
        match severity {
            models::Severity::P0 => Severity::P0,
            models::Severity::P1 => Severity::P1,
            models::Severity::P2 => Severity::P2,
            models::Severity::P3 => Severity::P3,
            models::Severity::P4 => Severity::P4,
        }
    }
}

impl From<Severity> for models::Severity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::P0 => models::Severity::P0,
            Severity::P1 => models::Severity::P1,
            Severity::P2 => models::Severity::P2,
            Severity::P3 => models::Severity::P3,
            Severity::P4 => models::Severity::P4,
        }
    }
}

/// Incident type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum IncidentType {
    Infrastructure,
    Application,
    Security,
    Data,
    Performance,
    Availability,
    Compliance,
    Unknown,
}

impl From<models::IncidentType> for IncidentType {
    fn from(incident_type: models::IncidentType) -> Self {
        match incident_type {
            models::IncidentType::Infrastructure => IncidentType::Infrastructure,
            models::IncidentType::Application => IncidentType::Application,
            models::IncidentType::Security => IncidentType::Security,
            models::IncidentType::Data => IncidentType::Data,
            models::IncidentType::Performance => IncidentType::Performance,
            models::IncidentType::Availability => IncidentType::Availability,
            models::IncidentType::Compliance => IncidentType::Compliance,
            models::IncidentType::Unknown => IncidentType::Unknown,
        }
    }
}

impl From<IncidentType> for models::IncidentType {
    fn from(incident_type: IncidentType) -> Self {
        match incident_type {
            IncidentType::Infrastructure => models::IncidentType::Infrastructure,
            IncidentType::Application => models::IncidentType::Application,
            IncidentType::Security => models::IncidentType::Security,
            IncidentType::Data => models::IncidentType::Data,
            IncidentType::Performance => models::IncidentType::Performance,
            IncidentType::Availability => models::IncidentType::Availability,
            IncidentType::Compliance => models::IncidentType::Compliance,
            IncidentType::Unknown => models::IncidentType::Unknown,
        }
    }
}

/// Resolution information
#[derive(Clone)]
pub struct Resolution(pub models::Resolution);

#[Object]
impl Resolution {
    async fn resolved_at(&self) -> DateTimeScalar {
        self.0.resolved_at.into()
    }

    async fn resolved_by(&self) -> &str {
        &self.0.resolved_by
    }

    async fn resolution_method(&self) -> ResolutionMethod {
        ResolutionMethod::from(self.0.resolution_method.clone())
    }

    async fn root_cause(&self) -> Option<&str> {
        self.0.root_cause.as_deref()
    }

    async fn notes(&self) -> &str {
        &self.0.notes
    }
}

/// Resolution method enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ResolutionMethod {
    Automated,
    Manual,
    AutoAssistedManual,
}

impl From<models::ResolutionMethod> for ResolutionMethod {
    fn from(method: models::ResolutionMethod) -> Self {
        match method {
            models::ResolutionMethod::Automated => ResolutionMethod::Automated,
            models::ResolutionMethod::Manual => ResolutionMethod::Manual,
            models::ResolutionMethod::AutoAssistedManual => ResolutionMethod::AutoAssistedManual,
        }
    }
}

impl From<ResolutionMethod> for models::ResolutionMethod {
    fn from(method: ResolutionMethod) -> Self {
        match method {
            ResolutionMethod::Automated => models::ResolutionMethod::Automated,
            ResolutionMethod::Manual => models::ResolutionMethod::Manual,
            ResolutionMethod::AutoAssistedManual => models::ResolutionMethod::AutoAssistedManual,
        }
    }
}

/// Timeline event
#[derive(Clone)]
pub struct TimelineEvent(pub models::TimelineEvent);

#[Object]
impl TimelineEvent {
    async fn timestamp(&self) -> DateTimeScalar {
        self.0.timestamp.into()
    }

    async fn event_type(&self) -> EventType {
        EventType::from(self.0.event_type.clone())
    }

    async fn actor(&self) -> &str {
        &self.0.actor
    }

    async fn description(&self) -> &str {
        &self.0.description
    }

    async fn metadata(&self) -> Vec<Label> {
        self.0
            .metadata
            .iter()
            .map(|(k, v)| Label {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }
}

/// Event type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum EventType {
    Created,
    StateChanged,
    ActionExecuted,
    NotificationSent,
    AssignmentChanged,
    CommentAdded,
    PlaybookStarted,
    PlaybookCompleted,
    Escalated,
    Resolved,
}

impl From<models::EventType> for EventType {
    fn from(event_type: models::EventType) -> Self {
        match event_type {
            models::EventType::Created => EventType::Created,
            models::EventType::StateChanged => EventType::StateChanged,
            models::EventType::ActionExecuted => EventType::ActionExecuted,
            models::EventType::NotificationSent => EventType::NotificationSent,
            models::EventType::AssignmentChanged => EventType::AssignmentChanged,
            models::EventType::CommentAdded => EventType::CommentAdded,
            models::EventType::PlaybookStarted => EventType::PlaybookStarted,
            models::EventType::PlaybookCompleted => EventType::PlaybookCompleted,
            models::EventType::Escalated => EventType::Escalated,
            models::EventType::Resolved => EventType::Resolved,
            models::EventType::AlertReceived => EventType::Created, // Map to closest equivalent
            models::EventType::SeverityChanged => EventType::StateChanged, // Map to closest equivalent
        }
    }
}

/// Key-value label
#[derive(SimpleObject, Clone)]
pub struct Label {
    pub key: String,
    pub value: String,
}

/// Filter input for incidents
#[derive(InputObject, Debug, Clone)]
pub struct IncidentFilterInput {
    /// Filter by states
    pub states: Option<Vec<IncidentState>>,

    /// Filter by severities
    pub severities: Option<Vec<Severity>>,

    /// Filter by sources
    pub sources: Option<Vec<String>>,

    /// Show only active incidents
    pub active_only: Option<bool>,
}

/// Sort field for incidents
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum IncidentSortField {
    CreatedAt,
    UpdatedAt,
    Severity,
    State,
}

/// Sort input for incidents
#[derive(InputObject, Debug, Clone)]
pub struct IncidentSortInput {
    /// Field to sort by
    pub field: IncidentSortField,

    /// Sort order
    #[graphql(default)]
    pub order: SortOrder,
}

/// Paginated incidents response
#[derive(SimpleObject)]
pub struct IncidentConnection {
    /// List of incidents
    pub incidents: Vec<Incident>,

    /// Pagination information
    pub page_info: PageInfo,
}

/// Create incident input
#[derive(InputObject, Debug)]
pub struct CreateIncidentInput {
    /// Source system
    #[graphql(validator(min_length = 1))]
    pub source: String,

    /// Human-readable title
    #[graphql(validator(min_length = 1, max_length = 500))]
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Severity level
    pub severity: Severity,

    /// Incident type
    pub incident_type: IncidentType,

    /// Affected resources
    #[graphql(default)]
    pub affected_resources: Vec<String>,

    /// Custom labels
    #[graphql(default)]
    pub labels: HashMap<String, String>,
}

/// Update incident input
#[derive(InputObject, Debug)]
pub struct UpdateIncidentInput {
    /// New state
    pub state: Option<IncidentState>,

    /// New assignees
    pub assignees: Option<Vec<String>>,

    /// Add labels
    pub add_labels: Option<HashMap<String, String>>,

    /// Remove label keys
    pub remove_labels: Option<Vec<String>>,
}

/// Resolve incident input
#[derive(InputObject, Debug)]
pub struct ResolveIncidentInput {
    /// Resolution method
    pub method: ResolutionMethod,

    /// Resolution notes
    pub notes: String,

    /// Root cause analysis
    pub root_cause: Option<String>,
}
