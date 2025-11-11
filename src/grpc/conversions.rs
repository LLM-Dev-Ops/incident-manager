use crate::grpc::proto::{alerts, incidents};
use crate::models::*;
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use std::collections::HashMap;
use uuid::Uuid;

/// Convert from proto Severity to domain Severity
impl From<incidents::Severity> for Severity {
    fn from(severity: incidents::Severity) -> Self {
        match severity {
            incidents::Severity::SeverityP0 => Severity::P0,
            incidents::Severity::SeverityP1 => Severity::P1,
            incidents::Severity::SeverityP2 => Severity::P2,
            incidents::Severity::SeverityP3 => Severity::P3,
            incidents::Severity::SeverityP4 => Severity::P4,
            _ => Severity::P3, // Default
        }
    }
}

/// Convert from domain Severity to proto Severity
impl From<Severity> for incidents::Severity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::P0 => incidents::Severity::SeverityP0,
            Severity::P1 => incidents::Severity::SeverityP1,
            Severity::P2 => incidents::Severity::SeverityP2,
            Severity::P3 => incidents::Severity::SeverityP3,
            Severity::P4 => incidents::Severity::SeverityP4,
        }
    }
}

/// Convert from proto IncidentType to domain IncidentType
impl From<incidents::IncidentType> for IncidentType {
    fn from(incident_type: incidents::IncidentType) -> Self {
        match incident_type {
            incidents::IncidentType::IncidentTypeInfrastructure => IncidentType::Infrastructure,
            incidents::IncidentType::IncidentTypeApplication => IncidentType::Application,
            incidents::IncidentType::IncidentTypeSecurity => IncidentType::Security,
            incidents::IncidentType::IncidentTypeData => IncidentType::Data,
            incidents::IncidentType::IncidentTypePerformance => IncidentType::Performance,
            incidents::IncidentType::IncidentTypeAvailability => IncidentType::Availability,
            incidents::IncidentType::IncidentTypeCompliance => IncidentType::Compliance,
            _ => IncidentType::Unknown,
        }
    }
}

/// Convert from domain IncidentType to proto IncidentType
impl From<IncidentType> for incidents::IncidentType {
    fn from(incident_type: IncidentType) -> Self {
        match incident_type {
            IncidentType::Infrastructure => incidents::IncidentType::IncidentTypeInfrastructure,
            IncidentType::Application => incidents::IncidentType::IncidentTypeApplication,
            IncidentType::Security => incidents::IncidentType::IncidentTypeSecurity,
            IncidentType::Data => incidents::IncidentType::IncidentTypeData,
            IncidentType::Performance => incidents::IncidentType::IncidentTypePerformance,
            IncidentType::Availability => incidents::IncidentType::IncidentTypeAvailability,
            IncidentType::Compliance => incidents::IncidentType::IncidentTypeCompliance,
            IncidentType::Unknown => incidents::IncidentType::IncidentTypeUnspecified,
        }
    }
}

/// Convert from proto IncidentState to domain IncidentState
impl From<incidents::IncidentState> for IncidentState {
    fn from(state: incidents::IncidentState) -> Self {
        match state {
            incidents::IncidentState::IncidentStateDetected => IncidentState::Detected,
            incidents::IncidentState::IncidentStateTriaged => IncidentState::Triaged,
            incidents::IncidentState::IncidentStateInvestigating => IncidentState::Investigating,
            incidents::IncidentState::IncidentStateRemediating => IncidentState::Remediating,
            incidents::IncidentState::IncidentStateResolved => IncidentState::Resolved,
            incidents::IncidentState::IncidentStateClosed => IncidentState::Closed,
            _ => IncidentState::Detected,
        }
    }
}

/// Convert from domain IncidentState to proto IncidentState
impl From<IncidentState> for incidents::IncidentState {
    fn from(state: IncidentState) -> Self {
        match state {
            IncidentState::Detected => incidents::IncidentState::IncidentStateDetected,
            IncidentState::Triaged => incidents::IncidentState::IncidentStateTriaged,
            IncidentState::Investigating => incidents::IncidentState::IncidentStateInvestigating,
            IncidentState::Remediating => incidents::IncidentState::IncidentStateRemediating,
            IncidentState::Resolved => incidents::IncidentState::IncidentStateResolved,
            IncidentState::Closed => incidents::IncidentState::IncidentStateClosed,
        }
    }
}

/// Convert from proto ResolutionMethod to domain ResolutionMethod
impl From<incidents::ResolutionMethod> for ResolutionMethod {
    fn from(method: incidents::ResolutionMethod) -> Self {
        match method {
            incidents::ResolutionMethod::ResolutionMethodAutomated => ResolutionMethod::Automated,
            incidents::ResolutionMethod::ResolutionMethodManual => ResolutionMethod::Manual,
            incidents::ResolutionMethod::ResolutionMethodAutoAssistedManual => {
                ResolutionMethod::AutoAssistedManual
            }
            _ => ResolutionMethod::Manual,
        }
    }
}

/// Convert from domain ResolutionMethod to proto ResolutionMethod
impl From<ResolutionMethod> for incidents::ResolutionMethod {
    fn from(method: ResolutionMethod) -> Self {
        match method {
            ResolutionMethod::Automated => incidents::ResolutionMethod::ResolutionMethodAutomated,
            ResolutionMethod::Manual => incidents::ResolutionMethod::ResolutionMethodManual,
            ResolutionMethod::AutoAssistedManual => {
                incidents::ResolutionMethod::ResolutionMethodAutoAssistedManual
            }
        }
    }
}

/// Convert from proto EventType to domain EventType
impl From<incidents::EventType> for EventType {
    fn from(event_type: incidents::EventType) -> Self {
        match event_type {
            incidents::EventType::EventTypeCreated => EventType::Created,
            incidents::EventType::EventTypeStateChanged => EventType::StateChanged,
            incidents::EventType::EventTypeActionExecuted => EventType::ActionExecuted,
            incidents::EventType::EventTypeNotificationSent => EventType::NotificationSent,
            incidents::EventType::EventTypeAssignmentChanged => EventType::AssignmentChanged,
            incidents::EventType::EventTypeCommentAdded => EventType::CommentAdded,
            incidents::EventType::EventTypePlaybookStarted => EventType::PlaybookStarted,
            incidents::EventType::EventTypePlaybookCompleted => EventType::PlaybookCompleted,
            incidents::EventType::EventTypeEscalated => EventType::Escalated,
            incidents::EventType::EventTypeResolved => EventType::Resolved,
            _ => EventType::Created,
        }
    }
}

/// Convert from domain EventType to proto EventType
impl From<EventType> for incidents::EventType {
    fn from(event_type: EventType) -> Self {
        match event_type {
            EventType::Created => incidents::EventType::EventTypeCreated,
            EventType::StateChanged => incidents::EventType::EventTypeStateChanged,
            EventType::ActionExecuted => incidents::EventType::EventTypeActionExecuted,
            EventType::NotificationSent => incidents::EventType::EventTypeNotificationSent,
            EventType::AssignmentChanged => incidents::EventType::EventTypeAssignmentChanged,
            EventType::CommentAdded => incidents::EventType::EventTypeCommentAdded,
            EventType::PlaybookStarted => incidents::EventType::EventTypePlaybookStarted,
            EventType::PlaybookCompleted => incidents::EventType::EventTypePlaybookCompleted,
            EventType::Escalated => incidents::EventType::EventTypeEscalated,
            EventType::Resolved => incidents::EventType::EventTypeResolved,
        }
    }
}

/// Convert DateTime<Utc> to Timestamp
pub fn datetime_to_timestamp(dt: DateTime<Utc>) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

/// Convert Timestamp to DateTime<Utc>
pub fn timestamp_to_datetime(ts: Option<Timestamp>) -> DateTime<Utc> {
    ts.map(|t| {
        DateTime::from_timestamp(t.seconds, t.nanos as u32).unwrap_or_else(Utc::now)
    })
    .unwrap_or_else(Utc::now)
}

/// Convert domain Incident to proto Incident
impl From<Incident> for incidents::Incident {
    fn from(incident: Incident) -> Self {
        incidents::Incident {
            id: incident.id.to_string(),
            created_at: datetime_to_timestamp(incident.created_at),
            updated_at: datetime_to_timestamp(incident.updated_at),
            state: incidents::IncidentState::from(incident.state) as i32,
            severity: incidents::Severity::from(incident.severity) as i32,
            r#type: incidents::IncidentType::from(incident.incident_type) as i32,
            source: incident.source,
            title: incident.title,
            description: incident.description,
            affected_resources: incident.affected_resources,
            labels: incident.labels,
            related_incidents: incident
                .related_incidents
                .iter()
                .map(|id| id.to_string())
                .collect(),
            active_playbook: incident.active_playbook.map(|id| id.to_string()),
            resolution: incident.resolution.map(|r| incidents::Resolution {
                resolved_at: datetime_to_timestamp(r.resolved_at),
                resolved_by: r.resolved_by,
                method: incidents::ResolutionMethod::from(r.resolution_method) as i32,
                root_cause: r.root_cause,
                notes: r.notes,
            }),
            timeline: incident
                .timeline
                .into_iter()
                .map(|te| incidents::TimelineEvent {
                    timestamp: datetime_to_timestamp(te.timestamp),
                    event_type: incidents::EventType::from(te.event_type) as i32,
                    actor: te.actor,
                    description: te.description,
                    metadata: te.metadata,
                })
                .collect(),
            assignees: incident.assignees,
        }
    }
}

/// Convert domain Alert to proto AlertMessage
impl From<Alert> for alerts::AlertMessage {
    fn from(alert: Alert) -> Self {
        alerts::AlertMessage {
            alert_id: alert.id.to_string(),
            source: alert.source,
            timestamp: datetime_to_timestamp(alert.timestamp),
            severity: incidents::Severity::from(alert.severity) as i32,
            r#type: incidents::IncidentType::from(alert.alert_type) as i32,
            title: alert.title,
            description: alert.description,
            labels: alert.labels,
            affected_services: alert.affected_services,
            runbook_url: alert.runbook_url,
            annotations: alert.annotations,
        }
    }
}

/// Convert domain AckStatus to proto AckStatus
impl From<AckStatus> for alerts::AckStatus {
    fn from(status: AckStatus) -> Self {
        match status {
            AckStatus::Accepted => alerts::AckStatus::AckStatusAccepted,
            AckStatus::Duplicate => alerts::AckStatus::AckStatusDuplicate,
            AckStatus::RateLimited => alerts::AckStatus::AckStatusRateLimited,
            AckStatus::Rejected => alerts::AckStatus::AckStatusRejected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_conversion() {
        let domain_severity = Severity::P0;
        let proto_severity: incidents::Severity = domain_severity.into();
        assert_eq!(proto_severity, incidents::Severity::SeverityP0);

        let back_to_domain: Severity = proto_severity.into();
        assert_eq!(back_to_domain, Severity::P0);
    }

    #[test]
    fn test_incident_state_conversion() {
        let domain_state = IncidentState::Investigating;
        let proto_state: incidents::IncidentState = domain_state.clone().into();
        assert_eq!(
            proto_state,
            incidents::IncidentState::IncidentStateInvestigating
        );

        let back_to_domain: IncidentState = proto_state.into();
        assert_eq!(back_to_domain, domain_state);
    }

    #[test]
    fn test_timestamp_conversion() {
        let now = Utc::now();
        let timestamp = datetime_to_timestamp(now);
        assert!(timestamp.is_some());

        let back_to_datetime = timestamp_to_datetime(timestamp);
        // Allow 1 second tolerance due to nano precision
        assert!((now.timestamp() - back_to_datetime.timestamp()).abs() <= 1);
    }
}
