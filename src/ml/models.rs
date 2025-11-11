use crate::models::{IncidentType, Severity};
use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ML model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLConfig {
    /// Enable ML classification
    pub enabled: bool,

    /// Enable severity prediction
    pub enable_severity_prediction: bool,

    /// Enable type prediction
    pub enable_type_prediction: bool,

    /// Enable priority prediction
    pub enable_priority_prediction: bool,

    /// Minimum confidence threshold (0.0 - 1.0)
    pub min_confidence: f64,

    /// Maximum training samples to keep in memory
    pub max_training_samples: usize,

    /// Retrain model after N new samples
    pub retrain_threshold: usize,

    /// Enable auto-retraining
    pub auto_retrain: bool,

    /// Model storage path
    pub model_path: Option<String>,

    /// Feature extraction configuration
    pub feature_config: FeatureConfig,
}

impl Default for MLConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_severity_prediction: true,
            enable_type_prediction: true,
            enable_priority_prediction: true,
            min_confidence: 0.7,
            max_training_samples: 10000,
            retrain_threshold: 100,
            auto_retrain: true,
            model_path: Some("./data/models".to_string()),
            feature_config: FeatureConfig::default(),
        }
    }
}

/// Feature extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Maximum vocabulary size for text features
    pub max_vocab_size: usize,

    /// Minimum document frequency for terms
    pub min_doc_freq: usize,

    /// Use TF-IDF weighting
    pub use_tfidf: bool,

    /// Include temporal features
    pub include_temporal: bool,

    /// Include source features
    pub include_source: bool,

    /// Include historical features
    pub include_historical: bool,

    /// N-gram range (min, max)
    pub ngram_range: (usize, usize),
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            max_vocab_size: 1000,
            min_doc_freq: 2,
            use_tfidf: true,
            include_temporal: true,
            include_source: true,
            include_historical: true,
            ngram_range: (1, 2), // Unigrams and bigrams
        }
    }
}

/// Prediction result with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction<T> {
    /// Predicted value
    pub value: T,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,

    /// All class probabilities
    pub probabilities: HashMap<String, f64>,

    /// Feature importance (if available)
    pub feature_importance: Option<HashMap<String, f64>>,
}

impl<T> Prediction<T> {
    pub fn new(value: T, confidence: f64) -> Self {
        Self {
            value,
            confidence,
            probabilities: HashMap::new(),
            feature_importance: None,
        }
    }

    pub fn with_probabilities(mut self, probabilities: HashMap<String, f64>) -> Self {
        self.probabilities = probabilities;
        self
    }

    pub fn with_feature_importance(mut self, importance: HashMap<String, f64>) -> Self {
        self.feature_importance = Some(importance);
        self
    }
}

/// Training sample for ML models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    /// Feature vector
    pub features: Vec<f64>,

    /// Severity label
    pub severity: Option<Severity>,

    /// Type label
    pub incident_type: Option<IncidentType>,

    /// Priority score (0-10)
    pub priority: Option<f64>,

    /// Source system
    pub source: String,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Sample weight (for importance weighting)
    pub weight: f64,
}

impl TrainingSample {
    pub fn new(features: Vec<f64>, source: String) -> Self {
        Self {
            features,
            severity: None,
            incident_type: None,
            priority: None,
            source,
            timestamp: chrono::Utc::now(),
            weight: 1.0,
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn with_type(mut self, incident_type: IncidentType) -> Self {
        self.incident_type = Some(incident_type);
        self
    }

    pub fn with_priority(mut self, priority: f64) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Training dataset
#[derive(Debug, Clone)]
pub struct TrainingDataset {
    /// Feature matrix (n_samples × n_features)
    pub features: Array2<f64>,

    /// Severity labels
    pub severity_labels: Option<Vec<Severity>>,

    /// Type labels
    pub type_labels: Option<Vec<IncidentType>>,

    /// Priority scores
    pub priority_scores: Option<Array1<f64>>,

    /// Sample weights
    pub weights: Array1<f64>,

    /// Number of samples
    pub n_samples: usize,

    /// Number of features
    pub n_features: usize,
}

impl TrainingDataset {
    /// Create a new training dataset from samples
    pub fn from_samples(samples: &[TrainingSample]) -> Self {
        let n_samples = samples.len();
        let n_features = if n_samples > 0 {
            samples[0].features.len()
        } else {
            0
        };

        // Build feature matrix
        let mut features = Array2::zeros((n_samples, n_features));
        let mut weights = Array1::zeros(n_samples);
        let mut severity_labels = Vec::new();
        let mut type_labels = Vec::new();
        let mut priority_scores = Vec::new();

        for (i, sample) in samples.iter().enumerate() {
            for (j, &val) in sample.features.iter().enumerate() {
                features[[i, j]] = val;
            }
            weights[i] = sample.weight;

            if let Some(sev) = &sample.severity {
                severity_labels.push(sev.clone());
            }
            if let Some(typ) = &sample.incident_type {
                type_labels.push(typ.clone());
            }
            if let Some(pri) = sample.priority {
                priority_scores.push(pri);
            }
        }

        Self {
            features,
            severity_labels: if severity_labels.is_empty() {
                None
            } else {
                Some(severity_labels)
            },
            type_labels: if type_labels.is_empty() {
                None
            } else {
                Some(type_labels)
            },
            priority_scores: if priority_scores.is_empty() {
                None
            } else {
                Some(Array1::from_vec(priority_scores))
            },
            weights,
            n_samples,
            n_features,
        }
    }

    /// Split dataset into train/test sets
    pub fn train_test_split(&self, test_size: f64) -> (TrainingDataset, TrainingDataset) {
        let n_test = (self.n_samples as f64 * test_size) as usize;
        let n_train = self.n_samples - n_test;

        let train_features = self.features.slice(ndarray::s![..n_train, ..]).to_owned();
        let test_features = self.features.slice(ndarray::s![n_train.., ..]).to_owned();

        let train_weights = self.weights.slice(ndarray::s![..n_train]).to_owned();
        let test_weights = self.weights.slice(ndarray::s![n_train..]).to_owned();

        let train_severity = self
            .severity_labels
            .as_ref()
            .map(|labels| labels[..n_train].to_vec());
        let test_severity = self
            .severity_labels
            .as_ref()
            .map(|labels| labels[n_train..].to_vec());

        let train_type = self
            .type_labels
            .as_ref()
            .map(|labels| labels[..n_train].to_vec());
        let test_type = self
            .type_labels
            .as_ref()
            .map(|labels| labels[n_train..].to_vec());

        let train_priority = self
            .priority_scores
            .as_ref()
            .map(|scores| scores.slice(ndarray::s![..n_train]).to_owned());
        let test_priority = self
            .priority_scores
            .as_ref()
            .map(|scores| scores.slice(ndarray::s![n_train..]).to_owned());

        let train_dataset = TrainingDataset {
            features: train_features,
            severity_labels: train_severity,
            type_labels: train_type,
            priority_scores: train_priority,
            weights: train_weights,
            n_samples: n_train,
            n_features: self.n_features,
        };

        let test_dataset = TrainingDataset {
            features: test_features,
            severity_labels: test_severity,
            type_labels: test_type,
            priority_scores: test_priority,
            weights: test_weights,
            n_samples: n_test,
            n_features: self.n_features,
        };

        (train_dataset, test_dataset)
    }
}

/// Model evaluation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    /// Accuracy
    pub accuracy: f64,

    /// Precision
    pub precision: f64,

    /// Recall
    pub recall: f64,

    /// F1 score
    pub f1_score: f64,

    /// Confusion matrix
    pub confusion_matrix: Option<Array2<usize>>,

    /// Per-class metrics
    pub per_class_metrics: HashMap<String, ClassMetrics>,
}

/// Per-class evaluation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMetrics {
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub support: usize,
}

impl ModelMetrics {
    pub fn new() -> Self {
        Self {
            accuracy: 0.0,
            precision: 0.0,
            recall: 0.0,
            f1_score: 0.0,
            confusion_matrix: None,
            per_class_metrics: HashMap::new(),
        }
    }
}

impl Default for ModelMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model name
    pub name: String,

    /// Model version
    pub version: String,

    /// Model type
    pub model_type: ModelType,

    /// Training timestamp
    pub trained_at: chrono::DateTime<chrono::Utc>,

    /// Number of training samples
    pub n_training_samples: usize,

    /// Number of features
    pub n_features: usize,

    /// Training metrics
    pub training_metrics: ModelMetrics,

    /// Validation metrics
    pub validation_metrics: Option<ModelMetrics>,

    /// Hyperparameters
    pub hyperparameters: HashMap<String, String>,
}

/// Model type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    /// Logistic regression
    LogisticRegression,

    /// Random forest
    RandomForest,

    /// Support vector machine
    SVM,

    /// Neural network
    NeuralNetwork,

    /// Gradient boosting
    GradientBoosting,

    /// Naive Bayes
    NaiveBayes,

    /// K-Nearest Neighbors
    KNN,

    /// Ensemble (multiple models)
    Ensemble,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::LogisticRegression => write!(f, "Logistic Regression"),
            ModelType::RandomForest => write!(f, "Random Forest"),
            ModelType::SVM => write!(f, "Support Vector Machine"),
            ModelType::NeuralNetwork => write!(f, "Neural Network"),
            ModelType::GradientBoosting => write!(f, "Gradient Boosting"),
            ModelType::NaiveBayes => write!(f, "Naive Bayes"),
            ModelType::KNN => write!(f, "K-Nearest Neighbors"),
            ModelType::Ensemble => write!(f, "Ensemble"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_sample_creation() {
        let sample = TrainingSample::new(vec![1.0, 2.0, 3.0], "test".to_string())
            .with_severity(Severity::P1)
            .with_type(IncidentType::Infrastructure)
            .with_priority(8.5)
            .with_weight(1.5);

        assert_eq!(sample.features.len(), 3);
        assert_eq!(sample.severity, Some(Severity::P1));
        assert_eq!(sample.incident_type, Some(IncidentType::Infrastructure));
        assert_eq!(sample.priority, Some(8.5));
        assert_eq!(sample.weight, 1.5);
    }

    #[test]
    fn test_training_dataset_from_samples() {
        let samples = vec![
            TrainingSample::new(vec![1.0, 2.0], "test".to_string()).with_severity(Severity::P1),
            TrainingSample::new(vec![3.0, 4.0], "test".to_string()).with_severity(Severity::P2),
            TrainingSample::new(vec![5.0, 6.0], "test".to_string()).with_severity(Severity::P1),
        ];

        let dataset = TrainingDataset::from_samples(&samples);

        assert_eq!(dataset.n_samples, 3);
        assert_eq!(dataset.n_features, 2);
        assert_eq!(dataset.features.shape(), &[3, 2]);
        assert!(dataset.severity_labels.is_some());
        assert_eq!(dataset.severity_labels.unwrap().len(), 3);
    }

    #[test]
    fn test_train_test_split() {
        let samples: Vec<TrainingSample> = (0..100)
            .map(|i| {
                TrainingSample::new(vec![i as f64, (i * 2) as f64], "test".to_string())
                    .with_severity(if i % 2 == 0 { Severity::P1 } else { Severity::P2 })
            })
            .collect();

        let dataset = TrainingDataset::from_samples(&samples);
        let (train, test) = dataset.train_test_split(0.2);

        assert_eq!(train.n_samples, 80);
        assert_eq!(test.n_samples, 20);
        assert_eq!(train.n_features, 2);
        assert_eq!(test.n_features, 2);
    }

    #[test]
    fn test_prediction_creation() {
        let prediction = Prediction::new(Severity::P1, 0.85)
            .with_probabilities(
                vec![
                    ("P0".to_string(), 0.05),
                    ("P1".to_string(), 0.85),
                    ("P2".to_string(), 0.10),
                ]
                .into_iter()
                .collect(),
            );

        assert_eq!(prediction.value, Severity::P1);
        assert_eq!(prediction.confidence, 0.85);
        assert_eq!(prediction.probabilities.len(), 3);
    }

    #[test]
    fn test_ml_config_default() {
        let config = MLConfig::default();
        assert!(config.enabled);
        assert!(config.enable_severity_prediction);
        assert!(config.auto_retrain);
        assert_eq!(config.min_confidence, 0.7);
    }

    #[test]
    fn test_model_type_display() {
        assert_eq!(ModelType::LogisticRegression.to_string(), "Logistic Regression");
        assert_eq!(ModelType::RandomForest.to_string(), "Random Forest");
        assert_eq!(ModelType::SVM.to_string(), "Support Vector Machine");
    }
}
