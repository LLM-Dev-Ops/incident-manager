use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Tracks the escalation state of an incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationState {
    /// Incident ID
    pub incident_id: Uuid,

    /// Policy being applied
    pub policy_id: Uuid,

    /// Current escalation level
    pub current_level: u32,

    /// When escalation started
    pub started_at: DateTime<Utc>,

    /// When the current level was reached
    pub level_reached_at: DateTime<Utc>,

    /// When to escalate to next level
    pub next_escalation_at: Option<DateTime<Utc>>,

    /// Whether incident was acknowledged
    pub acknowledged: bool,

    /// When incident was acknowledged
    pub acknowledged_at: Option<DateTime<Utc>>,

    /// Who acknowledged
    pub acknowledged_by: Option<String>,

    /// Number of times policy has repeated
    pub repeat_count: u32,

    /// Status of escalation
    pub status: EscalationStatus,

    /// History of notifications sent
    pub notification_history: Vec<EscalationNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EscalationStatus {
    /// Actively escalating
    Active,

    /// Acknowledged and stopped
    Acknowledged,

    /// Completed all levels
    Completed,

    /// Incident resolved
    Resolved,

    /// Escalation cancelled
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationNotification {
    /// When notification was sent
    pub sent_at: DateTime<Utc>,

    /// Escalation level
    pub level: u32,

    /// Target that was notified
    pub target: String,

    /// Notification channel used
    pub channel: String,

    /// Whether notification succeeded
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,
}

impl EscalationState {
    /// Create a new escalation state
    pub fn new(incident_id: Uuid, policy_id: Uuid, first_level_delay_minutes: u32) -> Self {
        let now = Utc::now();
        let next_escalation = if first_level_delay_minutes == 0 {
            Some(now)
        } else {
            Some(now + chrono::Duration::minutes(first_level_delay_minutes as i64))
        };

        Self {
            incident_id,
            policy_id,
            current_level: 0,
            started_at: now,
            level_reached_at: now,
            next_escalation_at: next_escalation,
            acknowledged: false,
            acknowledged_at: None,
            acknowledged_by: None,
            repeat_count: 0,
            status: EscalationStatus::Active,
            notification_history: Vec::new(),
        }
    }

    /// Check if it's time to escalate
    pub fn should_escalate(&self) -> bool {
        if self.status != EscalationStatus::Active {
            return false;
        }

        if self.acknowledged {
            return false;
        }

        if let Some(next_time) = self.next_escalation_at {
            Utc::now() >= next_time
        } else {
            false
        }
    }

    /// Move to next escalation level
    pub fn advance_to_next_level(&mut self, next_level_delay_minutes: u32) {
        self.current_level += 1;
        self.level_reached_at = Utc::now();

        if next_level_delay_minutes == 0 {
            // No more levels
            self.next_escalation_at = None;
            self.status = EscalationStatus::Completed;
        } else {
            self.next_escalation_at = Some(
                Utc::now() + chrono::Duration::minutes(next_level_delay_minutes as i64)
            );
        }
    }

    /// Acknowledge the incident
    pub fn acknowledge(&mut self, acknowledged_by: String) {
        self.acknowledged = true;
        self.acknowledged_at = Some(Utc::now());
        self.acknowledged_by = Some(acknowledged_by);
        self.status = EscalationStatus::Acknowledged;
    }

    /// Mark as resolved
    pub fn resolve(&mut self) {
        self.status = EscalationStatus::Resolved;
        self.next_escalation_at = None;
    }

    /// Cancel escalation
    pub fn cancel(&mut self) {
        self.status = EscalationStatus::Cancelled;
        self.next_escalation_at = None;
    }

    /// Reset for repeat
    pub fn reset_for_repeat(&mut self, first_level_delay_minutes: u32) {
        self.current_level = 0;
        self.level_reached_at = Utc::now();
        self.next_escalation_at = Some(
            Utc::now() + chrono::Duration::minutes(first_level_delay_minutes as i64)
        );
        self.repeat_count += 1;
        self.acknowledged = false;
        self.acknowledged_at = None;
        self.acknowledged_by = None;
        self.status = EscalationStatus::Active;
    }

    /// Add a notification to history
    pub fn add_notification(&mut self, notification: EscalationNotification) {
        self.notification_history.push(notification);
    }

    /// Get time until next escalation
    pub fn time_until_next_escalation(&self) -> Option<chrono::Duration> {
        self.next_escalation_at.map(|next| next - Utc::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escalation_state_creation() {
        let state = EscalationState::new(Uuid::new_v4(), Uuid::new_v4(), 5);

        assert_eq!(state.current_level, 0);
        assert_eq!(state.status, EscalationStatus::Active);
        assert!(!state.acknowledged);
        assert!(state.next_escalation_at.is_some());
    }

    #[test]
    fn test_should_escalate() {
        let mut state = EscalationState::new(Uuid::new_v4(), Uuid::new_v4(), 0);

        // Should escalate immediately if delay is 0
        assert!(state.should_escalate());

        // After acknowledgment, should not escalate
        state.acknowledge("test@example.com".to_string());
        assert!(!state.should_escalate());
    }

    #[test]
    fn test_advance_level() {
        let mut state = EscalationState::new(Uuid::new_v4(), Uuid::new_v4(), 5);

        assert_eq!(state.current_level, 0);

        state.advance_to_next_level(10);
        assert_eq!(state.current_level, 1);
        assert!(state.next_escalation_at.is_some());

        state.advance_to_next_level(0); // No more levels
        assert_eq!(state.current_level, 2);
        assert_eq!(state.status, EscalationStatus::Completed);
        assert!(state.next_escalation_at.is_none());
    }

    #[test]
    fn test_acknowledge() {
        let mut state = EscalationState::new(Uuid::new_v4(), Uuid::new_v4(), 5);

        state.acknowledge("oncall@example.com".to_string());

        assert!(state.acknowledged);
        assert!(state.acknowledged_at.is_some());
        assert_eq!(state.acknowledged_by, Some("oncall@example.com".to_string()));
        assert_eq!(state.status, EscalationStatus::Acknowledged);
    }

    #[test]
    fn test_reset_for_repeat() {
        let mut state = EscalationState::new(Uuid::new_v4(), Uuid::new_v4(), 5);

        state.advance_to_next_level(10);
        state.acknowledge("test@example.com".to_string());

        assert_eq!(state.current_level, 1);
        assert!(state.acknowledged);

        state.reset_for_repeat(5);

        assert_eq!(state.current_level, 0);
        assert!(!state.acknowledged);
        assert_eq!(state.repeat_count, 1);
        assert_eq!(state.status, EscalationStatus::Active);
    }
}
