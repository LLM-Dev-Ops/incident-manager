use llm_incident_manager::{
    models::{
        Action, ActionType, BackoffStrategy, Incident, IncidentType, Playbook, PlaybookStep,
        PlaybookTriggers, Severity, StepType,
    },
    playbooks::{create_default_registry, ExecutionContext, PlaybookExecutor, PlaybookService},
    state::InMemoryStore,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

fn create_test_incident() -> Incident {
    Incident::new(
        "test-source".to_string(),
        "Test Incident".to_string(),
        "Test description".to_string(),
        Severity::P1,
        IncidentType::Infrastructure,
    )
}

fn create_wait_playbook() -> Playbook {
    Playbook {
        id: Uuid::new_v4(),
        name: "Wait Playbook".to_string(),
        version: "1.0".to_string(),
        description: "Simple wait test".to_string(),
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
            id: "wait_step".to_string(),
            step_type: StepType::Custom,
            description: Some("Wait for 1 second".to_string()),
            actions: vec![Action {
                action_type: ActionType::Wait,
                parameters: {
                    let mut p = HashMap::new();
                    p.insert("duration".to_string(), serde_json::json!(1));
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
        tags: vec!["test".to_string()],
    }
}

#[tokio::test]
async fn test_execution_context_variables() {
    let incident = create_test_incident();
    let mut context = ExecutionContext::new(incident.clone());

    // Test incident variables
    assert_eq!(
        context.get_variable("incident_id").unwrap().as_str().unwrap(),
        incident.id.to_string()
    );

    // Test variable substitution
    context.set_variable("custom".to_string(), serde_json::json!("value"));
    let result = context.substitute_string("Incident {{incident_title}} has {{custom}}");
    assert!(result.contains("Test Incident"));
    assert!(result.contains("value"));
}

#[tokio::test]
async fn test_execution_context_conditions() {
    let incident = create_test_incident();
    let mut context = ExecutionContext::new(incident);

    context.set_variable("count".to_string(), serde_json::json!(5));
    context.set_variable("enabled".to_string(), serde_json::json!(true));

    assert!(context.evaluate_condition("$count == 5").unwrap());
    assert!(context.evaluate_condition("$count > 3").unwrap());
    assert!(context.evaluate_condition("$count < 10").unwrap());
    assert!(context.evaluate_condition("$enabled == true").unwrap());
    assert!(!context.evaluate_condition("$count == 10").unwrap());
}

#[tokio::test]
async fn test_simple_playbook_execution() {
    let incident = create_test_incident();
    let store = Arc::new(InMemoryStore::new());
    store.save_incident(&incident).await.unwrap();

    let registry = create_default_registry(None, store.clone());
    let executor = PlaybookExecutor::new(Arc::new(registry), store);

    let playbook = create_wait_playbook();
    let mut context = ExecutionContext::new(incident);

    let execution = executor.execute_playbook(&playbook, &mut context).await.unwrap();

    assert_eq!(execution.status, llm_incident_manager::models::ExecutionStatus::Completed);
    assert!(execution.completed_at.is_some());
    assert_eq!(execution.step_results.len(), 1);
    assert!(execution.step_results.contains_key("wait_step"));
}

#[tokio::test]
async fn test_playbook_with_variables() {
    let incident = create_test_incident();
    let store = Arc::new(InMemoryStore::new());
    store.save_incident(&incident).await.unwrap();

    let registry = create_default_registry(None, store.clone());
    let executor = PlaybookExecutor::new(Arc::new(registry), store);

    let mut playbook = create_wait_playbook();
    playbook.variables.insert("wait_time".to_string(), "2".to_string());
    playbook.steps[0].actions[0].parameters.insert(
        "duration".to_string(),
        serde_json::json!("{{wait_time}}"),
    );

    let mut context = ExecutionContext::new(incident);
    let execution = executor.execute_playbook(&playbook, &mut context).await.unwrap();

    assert_eq!(execution.status, llm_incident_manager::models::ExecutionStatus::Completed);
}

#[tokio::test]
async fn test_playbook_with_condition() {
    let incident = create_test_incident();
    let store = Arc::new(InMemoryStore::new());
    store.save_incident(&incident).await.unwrap();

    let registry = create_default_registry(None, store.clone());
    let executor = PlaybookExecutor::new(Arc::new(registry), store);

    let mut playbook = create_wait_playbook();

    // Add a condition that should evaluate to false
    playbook.steps[0].condition = Some("$incident_severity == \"P0\"".to_string());

    let mut context = ExecutionContext::new(incident);
    let execution = executor.execute_playbook(&playbook, &mut context).await.unwrap();

    assert_eq!(execution.status, llm_incident_manager::models::ExecutionStatus::Completed);
    // Step should be skipped
    let step_result = execution.step_results.get("wait_step").unwrap();
    assert!(step_result.error.is_some());
    assert!(step_result.error.as_ref().unwrap().contains("Skipped"));
}

#[tokio::test]
async fn test_playbook_service_register_and_list() {
    let store = Arc::new(InMemoryStore::new());
    let service = PlaybookService::new(store, None, false);

    let playbook = create_wait_playbook();
    let playbook_id = playbook.id;

    service.register_playbook(playbook).unwrap();

    let retrieved = service.get_playbook(&playbook_id);
    assert!(retrieved.is_some());

    let all_playbooks = service.list_playbooks();
    assert_eq!(all_playbooks.len(), 1);
}

#[tokio::test]
async fn test_playbook_service_find_matching() {
    let store = Arc::new(InMemoryStore::new());
    let service = PlaybookService::new(store, None, false);

    let playbook = create_wait_playbook();
    service.register_playbook(playbook).unwrap();

    // Matching incident
    let matching_incident = create_test_incident();
    let matches = service.find_matching_playbooks(&matching_incident);
    assert_eq!(matches.len(), 1);

    // Non-matching incident (different severity)
    let mut non_matching = create_test_incident();
    non_matching.severity = Severity::P4;
    let matches = service.find_matching_playbooks(&non_matching);
    assert_eq!(matches.len(), 0);
}

#[tokio::test]
async fn test_playbook_service_execution() {
    let store = Arc::new(InMemoryStore::new());
    let service = PlaybookService::new(store.clone(), None, false);

    let playbook = create_wait_playbook();
    let playbook_id = playbook.id;
    service.register_playbook(playbook).unwrap();

    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    let execution = service.execute_playbook(playbook_id, &incident).await.unwrap();

    assert_eq!(execution.status, llm_incident_manager::models::ExecutionStatus::Completed);
    assert_eq!(execution.playbook_id, playbook_id);
    assert_eq!(execution.incident_id, incident.id);

    // Check execution is stored
    let retrieved = service.get_execution(&execution.id);
    assert!(retrieved.is_some());

    // Check executions for incident
    let incident_executions = service.list_executions_for_incident(&incident.id);
    assert_eq!(incident_executions.len(), 1);
}

#[tokio::test]
async fn test_playbook_service_auto_execution_disabled() {
    let store = Arc::new(InMemoryStore::new());
    let service = PlaybookService::new(store.clone(), None, false); // auto_execute = false

    let playbook = create_wait_playbook();
    service.register_playbook(playbook).unwrap();

    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    let executions = service.auto_execute_for_incident(&incident).await;
    assert_eq!(executions.len(), 0);
}

#[tokio::test]
async fn test_playbook_service_auto_execution_enabled() {
    let store = Arc::new(InMemoryStore::new());
    let service = PlaybookService::new(store.clone(), None, true); // auto_execute = true

    let playbook = create_wait_playbook();
    service.register_playbook(playbook).unwrap();

    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    let executions = service.auto_execute_for_incident(&incident).await;
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].status, llm_incident_manager::models::ExecutionStatus::Completed);
}

#[tokio::test]
async fn test_playbook_service_stats() {
    let store = Arc::new(InMemoryStore::new());
    let service = PlaybookService::new(store.clone(), None, true);

    let playbook1 = create_wait_playbook();
    service.register_playbook(playbook1.clone()).unwrap();

    let mut playbook2 = create_wait_playbook();
    playbook2.id = Uuid::new_v4();
    playbook2.enabled = false;
    service.register_playbook(playbook2).unwrap();

    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    service.execute_playbook(playbook1.id, &incident).await.unwrap();

    let stats = service.get_stats();
    assert_eq!(stats.total_playbooks, 2);
    assert_eq!(stats.enabled_playbooks, 1);
    assert_eq!(stats.total_executions, 1);
    assert_eq!(stats.successful_executions, 1);
    assert_eq!(stats.failed_executions, 0);
    assert!(stats.auto_execute_enabled);
}

#[tokio::test]
async fn test_multi_step_playbook() {
    let incident = create_test_incident();
    let store = Arc::new(InMemoryStore::new());
    store.save_incident(&incident).await.unwrap();

    let registry = create_default_registry(None, store.clone());
    let executor = PlaybookExecutor::new(Arc::new(registry), store);

    let playbook = Playbook {
        id: Uuid::new_v4(),
        name: "Multi-Step Playbook".to_string(),
        version: "1.0".to_string(),
        description: "Multiple steps".to_string(),
        owner: "test".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        triggers: PlaybookTriggers {
            severity_trigger: vec![],
            type_trigger: vec![],
            source_trigger: vec![],
        },
        variables: HashMap::new(),
        steps: vec![
            PlaybookStep {
                id: "step1".to_string(),
                step_type: StepType::Custom,
                description: Some("First wait".to_string()),
                actions: vec![Action {
                    action_type: ActionType::Wait,
                    parameters: {
                        let mut p = HashMap::new();
                        p.insert("duration".to_string(), serde_json::json!(1));
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
            },
            PlaybookStep {
                id: "step2".to_string(),
                step_type: StepType::Custom,
                description: Some("Second wait".to_string()),
                actions: vec![Action {
                    action_type: ActionType::Wait,
                    parameters: {
                        let mut p = HashMap::new();
                        p.insert("duration".to_string(), serde_json::json!(1));
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
            },
        ],
        enabled: true,
        tags: vec![],
    };

    let mut context = ExecutionContext::new(incident);
    let execution = executor.execute_playbook(&playbook, &mut context).await.unwrap();

    assert_eq!(execution.status, llm_incident_manager::models::ExecutionStatus::Completed);
    assert_eq!(execution.step_results.len(), 2);
    assert!(execution.step_results.contains_key("step1"));
    assert!(execution.step_results.contains_key("step2"));
}
