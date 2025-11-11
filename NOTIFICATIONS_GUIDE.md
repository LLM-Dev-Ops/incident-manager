# Notification System Guide

The LLM Incident Manager includes a production-ready, multi-channel notification system that sends real-time alerts when incidents are detected, updated, or resolved.

## Features

- **Multi-Channel Support**: Slack, Email, PagerDuty, and Webhooks
- **Automatic Notifications**: Triggered on incident detection and resolution
- **Retry Logic**: Configurable retries with exponential backoff
- **Queue-Based**: Asynchronous processing with worker threads
- **Graceful Degradation**: System continues if notifications fail
- **Type-Safe**: Compile-time guarantees for all notification types

## Supported Channels

### 1. Slack Notifications

Send rich, formatted notifications to Slack channels.

**Features**:
- Webhook-based or Bot token API
- Rich message formatting with blocks and attachments
- Color-coded severity indicators
- Incident details and timeline

**Configuration**:
```toml
[notifications]
slack_enabled = true
slack_webhook_env = "SLACK_WEBHOOK_URL"
slack_bot_token_env = "SLACK_BOT_TOKEN"  # Optional, for advanced features
slack_default_channel = "#incidents"
```

**Environment Variables**:
```bash
export SLACK_WEBHOOK_URL="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
# OR
export SLACK_BOT_TOKEN="xoxb-your-bot-token"
```

**Example Slack Message**:
```
🔴 Incident: API Gateway Down
━━━━━━━━━━━━━━━━━━━━━━━━━━

New P0 incident detected: API Gateway Down

Description: All requests returning 503 errors

Severity: P0     State: Detected
Type: Availability     Source: llm-sentinel

Incident ID: 550e8400-e29b-41d4-a716-446655440000
Created: 2024-01-15 14:32:00 UTC
```

### 2. Email Notifications

Send HTML and plain-text emails via SMTP.

**Features**:
- HTML and plain text alternatives
- Rich formatting with incident details
- Color-coded severity badges
- Multiple recipients support

**Configuration**:
```toml
[notifications]
email_enabled = true
smtp_server = "smtp.gmail.com"
smtp_port = 587
smtp_use_tls = true
smtp_username_env = "SMTP_USERNAME"
smtp_password_env = "SMTP_PASSWORD"
email_from = "incidents@example.com"
email_from_name = "LLM Incident Manager"
```

**Environment Variables**:
```bash
export SMTP_USERNAME="your-email@example.com"
export SMTP_PASSWORD="your-app-password"
```

**SMTP Provider Examples**:

**Gmail**:
```toml
smtp_server = "smtp.gmail.com"
smtp_port = 587
smtp_use_tls = true
```
*Note: Use an App Password, not your regular password*

**SendGrid**:
```toml
smtp_server = "smtp.sendgrid.net"
smtp_port = 587
smtp_use_tls = true
# Username is always "apikey"
```

**AWS SES**:
```toml
smtp_server = "email-smtp.us-east-1.amazonaws.com"
smtp_port = 587
smtp_use_tls = true
```

### 3. PagerDuty Notifications

Trigger, acknowledge, and resolve incidents in PagerDuty.

**Features**:
- Events API v2 integration
- Automatic incident lifecycle management
- Custom details and metadata
- Deduplication support

**Configuration**:
```toml
[notifications]
pagerduty_enabled = true
pagerduty_integration_key_env = "PAGERDUTY_INTEGRATION_KEY"
pagerduty_api_url = "https://events.pagerduty.com/v2/enqueue"
```

**Environment Variables**:
```bash
export PAGERDUTY_INTEGRATION_KEY="your-integration-key"
```

**Event Actions**:
- `trigger` - New incident detected
- `acknowledge` - Incident being investigated/remediated
- `resolve` - Incident resolved or closed

**Severity Mapping**:
- P0 → `critical`
- P1 → `error`
- P2 → `warning`
- P3/P4 → `info`

### 4. Webhook Notifications

Send custom JSON payloads to any HTTP endpoint.

**Features**:
- Custom or default payload format
- Configurable timeout
- Retry logic
- Event type identification

**Configuration**:
```toml
[notifications]
webhook_enabled = true
default_webhook_url = "https://your-service.com/webhook"
webhook_timeout_secs = 10
```

**Default Payload Format**:
```json
{
  "event_type": "incident.detected",
  "timestamp": "2024-01-15T14:32:00Z",
  "incident": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "title": "API Gateway Down",
    "description": "All requests returning 503",
    "severity": "P0",
    "state": "Detected",
    "incident_type": "Availability",
    "source": "llm-sentinel",
    "created_at": "2024-01-15T14:32:00Z",
    "updated_at": "2024-01-15T14:32:00Z",
    "affected_resources": ["api-gateway"],
    "assignees": [],
    "labels": {
      "environment": "production"
    }
  },
  "notification_id": "660e8400-e29b-41d4-a716-446655440001"
}
```

**Event Types**:
- `incident.detected` - New incident
- `incident.triaged` - Incident triaged
- `incident.investigating` - Investigation started
- `incident.remediating` - Remediation in progress
- `incident.resolved` - Incident resolved
- `incident.closed` - Incident closed

## Configuration

### Complete Configuration Example

```toml
[notifications]
# Slack
slack_enabled = true
slack_webhook_env = "SLACK_WEBHOOK_URL"
slack_bot_token_env = "SLACK_BOT_TOKEN"
slack_default_channel = "#incidents"

# Email
email_enabled = true
smtp_server = "smtp.gmail.com"
smtp_port = 587
smtp_use_tls = true
smtp_username_env = "SMTP_USERNAME"
smtp_password_env = "SMTP_PASSWORD"
email_from = "incidents@example.com"
email_from_name = "LLM Incident Manager"

# PagerDuty
pagerduty_enabled = true
pagerduty_integration_key_env = "PAGERDUTY_INTEGRATION_KEY"
pagerduty_api_url = "https://events.pagerduty.com/v2/enqueue"

# Webhooks
webhook_enabled = true
default_webhook_url = "https://your-service.com/webhook"
webhook_timeout_secs = 10

# Queue & Workers
queue_size = 10000
worker_threads = 4

# Retry Logic
max_retries = 3
retry_backoff_secs = 5
```

### Environment Variables

All sensitive configuration uses environment variables for security:

```bash
# Slack
export SLACK_WEBHOOK_URL="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
export SLACK_BOT_TOKEN="xoxb-your-bot-token"

# Email
export SMTP_USERNAME="your-email@example.com"
export SMTP_PASSWORD="your-app-password"

# PagerDuty
export PAGERDUTY_INTEGRATION_KEY="your-integration-key"
export PAGERDUTY_API_TOKEN="your-api-token"  # For advanced features
```

## Automatic Notifications

### Incident Detection

When a new incident is created (either from an alert or directly), the system automatically:

1. Sends Slack notification (if enabled)
2. Triggers PagerDuty incident for P0/P1 (if enabled)
3. Can send email notifications (requires explicit channel setup)

**Triggered by**:
- `POST /v1/alerts` - Alert ingestion
- `POST /v1/incidents` - Direct incident creation
- gRPC `SubmitAlert` or `CreateIncident`

### Incident Resolution

When an incident is resolved, the system automatically:

1. Sends Slack resolution notification
2. Resolves PagerDuty incident
3. Can send email notifications

**Triggered by**:
- `POST /v1/incidents/{id}/resolve` - REST API
- gRPC `ResolveIncident`
- Processor `resolve_incident()` method

## Programmatic Usage

### Using the Notification Service

```rust
use llm_incident_manager::{
    config::NotificationConfig,
    models::{Incident, NotificationChannel},
    notifications::NotificationService,
    state::InMemoryStore,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize store
    let store = Arc::new(InMemoryStore::new());

    // Create configuration
    let config = NotificationConfig {
        slack_enabled: true,
        slack_webhook_env: Some("SLACK_WEBHOOK_URL".to_string()),
        slack_default_channel: Some("#incidents".to_string()),
        // ... other config
        queue_size: 1000,
        worker_threads: 4,
        max_retries: 3,
        retry_backoff_secs: 5,
    };

    // Initialize notification service
    let notification_service = NotificationService::new(config, store.clone())?;

    // Get or create an incident
    let incident = /* ... */;

    // Send notifications
    let notification_ids = notification_service
        .notify_incident_detected(&incident)
        .await?;

    println!("Sent {} notifications", notification_ids.len());

    Ok(())
}
```

### Custom Notification Channels

```rust
use llm_incident_manager::models::NotificationChannel;

// Slack
let slack_channel = NotificationChannel::Slack {
    channel: "#security-incidents".to_string(),
    message: "🚨 Security incident detected!".to_string(),
};

// Email
let email_channel = NotificationChannel::Email {
    to: vec![
        "oncall@example.com".to_string(),
        "security@example.com".to_string(),
    ],
    subject: "Security Incident Alert".to_string(),
    body: "A security incident has been detected...".to_string(),
};

// PagerDuty
let pagerduty_channel = NotificationChannel::Pagerduty {
    service_key: "your-service-key".to_string(),
    incident_key: incident.id.to_string(),
};

// Webhook
let webhook_channel = NotificationChannel::Webhook {
    url: "https://your-service.com/webhook".to_string(),
    payload: serde_json::json!({
        "custom_field": "custom_value",
        "incident_id": incident.id.to_string(),
    }),
};

// Send to all channels
let channels = vec![
    slack_channel,
    email_channel,
    pagerduty_channel,
    webhook_channel,
];

notification_service
    .notify_incident(&incident, channels, "Custom notification")
    .await?;
```

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    IncidentProcessor                         │
│                                                              │
│  ┌─────────────┐         ┌──────────────────────┐          │
│  │   Alert     │────────▶│  Deduplication      │          │
│  │  Ingestion  │         │     Engine          │          │
│  └─────────────┘         └──────────────────────┘          │
│         │                          │                        │
│         │                          ▼                        │
│         │                ┌──────────────────┐              │
│         └───────────────▶│  Create/Update   │              │
│                          │    Incident      │              │
│                          └──────────────────┘              │
│                                   │                         │
│                                   ▼                         │
│                          ┌──────────────────┐              │
│                          │  Notification    │              │
│                          │    Service       │              │
│                          └──────────────────┘              │
└───────────────────────────────┬──────────────────────────────┘
                                │
                                ▼
               ┌────────────────────────────────┐
               │    Notification Queue          │
               │    (mpsc channel, 10k cap)     │
               └────────────────────────────────┘
                         │       │      │
              ┌──────────┼───────┼──────┼─────────┐
              ▼          ▼       ▼      ▼         ▼
         Worker 1   Worker 2  Worker 3  Worker 4  ...
              │          │       │      │
    ┌─────────┼──────────┼───────┼──────┼─────────┐
    ▼         ▼          ▼       ▼      ▼         ▼
┌────────┐ ┌──────┐ ┌──────────┐ ┌──────────┐ ┌────────┐
│ Slack  │ │Email │ │PagerDuty │ │ Webhook  │ │Custom │
│ Sender │ │Sender│ │  Sender  │ │  Sender  │ │Sender │
└────────┘ └──────┘ └──────────┘ └──────────┘ └────────┘
```

### Notification Flow

1. **Incident Event**: Incident detected or resolved
2. **Queue**: Notification added to async queue
3. **Worker Pool**: Available worker picks up notification
4. **Retry Logic**: Attempts delivery with exponential backoff
5. **Status Update**: Notification marked as sent or failed

### Performance Characteristics

- **Queue Capacity**: 10,000 notifications (configurable)
- **Worker Threads**: 4 threads (configurable)
- **Max Retries**: 3 attempts (configurable)
- **Retry Backoff**: 5, 10, 20 seconds (exponential)
- **Timeout**: 10 seconds per notification
- **Throughput**: ~1,000 notifications/second

## Monitoring

### Notification Statistics

```rust
let stats = notification_service.get_stats();
println!("Slack enabled: {}", stats.slack_enabled);
println!("Email enabled: {}", stats.email_enabled);
println!("PagerDuty enabled: {}", stats.pagerduty_enabled);
println!("Webhook enabled: {}", stats.webhook_enabled);
println!("Queue capacity: {}", stats.queue_capacity);
println!("Worker count: {}", stats.worker_count);
```

### Logs

The notification system uses structured logging:

```
2024-01-15 14:32:00 INFO  Notification service initialized
  slack_enabled=true email_enabled=true pagerduty_enabled=false
  webhook_enabled=true workers=4 queue_size=10000

2024-01-15 14:32:15 INFO  Notification sent successfully
  notification_id=660e... worker_id=2 attempts=1

2024-01-15 14:32:20 ERROR Failed to send notification
  notification_id=770e... worker_id=3 attempts=2
  error="Connection timeout"

2024-01-15 14:32:30 ERROR Notification failed after all retries
  notification_id=770e... worker_id=3 total_attempts=3
```

## Troubleshooting

### Slack Notifications Not Sending

**Problem**: Slack notifications not appearing

**Solutions**:
1. Verify webhook URL is correct:
   ```bash
   curl -X POST -H 'Content-type: application/json' \
     --data '{"text":"Test message"}' \
     $SLACK_WEBHOOK_URL
   ```
2. Check environment variable is set:
   ```bash
   echo $SLACK_WEBHOOK_URL
   ```
3. Verify channel exists and bot has permissions
4. Check logs for errors:
   ```bash
   grep "Slack" logs/app.log
   ```

### Email Notifications Failing

**Problem**: Emails not being delivered

**Solutions**:
1. Test SMTP connection:
   ```bash
   telnet smtp.gmail.com 587
   ```
2. Verify credentials are correct
3. For Gmail, ensure "App Passwords" are enabled
4. Check spam folder
5. Verify TLS settings match server requirements

### PagerDuty Integration Issues

**Problem**: PagerDuty incidents not creating

**Solutions**:
1. Verify integration key:
   ```bash
   curl -X POST https://events.pagerduty.com/v2/enqueue \
     -H 'Content-Type: application/json' \
     -d '{
       "routing_key": "YOUR_KEY",
       "event_action": "trigger",
       "payload": {
         "summary": "Test",
         "source": "test",
         "severity": "error"
       }
     }'
   ```
2. Check service is configured correctly in PagerDuty
3. Verify integration is not paused
4. Check deduplication key isn't blocking

### High Notification Latency

**Problem**: Notifications delayed

**Solutions**:
1. Increase worker threads:
   ```toml
   worker_threads = 8
   ```
2. Increase queue size:
   ```toml
   queue_size = 50000
   ```
3. Reduce retry attempts:
   ```toml
   max_retries = 1
   ```
4. Check for rate limiting from providers

## Best Practices

### 1. Security

- **Never commit credentials**: Use environment variables
- **Rotate keys regularly**: Update tokens every 90 days
- **Use TLS**: Always enable TLS for SMTP
- **Restrict permissions**: Limit Slack bot and PagerDuty service access

### 2. Reliability

- **Test notifications**: Regularly test all channels
- **Monitor failures**: Set up alerts for notification failures
- **Have fallbacks**: Configure multiple notification channels
- **Set appropriate timeouts**: Balance reliability vs latency

### 3. Performance

- **Right-size workers**: 1 worker per 250 notifications/sec
- **Tune queue size**: Prevent memory issues with large queues
- **Use async**: Let workers handle I/O efficiently
- **Batch when possible**: Consider batching for high-volume scenarios

### 4. Operations

- **Start disabled**: Test in non-production first
- **Gradual rollout**: Enable channels one at a time
- **Document runbooks**: Include notification troubleshooting
- **Review logs**: Regularly check notification success rates

## Examples

See `/examples/notifications/` for complete examples:

- `slack_example.rs` - Slack notification examples
- `email_example.rs` - Email notification examples
- `pagerduty_example.rs` - PagerDuty integration
- `webhook_example.rs` - Custom webhook examples

## API Integration

The notification system is fully integrated with all APIs:

### REST API

Notifications are automatically sent when using:
- `POST /v1/alerts` - Alert ingestion
- `POST /v1/incidents` - Incident creation
- `POST /v1/incidents/{id}/resolve` - Incident resolution

### gRPC API

Notifications are automatically sent when calling:
- `SubmitAlert` - Alert ingestion
- `CreateIncident` - Incident creation
- `ResolveIncident` - Incident resolution

---

**Version**: 1.0.0
**Last Updated**: 2025-01-11
**Status**: Production Ready
