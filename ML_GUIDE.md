# ML Classification User Guide

## Overview

The ML Classification system provides intelligent, automated classification and prediction capabilities for incidents using machine learning algorithms. The system learns from historical incident data to make predictions about new incidents, improving accuracy over time.

## Key Features

- **Automatic Severity Classification**: Predicts incident severity (P0-P4) based on title, description, and context
- **Incident Type Classification**: Classifies incidents into categories (Infrastructure, Application, Security, Performance)
- **Priority Scoring**: Calculates priority scores (0-10) for incident triage
- **Continuous Learning**: Automatically retrains models as new incidents are processed
- **Multiple ML Algorithms**: Supports Logistic Regression, Decision Trees, Naive Bayes
- **Feature Engineering**: Extracts relevant features from incident text and metadata
- **Confidence Scores**: Provides confidence levels for predictions
- **Production-Ready**: Thread-safe, fault-tolerant, and performant

## How It Works

### 1. Feature Extraction

The system extracts features from incidents:

**Text Features**:
- TF-IDF (Term Frequency-Inverse Document Frequency) from title and description
- N-grams (unigrams and bigrams)
- Keyword extraction
- Text normalization

**Categorical Features**:
- Source system (one-hot encoded)
- Incident type
- Historical patterns

**Temporal Features**:
- Hour of day (0-23)
- Day of week
- Weekend vs. weekday
- Business hours vs. off-hours

### 2. Model Training

Models are trained on historical incident data:

1. **Data Collection**: Gather historical incidents from storage
2. **Feature Extraction**: Convert incidents to feature vectors
3. **Model Training**: Train classifiers on labeled data
4. **Evaluation**: Calculate accuracy, precision, recall, F1-score
5. **Deployment**: Deploy trained models for predictions

### 3. Prediction

When a new incident arrives:

1. Extract features from the incident
2. Run features through trained model
3. Get predicted class and confidence score
4. Return prediction with probabilities for all classes

### 4. Continuous Learning

The system automatically improves over time:

1. New incidents are added to training set
2. When threshold is reached (default: 100 new samples), retrain model
3. Updated model replaces old model
4. Performance metrics are tracked

## Configuration

### Basic Configuration

```toml
[ml]
# Enable ML classification
enabled = true

# Enable specific prediction types
enable_severity_prediction = true
enable_type_prediction = true
enable_priority_prediction = true

# Minimum confidence threshold (0.0 - 1.0)
min_confidence = 0.7

# Maximum training samples to keep in memory
max_training_samples = 10000

# Retrain after N new samples
retrain_threshold = 100

# Enable automatic retraining
auto_retrain = true

# Model storage path
model_path = "./data/models"

[ml.features]
# Maximum vocabulary size for text features
max_vocab_size = 1000

# Minimum document frequency for terms
min_doc_freq = 2

# Use TF-IDF weighting
use_tfidf = true

# Include temporal features
include_temporal = true

# Include source features
include_source = true

# N-gram range (min, max)
ngram_range = [1, 2]  # Unigrams and bigrams
```

### Configuration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enabled` | bool | true | Enable/disable ML system |
| `enable_severity_prediction` | bool | true | Enable severity prediction |
| `enable_type_prediction` | bool | true | Enable type prediction |
| `enable_priority_prediction` | bool | true | Enable priority prediction |
| `min_confidence` | f64 | 0.7 | Minimum confidence threshold (0-1) |
| `max_training_samples` | usize | 10000 | Max samples in memory |
| `retrain_threshold` | usize | 100 | New samples before retraining |
| `auto_retrain` | bool | true | Auto-retrain on threshold |
| `model_path` | string | "./data/models" | Model storage directory |

### Feature Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `max_vocab_size` | usize | 1000 | Maximum vocabulary size |
| `min_doc_freq` | usize | 2 | Minimum term frequency |
| `use_tfidf` | bool | true | Use TF-IDF weighting |
| `include_temporal` | bool | true | Include time features |
| `include_source` | bool | true | Include source features |
| `ngram_range` | [usize; 2] | [1, 2] | N-gram range |

## Usage

### Starting the ML Service

The ML service starts automatically with the application when `enabled = true`.

```bash
# Start with default configuration
./llm-incident-manager

# Start with custom ML configuration
./llm-incident-manager --config config.toml
```

### Making Predictions

#### Predict Severity

```bash
POST /v1/ml/predict/severity
```

Request:
```json
{
  "title": "Database connection timeout",
  "description": "Cannot connect to primary database instance",
  "source": "monitoring"
}
```

Response:
```json
{
  "value": "P0",
  "confidence": 0.89,
  "probabilities": {
    "P0": 0.89,
    "P1": 0.08,
    "P2": 0.02,
    "P3": 0.01,
    "P4": 0.00
  }
}
```

#### Predict All Aspects

```bash
POST /v1/ml/predict/all
```

Request:
```json
{
  "incident_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

Response:
```json
{
  "severity": {
    "value": "P0",
    "confidence": 0.89,
    "probabilities": {...}
  },
  "type": {
    "value": "Infrastructure",
    "confidence": 0.95
  },
  "priority": {
    "value": 9.5,
    "confidence": 0.89
  }
}
```

### Model Management

#### Get ML Service Statistics

```bash
GET /v1/ml/stats
```

Response:
```json
{
  "enabled": true,
  "is_trained": true,
  "n_training_samples": 1547,
  "n_features": 1024,
  "vocab_size": 985,
  "samples_since_training": 47,
  "retrain_threshold": 100
}
```

#### Get Model Metadata

```bash
GET /v1/ml/models
```

Response:
```json
{
  "models": [
    {
      "name": "Logistic Regression",
      "version": "1.0",
      "model_type": "logistic_regression",
      "trained_at": "2025-01-15T10:30:00Z",
      "n_training_samples": 1500,
      "n_features": 1024,
      "training_metrics": {
        "accuracy": 0.87,
        "precision": 0.85,
        "recall": 0.84,
        "f1_score": 0.845
      }
    }
  ]
}
```

#### Force Model Retraining

```bash
POST /v1/ml/retrain
```

Response:
```json
{
  "status": "success",
  "message": "Model retraining initiated",
  "models_retrained": ["severity_classifier"]
}
```

## Interpreting Results

### Confidence Scores

Prediction confidence ranges from 0.0 (no confidence) to 1.0 (certain).

| Confidence | Interpretation | Action |
|------------|----------------|--------|
| 0.9 - 1.0 | Very high confidence | Trust prediction |
| 0.7 - 0.9 | High confidence | Likely accurate |
| 0.5 - 0.7 | Moderate confidence | Review manually |
| 0.3 - 0.5 | Low confidence | Human review required |
| 0.0 - 0.3 | Very low confidence | Don't trust prediction |

### Probability Distributions

The `probabilities` field shows the model's confidence for each possible class:

```json
{
  "P0": 0.75,  // 75% confident it's P0
  "P1": 0.20,  // 20% confident it's P1
  "P2": 0.03,  // 3% confident it's P2
  "P3": 0.01,  // 1% confident it's P3
  "P4": 0.01   // 1% confident it's P4
}
```

If the distribution is flat (all values similar), the model is uncertain.

## Best Practices

### 1. Initial Training

**Requirement**: Minimum 50-100 historical incidents for effective training.

**Best Practice**:
- Start with at least 100 incidents
- Ensure incidents have correct labels (severity, type)
- Include diverse incident types
- Balance classes if possible

### 2. Feature Engineering

**Text Quality Matters**:
- Use descriptive titles and descriptions
- Include technical terms and error messages
- Maintain consistent terminology
- Avoid overly generic descriptions

**Examples**:

Good:
```
Title: "PostgreSQL connection pool exhausted"
Description: "Connection attempts failing with 'FATAL: sorry, too many clients already' error. Pool size: 100, active: 100."
```

Poor:
```
Title: "Database issue"
Description: "Something is wrong"
```

### 3. Tuning Confidence Thresholds

**Too Many False Positives** (incorrect predictions):
- Increase `min_confidence` to 0.8-0.9
- Retrain with more diverse data
- Increase `min_doc_freq` to focus on common terms

**Too Many False Negatives** (missing predictions):
- Decrease `min_confidence` to 0.5-0.6
- Add more training samples
- Review feature extraction settings

### 4. Model Retraining

**When to Retrain**:
- After significant changes to incident patterns
- When model accuracy drops
- After accumulating 100+ new samples
- Periodically (e.g., weekly)

**Monitoring Retraining**:
```bash
# Check samples since last training
GET /v1/ml/stats

# Check if threshold reached
if samples_since_training >= retrain_threshold:
    POST /v1/ml/retrain
```

### 5. Handling Low Confidence Predictions

When confidence is below threshold:

1. **Manual Review**: Have human operator classify
2. **Use Default**: Fall back to rule-based classification
3. **Request More Info**: Ask for additional details
4. **Conservative**: Default to higher severity

## Troubleshooting

### Problem: No Predictions Available

**Symptoms**:
- API returns error "Model not trained"
- All predictions fail

**Causes**:
- Insufficient training data
- ML service not started
- Model training failed

**Solutions**:
1. Check ML service status:
   ```bash
   GET /v1/ml/stats
   ```

2. Verify training data:
   ```bash
   GET /v1/incidents?limit=100
   ```

3. Force training:
   ```bash
   POST /v1/ml/retrain
   ```

4. Check logs:
   ```bash
   grep "ML" /var/log/llm-incident-manager.log
   ```

### Problem: Low Prediction Accuracy

**Symptoms**:
- Accuracy < 70%
- Frequent misclassifications

**Causes**:
- Insufficient training data
- Poor feature quality
- Imbalanced classes
- Overfitting

**Solutions**:
1. **Collect More Data**: Aim for 500+ incidents
2. **Balance Classes**: Ensure each severity has adequate samples
3. **Improve Text Quality**: Use descriptive titles/descriptions
4. **Tune Features**: Adjust `max_vocab_size`, `ngram_range`
5. **Cross-Validation**: Test on held-out data

### Problem: Models Not Retraining

**Symptoms**:
- `samples_since_training` keeps increasing
- Models never update

**Causes**:
- Auto-retrain disabled
- Threshold too high
- Retraining failures

**Solutions**:
```toml
[ml]
auto_retrain = true
retrain_threshold = 100
```

Check logs for training errors:
```bash
grep "retrain" /var/log/llm-incident-manager.log
```

### Problem: High Memory Usage

**Symptoms**:
- Memory usage growing
- OOM errors

**Causes**:
- Too many training samples
- Large vocabulary
- Memory leaks

**Solutions**:
```toml
[ml]
max_training_samples = 5000  # Reduce from 10000

[ml.features]
max_vocab_size = 500  # Reduce from 1000
```

Clear training cache periodically:
```bash
POST /v1/ml/cache/clear
```

## Performance Characteristics

### Latency

| Operation | Typical Latency | Notes |
|-----------|-----------------|-------|
| Feature extraction | 1-5ms | Text processing |
| Severity prediction | 5-20ms | Model inference |
| Training (100 samples) | 100-500ms | Depends on features |
| Training (1000 samples) | 1-5s | Full retrain |

### Throughput

- **Predictions**: 100-500 predictions/second
- **Feature extraction**: 1000+ incidents/second
- **Training**: ~1000 incidents/minute

### Accuracy Benchmarks

With 500+ training samples:

| Metric | Target | Typical |
|--------|--------|---------|
| Severity Accuracy | >80% | 85-90% |
| Type Accuracy | >85% | 88-93% |
| Priority MAE | <1.0 | 0.7-0.9 |

## Advanced Topics

### Custom Feature Engineering

Extend feature extraction:

```rust
use llm_incident_manager::ml::FeatureExtractor;

impl FeatureExtractor {
    pub fn extract_custom_features(&self, incident: &Incident) -> Vec<f64> {
        let mut features = vec![];

        // Add custom features
        features.push(self.extract_error_code_feature(incident));
        features.push(self.extract_stack_trace_feature(incident));

        features
    }
}
```

### Ensemble Models

Combine multiple models:

```rust
let ensemble = vec![
    SeverityClassifier::new(ModelType::LogisticRegression),
    SeverityClassifier::new(ModelType::RandomForest),
    SeverityClassifier::new(ModelType::NaiveBayes),
];

// Average predictions
let final_prediction = average_predictions(&ensemble, &incident);
```

### A/B Testing

Test new models before deployment:

1. Train new model variant
2. Run both models in parallel
3. Compare predictions and accuracy
4. Gradually shift traffic to better model

## FAQ

**Q: How much training data do I need?**

A: Minimum 50-100 incidents per class (severity level). More is better. Aim for 500+ total incidents for production use.

**Q: Will predictions work immediately?**

A: No. The system needs initial training data. On first start, it will train on existing historical incidents.

**Q: How often should models be retrained?**

A: Default is every 100 new incidents. For high-volume systems, consider daily or weekly retraining.

**Q: Can I use custom ML models?**

A: Yes. Implement the `Classifier` trait and integrate with the `MLService`.

**Q: What happens if predictions fail?**

A: The system falls back to manual classification. Incidents are still processed normally.

**Q: Does ML replace human operators?**

A: No. ML assists operators by providing predictions and automating routine classifications. Human oversight is still essential.

**Q: How is sensitive data handled?**

A: ML models are trained on incident metadata, not user data. Feature extraction removes PII. Models are stored locally.

## Support

For issues or questions:
- GitHub Issues: https://github.com/your-org/llm-incident-manager/issues
- Documentation: https://docs.example.com/ml
- Slack: #llm-incident-manager-ml

## See Also

- [ML Implementation Guide](ML_IMPLEMENTATION.md)
- [Correlation Guide](CORRELATION_GUIDE.md)
- [API Reference](API_REFERENCE.md)
