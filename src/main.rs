use llm_incident_manager::{
    api::{build_router, AppState},
    config::Config,
    correlation::{CorrelationConfig, CorrelationEngine},
    escalation::EscalationEngine,
    grpc::start_grpc_server,
    notifications::NotificationService,
    playbooks::PlaybookService,
    processing::{DeduplicationEngine, IncidentProcessor},
    state::create_store,
    websocket::{WebSocketConfig, WebSocketState},
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_incident_manager=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        eprintln!("Using default configuration");
        default_config()
    });

    tracing::info!("Starting LLM Incident Manager v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Deployment mode: {:?}", config.deployment.mode);

    // Initialize Prometheus metrics
    if config.observability.prometheus_enabled {
        if let Err(e) = llm_incident_manager::metrics::init_metrics() {
            tracing::warn!("Failed to initialize metrics: {}", e);
            tracing::warn!("Continuing without metrics");
        } else {
            tracing::info!("✅ Prometheus metrics initialized");
        }
    } else {
        tracing::info!("⚠️  Prometheus metrics disabled in configuration");
    }

    // Initialize storage backend
    tracing::info!("Storage backend: {:?}", config.state.backend);
    let store = create_store(&config.state).await?;
    tracing::info!("✅ Storage backend initialized");

    // Initialize components
    let dedup_engine = Arc::new(DeduplicationEngine::new(
        store.clone(),
        config.processing.deduplication_window_secs as i64,
    ));

    // Initialize notification service
    let notification_service = match NotificationService::new(config.notifications.clone(), store.clone()) {
        Ok(service) => {
            tracing::info!("✅ Notification service initialized");
            Some(Arc::new(service))
        }
        Err(e) => {
            tracing::warn!("⚠️  Notification service initialization failed: {}", e);
            tracing::warn!("   Continuing without notifications");
            None
        }
    };

    // Initialize playbook service
    let playbook_service = Arc::new(PlaybookService::new(
        store.clone(),
        notification_service.clone(),
        true, // Enable auto-execution
    ));
    tracing::info!("✅ Playbook service initialized with auto-execution enabled");

    // Initialize escalation engine
    let escalation_engine = Arc::new(EscalationEngine::new(
        notification_service.clone(),
        store.clone(),
    ).with_check_interval(30));
    tracing::info!("✅ Escalation engine initialized");

    // Spawn escalation monitor
    let monitor_engine = escalation_engine.clone();
    tokio::spawn(async move {
        monitor_engine.run_monitor().await;
    });
    tracing::info!("✅ Escalation monitor started");

    // Initialize routing rule evaluator
    let routing_evaluator = Arc::new(llm_incident_manager::escalation::RoutingRuleEvaluator::new(
        Some(playbook_service.clone()),
    ));
    tracing::info!("✅ Routing rule evaluator initialized");

    // Initialize correlation engine
    let correlation_config = if config.processing.correlation_enabled {
        CorrelationConfig::default()
    } else {
        CorrelationConfig {
            enabled: false,
            ..Default::default()
        }
    };

    let correlation_engine = Arc::new(CorrelationEngine::new(
        correlation_config,
        store.clone(),
    ));

    if config.processing.correlation_enabled {
        if let Err(e) = correlation_engine.start().await {
            tracing::error!("Failed to start correlation engine: {}", e);
        } else {
            tracing::info!("✅ Correlation engine initialized and started");
        }
    } else {
        tracing::info!("⚠️  Correlation engine disabled in configuration");
    }

    // Create processor with optional services
    let mut processor = IncidentProcessor::new(store, dedup_engine);
    if let Some(notif_service) = notification_service.clone() {
        processor.set_notification_service(notif_service);
        tracing::info!("✅ Notification service integrated with processor");
    }
    processor.set_playbook_service(playbook_service.clone());
    tracing::info!("✅ Playbook service integrated with processor");

    processor.set_escalation_engine(escalation_engine.clone());
    tracing::info!("✅ Escalation engine integrated with processor");

    processor.set_routing_evaluator(routing_evaluator.clone());
    tracing::info!("✅ Routing rule evaluator integrated with processor");

    if config.processing.correlation_enabled {
        processor.set_correlation_engine(correlation_engine.clone());
        tracing::info!("✅ Correlation engine integrated with processor");
    }

    // Initialize WebSocket state
    let ws_config = WebSocketConfig::default();
    let ws_state = Arc::new(WebSocketState::new(ws_config));
    tracing::info!("✅ WebSocket server initialized");

    // Integrate WebSocket handlers with processor (before Arc::new)
    processor.set_websocket_handlers(Arc::new(ws_state.handlers.clone()));
    tracing::info!("✅ WebSocket event handlers integrated with processor");

    // Spawn WebSocket cleanup task
    let cleanup_state = ws_state.clone();
    tokio::spawn(async move {
        llm_incident_manager::websocket::cleanup_task(cleanup_state).await;
    });
    tracing::info!("✅ WebSocket cleanup task started");

    let processor = Arc::new(processor);

    // Create application state for HTTP API with WebSocket
    let app_state = AppState::new(processor.clone()).with_websocket(ws_state.clone());

    // Build HTTP router with REST API
    let app = build_router(app_state.clone());

    // Note: GraphQL routes are now served separately or integrated directly into build_router
    // For now, we'll just use the REST router as the main app

    // Start HTTP server
    let http_addr = format!("{}:{}", config.server.host, config.server.http_port);
    let http_listener = tokio::net::TcpListener::bind(&http_addr).await?;

    tracing::info!("🚀 HTTP API server listening on http://{}", http_addr);
    tracing::info!("   Health check: http://{}/health", http_addr);
    tracing::info!("   REST API: http://{}/v1/incidents", http_addr);
    tracing::info!("   GraphQL API: http://{}/graphql", http_addr);
    tracing::info!("   GraphQL Playground: http://{}/graphql/playground", http_addr);
    tracing::info!("   GraphQL WebSocket: ws://{}/graphql/ws", http_addr);
    tracing::info!("   WebSocket Streaming: ws://{}/ws", http_addr);

    // Clone config for gRPC server
    let grpc_config = config.clone();
    let grpc_processor = processor.clone();

    // Spawn gRPC server in separate task
    let grpc_handle = tokio::spawn(async move {
        tracing::info!(
            "🚀 gRPC server listening on {}:{}",
            grpc_config.server.host,
            grpc_config.server.grpc_port
        );
        tracing::info!("   Incident Service: grpc://{}:{}", grpc_config.server.host, grpc_config.server.grpc_port);
        tracing::info!("   Alert Ingestion: grpc://{}:{}", grpc_config.server.host, grpc_config.server.grpc_port);

        if let Err(e) = start_grpc_server(grpc_config, grpc_processor).await {
            tracing::error!("gRPC server error: {}", e);
        }
    });

    tracing::info!("✅ All servers started successfully");
    tracing::info!("Press Ctrl+C to shutdown");

    // Run HTTP server
    let http_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(http_listener, app).await {
            tracing::error!("HTTP server error: {}", e);
        }
    });

    // Wait for both servers
    tokio::select! {
        _ = http_handle => {
            tracing::warn!("HTTP server stopped");
        }
        _ = grpc_handle => {
            tracing::warn!("gRPC server stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received");
        }
    }

    tracing::info!("Shutting down gracefully...");
    Ok(())
}

fn default_config() -> Config {
    use llm_incident_manager::config::*;

    Config {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            http_port: 8080,
            grpc_port: 9000,
            metrics_port: 9090,
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
            request_timeout_secs: 30,
            max_connections: 10000,
        },
        deployment: DeploymentConfig {
            mode: DeploymentMode::Standalone,
            worker_type: None,
            region: None,
            availability_zones: vec![],
        },
        state: StateConfig {
            backend: StateBackend::Sled,
            path: Some("./data/state".into()),
            redis_url: None,
            redis_cluster_nodes: vec![],
            pool_size: 100,
        },
        messaging: None,
        integrations: IntegrationsConfig::default(),
        observability: ObservabilityConfig {
            log_level: "info".to_string(),
            json_logs: false,
            otlp_enabled: false,
            otlp_endpoint: None,
            service_name: "llm-incident-manager".to_string(),
            prometheus_enabled: true,
        },
        processing: ProcessingConfig {
            max_concurrent_incidents: 10000,
            processing_timeout_secs: 300,
            deduplication_enabled: true,
            deduplication_window_secs: 900,
            correlation_enabled: true,
        },
        notifications: NotificationConfig {
            slack_enabled: false,
            slack_webhook_env: Some("SLACK_WEBHOOK_URL".to_string()),
            slack_bot_token_env: Some("SLACK_BOT_TOKEN".to_string()),
            slack_default_channel: Some("#incidents".to_string()),
            email_enabled: false,
            smtp_server: Some("smtp.example.com".to_string()),
            smtp_port: 587,
            smtp_use_tls: true,
            smtp_username_env: Some("SMTP_USERNAME".to_string()),
            smtp_password_env: Some("SMTP_PASSWORD".to_string()),
            email_from: Some("incidents@example.com".to_string()),
            email_from_name: Some("LLM Incident Manager".to_string()),
            pagerduty_enabled: false,
            pagerduty_api_token_env: Some("PAGERDUTY_API_TOKEN".to_string()),
            pagerduty_integration_key_env: Some("PAGERDUTY_INTEGRATION_KEY".to_string()),
            pagerduty_api_url: "https://events.pagerduty.com/v2/enqueue".to_string(),
            webhook_enabled: true,
            default_webhook_url: None,
            webhook_timeout_secs: 10,
            max_retries: 3,
            retry_backoff_secs: 5,
            queue_size: 10000,
            worker_threads: 4,
        },
    }
}
