use llm_incident_manager::{
    escalation::{
        EscalationEngine, EscalationStatus, RoutingRuleEvaluator, ScheduleResolver,
    },
    models::{
        policy::{
            ConditionOperator, EscalationLevel, EscalationPolicy, EscalationTarget,
            OnCallSchedule, RepeatConfig, RotationStrategy, RoutingAction, RoutingRule,
            RuleCondition, ScheduleLayer,
        },
        Incident, IncidentType, Severity,
    },
    state::InMemoryStore,
};
use std::sync::Arc;
use uuid::Uuid;

fn create_test_incident() -> Incident {
    Incident::new(
        "test".to_string(),
        "Test Incident".to_string(),
        "Test description".to_string(),
        Severity::P1,
        IncidentType::Infrastructure,
    )
}

fn create_test_policy() -> EscalationPolicy {
    EscalationPolicy {
        id: Uuid::new_v4(),
        name: "Standard Escalation".to_string(),
        description: "Standard escalation policy".to_string(),
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

#[tokio::test]
async fn test_escalation_engine_end_to_end() {
    let store = Arc::new(InMemoryStore::new());
    let engine = Arc::new(EscalationEngine::new(None, store.clone()));

    // Register policy
    let policy = create_test_policy();
    let policy_id = policy.id;
    engine.register_policy(policy).unwrap();

    // Create and save incident
    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    // Start escalation
    engine.start_escalation(&incident, policy_id).unwrap();

    // Verify escalation state
    let state = engine.get_escalation_state(&incident.id).unwrap();
    assert_eq!(state.status, EscalationStatus::Active);
    assert_eq!(state.current_level, 0);
    assert_eq!(state.policy_id, policy_id);

    // Get stats
    let stats = engine.get_stats();
    assert_eq!(stats.total_policies, 1);
    assert_eq!(stats.active_escalations, 1);
}

#[tokio::test]
async fn test_escalation_acknowledgment() {
    let store = Arc::new(InMemoryStore::new());
    let engine = Arc::new(EscalationEngine::new(None, store.clone()));

    let policy = create_test_policy();
    let policy_id = policy.id;
    engine.register_policy(policy).unwrap();

    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    engine.start_escalation(&incident, policy_id).unwrap();

    // Acknowledge escalation
    engine
        .acknowledge_escalation(&incident.id, "oncall@example.com".to_string())
        .unwrap();

    // Verify acknowledgment
    let state = engine.get_escalation_state(&incident.id).unwrap();
    assert_eq!(state.status, EscalationStatus::Acknowledged);
    assert!(state.acknowledged);
    assert_eq!(
        state.acknowledged_by,
        Some("oncall@example.com".to_string())
    );

    // Stats should reflect acknowledgment
    let stats = engine.get_stats();
    assert_eq!(stats.active_escalations, 0);
    assert_eq!(stats.acknowledged_escalations, 1);
}

#[tokio::test]
async fn test_escalation_with_schedule() {
    let store = Arc::new(InMemoryStore::new());
    let engine = Arc::new(EscalationEngine::new(None, store.clone()));

    // Create on-call schedule
    let schedule = OnCallSchedule {
        id: Uuid::new_v4(),
        name: "Primary On-Call".to_string(),
        timezone: "UTC".to_string(),
        layers: vec![ScheduleLayer {
            name: "Primary".to_string(),
            users: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
            rotation: RotationStrategy::Daily { handoff_hour: 9 },
            restrictions: None,
        }],
    };

    let schedule_id = schedule.id.to_string();

    // Register schedule with executor
    engine.executor().register_schedule(schedule);

    // Create policy with schedule target
    let mut policy = create_test_policy();
    policy.levels[0].targets = vec![EscalationTarget::Schedule {
        schedule_id: schedule_id.clone(),
    }];

    let policy_id = policy.id;
    engine.register_policy(policy).unwrap();

    // Create incident
    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    // Start escalation
    engine.start_escalation(&incident, policy_id).unwrap();

    // Verify escalation started
    let state = engine.get_escalation_state(&incident.id).unwrap();
    assert_eq!(state.status, EscalationStatus::Active);
}

#[tokio::test]
async fn test_escalation_with_team() {
    let store = Arc::new(InMemoryStore::new());
    let engine = Arc::new(EscalationEngine::new(None, store.clone()));

    // Register team
    engine.executor().register_team(
        "platform".to_string(),
        vec![
            "platform1@example.com".to_string(),
            "platform2@example.com".to_string(),
        ],
    );

    // Create policy with team target
    let mut policy = create_test_policy();
    policy.levels[0].targets = vec![EscalationTarget::Team {
        team_id: "platform".to_string(),
    }];

    let policy_id = policy.id;
    engine.register_policy(policy).unwrap();

    // Create incident
    let incident = create_test_incident();
    store.save_incident(&incident).await.unwrap();

    // Start escalation
    engine.start_escalation(&incident, policy_id).unwrap();

    // Verify escalation started
    let state = engine.get_escalation_state(&incident.id).unwrap();
    assert_eq!(state.status, EscalationStatus::Active);
}

#[tokio::test]
async fn test_routing_rule_evaluation() {
    let evaluator = RoutingRuleEvaluator::new(None);

    // Create routing rule for infrastructure incidents
    let rule = RoutingRule {
        id: Uuid::new_v4(),
        name: "Infrastructure Routing".to_string(),
        priority: 100,
        enabled: true,
        conditions: vec![
            RuleCondition {
                field: "incident_type".to_string(),
                operator: ConditionOperator::Equals,
                value: serde_json::json!("Infrastructure"),
            },
            RuleCondition {
                field: "severity".to_string(),
                operator: ConditionOperator::Equals,
                value: serde_json::json!("P1"),
            },
        ],
        actions: vec![
            RoutingAction::Notify {
                channels: vec!["#infrastructure".to_string()],
            },
            RoutingAction::Assign {
                assignees: vec!["platform-oncall@example.com".to_string()],
            },
        ],
    };

    evaluator.register_rule(rule).unwrap();

    // Evaluate incident
    let incident = create_test_incident();
    let matches = evaluator.evaluate_incident(&incident);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].rule_name, "Infrastructure Routing");
    assert_eq!(matches[0].actions.len(), 2);

    // Apply actions
    let result = evaluator.apply_actions(&incident, &matches).await.unwrap();

    assert_eq!(result.actions_applied, 2);
    assert_eq!(result.notifications.len(), 1);
    assert_eq!(result.suggested_assignees.len(), 1);
    assert_eq!(
        result.suggested_assignees[0],
        "platform-oncall@example.com"
    );
}

#[tokio::test]
async fn test_routing_priority_ordering() {
    let evaluator = RoutingRuleEvaluator::new(None);

    // Create multiple rules with different priorities
    let high_priority_rule = RoutingRule {
        id: Uuid::new_v4(),
        name: "High Priority Rule".to_string(),
        priority: 100,
        enabled: true,
        conditions: vec![RuleCondition {
            field: "severity".to_string(),
            operator: ConditionOperator::Equals,
            value: serde_json::json!("P1"),
        }],
        actions: vec![RoutingAction::Notify {
            channels: vec!["#high-priority".to_string()],
        }],
    };

    let low_priority_rule = RoutingRule {
        id: Uuid::new_v4(),
        name: "Low Priority Rule".to_string(),
        priority: 10,
        enabled: true,
        conditions: vec![RuleCondition {
            field: "severity".to_string(),
            operator: ConditionOperator::Equals,
            value: serde_json::json!("P1"),
        }],
        actions: vec![RoutingAction::Notify {
            channels: vec!["#low-priority".to_string()],
        }],
    };

    evaluator.register_rule(low_priority_rule).unwrap();
    evaluator.register_rule(high_priority_rule).unwrap();

    // Evaluate incident
    let incident = create_test_incident();
    let matches = evaluator.evaluate_incident(&incident);

    // Should match both, but high priority first
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].rule_name, "High Priority Rule");
    assert_eq!(matches[1].rule_name, "Low Priority Rule");
}

#[tokio::test]
async fn test_routing_with_label_condition() {
    let evaluator = RoutingRuleEvaluator::new(None);

    let rule = RoutingRule {
        id: Uuid::new_v4(),
        name: "Team-based Routing".to_string(),
        priority: 50,
        enabled: true,
        conditions: vec![RuleCondition {
            field: "labels.team".to_string(),
            operator: ConditionOperator::Equals,
            value: serde_json::json!("platform"),
        }],
        actions: vec![RoutingAction::Assign {
            assignees: vec!["platform@example.com".to_string()],
        }],
    };

    evaluator.register_rule(rule).unwrap();

    // Create incident with team label
    let mut incident = create_test_incident();
    incident
        .labels
        .insert("team".to_string(), "platform".to_string());

    let matches = evaluator.evaluate_incident(&incident);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].rule_name, "Team-based Routing");
}

#[tokio::test]
async fn test_schedule_resolver_daily_rotation() {
    let resolver = ScheduleResolver::new();

    let schedule = OnCallSchedule {
        id: Uuid::new_v4(),
        name: "Daily Rotation".to_string(),
        timezone: "UTC".to_string(),
        layers: vec![ScheduleLayer {
            name: "Primary".to_string(),
            users: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
                "user3@example.com".to_string(),
            ],
            rotation: RotationStrategy::Daily { handoff_hour: 9 },
            restrictions: None,
        }],
    };

    let oncall_users = resolver.resolve_oncall(&schedule).unwrap();

    assert_eq!(oncall_users.len(), 1);
    assert!(oncall_users[0].email.starts_with("user"));
    assert_eq!(oncall_users[0].layer_name, "Primary");
}

#[tokio::test]
async fn test_schedule_resolver_weekly_rotation() {
    let resolver = ScheduleResolver::new();

    let schedule = OnCallSchedule {
        id: Uuid::new_v4(),
        name: "Weekly Rotation".to_string(),
        timezone: "UTC".to_string(),
        layers: vec![ScheduleLayer {
            name: "Primary".to_string(),
            users: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
            rotation: RotationStrategy::Weekly {
                handoff_day: "Monday".to_string(),
                handoff_hour: 9,
            },
            restrictions: None,
        }],
    };

    let oncall_users = resolver.resolve_oncall(&schedule).unwrap();

    assert_eq!(oncall_users.len(), 1);
}

#[tokio::test]
async fn test_schedule_resolver_multiple_layers() {
    let resolver = ScheduleResolver::new();

    let schedule = OnCallSchedule {
        id: Uuid::new_v4(),
        name: "Multi-layer Schedule".to_string(),
        timezone: "UTC".to_string(),
        layers: vec![
            ScheduleLayer {
                name: "Primary".to_string(),
                users: vec!["primary@example.com".to_string()],
                rotation: RotationStrategy::Daily { handoff_hour: 9 },
                restrictions: None,
            },
            ScheduleLayer {
                name: "Secondary".to_string(),
                users: vec!["secondary@example.com".to_string()],
                rotation: RotationStrategy::Daily { handoff_hour: 9 },
                restrictions: None,
            },
        ],
    };

    let oncall_users = resolver.resolve_oncall(&schedule).unwrap();

    assert_eq!(oncall_users.len(), 2);
    assert_eq!(oncall_users[0].email, "primary@example.com");
    assert_eq!(oncall_users[1].email, "secondary@example.com");
}

#[tokio::test]
async fn test_escalation_policy_matching() {
    let store = Arc::new(InMemoryStore::new());
    let engine = Arc::new(EscalationEngine::new(None, store));

    // Register policies for different severities
    let p0_policy = EscalationPolicy {
        id: Uuid::new_v4(),
        name: "P0 Policy".to_string(),
        description: "For critical incidents".to_string(),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        levels: vec![EscalationLevel {
            level: 0,
            delay_minutes: 0,
            targets: vec![EscalationTarget::User {
                email: "ceo@example.com".to_string(),
            }],
            stop_on_ack: true,
        }],
        repeat: None,
        severity_filter: vec![Severity::P0],
    };

    let p1_policy = EscalationPolicy {
        id: Uuid::new_v4(),
        name: "P1 Policy".to_string(),
        description: "For high severity incidents".to_string(),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        levels: vec![EscalationLevel {
            level: 0,
            delay_minutes: 0,
            targets: vec![EscalationTarget::User {
                email: "oncall@example.com".to_string(),
            }],
            stop_on_ack: true,
        }],
        repeat: None,
        severity_filter: vec![Severity::P1],
    };

    engine.register_policy(p0_policy.clone()).unwrap();
    engine.register_policy(p1_policy.clone()).unwrap();

    // Test P1 incident matching
    let p1_incident = create_test_incident();
    let matched_policy = engine.find_policy_for_incident(&p1_incident);

    assert!(matched_policy.is_some());
    assert_eq!(matched_policy.unwrap().name, "P1 Policy");

    // Test P0 incident matching
    let mut p0_incident = create_test_incident();
    p0_incident.severity = Severity::P0;
    let matched_policy = engine.find_policy_for_incident(&p0_incident);

    assert!(matched_policy.is_some());
    assert_eq!(matched_policy.unwrap().name, "P0 Policy");
}

#[tokio::test]
async fn test_escalation_stats() {
    let store = Arc::new(InMemoryStore::new());
    let engine = Arc::new(EscalationEngine::new(None, store.clone()));

    // Register multiple policies
    let policy1 = create_test_policy();
    let mut policy2 = create_test_policy();
    policy2.id = Uuid::new_v4();
    policy2.name = "Second Policy".to_string();

    engine.register_policy(policy1.clone()).unwrap();
    engine.register_policy(policy2.clone()).unwrap();

    // Create multiple incidents with escalations
    let incident1 = create_test_incident();
    let incident2 = create_test_incident();
    let incident3 = create_test_incident();

    store.save_incident(&incident1).await.unwrap();
    store.save_incident(&incident2).await.unwrap();
    store.save_incident(&incident3).await.unwrap();

    engine.start_escalation(&incident1, policy1.id).unwrap();
    engine.start_escalation(&incident2, policy1.id).unwrap();
    engine.start_escalation(&incident3, policy1.id).unwrap();

    // Acknowledge one
    engine
        .acknowledge_escalation(&incident1.id, "user@example.com".to_string())
        .unwrap();

    // Resolve one
    engine.resolve_escalation(&incident2.id).unwrap();

    // Get stats
    let stats = engine.get_stats();

    assert_eq!(stats.total_policies, 2);
    assert_eq!(stats.total_escalations, 3);
    assert_eq!(stats.active_escalations, 1);
    assert_eq!(stats.acknowledged_escalations, 1);
}

#[tokio::test]
async fn test_routing_stats() {
    let evaluator = RoutingRuleEvaluator::new(None);

    let rule1 = RoutingRule {
        id: Uuid::new_v4(),
        name: "Rule 1".to_string(),
        priority: 100,
        enabled: true,
        conditions: vec![RuleCondition {
            field: "severity".to_string(),
            operator: ConditionOperator::Equals,
            value: serde_json::json!("P0"),
        }],
        actions: vec![],
    };

    let rule2 = RoutingRule {
        id: Uuid::new_v4(),
        name: "Rule 2".to_string(),
        priority: 50,
        enabled: false,
        conditions: vec![RuleCondition {
            field: "severity".to_string(),
            operator: ConditionOperator::Equals,
            value: serde_json::json!("P1"),
        }],
        actions: vec![],
    };

    evaluator.register_rule(rule1).unwrap();
    evaluator.register_rule(rule2).unwrap();

    let stats = evaluator.get_stats();

    assert_eq!(stats.total_rules, 2);
    assert_eq!(stats.enabled_rules, 1);
}
