//! Prometheus metrics for circuit breakers.

use lazy_static::lazy_static;
use prometheus::{CounterVec, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry};

/// Container for all circuit breaker metrics
pub struct CircuitBreakerMetrics {
    /// Current state of circuit breakers (0=closed, 1=open, 2=half-open)
    pub state: GaugeVec,

    /// Total number of calls made through circuit breakers
    pub calls_total: CounterVec,

    /// Total number of successful calls
    pub successful_calls: CounterVec,

    /// Total number of failed calls
    pub failed_calls: CounterVec,

    /// Total number of rejected calls (when circuit is open)
    pub rejected_calls: CounterVec,

    /// Duration of calls through circuit breakers
    pub call_duration: HistogramVec,

    /// State transition events
    pub state_transitions: CounterVec,
}

impl CircuitBreakerMetrics {
    fn new() -> Self {
        Self {
            state: GaugeVec::new(
                Opts::new("circuit_breaker_state", "Current state of circuit breakers")
                    .namespace("llm_incident_manager"),
                &["name"],
            )
            .expect("Failed to create circuit_breaker_state metric"),

            calls_total: CounterVec::new(
                Opts::new(
                    "circuit_breaker_calls_total",
                    "Total number of calls through circuit breakers",
                )
                .namespace("llm_incident_manager"),
                &["name", "status"],
            )
            .expect("Failed to create circuit_breaker_calls_total metric"),

            successful_calls: CounterVec::new(
                Opts::new(
                    "circuit_breaker_successful_calls_total",
                    "Total number of successful calls",
                )
                .namespace("llm_incident_manager"),
                &["name"],
            )
            .expect("Failed to create circuit_breaker_successful_calls_total metric"),

            failed_calls: CounterVec::new(
                Opts::new(
                    "circuit_breaker_failed_calls_total",
                    "Total number of failed calls",
                )
                .namespace("llm_incident_manager"),
                &["name"],
            )
            .expect("Failed to create circuit_breaker_failed_calls_total metric"),

            rejected_calls: CounterVec::new(
                Opts::new(
                    "circuit_breaker_rejected_calls_total",
                    "Total number of rejected calls (circuit open)",
                )
                .namespace("llm_incident_manager"),
                &["name"],
            )
            .expect("Failed to create circuit_breaker_rejected_calls_total metric"),

            call_duration: HistogramVec::new(
                HistogramOpts::new(
                    "circuit_breaker_call_duration_seconds",
                    "Duration of calls through circuit breakers",
                )
                .namespace("llm_incident_manager")
                .buckets(vec![
                    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
                    60.0,
                ]),
                &["name"],
            )
            .expect("Failed to create circuit_breaker_call_duration_seconds metric"),

            state_transitions: CounterVec::new(
                Opts::new(
                    "circuit_breaker_state_transitions_total",
                    "Total number of state transitions",
                )
                .namespace("llm_incident_manager"),
                &["name", "from_state", "to_state"],
            )
            .expect("Failed to create circuit_breaker_state_transitions_total metric"),
        }
    }
}

lazy_static! {
    /// Global circuit breaker metrics instance
    pub static ref CIRCUIT_BREAKER_METRICS: CircuitBreakerMetrics = CircuitBreakerMetrics::new();
}

/// Initialize circuit breaker metrics with the Prometheus registry
pub fn init_circuit_breaker_metrics(
    registry: &Registry,
) -> Result<(), prometheus::Error> {
    registry.register(Box::new(CIRCUIT_BREAKER_METRICS.state.clone()))?;
    registry.register(Box::new(CIRCUIT_BREAKER_METRICS.calls_total.clone()))?;
    registry.register(Box::new(CIRCUIT_BREAKER_METRICS.successful_calls.clone()))?;
    registry.register(Box::new(CIRCUIT_BREAKER_METRICS.failed_calls.clone()))?;
    registry.register(Box::new(CIRCUIT_BREAKER_METRICS.rejected_calls.clone()))?;
    registry.register(Box::new(CIRCUIT_BREAKER_METRICS.call_duration.clone()))?;
    registry.register(Box::new(CIRCUIT_BREAKER_METRICS.state_transitions.clone()))?;

    tracing::info!("Circuit breaker metrics initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        // Just verify metrics can be created without panic
        let _metrics = &*CIRCUIT_BREAKER_METRICS;
    }

    #[test]
    fn test_record_call() {
        CIRCUIT_BREAKER_METRICS
            .calls_total
            .with_label_values(&["test", "allowed"])
            .inc();

        let value = CIRCUIT_BREAKER_METRICS
            .calls_total
            .with_label_values(&["test", "allowed"])
            .get();
        assert!(value >= 1.0);
    }

    #[test]
    fn test_record_state() {
        CIRCUIT_BREAKER_METRICS
            .state
            .with_label_values(&["test"])
            .set(1.0);

        let value = CIRCUIT_BREAKER_METRICS
            .state
            .with_label_values(&["test"])
            .get();
        assert_eq!(value, 1.0);
    }
}
