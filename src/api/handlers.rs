use crate::api::AppState;
use crate::error::{AppError, Result};
use crate::models::*;
use crate::state::IncidentFilter;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Health check endpoint
pub async fn health_check() -> Result<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
    }))
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Submit an alert
pub async fn submit_alert(
    State(state): State<AppState>,
    Json(request): Json<SubmitAlertRequest>,
) -> Result<Json<AlertAckResponse>> {
    request.validate()?;

    let alert = Alert::new(
        request.external_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        request.source,
        request.title,
        request.description,
        request.severity,
        request.alert_type,
    );

    let ack = state.processor.process_alert(alert).await?;

    Ok(Json(AlertAckResponse {
        alert_id: ack.alert_id,
        incident_id: ack.incident_id,
        status: ack.status,
        message: ack.message,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SubmitAlertRequest {
    pub external_id: Option<String>,
    #[validate(length(min = 1))]
    pub source: String,
    #[validate(length(min = 1, max = 500))]
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub alert_type: IncidentType,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub affected_services: Vec<String>,
    pub runbook_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AlertAckResponse {
    pub alert_id: Uuid,
    pub incident_id: Option<Uuid>,
    pub status: AckStatus,
    pub message: String,
}

/// Create an incident directly
pub async fn create_incident(
    State(state): State<AppState>,
    Json(request): Json<CreateIncidentRequest>,
) -> Result<(StatusCode, Json<IncidentResponse>)> {
    request.validate()?;

    let incident = Incident::new(
        request.source,
        request.title,
        request.description,
        request.severity,
        request.incident_type,
    );

    let created = state.processor.create_incident(incident).await?;

    Ok((StatusCode::CREATED, Json(IncidentResponse::from(created))))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIncidentRequest {
    #[validate(length(min = 1))]
    pub source: String,
    #[validate(length(min = 1, max = 500))]
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub incident_type: IncidentType,
}

/// Get an incident by ID
pub async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IncidentResponse>> {
    let incident = state.processor.get_incident(&id).await?;
    Ok(Json(IncidentResponse::from(incident)))
}

/// List incidents
pub async fn list_incidents(
    State(state): State<AppState>,
    Query(params): Query<ListIncidentsQuery>,
) -> Result<Json<ListIncidentsResponse>> {
    let filter = IncidentFilter {
        states: params.states.unwrap_or_default(),
        severities: params.severities.unwrap_or_default(),
        sources: params.sources.unwrap_or_default(),
        active_only: params.active_only.unwrap_or(false),
    };

    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20).min(100); // Max 100 per page

    let incidents = state
        .processor
        .store
        .list_incidents(&filter, page, page_size)
        .await?;

    let total = state.processor.store.count_incidents(&filter).await?;

    Ok(Json(ListIncidentsResponse {
        incidents: incidents.into_iter().map(IncidentResponse::from).collect(),
        total,
        page,
        page_size,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListIncidentsQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub states: Option<Vec<IncidentState>>,
    pub severities: Option<Vec<Severity>>,
    pub sources: Option<Vec<String>>,
    pub active_only: Option<bool>,
}

/// Update incident
pub async fn update_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateIncidentRequest>,
) -> Result<Json<IncidentResponse>> {
    let mut incident = state.processor.get_incident(&id).await?;

    if let Some(new_state) = request.state {
        incident.update_state(new_state, request.actor.unwrap_or_else(|| "api".to_string()));
    }

    if let Some(assignees) = request.assignees {
        incident.assignees = assignees;
    }

    state.processor.store.update_incident(&incident).await?;

    Ok(Json(IncidentResponse::from(incident)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub state: Option<IncidentState>,
    pub assignees: Option<Vec<String>>,
    pub actor: Option<String>,
}

/// Resolve incident
pub async fn resolve_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<ResolveIncidentRequest>,
) -> Result<Json<IncidentResponse>> {
    request.validate()?;

    let incident = state
        .processor
        .resolve_incident(
            &id,
            request.resolved_by,
            request.method,
            request.notes,
            request.root_cause,
        )
        .await?;

    Ok(Json(IncidentResponse::from(incident)))
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResolveIncidentRequest {
    #[validate(length(min = 1))]
    pub resolved_by: String,
    pub method: ResolutionMethod,
    pub notes: String,
    pub root_cause: Option<String>,
}

/// Incident response DTO
#[derive(Debug, Serialize)]
pub struct IncidentResponse {
    pub id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub state: IncidentState,
    pub severity: Severity,
    pub incident_type: IncidentType,
    pub source: String,
    pub title: String,
    pub description: String,
    pub affected_resources: Vec<String>,
    pub labels: HashMap<String, String>,
    pub assignees: Vec<String>,
    pub resolution: Option<Resolution>,
}

impl From<Incident> for IncidentResponse {
    fn from(incident: Incident) -> Self {
        Self {
            id: incident.id,
            created_at: incident.created_at,
            updated_at: incident.updated_at,
            state: incident.state,
            severity: incident.severity,
            incident_type: incident.incident_type,
            source: incident.source,
            title: incident.title,
            description: incident.description,
            affected_resources: incident.affected_resources,
            labels: incident.labels,
            assignees: incident.assignees,
            resolution: incident.resolution,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListIncidentsResponse {
    pub incidents: Vec<IncidentResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}
