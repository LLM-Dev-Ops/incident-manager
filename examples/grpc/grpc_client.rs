/// Example gRPC client for LLM Incident Manager
///
/// This example demonstrates how to use the gRPC API to:
/// - Submit alerts
/// - Create incidents
/// - List and query incidents
/// - Resolve incidents
/// - Stream real-time incident updates

use llm_incident_manager::grpc::proto::{alerts, incidents};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let endpoint = std::env::var("GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:9000".to_string());

    println!("Connecting to gRPC server at {}", endpoint);

    // Create clients
    let mut incident_client =
        incidents::incident_service_client::IncidentServiceClient::connect(endpoint.clone())
            .await?;

    let mut alert_client =
        alerts::alert_ingestion_server::AlertIngestionClient::connect(endpoint).await?;

    // Example 1: Submit an alert
    println!("\n=== Example 1: Submit Alert ===");
    let alert_request = alerts::AlertMessage {
        id: "grpc-alert-001".to_string(),
        name: "High API Latency Detected".to_string(),
        description: "P95 latency exceeded 5 seconds".to_string(),
        severity: "P1".to_string(),
        source: "llm-sentinel".to_string(),
        labels: std::collections::HashMap::from([
            ("environment".to_string(), "production".to_string()),
            ("service".to_string(), "api-gateway".to_string()),
        ]),
        annotations: std::collections::HashMap::new(),
        fired_at: Some(prost_types::Timestamp {
            seconds: chrono::Utc::now().timestamp(),
            nanos: 0,
        }),
    };

    let alert_response = alert_client.submit_alert(alert_request).await?;
    let ack = alert_response.into_inner();
    println!("Alert submitted successfully!");
    println!("  Alert ID: {}", ack.alert_id);
    println!("  Incident ID: {}", ack.incident_id);
    println!("  Status: {:?}", alerts::AckStatus::try_from(ack.status));
    println!("  Message: {}", ack.message);

    // Example 2: Create an incident directly
    println!("\n=== Example 2: Create Incident ===");
    let create_request = incidents::CreateIncidentRequest {
        title: "Database Connection Pool Exhausted".to_string(),
        description: "All database connections in pool are in use".to_string(),
        severity: "P0".to_string(),
        source: "manual-entry".to_string(),
        metadata: std::collections::HashMap::from([
            ("database".to_string(), "postgres".to_string()),
            ("priority".to_string(), "critical".to_string()),
        ]),
        tags: vec!["postgres-primary".to_string()],
        assigned_to: "sre-team@example.com".to_string(),
    };

    let create_response = incident_client.create_incident(create_request).await?;
    let created_incident = create_response.into_inner().incident.unwrap();
    println!("Incident created successfully!");
    println!("  ID: {}", created_incident.id);
    println!("  Title: {}", created_incident.title);
    println!("  Severity: {}", created_incident.severity);
    println!("  State: {:?}", incidents::IncidentState::try_from(created_incident.incident_state));

    let incident_id = created_incident.id.clone();

    // Example 3: Get incident details
    println!("\n=== Example 3: Get Incident Details ===");
    let get_request = incidents::GetIncidentRequest {
        incident_id: incident_id.clone(),
    };

    let get_response = incident_client.get_incident(get_request).await?;
    let incident = get_response.into_inner().incident.unwrap();
    println!("Retrieved incident:");
    println!("  ID: {}", incident.id);
    println!("  Source: {}", incident.source);
    println!("  Title: {}", incident.title);
    println!("  Tags: {:?}", incident.tags);
    println!("  Timeline Events: {}", incident.timeline.len());

    // Example 4: List incidents
    println!("\n=== Example 4: List Incidents ===");
    let list_request = incidents::ListIncidentsRequest {
        status: Some("Open".to_string()),
        severity: Some("P0".to_string()),
        assigned_to: None,
        page: 1,
        page_size: 10,
        sort_by: "created_at".to_string(),
        sort_order: "desc".to_string(),
    };

    let list_response = incident_client.list_incidents(list_request).await?;
    let list = list_response.into_inner();
    println!("Found {} total incidents", list.total_count);
    println!("Showing page {} (size {})", list.page, list.page_size);
    for (i, inc) in list.incidents.iter().enumerate() {
        println!("  {}. {} - {} ({})", i + 1, inc.id, inc.title, inc.source);
    }

    // Example 5: Update incident
    println!("\n=== Example 5: Update Incident ===");
    let update_request = incidents::UpdateIncidentRequest {
        incident_id: incident_id.clone(),
        title: None,
        description: None,
        severity: Some("P1".to_string()),
        status: Some("Investigating".to_string()),
        assigned_to: Some("sre-team@example.com".to_string()),
        metadata: std::collections::HashMap::new(),
    };

    let update_response = incident_client.update_incident(update_request).await?;
    let updated = update_response.into_inner().incident.unwrap();
    println!("Incident updated:");
    println!("  New State: {:?}", incidents::IncidentState::try_from(updated.incident_state));
    println!("  Assigned To: {}", updated.assigned_to);

    // Example 6: Resolve incident
    println!("\n=== Example 6: Resolve Incident ===");
    let resolve_request = incidents::ResolveIncidentRequest {
        incident_id: incident_id.clone(),
        resolution_note: "Increased database connection pool size from 100 to 200. Monitored for 15 minutes with no issues.".to_string(),
        resolved_by: "sre-oncall@example.com".to_string(),
    };

    let resolve_response = incident_client.resolve_incident(resolve_request).await?;
    let resolved = resolve_response.into_inner().incident.unwrap();
    println!("Incident resolved:");
    println!("  State: {:?}", incidents::IncidentState::try_from(resolved.incident_state));
    if let Some(resolution) = resolved.resolution {
        println!("  Resolved By: {}", resolution.resolved_by);
        println!("  Method: {:?}", incidents::ResolutionMethod::try_from(resolution.method));
        println!("  Summary: {}", resolution.summary);
    }

    // Example 7: Stream incidents
    println!("\n=== Example 7: Stream Incidents (5 seconds) ===");
    let stream_request = incidents::StreamIncidentsRequest {
        statuses: vec!["Open".to_string(), "Acknowledged".to_string()],
        severities: vec!["P0".to_string(), "P1".to_string()],
    };

    let mut stream = incident_client
        .stream_incidents(stream_request)
        .await?
        .into_inner();

    println!("Listening for incident events...");
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            result = stream.message() => {
                match result {
                    Ok(Some(event)) => {
                        println!("  Event: {} - {}", event.incident_id,
                            incidents::EventType::try_from(event.event_type)
                                .map(|e| format!("{:?}", e))
                                .unwrap_or_else(|_| "Unknown".to_string())
                        );
                    }
                    Ok(None) => {
                        println!("Stream ended");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Stream error: {}", e);
                        break;
                    }
                }
            }
            _ = &mut timeout => {
                println!("Timeout reached, closing stream");
                break;
            }
        }
    }

    println!("\n✅ All examples completed successfully!");

    Ok(())
}
