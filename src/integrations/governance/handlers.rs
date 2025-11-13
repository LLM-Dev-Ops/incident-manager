use crate::error::{AppError, Result};
use crate::models::Incident;
use super::models::*;
use super::policy::PolicyEngine;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// Compliance event handler for processing governance events
pub struct ComplianceEventHandler {
    policy_engine: Arc<PolicyEngine>,
    audit_entries: Arc<RwLock<HashMap<Uuid, Vec<AuditEntry>>>>,
    metrics: Arc<RwLock<GovernanceMetrics>>,
}

impl ComplianceEventHandler {
    pub fn new() -> Self {
        Self {
            policy_engine: Arc::new(PolicyEngine::new()),
            audit_entries: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(GovernanceMetrics::default())),
        }
    }

    /// Handle incident creation event
    pub async fn on_incident_created(&self, incident: &Incident) -> Result<()> {
        tracing::info!(
            incident_id = %incident.id,
            "Handling incident creation for governance"
        );

        // Create audit entry
        let entry = AuditEntry::new(
            incident.id,
            "incident_created".to_string(),
            "system".to_string(),
        )
        .with_detail("severity".to_string(), serde_json::json!(format!("{:?}", incident.severity)))
        .with_detail("source".to_string(), serde_json::json!(&incident.source));

        self.add_audit_entry(entry).await?;

        // Determine required compliance checks based on incident data
        let frameworks = self.determine_required_frameworks(incident);

        if !frameworks.is_empty() {
            let data = self.extract_compliance_data(incident)?;
            let request = ComplianceRequest::new(incident.id, frameworks, data);

            // Perform compliance check
            let violations = self.policy_engine.validate(&request)?;

            // Update metrics
            self.update_metrics(violations.is_empty(), &violations).await?;

            // Log violations if any
            if !violations.is_empty() {
                tracing::warn!(
                    incident_id = %incident.id,
                    violations = violations.len(),
                    "Compliance violations detected"
                );

                for violation in &violations {
                    let entry = AuditEntry::new(
                        incident.id,
                        "compliance_violation".to_string(),
                        "policy_engine".to_string(),
                    )
                    .with_detail("policy_id".to_string(), serde_json::json!(&violation.policy_id))
                    .with_detail("severity".to_string(), serde_json::json!(format!("{:?}", violation.severity)));

                    self.add_audit_entry(entry).await?;
                }
            }
        }

        Ok(())
    }

    /// Handle incident update event
    pub async fn on_incident_updated(&self, incident: &Incident, updated_by: &str) -> Result<()> {
        tracing::debug!(
            incident_id = %incident.id,
            updated_by = updated_by,
            "Handling incident update for governance"
        );

        let entry = AuditEntry::new(
            incident.id,
            "incident_updated".to_string(),
            updated_by.to_string(),
        )
        .with_detail("state".to_string(), serde_json::json!(format!("{:?}", incident.state)))
        .with_detail("severity".to_string(), serde_json::json!(format!("{:?}", incident.severity)));

        self.add_audit_entry(entry).await?;

        Ok(())
    }

    /// Handle incident deletion event
    pub async fn on_incident_deleted(&self, incident_id: Uuid, deleted_by: &str) -> Result<()> {
        tracing::info!(
            incident_id = %incident_id,
            deleted_by = deleted_by,
            "Handling incident deletion for governance"
        );

        let entry = AuditEntry::new(
            incident_id,
            "incident_deleted".to_string(),
            deleted_by.to_string(),
        );

        self.add_audit_entry(entry).await?;

        Ok(())
    }

    /// Handle escalation event
    pub async fn on_incident_escalated(&self, incident: &Incident, escalated_to: &str) -> Result<()> {
        tracing::info!(
            incident_id = %incident.id,
            escalated_to = escalated_to,
            "Handling incident escalation for governance"
        );

        let entry = AuditEntry::new(
            incident.id,
            "incident_escalated".to_string(),
            "escalation_system".to_string(),
        )
        .with_detail("escalated_to".to_string(), serde_json::json!(escalated_to))
        .with_detail("severity".to_string(), serde_json::json!(format!("{:?}", incident.severity)));

        self.add_audit_entry(entry).await?;

        Ok(())
    }

    /// Add audit entry
    async fn add_audit_entry(&self, entry: AuditEntry) -> Result<()> {
        let mut entries = self.audit_entries.write().await;
        entries
            .entry(entry.incident_id)
            .or_insert_with(Vec::new)
            .push(entry);
        Ok(())
    }

    /// Get audit entries for an incident
    pub async fn get_audit_trail(&self, incident_id: Uuid) -> Result<Vec<AuditEntry>> {
        let entries = self.audit_entries.read().await;
        Ok(entries.get(&incident_id).cloned().unwrap_or_default())
    }

    /// Update governance metrics
    async fn update_metrics(&self, is_compliant: bool, violations: &[PolicyViolation]) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        metrics.total_checks += 1;

        if is_compliant {
            metrics.passed_checks += 1;
        } else {
            metrics.failed_checks += 1;
            metrics.total_violations += violations.len() as u64;

            // Update violations by severity
            for violation in violations {
                let severity_key = format!("{:?}", violation.severity);
                *metrics.violations_by_severity.entry(severity_key).or_insert(0) += 1;

                let framework_key = format!("{:?}", violation.framework);
                *metrics.violations_by_framework.entry(framework_key).or_insert(0) += 1;
            }
        }

        metrics.last_updated = Utc::now();
        Ok(())
    }

    /// Get current governance metrics
    pub async fn get_metrics(&self) -> Result<GovernanceMetrics> {
        let metrics = self.metrics.read().await;
        Ok(metrics.clone())
    }

    /// Determine which compliance frameworks apply to this incident
    fn determine_required_frameworks(&self, incident: &Incident) -> Vec<ComplianceFramework> {
        let mut frameworks = Vec::new();

        // Check labels for compliance indicators
        let labels = &incident.labels;
        if labels.contains_key("gdpr") || labels.contains_key("pii") {
            frameworks.push(ComplianceFramework::GDPR);
        }
        if labels.contains_key("hipaa") || labels.contains_key("phi") {
            frameworks.push(ComplianceFramework::HIPAA);
        }
        if labels.contains_key("pci") || labels.contains_key("payment") {
            frameworks.push(ComplianceFramework::PCI);
        }
        if labels.contains_key("soc2") {
            frameworks.push(ComplianceFramework::SOC2);
        }
        if labels.contains_key("iso27001") {
            frameworks.push(ComplianceFramework::ISO27001);
        }

        // Default to SOC2 for all incidents (general security control framework)
        if frameworks.is_empty() {
            frameworks.push(ComplianceFramework::SOC2);
        }

        frameworks
    }

    /// Extract compliance data from incident
    fn extract_compliance_data(&self, incident: &Incident) -> Result<ComplianceData> {
        let classification = if incident.labels.contains_key("restricted") {
            DataClassification::Restricted
        } else if incident.labels.contains_key("confidential") {
            DataClassification::Confidential
        } else {
            DataClassification::Internal
        };

        // Extract personal data info
        let personal_data = if incident.labels.contains_key("pii") {
            Some(PersonalDataInfo {
                contains_pii: true,
                data_subjects_affected: None,
                data_types: vec!["incident_metadata".to_string()],
                processing_purpose: "incident investigation".to_string(),
            })
        } else {
            None
        };

        // Extract health data info
        let health_data = if incident.labels.contains_key("phi") {
            Some(HealthDataInfo {
                contains_phi: true,
                covered_entities_affected: vec![],
                data_types: vec!["incident_metadata".to_string()],
            })
        } else {
            None
        };

        // Extract payment data info
        let payment_data = if incident.labels.contains_key("payment") {
            Some(PaymentDataInfo {
                contains_cardholder_data: true,
                card_brands_affected: vec![],
                transaction_count: None,
            })
        } else {
            None
        };

        // Build metadata from incident
        let mut metadata = HashMap::new();
        metadata.insert("incident_id".to_string(), serde_json::json!(incident.id.to_string()));
        metadata.insert("source".to_string(), serde_json::json!(&incident.source));

        // Check for compliance-related metadata
        let labels = &incident.labels;
        if let Some(notification_sent) = labels.get("notification_sent") {
            metadata.insert("notification_sent".to_string(), serde_json::json!(notification_sent == "true"));
        }
        if let Some(encrypted) = labels.get("data_encrypted") {
            metadata.insert("data_encrypted".to_string(), serde_json::json!(encrypted == "true"));
        }
        if let Some(access_controls) = labels.get("access_controls") {
            metadata.insert("access_controls_enabled".to_string(), serde_json::json!(access_controls == "enabled"));
        }

        Ok(ComplianceData {
            classification,
            personal_data,
            health_data,
            payment_data,
            metadata,
        })
    }

    /// Generate audit report
    pub async fn generate_audit_report(
        &self,
        incident_id: Uuid,
        report_type: AuditReportType,
    ) -> Result<AuditReport> {
        let entries = self.get_audit_trail(incident_id).await?;

        let mut report = AuditReport::new(
            incident_id,
            report_type,
            Utc::now(),
            Utc::now(),
        );

        // Count compliance checks and violations from audit entries
        for entry in &entries {
            if entry.action == "compliance_check" {
                report.total_checks += 1;
            } else if entry.action == "compliance_violation" {
                // Note: In a real implementation, we'd reconstruct PolicyViolation from audit entry
                report.total_checks += 1;
            }
        }

        report.compliant_checks = report.total_checks.saturating_sub(report.violations.len() as u64);
        report.summary = format!(
            "Audit report for incident {}: {} checks performed, {} violations found ({:.1}% compliance)",
            incident_id,
            report.total_checks,
            report.violations.len(),
            report.compliance_rate()
        );

        Ok(report)
    }
}

impl Default for ComplianceEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};

    #[tokio::test]
    async fn test_incident_created_handler() {
        let handler = ComplianceEventHandler::new();

        let mut incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P1,
            IncidentType::Security,
        );

        let mut labels = HashMap::new();
        labels.insert("pii".to_string(), "true".to_string());
        incident.labels = Some(labels);

        handler.on_incident_created(&incident).await.unwrap();

        let entries = handler.get_audit_trail(incident.id).await.unwrap();
        assert!(!entries.is_empty());
        assert_eq!(entries[0].action, "incident_created");
    }

    #[tokio::test]
    async fn test_incident_updated_handler() {
        let handler = ComplianceEventHandler::new();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Description".to_string(),
            Severity::P2,
            IncidentType::Application,
        );

        handler.on_incident_updated(&incident, "user@example.com").await.unwrap();

        let entries = handler.get_audit_trail(incident.id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "incident_updated");
        assert_eq!(entries[0].actor, "user@example.com");
    }

    #[tokio::test]
    async fn test_incident_deleted_handler() {
        let handler = ComplianceEventHandler::new();
        let incident_id = Uuid::new_v4();

        handler.on_incident_deleted(incident_id, "admin@example.com").await.unwrap();

        let entries = handler.get_audit_trail(incident_id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "incident_deleted");
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let handler = ComplianceEventHandler::new();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            Severity::P3,
            IncidentType::Infrastructure,
        );

        handler.on_incident_created(&incident).await.unwrap();

        let metrics = handler.get_metrics().await.unwrap();
        assert!(metrics.total_checks > 0);
    }

    #[tokio::test]
    async fn test_determine_frameworks() {
        let handler = ComplianceEventHandler::new();

        let mut incident = Incident::new(
            "test".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            Severity::P1,
            IncidentType::Security,
        );

        let mut labels = HashMap::new();
        labels.insert("gdpr".to_string(), "true".to_string());
        labels.insert("pii".to_string(), "true".to_string());
        incident.labels = Some(labels);

        let frameworks = handler.determine_required_frameworks(&incident);
        assert!(frameworks.contains(&ComplianceFramework::GDPR));
    }
}
