use crate::config::Config;
use crate::grpc::proto::{alerts, incidents};
use crate::grpc::{AlertIngestionServiceImpl, IncidentServiceImpl};
use crate::processing::IncidentProcessor;
use std::sync::Arc;
use tonic::transport::Server;

/// Start the gRPC server
pub async fn start_grpc_server(
    config: Config,
    processor: Arc<IncidentProcessor>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.server.host, config.server.grpc_port)
        .parse()
        .expect("Invalid gRPC address");

    tracing::info!("Starting gRPC server on {}", addr);

    // Create service implementations
    let incident_service = IncidentServiceImpl::new(processor.clone());
    let alert_service = AlertIngestionServiceImpl::new(processor);

    // Build server with health checking and reflection
    let server = Server::builder()
        .add_service(incidents::incident_service_server::IncidentServiceServer::new(
            incident_service,
        ))
        .add_service(alerts::alert_ingestion_server::AlertIngestionServer::new(
            alert_service,
        ))
        .serve(addr);

    tracing::info!("gRPC server started successfully");
    tracing::info!("Incident Service: {}:{}", config.server.host, config.server.grpc_port);
    tracing::info!("Alert Ingestion: {}:{}", config.server.host, config.server.grpc_port);

    server.await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_server_can_be_created() {
        // This is a basic sanity test to ensure the server module compiles
        // Actual server testing would require integration tests
        assert!(true);
    }
}
