use crate::error::AppError;
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
        request: Request<AlertMessage>,
    ) -> std::result::Result<Response<AlertAck>, Status> {
        let alert_msg = request.into_inner();

        tracing::info!(
            alert_id = %alert_msg.alert_id,
            source = %alert_msg.source,
            "gRPC: Submitting alert"
        );

        // Convert proto to domain Alert
        let mut alert = Alert::new(
            alert_msg.alert_id.clone(),
            alert_msg.source,
            alert_msg.title,
            alert_msg.description,
            Severity::try_from(alert_msg.severity).unwrap_or(Severity::P3),
            IncidentType::try_from(alert_msg.r#type)
                .unwrap_or(crate::models::IncidentType::Unknown),
        );

        alert.labels = alert_msg.labels;
        alert.affected_services = alert_msg.affected_services;
        alert.runbook_url = alert_msg.runbook_url;
        alert.annotations = alert_msg.annotations;
        alert.timestamp = timestamp_to_datetime(alert_msg.timestamp);

        // Process the alert
        let ack = self
            .processor
            .process_alert(alert)
            .await
            .map_err(Self::app_error_to_status)?;

        Ok(Response::new(AlertAck {
            alert_id: ack.alert_id.to_string(),
            incident_id: ack.incident_id.map(|id| id.to_string()).unwrap_or_default(),
            status: AckStatus::from(ack.status) as i32,
            message: ack.message,
            received_at: datetime_to_timestamp(ack.received_at),
        }))
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
                tracing::debug!(alert_id = %alert_msg.alert_id, "Processing streamed alert");

                // Convert and process alert
                let mut alert = Alert::new(
                    alert_msg.alert_id.clone(),
                    alert_msg.source,
                    alert_msg.title,
                    alert_msg.description,
                    Severity::try_from(alert_msg.severity).unwrap_or(Severity::P3),
                    IncidentType::try_from(alert_msg.r#type)
                        .unwrap_or(crate::models::IncidentType::Unknown),
                );

                alert.labels = alert_msg.labels;
                alert.affected_services = alert_msg.affected_services;
                alert.runbook_url = alert_msg.runbook_url;
                alert.annotations = alert_msg.annotations;
                alert.timestamp = timestamp_to_datetime(alert_msg.timestamp);

                match processor.process_alert(alert).await {
                    Ok(ack) => {
                        let response = AlertAck {
                            alert_id: ack.alert_id.to_string(),
                            incident_id: ack.incident_id.map(|id| id.to_string()).unwrap_or_default(),
                            status: AckStatus::from(ack.status) as i32,
                            message: ack.message,
                            received_at: datetime_to_timestamp(ack.received_at),
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
            status: HealthStatus::HealthStatusHealthy as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 0, // TODO: Track actual uptime
            metadata: std::collections::HashMap::new(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc::proto::incidents;
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

        let request = Request::new(AlertMessage {
            alert_id: "test-alert-123".to_string(),
            source: "llm-sentinel".to_string(),
            timestamp: datetime_to_timestamp(chrono::Utc::now()),
            severity: incidents::Severity::SeverityP1 as i32,
            r#type: incidents::IncidentType::IncidentTypePerformance as i32,
            title: "High Latency".to_string(),
            description: "P95 > 5s".to_string(),
            labels: std::collections::HashMap::new(),
            affected_services: vec!["api-service".to_string()],
            runbook_url: Some("https://runbooks.example.com/latency".to_string()),
            annotations: std::collections::HashMap::new(),
        });

        let response = service.submit_alert(request).await;
        assert!(response.is_ok());

        let ack = response.unwrap().into_inner();
        assert_eq!(ack.alert_id, "test-alert-123");
        assert!(!ack.incident_id.is_empty());
        assert_eq!(ack.status, AckStatus::AckStatusAccepted as i32);
    }

    #[tokio::test]
    async fn test_health_check_grpc() {
        let service = setup_service();

        let request = Request::new(HealthCheckRequest {});
        let response = service.health_check(request).await;

        assert!(response.is_ok());
        let health = response.unwrap().into_inner();
        assert_eq!(health.status, HealthStatus::HealthStatusHealthy as i32);
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}
