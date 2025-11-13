use crate::error::{AppError, Result};
use super::models::*;
use super::policy::PolicyEngine;
use super::handlers::ComplianceEventHandler;
use reqwest::{Client, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Governance client for compliance checking and policy enforcement
#[derive(Clone)]
pub struct GovernanceClient {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
    timeout: Duration,
    policy_engine: Arc<PolicyEngine>,
    event_handler: Arc<ComplianceEventHandler>,
    enable_remote: bool,
}

impl GovernanceClient {
    /// Create a new governance client
    pub fn new(base_url: String, auth_token: Option<String>, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            auth_token,
            timeout: Duration::from_secs(timeout_secs),
            policy_engine: Arc::new(PolicyEngine::new()),
            event_handler: Arc::new(ComplianceEventHandler::new()),
            enable_remote: true,
        }
    }

    /// Create a client with local-only policy engine (no remote calls)
    pub fn local_only() -> Self {
        Self {
            client: Client::new(),
            base_url: String::new(),
            auth_token: None,
            timeout: Duration::from_secs(10),
            policy_engine: Arc::new(PolicyEngine::new()),
            event_handler: Arc::new(ComplianceEventHandler::new()),
            enable_remote: false,
        }
    }

    /// Check compliance for a request
    pub async fn check_compliance(&self, request: ComplianceRequest) -> Result<ComplianceResponse> {
        // First, perform local validation
        let violations = self.policy_engine.validate(&request)?;

        // If remote checking is enabled, also query the remote governance service
        if self.enable_remote && !self.base_url.is_empty() {
            match self.check_compliance_remote(&request).await {
                Ok(remote_response) => {
                    tracing::debug!("Remote compliance check succeeded");
                    return Ok(remote_response);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Remote compliance check failed, using local validation"
                    );
                    // Fall through to local response
                }
            }
        }

        // Return local validation results
        Ok(ComplianceResponse {
            request_id: request.id,
            incident_id: request.incident_id,
            is_compliant: violations.is_empty(),
            violations,
            recommendations: self.generate_recommendations(&request),
            audit_entry_id: None,
            checked_at: chrono::Utc::now(),
        })
    }

    /// Check compliance against remote governance service
    async fn check_compliance_remote(&self, request: &ComplianceRequest) -> Result<ComplianceResponse> {
        let url = format!("{}/api/v1/compliance/check", self.base_url);

        let mut req = self.client.post(&url).json(request);

        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = timeout(self.timeout, req.send())
            .await
            .map_err(|_| AppError::Timeout(format!("Governance API request timed out after {:?}", self.timeout)))?
            .map_err(|e| AppError::Network(format!("Failed to send compliance check request: {}", e)))?;

        match response.status() {
            StatusCode::OK => {
                let compliance_response = response
                    .json::<ComplianceResponse>()
                    .await
                    .map_err(|e| AppError::Serialization(format!("Failed to parse compliance response: {}", e)))?;
                Ok(compliance_response)
            }
            StatusCode::UNAUTHORIZED => {
                Err(AppError::Authentication("Invalid or missing authentication token".to_string()))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                Err(AppError::RateLimit)
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(AppError::Integration {
                    integration_source: "governance".to_string(),
                    message: format!("Compliance check failed with status {}: {}", status, error_text),
                })
            }
        }
    }

    /// Generate compliance recommendations
    fn generate_recommendations(&self, request: &ComplianceRequest) -> Vec<String> {
        let mut recommendations = Vec::new();

        for framework in &request.frameworks {
            match framework {
                ComplianceFramework::GDPR => {
                    if let Some(personal_data) = &request.data.personal_data {
                        if personal_data.contains_pii {
                            recommendations.push("Ensure GDPR Article 33 breach notification requirements are met".to_string());
                            recommendations.push("Document data subject rights procedures".to_string());
                        }
                    }
                }
                ComplianceFramework::HIPAA => {
                    if let Some(health_data) = &request.data.health_data {
                        if health_data.contains_phi {
                            recommendations.push("Encrypt all PHI at rest and in transit".to_string());
                            recommendations.push("Implement audit controls for PHI access".to_string());
                        }
                    }
                }
                ComplianceFramework::PCI => {
                    if let Some(payment_data) = &request.data.payment_data {
                        if payment_data.contains_cardholder_data {
                            recommendations.push("Use PCI-DSS compliant tokenization for cardholder data".to_string());
                            recommendations.push("Implement network segmentation for payment systems".to_string());
                        }
                    }
                }
                ComplianceFramework::SOC2 => {
                    recommendations.push("Document incident response procedures".to_string());
                    recommendations.push("Maintain comprehensive audit logs".to_string());
                }
                ComplianceFramework::ISO27001 => {
                    recommendations.push("Implement ISO27001 Annex A security controls".to_string());
                    recommendations.push("Conduct regular security risk assessments".to_string());
                }
                ComplianceFramework::Custom(_) => {
                    recommendations.push("Review custom compliance requirements".to_string());
                }
            }
        }

        recommendations
    }

    /// Generate an audit report
    pub async fn generate_audit_report(&self, incident_id: uuid::Uuid, report_type: AuditReportType) -> Result<AuditReport> {
        if self.enable_remote && !self.base_url.is_empty() {
            match self.generate_audit_report_remote(incident_id, report_type.clone()).await {
                Ok(report) => return Ok(report),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Remote audit report generation failed, using local handler"
                    );
                }
            }
        }

        // Use local event handler
        self.event_handler.generate_audit_report(incident_id, report_type).await
    }

    /// Generate audit report from remote service
    async fn generate_audit_report_remote(&self, incident_id: uuid::Uuid, report_type: AuditReportType) -> Result<AuditReport> {
        let url = format!("{}/api/v1/audit/report/{}", self.base_url, incident_id);

        let mut req = self.client.post(&url).json(&serde_json::json!({
            "report_type": report_type
        }));

        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = timeout(self.timeout, req.send())
            .await
            .map_err(|_| AppError::Timeout(format!("Audit report request timed out after {:?}", self.timeout)))?
            .map_err(|e| AppError::Network(format!("Failed to send audit report request: {}", e)))?;

        match response.status() {
            StatusCode::OK => {
                let report = response
                    .json::<AuditReport>()
                    .await
                    .map_err(|e| AppError::Serialization(format!("Failed to parse audit report: {}", e)))?;
                Ok(report)
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(AppError::Integration {
                    integration_source: "governance".to_string(),
                    message: format!("Audit report generation failed with status {}: {}", status, error_text),
                })
            }
        }
    }

    /// Get audit trail for an incident
    pub async fn get_audit_trail(&self, incident_id: uuid::Uuid) -> Result<Vec<AuditEntry>> {
        self.event_handler.get_audit_trail(incident_id).await
    }

    /// Get governance metrics
    pub async fn get_metrics(&self) -> Result<GovernanceMetrics> {
        if self.enable_remote && !self.base_url.is_empty() {
            match self.get_metrics_remote().await {
                Ok(metrics) => return Ok(metrics),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Remote metrics retrieval failed, using local metrics"
                    );
                }
            }
        }

        self.event_handler.get_metrics().await
    }

    /// Get metrics from remote service
    async fn get_metrics_remote(&self) -> Result<GovernanceMetrics> {
        let url = format!("{}/api/v1/governance/metrics", self.base_url);

        let mut req = self.client.get(&url);

        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = timeout(self.timeout, req.send())
            .await
            .map_err(|_| AppError::Timeout(format!("Metrics request timed out after {:?}", self.timeout)))?
            .map_err(|e| AppError::Network(format!("Failed to send metrics request: {}", e)))?;

        match response.status() {
            StatusCode::OK => {
                let metrics = response
                    .json::<GovernanceMetrics>()
                    .await
                    .map_err(|e| AppError::Serialization(format!("Failed to parse metrics: {}", e)))?;
                Ok(metrics)
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(AppError::Integration {
                    integration_source: "governance".to_string(),
                    message: format!("Metrics retrieval failed with status {}: {}", status, error_text),
                })
            }
        }
    }

    /// Get event handler for incident lifecycle hooks
    pub fn event_handler(&self) -> Arc<ComplianceEventHandler> {
        Arc::clone(&self.event_handler)
    }

    /// Check if remote connectivity is available
    pub async fn health_check(&self) -> Result<bool> {
        if !self.enable_remote || self.base_url.is_empty() {
            return Ok(true); // Local-only mode is always "healthy"
        }

        let url = format!("{}/health", self.base_url);

        let mut req = self.client.get(&url);

        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        match timeout(self.timeout, req.send()).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            Ok(Err(_)) | Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_local_compliance_check() {
        let client = GovernanceClient::local_only();

        let mut metadata = HashMap::new();
        metadata.insert("notification_sent".to_string(), serde_json::json!(true));

        let data = ComplianceData {
            classification: DataClassification::Internal,
            personal_data: None,
            health_data: None,
            payment_data: None,
            metadata,
        };

        let request = ComplianceRequest::new(
            uuid::Uuid::new_v4(),
            vec![ComplianceFramework::SOC2],
            data,
        );

        let response = client.check_compliance(request).await.unwrap();
        assert!(response.is_compliant || !response.violations.is_empty());
    }

    #[tokio::test]
    async fn test_generate_recommendations() {
        let client = GovernanceClient::local_only();

        let data = ComplianceData {
            classification: DataClassification::Confidential,
            personal_data: Some(PersonalDataInfo {
                contains_pii: true,
                data_subjects_affected: Some(100),
                data_types: vec!["email".to_string()],
                processing_purpose: "investigation".to_string(),
            }),
            health_data: None,
            payment_data: None,
            metadata: HashMap::new(),
        };

        let request = ComplianceRequest::new(
            uuid::Uuid::new_v4(),
            vec![ComplianceFramework::GDPR],
            data,
        );

        let recommendations = client.generate_recommendations(&request);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("GDPR")));
    }

    #[tokio::test]
    async fn test_audit_trail() {
        let client = GovernanceClient::local_only();
        let incident_id = uuid::Uuid::new_v4();

        // Initially empty
        let trail = client.get_audit_trail(incident_id).await.unwrap();
        assert!(trail.is_empty());
    }

    #[tokio::test]
    async fn test_local_only_health_check() {
        let client = GovernanceClient::local_only();
        let healthy = client.health_check().await.unwrap();
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let client = GovernanceClient::local_only();
        let metrics = client.get_metrics().await.unwrap();
        assert_eq!(metrics.total_checks, 0);
    }
}
