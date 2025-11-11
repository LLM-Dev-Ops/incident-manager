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
        alert_id: "grpc-alert-001".to_string(),
        source: "llm-sentinel".to_string(),
        timestamp: Some(prost_types::Timestamp {
            seconds: chrono::Utc::now().timestamp(),
            nanos: 0,
        }),
        severity: incidents::Severity::SeverityP1 as i32,
        r#type: incidents::IncidentType::IncidentTypePerformance as i32,
        title: "High API Latency Detected".to_string(),
        description: "P95 latency exceeded 5 seconds".to_string(),
        labels: std::collections::HashMap::from([
            ("environment".to_string(), "production".to_string()),
            ("service".to_string(), "api-gateway".to_string()),
        ]),
        affected_services: vec!["api-gateway".to_string(), "backend-api".to_string()],
        runbook_url: Some("https://runbooks.example.com/latency".to_string()),
        annotations: std::collections::HashMap::new(),
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
        source: "manual-entry".to_string(),
        title: "Database Connection Pool Exhausted".to_string(),
        description: "All database connections in pool are in use".to_string(),
        severity: incidents::Severity::SeverityP0 as i32,
        r#type: incidents::IncidentType::IncidentTypeInfrastructure as i32,
        affected_resources: vec!["postgres-primary".to_string()],
        labels: std::collections::HashMap::from([
            ("database".to_string(), "postgres".to_string()),
            ("priority".to_string(), "critical".to_string()),
        ]),
        metadata: std::collections::HashMap::new(),
    };

    let create_response = incident_client.create_incident(create_request).await?;
    let created_incident = create_response.into_inner().incident.unwrap();
    println!("Incident created successfully!");
    println!("  ID: {}", created_incident.id);
    println!("  Title: {}", created_incident.title);
    println!("  Severity: {:?}", incidents::Severity::try_from(created_incident.severity));
    println!("  State: {:?}", incidents::IncidentState::try_from(created_incident.state));

    let incident_id = created_incident.id.clone();

    // Example 3: Get incident details
    println!("\n=== Example 3: Get Incident Details ===");
    let get_request = incidents::GetIncidentRequest {
        id: incident_id.clone(),
    };

    let get_response = incident_client.get_incident(get_request).await?;
    let incident = get_response.into_inner().incident.unwrap();
    println!("Retrieved incident:");
    println!("  ID: {}", incident.id);
    println!("  Source: {}", incident.source);
    println!("  Title: {}", incident.title);
    println!("  Affected Resources: {:?}", incident.affected_resources);
    println!("  Timeline Events: {}", incident.timeline.len());

    // Example 4: List incidents
    println!("\n=== Example 4: List Incidents ===");
    let list_request = incidents::ListIncidentsRequest {
        page: 0,
        page_size: 10,
        states: vec![
            incidents::IncidentState::IncidentStateDetected as i32,
            incidents::IncidentState::IncidentStateTriaged as i32,
        ],
        severities: vec![
            incidents::Severity::SeverityP0 as i32,
            incidents::Severity::SeverityP1 as i32,
        ],
        source_filter: String::new(),
    };

    let list_response = incident_client.list_incidents(list_request).await?;
    let list = list_response.into_inner();
    println!("Found {} total incidents", list.total);
    println!("Showing page {} (size {})", list.page, list.page_size);
    for (i, inc) in list.incidents.iter().enumerate() {
        println!("  {}. {} - {} ({})", i + 1, inc.id, inc.title, inc.source);
    }

    // Example 5: Update incident
    println!("\n=== Example 5: Update Incident ===");
    let update_request = incidents::UpdateIncidentRequest {
        id: incident_id.clone(),
        severity: Some(incidents::Severity::SeverityP1 as i32),
        state: Some(incidents::IncidentState::IncidentStateInvestigating as i32),
        assignees: vec!["sre-team@example.com".to_string()],
    };

    let update_response = incident_client.update_incident(update_request).await?;
    let updated = update_response.into_inner().incident.unwrap();
    println!("Incident updated:");
    println!("  New State: {:?}", incidents::IncidentState::try_from(updated.state));
    println!("  Assignees: {:?}", updated.assignees);

    // Example 6: Resolve incident
    println!("\n=== Example 6: Resolve Incident ===");
    let resolve_request = incidents::ResolveIncidentRequest {
        id: incident_id.clone(),
        resolved_by: "sre-oncall@example.com".to_string(),
        method: incidents::ResolutionMethod::ResolutionMethodManual as i32,
        notes: "Increased database connection pool size from 100 to 200. Monitored for 15 minutes with no issues.".to_string(),
        root_cause: Some("Connection pool size was undersized for current traffic load".to_string()),
    };

    let resolve_response = incident_client.resolve_incident(resolve_request).await?;
    let resolved = resolve_response.into_inner().incident.unwrap();
    println!("Incident resolved:");
    println!("  State: {:?}", incidents::IncidentState::try_from(resolved.state));
    if let Some(resolution) = resolved.resolution {
        println!("  Resolved By: {}", resolution.resolved_by);
        println!("  Method: {:?}", incidents::ResolutionMethod::try_from(resolution.method));
        println!("  Notes: {}", resolution.notes);
        if let Some(root_cause) = resolution.root_cause {
            println!("  Root Cause: {}", root_cause);
        }
    }

    // Example 7: Stream incidents
    println!("\n=== Example 7: Stream Incidents (5 seconds) ===");
    let stream_request = incidents::StreamIncidentsRequest {
        states: vec![],
        min_severity: vec![incidents::Severity::SeverityP0 as i32],
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
