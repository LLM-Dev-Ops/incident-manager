//! Circuit breaker wrappers for LLM integration clients.
//!
//! This module provides circuit breaker protection for all LLM service integrations
//! including Sentinel, Shield, Edge-Agent, and Governance clients.

use crate::circuit_breaker::{
    get_circuit_breaker, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerResult,
};
use crate::integrations::common::IntegrationError;
use std::sync::Arc;

/// Wrapper for Sentinel client with circuit breaker protection
pub struct SentinelClientWithBreaker {
    client: crate::integrations::sentinel::SentinelClient,
    breaker: Arc<CircuitBreaker>,
}

impl SentinelClientWithBreaker {
    /// Create a new Sentinel client with circuit breaker
    pub fn new(client: crate::integrations::sentinel::SentinelClient) -> Self {
        let config = CircuitBreakerConfig::for_llm_service();
        let breaker = get_circuit_breaker("sentinel-llm", config);

        Self { client, breaker }
    }

    /// Create with custom circuit breaker configuration
    pub fn with_config(
        client: crate::integrations::sentinel::SentinelClient,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("sentinel-llm", config);
        Self { client, breaker }
    }

    /// Get the underlying client
    pub fn inner(&self) -> &crate::integrations::sentinel::SentinelClient {
        &self.client
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }

    /// Fetch alerts with circuit breaker protection
    pub async fn fetch_alerts(
        &self,
        limit: Option<usize>,
    ) -> CircuitBreakerResult<Vec<crate::integrations::sentinel::SentinelAlert>> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .fetch_alerts(limit)
                        .await
                        .map_err(|e| IntegrationErrorWrapper(e))
                })
            })
            .await
            .map_err(|e| e)
    }

    /// Predict severity with circuit breaker protection
    pub async fn predict_severity(
        &self,
        request: crate::integrations::sentinel::SeverityPredictionRequest,
    ) -> CircuitBreakerResult<crate::integrations::sentinel::SeverityPrediction> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .predict_severity(request)
                        .await
                        .map_err(|e| IntegrationErrorWrapper(e))
                })
            })
            .await
            .map_err(|e| e)
    }

    /// Analyze anomaly with circuit breaker protection
    pub async fn analyze_anomaly(
        &self,
        alert: &crate::integrations::sentinel::SentinelAlert,
    ) -> CircuitBreakerResult<crate::integrations::sentinel::AnomalyAnalysis> {
        let client = self.client.clone();
        let alert_value = serde_json::to_value(alert)
            .map_err(|e| crate::circuit_breaker::CircuitBreakerError::OperationFailed(e.to_string()))?;
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .analyze_anomaly(alert_value)
                        .await
                        .map_err(|e| IntegrationErrorWrapper(e))
                })
            })
            .await
            .map_err(|e| e)
    }
}

/// Wrapper for Shield client with circuit breaker protection
pub struct ShieldClientWithBreaker {
    client: crate::integrations::shield::ShieldClient,
    breaker: Arc<CircuitBreaker>,
}

impl ShieldClientWithBreaker {
    /// Create a new Shield client with circuit breaker
    pub fn new(client: crate::integrations::shield::ShieldClient) -> Self {
        let config = CircuitBreakerConfig::for_llm_service();
        let breaker = get_circuit_breaker("shield-llm", config);

        Self { client, breaker }
    }

    /// Create with custom circuit breaker configuration
    pub fn with_config(
        client: crate::integrations::shield::ShieldClient,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("shield-llm", config);
        Self { client, breaker }
    }

    /// Get the underlying client
    pub fn inner(&self) -> &crate::integrations::shield::ShieldClient {
        &self.client
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }

    /// Analyze security event with circuit breaker protection
    pub async fn analyze_security_event(
        &self,
        event: crate::integrations::shield::SecurityEvent,
    ) -> CircuitBreakerResult<crate::integrations::shield::ThreatAnalysis> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .analyze_security_event(event)
                        .await
                        .map_err(|e| crate::error::AppError::Integration {
                            integration_source: "shield".to_string(),
                            message: e.to_string(),
                        })
                        .map_err(|e| IntegrationErrorWrapper(crate::integrations::common::IntegrationError::RequestFailed(e.to_string())))
                })
            })
            .await
            .map_err(|e| e)
    }

    /// Assess risk with circuit breaker protection
    pub async fn assess_risk(
        &self,
        request: crate::integrations::shield::AssessRiskRequest,
    ) -> CircuitBreakerResult<crate::integrations::shield::RiskAssessment> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .assess_risk(request)
                        .await
                        .map_err(|e| crate::error::AppError::Integration {
                            integration_source: "shield".to_string(),
                            message: e.to_string(),
                        })
                        .map_err(|e| IntegrationErrorWrapper(crate::integrations::common::IntegrationError::RequestFailed(e.to_string())))
                })
            })
            .await
            .map_err(|e| e)
    }
}

/// Wrapper for Edge-Agent client with circuit breaker protection
pub struct EdgeAgentClientWithBreaker {
    client: crate::integrations::edge_agent::EdgeAgentClient,
    breaker: Arc<CircuitBreaker>,
}

impl EdgeAgentClientWithBreaker {
    /// Create a new Edge-Agent client with circuit breaker
    pub fn new(client: crate::integrations::edge_agent::EdgeAgentClient) -> Self {
        let config = CircuitBreakerConfig::for_llm_service();
        let breaker = get_circuit_breaker("edge-agent-llm", config);

        Self { client, breaker }
    }

    /// Create with custom circuit breaker configuration
    pub fn with_config(
        client: crate::integrations::edge_agent::EdgeAgentClient,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("edge-agent-llm", config);
        Self { client, breaker }
    }

    /// Get the underlying client
    pub fn inner(&self) -> &crate::integrations::edge_agent::EdgeAgentClient {
        &self.client
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }

    /// Submit batch with circuit breaker protection
    pub async fn submit_batch(
        &self,
        batch: crate::integrations::edge_agent::BatchRequest,
    ) -> CircuitBreakerResult<crate::integrations::edge_agent::BatchResponse> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .submit_batch(batch)
                        .await
                        .map_err(|e| IntegrationErrorWrapper(IntegrationError::RequestFailed(e.to_string())))
                })
            })
            .await
            .map_err(|e| e)
    }

    /// Get resource usage with circuit breaker protection
    pub async fn get_resource_usage(
        &self,
    ) -> CircuitBreakerResult<crate::integrations::edge_agent::ResourceUsage> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .get_resource_usage()
                        .await
                        .map_err(|e| IntegrationErrorWrapper(IntegrationError::RequestFailed(e.to_string())))
                })
            })
            .await
            .map_err(|e| e)
    }
}

/// Wrapper for Governance client with circuit breaker protection
pub struct GovernanceClientWithBreaker {
    client: crate::integrations::governance::GovernanceClient,
    breaker: Arc<CircuitBreaker>,
}

impl GovernanceClientWithBreaker {
    /// Create a new Governance client with circuit breaker
    pub fn new(client: crate::integrations::governance::GovernanceClient) -> Self {
        let config = CircuitBreakerConfig::for_llm_service();
        let breaker = get_circuit_breaker("governance-llm", config);

        Self { client, breaker }
    }

    /// Create with custom circuit breaker configuration
    pub fn with_config(
        client: crate::integrations::governance::GovernanceClient,
        config: CircuitBreakerConfig,
    ) -> Self {
        let breaker = get_circuit_breaker("governance-llm", config);
        Self { client, breaker }
    }

    /// Get the underlying client
    pub fn inner(&self) -> &crate::integrations::governance::GovernanceClient {
        &self.client
    }

    /// Get the circuit breaker
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }

    /// Check compliance with circuit breaker protection
    pub async fn check_compliance(
        &self,
        request: crate::integrations::governance::ComplianceRequest,
    ) -> CircuitBreakerResult<crate::integrations::governance::ComplianceResponse> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .check_compliance(request)
                        .await
                        .map_err(|e| crate::error::AppError::Integration {
                            integration_source: "governance".to_string(),
                            message: e.to_string(),
                        })
                        .map_err(|e| IntegrationErrorWrapper(crate::integrations::common::IntegrationError::RequestFailed(e.to_string())))
                })
            })
            .await
            .map_err(|e| e)
    }

    /// Generate audit report with circuit breaker protection
    pub async fn generate_audit_report(
        &self,
        incident_id: uuid::Uuid,
        report_type: crate::integrations::governance::AuditReportType,
    ) -> CircuitBreakerResult<crate::integrations::governance::AuditReport> {
        let client = self.client.clone();
        self.breaker
            .call(|| {
                Box::pin(async move {
                    client
                        .generate_audit_report(incident_id, report_type)
                        .await
                        .map_err(|e| crate::error::AppError::Integration {
                            integration_source: "governance".to_string(),
                            message: e.to_string(),
                        })
                        .map_err(|e| IntegrationErrorWrapper(crate::integrations::common::IntegrationError::RequestFailed(e.to_string())))
                })
            })
            .await
            .map_err(|e| e)
    }
}

/// Wrapper for IntegrationError to implement std::error::Error
#[derive(Debug)]
struct IntegrationErrorWrapper(crate::integrations::common::IntegrationError);

impl std::fmt::Display for IntegrationErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for IntegrationErrorWrapper {}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require actual client instances which depend on configuration
    // In a real implementation, you would use mocks or test fixtures
}
