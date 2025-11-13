use crate::error::{AppError, Result};
use crate::ml::models::{ModelMetadata, ModelMetrics, ModelType, Prediction, TrainingDataset};
use crate::models::{IncidentType, Severity};
use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::linear::logistic_regression::{LogisticRegression, LogisticRegressionParameters};
use smartcore::naive_bayes::gaussian::GaussianNB;
use smartcore::tree::decision_tree_classifier::{
    DecisionTreeClassifier, DecisionTreeClassifierParameters, SplitCriterion,
};
use std::collections::HashMap;

/// Trait for classifiers
pub trait Classifier: Send + Sync {
    /// Train the classifier
    fn train(&mut self, dataset: &TrainingDataset) -> Result<ModelMetrics>;

    /// Predict class labels
    fn predict(&self, features: &Array2<f64>) -> Result<Vec<usize>>;

    /// Predict class probabilities
    fn predict_proba(&self, features: &Array2<f64>) -> Result<Array2<f64>>;

    /// Get model metadata
    fn metadata(&self) -> &ModelMetadata;

    /// Get model type
    fn model_type(&self) -> ModelType;

    /// Check if model is trained
    fn is_trained(&self) -> bool;
}

/// Logistic Regression Classifier
#[derive(Serialize, Deserialize)]
pub struct LogisticRegressionClassifier {
    /// Model metadata
    metadata: ModelMetadata,

    /// Trained model (serialized as weights and bias)
    #[serde(skip)]
    model: Option<LogisticRegression<f64, i32, DenseMatrix<f64>, Vec<i32>>>,

    /// Model weights (for serialization)
    weights: Option<Array2<f64>>,

    /// Model bias (for serialization)
    bias: Option<Array1<f64>>,

    /// Number of classes
    n_classes: usize,

    /// Is trained
    trained: bool,
}

impl LogisticRegressionClassifier {
    pub fn new(n_classes: usize) -> Self {
        Self {
            metadata: ModelMetadata {
                name: "Logistic Regression".to_string(),
                version: "1.0".to_string(),
                model_type: ModelType::LogisticRegression,
                trained_at: chrono::Utc::now(),
                n_training_samples: 0,
                n_features: 0,
                training_metrics: ModelMetrics::new(),
                validation_metrics: None,
                hyperparameters: HashMap::new(),
            },
            model: None,
            weights: None,
            bias: None,
            n_classes,
            trained: false,
        }
    }

    fn ndarray_to_densematrix(arr: &Array2<f64>) -> DenseMatrix<f64> {
        let shape = arr.shape();
        let data: Vec<f64> = arr.iter().copied().collect();
        DenseMatrix::new(shape[0], shape[1], data, false)
    }

    fn vec_to_labels(vec: &[usize]) -> Vec<i32> {
        vec.iter().map(|&x| x as i32).collect()
    }
}

impl Classifier for LogisticRegressionClassifier {
    fn train(&mut self, dataset: &TrainingDataset) -> Result<ModelMetrics> {
        // Convert labels to indices
        let labels: Vec<usize> = if let Some(ref severity_labels) = dataset.severity_labels {
            severity_labels
                .iter()
                .map(|s| Self::severity_to_index(s))
                .collect()
        } else {
            return Err(AppError::Internal(
                "No severity labels provided for training".to_string(),
            ));
        };

        // Convert to smartcore format
        let x = Self::ndarray_to_densematrix(&dataset.features);
        let y = Self::vec_to_labels(&labels);

        // Train model
        let params = LogisticRegressionParameters::default();
        let model = LogisticRegression::fit(&x, &y, params).map_err(|e| {
            AppError::Internal(format!("Failed to train logistic regression: {}", e))
        })?;

        self.model = Some(model);
        self.trained = true;

        // Calculate metrics
        let predictions = self.predict(&dataset.features)?;
        let metrics = Self::calculate_metrics(&labels, &predictions, self.n_classes);

        self.metadata.n_training_samples = dataset.n_samples;
        self.metadata.n_features = dataset.n_features;
        self.metadata.trained_at = chrono::Utc::now();
        self.metadata.training_metrics = metrics.clone();

        Ok(metrics)
    }

    fn predict(&self, features: &Array2<f64>) -> Result<Vec<usize>> {
        if !self.trained {
            return Err(AppError::Internal("Model not trained".to_string()));
        }

        let x = Self::ndarray_to_densematrix(features);
        let predictions = self
            .model
            .as_ref()
            .unwrap()
            .predict(&x)
            .map_err(|e| AppError::Internal(format!("Prediction failed: {}", e)))?;

        Ok(predictions.iter().map(|&x| x as usize).collect())
    }

    fn predict_proba(&self, features: &Array2<f64>) -> Result<Array2<f64>> {
        // Simplified: return one-hot encoded predictions
        let predictions = self.predict(features)?;
        let n_samples = predictions.len();
        let mut proba = Array2::zeros((n_samples, self.n_classes));

        for (i, &pred) in predictions.iter().enumerate() {
            if pred < self.n_classes {
                proba[[i, pred]] = 1.0;
            }
        }

        Ok(proba)
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    fn model_type(&self) -> ModelType {
        ModelType::LogisticRegression
    }

    fn is_trained(&self) -> bool {
        self.trained
    }
}

impl LogisticRegressionClassifier {
    fn severity_to_index(severity: &Severity) -> usize {
        match severity {
            Severity::P0 => 0,
            Severity::P1 => 1,
            Severity::P2 => 2,
            Severity::P3 => 3,
            Severity::P4 => 4,
        }
    }

    fn index_to_severity(index: usize) -> Severity {
        match index {
            0 => Severity::P0,
            1 => Severity::P1,
            2 => Severity::P2,
            3 => Severity::P3,
            4 => Severity::P4,
            _ => Severity::P4,
        }
    }

    fn calculate_metrics(
        y_true: &[usize],
        y_pred: &[usize],
        n_classes: usize,
    ) -> ModelMetrics {
        let n_samples = y_true.len();
        if n_samples == 0 {
            return ModelMetrics::new();
        }

        // Calculate accuracy
        let correct = y_true
            .iter()
            .zip(y_pred.iter())
            .filter(|(t, p)| t == p)
            .count();
        let accuracy = correct as f64 / n_samples as f64;

        // Calculate per-class metrics
        let mut per_class = HashMap::new();

        for class_idx in 0..n_classes {
            let tp = y_true
                .iter()
                .zip(y_pred.iter())
                .filter(|(t, p)| **t == class_idx && **p == class_idx)
                .count();

            let fp = y_pred
                .iter()
                .zip(y_true.iter())
                .filter(|(p, t)| **p == class_idx && **t != class_idx)
                .count();

            let fn_count = y_true
                .iter()
                .zip(y_pred.iter())
                .filter(|(t, p)| **t == class_idx && **p != class_idx)
                .count();

            let precision = if tp + fp > 0 {
                tp as f64 / (tp + fp) as f64
            } else {
                0.0
            };

            let recall = if tp + fn_count > 0 {
                tp as f64 / (tp + fn_count) as f64
            } else {
                0.0
            };

            let f1 = if precision + recall > 0.0 {
                2.0 * precision * recall / (precision + recall)
            } else {
                0.0
            };

            let support = y_true.iter().filter(|&&t| t == class_idx).count();

            per_class.insert(
                format!("class_{}", class_idx),
                crate::ml::models::ClassMetrics {
                    precision,
                    recall,
                    f1_score: f1,
                    support,
                },
            );
        }

        // Calculate macro-averaged metrics
        let avg_precision: f64 = per_class.values().map(|m| m.precision).sum::<f64>()
            / n_classes as f64;
        let avg_recall: f64 =
            per_class.values().map(|m| m.recall).sum::<f64>() / n_classes as f64;
        let avg_f1: f64 =
            per_class.values().map(|m| m.f1_score).sum::<f64>() / n_classes as f64;

        ModelMetrics {
            accuracy,
            precision: avg_precision,
            recall: avg_recall,
            f1_score: avg_f1,
            confusion_matrix: None,
            per_class_metrics: per_class,
        }
    }
}

/// Decision Tree Classifier
#[derive(Serialize, Deserialize)]
pub struct DecisionTreeClassifierWrapper {
    /// Model metadata
    metadata: ModelMetadata,

    /// Trained model (skip serialization)
    #[serde(skip)]
    model: Option<DecisionTreeClassifier<f64, i32, DenseMatrix<f64>, Vec<i32>>>,

    /// Number of classes
    n_classes: usize,

    /// Maximum depth
    max_depth: usize,

    /// Is trained
    trained: bool,
}

impl DecisionTreeClassifierWrapper {
    pub fn new(n_classes: usize, max_depth: usize) -> Self {
        Self {
            metadata: ModelMetadata {
                name: "Decision Tree".to_string(),
                version: "1.0".to_string(),
                model_type: ModelType::RandomForest,
                trained_at: chrono::Utc::now(),
                n_training_samples: 0,
                n_features: 0,
                training_metrics: ModelMetrics::new(),
                validation_metrics: None,
                hyperparameters: [("max_depth".to_string(), max_depth.to_string())]
                    .iter()
                    .cloned()
                    .collect(),
            },
            model: None,
            n_classes,
            max_depth,
            trained: false,
        }
    }
}

impl Classifier for DecisionTreeClassifierWrapper {
    fn train(&mut self, dataset: &TrainingDataset) -> Result<ModelMetrics> {
        let labels: Vec<usize> = if let Some(ref severity_labels) = dataset.severity_labels {
            severity_labels
                .iter()
                .map(|s| LogisticRegressionClassifier::severity_to_index(s))
                .collect()
        } else {
            return Err(AppError::Internal(
                "No severity labels provided for training".to_string(),
            ));
        };

        let x = LogisticRegressionClassifier::ndarray_to_densematrix(&dataset.features);
        let y = LogisticRegressionClassifier::vec_to_labels(&labels);

        let params = DecisionTreeClassifierParameters::default()
            .with_max_depth(self.max_depth as u16)
            .with_criterion(SplitCriterion::Gini);

        let model = DecisionTreeClassifier::fit(&x, &y, params)
            .map_err(|e| AppError::Internal(format!("Failed to train decision tree: {}", e)))?;

        self.model = Some(model);
        self.trained = true;

        let predictions = self.predict(&dataset.features)?;
        let metrics =
            LogisticRegressionClassifier::calculate_metrics(&labels, &predictions, self.n_classes);

        self.metadata.n_training_samples = dataset.n_samples;
        self.metadata.n_features = dataset.n_features;
        self.metadata.trained_at = chrono::Utc::now();
        self.metadata.training_metrics = metrics.clone();

        Ok(metrics)
    }

    fn predict(&self, features: &Array2<f64>) -> Result<Vec<usize>> {
        if !self.trained {
            return Err(AppError::Internal("Model not trained".to_string()));
        }

        let x = LogisticRegressionClassifier::ndarray_to_densematrix(features);
        let predictions = self
            .model
            .as_ref()
            .unwrap()
            .predict(&x)
            .map_err(|e| AppError::Internal(format!("Prediction failed: {}", e)))?;

        Ok(predictions.iter().map(|&x| x as usize).collect())
    }

    fn predict_proba(&self, features: &Array2<f64>) -> Result<Array2<f64>> {
        let predictions = self.predict(features)?;
        let n_samples = predictions.len();
        let mut proba = Array2::zeros((n_samples, self.n_classes));

        for (i, &pred) in predictions.iter().enumerate() {
            if pred < self.n_classes {
                proba[[i, pred]] = 1.0;
            }
        }

        Ok(proba)
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    fn model_type(&self) -> ModelType {
        ModelType::RandomForest
    }

    fn is_trained(&self) -> bool {
        self.trained
    }
}

/// Naive Bayes Classifier
#[derive(Serialize, Deserialize)]
pub struct NaiveBayesClassifier {
    /// Model metadata
    metadata: ModelMetadata,

    /// Trained model
    #[serde(skip)]
    model: Option<GaussianNB<f64, usize, DenseMatrix<f64>, Vec<usize>>>,

    /// Number of classes
    n_classes: usize,

    /// Is trained
    trained: bool,
}

impl NaiveBayesClassifier {
    pub fn new(n_classes: usize) -> Self {
        Self {
            metadata: ModelMetadata {
                name: "Naive Bayes".to_string(),
                version: "1.0".to_string(),
                model_type: ModelType::NaiveBayes,
                trained_at: chrono::Utc::now(),
                n_training_samples: 0,
                n_features: 0,
                training_metrics: ModelMetrics::new(),
                validation_metrics: None,
                hyperparameters: HashMap::new(),
            },
            model: None,
            n_classes,
            trained: false,
        }
    }
}

impl Classifier for NaiveBayesClassifier {
    fn train(&mut self, dataset: &TrainingDataset) -> Result<ModelMetrics> {
        let labels: Vec<usize> = if let Some(ref severity_labels) = dataset.severity_labels {
            severity_labels
                .iter()
                .map(|s| LogisticRegressionClassifier::severity_to_index(s))
                .collect()
        } else {
            return Err(AppError::Internal(
                "No severity labels provided for training".to_string(),
            ));
        };

        let x = LogisticRegressionClassifier::ndarray_to_densematrix(&dataset.features);
        let y = labels.clone(); // Use labels directly as usize

        let model = GaussianNB::fit(&x, &y, Default::default())
            .map_err(|e| AppError::Internal(format!("Failed to train Naive Bayes: {}", e)))?;

        self.model = Some(model);
        self.trained = true;

        let predictions = self.predict(&dataset.features)?;
        let metrics =
            LogisticRegressionClassifier::calculate_metrics(&labels, &predictions, self.n_classes);

        self.metadata.n_training_samples = dataset.n_samples;
        self.metadata.n_features = dataset.n_features;
        self.metadata.trained_at = chrono::Utc::now();
        self.metadata.training_metrics = metrics.clone();

        Ok(metrics)
    }

    fn predict(&self, features: &Array2<f64>) -> Result<Vec<usize>> {
        if !self.trained {
            return Err(AppError::Internal("Model not trained".to_string()));
        }

        let x = LogisticRegressionClassifier::ndarray_to_densematrix(features);
        let predictions = self
            .model
            .as_ref()
            .unwrap()
            .predict(&x)
            .map_err(|e| AppError::Internal(format!("Prediction failed: {}", e)))?;

        Ok(predictions) // Already Vec<usize>, no conversion needed
    }

    fn predict_proba(&self, features: &Array2<f64>) -> Result<Array2<f64>> {
        let predictions = self.predict(features)?;
        let n_samples = predictions.len();
        let mut proba = Array2::zeros((n_samples, self.n_classes));

        for (i, &pred) in predictions.iter().enumerate() {
            if pred < self.n_classes {
                proba[[i, pred]] = 1.0;
            }
        }

        Ok(proba)
    }

    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    fn model_type(&self) -> ModelType {
        ModelType::NaiveBayes
    }

    fn is_trained(&self) -> bool {
        self.trained
    }
}

/// Severity classifier using ensemble of models
pub struct SeverityClassifier {
    /// Primary model
    primary_model: Box<dyn Classifier>,

    /// Model type
    model_type: ModelType,
}

impl SeverityClassifier {
    /// Create a new severity classifier with specified model type
    pub fn new(model_type: ModelType) -> Self {
        let primary_model: Box<dyn Classifier> = match model_type {
            ModelType::LogisticRegression => Box::new(LogisticRegressionClassifier::new(5)),
            ModelType::RandomForest => Box::new(DecisionTreeClassifierWrapper::new(5, 10)),
            ModelType::NaiveBayes => Box::new(NaiveBayesClassifier::new(5)),
            _ => Box::new(LogisticRegressionClassifier::new(5)), // Default
        };

        Self {
            primary_model,
            model_type,
        }
    }

    /// Train the classifier
    pub fn train(&mut self, dataset: &TrainingDataset) -> Result<ModelMetrics> {
        self.primary_model.train(dataset)
    }

    /// Predict severity for a single incident
    pub fn predict_severity(&self, features: &[f64]) -> Result<Prediction<Severity>> {
        let features_array = Array2::from_shape_vec((1, features.len()), features.to_vec())
            .map_err(|e| AppError::Internal(format!("Failed to create feature array: {}", e)))?;

        let predictions = self.primary_model.predict(&features_array)?;
        let proba = self.primary_model.predict_proba(&features_array)?;

        let pred_idx = predictions[0];
        let severity = LogisticRegressionClassifier::index_to_severity(pred_idx);
        let confidence = proba[[0, pred_idx]];

        let probabilities: HashMap<String, f64> = (0..5)
            .map(|i| {
                (
                    format!("{:?}", LogisticRegressionClassifier::index_to_severity(i)),
                    proba[[0, i]],
                )
            })
            .collect();

        Ok(Prediction::new(severity, confidence).with_probabilities(probabilities))
    }

    /// Check if model is trained
    pub fn is_trained(&self) -> bool {
        self.primary_model.is_trained()
    }

    /// Get model metadata
    pub fn metadata(&self) -> &ModelMetadata {
        self.primary_model.metadata()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::models::TrainingSample;

    fn create_test_dataset(n_samples: usize) -> TrainingDataset {
        let samples: Vec<TrainingSample> = (0..n_samples)
            .map(|i| {
                let features = vec![i as f64, (i * 2) as f64, (i % 3) as f64];
                let severity = match i % 5 {
                    0 => Severity::P0,
                    1 => Severity::P1,
                    2 => Severity::P2,
                    3 => Severity::P3,
                    _ => Severity::P4,
                };
                TrainingSample::new(features, "test".to_string()).with_severity(severity)
            })
            .collect();

        TrainingDataset::from_samples(&samples)
    }

    #[test]
    fn test_logistic_regression_classifier() {
        let dataset = create_test_dataset(100);
        let mut classifier = LogisticRegressionClassifier::new(5);

        assert!(!classifier.is_trained());

        let metrics = classifier.train(&dataset).unwrap();

        assert!(classifier.is_trained());
        assert!(metrics.accuracy >= 0.0 && metrics.accuracy <= 1.0);
    }

    #[test]
    fn test_decision_tree_classifier() {
        let dataset = create_test_dataset(50);
        let mut classifier = DecisionTreeClassifierWrapper::new(5, 5);

        let metrics = classifier.train(&dataset).unwrap();

        assert!(classifier.is_trained());
        assert!(metrics.accuracy >= 0.0);
    }

    #[test]
    fn test_naive_bayes_classifier() {
        let dataset = create_test_dataset(50);
        let mut classifier = NaiveBayesClassifier::new(5);

        let metrics = classifier.train(&dataset).unwrap();

        assert!(classifier.is_trained());
        assert!(metrics.accuracy >= 0.0);
    }

    #[test]
    fn test_severity_classifier() {
        let dataset = create_test_dataset(100);
        let mut classifier = SeverityClassifier::new(ModelType::LogisticRegression);

        classifier.train(&dataset).unwrap();

        let features = vec![50.0, 100.0, 2.0];
        let prediction = classifier.predict_severity(&features).unwrap();

        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
        assert!(prediction.probabilities.len() > 0);
    }
}
