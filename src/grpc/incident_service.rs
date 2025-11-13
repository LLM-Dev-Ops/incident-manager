use crate::error::{AppError, Result};
use crate::grpc::conversions::*;
use crate::grpc::proto::incidents::*;
use crate::models::{Incident, IncidentState, Severity};
use crate::processing::IncidentProcessor;
use crate::state::IncidentFilter;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct IncidentServiceImpl {
    processor: Arc<IncidentProcessor>,
}

impl IncidentServiceImpl {
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        Self { processor }
    }

    fn app_error_to_status(error: AppError) -> Status {
        match error {
            AppError::NotFound(msg) => Status::not_found(msg),
            AppError::Validation(msg) => Status::invalid_argument(msg),
            AppError::Authentication(msg) => Status::unauthenticated(msg),
            AppError::Authorization(msg) => Status::permission_denied(msg),
            AppError::RateLimit => Status::resource_exhausted("Rate limit exceeded"),
            AppError::Timeout(msg) => Status::deadline_exceeded(msg),
            AppError::InvalidStateTransition(msg) => Status::failed_precondition(msg),
            _ => Status::internal(error.to_string()),
        }
    }
}

#[tonic::async_trait]
impl incident_service_server::IncidentService for IncidentServiceImpl {
    async fn create_incident(
        &self,
        request: Request<CreateIncidentRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        tracing::info!(
            source = %req.source,
            title = %req.title,
            "gRPC: Creating incident"
        );

        // Parse severity from string
        let severity = match req.severity.as_str() {
            "P0" => Severity::P0,
            "P1" => Severity::P1,
            "P2" => Severity::P2,
            "P3" => Severity::P3,
            "P4" => Severity::P4,
            _ => Severity::P3,
        };

        let incident = Incident::new(
            req.source,
            req.title,
            req.description,
            severity,
            crate::models::IncidentType::Unknown,
        );

        let created = self
            .processor
            .create_incident(incident)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(created.into()),
            message: "Incident created successfully".to_string(),
        }))
    }

    async fn get_incident(
        &self,
        request: Request<GetIncidentRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        let id = Uuid::parse_str(&req.incident_id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        tracing::debug!(incident_id = %id, "gRPC: Getting incident");

        let incident = self
            .processor
            .get_incident(&id)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(incident.into()),
            message: String::new(),
        }))
    }

    async fn list_incidents(
        &self,
        request: Request<ListIncidentsRequest>,
    ) -> std::result::Result<Response<ListIncidentsResponse>, Status> {
        let req = request.into_inner();

        // Parse status and severity from strings
        let states = if let Some(status_str) = req.status {
            vec![match status_str.as_str() {
                "Open" => IncidentState::Detected,
                "Acknowledged" => IncidentState::Triaged,
                "Investigating" => IncidentState::Investigating,
                "Resolved" => IncidentState::Resolved,
                "Closed" => IncidentState::Closed,
                _ => IncidentState::Detected,
            }]
        } else {
            vec![]
        };

        let severities = if let Some(severity_str) = req.severity {
            vec![match severity_str.as_str() {
                "P0" => Severity::P0,
                "P1" => Severity::P1,
                "P2" => Severity::P2,
                "P3" => Severity::P3,
                "P4" => Severity::P4,
                _ => Severity::P3,
            }]
        } else {
            vec![]
        };

        let filter = IncidentFilter {
            states,
            severities,
            sources: vec![],
            active_only: false,
        };

        let page = req.page;
        let page_size = req.page_size.min(100); // Max 100 per page

        tracing::debug!(page = page, page_size = page_size, "gRPC: Listing incidents");

        let incidents = self
            .processor
            .store()
            .list_incidents(&filter, page as u32, page_size as u32)
            .await
            .map_err(Self::app_error_to_status)?;

        let total = self
            .processor
            .store()
            .count_incidents(&filter)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(ListIncidentsResponse {
            incidents: incidents.into_iter().map(|i| i.into()).collect(),
            total_count: total as i32,
            page,
            page_size,
        }))
    }

    async fn update_incident(
        &self,
        request: Request<UpdateIncidentRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        let id = Uuid::parse_str(&req.incident_id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        tracing::info!(incident_id = %id, "gRPC: Updating incident");

        let mut incident = self
            .processor
            .get_incident(&id)
            .await
            .map_err(Self::app_error_to_status)?;

        // Update title if provided
        if let Some(title) = req.title {
            incident.title = title;
        }

        // Update description if provided
        if let Some(description) = req.description {
            incident.description = description;
        }

        // Update state if provided
        if let Some(status_str) = req.status {
            let state = match status_str.as_str() {
                "Open" => IncidentState::Detected,
                "Acknowledged" => IncidentState::Triaged,
                "Investigating" => IncidentState::Investigating,
                "Resolved" => IncidentState::Resolved,
                "Closed" => IncidentState::Closed,
                _ => return Err(Status::invalid_argument("Invalid status")),
            };
            incident.update_state(state, "grpc-api".to_string());
        }

        // Update severity if provided
        if let Some(severity_str) = req.severity {
            incident.severity = match severity_str.as_str() {
                "P0" => Severity::P0,
                "P1" => Severity::P1,
                "P2" => Severity::P2,
                "P3" => Severity::P3,
                "P4" => Severity::P4,
                _ => return Err(Status::invalid_argument("Invalid severity")),
            };
        }

        // Update assigned_to if provided
        if let Some(assigned_to) = req.assigned_to {
            incident.assignees = vec![assigned_to];
        }

        self.processor
            .store()
            .update_incident(&incident)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(incident.into()),
            message: "Incident updated successfully".to_string(),
        }))
    }

    async fn resolve_incident(
        &self,
        request: Request<ResolveIncidentRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        let id = Uuid::parse_str(&req.incident_id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        tracing::info!(incident_id = %id, "gRPC: Resolving incident");

        let incident = self
            .processor
            .resolve_incident(&id, req.resolved_by, crate::models::ResolutionMethod::Manual, req.resolution_note, None)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(incident.into()),
            message: "Incident resolved successfully".to_string(),
        }))
    }

    async fn add_note(
        &self,
        request: Request<AddNoteRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        let id = Uuid::parse_str(&req.incident_id)
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        tracing::info!(incident_id = %id, "gRPC: Adding note to incident");

        let mut incident = self
            .processor
            .get_incident(&id)
            .await
            .map_err(Self::app_error_to_status)?;

        // Add note to incident
        incident.add_note(req.author, req.note);

        self.processor
            .store()
            .update_incident(&incident)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(incident.into()),
            message: "Note added successfully".to_string(),
        }))
    }

    type StreamIncidentsStream =
        tokio_stream::wrappers::ReceiverStream<std::result::Result<IncidentUpdate, Status>>;

    async fn stream_incidents(
        &self,
        request: Request<StreamIncidentsRequest>,
    ) -> std::result::Result<Response<Self::StreamIncidentsStream>, Status> {
        let req = request.into_inner();

        tracing::info!("gRPC: Starting incident stream");

        // Parse statuses and severities from strings
        let states: Vec<IncidentState> = req
            .statuses
            .into_iter()
            .map(|s| match s.as_str() {
                "Open" => IncidentState::Detected,
                "Acknowledged" => IncidentState::Triaged,
                "Investigating" => IncidentState::Investigating,
                "Resolved" => IncidentState::Resolved,
                "Closed" => IncidentState::Closed,
                _ => IncidentState::Detected,
            })
            .collect();

        let severities: Vec<Severity> = req
            .severities
            .into_iter()
            .map(|s| match s.as_str() {
                "P0" => Severity::P0,
                "P1" => Severity::P1,
                "P2" => Severity::P2,
                "P3" => Severity::P3,
                "P4" => Severity::P4,
                _ => Severity::P3,
            })
            .collect();

        let filter = IncidentFilter {
            states,
            severities,
            sources: vec![],
            active_only: true,
        };

        // Create channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        let processor = self.processor.clone();

        // Spawn background task to send incidents
        tokio::spawn(async move {
            // Get initial incidents
            match processor.store().list_incidents(&filter, 0, 100).await {
                Ok(incidents) => {
                    for incident in incidents {
                        let update = IncidentUpdate {
                            incident_id: incident.id.to_string(),
                            update_type: "created".to_string(),
                            incident: Some(incident.into()),
                            timestamp: datetime_to_timestamp(chrono::Utc::now()),
                        };

                        if tx.send(Ok(update)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Err(Status::internal(format!(
                            "Failed to fetch incidents: {}",
                            e
                        ))))
                        .await;
                }
            }
        });

        Ok(Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::DeduplicationEngine;
    use crate::state::InMemoryStore;

    fn setup_service() -> IncidentServiceImpl {
        let store = Arc::new(InMemoryStore::new());
        let dedup = Arc::new(DeduplicationEngine::new(store.clone(), 900));
        let processor = Arc::new(IncidentProcessor::new(store, dedup));
        IncidentServiceImpl::new(processor)
    }

    #[tokio::test]
    async fn test_create_incident_grpc() {
        let service = setup_service();

        let request = Request::new(CreateIncidentRequest {
            title: "Test Incident".to_string(),
            description: "Test description".to_string(),
            severity: "P1".to_string(),
            source: "test-source".to_string(),
            metadata: std::collections::HashMap::new(),
            tags: vec![],
            assigned_to: String::new(),
        });

        let response = service.create_incident(request).await;
        assert!(response.is_ok());

        let incident = response.unwrap().into_inner().incident.unwrap();
        assert_eq!(incident.source, "test-source");
        assert_eq!(incident.title, "Test Incident");
    }

    #[tokio::test]
    async fn test_get_incident_grpc() {
        let service = setup_service();

        // First create an incident
        let create_req = Request::new(CreateIncidentRequest {
            title: "Test".to_string(),
            description: "Desc".to_string(),
            severity: "P2".to_string(),
            source: "test".to_string(),
            metadata: std::collections::HashMap::new(),
            tags: vec![],
            assigned_to: String::new(),
        });

        let create_response = service.create_incident(create_req).await.unwrap();
        let incident_id = create_response.into_inner().incident.unwrap().id;

        // Now get it
        let get_req = Request::new(GetIncidentRequest { incident_id });
        let response = service.get_incident(get_req).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_incident_grpc() {
        let service = setup_service();

        // Create incident
        let create_req = Request::new(CreateIncidentRequest {
            title: "Test".to_string(),
            description: "Desc".to_string(),
            severity: "P1".to_string(),
            source: "test".to_string(),
            metadata: std::collections::HashMap::new(),
            tags: vec![],
            assigned_to: String::new(),
        });

        let create_response = service.create_incident(create_req).await.unwrap();
        let incident_id = create_response.into_inner().incident.unwrap().id;

        // Resolve it
        let resolve_req = Request::new(ResolveIncidentRequest {
            incident_id: incident_id.clone(),
            resolution_note: "Test resolution".to_string(),
            resolved_by: "tester@example.com".to_string(),
        });

        let response = service.resolve_incident(resolve_req).await;
        assert!(response.is_ok());

        let incident = response.unwrap().into_inner().incident.unwrap();
        assert!(incident.resolution.is_some());
    }
}
