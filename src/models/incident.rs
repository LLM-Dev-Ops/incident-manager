use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;
use strum::{EnumString, Display};

/// Represents an incident in the system
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Incident {
    /// Unique identifier
    pub id: Uuid,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Current state
    pub state: IncidentState,

    /// Severity level
    pub severity: Severity,

    /// Incident type
    pub incident_type: IncidentType,

    /// Source system
    #[validate(length(min = 1, max = 255))]
    pub source: String,

    /// Human-readable title
    #[validate(length(min = 1, max = 500))]
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Affected services/resources
    pub affected_resources: Vec<String>,

    /// Custom labels
    pub labels: HashMap<String, String>,

    /// Related incidents
    pub related_incidents: Vec<Uuid>,

    /// Current playbook (if any)
    pub active_playbook: Option<Uuid>,

    /// Resolution details
    pub resolution: Option<Resolution>,

    /// Timeline of events
    pub timeline: Vec<TimelineEvent>,

    /// Assignees
    pub assignees: Vec<String>,

    /// Fingerprint for deduplication
    pub fingerprint: Option<String>,

    /// Correlation score
    pub correlation_score: Option<f64>,
}

impl Incident {
    /// Create a new incident
    pub fn new(
        source: String,
        title: String,
        description: String,
        severity: Severity,
        incident_type: IncidentType,
    ) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();

        Self {
            id,
            created_at: now,
            updated_at: now,
            state: IncidentState::Detected,
            severity,
            incident_type,
            source,
            title,
            description,
            affected_resources: Vec::new(),
            labels: HashMap::new(),
            related_incidents: Vec::new(),
            active_playbook: None,
            resolution: None,
            timeline: vec![TimelineEvent {
                timestamp: now,
                event_type: EventType::Created,
                actor: "system".to_string(),
                description: "Incident created".to_string(),
                metadata: HashMap::new(),
            }],
            assignees: Vec::new(),
            fingerprint: None,
            correlation_score: None,
        }
    }

    /// Add a timeline event
    pub fn add_timeline_event(&mut self, event: TimelineEvent) {
        self.timeline.push(event);
        self.updated_at = Utc::now();
    }

    /// Update incident state
    pub fn update_state(&mut self, new_state: IncidentState, actor: String) {
        let old_state = self.state.clone();
        self.state = new_state.clone();
        self.updated_at = Utc::now();

        self.add_timeline_event(TimelineEvent {
            timestamp: Utc::now(),
            event_type: EventType::StateChanged,
            actor,
            description: format!("State changed from {:?} to {:?}", old_state, new_state),
            metadata: HashMap::from([
                ("old_state".to_string(), format!("{:?}", old_state)),
                ("new_state".to_string(), format!("{:?}", new_state)),
            ]),
        });
    }

    /// Resolve the incident
    pub fn resolve(
        &mut self,
        resolved_by: String,
        method: ResolutionMethod,
        notes: String,
        root_cause: Option<String>,
    ) {
        self.resolution = Some(Resolution {
            resolved_at: Utc::now(),
            resolved_by: resolved_by.clone(),
            resolution_method: method,
            root_cause,
            notes,
        });
        self.update_state(IncidentState::Resolved, resolved_by);
    }

    /// Check if incident is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            IncidentState::Detected
                | IncidentState::Triaged
                | IncidentState::Investigating
                | IncidentState::Remediating
        )
    }

    /// Check if incident is critical
    pub fn is_critical(&self) -> bool {
        matches!(self.severity, Severity::P0 | Severity::P1)
    }

    /// Generate fingerprint for deduplication
    pub fn generate_fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.source.as_bytes());
        hasher.update(self.incident_type.to_string().as_bytes());
        hasher.update(self.title.as_bytes());

        // Include affected resources in fingerprint
        for resource in &self.affected_resources {
            hasher.update(resource.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, Display)]
pub enum IncidentState {
    Detected,
    Triaged,
    Investigating,
    Remediating,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, EnumString, Display)]
pub enum Severity {
    P0, // Critical - immediate action
    P1, // High - < 1 hour
    P2, // Medium - < 24 hours
    P3, // Low - < 1 week
    P4, // Informational
}

impl Severity {
    /// Get numeric priority (lower is more urgent)
    pub fn priority(&self) -> u8 {
        match self {
            Severity::P0 => 0,
            Severity::P1 => 1,
            Severity::P2 => 2,
            Severity::P3 => 3,
            Severity::P4 => 4,
        }
    }

    /// Check if severity requires immediate attention
    pub fn is_urgent(&self) -> bool {
        matches!(self, Severity::P0 | Severity::P1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, Display)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub resolved_at: DateTime<Utc>,
    pub resolved_by: String,
    pub resolution_method: ResolutionMethod,
    pub root_cause: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, Display)]
pub enum ResolutionMethod {
    Automated,
    Manual,
    AutoAssistedManual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub actor: String, // System or user
    pub description: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, Display)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_creation() {
        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "This is a test".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        assert_eq!(incident.state, IncidentState::Detected);
        assert_eq!(incident.severity, Severity::P2);
        assert_eq!(incident.incident_type, IncidentType::Application);
        assert_eq!(incident.timeline.len(), 1);
        assert!(incident.is_active());
        assert!(!incident.is_critical());
    }

    #[test]
    fn test_incident_state_transition() {
        let mut incident = Incident::new(
            "test-source".to_string(),
            "Test".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        incident.update_state(IncidentState::Investigating, "user@example.com".to_string());

        assert_eq!(incident.state, IncidentState::Investigating);
        assert!(incident.timeline.len() > 1);
        assert!(incident.is_critical());
    }

    #[test]
    fn test_incident_resolution() {
        let mut incident = Incident::new(
            "test-source".to_string(),
            "Test".to_string(),
            "Description".to_string(),
            Severity::P0,
            IncidentType::Security,
        );

        incident.resolve(
            "responder@example.com".to_string(),
            ResolutionMethod::Manual,
            "Fixed manually".to_string(),
            Some("Root cause identified".to_string()),
        );

        assert_eq!(incident.state, IncidentState::Resolved);
        assert!(incident.resolution.is_some());
        assert!(!incident.is_active());
    }

    #[test]
    fn test_fingerprint_generation() {
        let incident = Incident::new(
            "sentinel".to_string(),
            "API Latency High".to_string(),
            "P95 latency exceeded threshold".to_string(),
            Severity::P1,
            IncidentType::Performance,
        );

        let fingerprint = incident.generate_fingerprint();
        assert!(!fingerprint.is_empty());
        assert_eq!(fingerprint.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_severity_priority() {
        assert_eq!(Severity::P0.priority(), 0);
        assert_eq!(Severity::P4.priority(), 4);
        assert!(Severity::P0 < Severity::P2);
        assert!(Severity::P1.is_urgent());
        assert!(!Severity::P3.is_urgent());
    }
}
