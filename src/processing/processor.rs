use crate::correlation::CorrelationEngine;
use crate::enrichment::EnrichmentService;
use crate::error::{AppError, Result};
use crate::escalation::{EscalationEngine, RoutingRuleEvaluator};
use crate::execution::{Artifact, ExecutionContext};
use crate::ml::MLService;
use crate::models::{Alert, AlertAck, Incident, IncidentState};
use crate::notifications::NotificationService;
use crate::playbooks::PlaybookService;
use crate::processing::DeduplicationEngine;
use crate::state::IncidentStore;
use crate::websocket::EventHandlers;
use std::sync::Arc;
use uuid::Uuid;

/// Main incident processor
pub struct IncidentProcessor {
    store: Arc<dyn IncidentStore>,
    dedup_engine: Arc<DeduplicationEngine>,
    notification_service: Option<Arc<NotificationService>>,
    playbook_service: Option<Arc<PlaybookService>>,
    escalation_engine: Option<Arc<EscalationEngine>>,
    routing_evaluator: Option<Arc<RoutingRuleEvaluator>>,
    correlation_engine: Option<Arc<CorrelationEngine>>,
    ml_service: Option<Arc<MLService>>,
    enrichment_service: Option<Arc<EnrichmentService>>,
    websocket_handlers: Option<Arc<EventHandlers>>,
}

impl IncidentProcessor {
    pub fn new(store: Arc<dyn IncidentStore>, dedup_engine: Arc<DeduplicationEngine>) -> Self {
        Self {
            store,
            dedup_engine,
            notification_service: None,
            playbook_service: None,
            escalation_engine: None,
            routing_evaluator: None,
            correlation_engine: None,
            ml_service: None,
            enrichment_service: None,
            websocket_handlers: None,
        }
    }

    /// Get a reference to the incident store
    pub fn store(&self) -> &Arc<dyn IncidentStore> {
        &self.store
    }

    /// Set the notification service (for optional notification integration)
    pub fn with_notifications(mut self, notification_service: Arc<NotificationService>) -> Self {
        self.notification_service = Some(notification_service);
        self
    }

    /// Set notification service after construction
    pub fn set_notification_service(&mut self, notification_service: Arc<NotificationService>) {
        self.notification_service = Some(notification_service);
    }

    /// Set playbook service after construction
    pub fn set_playbook_service(&mut self, playbook_service: Arc<PlaybookService>) {
        self.playbook_service = Some(playbook_service);
    }

    /// Set escalation engine after construction
    pub fn set_escalation_engine(&mut self, escalation_engine: Arc<EscalationEngine>) {
        self.escalation_engine = Some(escalation_engine);
    }

    /// Set routing rule evaluator after construction
    pub fn set_routing_evaluator(&mut self, routing_evaluator: Arc<RoutingRuleEvaluator>) {
        self.routing_evaluator = Some(routing_evaluator);
    }

    /// Set correlation engine after construction
    pub fn set_correlation_engine(&mut self, correlation_engine: Arc<CorrelationEngine>) {
        self.correlation_engine = Some(correlation_engine);
    }

    /// Set ML service after construction
    pub fn set_ml_service(&mut self, ml_service: Arc<MLService>) {
        self.ml_service = Some(ml_service);
    }

    /// Set enrichment service after construction
    pub fn set_enrichment_service(&mut self, enrichment_service: Arc<EnrichmentService>) {
        self.enrichment_service = Some(enrichment_service);
    }

    /// Set WebSocket event handlers after construction
    pub fn set_websocket_handlers(&mut self, handlers: Arc<EventHandlers>) {
        self.websocket_handlers = Some(handlers);
    }

    /// Process an incoming alert
    pub async fn process_alert(
        &self,
        mut alert: Alert,
        exec_ctx: Option<&ExecutionContext>,
    ) -> Result<AlertAck> {
        tracing::info!(
            alert_id = %alert.id,
            source = %alert.source,
            severity = ?alert.severity,
            "Processing alert"
        );

        // Check for duplicates
        let duplicate_result = if let Some(ctx) = exec_ctx {
            execute_agent!(ctx, "DeduplicationEngine", {
                self.dedup_engine.find_duplicate(&alert).await
            })
        } else {
            self.dedup_engine.find_duplicate(&alert).await
        };

        if let Some(mut existing_incident) = duplicate_result? {
            tracing::info!(
                alert_id = %alert.id,
                incident_id = %existing_incident.id,
                "Alert is a duplicate, merging into existing incident"
            );

            alert.deduplicated = true;
            alert.parent_alert_id = Some(existing_incident.id);
            alert.incident_id = Some(existing_incident.id);

            // Merge into existing incident
            self.dedup_engine
                .merge_into_incident(&alert, &mut existing_incident)
                .await?;

            return Ok(AlertAck::duplicate(alert.id, existing_incident.id));
        }

        // Convert alert to incident
        let mut incident = alert.to_incident();

        // Generate and set fingerprint
        incident.fingerprint = Some(incident.generate_fingerprint());

        // Save incident
        self.store.save_incident(&incident).await?;

        // Update alert with incident ID
        alert.incident_id = Some(incident.id);

        tracing::info!(
            alert_id = %alert.id,
            incident_id = %incident.id,
            "Created new incident from alert"
        );

        // Publish WebSocket events
        if let Some(ref ws_handlers) = self.websocket_handlers {
            ws_handlers.alerts.on_alert_received(alert.clone()).await;
            ws_handlers
                .alerts
                .on_alert_converted(alert.clone(), incident.id)
                .await;
            ws_handlers
                .incidents
                .on_incident_created(incident.clone())
                .await;
        }

        // Run agent pipeline
        self.run_agent_pipeline(&incident, exec_ctx).await;

        Ok(AlertAck::accepted(alert.id, incident.id))
    }

    /// Create a new incident directly
    pub async fn create_incident(
        &self,
        mut incident: Incident,
        exec_ctx: Option<&ExecutionContext>,
    ) -> Result<Incident> {
        // Generate fingerprint if not present
        if incident.fingerprint.is_none() {
            incident.fingerprint = Some(incident.generate_fingerprint());
        }

        // Check for duplicates
        let is_dup = if let Some(ctx) = exec_ctx {
            execute_agent!(ctx, "DeduplicationEngine", {
                self.dedup_engine.is_duplicate_incident(&incident).await
            })
        } else {
            self.dedup_engine.is_duplicate_incident(&incident).await
        }?;

        if is_dup {
            return Err(AppError::Validation(
                "Incident appears to be a duplicate".to_string(),
            ));
        }

        // Save incident
        self.store.save_incident(&incident).await?;

        tracing::info!(
            incident_id = %incident.id,
            severity = ?incident.severity,
            "Created new incident"
        );

        // Run agent pipeline
        self.run_agent_pipeline(&incident, exec_ctx).await;

        Ok(incident)
    }

    /// Run the post-creation agent pipeline (enrichment, notifications, playbooks,
    /// routing, escalation, correlation, ML). Shared between process_alert and create_incident.
    async fn run_agent_pipeline(
        &self,
        incident: &Incident,
        exec_ctx: Option<&ExecutionContext>,
    ) {
        // Enrich incident with additional context
        if let Some(ref enrichment_service) = self.enrichment_service {
            if let Some(ctx) = exec_ctx {
                let mut guard = ctx.start_agent_span("EnrichmentService");
                match enrichment_service.enrich_incident(incident).await {
                    Ok(context) => {
                        guard.add_artifact(Artifact {
                            name: "enrichment_result".to_string(),
                            artifact_type: "enrichment_context".to_string(),
                            reference: format!("incident:{}", incident.id),
                            data: serde_json::json!({
                                "total_enrichers": context.total_enrichers(),
                                "successful": context.successful_enrichers.len(),
                                "duration_ms": context.enrichment_duration_ms,
                            }),
                            created_at: chrono::Utc::now(),
                        });
                        tracing::info!(
                            incident_id = %incident.id,
                            enrichers = context.total_enrichers(),
                            successful = context.successful_enrichers.len(),
                            duration_ms = context.enrichment_duration_ms,
                            "Incident enriched with context"
                        );
                        guard.complete_ok(vec![]);
                    }
                    Err(e) => {
                        tracing::error!(
                            incident_id = %incident.id,
                            error = %e,
                            "Failed to enrich incident context"
                        );
                        guard.complete_err(format!("{}", e));
                    }
                }
            } else {
                match enrichment_service.enrich_incident(incident).await {
                    Ok(context) => {
                        tracing::info!(
                            incident_id = %incident.id,
                            enrichers = context.total_enrichers(),
                            successful = context.successful_enrichers.len(),
                            duration_ms = context.enrichment_duration_ms,
                            "Incident enriched with context"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            incident_id = %incident.id,
                            error = %e,
                            "Failed to enrich incident context"
                        );
                    }
                }
            }
        }

        // Send notifications
        if let Some(ref notif_service) = self.notification_service {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("NotificationService");
                match notif_service.notify_incident_detected(incident).await {
                    Ok(ids) => {
                        guard.complete_ok(vec![Artifact {
                            name: "notification_dispatch".to_string(),
                            artifact_type: "notification_ids".to_string(),
                            reference: format!("incident:{}", incident.id),
                            data: serde_json::json!({ "notification_count": ids.len() }),
                            created_at: chrono::Utc::now(),
                        }]);
                    }
                    Err(e) => {
                        tracing::error!(
                            incident_id = %incident.id,
                            error = %e,
                            "Failed to send incident detection notification"
                        );
                        guard.complete_err(format!("{}", e));
                    }
                }
            } else if let Err(e) = notif_service.notify_incident_detected(incident).await {
                tracing::error!(
                    incident_id = %incident.id,
                    error = %e,
                    "Failed to send incident detection notification"
                );
            }
        }

        // Auto-execute playbooks
        if let Some(ref playbook_service) = self.playbook_service {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("PlaybookService");
                let executions = playbook_service
                    .auto_execute_for_incident(incident)
                    .await;
                if !executions.is_empty() {
                    tracing::info!(
                        incident_id = %incident.id,
                        execution_count = executions.len(),
                        "Auto-executed playbooks for incident"
                    );
                }
                guard.complete_ok(vec![Artifact {
                    name: "playbook_executions".to_string(),
                    artifact_type: "playbook_result".to_string(),
                    reference: format!("incident:{}", incident.id),
                    data: serde_json::json!({ "execution_count": executions.len() }),
                    created_at: chrono::Utc::now(),
                }]);
            } else {
                let executions = playbook_service
                    .auto_execute_for_incident(incident)
                    .await;
                if !executions.is_empty() {
                    tracing::info!(
                        incident_id = %incident.id,
                        execution_count = executions.len(),
                        "Auto-executed playbooks for incident"
                    );
                }
            }
        }

        // Apply routing rules
        if let Some(ref routing_evaluator) = self.routing_evaluator {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("RoutingRuleEvaluator");
                let matches = routing_evaluator.evaluate_incident(incident);
                if !matches.is_empty() {
                    tracing::info!(
                        incident_id = %incident.id,
                        rule_count = matches.len(),
                        "Routing rules matched"
                    );

                    if let Ok(action_result) =
                        routing_evaluator.apply_actions(incident, &matches).await
                    {
                        if !action_result.suggested_assignees.is_empty() {
                            tracing::info!(
                                incident_id = %incident.id,
                                assignees = ?action_result.suggested_assignees,
                                "Routing suggested assignees"
                            );
                        }
                        guard.complete_ok(vec![Artifact {
                            name: "routing_result".to_string(),
                            artifact_type: "routing_evaluation".to_string(),
                            reference: format!("incident:{}", incident.id),
                            data: serde_json::json!({
                                "matched_rules": matches.len(),
                                "suggested_assignees": action_result.suggested_assignees,
                            }),
                            created_at: chrono::Utc::now(),
                        }]);
                    } else {
                        guard.complete_ok(vec![Artifact {
                            name: "routing_result".to_string(),
                            artifact_type: "routing_evaluation".to_string(),
                            reference: format!("incident:{}", incident.id),
                            data: serde_json::json!({ "matched_rules": matches.len() }),
                            created_at: chrono::Utc::now(),
                        }]);
                    }
                } else {
                    guard.complete_ok(vec![Artifact {
                        name: "routing_result".to_string(),
                        artifact_type: "routing_evaluation".to_string(),
                        reference: format!("incident:{}", incident.id),
                        data: serde_json::json!({ "matched_rules": 0 }),
                        created_at: chrono::Utc::now(),
                    }]);
                }
            } else {
                let matches = routing_evaluator.evaluate_incident(incident);
                if !matches.is_empty() {
                    tracing::info!(
                        incident_id = %incident.id,
                        rule_count = matches.len(),
                        "Routing rules matched"
                    );

                    if let Ok(action_result) =
                        routing_evaluator.apply_actions(incident, &matches).await
                    {
                        if !action_result.suggested_assignees.is_empty() {
                            tracing::info!(
                                incident_id = %incident.id,
                                assignees = ?action_result.suggested_assignees,
                                "Routing suggested assignees"
                            );
                        }
                    }
                }
            }
        }

        // Auto-start escalation
        if let Some(ref escalation_engine) = self.escalation_engine {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("EscalationEngine");
                if let Some(policy) = escalation_engine.find_policy_for_incident(incident) {
                    match escalation_engine.start_escalation(incident, policy.id) {
                        Ok(_) => {
                            tracing::info!(
                                incident_id = %incident.id,
                                policy_id = %policy.id,
                                policy_name = %policy.name,
                                "Started escalation for incident"
                            );
                            guard.complete_ok(vec![Artifact {
                                name: "escalation_started".to_string(),
                                artifact_type: "escalation_policy".to_string(),
                                reference: format!("policy:{}", policy.id),
                                data: serde_json::json!({
                                    "policy_id": policy.id.to_string(),
                                    "policy_name": policy.name,
                                }),
                                created_at: chrono::Utc::now(),
                            }]);
                        }
                        Err(e) => {
                            tracing::error!(
                                incident_id = %incident.id,
                                error = %e,
                                "Failed to start escalation"
                            );
                            guard.complete_err(format!("{}", e));
                        }
                    }
                } else {
                    guard.complete_ok(vec![Artifact {
                        name: "escalation_check".to_string(),
                        artifact_type: "escalation_policy".to_string(),
                        reference: format!("incident:{}", incident.id),
                        data: serde_json::json!({ "policy_matched": false }),
                        created_at: chrono::Utc::now(),
                    }]);
                }
            } else {
                if let Some(policy) = escalation_engine.find_policy_for_incident(incident) {
                    match escalation_engine.start_escalation(incident, policy.id) {
                        Ok(_) => {
                            tracing::info!(
                                incident_id = %incident.id,
                                policy_id = %policy.id,
                                policy_name = %policy.name,
                                "Started escalation for incident"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                incident_id = %incident.id,
                                error = %e,
                                "Failed to start escalation"
                            );
                        }
                    }
                }
            }
        }

        // Analyze for correlations
        if let Some(ref correlation_engine) = self.correlation_engine {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("CorrelationEngine");
                match correlation_engine.analyze_incident(incident).await {
                    Ok(result) => {
                        if result.has_correlations() {
                            tracing::info!(
                                incident_id = %incident.id,
                                correlation_count = result.correlation_count(),
                                groups_affected = result.groups_affected.len(),
                                groups_created = result.groups_created.len(),
                                "Correlations detected for incident"
                            );
                        }
                        guard.complete_ok(vec![Artifact {
                            name: "correlation_analysis".to_string(),
                            artifact_type: "correlation_result".to_string(),
                            reference: format!("incident:{}", incident.id),
                            data: serde_json::json!({
                                "correlation_count": result.correlation_count(),
                                "groups_affected": result.groups_affected.len(),
                                "groups_created": result.groups_created.len(),
                            }),
                            created_at: chrono::Utc::now(),
                        }]);
                    }
                    Err(e) => {
                        tracing::error!(
                            incident_id = %incident.id,
                            error = %e,
                            "Failed to analyze incident correlations"
                        );
                        guard.complete_err(format!("{}", e));
                    }
                }
            } else {
                match correlation_engine.analyze_incident(incident).await {
                    Ok(result) => {
                        if result.has_correlations() {
                            tracing::info!(
                                incident_id = %incident.id,
                                correlation_count = result.correlation_count(),
                                groups_affected = result.groups_affected.len(),
                                groups_created = result.groups_created.len(),
                                "Correlations detected for incident"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            incident_id = %incident.id,
                            error = %e,
                            "Failed to analyze incident correlations"
                        );
                    }
                }
            }
        }

        // Add to ML training set
        if let Some(ref ml_service) = self.ml_service {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("MLService");
                match ml_service.add_training_sample(incident).await {
                    Ok(()) => {
                        guard.complete_ok(vec![Artifact {
                            name: "ml_training_sample".to_string(),
                            artifact_type: "ml_sample".to_string(),
                            reference: format!("incident:{}", incident.id),
                            data: serde_json::json!({ "added": true }),
                            created_at: chrono::Utc::now(),
                        }]);
                    }
                    Err(e) => {
                        tracing::warn!(
                            incident_id = %incident.id,
                            error = %e,
                            "Failed to add incident to ML training set"
                        );
                        guard.complete_err(format!("{}", e));
                    }
                }
            } else {
                if let Err(e) = ml_service.add_training_sample(incident).await {
                    tracing::warn!(
                        incident_id = %incident.id,
                        error = %e,
                        "Failed to add incident to ML training set"
                    );
                }
            }
        }
    }

    /// Get an incident by ID
    pub async fn get_incident(&self, id: &Uuid) -> Result<Incident> {
        self.store
            .get_incident(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Incident {} not found", id)))
    }

    /// Update incident state
    pub async fn update_incident_state(
        &self,
        id: &Uuid,
        new_state: IncidentState,
        actor: String,
    ) -> Result<Incident> {
        let mut incident = self.get_incident(id).await?;

        incident.update_state(new_state, actor);
        self.store.update_incident(&incident).await?;

        tracing::info!(
            incident_id = %id,
            new_state = ?incident.state,
            "Incident state updated"
        );

        Ok(incident)
    }

    /// Resolve an incident
    pub async fn resolve_incident(
        &self,
        id: &Uuid,
        resolved_by: String,
        method: crate::models::ResolutionMethod,
        notes: String,
        root_cause: Option<String>,
        exec_ctx: Option<&ExecutionContext>,
    ) -> Result<Incident> {
        let mut incident = self.get_incident(id).await?;

        incident.resolve(resolved_by, method, notes, root_cause);
        self.store.update_incident(&incident).await?;

        tracing::info!(
            incident_id = %id,
            "Incident resolved"
        );

        // Send resolution notifications
        if let Some(ref notif_service) = self.notification_service {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("NotificationService");
                match notif_service.notify_incident_resolved(&incident).await {
                    Ok(ids) => {
                        guard.complete_ok(vec![Artifact {
                            name: "resolution_notification".to_string(),
                            artifact_type: "notification_ids".to_string(),
                            reference: format!("incident:{}", incident.id),
                            data: serde_json::json!({ "notification_count": ids.len() }),
                            created_at: chrono::Utc::now(),
                        }]);
                    }
                    Err(e) => {
                        tracing::error!(
                            incident_id = %incident.id,
                            error = %e,
                            "Failed to send incident resolution notification"
                        );
                        guard.complete_err(format!("{}", e));
                    }
                }
            } else if let Err(e) = notif_service.notify_incident_resolved(&incident).await {
                tracing::error!(
                    incident_id = %incident.id,
                    error = %e,
                    "Failed to send incident resolution notification"
                );
            }
        }

        // Resolve escalation
        if let Some(ref escalation_engine) = self.escalation_engine {
            if let Some(ctx) = exec_ctx {
                let guard = ctx.start_agent_span("EscalationEngine");
                match escalation_engine.resolve_escalation(id) {
                    Ok(()) => {
                        tracing::info!(incident_id = %id, "Escalation resolved");
                        guard.complete_ok(vec![Artifact {
                            name: "escalation_resolved".to_string(),
                            artifact_type: "escalation_resolution".to_string(),
                            reference: format!("incident:{}", id),
                            data: serde_json::json!({ "resolved": true }),
                            created_at: chrono::Utc::now(),
                        }]);
                    }
                    Err(e) => {
                        tracing::error!(
                            incident_id = %id,
                            error = %e,
                            "Failed to resolve escalation"
                        );
                        guard.complete_err(format!("{}", e));
                    }
                }
            } else {
                if let Err(e) = escalation_engine.resolve_escalation(id) {
                    tracing::error!(
                        incident_id = %id,
                        error = %e,
                        "Failed to resolve escalation"
                    );
                } else {
                    tracing::info!(
                        incident_id = %id,
                        "Escalation resolved"
                    );
                }
            }
        }

        Ok(incident)
    }

    /// Assign incident to users
    pub async fn assign_incident(&self, id: &Uuid, assignees: Vec<String>) -> Result<Incident> {
        let mut incident = self.get_incident(id).await?;

        incident.assignees = assignees.clone();
        incident.add_timeline_event(crate::models::TimelineEvent {
            timestamp: chrono::Utc::now(),
            event_type: crate::models::EventType::AssignmentChanged,
            actor: "system".to_string(),
            description: format!("Assigned to: {}", assignees.join(", ")),
            metadata: std::collections::HashMap::new(),
        });

        self.store.update_incident(&incident).await?;

        tracing::info!(
            incident_id = %id,
            assignees = ?assignees,
            "Incident assigned"
        );

        Ok(incident)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};
    use crate::state::InMemoryStore;

    #[tokio::test]
    async fn test_process_alert_creates_incident() {
        let store = Arc::new(InMemoryStore::new());
        let dedup = Arc::new(DeduplicationEngine::new(store.clone(), 900));
        let processor = IncidentProcessor::new(store.clone(), dedup);

        let alert = Alert::new(
            "ext-123".to_string(),
            "sentinel".to_string(),
            "High CPU".to_string(),
            "CPU > 90%".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let ack = processor.process_alert(alert.clone(), None).await.unwrap();

        assert_eq!(ack.status, crate::models::AckStatus::Accepted);
        assert!(ack.incident_id.is_some());

        // Verify incident was created
        let incident = store.get_incident(&ack.incident_id.unwrap()).await.unwrap();
        assert!(incident.is_some());
    }

    #[tokio::test]
    async fn test_process_duplicate_alert() {
        let store = Arc::new(InMemoryStore::new());
        let dedup = Arc::new(DeduplicationEngine::new(store.clone(), 900));
        let processor = IncidentProcessor::new(store.clone(), dedup);

        // Process first alert
        let alert1 = Alert::new(
            "ext-1".to_string(),
            "sentinel".to_string(),
            "Test Alert".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        let ack1 = processor.process_alert(alert1, None).await.unwrap();

        // Process duplicate alert
        let alert2 = Alert::new(
            "ext-2".to_string(),
            "sentinel".to_string(),
            "Test Alert".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        let ack2 = processor.process_alert(alert2, None).await.unwrap();

        // Second alert should be marked as duplicate
        assert_eq!(ack2.status, crate::models::AckStatus::Duplicate);
        assert_eq!(ack2.incident_id, ack1.incident_id);
    }

    #[tokio::test]
    async fn test_update_incident_state() {
        let store = Arc::new(InMemoryStore::new());
        let dedup = Arc::new(DeduplicationEngine::new(store.clone(), 900));
        let processor = IncidentProcessor::new(store, dedup);

        let incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let id = incident.id;
        processor.create_incident(incident, None).await.unwrap();

        let updated = processor
            .update_incident_state(&id, IncidentState::Investigating, "user@test.com".to_string())
            .await
            .unwrap();

        assert_eq!(updated.state, IncidentState::Investigating);
    }

    #[tokio::test]
    async fn test_process_alert_with_execution_context() {
        let store = Arc::new(InMemoryStore::new());
        let dedup = Arc::new(DeduplicationEngine::new(store.clone(), 900));
        let processor = IncidentProcessor::new(store.clone(), dedup);

        let exec_ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let alert = Alert::new(
            "ext-ctx-1".to_string(),
            "sentinel".to_string(),
            "Context Alert".to_string(),
            "Testing execution context".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        let ack = processor
            .process_alert(alert, Some(&exec_ctx))
            .await
            .unwrap();

        assert_eq!(ack.status, crate::models::AckStatus::Accepted);
        assert!(exec_ctx.has_agent_spans());

        let graph = exec_ctx.finalize(None);
        assert_eq!(graph.repo_span.status, crate::execution::SpanStatus::Ok);
        assert!(!graph.repo_span.children.is_empty());
        // At minimum, DeduplicationEngine should have emitted a span
        assert!(graph
            .repo_span
            .children
            .iter()
            .any(|s| s.name == "DeduplicationEngine"));
    }
}
