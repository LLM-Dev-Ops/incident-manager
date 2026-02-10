use crate::error::AppError;
use crate::execution::middleware::{
    attach_execution_graph_to_grpc_response, extract_execution_context_from_grpc_metadata,
};
use crate::grpc::conversions::*;
use crate::grpc::proto::alerts::*;
use crate::models::{Alert, IncidentType, Severity};
use crate::processing::IncidentProcessor;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct AlertIngestionServiceImpl {
    processor: Arc<IncidentProcessor>,
}

impl AlertIngestionServiceImpl {
    pub fn new(processor: Arc<IncidentProcessor>) -> Self {
        Self { processor }
    }

    fn app_error_to_status(error: AppError) -> Status {
        match error {
            AppError::NotFound(msg) => Status::not_found(msg),
            AppError::Validation(msg) => Status::invalid_argument(msg),
            AppError::RateLimit => Status::resource_exhausted("Rate limit exceeded"),
            _ => Status::internal(error.to_string()),
        }
    }
}

#[tonic::async_trait]
impl alert_ingestion_server::AlertIngestion for AlertIngestionServiceImpl {
    async fn submit_alert(
        &self,
        request: Request<CreateAlertRequest>,
    ) -> std::result::Result<Response<AlertResponse>, Status> {
        let exec_ctx = extract_execution_context_from_grpc_metadata(request.metadata()).ok();
        let create_req = request.into_inner();

        tracing::info!(
            source = %create_req.source,
            name = %create_req.name,
            "gRPC: Submitting alert"
        );

        // Generate alert ID
        let alert_id = uuid::Uuid::new_v4().to_string();

        // Parse severity
        let severity = match create_req.severity.as_str() {
            "P0" => Severity::P0,
            "P1" => Severity::P1,
            "P2" => Severity::P2,
            "P3" => Severity::P3,
            "P4" => Severity::P4,
            _ => Severity::P3,
        };

        // Convert proto to domain Alert
        let mut alert = Alert::new(
            alert_id.clone(),
            create_req.source,
            create_req.name,
            create_req.description,
            severity,
            IncidentType::Unknown, // Infer from labels if needed
        );

        alert.labels = create_req.labels;
        alert.annotations = create_req.annotations;
        alert.timestamp = chrono::Utc::now();

        // Process the alert
        let ack = self
            .processor
            .process_alert(alert.clone(), exec_ctx.as_ref())
            .await
            .map_err(Self::app_error_to_status)?;

        // Convert domain alert to proto Alert
        let proto_alert = crate::grpc::proto::alerts::Alert {
            id: alert_id.clone(),
            name: alert.title.clone(),
            description: alert.description.clone(),
            severity: match alert.severity {
                Severity::P0 => "P0".to_string(),
                Severity::P1 => "P1".to_string(),
                Severity::P2 => "P2".to_string(),
                Severity::P3 => "P3".to_string(),
                Severity::P4 => "P4".to_string(),
            },
            status: "FIRED".to_string(),
            source: alert.source.clone(),
            rule_id: String::new(),
            labels: alert.labels.clone(),
            annotations: alert.annotations.clone(),
            value: 0.0,
            threshold_operator: String::new(),
            threshold_value: 0.0,
            fired_at: datetime_to_timestamp(alert.timestamp),
            acknowledged_at: None,
            resolved_at: None,
            acknowledged_by: String::new(),
            resolved_by: String::new(),
            fingerprint: String::new(),
        };

        let mut response = Response::new(AlertResponse {
            alert: Some(proto_alert),
            message: format!("Alert processed successfully, incident: {:?}", ack.incident_id),
        });

        if let Some(ctx) = exec_ctx {
            let graph = ctx.finalize(None);
            attach_execution_graph_to_grpc_response(&mut response, &graph);
        }

        Ok(response)
    }

    type StreamAlertsStream =
        tokio_stream::wrappers::ReceiverStream<std::result::Result<AlertAck, Status>>;

    async fn stream_alerts(
        &self,
        request: Request<tonic::Streaming<AlertMessage>>,
    ) -> std::result::Result<Response<Self::StreamAlertsStream>, Status> {
        let mut stream = request.into_inner();

        tracing::info!("gRPC: Starting alert stream");

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let processor = self.processor.clone();

        tokio::spawn(async move {
            while let Ok(Some(alert_msg)) = stream.message().await {
                tracing::debug!(alert_id = %alert_msg.id, "Processing streamed alert");

                // Parse severity
                let severity = match alert_msg.severity.as_str() {
                    "P0" => Severity::P0,
                    "P1" => Severity::P1,
                    "P2" => Severity::P2,
                    "P3" => Severity::P3,
                    "P4" => Severity::P4,
                    _ => Severity::P3,
                };

                // Convert and process alert
                let mut alert = Alert::new(
                    alert_msg.id.clone(),
                    alert_msg.source,
                    alert_msg.name,
                    alert_msg.description,
                    severity,
                    IncidentType::Unknown,
                );

                alert.labels = alert_msg.labels;
                alert.annotations = alert_msg.annotations;
                alert.timestamp = timestamp_to_datetime(alert_msg.fired_at);

                match processor.process_alert(alert, None).await {
                    Ok(ack) => {
                        let response = AlertAck {
                            alert_id: alert_msg.id,
                            status: AckStatus::Accepted as i32,
                            message: format!("Alert processed, incident: {:?}", ack.incident_id),
                            timestamp: datetime_to_timestamp(chrono::Utc::now()),
                        };

                        if tx.send(Ok(response)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        if tx
                            .send(Err(Status::internal(format!("Processing error: {}", e))))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        ))
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> std::result::Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: HealthStatus::Healthy as i32,
            message: format!("Healthy - version {}", env!("CARGO_PKG_VERSION")),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc::proto::alerts::alert_ingestion_server::AlertIngestion;
    use crate::processing::DeduplicationEngine;
    use crate::state::InMemoryStore;

    fn setup_service() -> AlertIngestionServiceImpl {
        let store = Arc::new(InMemoryStore::new());
        let dedup = Arc::new(DeduplicationEngine::new(store.clone(), 900));
        let processor = Arc::new(IncidentProcessor::new(store, dedup));
        AlertIngestionServiceImpl::new(processor)
    }

    #[tokio::test]
    async fn test_submit_alert_grpc() {
        let service = setup_service();

        let request = Request::new(CreateAlertRequest {
            name: "High Latency".to_string(),
            description: "P95 > 5s".to_string(),
            severity: "P1".to_string(),
            source: "llm-sentinel".to_string(),
            rule_id: "latency-rule-1".to_string(),
            labels: std::collections::HashMap::new(),
            annotations: std::collections::HashMap::new(),
            value: 5000.0,
            threshold_operator: "gt".to_string(),
            threshold_value: 1000.0,
        });

        let response = service.submit_alert(request).await;
        assert!(response.is_ok());

        let resp = response.unwrap().into_inner();
        let alert = resp.alert.expect("alert should be present");
        assert!(!alert.id.is_empty());
    }

    #[tokio::test]
    async fn test_health_check_grpc() {
        let service = setup_service();

        let request = Request::new(HealthCheckRequest {
            service: String::new(),
        });
        let response = service.health_check(request).await;

        assert!(response.is_ok());
        let health = response.unwrap().into_inner();
        assert_eq!(health.status, HealthStatus::Healthy as i32);
    }
}
