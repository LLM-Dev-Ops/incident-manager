use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,

    /// Deployment configuration
    pub deployment: DeploymentConfig,

    /// State backend configuration
    pub state: StateConfig,

    /// Messaging configuration
    #[serde(default)]
    pub messaging: Option<MessagingConfig>,

    /// Integration configurations
    #[serde(default)]
    pub integrations: IntegrationsConfig,

    /// Observability configuration
    pub observability: ObservabilityConfig,

    /// Processing configuration
    pub processing: ProcessingConfig,

    /// Notification configuration
    pub notifications: NotificationConfig,
}

impl Config {
    /// Load configuration from file and environment
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config/default.toml".to_string());

        config::Config::builder()
            // Start with default values
            .add_source(config::File::from_str(
                include_str!("../config/default.toml"),
                config::FileFormat::Toml,
            ))
            // Override with config file if it exists
            .add_source(config::File::with_name(&config_path).required(false))
            // Override with environment variables (prefix: LLM_IM_)
            .add_source(
                config::Environment::with_prefix("LLM_IM")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP server host
    #[serde(default = "default_host")]
    pub host: String,

    /// HTTP server port
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// gRPC server port
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,

    /// Metrics port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// Enable TLS
    #[serde(default)]
    pub tls_enabled: bool,

    /// TLS certificate path
    pub tls_cert: Option<PathBuf>,

    /// TLS key path
    pub tls_key: Option<PathBuf>,

    /// Request timeout (seconds)
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,

    /// Max concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment mode
    #[serde(default)]
    pub mode: DeploymentMode,

    /// Worker type (for worker mode)
    pub worker_type: Option<WorkerType>,

    /// Region (for HA mode)
    pub region: Option<String>,

    /// Availability zones (for HA mode)
    #[serde(default)]
    pub availability_zones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentMode {
    #[default]
    Standalone,
    Worker,
    Sidecar,
    Ha,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerType {
    All,
    Ingestion,
    Processing,
    Orchestration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    /// State backend type
    #[serde(default)]
    pub backend: StateBackend,

    /// Path for embedded database (sled/redb)
    pub path: Option<PathBuf>,

    /// Redis connection string
    pub redis_url: Option<String>,

    /// Redis cluster nodes
    #[serde(default)]
    pub redis_cluster_nodes: Vec<String>,

    /// Database connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StateBackend {
    #[default]
    Sled,
    Redb,
    Redis,
    RedisCluster,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingConfig {
    /// Messaging backend
    pub backend: MessagingBackend,

    /// Kafka brokers
    #[serde(default)]
    pub kafka_brokers: Vec<String>,

    /// Kafka topics
    #[serde(default)]
    pub kafka_topics: KafkaTopics,

    /// Consumer group
    pub consumer_group: Option<String>,

    /// NATS server URL
    pub nats_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessagingBackend {
    Kafka,
    Nats,
    InMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTopics {
    #[serde(default = "default_ingest_topic")]
    pub ingest: String,

    #[serde(default = "default_process_topic")]
    pub process: String,

    #[serde(default = "default_orchestrate_topic")]
    pub orchestrate: String,
}

impl Default for KafkaTopics {
    fn default() -> Self {
        Self {
            ingest: default_ingest_topic(),
            process: default_process_topic(),
            orchestrate: default_orchestrate_topic(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationsConfig {
    pub llm_sentinel: Option<IntegrationConfig>,
    pub llm_shield: Option<IntegrationConfig>,
    pub llm_edge_agent: Option<IntegrationConfig>,
    pub llm_governance_core: Option<IntegrationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub auth_token_env: Option<String>,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub tls_ca: Option<PathBuf>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Enable JSON logging
    #[serde(default)]
    pub json_logs: bool,

    /// Enable OpenTelemetry
    #[serde(default)]
    pub otlp_enabled: bool,

    /// OTLP endpoint
    pub otlp_endpoint: Option<String>,

    /// Service name
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Enable Prometheus metrics
    #[serde(default = "default_true")]
    pub prometheus_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Maximum concurrent incidents to process
    #[serde(default = "default_max_concurrent_incidents")]
    pub max_concurrent_incidents: usize,

    /// Incident processing timeout (seconds)
    #[serde(default = "default_processing_timeout")]
    pub processing_timeout_secs: u64,

    /// Enable deduplication
    #[serde(default = "default_true")]
    pub deduplication_enabled: bool,

    /// Deduplication window (seconds)
    #[serde(default = "default_dedup_window")]
    pub deduplication_window_secs: u64,

    /// Enable correlation
    #[serde(default = "default_true")]
    pub correlation_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enable Slack notifications
    #[serde(default)]
    pub slack_enabled: bool,

    /// Slack webhook URL (from env var)
    pub slack_webhook_env: Option<String>,

    /// Slack Bot token (from env var) for advanced features
    pub slack_bot_token_env: Option<String>,

    /// Default Slack channel
    pub slack_default_channel: Option<String>,

    /// Enable email notifications
    #[serde(default)]
    pub email_enabled: bool,

    /// SMTP server
    pub smtp_server: Option<String>,

    /// SMTP port
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,

    /// Use STARTTLS for SMTP
    #[serde(default = "default_true")]
    pub smtp_use_tls: bool,

    /// SMTP username (from env var)
    pub smtp_username_env: Option<String>,

    /// SMTP password (from env var)
    pub smtp_password_env: Option<String>,

    /// From email address
    pub email_from: Option<String>,

    /// From email name
    pub email_from_name: Option<String>,

    /// Enable PagerDuty notifications
    #[serde(default)]
    pub pagerduty_enabled: bool,

    /// PagerDuty API token (from env var)
    pub pagerduty_api_token_env: Option<String>,

    /// PagerDuty integration key (from env var)
    pub pagerduty_integration_key_env: Option<String>,

    /// PagerDuty API URL
    #[serde(default = "default_pagerduty_api_url")]
    pub pagerduty_api_url: String,

    /// Enable webhook notifications
    #[serde(default)]
    pub webhook_enabled: bool,

    /// Default webhook URL for custom integrations
    pub default_webhook_url: Option<String>,

    /// Webhook timeout (seconds)
    #[serde(default = "default_webhook_timeout")]
    pub webhook_timeout_secs: u64,

    /// Max retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff (seconds)
    #[serde(default = "default_retry_backoff")]
    pub retry_backoff_secs: u64,

    /// Notification queue size
    #[serde(default = "default_notification_queue_size")]
    pub queue_size: usize,

    /// Number of worker threads for sending notifications
    #[serde(default = "default_notification_workers")]
    pub worker_threads: usize,
}

// Default value functions
fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u16 {
    8080
}

fn default_grpc_port() -> u16 {
    9000
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_request_timeout() -> u64 {
    30
}

fn default_max_connections() -> usize {
    10000
}

fn default_pool_size() -> u32 {
    100
}

fn default_ingest_topic() -> String {
    "incidents.ingest".to_string()
}

fn default_process_topic() -> String {
    "incidents.process".to_string()
}

fn default_orchestrate_topic() -> String {
    "incidents.orchestrate".to_string()
}

fn default_timeout() -> u64 {
    10
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_service_name() -> String {
    "llm-incident-manager".to_string()
}

fn default_true() -> bool {
    true
}

fn default_max_concurrent_incidents() -> usize {
    10000
}

fn default_processing_timeout() -> u64 {
    300
}

fn default_dedup_window() -> u64 {
    900 // 15 minutes
}

fn default_smtp_port() -> u16 {
    587
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_backoff() -> u64 {
    5
}

fn default_pagerduty_api_url() -> String {
    "https://events.pagerduty.com/v2/enqueue".to_string()
}

fn default_webhook_timeout() -> u64 {
    10
}

fn default_notification_queue_size() -> usize {
    10000
}

fn default_notification_workers() -> usize {
    4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        assert_eq!(default_http_port(), 8080);
        assert_eq!(default_grpc_port(), 9000);
        assert_eq!(default_metrics_port(), 9090);
        assert_eq!(default_log_level(), "info");
        assert!(default_true());
    }

    #[test]
    fn test_deployment_mode() {
        assert_eq!(DeploymentMode::default(), DeploymentMode::Standalone);
    }
}
