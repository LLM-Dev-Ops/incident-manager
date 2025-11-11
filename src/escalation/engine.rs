use crate::error::{AppError, Result};
use crate::escalation::executor::{EscalationLevelExecutor, EscalationLevelResult};
use crate::escalation::state::{EscalationState, EscalationStatus};
use crate::models::policy::EscalationPolicy;
use crate::models::Incident;
use crate::notifications::NotificationService;
use crate::state::IncidentStore;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Main escalation engine that monitors and executes escalations
pub struct EscalationEngine {
    /// Active escalation states indexed by incident ID
    escalations: Arc<DashMap<Uuid, EscalationState>>,

    /// Escalation policies indexed by policy ID
    policies: Arc<DashMap<Uuid, EscalationPolicy>>,

    /// Executor for escalation levels
    executor: Arc<EscalationLevelExecutor>,

    /// Incident store
    store: Arc<dyn IncidentStore>,

    /// Check interval for escalation monitoring
    check_interval_secs: u64,
}

impl EscalationEngine {
    pub fn new(
        notification_service: Option<Arc<NotificationService>>,
        store: Arc<dyn IncidentStore>,
    ) -> Self {
        Self {
            escalations: Arc::new(DashMap::new()),
            policies: Arc::new(DashMap::new()),
            executor: Arc::new(EscalationLevelExecutor::new(notification_service)),
            store,
            check_interval_secs: 30, // Check every 30 seconds by default
        }
    }

    /// Set the check interval for escalation monitoring
    pub fn with_check_interval(mut self, interval_secs: u64) -> Self {
        self.check_interval_secs = interval_secs;
        self
    }

    /// Get reference to the executor (for registering schedules/teams)
    pub fn executor(&self) -> &EscalationLevelExecutor {
        &self.executor
    }

    /// Register an escalation policy
    pub fn register_policy(&self, policy: EscalationPolicy) -> Result<()> {
        if policy.levels.is_empty() {
            return Err(AppError::Validation(
                "Escalation policy must have at least one level".to_string(),
            ));
        }

        tracing::info!(
            policy_id = %policy.id,
            policy_name = %policy.name,
            levels = policy.levels.len(),
            "Registered escalation policy"
        );

        self.policies.insert(policy.id, policy);
        Ok(())
    }

    /// Get a policy by ID
    pub fn get_policy(&self, policy_id: &Uuid) -> Option<EscalationPolicy> {
        self.policies.get(policy_id).map(|e| e.value().clone())
    }

    /// List all registered policies
    pub fn list_policies(&self) -> Vec<EscalationPolicy> {
        self.policies
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Find applicable policy for an incident
    pub fn find_policy_for_incident(&self, incident: &Incident) -> Option<EscalationPolicy> {
        self.policies
            .iter()
            .find(|entry| {
                let policy = entry.value();
                policy.enabled
                    && (policy.severity_filter.is_empty()
                        || policy.severity_filter.contains(&incident.severity))
            })
            .map(|entry| entry.value().clone())
    }

    /// Start escalation for an incident
    pub fn start_escalation(&self, incident: &Incident, policy_id: Uuid) -> Result<Uuid> {
        // Check if incident already has active escalation
        if let Some(existing) = self.escalations.get(&incident.id) {
            if existing.status == EscalationStatus::Active {
                return Err(AppError::Validation(format!(
                    "Incident {} already has active escalation",
                    incident.id
                )));
            }
        }

        // Get policy
        let policy = self
            .get_policy(&policy_id)
            .ok_or_else(|| AppError::NotFound(format!("Policy {} not found", policy_id)))?;

        if !policy.enabled {
            return Err(AppError::Validation(format!(
                "Policy {} is not enabled",
                policy_id
            )));
        }

        // Get first level delay
        let first_level = policy
            .levels
            .first()
            .ok_or_else(|| AppError::Validation("Policy has no escalation levels".to_string()))?;

        // Create escalation state
        let state = EscalationState::new(incident.id, policy_id, first_level.delay_minutes);

        tracing::info!(
            incident_id = %incident.id,
            policy_id = %policy_id,
            first_delay = first_level.delay_minutes,
            "Started escalation"
        );

        self.escalations.insert(incident.id, state);

        Ok(incident.id)
    }

    /// Acknowledge an escalation
    pub fn acknowledge_escalation(&self, incident_id: &Uuid, acknowledged_by: String) -> Result<()> {
        let mut state = self
            .escalations
            .get_mut(incident_id)
            .ok_or_else(|| AppError::NotFound(format!("No escalation for incident {}", incident_id)))?;

        if state.status != EscalationStatus::Active {
            return Err(AppError::Validation(format!(
                "Escalation for incident {} is not active",
                incident_id
            )));
        }

        state.acknowledge(acknowledged_by.clone());

        tracing::info!(
            incident_id = %incident_id,
            acknowledged_by = %acknowledged_by,
            "Escalation acknowledged"
        );

        Ok(())
    }

    /// Resolve an escalation (when incident is resolved)
    pub fn resolve_escalation(&self, incident_id: &Uuid) -> Result<()> {
        let mut state = self
            .escalations
            .get_mut(incident_id)
            .ok_or_else(|| AppError::NotFound(format!("No escalation for incident {}", incident_id)))?;

        state.resolve();

        tracing::info!(
            incident_id = %incident_id,
            "Escalation resolved"
        );

        Ok(())
    }

    /// Cancel an escalation
    pub fn cancel_escalation(&self, incident_id: &Uuid) -> Result<()> {
        let mut state = self
            .escalations
            .get_mut(incident_id)
            .ok_or_else(|| AppError::NotFound(format!("No escalation for incident {}", incident_id)))?;

        state.cancel();

        tracing::info!(
            incident_id = %incident_id,
            "Escalation cancelled"
        );

        Ok(())
    }

    /// Get escalation state for an incident
    pub fn get_escalation_state(&self, incident_id: &Uuid) -> Option<EscalationState> {
        self.escalations.get(incident_id).map(|e| e.value().clone())
    }

    /// List all active escalations
    pub fn list_active_escalations(&self) -> Vec<EscalationState> {
        self.escalations
            .iter()
            .filter(|entry| entry.value().status == EscalationStatus::Active)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Run the escalation monitor loop
    pub async fn run_monitor(self: Arc<Self>) {
        tracing::info!(
            check_interval = self.check_interval_secs,
            "Starting escalation monitor"
        );

        loop {
            // Check all active escalations
            let active_escalations = self.list_active_escalations();

            for state in active_escalations {
                if let Err(e) = self.check_and_escalate(&state.incident_id).await {
                    tracing::error!(
                        incident_id = %state.incident_id,
                        error = %e,
                        "Failed to check/escalate incident"
                    );
                }
            }

            // Sleep before next check
            sleep(Duration::from_secs(self.check_interval_secs)).await;
        }
    }

    /// Check if an escalation needs to be executed and execute it if needed
    async fn check_and_escalate(&self, incident_id: &Uuid) -> Result<()> {
        // Get current state
        let should_escalate = self
            .escalations
            .get(incident_id)
            .map(|e| e.value().should_escalate())
            .unwrap_or(false);

        if !should_escalate {
            return Ok(());
        }

        // Get policy and state
        let (policy_id, current_level) = {
            let state = self
                .escalations
                .get(incident_id)
                .ok_or_else(|| AppError::NotFound(format!("No escalation for incident {}", incident_id)))?;
            (state.policy_id, state.current_level)
        };

        let policy = self
            .get_policy(&policy_id)
            .ok_or_else(|| AppError::NotFound(format!("Policy {} not found", policy_id)))?;

        // Get incident
        let incident = self
            .store
            .get_incident(incident_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Incident {} not found", incident_id)))?;

        // Find current level
        let level = policy
            .levels
            .iter()
            .find(|l| l.level == current_level)
            .ok_or_else(|| {
                AppError::Validation(format!("Level {} not found in policy", current_level))
            })?;

        tracing::info!(
            incident_id = %incident_id,
            level = current_level,
            "Escalating incident"
        );

        // Execute level
        let result = {
            let mut state = self
                .escalations
                .get_mut(incident_id)
                .ok_or_else(|| AppError::NotFound(format!("No escalation for incident {}", incident_id)))?;

            self.executor
                .execute_level(&incident, level, &mut state)
                .await?
        };

        // Move to next level or complete
        self.advance_escalation(incident_id, &policy, &result).await?;

        Ok(())
    }

    /// Advance escalation to next level or handle completion
    async fn advance_escalation(
        &self,
        incident_id: &Uuid,
        policy: &EscalationPolicy,
        result: &EscalationLevelResult,
    ) -> Result<()> {
        let mut state = self
            .escalations
            .get_mut(incident_id)
            .ok_or_else(|| AppError::NotFound(format!("No escalation for incident {}", incident_id)))?;

        let current_level = state.current_level;

        // Find next level
        let next_level = policy
            .levels
            .iter()
            .find(|l| l.level == current_level + 1);

        if let Some(next) = next_level {
            // Advance to next level
            state.advance_to_next_level(next.delay_minutes);

            tracing::info!(
                incident_id = %incident_id,
                new_level = next.level,
                delay_minutes = next.delay_minutes,
                "Advanced to next escalation level"
            );
        } else {
            // No more levels, check if should repeat
            if let Some(ref repeat_config) = policy.repeat {
                if state.repeat_count < repeat_config.max_repeats {
                    // Get first level for repeat
                    let first_level = policy.levels.first().unwrap();

                    state.reset_for_repeat(first_level.delay_minutes + repeat_config.interval_minutes);

                    tracing::info!(
                        incident_id = %incident_id,
                        repeat_count = state.repeat_count,
                        max_repeats = repeat_config.max_repeats,
                        "Escalation will repeat"
                    );
                } else {
                    // Mark as completed
                    state.advance_to_next_level(0);

                    tracing::info!(
                        incident_id = %incident_id,
                        "Escalation completed (max repeats reached)"
                    );
                }
            } else {
                // Mark as completed
                state.advance_to_next_level(0);

                tracing::info!(
                    incident_id = %incident_id,
                    "Escalation completed"
                );
            }
        }

        Ok(())
    }

    /// Get escalation statistics
    pub fn get_stats(&self) -> EscalationStats {
        let total = self.escalations.len();
        let active = self
            .escalations
            .iter()
            .filter(|e| e.value().status == EscalationStatus::Active)
            .count();
        let acknowledged = self
            .escalations
            .iter()
            .filter(|e| e.value().status == EscalationStatus::Acknowledged)
            .count();
        let completed = self
            .escalations
            .iter()
            .filter(|e| e.value().status == EscalationStatus::Completed)
            .count();

        EscalationStats {
            total_policies: self.policies.len(),
            total_escalations: total,
            active_escalations: active,
            acknowledged_escalations: acknowledged,
            completed_escalations: completed,
        }
    }
}

/// Escalation engine statistics
#[derive(Debug, Clone)]
pub struct EscalationStats {
    pub total_policies: usize,
    pub total_escalations: usize,
    pub active_escalations: usize,
    pub acknowledged_escalations: usize,
    pub completed_escalations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::policy::{EscalationLevel, EscalationTarget, RepeatConfig};
    use crate::models::{IncidentType, Severity};
    use crate::state::InMemoryStore;

    fn create_test_policy() -> EscalationPolicy {
        EscalationPolicy {
            id: Uuid::new_v4(),
            name: "Test Policy".to_string(),
            description: "Test escalation policy".to_string(),
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            levels: vec![
                EscalationLevel {
                    level: 0,
                    delay_minutes: 0,
                    targets: vec![EscalationTarget::User {
                        email: "oncall@example.com".to_string(),
                    }],
                    stop_on_ack: true,
                },
                EscalationLevel {
                    level: 1,
                    delay_minutes: 5,
                    targets: vec![EscalationTarget::User {
                        email: "manager@example.com".to_string(),
                    }],
                    stop_on_ack: true,
                },
            ],
            repeat: Some(RepeatConfig {
                max_repeats: 2,
                interval_minutes: 10,
            }),
            severity_filter: vec![Severity::P0, Severity::P1],
        }
    }

    fn create_test_incident() -> Incident {
        Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[tokio::test]
    async fn test_register_policy() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store);

        let policy = create_test_policy();
        let policy_id = policy.id;

        engine.register_policy(policy).unwrap();

        let retrieved = engine.get_policy(&policy_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, policy_id);
    }

    #[tokio::test]
    async fn test_start_escalation() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store.clone());

        let policy = create_test_policy();
        let policy_id = policy.id;
        engine.register_policy(policy).unwrap();

        let incident = create_test_incident();
        store.save_incident(&incident).await.unwrap();

        let result = engine.start_escalation(&incident, policy_id);
        assert!(result.is_ok());

        let state = engine.get_escalation_state(&incident.id);
        assert!(state.is_some());
        assert_eq!(state.unwrap().status, EscalationStatus::Active);
    }

    #[tokio::test]
    async fn test_acknowledge_escalation() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store.clone());

        let policy = create_test_policy();
        let policy_id = policy.id;
        engine.register_policy(policy).unwrap();

        let incident = create_test_incident();
        store.save_incident(&incident).await.unwrap();

        engine.start_escalation(&incident, policy_id).unwrap();
        engine
            .acknowledge_escalation(&incident.id, "oncall@example.com".to_string())
            .unwrap();

        let state = engine.get_escalation_state(&incident.id).unwrap();
        assert_eq!(state.status, EscalationStatus::Acknowledged);
        assert!(state.acknowledged);
        assert_eq!(
            state.acknowledged_by,
            Some("oncall@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_resolve_escalation() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store.clone());

        let policy = create_test_policy();
        let policy_id = policy.id;
        engine.register_policy(policy).unwrap();

        let incident = create_test_incident();
        store.save_incident(&incident).await.unwrap();

        engine.start_escalation(&incident, policy_id).unwrap();
        engine.resolve_escalation(&incident.id).unwrap();

        let state = engine.get_escalation_state(&incident.id).unwrap();
        assert_eq!(state.status, EscalationStatus::Resolved);
    }

    #[tokio::test]
    async fn test_list_active_escalations() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store.clone());

        let policy = create_test_policy();
        let policy_id = policy.id;
        engine.register_policy(policy).unwrap();

        let incident1 = create_test_incident();
        let incident2 = create_test_incident();

        store.save_incident(&incident1).await.unwrap();
        store.save_incident(&incident2).await.unwrap();

        engine.start_escalation(&incident1, policy_id).unwrap();
        engine.start_escalation(&incident2, policy_id).unwrap();

        let active = engine.list_active_escalations();
        assert_eq!(active.len(), 2);

        // Acknowledge one
        engine
            .acknowledge_escalation(&incident1.id, "user@example.com".to_string())
            .unwrap();

        let active = engine.list_active_escalations();
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_find_policy_for_incident() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store);

        let policy = create_test_policy();
        engine.register_policy(policy.clone()).unwrap();

        let incident = create_test_incident();
        let found = engine.find_policy_for_incident(&incident);

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, policy.id);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store.clone());

        let policy = create_test_policy();
        let policy_id = policy.id;
        engine.register_policy(policy).unwrap();

        let incident1 = create_test_incident();
        let incident2 = create_test_incident();

        store.save_incident(&incident1).await.unwrap();
        store.save_incident(&incident2).await.unwrap();

        engine.start_escalation(&incident1, policy_id).unwrap();
        engine.start_escalation(&incident2, policy_id).unwrap();
        engine
            .acknowledge_escalation(&incident1.id, "user@example.com".to_string())
            .unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_policies, 1);
        assert_eq!(stats.total_escalations, 2);
        assert_eq!(stats.active_escalations, 1);
        assert_eq!(stats.acknowledged_escalations, 1);
    }

    #[tokio::test]
    async fn test_cannot_start_duplicate_escalation() {
        let store = Arc::new(InMemoryStore::new());
        let engine = EscalationEngine::new(None, store.clone());

        let policy = create_test_policy();
        let policy_id = policy.id;
        engine.register_policy(policy).unwrap();

        let incident = create_test_incident();
        store.save_incident(&incident).await.unwrap();

        engine.start_escalation(&incident, policy_id).unwrap();
        let result = engine.start_escalation(&incident, policy_id);

        assert!(result.is_err());
    }
}
