use crate::error::{AppError, Result};
use crate::models::{Incident, Playbook, PlaybookExecution};
use crate::notifications::NotificationService;
use crate::playbooks::{
    create_default_registry, ExecutionContext, PlaybookExecutor,
};
use crate::state::IncidentStore;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Playbook service manages playbook storage and execution
pub struct PlaybookService {
    /// In-memory playbook storage
    playbooks: Arc<DashMap<Uuid, Playbook>>,

    /// Playbook executor
    executor: Arc<PlaybookExecutor>,

    /// Execution history
    executions: Arc<DashMap<Uuid, PlaybookExecution>>,

    /// Store for incidents
    #[allow(dead_code)]
    store: Arc<dyn IncidentStore>,

    /// Whether automatic execution is enabled
    auto_execute: bool,
}

impl PlaybookService {
    /// Create a new playbook service
    pub fn new(
        store: Arc<dyn IncidentStore>,
        notification_service: Option<Arc<NotificationService>>,
        auto_execute: bool,
    ) -> Self {
        let action_registry = create_default_registry(notification_service, store.clone());
        let executor = Arc::new(PlaybookExecutor::new(
            Arc::new(action_registry),
            store.clone(),
        ));

        Self {
            playbooks: Arc::new(DashMap::new()),
            executor,
            executions: Arc::new(DashMap::new()),
            store,
            auto_execute,
        }
    }

    /// Register a playbook
    pub fn register_playbook(&self, playbook: Playbook) -> Result<()> {
        info!(
            playbook_id = %playbook.id,
            playbook_name = %playbook.name,
            "Registering playbook"
        );

        self.playbooks.insert(playbook.id, playbook);
        Ok(())
    }

    /// Get a playbook by ID
    pub fn get_playbook(&self, id: &Uuid) -> Option<Playbook> {
        self.playbooks.get(id).map(|p| p.clone())
    }

    /// List all playbooks
    pub fn list_playbooks(&self) -> Vec<Playbook> {
        self.playbooks.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Update a playbook
    pub fn update_playbook(&self, playbook: Playbook) -> Result<()> {
        if !self.playbooks.contains_key(&playbook.id) {
            return Err(AppError::NotFound(format!(
                "Playbook {} not found",
                playbook.id
            )));
        }

        self.playbooks.insert(playbook.id, playbook);
        Ok(())
    }

    /// Delete a playbook
    pub fn delete_playbook(&self, id: &Uuid) -> Result<()> {
        self.playbooks
            .remove(id)
            .ok_or_else(|| AppError::NotFound(format!("Playbook {} not found", id)))?;
        Ok(())
    }

    /// Find matching playbooks for an incident
    pub fn find_matching_playbooks(&self, incident: &Incident) -> Vec<Playbook> {
        self.playbooks
            .iter()
            .filter(|entry| {
                let playbook = entry.value();
                playbook.matches_incident(&incident.severity, &incident.incident_type)
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Execute a playbook for an incident
    pub async fn execute_playbook(
        &self,
        playbook_id: Uuid,
        incident: &Incident,
    ) -> Result<PlaybookExecution> {
        let playbook = self
            .get_playbook(&playbook_id)
            .ok_or_else(|| AppError::NotFound(format!("Playbook {} not found", playbook_id)))?;

        if !playbook.enabled {
            return Err(AppError::Validation(format!(
                "Playbook {} is disabled",
                playbook_id
            )));
        }

        info!(
            playbook_id = %playbook_id,
            incident_id = %incident.id,
            "Executing playbook"
        );

        let mut context = ExecutionContext::new(incident.clone());
        let execution = self.executor.execute_playbook(&playbook, &mut context).await?;

        // Store execution result
        self.executions.insert(execution.id, execution.clone());

        info!(
            execution_id = %execution.id,
            playbook_id = %playbook_id,
            incident_id = %incident.id,
            status = ?execution.status,
            "Playbook execution completed"
        );

        Ok(execution)
    }

    /// Auto-execute playbooks for an incident
    pub async fn auto_execute_for_incident(&self, incident: &Incident) -> Vec<PlaybookExecution> {
        if !self.auto_execute {
            return Vec::new();
        }

        let matching_playbooks = self.find_matching_playbooks(incident);

        if matching_playbooks.is_empty() {
            info!(
                incident_id = %incident.id,
                "No matching playbooks found for incident"
            );
            return Vec::new();
        }

        info!(
            incident_id = %incident.id,
            playbook_count = matching_playbooks.len(),
            "Found matching playbooks for auto-execution"
        );

        let mut executions = Vec::new();

        for playbook in matching_playbooks {
            match self.execute_playbook(playbook.id, incident).await {
                Ok(execution) => {
                    executions.push(execution);
                }
                Err(e) => {
                    error!(
                        playbook_id = %playbook.id,
                        incident_id = %incident.id,
                        error = %e,
                        "Failed to auto-execute playbook"
                    );
                }
            }
        }

        executions
    }

    /// Get execution by ID
    pub fn get_execution(&self, id: &Uuid) -> Option<PlaybookExecution> {
        self.executions.get(id).map(|e| e.clone())
    }

    /// List executions for an incident
    pub fn list_executions_for_incident(&self, incident_id: &Uuid) -> Vec<PlaybookExecution> {
        self.executions
            .iter()
            .filter(|entry| entry.value().incident_id == *incident_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List all executions
    pub fn list_executions(&self) -> Vec<PlaybookExecution> {
        self.executions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get service statistics
    pub fn get_stats(&self) -> PlaybookServiceStats {
        let total_playbooks = self.playbooks.len();
        let enabled_playbooks = self
            .playbooks
            .iter()
            .filter(|e| e.value().enabled)
            .count();

        let total_executions = self.executions.len();
        let successful_executions = self
            .executions
            .iter()
            .filter(|e| e.value().status == crate::models::ExecutionStatus::Completed)
            .count();
        let failed_executions = self
            .executions
            .iter()
            .filter(|e| e.value().status == crate::models::ExecutionStatus::Failed)
            .count();

        PlaybookServiceStats {
            total_playbooks,
            enabled_playbooks,
            total_executions,
            successful_executions,
            failed_executions,
            auto_execute_enabled: self.auto_execute,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybookServiceStats {
    pub total_playbooks: usize,
    pub enabled_playbooks: usize,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub auto_execute_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Action, ActionType, BackoffStrategy, IncidentType, PlaybookStep, PlaybookTriggers,
        Severity, StepType,
    };
    use crate::state::InMemoryStore;
    use std::collections::HashMap;

    fn create_test_playbook() -> Playbook {
        Playbook {
            id: Uuid::new_v4(),
            name: "Test Playbook".to_string(),
            version: "1.0".to_string(),
            description: "Test playbook".to_string(),
            owner: "test".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            triggers: PlaybookTriggers {
                severity_trigger: vec![Severity::P0, Severity::P1],
                type_trigger: vec![IncidentType::Infrastructure],
                source_trigger: vec![],
            },
            variables: HashMap::new(),
            steps: vec![PlaybookStep {
                id: "step1".to_string(),
                step_type: StepType::Custom,
                description: Some("Wait".to_string()),
                actions: vec![Action {
                    action_type: ActionType::Wait,
                    parameters: {
                        let mut p = HashMap::new();
                        p.insert("duration".to_string(), serde_json::Value::Number(1.into()));
                        p
                    },
                    on_success: None,
                    on_failure: None,
                }],
                parallel: false,
                timeout: Some("30s".to_string()),
                retry: 0,
                backoff: BackoffStrategy::Fixed,
                condition: None,
            }],
            enabled: true,
            tags: vec![],
        }
    }

    fn create_test_incident() -> Incident {
        Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[tokio::test]
    async fn test_register_and_get_playbook() {
        let store = Arc::new(InMemoryStore::new());
        let service = PlaybookService::new(store, None, false);

        let playbook = create_test_playbook();
        let playbook_id = playbook.id;

        service.register_playbook(playbook).unwrap();

        let retrieved = service.get_playbook(&playbook_id).unwrap();
        assert_eq!(retrieved.id, playbook_id);
    }

    #[tokio::test]
    async fn test_find_matching_playbooks() {
        let store = Arc::new(InMemoryStore::new());
        let service = PlaybookService::new(store, None, false);

        let playbook = create_test_playbook();
        service.register_playbook(playbook).unwrap();

        let incident = create_test_incident();
        let matches = service.find_matching_playbooks(&incident);

        assert_eq!(matches.len(), 1);
    }

    #[tokio::test]
    async fn test_execute_playbook() {
        let store = Arc::new(InMemoryStore::new());
        let service = PlaybookService::new(store.clone(), None, false);

        let playbook = create_test_playbook();
        let playbook_id = playbook.id;
        service.register_playbook(playbook).unwrap();

        let incident = create_test_incident();
        store.save_incident(&incident).await.unwrap();

        let execution = service.execute_playbook(playbook_id, &incident).await.unwrap();

        assert_eq!(execution.status, crate::models::ExecutionStatus::Completed);
        assert_eq!(execution.playbook_id, playbook_id);
        assert_eq!(execution.incident_id, incident.id);
    }

    #[tokio::test]
    async fn test_auto_execute_disabled() {
        let store = Arc::new(InMemoryStore::new());
        let service = PlaybookService::new(store, None, false); // auto_execute = false

        let playbook = create_test_playbook();
        service.register_playbook(playbook).unwrap();

        let incident = create_test_incident();
        let executions = service.auto_execute_for_incident(&incident).await;

        assert_eq!(executions.len(), 0);
    }

    #[tokio::test]
    async fn test_auto_execute_enabled() {
        let store = Arc::new(InMemoryStore::new());
        let service = PlaybookService::new(store.clone(), None, true); // auto_execute = true

        let playbook = create_test_playbook();
        service.register_playbook(playbook).unwrap();

        let incident = create_test_incident();
        store.save_incident(&incident).await.unwrap();

        let executions = service.auto_execute_for_incident(&incident).await;

        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].status, crate::models::ExecutionStatus::Completed);
    }

    #[tokio::test]
    async fn test_service_stats() {
        let store = Arc::new(InMemoryStore::new());
        let service = PlaybookService::new(store.clone(), None, true);

        let playbook = create_test_playbook();
        service.register_playbook(playbook.clone()).unwrap();

        let incident = create_test_incident();
        store.save_incident(&incident).await.unwrap();

        service.execute_playbook(playbook.id, &incident).await.unwrap();

        let stats = service.get_stats();
        assert_eq!(stats.total_playbooks, 1);
        assert_eq!(stats.enabled_playbooks, 1);
        assert_eq!(stats.total_executions, 1);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.failed_executions, 0);
    }
}
