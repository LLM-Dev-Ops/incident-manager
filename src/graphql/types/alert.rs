//! GraphQL types for alerts

use async_graphql::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models;
use super::common::DateTimeScalar;
use super::incident::{IncidentType, Severity};

/// Alert object type
#[derive(Clone)]
pub struct Alert(pub models::Alert);

#[Object]
impl Alert {
    /// Unique alert identifier
    async fn id(&self) -> &Uuid {
        &self.0.id
    }

    /// External alert ID (from source system)
    async fn external_id(&self) -> &str {
        &self.0.external_id
    }

    /// Alert source
    async fn source(&self) -> &str {
        &self.0.source
    }

    /// Timestamp when alert was generated
    async fn timestamp(&self) -> DateTimeScalar {
        self.0.timestamp.into()
    }

    /// Timestamp when alert was received
    async fn received_at(&self) -> DateTimeScalar {
        self.0.received_at.into()
    }

    /// Alert severity
    async fn severity(&self) -> Severity {
        Severity::from(self.0.severity)
    }

    /// Alert type
    async fn alert_type(&self) -> IncidentType {
        IncidentType::from(self.0.alert_type.clone())
    }

    /// Alert title
    async fn title(&self) -> &str {
        &self.0.title
    }

    /// Alert description
    async fn description(&self) -> &str {
        &self.0.description
    }

    /// Labels/tags
    async fn labels(&self) -> Vec<super::incident::Label> {
        self.0
            .labels
            .iter()
            .map(|(k, v)| super::incident::Label {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }

    /// Affected services/resources
    async fn affected_services(&self) -> &[String] {
        &self.0.affected_services
    }

    /// Runbook URL
    async fn runbook_url(&self) -> Option<&str> {
        self.0.runbook_url.as_deref()
    }

    /// Annotations (additional metadata)
    async fn annotations(&self) -> Vec<super::incident::Label> {
        self.0
            .annotations
            .iter()
            .map(|(k, v)| super::incident::Label {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }

    /// Generated incident ID (if converted to incident)
    async fn incident_id(&self) -> Option<&Uuid> {
        self.0.incident_id.as_ref()
    }

    /// Whether this alert was deduplicated
    async fn deduplicated(&self) -> bool {
        self.0.deduplicated
    }

    /// Parent alert ID if this is a duplicate
    async fn parent_alert_id(&self) -> Option<&Uuid> {
        self.0.parent_alert_id.as_ref()
    }

    /// Check if alert is urgent
    async fn is_urgent(&self) -> bool {
        self.0.is_urgent()
    }
}

/// Submit alert input
#[derive(InputObject, Debug)]
pub struct SubmitAlertInput {
    /// External alert ID
    pub external_id: Option<String>,

    /// Alert source
    #[graphql(validator(min_length = 1))]
    pub source: String,

    /// Alert title
    #[graphql(validator(min_length = 1, max_length = 500))]
    pub title: String,

    /// Alert description
    pub description: String,

    /// Alert severity
    pub severity: Severity,

    /// Alert type
    pub alert_type: IncidentType,

    /// Labels/tags
    #[graphql(default)]
    pub labels: HashMap<String, String>,

    /// Affected services
    #[graphql(default)]
    pub affected_services: Vec<String>,

    /// Runbook URL
    pub runbook_url: Option<String>,

    /// Annotations
    #[graphql(default)]
    pub annotations: HashMap<String, String>,
}

/// Alert acknowledgment status
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AckStatus {
    Accepted,
    Duplicate,
    RateLimited,
    Rejected,
}

impl From<models::AckStatus> for AckStatus {
    fn from(status: models::AckStatus) -> Self {
        match status {
            models::AckStatus::Accepted => AckStatus::Accepted,
            models::AckStatus::Duplicate => AckStatus::Duplicate,
            models::AckStatus::RateLimited => AckStatus::RateLimited,
            models::AckStatus::Rejected => AckStatus::Rejected,
        }
    }
}

/// Alert acknowledgment response
#[derive(SimpleObject)]
pub struct AlertAck {
    /// Alert ID
    pub alert_id: Uuid,

    /// Generated incident ID (if created)
    pub incident_id: Option<Uuid>,

    /// Acknowledgment status
    pub status: AckStatus,

    /// Status message
    pub message: String,

    /// Received timestamp
    pub received_at: DateTimeScalar,
}

impl From<models::AlertAck> for AlertAck {
    fn from(ack: models::AlertAck) -> Self {
        Self {
            alert_id: ack.alert_id,
            incident_id: ack.incident_id,
            status: AckStatus::from(ack.status),
            message: ack.message,
            received_at: ack.received_at.into(),
        }
    }
}
