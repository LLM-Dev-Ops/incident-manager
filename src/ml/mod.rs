/// Machine Learning module for incident classification and prediction
///
/// This module provides ML-based classification capabilities including:
/// - Automatic severity prediction
/// - Incident type classification
/// - Priority scoring
/// - Feature extraction from incident text
/// - Model training and retraining
/// - Multiple ML algorithms (Logistic Regression, Decision Trees, Naive Bayes)

pub mod classifier;
pub mod features;
pub mod models;
pub mod service;

pub use classifier::{Classifier, LogisticRegressionClassifier, SeverityClassifier};
pub use features::{FeatureExtractor, TextPreprocessor};
pub use models::{
    FeatureConfig, MLConfig, ModelMetadata, ModelMetrics, ModelType, Prediction, TrainingDataset,
    TrainingSample,
};
pub use service::{IncidentPredictions, MLService, MLServiceStats};
