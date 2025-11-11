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

        let incident = Incident::new(
            req.source,
            req.title,
            req.description,
            Severity::from(Severity::try_from(req.severity).unwrap_or(Severity::P3 as i32)),
            req.r#type.try_into().unwrap_or(crate::models::IncidentType::Unknown),
        );

        let created = self
            .processor
            .create_incident(incident)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(created.into()),
        }))
    }

    async fn get_incident(
        &self,
        request: Request<GetIncidentRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        let id = Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        tracing::debug!(incident_id = %id, "gRPC: Getting incident");

        let incident = self
            .processor
            .get_incident(&id)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(incident.into()),
        }))
    }

    async fn list_incidents(
        &self,
        request: Request<ListIncidentsRequest>,
    ) -> std::result::Result<Response<ListIncidentsResponse>, Status> {
        let req = request.into_inner();

        let filter = IncidentFilter {
            states: req
                .states
                .into_iter()
                .map(|s| IncidentState::try_from(s).unwrap_or(IncidentState::Detected))
                .collect(),
            severities: req
                .severities
                .into_iter()
                .map(|s| Severity::try_from(s).unwrap_or(Severity::P3))
                .collect(),
            sources: if req.source_filter.is_empty() {
                vec![]
            } else {
                vec![req.source_filter]
            },
            active_only: false,
        };

        let page = req.page;
        let page_size = req.page_size.min(100); // Max 100 per page

        tracing::debug!(page = page, page_size = page_size, "gRPC: Listing incidents");

        let incidents = self
            .processor
            .store
            .list_incidents(&filter, page as u32, page_size as u32)
            .await
            .map_err(Self::app_error_to_status)?;

        let total = self
            .processor
            .store
            .count_incidents(&filter)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(ListIncidentsResponse {
            incidents: incidents.into_iter().map(|i| i.into()).collect(),
            total: total as i32,
            page,
            page_size,
        }))
    }

    async fn update_incident(
        &self,
        request: Request<UpdateIncidentRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        let id = Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        tracing::info!(incident_id = %id, "gRPC: Updating incident");

        let mut incident = self
            .processor
            .get_incident(&id)
            .await
            .map_err(Self::app_error_to_status)?;

        // Update state if provided
        if let Some(new_state) = req.state {
            let state: IncidentState = IncidentState::try_from(new_state)
                .map_err(|_| Status::invalid_argument("Invalid state"))?;
            incident.update_state(state, "grpc-api".to_string());
        }

        // Update severity if provided
        if let Some(new_severity) = req.severity {
            incident.severity = Severity::try_from(new_severity)
                .map_err(|_| Status::invalid_argument("Invalid severity"))?;
        }

        // Update assignees
        if !req.assignees.is_empty() {
            incident.assignees = req.assignees;
        }

        self.processor
            .store
            .update_incident(&incident)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(incident.into()),
        }))
    }

    async fn resolve_incident(
        &self,
        request: Request<ResolveIncidentRequest>,
    ) -> std::result::Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();

        let id = Uuid::parse_str(&req.id).map_err(|_| Status::invalid_argument("Invalid UUID"))?;

        tracing::info!(incident_id = %id, "gRPC: Resolving incident");

        let method = crate::models::ResolutionMethod::try_from(req.method)
            .map_err(|_| Status::invalid_argument("Invalid resolution method"))?;

        let incident = self
            .processor
            .resolve_incident(&id, req.resolved_by, method, req.notes, req.root_cause)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(IncidentResponse {
            incident: Some(incident.into()),
        }))
    }

    type StreamIncidentsStream =
        tokio_stream::wrappers::ReceiverStream<std::result::Result<IncidentEvent, Status>>;

    async fn stream_incidents(
        &self,
        request: Request<StreamIncidentsRequest>,
    ) -> std::result::Result<Response<Self::StreamIncidentsStream>, Status> {
        let req = request.into_inner();

        tracing::info!("gRPC: Starting incident stream");

        let filter = IncidentFilter {
            states: req
                .states
                .into_iter()
                .map(|s| IncidentState::try_from(s).unwrap_or(IncidentState::Detected))
                .collect(),
            severities: req
                .min_severity
                .into_iter()
                .map(|s| Severity::try_from(s).unwrap_or(Severity::P3))
                .collect(),
            sources: vec![],
            active_only: true,
        };

        // Create channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        let processor = self.processor.clone();

        // Spawn background task to send incidents
        tokio::spawn(async move {
            // Get initial incidents
            match processor.store.list_incidents(&filter, 0, 100).await {
                Ok(incidents) => {
                    for incident in incidents {
                        let event = IncidentEvent {
                            incident_id: incident.id.to_string(),
                            event_type: EventType::EventTypeCreated as i32,
                            incident: Some(incident.into()),
                            timestamp: datetime_to_timestamp(chrono::Utc::now()),
                        };

                        if tx.send(Ok(event)).await.is_err() {
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
            source: "test-source".to_string(),
            title: "Test Incident".to_string(),
            description: "Test description".to_string(),
            severity: Severity::P1 as i32,
            r#type: IncidentType::IncidentTypeApplication as i32,
            affected_resources: vec!["service-a".to_string()],
            labels: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
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
            source: "test".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            severity: Severity::P2 as i32,
            r#type: IncidentType::IncidentTypeInfrastructure as i32,
            affected_resources: vec![],
            labels: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        });

        let create_response = service.create_incident(create_req).await.unwrap();
        let incident_id = create_response.into_inner().incident.unwrap().id;

        // Now get it
        let get_req = Request::new(GetIncidentRequest { id: incident_id });
        let response = service.get_incident(get_req).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_incident_grpc() {
        let service = setup_service();

        // Create incident
        let create_req = Request::new(CreateIncidentRequest {
            source: "test".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            severity: Severity::P1 as i32,
            r#type: IncidentType::IncidentTypeSecurity as i32,
            affected_resources: vec![],
            labels: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        });

        let create_response = service.create_incident(create_req).await.unwrap();
        let incident_id = create_response.into_inner().incident.unwrap().id;

        // Resolve it
        let resolve_req = Request::new(ResolveIncidentRequest {
            id: incident_id.clone(),
            resolved_by: "tester@example.com".to_string(),
            method: ResolutionMethod::ResolutionMethodManual as i32,
            notes: "Test resolution".to_string(),
            root_cause: Some("Test root cause".to_string()),
        });

        let response = service.resolve_incident(resolve_req).await;
        assert!(response.is_ok());

        let incident = response.unwrap().into_inner().incident.unwrap();
        assert!(incident.resolution.is_some());
    }
}
