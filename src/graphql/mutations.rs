//! GraphQL mutation resolvers
//!
//! All write operations for the incident management system

use async_graphql::*;
use uuid::Uuid;

use crate::models;
use super::context::GraphQLContext;
use super::types::*;

/// Root mutation object
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Submit an alert for processing
    async fn submit_alert(
        &self,
        ctx: &Context<'_>,
        input: SubmitAlertInput,
    ) -> Result<AlertAck> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Create alert from input
        let mut alert = models::Alert::new(
            input.external_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            input.source,
            input.title,
            input.description,
            input.severity.into(),
            input.alert_type.into(),
        );

        alert.labels = input.labels;
        alert.affected_services = input.affected_services;
        alert.runbook_url = input.runbook_url;
        alert.annotations = input.annotations;

        // Process the alert
        let ack = gql_ctx
            .processor
            .process_alert(alert)
            .await
            .map_err(|e| Error::new(format!("Failed to process alert: {}", e)))?;

        Ok(AlertAck::from(ack))
    }

    /// Create an incident directly
    async fn create_incident(
        &self,
        ctx: &Context<'_>,
        input: CreateIncidentInput,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Create incident from input
        let mut incident = models::Incident::new(
            input.source,
            input.title,
            input.description,
            input.severity.into(),
            input.incident_type.into(),
        );

        incident.affected_resources = input.affected_resources;
        incident.labels = input.labels;

        // Create the incident
        let created = gql_ctx
            .processor
            .create_incident(incident)
            .await
            .map_err(|e| Error::new(format!("Failed to create incident: {}", e)))?;

        Ok(Incident(created))
    }

    /// Update an incident
    async fn update_incident(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: UpdateIncidentInput,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Get existing incident
        let mut incident = gql_ctx
            .processor
            .get_incident(&id)
            .await
            .map_err(|e| Error::new(format!("Failed to get incident: {}", e)))?;

        // Apply updates
        if let Some(new_state) = input.state {
            let actor = gql_ctx.current_user();
            incident.update_state(new_state.into(), actor);
        }

        if let Some(assignees) = input.assignees {
            incident.assignees = assignees;
            incident.updated_at = chrono::Utc::now();
        }

        if let Some(add_labels) = input.add_labels {
            for (key, value) in add_labels {
                incident.labels.insert(key, value);
            }
            incident.updated_at = chrono::Utc::now();
        }

        if let Some(remove_labels) = input.remove_labels {
            for key in remove_labels {
                incident.labels.remove(&key);
            }
            incident.updated_at = chrono::Utc::now();
        }

        // Save the updated incident
        gql_ctx
            .processor
            .store()
            .update_incident(&incident)
            .await
            .map_err(|e| Error::new(format!("Failed to update incident: {}", e)))?;

        Ok(Incident(incident))
    }

    /// Resolve an incident
    async fn resolve_incident(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: ResolveIncidentInput,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        let resolved_by = gql_ctx.current_user();

        let incident = gql_ctx
            .processor
            .resolve_incident(
                &id,
                resolved_by,
                input.method.into(),
                input.notes,
                input.root_cause,
            )
            .await
            .map_err(|e| Error::new(format!("Failed to resolve incident: {}", e)))?;

        Ok(Incident(incident))
    }

    /// Add a comment to an incident
    async fn add_comment(
        &self,
        ctx: &Context<'_>,
        incident_id: Uuid,
        comment: String,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Get existing incident
        let mut incident = gql_ctx
            .processor
            .get_incident(&incident_id)
            .await
            .map_err(|e| Error::new(format!("Failed to get incident: {}", e)))?;

        // Add comment as timeline event
        let actor = gql_ctx.current_user();
        let event = models::TimelineEvent {
            timestamp: chrono::Utc::now(),
            event_type: models::EventType::CommentAdded,
            actor,
            description: comment,
            metadata: std::collections::HashMap::new(),
        };

        incident.add_timeline_event(event);

        // Save the updated incident
        gql_ctx
            .processor
            .store()
            .update_incident(&incident)
            .await
            .map_err(|e| Error::new(format!("Failed to update incident: {}", e)))?;

        Ok(Incident(incident))
    }

    /// Assign users to an incident
    async fn assign_incident(
        &self,
        ctx: &Context<'_>,
        incident_id: Uuid,
        assignees: Vec<String>,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Get existing incident
        let mut incident = gql_ctx
            .processor
            .get_incident(&incident_id)
            .await
            .map_err(|e| Error::new(format!("Failed to get incident: {}", e)))?;

        // Update assignees
        incident.assignees = assignees.clone();
        incident.updated_at = chrono::Utc::now();

        // Add timeline event
        let actor = gql_ctx.current_user();
        let event = models::TimelineEvent {
            timestamp: chrono::Utc::now(),
            event_type: models::EventType::AssignmentChanged,
            actor,
            description: format!("Assigned to: {}", assignees.join(", ")),
            metadata: std::collections::HashMap::new(),
        };

        incident.add_timeline_event(event);

        // Save the updated incident
        gql_ctx
            .processor
            .store()
            .update_incident(&incident)
            .await
            .map_err(|e| Error::new(format!("Failed to update incident: {}", e)))?;

        Ok(Incident(incident))
    }

    /// Link related incidents
    async fn link_incidents(
        &self,
        ctx: &Context<'_>,
        incident_id: Uuid,
        related_id: Uuid,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Get both incidents
        let mut incident = gql_ctx
            .processor
            .get_incident(&incident_id)
            .await
            .map_err(|e| Error::new(format!("Failed to get incident: {}", e)))?;

        let related_incident = gql_ctx
            .processor
            .get_incident(&related_id)
            .await
            .map_err(|e| Error::new(format!("Failed to get related incident: {}", e)))?;

        // Add to related incidents if not already present
        if !incident.related_incidents.contains(&related_id) {
            incident.related_incidents.push(related_id);
            incident.updated_at = chrono::Utc::now();

            // Add timeline event
            let actor = gql_ctx.current_user();
            let event = models::TimelineEvent {
                timestamp: chrono::Utc::now(),
                event_type: models::EventType::StateChanged,
                actor,
                description: format!("Linked to incident: {}", related_incident.title),
                metadata: std::collections::HashMap::from([
                    ("related_incident_id".to_string(), related_id.to_string()),
                ]),
            };

            incident.add_timeline_event(event);

            // Save the updated incident
            gql_ctx
                .processor
                .store()
                .update_incident(&incident)
                .await
                .map_err(|e| Error::new(format!("Failed to update incident: {}", e)))?;
        }

        Ok(Incident(incident))
    }

    /// Escalate an incident (increase severity)
    async fn escalate_incident(
        &self,
        ctx: &Context<'_>,
        incident_id: Uuid,
        new_severity: Severity,
        reason: String,
    ) -> Result<Incident> {
        let gql_ctx = ctx.data::<GraphQLContext>()?;

        // Get existing incident
        let mut incident = gql_ctx
            .processor
            .get_incident(&incident_id)
            .await
            .map_err(|e| Error::new(format!("Failed to get incident: {}", e)))?;

        let old_severity = incident.severity;
        incident.severity = new_severity.into();
        incident.updated_at = chrono::Utc::now();

        // Add timeline event
        let actor = gql_ctx.current_user();
        let event = models::TimelineEvent {
            timestamp: chrono::Utc::now(),
            event_type: models::EventType::Escalated,
            actor,
            description: format!(
                "Escalated from {:?} to {:?}: {}",
                old_severity, new_severity, reason
            ),
            metadata: std::collections::HashMap::from([
                ("old_severity".to_string(), format!("{:?}", old_severity)),
                ("new_severity".to_string(), format!("{:?}", new_severity)),
                ("reason".to_string(), reason),
            ]),
        };

        incident.add_timeline_event(event);

        // Save the updated incident
        gql_ctx
            .processor
            .store()
            .update_incident(&incident)
            .await
            .map_err(|e| Error::new(format!("Failed to update incident: {}", e)))?;

        Ok(Incident(incident))
    }
}
