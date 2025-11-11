# Escalation Engine User Guide

## Overview

The Escalation Engine provides automated incident escalation with support for multi-level escalation policies, on-call schedules, routing rules, and acknowledgment tracking. This guide covers everything you need to know to configure and use the escalation system.

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [Escalation Policies](#escalation-policies)
3. [On-Call Schedules](#on-call-schedules)
4. [Routing Rules](#routing-rules)
5. [Integration](#integration)
6. [API Examples](#api-examples)
7. [Best Practices](#best-practices)

## Core Concepts

### Escalation Flow

```
Incident Created
    ↓
Routing Rules Evaluated
    ↓
Escalation Policy Selected
    ↓
Level 0 (Immediate)
    ↓ (after delay)
Level 1
    ↓ (after delay)
Level 2
    ↓ (if not acknowledged)
Repeat or Complete
```

### Key Components

- **Escalation Engine**: Main orchestrator that monitors and executes escalations
- **Escalation Policy**: Defines levels, delays, and targets for escalation
- **Escalation Level**: A single step in the escalation chain
- **Escalation Target**: Who gets notified (user, team, schedule, webhook)
- **On-Call Schedule**: Defines rotation of on-call engineers
- **Routing Rules**: Conditional logic for incident routing and actions

## Escalation Policies

### Policy Structure

An escalation policy consists of:
- **ID**: Unique identifier
- **Name**: Human-readable name
- **Levels**: Sequential escalation steps
- **Repeat**: Optional repeat configuration
- **Severity Filter**: Which severities this policy applies to

### Creating a Policy

```rust
use llm_incident_manager::{
    escalation::EscalationEngine,
    models::policy::{
        EscalationPolicy, EscalationLevel, EscalationTarget, RepeatConfig
    },
    models::Severity,
};

// Create policy
let policy = EscalationPolicy {
    id: Uuid::new_v4(),
    name: "Standard P1 Escalation".to_string(),
    description: "Standard escalation for P1 incidents".to_string(),
    enabled: true,
    created_at: Utc::now(),
    updated_at: Utc::now(),

    levels: vec![
        // Level 0: Immediate notification to on-call
        EscalationLevel {
            level: 0,
            delay_minutes: 0,  // Immediate
            targets: vec![
                EscalationTarget::Schedule {
                    schedule_id: "primary-oncall".to_string(),
                },
            ],
            stop_on_ack: true,
        },

        // Level 1: After 5 minutes, notify team lead
        EscalationLevel {
            level: 1,
            delay_minutes: 5,
            targets: vec![
                EscalationTarget::User {
                    email: "team-lead@example.com".to_string(),
                },
            ],
            stop_on_ack: true,
        },

        // Level 2: After 15 minutes, notify entire team
        EscalationLevel {
            level: 2,
            delay_minutes: 10,
            targets: vec![
                EscalationTarget::Team {
                    team_id: "platform-team".to_string(),
                },
            ],
            stop_on_ack: true,
        },
    ],

    // Repeat up to 3 times if not acknowledged
    repeat: Some(RepeatConfig {
        max_repeats: 3,
        interval_minutes: 20,
    }),

    // Apply to P0 and P1 incidents
    severity_filter: vec![Severity::P0, Severity::P1],
};

// Register with engine
engine.register_policy(policy)?;
```

### Escalation Targets

Four types of targets are supported:

#### 1. User Target
Direct notification to a specific user by email:
```rust
EscalationTarget::User {
    email: "engineer@example.com".to_string(),
}
```

#### 2. Team Target
Notification to all members of a team:
```rust
EscalationTarget::Team {
    team_id: "platform-team".to_string(),
}
```

First register the team:
```rust
engine.executor().register_team(
    "platform-team".to_string(),
    vec![
        "engineer1@example.com".to_string(),
        "engineer2@example.com".to_string(),
        "engineer3@example.com".to_string(),
    ],
);
```

#### 3. Schedule Target
Notification to whoever is currently on-call:
```rust
EscalationTarget::Schedule {
    schedule_id: "primary-oncall".to_string(),
}
```

#### 4. Webhook Target
HTTP POST to a webhook URL:
```rust
EscalationTarget::Webhook {
    url: "https://example.com/webhook/escalation".to_string(),
}
```

### Repeat Configuration

Policies can repeat if incidents remain unacknowledged:

```rust
repeat: Some(RepeatConfig {
    max_repeats: 3,          // Repeat up to 3 times
    interval_minutes: 20,    // Wait 20 minutes before repeating
})
```

If `repeat` is `None`, the policy runs once and completes.

## On-Call Schedules

### Schedule Structure

```rust
use llm_incident_manager::models::policy::{
    OnCallSchedule, ScheduleLayer, RotationStrategy, TimeRestrictions
};

let schedule = OnCallSchedule {
    id: Uuid::new_v4(),
    name: "Primary On-Call".to_string(),
    timezone: "America/New_York".to_string(),

    layers: vec![
        ScheduleLayer {
            name: "Primary".to_string(),
            users: vec![
                "engineer1@example.com".to_string(),
                "engineer2@example.com".to_string(),
                "engineer3@example.com".to_string(),
            ],
            rotation: RotationStrategy::Weekly {
                handoff_day: "Monday".to_string(),
                handoff_hour: 9,  // 9 AM
            },
            restrictions: Some(TimeRestrictions {
                days_of_week: vec![1, 2, 3, 4, 5],  // Mon-Fri
                start_hour: 9,
                end_hour: 17,  // 9 AM - 5 PM
            }),
        },
        ScheduleLayer {
            name: "After Hours".to_string(),
            users: vec![
                "oncall1@example.com".to_string(),
                "oncall2@example.com".to_string(),
            ],
            rotation: RotationStrategy::Daily {
                handoff_hour: 0,  // Midnight
            },
            restrictions: None,  // Active 24/7
        },
    ],
};

// Register schedule
engine.executor().register_schedule(schedule);
```

### Rotation Strategies

#### Daily Rotation
Engineers rotate every day at a specific hour:
```rust
RotationStrategy::Daily {
    handoff_hour: 9,  // 9 AM handoff
}
```

#### Weekly Rotation
Engineers rotate weekly on a specific day and time:
```rust
RotationStrategy::Weekly {
    handoff_day: "Monday".to_string(),
    handoff_hour: 9,  // Monday at 9 AM
}
```

#### Custom Duration
Engineers rotate after a custom number of hours:
```rust
RotationStrategy::Custom {
    duration_hours: 12,  // 12-hour shifts
}
```

### Time Restrictions

Limit when a layer is active:

```rust
restrictions: Some(TimeRestrictions {
    days_of_week: vec![1, 2, 3, 4, 5],  // 0 = Sunday, 6 = Saturday
    start_hour: 9,    // 9 AM
    end_hour: 17,     // 5 PM
})
```

### Multi-Layer Schedules

Support primary, secondary, and tertiary on-call:

```rust
layers: vec![
    ScheduleLayer {
        name: "Primary".to_string(),
        users: vec!["primary@example.com".to_string()],
        rotation: RotationStrategy::Weekly {
            handoff_day: "Monday".to_string(),
            handoff_hour: 9,
        },
        restrictions: None,
    },
    ScheduleLayer {
        name: "Secondary".to_string(),
        users: vec!["secondary@example.com".to_string()],
        rotation: RotationStrategy::Weekly {
            handoff_day: "Monday".to_string(),
            handoff_hour: 9,
        },
        restrictions: None,
    },
]
```

## Routing Rules

### Rule Structure

Routing rules provide conditional logic to automatically assign, label, and notify based on incident properties:

```rust
use llm_incident_manager::models::policy::{
    RoutingRule, RuleCondition, RoutingAction, ConditionOperator
};

let rule = RoutingRule {
    id: Uuid::new_v4(),
    name: "Route Security Incidents".to_string(),
    priority: 100,  // Higher priority rules evaluated first
    enabled: true,

    conditions: vec![
        RuleCondition {
            field: "incident_type".to_string(),
            operator: ConditionOperator::Equals,
            value: serde_json::json!("Security"),
        },
        RuleCondition {
            field: "severity".to_string(),
            operator: ConditionOperator::In,
            value: serde_json::json!(["P0", "P1"]),
        },
    ],

    actions: vec![
        RoutingAction::Notify {
            channels: vec!["#security-incidents".to_string()],
        },
        RoutingAction::Assign {
            assignees: vec!["security-oncall@example.com".to_string()],
        },
        RoutingAction::AddLabels {
            labels: {
                let mut labels = HashMap::new();
                labels.insert("team".to_string(), "security".to_string());
                labels.insert("automated".to_string(), "true".to_string());
                labels
            },
        },
    ],
};

evaluator.register_rule(rule)?;
```

### Condition Operators

- **Equals**: Exact match
- **NotEquals**: Not equal
- **Contains**: String contains substring
- **NotContains**: String doesn't contain substring
- **GreaterThan**: Numeric comparison
- **LessThan**: Numeric comparison
- **In**: Value in array
- **NotIn**: Value not in array
- **Matches**: Regex pattern match

### Available Fields

- `id`: Incident UUID
- `source`: Incident source system
- `title`: Incident title
- `description`: Incident description
- `severity`: Severity level (P0-P4)
- `state`: Current state
- `incident_type`: Type of incident
- `priority_score`: Numeric priority
- `assignees`: List of assignees
- `labels.<key>`: Custom label value

### Routing Actions

#### Notify
Send notifications to channels:
```rust
RoutingAction::Notify {
    channels: vec!["#ops", "#platform"],
}
```

#### Assign
Auto-assign incidents:
```rust
RoutingAction::Assign {
    assignees: vec!["oncall@example.com"],
}
```

#### Apply Playbook
Auto-execute a playbook:
```rust
RoutingAction::ApplyPlaybook {
    playbook_id: playbook_uuid,
}
```

#### Add Labels
Add custom labels:
```rust
RoutingAction::AddLabels {
    labels: {
        let mut labels = HashMap::new();
        labels.insert("routed".to_string(), "true".to_string());
        labels
    },
}
```

#### Set Severity
Change incident severity:
```rust
RoutingAction::SetSeverity {
    severity: Severity::P0,
}
```

#### Suppress
Suppress notifications temporarily:
```rust
RoutingAction::Suppress {
    duration_minutes: 30,
}
```

## Integration

### Initializing the Engine

```rust
use llm_incident_manager::{
    escalation::EscalationEngine,
    notifications::NotificationService,
    state::InMemoryStore,
};

// Create engine
let store = Arc::new(InMemoryStore::new());
let notification_service = Arc::new(NotificationService::new(config, store.clone())?);

let engine = Arc::new(EscalationEngine::new(
    Some(notification_service),
    store.clone(),
).with_check_interval(30));  // Check every 30 seconds

// Start monitoring
let monitor = engine.clone();
tokio::spawn(async move {
    monitor.run_monitor().await;
});
```

### Starting Escalations

Escalations can start automatically or manually:

#### Automatic Start
Configure IncidentProcessor to auto-start escalations:
```rust
processor.set_escalation_engine(engine.clone());
```

When incidents are created, the engine finds matching policies and starts escalation automatically.

#### Manual Start
```rust
// Find policy for incident
if let Some(policy) = engine.find_policy_for_incident(&incident) {
    engine.start_escalation(&incident, policy.id)?;
}
```

### Acknowledging Escalations

```rust
engine.acknowledge_escalation(
    &incident_id,
    "oncall@example.com".to_string(),
)?;
```

This stops the escalation from progressing to higher levels.

### Resolving Escalations

```rust
engine.resolve_escalation(&incident_id)?;
```

Called automatically when incidents are resolved.

### Canceling Escalations

```rust
engine.cancel_escalation(&incident_id)?;
```

## API Examples

### Complete Workflow Example

```rust
use llm_incident_manager::{
    escalation::{EscalationEngine, RoutingRuleEvaluator},
    models::{Incident, IncidentType, Severity},
    state::InMemoryStore,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize
    let store = Arc::new(InMemoryStore::new());
    let engine = Arc::new(EscalationEngine::new(None, store.clone()));
    let routing = Arc::new(RoutingRuleEvaluator::new(None));

    // Register on-call schedule
    let schedule = OnCallSchedule {
        id: Uuid::new_v4(),
        name: "Primary".to_string(),
        timezone: "UTC".to_string(),
        layers: vec![
            ScheduleLayer {
                name: "Primary".to_string(),
                users: vec!["oncall@example.com".to_string()],
                rotation: RotationStrategy::Daily { handoff_hour: 9 },
                restrictions: None,
            },
        ],
    };
    engine.executor().register_schedule(schedule.clone());

    // Register team
    engine.executor().register_team(
        "platform".to_string(),
        vec!["eng1@example.com".to_string(), "eng2@example.com".to_string()],
    );

    // Create escalation policy
    let policy = EscalationPolicy {
        id: Uuid::new_v4(),
        name: "Standard".to_string(),
        description: "Standard escalation".to_string(),
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        levels: vec![
            EscalationLevel {
                level: 0,
                delay_minutes: 0,
                targets: vec![
                    EscalationTarget::Schedule {
                        schedule_id: schedule.id.to_string(),
                    },
                ],
                stop_on_ack: true,
            },
            EscalationLevel {
                level: 1,
                delay_minutes: 5,
                targets: vec![
                    EscalationTarget::Team {
                        team_id: "platform".to_string(),
                    },
                ],
                stop_on_ack: true,
            },
        ],
        repeat: None,
        severity_filter: vec![Severity::P0, Severity::P1],
    };
    engine.register_policy(policy.clone())?;

    // Create routing rule
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
        ],
        actions: vec![
            RoutingAction::Assign {
                assignees: vec!["platform@example.com".to_string()],
            },
        ],
    };
    routing.register_rule(rule)?;

    // Create incident
    let incident = Incident::new(
        "monitoring".to_string(),
        "High CPU".to_string(),
        "CPU usage at 95%".to_string(),
        Severity::P1,
        IncidentType::Infrastructure,
    );
    store.save_incident(&incident).await?;

    // Apply routing rules
    let matches = routing.evaluate_incident(&incident);
    if !matches.is_empty() {
        routing.apply_actions(&incident, &matches).await?;
    }

    // Start escalation
    engine.start_escalation(&incident, policy.id)?;

    // Check escalation state
    let state = engine.get_escalation_state(&incident.id).unwrap();
    println!("Escalation started at level {}", state.current_level);

    // Later: acknowledge
    engine.acknowledge_escalation(&incident.id, "oncall@example.com".to_string())?;

    Ok(())
}
```

## Best Practices

### 1. Policy Design

- **Start Simple**: Begin with 2-3 levels
- **Reasonable Delays**: 5-15 minutes between levels
- **Clear Ownership**: Each level should have clear responsibilities
- **Test Thoroughly**: Test policies in staging first

### 2. On-Call Schedules

- **Multiple Layers**: Have primary, secondary, and escalation contacts
- **Reasonable Shifts**: 12-24 hour rotations work best
- **Time Restrictions**: Use for business hours vs. after-hours coverage
- **Timezone Awareness**: Always specify correct timezone

### 3. Routing Rules

- **Priority Ordering**: Higher priority (100) before lower priority (10)
- **Specific Conditions**: More specific rules should have higher priority
- **Action Order**: Order actions from most to least important
- **Test Matching**: Verify rules match expected incidents

### 4. Acknowledgments

- **Quick Response**: Acknowledge within minutes to prevent escalation
- **Clear Ownership**: Acknowledgment should mean active investigation
- **Follow Up**: Resolve or update incidents regularly

### 5. Monitoring

- **Track Metrics**: Monitor escalation counts, acknowledgment times
- **Alert on Patterns**: Alert if escalations consistently reach high levels
- **Regular Review**: Review policies quarterly
- **Adjust Timing**: Refine delays based on response patterns

### 6. Testing

```rust
#[tokio::test]
async fn test_escalation_workflow() {
    let engine = setup_test_engine();
    let incident = create_test_incident();

    // Test policy matching
    let policy = engine.find_policy_for_incident(&incident);
    assert!(policy.is_some());

    // Test escalation start
    engine.start_escalation(&incident, policy.unwrap().id).unwrap();

    // Test state
    let state = engine.get_escalation_state(&incident.id).unwrap();
    assert_eq!(state.status, EscalationStatus::Active);

    // Test acknowledgment
    engine.acknowledge_escalation(&incident.id, "test@example.com".to_string()).unwrap();

    let state = engine.get_escalation_state(&incident.id).unwrap();
    assert_eq!(state.status, EscalationStatus::Acknowledged);
}
```

## Troubleshooting

### Escalations Not Starting

1. Check policy is enabled
2. Verify severity filter matches
3. Confirm policy is registered
4. Check engine is integrated with processor

### On-Call Resolution Issues

1. Verify schedule is registered
2. Check timezone configuration
3. Confirm users are in rotation
4. Verify time restrictions

### Routing Rules Not Matching

1. Check rule is enabled
2. Verify condition operators
3. Test field values
4. Check priority ordering

### Notifications Not Sending

1. Verify notification service is configured
2. Check target email/webhook validity
3. Review notification service logs
4. Confirm notification queue is processing

## Performance Considerations

- **Check Interval**: Default 30 seconds, adjust based on requirements
- **Policy Count**: Engine handles hundreds of policies efficiently
- **Concurrent Escalations**: No practical limit on active escalations
- **Schedule Resolution**: O(1) resolution time for on-call lookups
- **Routing Rules**: O(n) evaluation where n = number of rules

## Security Considerations

- **Webhook URLs**: Validate and sanitize webhook URLs
- **Access Control**: Implement authorization for policy management
- **Audit Logging**: Log all escalation state changes
- **Data Privacy**: Be mindful of PII in notifications
- **Rate Limiting**: Implement rate limits on acknowledgments

## Next Steps

- Review [PLAYBOOKS_IMPLEMENTATION.md](./PLAYBOOKS_IMPLEMENTATION.md) for playbook integration
- Review [NOTIFICATIONS_GUIDE.md](./NOTIFICATIONS_GUIDE.md) for notification setup
- Check [API documentation](./API.md) for HTTP endpoints
- See [examples/](./examples/) for more code samples
