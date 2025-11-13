# Circuit Breaker Initialization Example

## Add to main.rs

Here's how to initialize circuit breaker metrics in your main.rs file:

```rust
use llm_incident_manager::{
    circuit_breaker::init_circuit_breaker_metrics,
    metrics::{init_metrics, PROMETHEUS_REGISTRY},
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting LLM Incident Manager");

    // Initialize standard metrics
    init_metrics()?;
    info!("Standard metrics initialized");

    // Initialize circuit breaker metrics
    init_circuit_breaker_metrics(&PROMETHEUS_REGISTRY)?;
    info!("Circuit breaker metrics initialized");

    // Rest of your initialization...

    Ok(())
}
```

## Add Circuit Breaker Health Endpoint

Add this to your API routes (e.g., in `src/api/health.rs`):

```rust
use axum::{Json, response::IntoResponse};
use llm_incident_manager::circuit_breaker::GLOBAL_CIRCUIT_BREAKER_REGISTRY;

/// Circuit breaker health endpoint
pub async fn circuit_breaker_health() -> impl IntoResponse {
    let health = GLOBAL_CIRCUIT_BREAKER_REGISTRY.health_check();

    let status = if health.healthy {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(serde_json::json!({
        "status": if health.healthy { "healthy" } else { "degraded" },
        "circuit_breakers": {
            "total": health.total_breakers,
            "closed": health.closed,
            "open": health.open,
            "half_open": health.half_open,
        },
        "details": GLOBAL_CIRCUIT_BREAKER_REGISTRY
            .get_all_stats()
            .iter()
            .map(|s| serde_json::json!({
                "name": s.name,
                "state": format!("{:?}", s.state),
                "consecutive_failures": s.consecutive_failures,
                "consecutive_successes": s.consecutive_successes,
                "transition_count": s.transition_count,
            }))
            .collect::<Vec<_>>()
    })))
}
```

Add the route to your router:

```rust
use axum::{Router, routing::get};

pub fn health_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/circuit-breakers", get(circuit_breaker_health))
}
```

## Example Integration in Service

Here's how to use circuit breakers in your enrichment service:

```rust
// In src/enrichment/service.rs or similar

use llm_incident_manager::{
    circuit_breaker::CircuitBreakerConfig,
    integrations::SentinelClientWithBreaker,
};

pub struct EnrichmentService {
    sentinel: SentinelClientWithBreaker,
    // ... other fields
}

impl EnrichmentService {
    pub fn new(config: &Config) -> Result<Self> {
        // Create base Sentinel client
        let sentinel_client = SentinelClient::new(
            config.sentinel_connection.clone(),
            config.sentinel_credentials.clone(),
        )?;

        // Wrap with circuit breaker
        let sentinel = SentinelClientWithBreaker::new(sentinel_client);

        Ok(Self {
            sentinel,
            // ... initialize other fields
        })
    }

    pub async fn enrich_with_sentinel(&self, incident: &Incident) -> Result<Incident> {
        // This call is now protected by circuit breaker
        match self.sentinel.analyze_anomaly(incident).await {
            Ok(analysis) => {
                // Process analysis
                Ok(incident.clone()) // Return enriched incident
            }
            Err(e) => {
                // Circuit breaker error handling
                tracing::warn!("Sentinel enrichment failed: {}", e);
                // Return incident without enrichment or use cached data
                Ok(incident.clone())
            }
        }
    }
}
```

## Example Database Wrapper

Here's how to wrap your database store:

```rust
// In src/state/factory.rs

use llm_incident_manager::state::{CircuitBreakerStore, create_store};

pub async fn create_protected_store(config: &Config) -> Result<impl IncidentStore> {
    // Create the base store
    let base_store = create_store(config).await?;

    // Wrap with circuit breaker
    let protected_store = CircuitBreakerStore::new(
        base_store,
        "incident-store"
    );

    Ok(protected_store)
}
```

Then use it in your application:

```rust
// In main.rs or application setup

let store = create_protected_store(&config).await?;

// All store operations now protected
let incident = store.get_incident(&id).await?;
store.save_incident(&incident).await?;
```

## Example Notification Wrapper

Here's how to protect notification services:

```rust
// In src/notifications/service.rs

use llm_incident_manager::notifications::{
    SlackSenderWithBreaker,
    EmailSenderWithBreaker,
    PagerDutySenderWithBreaker,
};

pub struct NotificationService {
    slack: Option<SlackSenderWithBreaker>,
    email: Option<EmailSenderWithBreaker>,
    pagerduty: Option<PagerDutySenderWithBreaker>,
}

impl NotificationService {
    pub fn new(config: &Config) -> Result<Self> {
        let slack = if let Some(slack_config) = &config.slack {
            let sender = SlackSender::new(slack_config.clone());
            Some(SlackSenderWithBreaker::new(sender))
        } else {
            None
        };

        let email = if let Some(email_config) = &config.email {
            let sender = EmailSender::new(email_config.clone())?;
            Some(EmailSenderWithBreaker::new(sender))
        } else {
            None
        };

        let pagerduty = if let Some(pd_config) = &config.pagerduty {
            let sender = PagerDutySender::new(pd_config.clone());
            Some(PagerDutySenderWithBreaker::new(sender))
        } else {
            None
        };

        Ok(Self {
            slack,
            email,
            pagerduty,
        })
    }

    pub async fn notify(&self, incident: &Incident) -> Result<()> {
        let mut errors = Vec::new();

        // Try Slack (circuit breaker handles failures)
        if let Some(slack) = &self.slack {
            if let Err(e) = slack.send(incident).await {
                tracing::warn!("Slack notification failed: {}", e);
                errors.push(e);
            }
        }

        // Try Email (circuit breaker handles failures)
        if let Some(email) = &self.email {
            if let Err(e) = email.send(incident).await {
                tracing::warn!("Email notification failed: {}", e);
                errors.push(e);
            }
        }

        // Try PagerDuty (circuit breaker handles failures)
        if let Some(pagerduty) = &self.pagerduty {
            if let Err(e) = pagerduty.send(incident).await {
                tracing::warn!("PagerDuty notification failed: {}", e);
                errors.push(e);
            }
        }

        // Return error only if all channels failed
        if !errors.is_empty() && errors.len() == self.channel_count() {
            Err(AppError::Internal("All notification channels failed".to_string()))
        } else {
            Ok(())
        }
    }

    fn channel_count(&self) -> usize {
        let mut count = 0;
        if self.slack.is_some() { count += 1; }
        if self.email.is_some() { count += 1; }
        if self.pagerduty.is_some() { count += 1; }
        count
    }
}
```

## Configuration File Example

Add circuit breaker settings to your config.toml:

```toml
[circuit_breaker]
# Enable circuit breakers
enabled = true

# Default settings
default_failure_threshold = 5
default_success_threshold = 2
default_timeout_seconds = 60

# Per-service overrides
[circuit_breaker.sentinel]
failure_threshold = 3
timeout_seconds = 120

[circuit_breaker.database]
failure_threshold = 10
timeout_seconds = 10

[circuit_breaker.notifications]
failure_threshold = 10
timeout_seconds = 60
```

## Metrics Initialization Check

To verify metrics are properly initialized, check the /metrics endpoint:

```bash
curl http://localhost:8080/metrics | grep circuit_breaker
```

Expected output:
```
# HELP llm_incident_manager_circuit_breaker_state Current state of circuit breakers
# TYPE llm_incident_manager_circuit_breaker_state gauge
llm_incident_manager_circuit_breaker_state{name="sentinel-llm"} 0
llm_incident_manager_circuit_breaker_state{name="incident-store"} 0
...
```

## Testing the Implementation

```rust
#[tokio::test]
async fn test_circuit_breaker_integration() {
    // Initialize
    init_metrics().unwrap();
    init_circuit_breaker_metrics(&PROMETHEUS_REGISTRY).unwrap();

    // Create protected client
    let client = SentinelClient::new(test_config(), test_credentials()).unwrap();
    let protected = SentinelClientWithBreaker::new(client);

    // Test normal operation
    let result = protected.fetch_alerts(Some(10)).await;
    assert!(result.is_ok() || matches!(result, Err(CircuitBreakerError::Open(_))));

    // Check registry
    let health = GLOBAL_CIRCUIT_BREAKER_REGISTRY.health_check();
    assert!(health.total_breakers >= 1);
}
```

## Deployment Checklist

- [ ] Add `init_circuit_breaker_metrics()` to main.rs
- [ ] Add circuit breaker health endpoint
- [ ] Update service constructors to use circuit breaker wrappers
- [ ] Configure Prometheus to scrape metrics
- [ ] Set up Grafana dashboards
- [ ] Configure alerting rules
- [ ] Test in staging environment
- [ ] Document service-specific configurations
- [ ] Train operations team
- [ ] Deploy to production

## Monitoring Setup

### Prometheus Alert Rules

Create `alerts/circuit_breaker.yml`:

```yaml
groups:
  - name: circuit_breaker
    interval: 30s
    rules:
      - alert: CircuitBreakerOpen
        expr: llm_incident_manager_circuit_breaker_state == 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Circuit breaker {{ $labels.name }} is open"
          description: "Circuit breaker {{ $labels.name }} has been open for 5 minutes"

      - alert: CircuitBreakerHighRejectionRate
        expr: rate(llm_incident_manager_circuit_breaker_rejected_calls_total[5m]) > 10
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High rejection rate on {{ $labels.name }}"
          description: "Circuit breaker {{ $labels.name }} is rejecting > 10 requests/sec"
```

### Grafana Dashboard

Import the circuit breaker dashboard:

```json
{
  "dashboard": {
    "title": "Circuit Breakers",
    "panels": [
      {
        "title": "Circuit Breaker States",
        "targets": [
          {
            "expr": "llm_incident_manager_circuit_breaker_state"
          }
        ]
      },
      {
        "title": "Call Success Rate",
        "targets": [
          {
            "expr": "rate(llm_incident_manager_circuit_breaker_successful_calls_total[5m]) / rate(llm_incident_manager_circuit_breaker_calls_total[5m])"
          }
        ]
      }
    ]
  }
}
```

## See Also

- [Circuit Breaker Implementation Guide](CIRCUIT_BREAKER_IMPLEMENTATION.md)
- [Quick Reference](CIRCUIT_BREAKER_QUICK_REFERENCE.md)
- [Executive Summary](CIRCUIT_BREAKER_EXECUTIVE_SUMMARY.md)
