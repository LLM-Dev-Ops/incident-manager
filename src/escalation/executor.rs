use crate::error::{AppError, Result};
use crate::escalation::schedule::{OnCallUser, ScheduleResolver};
use crate::escalation::state::{EscalationNotification, EscalationState};
use crate::models::policy::{EscalationLevel, EscalationTarget, OnCallSchedule};
use crate::models::Incident;
use crate::notifications::NotificationService;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Executes escalation levels by resolving targets and sending notifications
pub struct EscalationLevelExecutor {
    notification_service: Option<Arc<NotificationService>>,
    schedule_resolver: ScheduleResolver,
    schedules: Arc<DashMap<String, OnCallSchedule>>,
    teams: Arc<DashMap<String, Vec<String>>>,
}

impl EscalationLevelExecutor {
    pub fn new(notification_service: Option<Arc<NotificationService>>) -> Self {
        Self {
            notification_service,
            schedule_resolver: ScheduleResolver::new(),
            schedules: Arc::new(DashMap::new()),
            teams: Arc::new(DashMap::new()),
        }
    }

    /// Register an on-call schedule
    pub fn register_schedule(&self, schedule: OnCallSchedule) {
        self.schedules.insert(schedule.id.to_string(), schedule);
    }

    /// Register a team with its members
    pub fn register_team(&self, team_id: String, members: Vec<String>) {
        self.teams.insert(team_id, members);
    }

    /// Execute an escalation level for an incident
    pub async fn execute_level(
        &self,
        incident: &Incident,
        level: &EscalationLevel,
        state: &mut EscalationState,
    ) -> Result<EscalationLevelResult> {
        tracing::info!(
            incident_id = %incident.id,
            level = level.level,
            "Executing escalation level"
        );

        let mut result = EscalationLevelResult {
            level: level.level,
            notifications_sent: 0,
            notifications_failed: 0,
            targets_resolved: Vec::new(),
            errors: Vec::new(),
        };

        // Resolve all targets to actual recipients
        let recipients = self.resolve_targets(&level.targets, &mut result).await?;

        // Send notifications to each recipient
        for recipient in recipients {
            match self.send_notification(incident, &recipient, level.level).await {
                Ok(()) => {
                    result.notifications_sent += 1;
                    state.add_notification(EscalationNotification {
                        sent_at: Utc::now(),
                        level: level.level,
                        target: recipient.email.clone(),
                        channel: recipient.channel.clone(),
                        success: true,
                        error: None,
                    });

                    tracing::info!(
                        incident_id = %incident.id,
                        target = %recipient.email,
                        channel = %recipient.channel,
                        "Notification sent successfully"
                    );
                }
                Err(e) => {
                    result.notifications_failed += 1;
                    result.errors.push(format!(
                        "Failed to notify {}: {}",
                        recipient.email, e
                    ));

                    state.add_notification(EscalationNotification {
                        sent_at: Utc::now(),
                        level: level.level,
                        target: recipient.email.clone(),
                        channel: recipient.channel.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    });

                    tracing::error!(
                        incident_id = %incident.id,
                        target = %recipient.email,
                        error = %e,
                        "Failed to send notification"
                    );
                }
            }
        }

        tracing::info!(
            incident_id = %incident.id,
            level = level.level,
            sent = result.notifications_sent,
            failed = result.notifications_failed,
            "Escalation level execution completed"
        );

        Ok(result)
    }

    /// Resolve escalation targets to actual recipients
    async fn resolve_targets(
        &self,
        targets: &[EscalationTarget],
        result: &mut EscalationLevelResult,
    ) -> Result<Vec<NotificationRecipient>> {
        let mut recipients = Vec::new();

        for target in targets {
            match target {
                EscalationTarget::User { email } => {
                    recipients.push(NotificationRecipient {
                        email: email.clone(),
                        channel: "email".to_string(),
                        source: "user".to_string(),
                    });
                    result.targets_resolved.push(format!("User: {}", email));
                }
                EscalationTarget::Team { team_id } => {
                    match self.resolve_team(team_id) {
                        Ok(members) => {
                            for member in members {
                                recipients.push(NotificationRecipient {
                                    email: member.clone(),
                                    channel: "email".to_string(),
                                    source: format!("team:{}", team_id),
                                });
                            }
                            result
                                .targets_resolved
                                .push(format!("Team: {} ({} members)", team_id, recipients.len()));
                        }
                        Err(e) => {
                            result
                                .errors
                                .push(format!("Failed to resolve team {}: {}", team_id, e));
                            tracing::warn!(
                                team_id = %team_id,
                                error = %e,
                                "Failed to resolve team"
                            );
                        }
                    }
                }
                EscalationTarget::Schedule { schedule_id } => {
                    match self.resolve_schedule(schedule_id) {
                        Ok(oncall_users) => {
                            for user in &oncall_users {
                                recipients.push(NotificationRecipient {
                                    email: user.email.clone(),
                                    channel: "email".to_string(),
                                    source: format!(
                                        "schedule:{}:{}",
                                        schedule_id, user.layer_name
                                    ),
                                });
                            }
                            result.targets_resolved.push(format!(
                                "Schedule: {} ({} on-call)",
                                schedule_id,
                                oncall_users.len()
                            ));
                        }
                        Err(e) => {
                            result
                                .errors
                                .push(format!("Failed to resolve schedule {}: {}", schedule_id, e));
                            tracing::warn!(
                                schedule_id = %schedule_id,
                                error = %e,
                                "Failed to resolve schedule"
                            );
                        }
                    }
                }
                EscalationTarget::Webhook { url } => {
                    recipients.push(NotificationRecipient {
                        email: url.clone(),
                        channel: "webhook".to_string(),
                        source: "webhook".to_string(),
                    });
                    result.targets_resolved.push(format!("Webhook: {}", url));
                }
            }
        }

        Ok(recipients)
    }

    /// Resolve team members
    fn resolve_team(&self, team_id: &str) -> Result<Vec<String>> {
        self.teams
            .get(team_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AppError::NotFound(format!("Team {} not found", team_id)))
    }

    /// Resolve who is currently on-call for a schedule
    fn resolve_schedule(&self, schedule_id: &str) -> Result<Vec<OnCallUser>> {
        let schedule = self
            .schedules
            .get(schedule_id)
            .ok_or_else(|| AppError::NotFound(format!("Schedule {} not found", schedule_id)))?;

        self.schedule_resolver.resolve_oncall(schedule.value())
    }

    /// Send notification to a recipient
    async fn send_notification(
        &self,
        incident: &Incident,
        recipient: &NotificationRecipient,
        level: u32,
    ) -> Result<()> {
        match recipient.channel.as_str() {
            "email" => self.send_email_notification(incident, recipient, level).await,
            "webhook" => self.send_webhook_notification(incident, recipient, level).await,
            _ => Err(AppError::Validation(format!(
                "Unsupported notification channel: {}",
                recipient.channel
            ))),
        }
    }

    /// Send email notification
    async fn send_email_notification(
        &self,
        incident: &Incident,
        recipient: &NotificationRecipient,
        level: u32,
    ) -> Result<()> {
        if let Some(ref notif_service) = self.notification_service {
            let notification = crate::models::Notification {
                id: uuid::Uuid::new_v4(),
                incident_id: incident.id,
                channel: crate::models::NotificationChannel::Email {
                    to: vec![recipient.email.clone()],
                    subject: format!("Escalation Level {} - {}", level, incident.title),
                    body: self.build_notification_message(incident, level),
                },
                status: crate::models::NotificationStatus::Pending,
                created_at: chrono::Utc::now(),
                sent_at: None,
                retry_count: 0,
                error: None,
            };

            notif_service.queue_notification(notification).await?;
            Ok(())
        } else {
            // Simulate notification if no service is configured
            tracing::warn!(
                "No notification service configured, simulating email to {}",
                recipient.email
            );
            Ok(())
        }
    }

    /// Send webhook notification
    async fn send_webhook_notification(
        &self,
        incident: &Incident,
        recipient: &NotificationRecipient,
        level: u32,
    ) -> Result<()> {
        if let Some(ref notif_service) = self.notification_service {
            let notification = crate::models::Notification {
                id: uuid::Uuid::new_v4(),
                incident_id: incident.id,
                channel: crate::models::NotificationChannel::Webhook {
                    url: recipient.email.clone(), // URL in this case
                    payload: serde_json::json!({
                        "level": level,
                        "title": format!("Escalation Level {}", level),
                        "message": self.build_notification_message(incident, level),
                        "incident_id": incident.id,
                    }),
                },
                status: crate::models::NotificationStatus::Pending,
                created_at: chrono::Utc::now(),
                sent_at: None,
                retry_count: 0,
                error: None,
            };

            notif_service.queue_notification(notification).await?;
            Ok(())
        } else {
            // Simulate notification if no service is configured
            tracing::warn!(
                "No notification service configured, simulating webhook to {}",
                recipient.email
            );
            Ok(())
        }
    }

    /// Build notification message
    fn build_notification_message(&self, incident: &Incident, level: u32) -> String {
        format!(
            "Incident escalated to level {}\n\n\
            Title: {}\n\
            Severity: {:?}\n\
            State: {:?}\n\
            Description: {}\n\n\
            Incident ID: {}",
            level,
            incident.title,
            incident.severity,
            incident.state,
            incident.description,
            incident.id
        )
    }

    /// Get all registered schedules
    pub fn list_schedules(&self) -> Vec<OnCallSchedule> {
        self.schedules
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get all registered teams
    pub fn list_teams(&self) -> Vec<(String, Vec<String>)> {
        self.teams
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

/// Recipient for escalation notification
#[derive(Debug, Clone)]
struct NotificationRecipient {
    email: String,
    channel: String,
    source: String,
}

/// Result of executing an escalation level
#[derive(Debug, Clone)]
pub struct EscalationLevelResult {
    pub level: u32,
    pub notifications_sent: usize,
    pub notifications_failed: usize,
    pub targets_resolved: Vec<String>,
    pub errors: Vec<String>,
}

impl EscalationLevelResult {
    pub fn is_successful(&self) -> bool {
        self.notifications_failed == 0 && !self.targets_resolved.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::policy::{RotationStrategy, ScheduleLayer};
    use crate::models::{IncidentType, Severity};

    fn create_test_incident() -> Incident {
        Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    fn create_test_schedule() -> OnCallSchedule {
        OnCallSchedule {
            id: Uuid::new_v4(),
            name: "Primary Schedule".to_string(),
            timezone: "UTC".to_string(),
            layers: vec![ScheduleLayer {
                name: "Primary".to_string(),
                users: vec!["oncall@example.com".to_string()],
                rotation: RotationStrategy::Daily { handoff_hour: 9 },
                restrictions: None,
            }],
        }
    }

    #[tokio::test]
    async fn test_execute_level_with_user_target() {
        let executor = EscalationLevelExecutor::new(None);
        let incident = create_test_incident();

        let level = EscalationLevel {
            level: 0,
            delay_minutes: 0,
            targets: vec![EscalationTarget::User {
                email: "test@example.com".to_string(),
            }],
            stop_on_ack: true,
        };

        let mut state = EscalationState::new(incident.id, Uuid::new_v4(), 5);

        let result = executor.execute_level(&incident, &level, &mut state).await.unwrap();

        assert_eq!(result.level, 0);
        assert_eq!(result.notifications_sent, 1);
        assert_eq!(result.notifications_failed, 0);
        assert_eq!(result.targets_resolved.len(), 1);
        assert!(result.is_successful());
    }

    #[tokio::test]
    async fn test_execute_level_with_team_target() {
        let executor = EscalationLevelExecutor::new(None);
        executor.register_team(
            "platform".to_string(),
            vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
        );

        let incident = create_test_incident();

        let level = EscalationLevel {
            level: 1,
            delay_minutes: 5,
            targets: vec![EscalationTarget::Team {
                team_id: "platform".to_string(),
            }],
            stop_on_ack: true,
        };

        let mut state = EscalationState::new(incident.id, Uuid::new_v4(), 5);

        let result = executor.execute_level(&incident, &level, &mut state).await.unwrap();

        assert_eq!(result.notifications_sent, 2);
        assert_eq!(result.targets_resolved.len(), 1);
        assert!(result.is_successful());
    }

    #[tokio::test]
    async fn test_execute_level_with_schedule_target() {
        let executor = EscalationLevelExecutor::new(None);
        let schedule = create_test_schedule();
        let schedule_id = schedule.id.to_string();
        executor.register_schedule(schedule);

        let incident = create_test_incident();

        let level = EscalationLevel {
            level: 0,
            delay_minutes: 0,
            targets: vec![EscalationTarget::Schedule {
                schedule_id: schedule_id.clone(),
            }],
            stop_on_ack: true,
        };

        let mut state = EscalationState::new(incident.id, Uuid::new_v4(), 5);

        let result = executor.execute_level(&incident, &level, &mut state).await.unwrap();

        assert_eq!(result.notifications_sent, 1);
        assert_eq!(result.targets_resolved.len(), 1);
        assert!(result.is_successful());
    }

    #[tokio::test]
    async fn test_execute_level_with_webhook_target() {
        let executor = EscalationLevelExecutor::new(None);
        let incident = create_test_incident();

        let level = EscalationLevel {
            level: 0,
            delay_minutes: 0,
            targets: vec![EscalationTarget::Webhook {
                url: "https://example.com/webhook".to_string(),
            }],
            stop_on_ack: true,
        };

        let mut state = EscalationState::new(incident.id, Uuid::new_v4(), 5);

        let result = executor.execute_level(&incident, &level, &mut state).await.unwrap();

        assert_eq!(result.notifications_sent, 1);
        assert_eq!(result.targets_resolved.len(), 1);
        assert!(result.is_successful());
    }

    #[tokio::test]
    async fn test_execute_level_with_unknown_team() {
        let executor = EscalationLevelExecutor::new(None);
        let incident = create_test_incident();

        let level = EscalationLevel {
            level: 0,
            delay_minutes: 0,
            targets: vec![EscalationTarget::Team {
                team_id: "nonexistent".to_string(),
            }],
            stop_on_ack: true,
        };

        let mut state = EscalationState::new(incident.id, Uuid::new_v4(), 5);

        let result = executor.execute_level(&incident, &level, &mut state).await.unwrap();

        assert_eq!(result.notifications_sent, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(!result.is_successful());
    }

    #[tokio::test]
    async fn test_execute_level_with_multiple_targets() {
        let executor = EscalationLevelExecutor::new(None);
        executor.register_team(
            "ops".to_string(),
            vec!["ops1@example.com".to_string(), "ops2@example.com".to_string()],
        );

        let incident = create_test_incident();

        let level = EscalationLevel {
            level: 2,
            delay_minutes: 10,
            targets: vec![
                EscalationTarget::User {
                    email: "manager@example.com".to_string(),
                },
                EscalationTarget::Team {
                    team_id: "ops".to_string(),
                },
                EscalationTarget::Webhook {
                    url: "https://example.com/escalation".to_string(),
                },
            ],
            stop_on_ack: true,
        };

        let mut state = EscalationState::new(incident.id, Uuid::new_v4(), 5);

        let result = executor.execute_level(&incident, &level, &mut state).await.unwrap();

        // Should send to: 1 user + 2 team members + 1 webhook = 4 total
        assert_eq!(result.notifications_sent, 4);
        assert_eq!(result.targets_resolved.len(), 3);
        assert!(result.is_successful());
    }

    #[test]
    fn test_register_and_list_schedules() {
        let executor = EscalationLevelExecutor::new(None);
        let schedule = create_test_schedule();
        let schedule_id = schedule.id;

        executor.register_schedule(schedule);

        let schedules = executor.list_schedules();
        assert_eq!(schedules.len(), 1);
        assert_eq!(schedules[0].id, schedule_id);
    }

    #[test]
    fn test_register_and_list_teams() {
        let executor = EscalationLevelExecutor::new(None);
        executor.register_team(
            "team1".to_string(),
            vec!["user1@example.com".to_string()],
        );
        executor.register_team(
            "team2".to_string(),
            vec!["user2@example.com".to_string()],
        );

        let teams = executor.list_teams();
        assert_eq!(teams.len(), 2);
    }
}
