use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::models::{
    AnomalyAnalysis, SentinelAlert, SentinelResponse, SeverityPrediction,
    SeverityPredictionRequest,
};
use crate::integrations::common::{
    ConnectionConfig, ConnectionState, Credentials, HealthCheck, HealthStatus, IntegrationError,
    IntegrationResult, LLMClient, RetryPolicy,
};

/// Sentinel client for LLM monitoring and alerting
#[derive(Clone)]
pub struct SentinelClient {
    /// HTTP client
    client: Client,
    /// Connection configuration
    config: ConnectionConfig,
    /// Authentication credentials
    credentials: Credentials,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Retry policy
    retry_policy: RetryPolicy,
}

impl SentinelClient {
    /// Create a new Sentinel client
    pub fn new(config: ConnectionConfig, credentials: Credentials) -> IntegrationResult<Self> {
        // Validate credentials
        credentials.validate()?;

        // Build HTTP client
        let client = Client::builder()
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(config.max_connections)
            .danger_accept_invalid_certs(!config.verify_tls)
            .build()
            .map_err(|e| {
                IntegrationError::Configuration(format!("Failed to build HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            config,
            credentials,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            retry_policy: RetryPolicy::default(),
        })
    }

    /// Set custom retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Fetch recent alerts from Sentinel
    pub async fn fetch_alerts(&self, limit: Option<usize>) -> IntegrationResult<Vec<SentinelAlert>> {
        if !self.is_connected() {
            return Err(IntegrationError::Connection(
                "Client is not connected".to_string(),
            ));
        }

        let url = format!("{}/api/v1/alerts", self.config.base_url);
        
        debug!(url = %url, limit = ?limit, "Fetching alerts from Sentinel");

        let mut request = self.client.get(&url);
        
        // Add limit query parameter
        if let Some(limit) = limit {
            request = request.query(&[("limit", limit.to_string())]);
        }

        // Apply authentication
        request = self.credentials.apply_to_request(request)?;

        let response = request.send().await.map_err(|e| {
            error!(error = %e, "Failed to fetch alerts from Sentinel");
            IntegrationError::RequestFailed(e.to_string())
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown".to_string());
            error!(status = %status, body = %body, "Sentinel API returned error");
            return Err(IntegrationError::RequestFailed(format!(
                "API returned {}: {}",
                status, body
            )));
        }

        let sentinel_response: SentinelResponse<Vec<SentinelAlert>> = response
            .json()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to parse Sentinel response");
                IntegrationError::Serialization(e.to_string())
            })?;

        let alerts = sentinel_response.into_result().map_err(|e| {
            IntegrationError::InvalidResponse(format!("Sentinel returned error: {}", e))
        })?;

        info!(count = alerts.len(), "Fetched alerts from Sentinel");
        Ok(alerts)
    }

    /// Analyze data for anomalies
    pub async fn analyze_anomaly(
        &self,
        data: serde_json::Value,
    ) -> IntegrationResult<AnomalyAnalysis> {
        if !self.is_connected() {
            return Err(IntegrationError::Connection(
                "Client is not connected".to_string(),
            ));
        }

        let url = format!("{}/api/v1/analyze", self.config.base_url);
        
        debug!(url = %url, "Requesting anomaly analysis");

        let mut request = self.client.post(&url).json(&data);
        request = self.credentials.apply_to_request(request)?;

        let response = request.send().await.map_err(|e| {
            error!(error = %e, "Failed to request anomaly analysis");
            IntegrationError::RequestFailed(e.to_string())
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown".to_string());
            error!(status = %status, body = %body, "Anomaly analysis failed");
            return Err(IntegrationError::RequestFailed(format!(
                "API returned {}: {}",
                status, body
            )));
        }

        let sentinel_response: SentinelResponse<AnomalyAnalysis> = response
            .json()
            .await
            .map_err(|e| IntegrationError::Serialization(e.to_string()))?;

        let analysis = sentinel_response.into_result().map_err(|e| {
            IntegrationError::InvalidResponse(format!("Sentinel returned error: {}", e))
        })?;

        info!(
            is_anomalous = analysis.is_anomalous,
            confidence = analysis.confidence,
            "Anomaly analysis completed"
        );

        Ok(analysis)
    }

    /// Predict severity for an incident
    pub async fn predict_severity(
        &self,
        request: SeverityPredictionRequest,
    ) -> IntegrationResult<SeverityPrediction> {
        if !self.is_connected() {
            return Err(IntegrationError::Connection(
                "Client is not connected".to_string(),
            ));
        }

        let url = format!("{}/api/v1/predict-severity", self.config.base_url);
        
        debug!(url = %url, "Requesting severity prediction");

        let mut http_request = self.client.post(&url).json(&request);
        http_request = self.credentials.apply_to_request(http_request)?;

        let response = http_request.send().await.map_err(|e| {
            error!(error = %e, "Failed to request severity prediction");
            IntegrationError::RequestFailed(e.to_string())
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown".to_string());
            error!(status = %status, body = %body, "Severity prediction failed");
            return Err(IntegrationError::RequestFailed(format!(
                "API returned {}: {}",
                status, body
            )));
        }

        let sentinel_response: SentinelResponse<SeverityPrediction> = response
            .json()
            .await
            .map_err(|e| IntegrationError::Serialization(e.to_string()))?;

        let prediction = sentinel_response.into_result().map_err(|e| {
            IntegrationError::InvalidResponse(format!("Sentinel returned error: {}", e))
        })?;

        info!(
            predicted_severity = ?prediction.predicted_severity,
            confidence = prediction.confidence,
            "Severity prediction completed"
        );

        Ok(prediction)
    }
}

#[async_trait]
impl LLMClient for SentinelClient {
    async fn connect(&mut self) -> IntegrationResult<()> {
        let mut state = self.state.write().await;
        
        if state.is_connected() {
            warn!("Client is already connected");
            return Ok(());
        }

        *state = ConnectionState::Connecting;
        drop(state);

        info!(base_url = %self.config.base_url, "Connecting to Sentinel");

        // Perform health check to verify connection
        match self.health_check().await {
            Ok(health) if health.is_operational() => {
                let mut state = self.state.write().await;
                *state = ConnectionState::Connected;
                info!("Successfully connected to Sentinel");
                Ok(())
            }
            Ok(health) => {
                let mut state = self.state.write().await;
                *state = ConnectionState::Failed;
                Err(IntegrationError::Connection(format!(
                    "Sentinel is unhealthy: {:?}",
                    health.details
                )))
            }
            Err(e) => {
                let mut state = self.state.write().await;
                *state = ConnectionState::Failed;
                error!(error = %e, "Failed to connect to Sentinel");
                Err(e)
            }
        }
    }

    async fn disconnect(&mut self) -> IntegrationResult<()> {
        let mut state = self.state.write().await;
        
        if !state.is_connected() {
            warn!("Client is not connected");
            return Ok(());
        }

        *state = ConnectionState::Disconnected;
        info!("Disconnected from Sentinel");
        Ok(())
    }

    async fn health_check(&self) -> IntegrationResult<HealthCheck> {
        let url = format!("{}/api/v1/health", self.config.base_url);
        
        debug!(url = %url, "Performing health check");

        let start = std::time::Instant::now();
        
        let mut request = self.client.get(&url);
        request = self.credentials.apply_to_request(request)?;

        let response = request
            .send()
            .await
            .map_err(|e| IntegrationError::RequestFailed(e.to_string()))?;

        let response_time_ms = start.elapsed().as_millis() as u64;

        let status = if response.status().is_success() {
            HealthStatus::Healthy
        } else if response.status().is_server_error() {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        };

        let details = if !response.status().is_success() {
            Some(format!("HTTP {}", response.status()))
        } else {
            None
        };

        let check = HealthCheck::new("sentinel".to_string(), status, response_time_ms);
        let check = if let Some(details) = details {
            check.with_details(details)
        } else {
            check
        };

        debug!(
            status = ?check.status,
            response_time_ms = check.response_time_ms,
            "Health check completed"
        );

        Ok(check)
    }

    fn is_connected(&self) -> bool {
        // Use try_read to avoid blocking
        self.state
            .try_read()
            .map(|s| s.is_connected())
            .unwrap_or(false)
    }

    fn service_name(&self) -> &str {
        "sentinel"
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentinel_client_creation() {
        let config = ConnectionConfig::new("https://sentinel.example.com".to_string());
        let credentials = Credentials::api_key("test-key");

        let client = SentinelClient::new(config, credentials);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.service_name(), "sentinel");
        assert!(!client.is_connected());
    }

    #[test]
    fn test_sentinel_client_with_invalid_credentials() {
        let config = ConnectionConfig::new("https://sentinel.example.com".to_string());
        let credentials = Credentials::api_key(""); // Empty key should fail validation

        let client = SentinelClient::new(config, credentials);
        assert!(client.is_err());
    }

    #[test]
    fn test_sentinel_client_with_retry_policy() {
        let config = ConnectionConfig::new("https://sentinel.example.com".to_string());
        let credentials = Credentials::api_key("test-key");
        let retry_policy = RetryPolicy::new(5);

        let client = SentinelClient::new(config, credentials)
            .unwrap()
            .with_retry_policy(retry_policy);

        assert_eq!(client.retry_policy.max_attempts, 5);
    }
}
