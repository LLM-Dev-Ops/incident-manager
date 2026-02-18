use crate::api::AppState;
use crate::error::Result;
use crate::execution::{ExecutionContext, ExecutionResponse};
use crate::models::*;
use crate::state::IncidentFilter;
use axum::{
    extract::{Extension, Path, Query, State},
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
    exec_ctx: Option<Extension<ExecutionContext>>,
    Json(request): Json<SubmitAlertRequest>,
) -> Result<Json<ExecutionResponse<AlertAckResponse>>> {
    request.validate()?;

    let ctx = exec_ctx.map(|Extension(c)| c);

    let alert = Alert::new(
        request.external_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        request.source,
        request.title,
        request.description,
        request.severity,
        request.alert_type,
    );

    let ack = state.processor.process_alert(alert, ctx.as_ref()).await?;

    let graph = ctx.map(|c| c.finalize(None));

    Ok(Json(ExecutionResponse::new(
        AlertAckResponse {
            alert_id: ack.alert_id,
            incident_id: ack.incident_id,
            status: ack.status,
            message: ack.message,
        },
        graph,
    )))
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
    exec_ctx: Option<Extension<ExecutionContext>>,
    Json(request): Json<CreateIncidentRequest>,
) -> Result<(StatusCode, Json<ExecutionResponse<IncidentResponse>>)> {
    request.validate()?;

    let ctx = exec_ctx.map(|Extension(c)| c);

    let incident = Incident::new(
        request.source,
        request.title,
        request.description,
        request.severity,
        request.incident_type,
    );

    let created = state.processor.create_incident(incident, ctx.as_ref()).await?;

    let graph = ctx.map(|c| c.finalize(None));

    Ok((
        StatusCode::CREATED,
        Json(ExecutionResponse::new(IncidentResponse::from(created), graph)),
    ))
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
        .store()
        .list_incidents(&filter, page, page_size)
        .await?;

    let total = state.processor.store().count_incidents(&filter).await?;

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

    state.processor.store().update_incident(&incident).await?;

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
    exec_ctx: Option<Extension<ExecutionContext>>,
    Path(id): Path<Uuid>,
    Json(request): Json<ResolveIncidentRequest>,
) -> Result<Json<ExecutionResponse<IncidentResponse>>> {
    request.validate()?;

    let ctx = exec_ctx.map(|Extension(c)| c);

    let incident = state
        .processor
        .resolve_incident(
            &id,
            request.resolved_by,
            request.method,
            request.notes,
            request.root_cause,
            ctx.as_ref(),
        )
        .await?;

    let graph = ctx.map(|c| c.finalize(None));

    Ok(Json(ExecutionResponse::new(
        IncidentResponse::from(incident),
        graph,
    )))
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

/// Ingest a security event from internal core-bundle fanout
pub async fn ingest_event(
    Json(request): Json<IngestEventRequest>,
) -> (StatusCode, Json<IngestEventResponse>) {
    tracing::info!(
        execution_id = %request.execution_id,
        severity = %request.severity,
        source = %request.source,
        event_type = %request.event_type,
        "Inbound security event"
    );

    let execution_id = request.execution_id.clone();

    // Process asynchronously
    tokio::spawn(async move {
        tracing::debug!(
            execution_id = %request.execution_id,
            "Processing security event"
        );
        // Future: route to correlation engine, create incidents, etc.
    });

    (
        StatusCode::ACCEPTED,
        Json(IngestEventResponse {
            status: "accepted".to_string(),
            execution_id,
        }),
    )
}

#[derive(Debug, Deserialize)]
pub struct IngestEventRequest {
    pub source: String,
    pub event_type: String,
    pub execution_id: String,
    pub timestamp: String,
    pub severity: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct IngestEventResponse {
    pub status: String,
    pub execution_id: String,
}

/// Prometheus metrics endpoint
///
/// Returns metrics in Prometheus text exposition format
pub async fn metrics() -> (StatusCode, String) {
    let metrics = crate::metrics::gather_metrics();
    (StatusCode::OK, metrics)
}
