use crate::error::{AppError, Result};
use crate::models::{Incident, Notification, NotificationStatus};
use chrono::Utc;
use lettre::message::{header, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use tracing::{error, info};

/// Email notification sender
#[derive(Clone)]
pub struct EmailSender {
    pub(crate) smtp_server: String,
    pub(crate) smtp_port: u16,
    pub(crate) smtp_username: Option<String>,
    pub(crate) smtp_password: Option<String>,
    pub(crate) from_email: String,
    pub(crate) from_name: Option<String>,
    pub(crate) use_tls: bool,
}

impl EmailSender {
    /// Create a new email sender
    pub fn new(
        smtp_server: String,
        smtp_port: u16,
        smtp_username: Option<String>,
        smtp_password: Option<String>,
        from_email: String,
        from_name: Option<String>,
        use_tls: bool,
    ) -> Result<Self> {
        if smtp_server.is_empty() {
            return Err(AppError::Configuration(
                "SMTP server cannot be empty".to_string(),
            ));
        }

        if from_email.is_empty() {
            return Err(AppError::Configuration(
                "From email cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            smtp_server,
            smtp_port,
            smtp_username,
            smtp_password,
            from_email,
            from_name,
            use_tls,
        })
    }

    /// Send an email notification
    pub async fn send(&self, notification: &mut Notification, incident: &Incident) -> Result<()> {
        // Extract email details from notification
        let (recipients, subject, body) = match &notification.channel {
            crate::models::NotificationChannel::Email { to, subject, body } => {
                (to.clone(), subject.clone(), body.clone())
            }
            _ => {
                return Err(AppError::Validation(
                    "Invalid notification channel type for Email".to_string(),
                ))
            }
        };

        if recipients.is_empty() {
            return Err(AppError::Validation("No recipients specified".to_string()));
        }

        notification.status = NotificationStatus::Sending;

        // Build email message
        let message = self.build_email_message(incident, &recipients, &subject, &body)?;

        // Send email
        let result = tokio::task::spawn_blocking({
            let smtp_server = self.smtp_server.clone();
            let smtp_port = self.smtp_port;
            let username = self.smtp_username.clone();
            let password = self.smtp_password.clone();
            let use_tls = self.use_tls;

            move || {
                let mut transport_builder = SmtpTransport::relay(&smtp_server)
                    .map_err(|e| AppError::Configuration(format!("Invalid SMTP server: {}", e)))?;

                // Add authentication if provided
                if let (Some(user), Some(pass)) = (username, password) {
                    transport_builder = transport_builder.credentials(Credentials::new(user, pass));
                }

                // Configure TLS
                if !use_tls {
                    transport_builder = transport_builder.port(smtp_port);
                }

                let mailer = transport_builder.build();

                // Send the email
                mailer
                    .send(&message)
                    .map_err(|e| AppError::Internal(format!("Failed to send email: {}", e)))?;

                Ok::<(), AppError>(())
            }
        })
        .await
        .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?;

        match result {
            Ok(_) => {
                notification.status = NotificationStatus::Sent;
                notification.sent_at = Some(Utc::now());
                info!(
                    notification_id = %notification.id,
                    incident_id = %incident.id,
                    recipients = ?recipients,
                    "Email notification sent successfully"
                );
                Ok(())
            }
            Err(e) => {
                notification.status = NotificationStatus::Failed;
                notification.error = Some(e.to_string());
                error!(
                    notification_id = %notification.id,
                    incident_id = %incident.id,
                    error = %e,
                    "Failed to send email notification"
                );
                Err(e)
            }
        }
    }

    /// Build email message with HTML and plain text parts
    fn build_email_message(
        &self,
        incident: &Incident,
        recipients: &[String],
        subject: &str,
        body: &str,
    ) -> Result<Message> {
        // Build from address
        let from_mailbox = if let Some(name) = &self.from_name {
            format!("{} <{}>", name, self.from_email)
        } else {
            self.from_email.clone()
        };

        let from = from_mailbox
            .parse()
            .map_err(|e| AppError::Configuration(format!("Invalid from address: {}", e)))?;

        // Parse recipients
        let to_addresses: Result<Vec<_>> = recipients
            .iter()
            .map(|addr| {
                addr.parse().map_err(|e| {
                    AppError::Validation(format!("Invalid recipient address '{}': {}", addr, e))
                })
            })
            .collect();

        let to_addresses = to_addresses?;

        // Build plain text version
        let plain_text = self.build_plain_text(incident, body);

        // Build HTML version
        let html_text = self.build_html(incident, body);

        // Build message with multiple recipients
        let mut message_builder = Message::builder()
            .from(from)
            .subject(subject)
            .header(header::ContentType::TEXT_HTML);

        // Add all recipients
        for addr in to_addresses {
            message_builder = message_builder.to(addr);
        }

        // Create multipart message with both plain text and HTML
        let message = message_builder
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN)
                            .body(plain_text),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(html_text),
                    ),
            )
            .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        Ok(message)
    }

    /// Build plain text email body
    fn build_plain_text(&self, incident: &Incident, body: &str) -> String {
        format!(
            r#"
{}

===========================================
INCIDENT DETAILS
===========================================

Incident ID: {}
Title: {}
Description: {}

Severity: {:?}
State: {:?}
Type: {:?}
Source: {}

Created: {}
Updated: {}

Affected Resources:
{}

Assignees:
{}

---
This notification was sent by LLM Incident Manager
"#,
            body,
            incident.id,
            incident.title,
            incident.description,
            incident.severity,
            incident.state,
            incident.incident_type,
            incident.source,
            incident.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            incident.updated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            if incident.affected_resources.is_empty() {
                "None".to_string()
            } else {
                incident.affected_resources.join(", ")
            },
            if incident.assignees.is_empty() {
                "Unassigned".to_string()
            } else {
                incident.assignees.join(", ")
            }
        )
    }

    /// Build HTML email body
    fn build_html(&self, incident: &Incident, body: &str) -> String {
        let severity_color = match incident.severity {
            crate::models::Severity::P0 => "#d00000",
            crate::models::Severity::P1 => "#ff6b35",
            crate::models::Severity::P2 => "#f7b801",
            crate::models::Severity::P3 => "#0077b6",
            crate::models::Severity::P4 => "#00b4d8",
        };

        let severity_badge = format!(
            r#"<span style="background-color: {}; color: white; padding: 4px 8px; border-radius: 4px; font-weight: bold;">{:?}</span>"#,
            severity_color, incident.severity
        );

        format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            background-color: #f8f9fa;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
        }}
        .incident-title {{
            font-size: 24px;
            font-weight: bold;
            margin-bottom: 10px;
            color: #1a1a1a;
        }}
        .message {{
            background-color: #fff;
            border-left: 4px solid {};
            padding: 15px;
            margin-bottom: 20px;
        }}
        .details {{
            background-color: #f8f9fa;
            padding: 15px;
            border-radius: 8px;
        }}
        .detail-row {{
            display: flex;
            padding: 8px 0;
            border-bottom: 1px solid #e9ecef;
        }}
        .detail-label {{
            font-weight: bold;
            width: 150px;
            color: #6c757d;
        }}
        .detail-value {{
            color: #1a1a1a;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
            text-align: center;
        }}
        .code {{
            background-color: #f1f3f5;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Monaco', 'Courier New', monospace;
            font-size: 13px;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div class="incident-title">{}</div>
        <div>{}</div>
    </div>

    <div class="message">
        {}
    </div>

    <div class="details">
        <h3>Incident Details</h3>

        <div class="detail-row">
            <div class="detail-label">Incident ID:</div>
            <div class="detail-value"><span class="code">{}</span></div>
        </div>

        <div class="detail-row">
            <div class="detail-label">Severity:</div>
            <div class="detail-value">{}</div>
        </div>

        <div class="detail-row">
            <div class="detail-label">State:</div>
            <div class="detail-value">{:?}</div>
        </div>

        <div class="detail-row">
            <div class="detail-label">Type:</div>
            <div class="detail-value">{:?}</div>
        </div>

        <div class="detail-row">
            <div class="detail-label">Source:</div>
            <div class="detail-value">{}</div>
        </div>

        <div class="detail-row">
            <div class="detail-label">Created:</div>
            <div class="detail-value">{}</div>
        </div>

        <div class="detail-row">
            <div class="detail-label">Affected Resources:</div>
            <div class="detail-value">{}</div>
        </div>

        <div class="detail-row">
            <div class="detail-label">Assignees:</div>
            <div class="detail-value">{}</div>
        </div>
    </div>

    <div class="footer">
        This notification was sent by <strong>LLM Incident Manager</strong>
    </div>
</body>
</html>
"#,
            severity_color,
            incident.title,
            severity_badge,
            body,
            incident.id,
            severity_badge,
            incident.state,
            incident.incident_type,
            incident.source,
            incident.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            if incident.affected_resources.is_empty() {
                "None".to_string()
            } else {
                incident.affected_resources.join(", ")
            },
            if incident.assignees.is_empty() {
                "Unassigned".to_string()
            } else {
                incident.assignees.join(", ")
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentState, IncidentType, Severity};

    #[test]
    fn test_email_sender_creation() {
        let sender = EmailSender::new(
            "smtp.example.com".to_string(),
            587,
            Some("user".to_string()),
            Some("pass".to_string()),
            "incidents@example.com".to_string(),
            Some("Incident Manager".to_string()),
            true,
        );
        assert!(sender.is_ok());
    }

    #[test]
    fn test_email_sender_validation() {
        let result = EmailSender::new(
            "".to_string(),
            587,
            None,
            None,
            "test@example.com".to_string(),
            None,
            true,
        );
        assert!(result.is_err());

        let result = EmailSender::new(
            "smtp.example.com".to_string(),
            587,
            None,
            None,
            "".to_string(),
            None,
            true,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_build_plain_text() {
        let sender = EmailSender::new(
            "smtp.example.com".to_string(),
            587,
            None,
            None,
            "test@example.com".to_string(),
            None,
            true,
        )
        .unwrap();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        );

        let plain_text = sender.build_plain_text(&incident, "Test message");

        assert!(plain_text.contains("Test message"));
        assert!(plain_text.contains("Test Incident"));
        assert!(plain_text.contains(&incident.id.to_string()));
        assert!(plain_text.contains("P1"));
    }

    #[test]
    fn test_build_html() {
        let sender = EmailSender::new(
            "smtp.example.com".to_string(),
            587,
            None,
            None,
            "test@example.com".to_string(),
            Some("Incident Manager".to_string()),
            true,
        )
        .unwrap();

        let incident = Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P0,
            IncidentType::Security,
        );

        let html = sender.build_html(&incident, "Test message");

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Test message"));
        assert!(html.contains("Test Incident"));
        assert!(html.contains(&incident.id.to_string()));
        assert!(html.contains("#d00000")); // P0 color
    }
}
