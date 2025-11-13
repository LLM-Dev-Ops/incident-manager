use crate::error::{AppError, Result};
use crate::ml::models::{FeatureConfig, TrainingSample};
use crate::models::Incident;
use chrono::{Datelike, Timelike};
use ndarray::Array1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Feature extractor for incidents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureExtractor {
    /// Configuration
    config: FeatureConfig,

    /// Vocabulary mapping (term -> index)
    vocabulary: HashMap<String, usize>,

    /// Inverse document frequency (IDF) values
    idf_values: HashMap<String, f64>,

    /// Source encodings
    source_encodings: HashMap<String, usize>,

    /// Number of text features
    n_text_features: usize,

    /// Number of categorical features
    n_categorical_features: usize,

    /// Number of temporal features
    n_temporal_features: usize,

    /// Total number of features
    n_features: usize,

    /// Is fitted (vocabulary built)
    is_fitted: bool,
}

impl FeatureExtractor {
    /// Create a new feature extractor
    pub fn new(config: FeatureConfig) -> Self {
        Self {
            config,
            vocabulary: HashMap::new(),
            idf_values: HashMap::new(),
            source_encodings: HashMap::new(),
            n_text_features: 0,
            n_categorical_features: 0,
            n_temporal_features: 0,
            n_features: 0,
            is_fitted: false,
        }
    }

    /// Fit the feature extractor on a set of incidents
    pub fn fit(&mut self, incidents: &[Incident]) -> Result<()> {
        // Build vocabulary from all text
        let mut term_doc_freq: HashMap<String, usize> = HashMap::new();
        let mut sources: Vec<String> = Vec::new();

        for incident in incidents {
            // Extract terms
            let terms = self.extract_terms(incident);
            let unique_terms: std::collections::HashSet<_> = terms.into_iter().collect();

            for term in unique_terms {
                *term_doc_freq.entry(term).or_insert(0) += 1;
            }

            // Collect sources
            if !sources.contains(&incident.source) {
                sources.push(incident.source.clone());
            }
        }

        // Filter vocabulary by document frequency
        let min_df = self.config.min_doc_freq;
        let mut vocab_list: Vec<(String, usize)> = term_doc_freq
            .into_iter()
            .filter(|(_, freq)| *freq >= min_df)
            .collect();

        // Sort by frequency and limit vocabulary size
        vocab_list.sort_by(|a, b| b.1.cmp(&a.1));
        vocab_list.truncate(self.config.max_vocab_size);

        // Build vocabulary index
        self.vocabulary = vocab_list
            .into_iter()
            .enumerate()
            .map(|(idx, (term, _))| (term, idx))
            .collect();

        self.n_text_features = self.vocabulary.len();

        // Calculate IDF values if using TF-IDF
        if self.config.use_tfidf {
            let n_docs = incidents.len() as f64;
            for (term, _) in &self.vocabulary {
                let doc_freq = incidents
                    .iter()
                    .filter(|inc| {
                        let terms = self.extract_terms(inc);
                        terms.contains(term)
                    })
                    .count() as f64;

                let idf = (n_docs / (1.0 + doc_freq)).ln() + 1.0;
                self.idf_values.insert(term.clone(), idf);
            }
        }

        // Encode sources
        self.source_encodings = sources
            .into_iter()
            .enumerate()
            .map(|(idx, source)| (source, idx))
            .collect();

        self.n_categorical_features = self.source_encodings.len();

        // Calculate temporal features count
        self.n_temporal_features = if self.config.include_temporal {
            4 // hour, day_of_week, is_weekend, is_business_hours
        } else {
            0
        };

        // Calculate total features
        self.n_features = self.n_text_features
            + self.n_categorical_features
            + self.n_temporal_features
            + 2; // severity_numeric, type_numeric

        self.is_fitted = true;

        Ok(())
    }

    /// Transform an incident into a feature vector
    pub fn transform(&self, incident: &Incident) -> Result<Vec<f64>> {
        if !self.is_fitted {
            return Err(AppError::Internal(
                "FeatureExtractor must be fitted before transform".to_string(),
            ));
        }

        let mut features = vec![0.0; self.n_features];
        let mut offset = 0;

        // Text features (TF-IDF or term frequency)
        let terms = self.extract_terms(incident);
        let term_counts = self.count_terms(&terms);

        for (term, idx) in &self.vocabulary {
            if let Some(&count) = term_counts.get(term) {
                let tf = count as f64;
                let value = if self.config.use_tfidf {
                    let idf = self.idf_values.get(term).unwrap_or(&1.0);
                    tf * idf
                } else {
                    tf
                };
                features[offset + idx] = value;
            }
        }
        offset += self.n_text_features;

        // Source features (one-hot encoding)
        if let Some(&source_idx) = self.source_encodings.get(&incident.source) {
            if source_idx < self.n_categorical_features {
                features[offset + source_idx] = 1.0;
            }
        }
        offset += self.n_categorical_features;

        // Temporal features
        if self.config.include_temporal {
            let temporal = self.extract_temporal_features(incident);
            features[offset..offset + 4].copy_from_slice(&temporal);
            offset += self.n_temporal_features;
        }

        // Severity numeric
        features[offset] = self.severity_to_numeric(&incident.severity);
        offset += 1;

        // Type numeric
        features[offset] = self.type_to_numeric(&incident.incident_type);

        Ok(features)
    }

    /// Fit and transform in one step
    pub fn fit_transform(&mut self, incidents: &[Incident]) -> Result<Vec<Vec<f64>>> {
        self.fit(incidents)?;
        incidents.iter().map(|inc| self.transform(inc)).collect()
    }

    /// Extract terms from incident text
    fn extract_terms(&self, incident: &Incident) -> Vec<String> {
        let text = format!("{} {}", incident.title, incident.description);
        let text = text.to_lowercase();

        // Simple tokenization: split on whitespace and punctuation
        let words: Vec<String> = text
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|w| !w.is_empty() && w.len() > 2) // Filter short words
            .map(|w| w.to_string())
            .collect();

        // Generate n-grams
        let mut terms = Vec::new();

        for n in self.config.ngram_range.0..=self.config.ngram_range.1 {
            for window in words.windows(n) {
                terms.push(window.join("_"));
            }
        }

        terms
    }

    /// Count term occurrences
    fn count_terms(&self, terms: &[String]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for term in terms {
            *counts.entry(term.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Extract temporal features from incident
    fn extract_temporal_features(&self, incident: &Incident) -> [f64; 4] {
        let dt = incident.created_at;

        // Hour of day (0-23) normalized to 0-1
        let hour = dt.hour() as f64 / 23.0;

        // Day of week (0-6) normalized to 0-1
        let day_of_week = dt.weekday().num_days_from_monday() as f64 / 6.0;

        // Is weekend (0 or 1)
        let is_weekend = if dt.weekday().num_days_from_monday() >= 5 {
            1.0
        } else {
            0.0
        };

        // Is business hours (9-17) (0 or 1)
        let is_business_hours = if dt.hour() >= 9 && dt.hour() < 17 {
            1.0
        } else {
            0.0
        };

        [hour, day_of_week, is_weekend, is_business_hours]
    }

    /// Convert severity to numeric value
    fn severity_to_numeric(&self, severity: &crate::models::Severity) -> f64 {
        use crate::models::Severity;
        match severity {
            Severity::P0 => 0.0,
            Severity::P1 => 0.25,
            Severity::P2 => 0.5,
            Severity::P3 => 0.75,
            Severity::P4 => 1.0,
        }
    }

    /// Convert type to numeric value
    fn type_to_numeric(&self, incident_type: &crate::models::IncidentType) -> f64 {
        use crate::models::IncidentType;
        match incident_type {
            IncidentType::Infrastructure => 0.0,
            IncidentType::Application => 0.14,
            IncidentType::Security => 0.29,
            IncidentType::Data => 0.43,
            IncidentType::Performance => 0.57,
            IncidentType::Availability => 0.71,
            IncidentType::Compliance => 0.86,
            IncidentType::Unknown => 1.0,
        }
    }

    /// Convert numeric to severity
    pub fn numeric_to_severity(value: f64) -> crate::models::Severity {
        use crate::models::Severity;
        if value < 0.125 {
            Severity::P0
        } else if value < 0.375 {
            Severity::P1
        } else if value < 0.625 {
            Severity::P2
        } else if value < 0.875 {
            Severity::P3
        } else {
            Severity::P4
        }
    }

    /// Convert numeric to type
    pub fn numeric_to_type(value: f64) -> crate::models::IncidentType {
        use crate::models::IncidentType;
        if value < 0.07 {
            IncidentType::Infrastructure
        } else if value < 0.215 {
            IncidentType::Application
        } else if value < 0.36 {
            IncidentType::Security
        } else if value < 0.5 {
            IncidentType::Data
        } else if value < 0.64 {
            IncidentType::Performance
        } else if value < 0.785 {
            IncidentType::Availability
        } else if value < 0.93 {
            IncidentType::Compliance
        } else {
            IncidentType::Unknown
        }
    }

    /// Get number of features
    pub fn n_features(&self) -> usize {
        self.n_features
    }

    /// Check if fitted
    pub fn is_fitted(&self) -> bool {
        self.is_fitted
    }

    /// Get vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.vocabulary.len()
    }
}

/// Text preprocessing utilities
pub struct TextPreprocessor;

impl TextPreprocessor {
    /// Remove stopwords from text
    pub fn remove_stopwords(text: &str) -> String {
        // Common English stopwords
        let stopwords = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "up", "about", "into", "through", "during", "is", "was", "are", "were",
            "been", "be", "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "must", "can",
        ];

        text.split_whitespace()
            .filter(|word| !stopwords.contains(&word.to_lowercase().as_str()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Normalize text (lowercase, trim, etc.)
    pub fn normalize(text: &str) -> String {
        text.to_lowercase().trim().to_string()
    }

    /// Extract keywords from text (simple frequency-based)
    pub fn extract_keywords(text: &str, top_k: usize) -> Vec<String> {
        let words: Vec<String> = text
            .to_lowercase()
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|w| !w.is_empty() && w.len() > 3)
            .map(|w| w.to_string())
            .collect();

        let mut word_counts: HashMap<String, usize> = HashMap::new();
        for word in words {
            *word_counts.entry(word).or_insert(0) += 1;
        }

        let mut counts: Vec<_> = word_counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));

        counts.into_iter().take(top_k).map(|(w, _)| w).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};

    fn create_test_incident(title: &str, description: &str, source: &str) -> Incident {
        Incident::new(
            source.to_string(),
            title.to_string(),
            description.to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[test]
    fn test_feature_extractor_creation() {
        let config = FeatureConfig::default();
        let extractor = FeatureExtractor::new(config);

        assert!(!extractor.is_fitted());
        assert_eq!(extractor.vocab_size(), 0);
    }

    #[test]
    fn test_feature_extraction() {
        let incidents = vec![
            create_test_incident(
                "Database connection timeout",
                "Connection to database failed",
                "monitoring",
            ),
            create_test_incident(
                "API latency spike",
                "API response time exceeded threshold",
                "monitoring",
            ),
            create_test_incident(
                "Memory leak detected",
                "Application memory usage is high",
                "monitoring",
            ),
        ];

        let mut extractor = FeatureExtractor::new(FeatureConfig::default());
        extractor.fit(&incidents).unwrap();

        assert!(extractor.is_fitted());
        assert!(extractor.vocab_size() > 0);

        let features = extractor.transform(&incidents[0]).unwrap();
        assert_eq!(features.len(), extractor.n_features());
    }

    #[test]
    fn test_fit_transform() {
        let incidents = vec![
            create_test_incident("Test 1", "Description 1", "source1"),
            create_test_incident("Test 2", "Description 2", "source2"),
        ];

        let mut extractor = FeatureExtractor::new(FeatureConfig::default());
        let features = extractor.fit_transform(&incidents).unwrap();

        assert_eq!(features.len(), 2);
        assert!(features[0].len() > 0);
    }

    #[test]
    fn test_text_preprocessing() {
        let text = "The quick brown fox jumps over the lazy dog";
        let processed = TextPreprocessor::remove_stopwords(text);

        assert!(!processed.contains("the"));
        assert!(!processed.contains("over"));
        assert!(processed.contains("quick"));
        assert!(processed.contains("brown"));
    }

    #[test]
    fn test_keyword_extraction() {
        let text = "database connection timeout database error connection pool exhausted";
        let keywords = TextPreprocessor::extract_keywords(text, 3);

        assert!(keywords.len() <= 3);
        assert!(keywords.contains(&"database".to_string()));
        assert!(keywords.contains(&"connection".to_string()));
    }

    #[test]
    fn test_severity_conversion() {
        let extractor = FeatureExtractor::new(FeatureConfig::default());

        assert_eq!(extractor.severity_to_numeric(&Severity::P0), 0.0);
        assert_eq!(extractor.severity_to_numeric(&Severity::P4), 1.0);

        assert_eq!(FeatureExtractor::numeric_to_severity(0.0), Severity::P0);
        assert_eq!(FeatureExtractor::numeric_to_severity(1.0), Severity::P4);
    }

    #[test]
    fn test_type_conversion() {
        let extractor = FeatureExtractor::new(FeatureConfig::default());

        assert_eq!(
            extractor.type_to_numeric(&IncidentType::Infrastructure),
            0.0
        );
        assert_eq!(extractor.type_to_numeric(&IncidentType::Unknown), 1.0);

        assert_eq!(
            FeatureExtractor::numeric_to_type(0.0),
            IncidentType::Infrastructure
        );
        assert_eq!(
            FeatureExtractor::numeric_to_type(1.0),
            IncidentType::Unknown
        );
    }

    #[test]
    fn test_temporal_features() {
        let incident = create_test_incident("Test", "Description", "source");
        let extractor = FeatureExtractor::new(FeatureConfig::default());

        let temporal = extractor.extract_temporal_features(&incident);

        assert_eq!(temporal.len(), 4);
        assert!(temporal[0] >= 0.0 && temporal[0] <= 1.0); // hour
        assert!(temporal[1] >= 0.0 && temporal[1] <= 1.0); // day_of_week
        assert!(temporal[2] == 0.0 || temporal[2] == 1.0); // is_weekend
        assert!(temporal[3] == 0.0 || temporal[3] == 1.0); // is_business_hours
    }

    #[test]
    fn test_ngram_extraction() {
        let incident = create_test_incident("database connection error", "timeout occurred", "test");
        let mut config = FeatureConfig::default();
        config.ngram_range = (1, 2);

        let extractor = FeatureExtractor::new(config);
        let terms = extractor.extract_terms(&incident);

        // Should have unigrams and bigrams
        assert!(terms.len() > 0);
    }
}
