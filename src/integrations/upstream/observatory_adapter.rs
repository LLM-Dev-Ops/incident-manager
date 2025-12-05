//! LLM-Observatory Core Adapter
//!
//! Thin runtime adapter for consuming telemetry streams, traces, and structured events
//! from the upstream llm-observatory-core crate. Translates observatory types to internal
//! incident-manager types without modifying existing alerting logic.
//!
//! This adapter provides type-safe consumption of observatory spans and events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Adapter for consuming telemetry from llm-observatory-core
#[derive(Debug, Clone)]
pub struct ObservatoryCoreAdapter {
    /// Source identifier for tracing
    source_id: String,
    /// Enable verbose telemetry
    verbose: bool,
}

impl Default for ObservatoryCoreAdapter {
    fn default() -> Self {
        Self::new("llm-observatory-core")
    }
}

impl ObservatoryCoreAdapter {
    /// Create a new observatory core adapter
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            verbose: false,
        }
    }

    /// Enable verbose mode
    pub fn with_verbose(mut self, enabled: bool) -> Self {
        self.verbose = enabled;
        self
    }

    /// Convert upstream LlmSpan to internal trace span representation
    ///
    /// This is the primary consumption point for traces from observatory-core.
    /// Does not modify trace collection - pure type translation.
    pub fn convert_span(&self, span: &UpstreamLlmSpan) -> UpstreamTraceSpan {
        UpstreamTraceSpan {
            trace_id: span.trace_id.clone(),
            span_id: span.span_id.clone(),
            parent_span_id: span.parent_span_id.clone(),
            operation_name: span.operation_name.clone(),
            service_name: span.service_name.clone(),
            start_time: span.start_time,
            end_time: span.end_time,
            duration_ms: span.duration_ms,
            status: span.status.clone(),
            provider: span.provider.clone(),
            model: span.model.clone(),
            token_usage: span.token_usage.clone(),
            cost: span.cost.clone(),
            events: span.events.iter().map(|e| self.convert_span_event(e)).collect(),
            attributes: span.attributes.clone(),
            source: self.source_id.clone(),
        }
    }

    /// Convert span events to internal structured events
    pub fn convert_span_event(&self, event: &UpstreamSpanEvent) -> UpstreamStructuredEvent {
        UpstreamStructuredEvent {
            name: event.name.clone(),
            timestamp: event.timestamp,
            attributes: event.attributes.clone(),
            severity: self.infer_event_severity(&event.name),
        }
    }

    /// Extract telemetry stream from multiple spans
    pub fn create_telemetry_stream(&self, spans: &[UpstreamLlmSpan]) -> UpstreamTelemetryStream {
        let converted_spans: Vec<UpstreamTraceSpan> = spans
            .iter()
            .map(|s| self.convert_span(s))
            .collect();

        let total_tokens: u32 = spans
            .iter()
            .filter_map(|s| s.token_usage.as_ref())
            .map(|t| t.total_tokens)
            .sum();

        let total_cost: f64 = spans
            .iter()
            .filter_map(|s| s.cost.as_ref())
            .map(|c| c.total_cost)
            .sum();

        let error_count = spans
            .iter()
            .filter(|s| matches!(s.status, TraceStatus::Error(_)))
            .count();

        UpstreamTelemetryStream {
            stream_id: format!("stream-{}", uuid::Uuid::new_v4()),
            start_time: spans.first().map(|s| s.start_time).unwrap_or_else(Utc::now),
            end_time: spans.last().and_then(|s| s.end_time),
            span_count: spans.len(),
            spans: converted_spans,
            aggregated_metrics: TelemetryMetrics {
                total_requests: spans.len() as u64,
                total_tokens,
                total_cost,
                error_count: error_count as u64,
                avg_latency_ms: self.calculate_avg_latency(spans),
            },
            source: self.source_id.clone(),
        }
    }

    /// Create from llm_observatory_core::span::LlmSpan
    pub fn from_observatory_span(span: &llm_observatory_core::span::LlmSpan) -> UpstreamLlmSpan {
        use llm_observatory_core::types::Provider;

        UpstreamLlmSpan {
            trace_id: span.trace_id.clone(),
            span_id: span.span_id.clone(),
            parent_span_id: span.parent_span_id.clone(),
            operation_name: span.name.clone(),
            service_name: "observatory".to_string(),
            start_time: span.latency.start_time,
            end_time: Some(span.latency.end_time),
            duration_ms: span.latency.total_ms as f64,
            status: match &span.status {
                llm_observatory_core::span::SpanStatus::Ok => TraceStatus::Ok,
                llm_observatory_core::span::SpanStatus::Error => TraceStatus::Error("Error".to_string()),
                llm_observatory_core::span::SpanStatus::Unset => TraceStatus::Unset,
            },
            provider: match &span.provider {
                Provider::OpenAI => "openai".to_string(),
                Provider::Anthropic => "anthropic".to_string(),
                Provider::Google => "google".to_string(),
                Provider::Mistral => "mistral".to_string(),
                Provider::Cohere => "cohere".to_string(),
                Provider::SelfHosted => "self-hosted".to_string(),
                Provider::Custom(name) => name.clone(),
            },
            model: span.model.clone(),
            token_usage: span.token_usage.as_ref().map(|t| UpstreamTokenUsage {
                prompt_tokens: t.prompt_tokens,
                completion_tokens: t.completion_tokens,
                total_tokens: t.total_tokens,
            }),
            cost: span.cost.as_ref().map(|c| UpstreamCost {
                total_cost: c.amount_usd,
                currency: c.currency.clone(),
            }),
            events: span.events.iter().map(|e| UpstreamSpanEvent {
                name: e.name.clone(),
                timestamp: e.timestamp,
                attributes: e.attributes.clone(),
            }).collect(),
            attributes: HashMap::new(),
        }
    }

    /// Extract structured events from span for incident correlation
    pub fn extract_structured_events(&self, span: &UpstreamLlmSpan) -> Vec<UpstreamStructuredEvent> {
        span.events.iter().map(|e| self.convert_span_event(e)).collect()
    }

    /// Batch convert multiple spans
    pub fn convert_span_batch(&self, spans: &[UpstreamLlmSpan]) -> Vec<UpstreamTraceSpan> {
        spans.iter().map(|s| self.convert_span(s)).collect()
    }

    // --- Private helper methods ---

    fn infer_event_severity(&self, event_name: &str) -> EventSeverity {
        let name_lower = event_name.to_lowercase();
        if name_lower.contains("error") || name_lower.contains("exception") {
            EventSeverity::Error
        } else if name_lower.contains("warn") {
            EventSeverity::Warning
        } else if name_lower.contains("debug") {
            EventSeverity::Debug
        } else {
            EventSeverity::Info
        }
    }

    fn calculate_avg_latency(&self, spans: &[UpstreamLlmSpan]) -> f64 {
        let latencies: Vec<f64> = spans.iter().map(|s| s.duration_ms).collect();

        if latencies.is_empty() {
            0.0
        } else {
            latencies.iter().sum::<f64>() / latencies.len() as f64
        }
    }
}

/// Internal LLM span representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamLlmSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: f64,
    pub status: TraceStatus,
    pub provider: String,
    pub model: String,
    pub token_usage: Option<UpstreamTokenUsage>,
    pub cost: Option<UpstreamCost>,
    pub events: Vec<UpstreamSpanEvent>,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Span event from observatory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamSpanEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Telemetry stream aggregating multiple spans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamTelemetryStream {
    pub stream_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub span_count: usize,
    pub spans: Vec<UpstreamTraceSpan>,
    pub aggregated_metrics: TelemetryMetrics,
    pub source: String,
}

/// Aggregated telemetry metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryMetrics {
    pub total_requests: u64,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub error_count: u64,
    pub avg_latency_ms: f64,
}

/// Trace span from observatory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamTraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: f64,
    pub status: TraceStatus,
    pub provider: String,
    pub model: String,
    pub token_usage: Option<UpstreamTokenUsage>,
    pub cost: Option<UpstreamCost>,
    pub events: Vec<UpstreamStructuredEvent>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub source: String,
}

/// Trace status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Ok,
    Error(String),
    Unset,
}

/// Token usage from span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamTokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Cost information from span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamCost {
    pub total_cost: f64,
    pub currency: String,
}

/// Structured event from span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamStructuredEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub severity: EventSeverity,
}

/// Event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = ObservatoryCoreAdapter::new("test-source");
        assert_eq!(adapter.source_id, "test-source");
    }

    #[test]
    fn test_default_adapter() {
        let adapter = ObservatoryCoreAdapter::default();
        assert_eq!(adapter.source_id, "llm-observatory-core");
    }

    #[test]
    fn test_event_severity_inference() {
        let adapter = ObservatoryCoreAdapter::default();
        assert_eq!(adapter.infer_event_severity("error_occurred"), EventSeverity::Error);
        assert_eq!(adapter.infer_event_severity("warning_threshold"), EventSeverity::Warning);
        assert_eq!(adapter.infer_event_severity("request_started"), EventSeverity::Info);
    }
}
