use crate::error::{AppError, Result};
use crate::models::{
    BackoffStrategy, ExecutionStatus, Playbook, PlaybookExecution, PlaybookStep, StepResult,
};
use crate::playbooks::{ActionExecutorRegistry, ExecutionContext};
use crate::state::IncidentStore;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Playbook executor - executes playbooks for incidents
pub struct PlaybookExecutor {
    action_registry: Arc<ActionExecutorRegistry>,
    store: Arc<dyn IncidentStore>,
}

impl PlaybookExecutor {
    /// Create a new playbook executor
    pub fn new(action_registry: Arc<ActionExecutorRegistry>, store: Arc<dyn IncidentStore>) -> Self {
        Self {
            action_registry,
            store,
        }
    }

    /// Execute a playbook for an incident
    pub async fn execute_playbook(
        &self,
        playbook: &Playbook,
        context: &mut ExecutionContext,
    ) -> Result<PlaybookExecution> {
        let execution_id = Uuid::new_v4();
        let incident_id = context.incident().id;

        info!(
            execution_id = %execution_id,
            playbook_id = %playbook.id,
            incident_id = %incident_id,
            "Starting playbook execution"
        );

        let mut execution = PlaybookExecution {
            id: execution_id,
            playbook_id: playbook.id,
            incident_id,
            started_at: Utc::now(),
            completed_at: None,
            status: ExecutionStatus::Running,
            current_step: None,
            step_results: std::collections::HashMap::new(),
            error: None,
        };

        // Apply playbook variables to context
        for (key, value) in &playbook.variables {
            context.set_variable(key.clone(), serde_json::Value::String(value.clone()));
        }

        // Execute steps in sequence
        for step in &playbook.steps {
            execution.current_step = Some(step.id.clone());

            info!(
                execution_id = %execution_id,
                step_id = %step.id,
                step_type = ?step.step_type,
                "Executing step"
            );

            // Evaluate condition if present
            if let Some(ref condition) = step.condition {
                match context.evaluate_condition(condition) {
                    Ok(should_execute) => {
                        if !should_execute {
                            info!(
                                execution_id = %execution_id,
                                step_id = %step.id,
                                condition = %condition,
                                "Skipping step due to condition"
                            );

                            // Add skipped step result
                            let step_result = StepResult {
                                step_id: step.id.clone(),
                                started_at: Utc::now(),
                                completed_at: Some(Utc::now()),
                                status: ExecutionStatus::Completed,
                                output: std::collections::HashMap::new(),
                                error: Some("Skipped due to condition".to_string()),
                            };
                            execution.step_results.insert(step.id.clone(), step_result);
                            continue;
                        }
                    }
                    Err(e) => {
                        error!(
                            execution_id = %execution_id,
                            step_id = %step.id,
                            error = %e,
                            "Failed to evaluate step condition"
                        );
                        execution.status = ExecutionStatus::Failed;
                        execution.error = Some(format!("Condition evaluation failed: {}", e));
                        execution.completed_at = Some(Utc::now());
                        return Ok(execution);
                    }
                }
            }

            // Execute the step
            match self.execute_step(step, context).await {
                Ok(step_result) => {
                    let step_success = step_result.status == ExecutionStatus::Completed
                        && step_result.error.is_none();

                    // Store step output in context
                    context.set_step_output(step.id.clone(), step_result.output.clone());
                    execution.step_results.insert(step.id.clone(), step_result);

                    if !step_success {
                        warn!(
                            execution_id = %execution_id,
                            step_id = %step.id,
                            "Step failed"
                        );
                        execution.status = ExecutionStatus::Failed;
                        execution.completed_at = Some(Utc::now());
                        return Ok(execution);
                    }
                }
                Err(e) => {
                    error!(
                        execution_id = %execution_id,
                        step_id = %step.id,
                        error = %e,
                        "Step execution error"
                    );

                    let step_result = StepResult {
                        step_id: step.id.clone(),
                        started_at: Utc::now(),
                        completed_at: Some(Utc::now()),
                        status: ExecutionStatus::Failed,
                        output: std::collections::HashMap::new(),
                        error: Some(e.to_string()),
                    };
                    execution.step_results.insert(step.id.clone(), step_result);
                    execution.status = ExecutionStatus::Failed;
                    execution.error = Some(format!("Step {} failed: {}", step.id, e));
                    execution.completed_at = Some(Utc::now());
                    return Ok(execution);
                }
            }
        }

        // All steps completed successfully
        execution.status = ExecutionStatus::Completed;
        execution.completed_at = Some(Utc::now());
        execution.current_step = None;

        info!(
            execution_id = %execution_id,
            playbook_id = %playbook.id,
            duration_secs = (execution.completed_at.unwrap() - execution.started_at).num_seconds(),
            "Playbook execution completed successfully"
        );

        Ok(execution)
    }

    /// Execute a single step with retry logic
    async fn execute_step(
        &self,
        step: &PlaybookStep,
        context: &mut ExecutionContext,
    ) -> Result<StepResult> {
        let mut step_result = StepResult {
            step_id: step.id.clone(),
            started_at: Utc::now(),
            completed_at: None,
            status: ExecutionStatus::Running,
            output: std::collections::HashMap::new(),
            error: None,
        };

        // Parse timeout if provided
        let timeout = step
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t).ok())
            .unwrap_or(Duration::from_secs(300)); // Default 5 minutes

        // Execute actions (parallel or sequential)
        let mut attempt = 0;
        let max_attempts = step.retry + 1; // Initial attempt + retries

        loop {
            attempt += 1;

            info!(
                step_id = %step.id,
                attempt = attempt,
                max_attempts = max_attempts,
                "Executing step attempt"
            );

            // Execute actions
            let actions_result = if step.parallel {
                self.execute_actions_parallel(step, context, timeout).await
            } else {
                self.execute_actions_sequential(step, context, timeout).await
            };

            match actions_result {
                Ok(output) => {
                    // Success
                    step_result.output = output;
                    step_result.status = ExecutionStatus::Completed;
                    step_result.completed_at = Some(Utc::now());
                    return Ok(step_result);
                }
                Err(e) => {
                    warn!(
                        step_id = %step.id,
                        attempt = attempt,
                        error = %e,
                        "Step attempt failed"
                    );

                    if attempt >= max_attempts {
                        // No more retries
                        step_result.status = ExecutionStatus::Failed;
                        step_result.error = Some(format!("Failed after {} attempts: {}", attempt, e));
                        step_result.completed_at = Some(Utc::now());
                        return Ok(step_result);
                    }

                    // Calculate backoff and retry
                    let backoff_duration = calculate_backoff(&step.backoff, attempt - 1);
                    info!(
                        step_id = %step.id,
                        backoff_secs = backoff_duration.as_secs(),
                        "Waiting before retry"
                    );
                    sleep(backoff_duration).await;
                }
            }
        }
    }

    /// Execute actions sequentially
    async fn execute_actions_sequential(
        &self,
        step: &PlaybookStep,
        context: &mut ExecutionContext,
        _timeout: Duration,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        let mut combined_output = std::collections::HashMap::new();

        for (idx, action) in step.actions.iter().enumerate() {
            info!(
                step_id = %step.id,
                action_index = idx,
                action_type = ?action.action_type,
                "Executing action"
            );

            let result = self.action_registry.execute(action, context).await?;

            if !result.success {
                return Err(AppError::Internal(
                    result.error.unwrap_or_else(|| "Action failed".to_string()),
                ));
            }

            // Merge output
            for (key, value) in result.output {
                combined_output.insert(format!("action_{}.{}", idx, key), value);
            }
        }

        Ok(combined_output)
    }

    /// Execute actions in parallel
    async fn execute_actions_parallel(
        &self,
        step: &PlaybookStep,
        context: &mut ExecutionContext,
        _timeout: Duration,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        let mut tasks = Vec::new();

        for (idx, action) in step.actions.iter().enumerate() {
            let registry = self.action_registry.clone();
            let action = action.clone();
            let mut ctx = context.clone();

            let task = tokio::spawn(async move {
                let result = registry.execute(&action, &mut ctx).await;
                (idx, result)
            });

            tasks.push(task);
        }

        // Wait for all tasks
        let mut combined_output = std::collections::HashMap::new();
        let mut errors = Vec::new();

        for task in tasks {
            match task.await {
                Ok((idx, Ok(result))) => {
                    if !result.success {
                        errors.push(result.error.unwrap_or_else(|| "Action failed".to_string()));
                    } else {
                        for (key, value) in result.output {
                            combined_output.insert(format!("action_{}.{}", idx, key), value);
                        }
                    }
                }
                Ok((idx, Err(e))) => {
                    errors.push(format!("Action {}: {}", idx, e));
                }
                Err(e) => {
                    errors.push(format!("Task execution error: {}", e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(AppError::Internal(format!(
                "Parallel action execution failed: {}",
                errors.join(", ")
            )));
        }

        Ok(combined_output)
    }
}

/// Parse duration string (e.g., "5s", "10m", "1h")
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(AppError::Validation("Empty duration string".to_string()));
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .map_err(|_| AppError::Validation(format!("Invalid duration number: {}", num_str)))?;

    match unit {
        "s" => Ok(Duration::from_secs(num)),
        "m" => Ok(Duration::from_secs(num * 60)),
        "h" => Ok(Duration::from_secs(num * 3600)),
        _ => Err(AppError::Validation(format!("Invalid duration unit: {}", unit))),
    }
}

/// Calculate backoff duration based on strategy
fn calculate_backoff(strategy: &BackoffStrategy, attempt: u32) -> Duration {
    match strategy {
        BackoffStrategy::Fixed => Duration::from_secs(5),
        BackoffStrategy::Linear => Duration::from_secs((attempt as u64 + 1) * 5),
        BackoffStrategy::Exponential => {
            let secs = 2_u64.pow(attempt).min(300); // Cap at 5 minutes
            Duration::from_secs(secs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Action, ActionType, Incident, IncidentType, Severity};
    use crate::playbooks::create_default_registry;
    use crate::state::InMemoryStore;
    use std::collections::HashMap;

    fn create_test_incident() -> Incident {
        Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("10m").unwrap(), Duration::from_secs(600));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        assert!(parse_duration("invalid").is_err());
    }

    #[test]
    fn test_calculate_backoff() {
        assert_eq!(calculate_backoff(&BackoffStrategy::Fixed, 0), Duration::from_secs(5));
        assert_eq!(calculate_backoff(&BackoffStrategy::Fixed, 5), Duration::from_secs(5));

        assert_eq!(calculate_backoff(&BackoffStrategy::Linear, 0), Duration::from_secs(5));
        assert_eq!(calculate_backoff(&BackoffStrategy::Linear, 1), Duration::from_secs(10));
        assert_eq!(calculate_backoff(&BackoffStrategy::Linear, 2), Duration::from_secs(15));

        assert_eq!(calculate_backoff(&BackoffStrategy::Exponential, 0), Duration::from_secs(1));
        assert_eq!(calculate_backoff(&BackoffStrategy::Exponential, 1), Duration::from_secs(2));
        assert_eq!(calculate_backoff(&BackoffStrategy::Exponential, 2), Duration::from_secs(4));
    }

    #[tokio::test]
    async fn test_simple_playbook_execution() {
        let incident = create_test_incident();
        let store = Arc::new(InMemoryStore::new());
        store.save_incident(&incident).await.unwrap();

        let registry = create_default_registry(None, store.clone());
        let executor = PlaybookExecutor::new(Arc::new(registry), store);

        let playbook = Playbook {
            id: Uuid::new_v4(),
            name: "Test Playbook".to_string(),
            version: "1.0".to_string(),
            description: "Test".to_string(),
            owner: "test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            triggers: crate::models::PlaybookTriggers {
                severity_trigger: vec![],
                type_trigger: vec![],
                source_trigger: vec![],
            },
            variables: HashMap::new(),
            steps: vec![PlaybookStep {
                id: "step1".to_string(),
                step_type: crate::models::StepType::Custom,
                description: Some("Wait step".to_string()),
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
        };

        let mut context = ExecutionContext::new(incident);
        let result = executor.execute_playbook(&playbook, &mut context).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Completed);
        assert!(result.completed_at.is_some());
        assert_eq!(result.step_results.len(), 1);
    }
}
