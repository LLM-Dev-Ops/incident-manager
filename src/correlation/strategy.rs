use crate::correlation::models::{Correlation, CorrelationConfig, CorrelationType};
use crate::error::Result;
use crate::models::Incident;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for correlation strategies
#[async_trait]
pub trait CorrelationStrategy: Send + Sync {
    /// Analyze if two incidents are correlated
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>>;

    /// Get strategy name
    fn name(&self) -> &str;

    /// Get correlation type this strategy produces
    fn correlation_type(&self) -> CorrelationType;
}

/// Temporal correlation strategy - correlates incidents close in time
pub struct TemporalStrategy;

impl TemporalStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Calculate time-based score
    fn calculate_temporal_score(time_diff_secs: i64, window_secs: u64) -> f64 {
        if time_diff_secs < 0 {
            return 0.0;
        }

        let time_diff = time_diff_secs as f64;
        let window = window_secs as f64;

        if time_diff > window {
            0.0
        } else {
            // Exponential decay: score decreases as time difference increases
            let decay_rate = 3.0 / window; // 95% decay at window edge
            (-decay_rate * time_diff).exp()
        }
    }
}

#[async_trait]
impl CorrelationStrategy for TemporalStrategy {
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>> {
        if !config.enable_temporal {
            return Ok(None);
        }

        let time_diff = (incident2.created_at - incident1.created_at)
            .num_seconds()
            .abs();

        let score = Self::calculate_temporal_score(time_diff, config.temporal_window_secs);

        if score >= config.min_correlation_score {
            let correlation = Correlation::new(
                vec![incident1.id, incident2.id],
                score,
                CorrelationType::Temporal,
                format!(
                    "Incidents occurred within {} seconds of each other",
                    time_diff
                ),
            );

            Ok(Some(correlation))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "temporal"
    }

    fn correlation_type(&self) -> CorrelationType {
        CorrelationType::Temporal
    }
}

/// Pattern correlation strategy - correlates incidents with similar content
pub struct PatternStrategy;

impl PatternStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Calculate Jaccard similarity between two strings
    fn jaccard_similarity(s1: &str, s2: &str) -> f64 {
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();
        let words1: std::collections::HashSet<_> =
            s1_lower.split_whitespace().collect();
        let words2: std::collections::HashSet<_> =
            s2_lower.split_whitespace().collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Calculate Levenshtein distance ratio
    fn levenshtein_ratio(s1: &str, s2: &str) -> f64 {
        let len1 = s1.len();
        let len2 = s2.len();

        if len1 == 0 && len2 == 0 {
            return 1.0;
        }

        let distance = Self::levenshtein_distance(s1, s2);
        let max_len = len1.max(len2);

        1.0 - (distance as f64 / max_len as f64)
    }

    /// Calculate Levenshtein distance
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }

        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for (i, c1) in s1.chars().enumerate() {
            for (j, c2) in s2.chars().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };

                matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                    .min(matrix[i + 1][j] + 1)
                    .min(matrix[i][j] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// Calculate overall pattern similarity
    fn calculate_pattern_score(incident1: &Incident, incident2: &Incident) -> f64 {
        // Title similarity (weight: 0.6)
        let title_sim = Self::jaccard_similarity(&incident1.title, &incident2.title);

        // Description similarity (weight: 0.3)
        let desc_sim = Self::jaccard_similarity(&incident1.description, &incident2.description);

        // Exact match bonuses
        let severity_match = if incident1.severity == incident2.severity {
            0.1
        } else {
            0.0
        };

        let type_match = if incident1.incident_type == incident2.incident_type {
            0.1
        } else {
            0.0
        };

        // Weighted combination
        let base_score = (title_sim * 0.6) + (desc_sim * 0.3);
        let bonus = severity_match + type_match;

        (base_score + bonus).min(1.0)
    }
}

#[async_trait]
impl CorrelationStrategy for PatternStrategy {
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>> {
        if !config.enable_pattern {
            return Ok(None);
        }

        let score = Self::calculate_pattern_score(incident1, incident2);

        if score >= config.pattern_similarity_threshold {
            let correlation = Correlation::new(
                vec![incident1.id, incident2.id],
                score,
                CorrelationType::Pattern,
                format!(
                    "Incidents have similar patterns (similarity: {:.2})",
                    score
                ),
            );

            Ok(Some(correlation))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "pattern"
    }

    fn correlation_type(&self) -> CorrelationType {
        CorrelationType::Pattern
    }
}

/// Source correlation strategy - correlates incidents from the same source
pub struct SourceStrategy;

impl SourceStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Calculate source similarity score
    fn calculate_source_score(incident1: &Incident, incident2: &Incident) -> f64 {
        // Exact source match
        if incident1.source == incident2.source {
            return 1.0;
        }

        // Partial source match (e.g., "service-a" and "service-a-worker")
        let source1_lower = incident1.source.to_lowercase();
        let source2_lower = incident2.source.to_lowercase();

        if source1_lower.contains(&source2_lower) || source2_lower.contains(&source1_lower) {
            return 0.8;
        }

        // Check for common prefixes
        let common_prefix_len = source1_lower
            .chars()
            .zip(source2_lower.chars())
            .take_while(|(a, b)| a == b)
            .count();

        let min_len = source1_lower.len().min(source2_lower.len());

        if min_len > 0 && common_prefix_len >= min_len / 2 {
            return 0.6;
        }

        0.0
    }
}

#[async_trait]
impl CorrelationStrategy for SourceStrategy {
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>> {
        if !config.enable_source {
            return Ok(None);
        }

        let score = Self::calculate_source_score(incident1, incident2);

        if score >= config.min_correlation_score {
            let correlation = Correlation::new(
                vec![incident1.id, incident2.id],
                score,
                CorrelationType::Source,
                format!(
                    "Incidents from related sources: '{}' and '{}'",
                    incident1.source, incident2.source
                ),
            );

            Ok(Some(correlation))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "source"
    }

    fn correlation_type(&self) -> CorrelationType {
        CorrelationType::Source
    }
}

/// Fingerprint correlation strategy - correlates incidents with similar fingerprints
pub struct FingerprintStrategy;

impl FingerprintStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Calculate fingerprint similarity
    fn calculate_fingerprint_score(
        fingerprint1: Option<&String>,
        fingerprint2: Option<&String>,
    ) -> f64 {
        match (fingerprint1, fingerprint2) {
            (Some(fp1), Some(fp2)) => {
                if fp1 == fp2 {
                    1.0
                } else {
                    // Calculate string similarity for partial matches
                    PatternStrategy::jaccard_similarity(fp1, fp2)
                }
            }
            _ => 0.0,
        }
    }
}

#[async_trait]
impl CorrelationStrategy for FingerprintStrategy {
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>> {
        if !config.enable_fingerprint {
            return Ok(None);
        }

        let score = Self::calculate_fingerprint_score(
            incident1.fingerprint.as_ref(),
            incident2.fingerprint.as_ref(),
        );

        if score >= config.min_correlation_score {
            let correlation = Correlation::new(
                vec![incident1.id, incident2.id],
                score,
                CorrelationType::Fingerprint,
                format!(
                    "Incidents have similar fingerprints (similarity: {:.2})",
                    score
                ),
            );

            Ok(Some(correlation))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "fingerprint"
    }

    fn correlation_type(&self) -> CorrelationType {
        CorrelationType::Fingerprint
    }
}

/// Topology correlation strategy - correlates incidents affecting related infrastructure
pub struct TopologyStrategy;

impl TopologyStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Calculate topology-based correlation score
    /// This is a placeholder - real implementation would integrate with service topology
    fn calculate_topology_score(incident1: &Incident, incident2: &Incident) -> f64 {
        // Check if incidents have topology-related labels
        let has_service_label1 = incident1.labels.contains_key("service");
        let has_service_label2 = incident2.labels.contains_key("service");

        if !has_service_label1 || !has_service_label2 {
            return 0.0;
        }

        let service1 = incident1.labels.get("service").unwrap();
        let service2 = incident2.labels.get("service").unwrap();

        // Same service
        if service1 == service2 {
            return 0.9;
        }

        // Check for dependency hints in labels
        if let (Some(deps1), Some(deps2)) = (
            incident1.labels.get("depends_on"),
            incident2.labels.get("depends_on"),
        ) {
            if deps1.contains(service2) || deps2.contains(service1) {
                return 0.8;
            }
        }

        // Check for common infrastructure
        if let (Some(infra1), Some(infra2)) = (
            incident1.labels.get("infrastructure"),
            incident2.labels.get("infrastructure"),
        ) {
            if infra1 == infra2 {
                return 0.7;
            }
        }

        0.0
    }
}

#[async_trait]
impl CorrelationStrategy for TopologyStrategy {
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>> {
        if !config.enable_topology {
            return Ok(None);
        }

        let score = Self::calculate_topology_score(incident1, incident2);

        if score >= config.min_correlation_score {
            let correlation = Correlation::new(
                vec![incident1.id, incident2.id],
                score,
                CorrelationType::Topology,
                "Incidents affect related infrastructure components".to_string(),
            );

            Ok(Some(correlation))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "topology"
    }

    fn correlation_type(&self) -> CorrelationType {
        CorrelationType::Topology
    }
}

/// Combined correlation strategy - uses multiple strategies
pub struct CombinedStrategy {
    strategies: Vec<Arc<dyn CorrelationStrategy>>,
}

impl CombinedStrategy {
    pub fn new(strategies: Vec<Arc<dyn CorrelationStrategy>>) -> Self {
        Self { strategies }
    }

    /// Create default combined strategy with all built-in strategies
    pub fn default_strategies() -> Self {
        let strategies: Vec<Arc<dyn CorrelationStrategy>> = vec![
            Arc::new(TemporalStrategy::new()),
            Arc::new(PatternStrategy::new()),
            Arc::new(SourceStrategy::new()),
            Arc::new(FingerprintStrategy::new()),
            Arc::new(TopologyStrategy::new()),
        ];

        Self::new(strategies)
    }
}

#[async_trait]
impl CorrelationStrategy for CombinedStrategy {
    async fn correlate(
        &self,
        incident1: &Incident,
        incident2: &Incident,
        config: &CorrelationConfig,
    ) -> Result<Option<Correlation>> {
        let mut correlations = Vec::new();

        // Run all strategies
        for strategy in &self.strategies {
            if let Some(corr) = strategy.correlate(incident1, incident2, config).await? {
                correlations.push(corr);
            }
        }

        if correlations.is_empty() {
            return Ok(None);
        }

        // Calculate combined score (weighted average)
        let total_score: f64 = correlations.iter().map(|c| c.score).sum();
        let avg_score = total_score / correlations.len() as f64;

        // Boost score if multiple strategies agree
        let boost = if correlations.len() > 1 {
            0.1 * (correlations.len() - 1) as f64
        } else {
            0.0
        };

        let final_score = (avg_score + boost).min(1.0);

        if final_score >= config.min_correlation_score {
            let reasons: Vec<String> = correlations
                .iter()
                .map(|c| format!("{:?}: {}", c.correlation_type, c.reason))
                .collect();

            let correlation = Correlation::new(
                vec![incident1.id, incident2.id],
                final_score,
                CorrelationType::Combined,
                reasons.join("; "),
            );

            Ok(Some(correlation))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "combined"
    }

    fn correlation_type(&self) -> CorrelationType {
        CorrelationType::Combined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};

    fn create_test_incident(title: &str, source: &str) -> Incident {
        Incident::new(
            source.to_string(),
            title.to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[tokio::test]
    async fn test_temporal_correlation() {
        let strategy = TemporalStrategy::new();
        let config = CorrelationConfig::default();

        let incident1 = create_test_incident("Incident 1", "service-a");
        let mut incident2 = create_test_incident("Incident 2", "service-b");

        // Set incident2 to be 60 seconds after incident1
        incident2.created_at = incident1.created_at + chrono::Duration::seconds(60);

        let result = strategy
            .correlate(&incident1, &incident2, &config)
            .await
            .unwrap();

        assert!(result.is_some());
        let correlation = result.unwrap();
        assert!(correlation.score > 0.5);
        assert_eq!(correlation.correlation_type, CorrelationType::Temporal);
    }

    #[tokio::test]
    async fn test_pattern_correlation() {
        let strategy = PatternStrategy::new();
        let config = CorrelationConfig::default();

        let incident1 = create_test_incident("High CPU usage on server-1", "service-a");
        let incident2 = create_test_incident("High CPU usage on server-2", "service-b");

        let result = strategy
            .correlate(&incident1, &incident2, &config)
            .await
            .unwrap();

        assert!(result.is_some());
        let correlation = result.unwrap();
        assert!(correlation.score >= 0.7);
        assert_eq!(correlation.correlation_type, CorrelationType::Pattern);
    }

    #[tokio::test]
    async fn test_source_correlation() {
        let strategy = SourceStrategy::new();
        let config = CorrelationConfig::default();

        let incident1 = create_test_incident("Incident 1", "service-a");
        let incident2 = create_test_incident("Incident 2", "service-a");

        let result = strategy
            .correlate(&incident1, &incident2, &config)
            .await
            .unwrap();

        assert!(result.is_some());
        let correlation = result.unwrap();
        assert_eq!(correlation.score, 1.0);
        assert_eq!(correlation.correlation_type, CorrelationType::Source);
    }

    #[tokio::test]
    async fn test_fingerprint_correlation() {
        let strategy = FingerprintStrategy::new();
        let config = CorrelationConfig::default();

        let mut incident1 = create_test_incident("Incident 1", "service-a");
        let mut incident2 = create_test_incident("Incident 2", "service-b");

        incident1.fingerprint = Some("cpu-high-alert".to_string());
        incident2.fingerprint = Some("cpu-high-alert".to_string());

        let result = strategy
            .correlate(&incident1, &incident2, &config)
            .await
            .unwrap();

        assert!(result.is_some());
        let correlation = result.unwrap();
        assert_eq!(correlation.score, 1.0);
        assert_eq!(correlation.correlation_type, CorrelationType::Fingerprint);
    }

    #[test]
    fn test_jaccard_similarity() {
        let sim = PatternStrategy::jaccard_similarity("hello world", "hello universe");
        assert!(sim > 0.0 && sim < 1.0);

        let sim = PatternStrategy::jaccard_similarity("hello world", "hello world");
        assert_eq!(sim, 1.0);

        let sim = PatternStrategy::jaccard_similarity("abc", "xyz");
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_temporal_score_calculation() {
        // Within window, should have high score
        let score = TemporalStrategy::calculate_temporal_score(30, 300);
        assert!(score > 0.7);

        // At window edge, should have lower score
        let score = TemporalStrategy::calculate_temporal_score(300, 300);
        assert!(score < 0.1);

        // Outside window, should be 0
        let score = TemporalStrategy::calculate_temporal_score(400, 300);
        assert_eq!(score, 0.0);
    }
}
