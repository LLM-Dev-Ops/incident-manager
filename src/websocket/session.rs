//! WebSocket session management
//!
//! This module handles session lifecycle, authentication, and subscription tracking.

use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::messages::{EventType, SubscriptionFilters};

/// WebSocket session
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub id: String,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_active: DateTime<Utc>,
    /// User identifier (if authenticated)
    pub user_id: Option<String>,
    /// Active subscriptions
    pub subscriptions: HashMap<String, Subscription>,
    /// Session metadata
    pub metadata: HashMap<String, String>,
    /// Message counter for tracking
    pub message_count: u64,
}

impl Session {
    /// Create a new session
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            last_active: now,
            user_id: None,
            subscriptions: HashMap::new(),
            metadata: HashMap::new(),
            message_count: 0,
        }
    }

    /// Create a session with a specific ID
    pub fn with_id(id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            created_at: now,
            last_active: now,
            user_id: None,
            subscriptions: HashMap::new(),
            metadata: HashMap::new(),
            message_count: 0,
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_active = Utc::now();
        self.message_count += 1;
    }

    /// Check if session is expired
    pub fn is_expired(&self, timeout_secs: i64) -> bool {
        let now = Utc::now();
        (now - self.last_active).num_seconds() > timeout_secs
    }

    /// Add a subscription
    pub fn subscribe(&mut self, subscription_id: String, filters: SubscriptionFilters) {
        self.subscriptions.insert(
            subscription_id.clone(),
            Subscription {
                id: subscription_id,
                filters,
                created_at: Utc::now(),
                event_count: 0,
            },
        );
        self.touch();
    }

    /// Remove a subscription
    pub fn unsubscribe(&mut self, subscription_id: &str) -> bool {
        self.touch();
        self.subscriptions.remove(subscription_id).is_some()
    }

    /// Get all event types this session is interested in
    pub fn interested_event_types(&self) -> HashSet<EventType> {
        let mut types = HashSet::new();
        for sub in self.subscriptions.values() {
            if sub.filters.event_types.is_empty() {
                // Empty filter means all events
                types.insert(EventType::IncidentCreated);
                types.insert(EventType::IncidentUpdated);
                types.insert(EventType::IncidentResolved);
                types.insert(EventType::IncidentClosed);
                types.insert(EventType::AlertReceived);
                types.insert(EventType::AlertConverted);
                types.insert(EventType::Escalated);
                types.insert(EventType::PlaybookStarted);
                types.insert(EventType::PlaybookActionExecuted);
                types.insert(EventType::PlaybookCompleted);
                types.insert(EventType::NotificationSent);
                types.insert(EventType::AssignmentChanged);
                types.insert(EventType::CommentAdded);
                types.insert(EventType::SystemEvent);
            } else {
                types.extend(sub.filters.event_types.iter());
            }
        }
        types
    }

    /// Check if session has any active subscriptions
    pub fn has_subscriptions(&self) -> bool {
        !self.subscriptions.is_empty()
    }

    /// Get session age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// Get idle time in seconds
    pub fn idle_seconds(&self) -> i64 {
        (Utc::now() - self.last_active).num_seconds()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscription to specific events
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Subscription identifier
    pub id: String,
    /// Event filters
    pub filters: SubscriptionFilters,
    /// When subscription was created
    pub created_at: DateTime<Utc>,
    /// Number of events delivered via this subscription
    pub event_count: u64,
}

impl Subscription {
    /// Record that an event was delivered
    pub fn record_event(&mut self) {
        self.event_count += 1;
    }
}

/// Session statistics
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub total_subscriptions: u64,
    pub total_messages: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new();
        assert!(!session.id.is_empty());
        assert!(session.user_id.is_none());
        assert!(session.subscriptions.is_empty());
        assert_eq!(session.message_count, 0);
    }

    #[test]
    fn test_session_activity() {
        let mut session = Session::new();
        let initial_active = session.last_active;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.last_active > initial_active);
        assert_eq!(session.message_count, 1);
    }

    #[test]
    fn test_session_expiration() {
        let mut session = Session::new();
        assert!(!session.is_expired(3600)); // 1 hour timeout

        // Simulate old session
        session.last_active = Utc::now() - chrono::Duration::seconds(7200);
        assert!(session.is_expired(3600)); // Should be expired
    }

    #[test]
    fn test_subscription_management() {
        let mut session = Session::new();
        assert!(!session.has_subscriptions());

        let filters = SubscriptionFilters {
            event_types: vec![EventType::IncidentCreated],
            ..Default::default()
        };

        session.subscribe("sub1".to_string(), filters);
        assert!(session.has_subscriptions());
        assert_eq!(session.subscriptions.len(), 1);

        let removed = session.unsubscribe("sub1");
        assert!(removed);
        assert!(!session.has_subscriptions());

        let not_found = session.unsubscribe("nonexistent");
        assert!(!not_found);
    }

    #[test]
    fn test_interested_event_types() {
        let mut session = Session::new();

        // Empty subscriptions = no interests
        let types = session.interested_event_types();
        assert!(types.is_empty());

        // Add subscription with specific event types
        let filters = SubscriptionFilters {
            event_types: vec![EventType::IncidentCreated, EventType::AlertReceived],
            ..Default::default()
        };
        session.subscribe("sub1".to_string(), filters);

        let types = session.interested_event_types();
        assert!(types.contains(&EventType::IncidentCreated));
        assert!(types.contains(&EventType::AlertReceived));
        assert!(!types.contains(&EventType::CommentAdded));
    }

    #[test]
    fn test_session_metrics() {
        let session = Session::new();
        assert!(session.age_seconds() >= 0);
        assert!(session.idle_seconds() >= 0);
    }
}
