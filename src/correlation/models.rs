use crate::models::Incident;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a correlation between incidents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correlation {
    /// Unique correlation ID
    pub id: Uuid,

    /// Incidents involved in this correlation
    pub incident_ids: Vec<Uuid>,

    /// Primary (root cause) incident
    pub primary_incident_id: Option<Uuid>,

    /// Correlation score (0.0 - 1.0, higher = more correlated)
    pub score: f64,

    /// Correlation type
    pub correlation_type: CorrelationType,

    /// When correlation was detected
    pub detected_at: DateTime<Utc>,

    /// Correlation metadata
    pub metadata: HashMap<String, String>,

    /// Correlation reasoning
    pub reason: String,
}

/// Type of correlation detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CorrelationType {
    /// Time-based correlation (incidents close in time)
    Temporal,

    /// Pattern-based correlation (similar content)
    Pattern,

    /// Source-based correlation (same source system)
    Source,

    /// Fingerprint-based correlation (similar fingerprints)
    Fingerprint,

    /// Topology-based correlation (related infrastructure)
    Topology,

    /// Manual correlation by operator
    Manual,

    /// Combined correlation (multiple signals)
    Combined,
}

/// Represents a group of correlated incidents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationGroup {
    /// Group ID
    pub id: Uuid,

    /// Name/title of the group
    pub title: String,

    /// Primary (root cause) incident
    pub primary_incident_id: Uuid,

    /// Related incidents
    pub related_incident_ids: Vec<Uuid>,

    /// Group creation time
    pub created_at: DateTime<Utc>,

    /// Last updated time
    pub updated_at: DateTime<Utc>,

    /// Group status
    pub status: GroupStatus,

    /// Correlations that formed this group
    pub correlations: Vec<Correlation>,

    /// Aggregate score
    pub aggregate_score: f64,

    /// Group metadata
    pub metadata: HashMap<String, String>,
}

/// Status of a correlation group
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupStatus {
    /// Active group, still accumulating incidents
    Active,

    /// Stable group, no new correlations expected
    Stable,

    /// Resolved group
    Resolved,

    /// Archived group
    Archived,
}

/// Result of correlation analysis
#[derive(Debug, Clone)]
pub struct CorrelationResult {
    /// New correlations detected
    pub correlations: Vec<Correlation>,

    /// Groups affected
    pub groups_affected: Vec<Uuid>,

    /// New groups created
    pub groups_created: Vec<CorrelationGroup>,

    /// Processing time
    pub processing_time_ms: u64,
}

/// Configuration for correlation analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationConfig {
    /// Enable correlation engine
    pub enabled: bool,

    /// Time window for temporal correlation (seconds)
    pub temporal_window_secs: u64,

    /// Minimum score threshold to consider incidents correlated
    pub min_correlation_score: f64,

    /// Maximum incidents per group
    pub max_group_size: usize,

    /// Enable temporal correlation
    pub enable_temporal: bool,

    /// Enable pattern correlation
    pub enable_pattern: bool,

    /// Enable source correlation
    pub enable_source: bool,

    /// Enable fingerprint correlation
    pub enable_fingerprint: bool,

    /// Enable topology correlation
    pub enable_topology: bool,

    /// Pattern similarity threshold (0.0 - 1.0)
    pub pattern_similarity_threshold: f64,

    /// Auto-merge groups with high correlation
    pub auto_merge_groups: bool,

    /// Merge threshold score
    pub merge_threshold: f64,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            temporal_window_secs: 300, // 5 minutes
            min_correlation_score: 0.5,
            max_group_size: 100,
            enable_temporal: true,
            enable_pattern: true,
            enable_source: true,
            enable_fingerprint: true,
            enable_topology: false, // Disabled by default, needs topology data
            pattern_similarity_threshold: 0.7,
            auto_merge_groups: true,
            merge_threshold: 0.8,
        }
    }
}

impl Correlation {
    /// Create a new correlation
    pub fn new(
        incident_ids: Vec<Uuid>,
        score: f64,
        correlation_type: CorrelationType,
        reason: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            incident_ids,
            primary_incident_id: None,
            score,
            correlation_type,
            detected_at: Utc::now(),
            metadata: HashMap::new(),
            reason,
        }
    }

    /// Check if correlation involves an incident
    pub fn involves_incident(&self, incident_id: &Uuid) -> bool {
        self.incident_ids.contains(incident_id)
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

impl CorrelationGroup {
    /// Create a new correlation group
    pub fn new(primary_incident: &Incident) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: format!("Correlated: {}", primary_incident.title),
            primary_incident_id: primary_incident.id,
            related_incident_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: GroupStatus::Active,
            correlations: Vec::new(),
            aggregate_score: 0.0,
            metadata: HashMap::new(),
        }
    }

    /// Add an incident to the group
    pub fn add_incident(&mut self, incident_id: Uuid, correlation: Correlation) {
        if !self.related_incident_ids.contains(&incident_id) && incident_id != self.primary_incident_id {
            self.related_incident_ids.push(incident_id);
            self.correlations.push(correlation);
            self.updated_at = Utc::now();
            self.recalculate_aggregate_score();
        }
    }

    /// Remove an incident from the group
    pub fn remove_incident(&mut self, incident_id: &Uuid) {
        self.related_incident_ids.retain(|id| id != incident_id);
        self.correlations.retain(|c| !c.involves_incident(incident_id));
        self.updated_at = Utc::now();
        self.recalculate_aggregate_score();
    }

    /// Check if group contains an incident
    pub fn contains_incident(&self, incident_id: &Uuid) -> bool {
        self.primary_incident_id == *incident_id || self.related_incident_ids.contains(incident_id)
    }

    /// Get all incident IDs in the group
    pub fn all_incident_ids(&self) -> Vec<Uuid> {
        let mut ids = vec![self.primary_incident_id];
        ids.extend(&self.related_incident_ids);
        ids
    }

    /// Get group size
    pub fn size(&self) -> usize {
        1 + self.related_incident_ids.len()
    }

    /// Recalculate aggregate score
    fn recalculate_aggregate_score(&mut self) {
        if self.correlations.is_empty() {
            self.aggregate_score = 0.0;
        } else {
            let sum: f64 = self.correlations.iter().map(|c| c.score).sum();
            self.aggregate_score = sum / self.correlations.len() as f64;
        }
    }

    /// Mark group as resolved
    pub fn resolve(&mut self) {
        self.status = GroupStatus::Resolved;
        self.updated_at = Utc::now();
    }

    /// Mark group as stable
    pub fn stabilize(&mut self) {
        self.status = GroupStatus::Stable;
        self.updated_at = Utc::now();
    }
}

impl CorrelationResult {
    /// Create an empty result
    pub fn empty() -> Self {
        Self {
            correlations: Vec::new(),
            groups_affected: Vec::new(),
            groups_created: Vec::new(),
            processing_time_ms: 0,
        }
    }

    /// Check if any correlations were found
    pub fn has_correlations(&self) -> bool {
        !self.correlations.is_empty()
    }

    /// Get total number of correlations
    pub fn correlation_count(&self) -> usize {
        self.correlations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};

    #[test]
    fn test_correlation_creation() {
        let correlation = Correlation::new(
            vec![Uuid::new_v4(), Uuid::new_v4()],
            0.85,
            CorrelationType::Temporal,
            "Incidents occurred within 60 seconds".to_string(),
        );

        assert_eq!(correlation.incident_ids.len(), 2);
        assert_eq!(correlation.score, 0.85);
        assert_eq!(correlation.correlation_type, CorrelationType::Temporal);
    }

    #[test]
    fn test_correlation_group_creation() {
        let incident = Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let group = CorrelationGroup::new(&incident);

        assert_eq!(group.primary_incident_id, incident.id);
        assert_eq!(group.size(), 1);
        assert_eq!(group.status, GroupStatus::Active);
    }

    #[test]
    fn test_add_incident_to_group() {
        let incident = Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let mut group = CorrelationGroup::new(&incident);

        let related_id = Uuid::new_v4();
        let correlation = Correlation::new(
            vec![incident.id, related_id],
            0.8,
            CorrelationType::Pattern,
            "Similar patterns".to_string(),
        );

        group.add_incident(related_id, correlation);

        assert_eq!(group.size(), 2);
        assert!(group.contains_incident(&related_id));
        assert_eq!(group.aggregate_score, 0.8);
    }

    #[test]
    fn test_aggregate_score_calculation() {
        let incident = Incident::new(
            "test".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let mut group = CorrelationGroup::new(&incident);

        // Add multiple correlated incidents
        let corr1 = Correlation::new(
            vec![incident.id, Uuid::new_v4()],
            0.8,
            CorrelationType::Temporal,
            "Reason".to_string(),
        );

        let corr2 = Correlation::new(
            vec![incident.id, Uuid::new_v4()],
            0.6,
            CorrelationType::Pattern,
            "Reason".to_string(),
        );

        group.add_incident(Uuid::new_v4(), corr1);
        group.add_incident(Uuid::new_v4(), corr2);

        // Aggregate score should be average: (0.8 + 0.6) / 2 = 0.7
        assert_eq!(group.aggregate_score, 0.7);
    }

    #[test]
    fn test_correlation_involves_incident() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let correlation = Correlation::new(
            vec![id1, id2],
            0.75,
            CorrelationType::Source,
            "Same source".to_string(),
        );

        assert!(correlation.involves_incident(&id1));
        assert!(correlation.involves_incident(&id2));
        assert!(!correlation.involves_incident(&Uuid::new_v4()));
    }

    #[test]
    fn test_group_all_incident_ids() {
        let incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let mut group = CorrelationGroup::new(&incident);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let corr1 = Correlation::new(
            vec![incident.id, id1],
            0.8,
            CorrelationType::Temporal,
            "Reason".to_string(),
        );

        let corr2 = Correlation::new(
            vec![incident.id, id2],
            0.7,
            CorrelationType::Temporal,
            "Reason".to_string(),
        );

        group.add_incident(id1, corr1);
        group.add_incident(id2, corr2);

        let all_ids = group.all_incident_ids();
        assert_eq!(all_ids.len(), 3);
        assert!(all_ids.contains(&incident.id));
        assert!(all_ids.contains(&id1));
        assert!(all_ids.contains(&id2));
    }
}
