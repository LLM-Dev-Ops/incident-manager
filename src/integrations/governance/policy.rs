use crate::error::Result;
use super::models::*;
use std::collections::HashMap;

/// Policy engine for interpreting and validating compliance policies
pub struct PolicyEngine {
    policies: HashMap<ComplianceFramework, Vec<CompliancePolicy>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            policies: HashMap::new(),
        };
        engine.load_default_policies();
        engine
    }

    /// Load default compliance policies
    fn load_default_policies(&mut self) {
        // GDPR Policies
        self.add_policy(ComplianceFramework::GDPR, CompliancePolicy {
            id: "GDPR-001".to_string(),
            name: "Data Breach Notification".to_string(),
            description: "Notify authorities within 72 hours of breach discovery".to_string(),
            severity: ViolationSeverity::Critical,
            check_fn: PolicyCheck::NotificationWindow,
        });

        self.add_policy(ComplianceFramework::GDPR, CompliancePolicy {
            id: "GDPR-002".to_string(),
            name: "Data Subject Rights".to_string(),
            description: "Ensure data subject rights are protected".to_string(),
            severity: ViolationSeverity::High,
            check_fn: PolicyCheck::DataSubjectRights,
        });

        self.add_policy(ComplianceFramework::GDPR, CompliancePolicy {
            id: "GDPR-003".to_string(),
            name: "Data Minimization".to_string(),
            description: "Collect only necessary personal data".to_string(),
            severity: ViolationSeverity::Medium,
            check_fn: PolicyCheck::DataMinimization,
        });

        // HIPAA Policies
        self.add_policy(ComplianceFramework::HIPAA, CompliancePolicy {
            id: "HIPAA-001".to_string(),
            name: "PHI Protection".to_string(),
            description: "Protected Health Information must be encrypted".to_string(),
            severity: ViolationSeverity::Critical,
            check_fn: PolicyCheck::PhiProtection,
        });

        self.add_policy(ComplianceFramework::HIPAA, CompliancePolicy {
            id: "HIPAA-002".to_string(),
            name: "Access Controls".to_string(),
            description: "Implement strict access controls for PHI".to_string(),
            severity: ViolationSeverity::High,
            check_fn: PolicyCheck::AccessControls,
        });

        // SOC2 Policies
        self.add_policy(ComplianceFramework::SOC2, CompliancePolicy {
            id: "SOC2-001".to_string(),
            name: "Incident Response".to_string(),
            description: "Document and respond to security incidents".to_string(),
            severity: ViolationSeverity::High,
            check_fn: PolicyCheck::IncidentResponse,
        });

        self.add_policy(ComplianceFramework::SOC2, CompliancePolicy {
            id: "SOC2-002".to_string(),
            name: "Audit Logging".to_string(),
            description: "Maintain comprehensive audit logs".to_string(),
            severity: ViolationSeverity::Medium,
            check_fn: PolicyCheck::AuditLogging,
        });

        // PCI Policies
        self.add_policy(ComplianceFramework::PCI, CompliancePolicy {
            id: "PCI-001".to_string(),
            name: "Cardholder Data Protection".to_string(),
            description: "Protect stored cardholder data".to_string(),
            severity: ViolationSeverity::Critical,
            check_fn: PolicyCheck::CardholderDataProtection,
        });

        // ISO27001 Policies
        self.add_policy(ComplianceFramework::ISO27001, CompliancePolicy {
            id: "ISO27001-001".to_string(),
            name: "Information Security Controls".to_string(),
            description: "Implement appropriate security controls".to_string(),
            severity: ViolationSeverity::High,
            check_fn: PolicyCheck::SecurityControls,
        });
    }

    fn add_policy(&mut self, framework: ComplianceFramework, policy: CompliancePolicy) {
        self.policies
            .entry(framework)
            .or_insert_with(Vec::new)
            .push(policy);
    }

    /// Validate compliance data against policies
    pub fn validate(&self, request: &ComplianceRequest) -> Result<Vec<PolicyViolation>> {
        let mut violations = Vec::new();

        for framework in &request.frameworks {
            if let Some(policies) = self.policies.get(framework) {
                for policy in policies {
                    if let Some(violation) = self.check_policy(policy, framework, &request.data)? {
                        violations.push(violation);
                    }
                }
            }
        }

        Ok(violations)
    }

    fn check_policy(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        match &policy.check_fn {
            PolicyCheck::NotificationWindow => {
                self.check_notification_window(policy, framework, data)
            }
            PolicyCheck::DataSubjectRights => {
                self.check_data_subject_rights(policy, framework, data)
            }
            PolicyCheck::DataMinimization => {
                self.check_data_minimization(policy, framework, data)
            }
            PolicyCheck::PhiProtection => {
                self.check_phi_protection(policy, framework, data)
            }
            PolicyCheck::AccessControls => {
                self.check_access_controls(policy, framework, data)
            }
            PolicyCheck::IncidentResponse => {
                self.check_incident_response(policy, framework, data)
            }
            PolicyCheck::AuditLogging => {
                self.check_audit_logging(policy, framework, data)
            }
            PolicyCheck::CardholderDataProtection => {
                self.check_cardholder_data_protection(policy, framework, data)
            }
            PolicyCheck::SecurityControls => {
                self.check_security_controls(policy, framework, data)
            }
        }
    }

    fn check_notification_window(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        // Check if notification window requirement is met
        if data.classification == DataClassification::Restricted {
            if let Some(notification_sent) = data.metadata.get("notification_sent") {
                if !notification_sent.as_bool().unwrap_or(false) {
                    return Ok(Some(PolicyViolation::new(
                        framework.clone(),
                        policy.id.clone(),
                        policy.name.clone(),
                        policy.severity.clone(),
                        "Data breach notification not sent within required timeframe".to_string(),
                        "Send breach notification to authorities immediately".to_string(),
                    )));
                }
            }
        }
        Ok(None)
    }

    fn check_data_subject_rights(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        if let Some(personal_data) = &data.personal_data {
            if personal_data.contains_pii {
                if let Some(rights_protected) = data.metadata.get("data_subject_rights_protected") {
                    if !rights_protected.as_bool().unwrap_or(false) {
                        return Ok(Some(PolicyViolation::new(
                            framework.clone(),
                            policy.id.clone(),
                            policy.name.clone(),
                            policy.severity.clone(),
                            "Data subject rights not adequately protected".to_string(),
                            "Implement data subject access request (DSAR) procedures".to_string(),
                        )));
                    }
                }
            }
        }
        Ok(None)
    }

    fn check_data_minimization(
        &self,
        _policy: &CompliancePolicy,
        _framework: &ComplianceFramework,
        _data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        // Data minimization is typically enforced at design time
        Ok(None)
    }

    fn check_phi_protection(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        if let Some(health_data) = &data.health_data {
            if health_data.contains_phi {
                if let Some(encrypted) = data.metadata.get("data_encrypted") {
                    if !encrypted.as_bool().unwrap_or(false) {
                        return Ok(Some(PolicyViolation::new(
                            framework.clone(),
                            policy.id.clone(),
                            policy.name.clone(),
                            policy.severity.clone(),
                            "Protected Health Information not encrypted".to_string(),
                            "Enable encryption for all PHI storage and transmission".to_string(),
                        )));
                    }
                }
            }
        }
        Ok(None)
    }

    fn check_access_controls(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        if data.classification == DataClassification::Restricted {
            if let Some(access_controls) = data.metadata.get("access_controls_enabled") {
                if !access_controls.as_bool().unwrap_or(false) {
                    return Ok(Some(PolicyViolation::new(
                        framework.clone(),
                        policy.id.clone(),
                        policy.name.clone(),
                        policy.severity.clone(),
                        "Access controls not properly configured".to_string(),
                        "Implement role-based access controls (RBAC)".to_string(),
                    )));
                }
            }
        }
        Ok(None)
    }

    fn check_incident_response(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        if let Some(response_plan) = data.metadata.get("incident_response_plan_executed") {
            if !response_plan.as_bool().unwrap_or(false) {
                return Ok(Some(PolicyViolation::new(
                    framework.clone(),
                    policy.id.clone(),
                    policy.name.clone(),
                    policy.severity.clone(),
                    "Incident response plan not executed".to_string(),
                    "Follow documented incident response procedures".to_string(),
                )));
            }
        }
        Ok(None)
    }

    fn check_audit_logging(
        &self,
        _policy: &CompliancePolicy,
        _framework: &ComplianceFramework,
        _data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        // Audit logging is typically enforced at infrastructure level
        Ok(None)
    }

    fn check_cardholder_data_protection(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        if let Some(payment_data) = &data.payment_data {
            if payment_data.contains_cardholder_data {
                if let Some(pci_compliant) = data.metadata.get("pci_compliant_storage") {
                    if !pci_compliant.as_bool().unwrap_or(false) {
                        return Ok(Some(PolicyViolation::new(
                            framework.clone(),
                            policy.id.clone(),
                            policy.name.clone(),
                            policy.severity.clone(),
                            "Cardholder data not stored in PCI-compliant manner".to_string(),
                            "Use PCI-DSS compliant storage and tokenization".to_string(),
                        )));
                    }
                }
            }
        }
        Ok(None)
    }

    fn check_security_controls(
        &self,
        policy: &CompliancePolicy,
        framework: &ComplianceFramework,
        data: &ComplianceData,
    ) -> Result<Option<PolicyViolation>> {
        if data.classification != DataClassification::Public {
            if let Some(controls) = data.metadata.get("security_controls_implemented") {
                if !controls.as_bool().unwrap_or(false) {
                    return Ok(Some(PolicyViolation::new(
                        framework.clone(),
                        policy.id.clone(),
                        policy.name.clone(),
                        policy.severity.clone(),
                        "Required security controls not implemented".to_string(),
                        "Implement ISO27001 Annex A controls".to_string(),
                    )));
                }
            }
        }
        Ok(None)
    }

    /// Get all policies for a framework
    pub fn get_policies(&self, framework: &ComplianceFramework) -> Vec<&CompliancePolicy> {
        self.policies
            .get(framework)
            .map(|policies| policies.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Compliance policy definition
#[derive(Debug, Clone)]
pub struct CompliancePolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: ViolationSeverity,
    pub check_fn: PolicyCheck,
}

/// Policy check function types
#[derive(Debug, Clone)]
pub enum PolicyCheck {
    NotificationWindow,
    DataSubjectRights,
    DataMinimization,
    PhiProtection,
    AccessControls,
    IncidentResponse,
    AuditLogging,
    CardholderDataProtection,
    SecurityControls,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_policy_engine_creation() {
        let engine = PolicyEngine::new();

        let gdpr_policies = engine.get_policies(&ComplianceFramework::GDPR);
        assert!(!gdpr_policies.is_empty());

        let hipaa_policies = engine.get_policies(&ComplianceFramework::HIPAA);
        assert!(!hipaa_policies.is_empty());
    }

    #[test]
    fn test_notification_window_violation() {
        let engine = PolicyEngine::new();

        let mut metadata = HashMap::new();
        metadata.insert("notification_sent".to_string(), serde_json::json!(false));

        let data = ComplianceData {
            classification: DataClassification::Restricted,
            personal_data: None,
            health_data: None,
            payment_data: None,
            metadata,
        };

        let request = ComplianceRequest::new(
            uuid::Uuid::new_v4(),
            vec![ComplianceFramework::GDPR],
            data,
        );

        let violations = engine.validate(&request).unwrap();
        assert!(!violations.is_empty());
        assert_eq!(violations[0].policy_id, "GDPR-001");
    }

    #[test]
    fn test_phi_protection_violation() {
        let engine = PolicyEngine::new();

        let mut metadata = HashMap::new();
        metadata.insert("data_encrypted".to_string(), serde_json::json!(false));

        let data = ComplianceData {
            classification: DataClassification::Confidential,
            personal_data: None,
            health_data: Some(HealthDataInfo {
                contains_phi: true,
                covered_entities_affected: vec!["Hospital A".to_string()],
                data_types: vec!["medical_records".to_string()],
            }),
            payment_data: None,
            metadata,
        };

        let request = ComplianceRequest::new(
            uuid::Uuid::new_v4(),
            vec![ComplianceFramework::HIPAA],
            data,
        );

        let violations = engine.validate(&request).unwrap();
        assert!(!violations.is_empty());
        assert_eq!(violations[0].policy_id, "HIPAA-001");
    }

    #[test]
    fn test_no_violations_when_compliant() {
        let engine = PolicyEngine::new();

        let mut metadata = HashMap::new();
        metadata.insert("notification_sent".to_string(), serde_json::json!(true));
        metadata.insert("data_encrypted".to_string(), serde_json::json!(true));

        let data = ComplianceData {
            classification: DataClassification::Internal,
            personal_data: None,
            health_data: None,
            payment_data: None,
            metadata,
        };

        let request = ComplianceRequest::new(
            uuid::Uuid::new_v4(),
            vec![ComplianceFramework::GDPR, ComplianceFramework::HIPAA],
            data,
        );

        let violations = engine.validate(&request).unwrap();
        assert!(violations.is_empty());
    }
}
