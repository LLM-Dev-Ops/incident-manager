//! Prometheus metrics for messaging

use lazy_static::lazy_static;
use prometheus::{register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec, HistogramVec};

/// Messaging metrics
pub struct MessagingMetrics {
    /// Messages published counter
    pub messages_published: CounterVec,

    /// Messages consumed counter
    pub messages_consumed: CounterVec,

    /// Message publish failures
    pub publish_failures: CounterVec,

    /// Message consume failures
    pub consume_failures: CounterVec,

    /// Active connections gauge
    pub active_connections: GaugeVec,

    /// Message publish latency
    pub publish_latency: HistogramVec,

    /// Message size histogram
    pub message_size: HistogramVec,
}

lazy_static! {
    pub static ref MESSAGING_METRICS: MessagingMetrics = MessagingMetrics {
        messages_published: register_counter_vec!(
            "messaging_messages_published_total",
            "Total number of messages published",
            &["topic", "backend"]
        )
        .unwrap(),

        messages_consumed: register_counter_vec!(
            "messaging_messages_consumed_total",
            "Total number of messages consumed",
            &["topic", "backend"]
        )
        .unwrap(),

        publish_failures: register_counter_vec!(
            "messaging_publish_failures_total",
            "Total number of publish failures",
            &["topic", "backend", "error"]
        )
        .unwrap(),

        consume_failures: register_counter_vec!(
            "messaging_consume_failures_total",
            "Total number of consume failures",
            &["topic", "backend", "error"]
        )
        .unwrap(),

        active_connections: register_gauge_vec!(
            "messaging_active_connections",
            "Number of active messaging connections",
            &["backend"]
        )
        .unwrap(),

        publish_latency: register_histogram_vec!(
            "messaging_publish_latency_seconds",
            "Message publish latency in seconds",
            &["topic", "backend"]
        )
        .unwrap(),

        message_size: register_histogram_vec!(
            "messaging_message_size_bytes",
            "Message size in bytes",
            &["topic", "backend"]
        )
        .unwrap(),
    };
}

/// Initialize messaging metrics
pub fn init_messaging_metrics() {
    lazy_static::initialize(&MESSAGING_METRICS);
}
