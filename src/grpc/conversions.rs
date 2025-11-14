use crate::grpc::proto::{alerts, incidents};
use crate::models::*;
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use uuid::Uuid;

/// Convert from proto Severity to domain Severity
impl From<incidents::Severity> for Severity {
    fn from(severity: incidents::Severity) -> Self {
        match severity {
            incidents::Severity::P0 => Severity::P0,
            incidents::Severity::P1 => Severity::P1,
            incidents::Severity::P2 => Severity::P2,
            incidents::Severity::P3 => Severity::P3,
            incidents::Severity::P4 => Severity::P4,
            _ => Severity::P3, // Default
        }
    }
}

/// Convert from domain Severity to proto Severity
impl From<Severity> for incidents::Severity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::P0 => incidents::Severity::P0,
            Severity::P1 => incidents::Severity::P1,
            Severity::P2 => incidents::Severity::P2,
            Severity::P3 => incidents::Severity::P3,
            Severity::P4 => incidents::Severity::P4,
        }
    }
}

/// Convert from proto IncidentType to domain IncidentType
impl From<incidents::IncidentType> for IncidentType {
    fn from(incident_type: incidents::IncidentType) -> Self {
        match incident_type {
            incidents::IncidentType::Infrastructure => IncidentType::Infrastructure,
            incidents::IncidentType::Application => IncidentType::Application,
            incidents::IncidentType::Security => IncidentType::Security,
            incidents::IncidentType::Data => IncidentType::Data,
            incidents::IncidentType::Performance => IncidentType::Performance,
            incidents::IncidentType::Availability => IncidentType::Availability,
            incidents::IncidentType::Compliance => IncidentType::Compliance,
            _ => IncidentType::Unknown,
        }
    }
}

/// Convert from domain IncidentType to proto IncidentType
impl From<IncidentType> for incidents::IncidentType {
    fn from(incident_type: IncidentType) -> Self {
        match incident_type {
            IncidentType::Infrastructure => incidents::IncidentType::Infrastructure,
            IncidentType::Application => incidents::IncidentType::Application,
            IncidentType::Security => incidents::IncidentType::Security,
            IncidentType::Data => incidents::IncidentType::Data,
            IncidentType::Performance => incidents::IncidentType::Performance,
            IncidentType::Availability => incidents::IncidentType::Availability,
            IncidentType::Compliance => incidents::IncidentType::Compliance,
            IncidentType::Unknown => incidents::IncidentType::Unspecified,
        }
    }
}

/// Convert from proto IncidentState to domain IncidentState
impl From<incidents::IncidentState> for IncidentState {
    fn from(state: incidents::IncidentState) -> Self {
        match state {
            incidents::IncidentState::Open => IncidentState::Detected,
            incidents::IncidentState::Acknowledged => IncidentState::Triaged,
            incidents::IncidentState::Investigating => IncidentState::Investigating,
            incidents::IncidentState::Resolved => IncidentState::Resolved,
            incidents::IncidentState::Closed => IncidentState::Closed,
            _ => IncidentState::Detected,
        }
    }
}

/// Convert from domain IncidentState to proto IncidentState
impl From<IncidentState> for incidents::IncidentState {
    fn from(state: IncidentState) -> Self {
        match state {
            IncidentState::Detected => incidents::IncidentState::Open,
            IncidentState::Triaged => incidents::IncidentState::Acknowledged,
            IncidentState::Investigating => incidents::IncidentState::Investigating,
            IncidentState::Remediating => incidents::IncidentState::Investigating,
            IncidentState::Resolved => incidents::IncidentState::Resolved,
            IncidentState::Closed => incidents::IncidentState::Closed,
        }
    }
}

/// Convert from proto ResolutionMethod to domain ResolutionMethod
impl From<incidents::ResolutionMethod> for ResolutionMethod {
    fn from(method: incidents::ResolutionMethod) -> Self {
        match method {
            incidents::ResolutionMethod::Automated => ResolutionMethod::Automated,
            incidents::ResolutionMethod::Manual => ResolutionMethod::Manual,
            incidents::ResolutionMethod::AutoAssistedManual => {
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
            ResolutionMethod::Automated => incidents::ResolutionMethod::Automated,
            ResolutionMethod::Manual => incidents::ResolutionMethod::Manual,
            ResolutionMethod::AutoAssistedManual => {
                incidents::ResolutionMethod::AutoAssistedManual
            }
        }
    }
}

/// Convert from proto EventType to domain EventType
impl From<incidents::EventType> for EventType {
    fn from(event_type: incidents::EventType) -> Self {
        match event_type {
            incidents::EventType::Created => EventType::Created,
            incidents::EventType::StateChanged => EventType::StateChanged,
            incidents::EventType::ActionExecuted => EventType::ActionExecuted,
            incidents::EventType::NotificationSent => EventType::NotificationSent,
            incidents::EventType::AssignmentChanged => EventType::AssignmentChanged,
            incidents::EventType::CommentAdded => EventType::CommentAdded,
            incidents::EventType::PlaybookStarted => EventType::PlaybookStarted,
            incidents::EventType::PlaybookCompleted => EventType::PlaybookCompleted,
            incidents::EventType::Escalated => EventType::Escalated,
            incidents::EventType::Resolved => EventType::Resolved,
            _ => EventType::Created,
        }
    }
}

/// Convert from domain EventType to proto EventType
impl From<EventType> for incidents::EventType {
    fn from(event_type: EventType) -> Self {
        match event_type {
            EventType::Created => incidents::EventType::Created,
            EventType::StateChanged => incidents::EventType::StateChanged,
            EventType::ActionExecuted => incidents::EventType::ActionExecuted,
            EventType::NotificationSent => incidents::EventType::NotificationSent,
            EventType::AssignmentChanged => incidents::EventType::AssignmentChanged,
            EventType::CommentAdded => incidents::EventType::CommentAdded,
            EventType::PlaybookStarted => incidents::EventType::PlaybookStarted,
            EventType::PlaybookCompleted => incidents::EventType::PlaybookCompleted,
            EventType::Escalated => incidents::EventType::Escalated,
            EventType::Resolved => incidents::EventType::Resolved,
            EventType::AlertReceived => incidents::EventType::Created, // Map to closest equivalent
            EventType::SeverityChanged => incidents::EventType::StateChanged, // Map to closest equivalent
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
        // Calculate time to resolve if resolved
        let time_to_resolve_seconds = incident.resolution.as_ref().map(|r| {
            (r.resolved_at.timestamp() - incident.created_at.timestamp()) as i32
        }).unwrap_or(0);

        let severity_str = match incident.severity {
            Severity::P0 => "P0",
            Severity::P1 => "P1",
            Severity::P2 => "P2",
            Severity::P3 => "P3",
            Severity::P4 => "P4",
        };

        let status_str = match incident.state {
            IncidentState::Detected => "Open",
            IncidentState::Triaged => "Acknowledged",
            IncidentState::Investigating => "Investigating",
            IncidentState::Remediating => "Investigating",
            IncidentState::Resolved => "Resolved",
            IncidentState::Closed => "Closed",
        };

        incidents::Incident {
            id: incident.id.to_string(),
            title: incident.title,
            description: incident.description,
            severity: severity_str.to_string(),
            status: status_str.to_string(),
            source: incident.source,
            assigned_to: incident.assignees.first().cloned().unwrap_or_default(),
            created_at: datetime_to_timestamp(incident.created_at),
            updated_at: datetime_to_timestamp(incident.updated_at),
            resolved_at: incident.resolution.as_ref().and_then(|r| datetime_to_timestamp(r.resolved_at)),
            metadata: incident.labels,
            tags: incident.affected_resources,
            notes: incident
                .notes
                .into_iter()
                .map(|n| incidents::Note {
                    id: n.id.to_string(),
                    content: n.content,
                    author: n.author,
                    created_at: datetime_to_timestamp(n.created_at),
                })
                .collect(),
            fingerprint: incident.fingerprint.unwrap_or_default(),
            resolution: incident.resolution.map(|r| incidents::Resolution {
                method: incidents::ResolutionMethod::from(r.resolution_method) as i32,
                summary: r.notes,
                resolved_by: r.resolved_by,
                resolved_at: datetime_to_timestamp(r.resolved_at),
                time_to_resolve_seconds,
            }),
            timeline: incident
                .timeline
                .into_iter()
                .map(|te| incidents::TimelineEvent {
                    id: Uuid::new_v4().to_string(),
                    event_type: incidents::EventType::from(te.event_type) as i32,
                    description: te.description,
                    actor: te.actor,
                    timestamp: datetime_to_timestamp(te.timestamp),
                    metadata: te.metadata,
                })
                .collect(),
            incident_type: incidents::IncidentType::from(incident.incident_type) as i32,
            incident_state: incidents::IncidentState::from(incident.state) as i32,
        }
    }
}

/// Convert domain Alert to proto AlertMessage
impl From<Alert> for alerts::AlertMessage {
    fn from(alert: Alert) -> Self {
        let severity_str = match alert.severity {
            Severity::P0 => "P0",
            Severity::P1 => "P1",
            Severity::P2 => "P2",
            Severity::P3 => "P3",
            Severity::P4 => "P4",
        };

        alerts::AlertMessage {
            id: alert.id.to_string(),
            name: alert.title,
            description: alert.description,
            severity: severity_str.to_string(),
            source: alert.source,
            labels: alert.labels,
            annotations: alert.annotations,
            fired_at: datetime_to_timestamp(alert.timestamp),
        }
    }
}

/// Convert domain AckStatus to proto AckStatus
impl From<AckStatus> for alerts::AckStatus {
    fn from(status: AckStatus) -> Self {
        match status {
            AckStatus::Accepted => alerts::AckStatus::Accepted,
            AckStatus::Duplicate => alerts::AckStatus::Duplicate,
            AckStatus::RateLimited => alerts::AckStatus::RateLimited,
            AckStatus::Rejected => alerts::AckStatus::Error,
        }
    }
}

/// Convert proto AckStatus to domain AckStatus
impl From<alerts::AckStatus> for AckStatus {
    fn from(status: alerts::AckStatus) -> Self {
        match status {
            alerts::AckStatus::Accepted => AckStatus::Accepted,
            alerts::AckStatus::Duplicate => AckStatus::Duplicate,
            alerts::AckStatus::RateLimited => AckStatus::RateLimited,
            alerts::AckStatus::Error => AckStatus::Rejected,
            _ => AckStatus::Rejected,
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
        assert_eq!(proto_severity, incidents::Severity::P0);

        let back_to_domain: Severity = proto_severity.into();
        assert_eq!(back_to_domain, Severity::P0);
    }

    #[test]
    fn test_incident_state_conversion() {
        let domain_state = IncidentState::Investigating;
        let proto_state: incidents::IncidentState = domain_state.clone().into();
        assert_eq!(
            proto_state,
            incidents::IncidentState::Investigating
        );

        let back_to_domain: IncidentState = proto_state.into();
        assert_eq!(back_to_domain, domain_state);
    }

    #[test]
    fn test_incident_conversion() {
        use crate::models::IncidentType;

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Application,
        );

        let proto_incident: incidents::Incident = incident.clone().into();
        assert_eq!(proto_incident.source, "test-source");
        assert_eq!(proto_incident.title, "Test Incident");
        assert_eq!(proto_incident.severity, "P1");
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
