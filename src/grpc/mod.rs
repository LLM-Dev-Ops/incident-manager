pub mod conversions;
pub mod incident_service;
pub mod alert_service;
pub mod server;

pub use incident_service::IncidentServiceImpl;
pub use alert_service::AlertIngestionServiceImpl;
pub use server::start_grpc_server;

// Include generated proto code
pub mod proto {
    pub mod incidents {
        tonic::include_proto!("incidents");
    }
    pub mod alerts {
        tonic::include_proto!("alerts");
    }
    pub mod integrations {
        tonic::include_proto!("integrations");
    }
}
