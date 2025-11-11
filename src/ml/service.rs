use crate::error::{AppError, Result};
use crate::ml::classifier::SeverityClassifier;
use crate::ml::features::FeatureExtractor;
use crate::ml::models::{
    FeatureConfig, MLConfig, ModelMetadata, ModelType, Prediction, TrainingDataset,
    TrainingSample,
};
use crate::models::{Incident, IncidentType, Severity};
use crate::state::IncidentStore;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// ML prediction service
pub struct MLService {
    /// Configuration
    config: Arc<RwLock<MLConfig>>,

    /// Feature extractor
    feature_extractor: Arc<RwLock<FeatureExtractor>>,

    /// Severity classifier
    severity_classifier: Arc<RwLock<Option<SeverityClassifier>>>,

    /// Training samples cache
    training_samples: Arc<DashMap<uuid::Uuid, TrainingSample>>,

    /// Incident store for fetching historical data
    incident_store: Arc<dyn IncidentStore>,

    /// Number of samples since last training
    samples_since_training: Arc<RwLock<usize>>,

    /// Service running state
    running: Arc<RwLock<bool>>,
}

impl MLService {
    /// Create a new ML service
    pub fn new(config: MLConfig, incident_store: Arc<dyn IncidentStore>) -> Self {
        let feature_extractor = FeatureExtractor::new(config.feature_config.clone());

        Self {
            config: Arc::new(RwLock::new(config)),
            feature_extractor: Arc::new(RwLock::new(feature_extractor)),
            severity_classifier: Arc::new(RwLock::new(None)),
            training_samples: Arc::new(DashMap::new()),
            incident_store,
            samples_since_training: Arc::new(RwLock::new(0)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the ML service
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(AppError::Internal("ML service already running".to_string()));
        }
        *running = true;
        drop(running);

        info!("🚀 Starting ML service");

        // Load or train initial models
        if let Err(e) = self.initialize_models().await {
            error!("Failed to initialize ML models: {}", e);
            warn!("ML service will continue without pre-trained models");
        }

        Ok(())
    }

    /// Stop the ML service
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        info!("🛑 Stopping ML service");
        Ok(())
    }

    /// Check if service is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Initialize models (load or train)
    async fn initialize_models(&self) -> Result<()> {
        let config = self.config.read().await;

        if !config.enabled {
            info!("ML service is disabled in configuration");
            return Ok(());
        }

        // Try to load historical incidents for training
        let filter = crate::state::IncidentFilter::default();
        let incidents = self.incident_store.list_incidents(&filter, 0, 1000).await?;

        if incidents.is_empty() {
            info!("No historical incidents found for training");
            return Ok(());
        }

        info!("Found {} historical incidents for training", incidents.len());

        // Train initial models
        self.train_on_incidents(&incidents).await?;

        Ok(())
    }

    /// Train models on a set of incidents
    pub async fn train_on_incidents(&self, incidents: &[Incident]) -> Result<()> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(());
        }

        if incidents.len() < 10 {
            warn!("Too few incidents ({}) for effective training", incidents.len());
            return Ok(());
        }

        info!("Training ML models on {} incidents", incidents.len());

        // Fit feature extractor
        let mut extractor = self.feature_extractor.write().await;
        extractor.fit(incidents)?;
        drop(extractor);

        // Extract features for all incidents
        let extractor = self.feature_extractor.read().await;
        let samples: Result<Vec<TrainingSample>> = incidents
            .iter()
            .map(|inc| {
                let features = extractor.transform(inc)?;
                let sample = TrainingSample::new(features, inc.source.clone())
                    .with_severity(inc.severity.clone())
                    .with_type(inc.incident_type.clone());
                Ok(sample)
            })
            .collect();

        let samples = samples?;
        drop(extractor);

        // Create training dataset
        let dataset = TrainingDataset::from_samples(&samples);

        // Train severity classifier
        if config.enable_severity_prediction {
            info!("Training severity classifier");
            let mut classifier =
                SeverityClassifier::new(ModelType::LogisticRegression);

            match classifier.train(&dataset) {
                Ok(metrics) => {
                    info!(
                        "Severity classifier trained successfully - Accuracy: {:.2}%",
                        metrics.accuracy * 100.0
                    );
                    *self.severity_classifier.write().await = Some(classifier);
                }
                Err(e) => {
                    error!("Failed to train severity classifier: {}", e);
                }
            }
        }

        // Reset counter
        *self.samples_since_training.write().await = 0;

        info!("✅ ML models training completed");

        Ok(())
    }

    /// Add an incident to the training set
    pub async fn add_training_sample(&self, incident: &Incident) -> Result<()> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(());
        }

        // Check if feature extractor is fitted
        let extractor = self.feature_extractor.read().await;
        if !extractor.is_fitted() {
            drop(extractor);
            debug!("Feature extractor not fitted, skipping training sample");
            return Ok(());
        }

        // Extract features
        let features = extractor.transform(incident)?;
        drop(extractor);

        // Create training sample
        let sample = TrainingSample::new(features, incident.source.clone())
            .with_severity(incident.severity.clone())
            .with_type(incident.incident_type.clone());

        // Add to cache
        self.training_samples.insert(incident.id, sample);

        // Check if we need to retrain
        let mut samples_count = self.samples_since_training.write().await;
        *samples_count += 1;

        let should_retrain = config.auto_retrain && *samples_count >= config.retrain_threshold;
        drop(samples_count);
        drop(config);

        if should_retrain {
            info!("Retraining threshold reached, triggering model retraining");
            self.retrain_models().await?;
        }

        Ok(())
    }

    /// Retrain models with accumulated samples
    async fn retrain_models(&self) -> Result<()> {
        // Fetch recent incidents
        let filter = crate::state::IncidentFilter::default();
        let incidents = self
            .incident_store
            .list_incidents(&filter, 0, 1000)
            .await?;

        if incidents.len() >= 10 {
            self.train_on_incidents(&incidents).await?;
        }

        Ok(())
    }

    /// Predict severity for an incident
    pub async fn predict_severity(&self, incident: &Incident) -> Result<Prediction<Severity>> {
        let config = self.config.read().await;

        if !config.enabled || !config.enable_severity_prediction {
            return Err(AppError::Internal(
                "Severity prediction is disabled".to_string(),
            ));
        }

        drop(config);

        // Check if classifier is trained
        let classifier_guard = self.severity_classifier.read().await;
        let classifier = classifier_guard
            .as_ref()
            .ok_or_else(|| AppError::Internal("Severity classifier not trained".to_string()))?;

        // Extract features
        let extractor = self.feature_extractor.read().await;
        let features = extractor.transform(incident)?;
        drop(extractor);

        // Make prediction
        let prediction = classifier.predict_severity(&features)?;

        // Check confidence threshold
        let config = self.config.read().await;
        if prediction.confidence < config.min_confidence {
            debug!(
                "Prediction confidence {:.2} below threshold {:.2}",
                prediction.confidence, config.min_confidence
            );
        }

        Ok(prediction)
    }

    /// Predict type for an incident (simplified - returns current type with confidence)
    pub async fn predict_type(&self, incident: &Incident) -> Result<Prediction<IncidentType>> {
        // Simplified implementation: return current type with high confidence
        // In production, this would use a separate classifier
        Ok(Prediction::new(incident.incident_type.clone(), 0.95))
    }

    /// Predict priority score for an incident (0-10)
    pub async fn predict_priority(&self, incident: &Incident) -> Result<Prediction<f64>> {
        // Simplified: derive priority from severity
        let severity_pred = self.predict_severity(incident).await?;

        let priority = match severity_pred.value {
            Severity::P0 => 10.0,
            Severity::P1 => 8.0,
            Severity::P2 => 6.0,
            Severity::P3 => 4.0,
            Severity::P4 => 2.0,
        };

        Ok(Prediction::new(priority, severity_pred.confidence))
    }

    /// Get prediction for all aspects of an incident
    pub async fn predict_all(&self, incident: &Incident) -> Result<IncidentPredictions> {
        let severity = self.predict_severity(incident).await.ok();
        let incident_type = self.predict_type(incident).await.ok();
        let priority = self.predict_priority(incident).await.ok();

        Ok(IncidentPredictions {
            severity,
            incident_type,
            priority,
        })
    }

    /// Get model metadata
    pub async fn get_model_metadata(&self) -> Result<Vec<ModelMetadata>> {
        let mut metadata = Vec::new();

        let classifier_guard = self.severity_classifier.read().await;
        if let Some(classifier) = classifier_guard.as_ref() {
            metadata.push(classifier.metadata().clone());
        }

        Ok(metadata)
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> MLServiceStats {
        let config = self.config.read().await;
        let extractor = self.feature_extractor.read().await;
        let samples_count = self.samples_since_training.read().await;

        MLServiceStats {
            enabled: config.enabled,
            is_trained: extractor.is_fitted(),
            n_training_samples: self.training_samples.len(),
            n_features: extractor.n_features(),
            vocab_size: extractor.vocab_size(),
            samples_since_training: *samples_count,
            retrain_threshold: config.retrain_threshold,
        }
    }

    /// Force model retraining
    pub async fn force_retrain(&self) -> Result<()> {
        info!("Forcing model retraining");
        self.retrain_models().await
    }

    /// Clear training samples cache
    pub async fn clear_training_cache(&self) {
        self.training_samples.clear();
        *self.samples_since_training.write().await = 0;
        info!("Training cache cleared");
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: MLConfig) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("ML service configuration updated");
        Ok(())
    }
}

/// Combined predictions for an incident
#[derive(Debug, Clone)]
pub struct IncidentPredictions {
    pub severity: Option<Prediction<Severity>>,
    pub incident_type: Option<Prediction<IncidentType>>,
    pub priority: Option<Prediction<f64>>,
}

/// ML service statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MLServiceStats {
    pub enabled: bool,
    pub is_trained: bool,
    pub n_training_samples: usize,
    pub n_features: usize,
    pub vocab_size: usize,
    pub samples_since_training: usize,
    pub retrain_threshold: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::InMemoryStore;

    fn create_test_incident(title: &str, description: &str, severity: Severity) -> Incident {
        Incident::new(
            "test".to_string(),
            title.to_string(),
            description.to_string(),
            severity,
            IncidentType::Infrastructure,
        )
    }

    #[tokio::test]
    async fn test_ml_service_creation() {
        let store = Arc::new(InMemoryStore::new());
        let config = MLConfig::default();
        let service = MLService::new(config, store);

        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_ml_service_start_stop() {
        let store = Arc::new(InMemoryStore::new());
        let config = MLConfig::default();
        let service = MLService::new(config, store);

        service.start().await.unwrap();
        assert!(service.is_running().await);

        service.stop().await.unwrap();
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_train_on_incidents() {
        let store = Arc::new(InMemoryStore::new());
        let config = MLConfig::default();
        let service = MLService::new(config, store);

        // Create test incidents
        let incidents: Vec<Incident> = (0..50)
            .map(|i| {
                create_test_incident(
                    &format!("Incident {}", i),
                    &format!("Description {}", i),
                    if i % 2 == 0 { Severity::P1 } else { Severity::P2 },
                )
            })
            .collect();

        // Train
        let result = service.train_on_incidents(&incidents).await;
        assert!(result.is_ok());

        // Check stats
        let stats = service.get_stats().await;
        assert!(stats.is_trained);
        assert!(stats.vocab_size > 0);
    }

    #[tokio::test]
    async fn test_predict_severity() {
        let store = Arc::new(InMemoryStore::new());
        let config = MLConfig::default();
        let service = MLService::new(config, store);

        // Train first
        let incidents: Vec<Incident> = (0..50)
            .map(|i| {
                create_test_incident(
                    &format!("Database error {}", i),
                    &format!("Connection timeout {}", i),
                    if i % 2 == 0 { Severity::P0 } else { Severity::P1 },
                )
            })
            .collect();

        service.train_on_incidents(&incidents).await.unwrap();

        // Make prediction
        let test_incident =
            create_test_incident("Database error", "Connection timeout", Severity::P0);

        let prediction = service.predict_severity(&test_incident).await.unwrap();

        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
        assert!(!prediction.probabilities.is_empty());
    }

    #[tokio::test]
    async fn test_add_training_sample() {
        let store = Arc::new(InMemoryStore::new());
        let mut config = MLConfig::default();
        config.retrain_threshold = 100; // High threshold to avoid auto-retraining in test
        let service = MLService::new(config, store);

        // Train first to fit feature extractor
        let incidents: Vec<Incident> = (0..20)
            .map(|i| create_test_incident(&format!("Test {}", i), "Description", Severity::P1))
            .collect();

        service.train_on_incidents(&incidents).await.unwrap();

        // Add sample
        let new_incident = create_test_incident("New incident", "New description", Severity::P2);

        let result = service.add_training_sample(&new_incident).await;
        assert!(result.is_ok());

        let stats = service.get_stats().await;
        assert!(stats.samples_since_training > 0);
    }

    #[tokio::test]
    async fn test_predict_all() {
        let store = Arc::new(InMemoryStore::new());
        let config = MLConfig::default();
        let service = MLService::new(config, store);

        // Train
        let incidents: Vec<Incident> = (0..30)
            .map(|i| create_test_incident(&format!("Test {}", i), "Description", Severity::P1))
            .collect();

        service.train_on_incidents(&incidents).await.unwrap();

        // Predict all
        let test_incident = create_test_incident("Test", "Description", Severity::P1);
        let predictions = service.predict_all(&test_incident).await.unwrap();

        assert!(predictions.severity.is_some());
        assert!(predictions.incident_type.is_some());
        assert!(predictions.priority.is_some());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let store = Arc::new(InMemoryStore::new());
        let config = MLConfig::default();
        let service = MLService::new(config, store);

        let stats = service.get_stats().await;

        assert!(stats.enabled);
        assert_eq!(stats.n_training_samples, 0);
    }

    #[tokio::test]
    async fn test_clear_training_cache() {
        let store = Arc::new(InMemoryStore::new());
        let config = MLConfig::default();
        let service = MLService::new(config, store);

        service.clear_training_cache().await;

        let stats = service.get_stats().await;
        assert_eq!(stats.n_training_samples, 0);
        assert_eq!(stats.samples_since_training, 0);
    }
}
