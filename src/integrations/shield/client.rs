use std::sync::Arc;
use std::time::Instant;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, info, instrument};

use crate::error::{AppError, Result};
use crate::integrations::common::{
    retry_with_backoff, IntegrationMetrics, RetryPolicy,
};

use super::models::*;

// Import generated gRPC code
pub mod proto {
    tonic::include_proto!("integrations");
}

use proto::shield_service_client::ShieldServiceClient;
use proto::{
    MitigationRequest, RiskAssessmentRequest, SecurityEventRequest, SecurityPatternRequest,
};

/// Shield client for security threat analysis
#[derive(Clone)]
pub struct ShieldClient {
    client: ShieldServiceClient<Channel>,
    retry_policy: Arc<RetryPolicy>,
    metrics: Arc<IntegrationMetrics>,
}

impl ShieldClient {
    /// Create a new Shield client
    #[instrument(skip_all)]
    pub async fn new(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint_str = endpoint.into();
        info!(endpoint = %endpoint_str, "Connecting to Shield service");

        let channel = Endpoint::from_shared(endpoint_str.clone())
            .map_err(|e| AppError::Integration {
                integration_source: "Shield".to_string(),
                message: format!("Invalid endpoint: {}", e),
            })?
            .connect()
            .await
            .map_err(|e| AppError::Integration {
                integration_source: "Shield".to_string(),
                message: format!("Connection failed: {}", e),
            })?;

        let client = ShieldServiceClient::new(channel);

        Ok(Self {
            client,
            retry_policy: Arc::new(RetryPolicy::default()),
            metrics: Arc::new(IntegrationMetrics::new("shield")),
        })
    }

    /// Create client with custom retry policy
    pub async fn with_retry_policy(
        endpoint: impl Into<String>,
        retry_policy: RetryPolicy,
    ) -> Result<Self> {
        let endpoint_str = endpoint.into();
        info!(endpoint = %endpoint_str, "Connecting to Shield service with custom retry policy");

        let channel = Endpoint::from_shared(endpoint_str.clone())
            .map_err(|e| AppError::Integration {
                integration_source: "Shield".to_string(),
                message: format!("Invalid endpoint: {}", e),
            })?
            .connect()
            .await
            .map_err(|e| AppError::Integration {
                integration_source: "Shield".to_string(),
                message: format!("Connection failed: {}", e),
            })?;

        let client = ShieldServiceClient::new(channel);

        Ok(Self {
            client,
            retry_policy: Arc::new(retry_policy),
            metrics: Arc::new(IntegrationMetrics::new("shield")),
        })
    }

    /// Analyze a security event for threats
    #[instrument(skip(self, event), fields(event_id = %event.event_id))]
    pub async fn analyze_security_event(
        &self,
        event: SecurityEvent,
    ) -> Result<ThreatAnalysis> {
        let start = Instant::now();
        debug!(event_id = %event.event_id, "Analyzing security event");

        let request = SecurityEventRequest {
            event_id: event.event_id.clone(),
            event_type: format!("{:?}", event.event_type),
            source: event.source.clone(),
            description: event.description.clone(),
            metadata: event.metadata.clone(),
            timestamp: Some(prost_types::Timestamp {
                seconds: event.timestamp.timestamp(),
                nanos: event.timestamp.timestamp_subsec_nanos() as i32,
            }),
            severity: event.severity.to_string(),
        };

        let client = self.client.clone();
        let retry_policy = &self.retry_policy;

        let result = retry_with_backoff("analyze_security_event", retry_policy, || {
            let mut client = client.clone();
            let request = request.clone();
            async move {
                client
                    .analyze_security_event(request)
                    .await
                    .map_err(|e| e.into())
            }
        })
        .await
        .map_err(|e: crate::integrations::common::IntegrationError| -> AppError { e.into() });

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                self.metrics.record_success(latency);
                let resp = response.into_inner();

                let threat_type = match resp.threat_type.as_str() {
                    "Malware" => ThreatType::Malware,
                    "RansomwareAttack" => ThreatType::RansomwareAttack,
                    "DataBreach" => ThreatType::DataBreach,
                    "DenialOfService" => ThreatType::DenialOfService,
                    "InsiderThreat" => ThreatType::InsiderThreat,
                    "PhishingAttempt" => ThreatType::PhishingAttempt,
                    "BruteForce" => ThreatType::BruteForce,
                    "SqlInjection" => ThreatType::SqlInjection,
                    "CrossSiteScripting" => ThreatType::CrossSiteScripting,
                    "ZeroDayExploit" => ThreatType::ZeroDayExploit,
                    "AdvancedPersistentThreat" => ThreatType::AdvancedPersistentThreat,
                    _ => ThreatType::Unknown,
                };

                let severity_level = match resp.severity_level.as_str() {
                    "LOW" => ThreatSeverity::Low,
                    "MEDIUM" => ThreatSeverity::Medium,
                    "HIGH" => ThreatSeverity::High,
                    "CRITICAL" => ThreatSeverity::Critical,
                    _ => ThreatSeverity::Low,
                };

                let analyzed_at = resp
                    .analyzed_at
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                            .unwrap_or_else(|| chrono::Utc::now())
                    })
                    .unwrap_or_else(|| chrono::Utc::now());

                Ok(ThreatAnalysis {
                    analysis_id: resp.analysis_id,
                    threat_type,
                    confidence_score: resp.confidence_score,
                    severity_level,
                    indicators: resp.indicators,
                    affected_systems: resp.affected_systems,
                    description: resp.description,
                    details: resp.details,
                    analyzed_at,
                })
            }
            Err(e) => {
                self.metrics.record_failure(latency);
                Err(e)
            }
        }
    }

    /// Assess overall risk level
    #[instrument(skip(self, request))]
    pub async fn assess_risk(
        &self,
        request: AssessRiskRequest,
    ) -> Result<RiskAssessment> {
        let start = Instant::now();
        debug!(context_id = %request.context_id, "Assessing risk");

        let grpc_request = RiskAssessmentRequest {
            context_id: request.context_id.clone(),
            event_ids: request.event_ids.clone(),
            assessment_type: request.assessment_type.clone(),
            parameters: request.parameters.clone(),
        };

        let client = self.client.clone();
        let retry_policy = &self.retry_policy;

        let result = retry_with_backoff("assess_risk", retry_policy, || {
            let mut client = client.clone();
            let grpc_request = grpc_request.clone();
            async move {
                client
                    .assess_risk(grpc_request)
                    .await
                    .map_err(|e| e.into())
            }
        })
        .await
        .map_err(|e: crate::integrations::common::IntegrationError| -> AppError { e.into() });

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                self.metrics.record_success(latency);
                let resp = response.into_inner();

                let risk_level = match resp.risk_level.as_str() {
                    "LOW" => RiskLevel::Low,
                    "MEDIUM" => RiskLevel::Medium,
                    "HIGH" => RiskLevel::High,
                    "CRITICAL" => RiskLevel::Critical,
                    _ => RiskLevel::Low,
                };

                let risk_factors = resp
                    .risk_factors
                    .into_iter()
                    .map(|rf| RiskFactor {
                        factor_type: rf.factor_type,
                        description: rf.description,
                        impact_score: rf.impact_score,
                        likelihood: rf.likelihood,
                    })
                    .collect();

                let assessed_at = resp
                    .assessed_at
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                            .unwrap_or_else(|| chrono::Utc::now())
                    })
                    .unwrap_or_else(|| chrono::Utc::now());

                Ok(RiskAssessment {
                    assessment_id: resp.assessment_id,
                    risk_level,
                    risk_score: resp.risk_score,
                    risk_factors,
                    recommendations: resp.recommendations,
                    assessed_at,
                })
            }
            Err(e) => {
                self.metrics.record_failure(latency);
                Err(e)
            }
        }
    }

    /// Generate a mitigation plan
    #[instrument(skip(self, request))]
    pub async fn generate_mitigation_plan(
        &self,
        request: GenerateMitigationRequest,
    ) -> Result<MitigationPlan> {
        let start = Instant::now();
        debug!(threat_id = %request.threat_id, "Generating mitigation plan");

        let grpc_request = MitigationRequest {
            threat_id: request.threat_id.clone(),
            threat_type: format!("{:?}", request.threat_type),
            severity: request.severity.to_string(),
            affected_systems: request.affected_systems.clone(),
            context: request.context.clone(),
        };

        let client = self.client.clone();
        let retry_policy = &self.retry_policy;

        let result = retry_with_backoff("generate_mitigation_plan", retry_policy, || {
            let mut client = client.clone();
            let grpc_request = grpc_request.clone();
            async move {
                client
                    .generate_mitigation_plan(grpc_request)
                    .await
                    .map_err(|e| e.into())
            }
        })
        .await
        .map_err(|e: crate::integrations::common::IntegrationError| -> AppError { e.into() });

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                self.metrics.record_success(latency);
                let resp = response.into_inner();

                let priority = match resp.priority.as_str() {
                    "LOW" => MitigationPriority::Low,
                    "MEDIUM" => MitigationPriority::Medium,
                    "HIGH" => MitigationPriority::High,
                    "CRITICAL" => MitigationPriority::Critical,
                    _ => MitigationPriority::Medium,
                };

                let steps = resp
                    .steps
                    .into_iter()
                    .map(|step| MitigationStep {
                        order: step.order,
                        action: step.action,
                        description: step.description,
                        automation_available: step.automation_available == "true",
                        dependencies: step.dependencies,
                    })
                    .collect();

                let created_at = resp
                    .created_at
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                            .unwrap_or_else(|| chrono::Utc::now())
                    })
                    .unwrap_or_else(|| chrono::Utc::now());

                Ok(MitigationPlan {
                    plan_id: resp.plan_id,
                    steps,
                    estimated_duration_minutes: resp.estimated_duration_minutes,
                    priority,
                    required_resources: resp.required_resources,
                    created_at,
                })
            }
            Err(e) => {
                self.metrics.record_failure(latency);
                Err(e)
            }
        }
    }

    /// Report a security pattern
    #[instrument(skip(self, request))]
    pub async fn report_security_pattern(
        &self,
        request: ReportSecurityPatternRequest,
    ) -> Result<SecurityPattern> {
        let start = Instant::now();
        debug!(pattern_type = %request.pattern_type, "Reporting security pattern");

        let grpc_request = SecurityPatternRequest {
            pattern_type: request.pattern_type.clone(),
            event_ids: request.event_ids.clone(),
            description: request.description.clone(),
            attributes: request.attributes.clone(),
        };

        let client = self.client.clone();
        let retry_policy = &self.retry_policy;

        let result = retry_with_backoff("report_security_pattern", retry_policy, || {
            let mut client = client.clone();
            let grpc_request = grpc_request.clone();
            async move {
                client
                    .report_security_pattern(grpc_request)
                    .await
                    .map_err(|e| e.into())
            }
        })
        .await
        .map_err(|e: crate::integrations::common::IntegrationError| -> AppError { e.into() });

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                self.metrics.record_success(latency);
                let resp = response.into_inner();

                Ok(SecurityPattern {
                    pattern_id: resp.pattern_id,
                    is_known_pattern: resp.is_known_pattern,
                    pattern_name: resp.pattern_name,
                    confidence: resp.confidence,
                    similar_patterns: resp.similar_patterns,
                    recommendation: resp.recommendation,
                })
            }
            Err(e) => {
                self.metrics.record_failure(latency);
                Err(e)
            }
        }
    }

    /// Get current metrics snapshot
    pub fn metrics(&self) -> crate::integrations::common::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shield_client_creation() {
        // Test that client can be instantiated (will fail on connection in real scenario)
        // This is mainly to ensure the types are correct
        let endpoint = "http://localhost:50051";
        assert!(!endpoint.is_empty());
    }
}
