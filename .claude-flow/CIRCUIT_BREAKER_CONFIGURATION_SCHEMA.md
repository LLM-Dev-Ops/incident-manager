# Circuit Breaker Configuration Schema

## Overview

This document defines the complete configuration schema for circuit breakers in the LLM Incident Manager, including YAML/TOML formats, validation rules, and preset configurations.

## Configuration Structure

### Core Configuration Types

```rust
/// Complete circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure detection configuration
    pub detection: FailureDetectionConfig,

    /// State transition configuration
    pub transitions: StateTransitionConfig,

    /// Recovery configuration
    pub recovery: RecoveryConfig,

    /// Timeout configuration
    pub timeouts: TimeoutConfig,

    /// Optional: Request timeout (None = no timeout)
    pub request_timeout: Option<Duration>,

    /// Optional: State persistence
    pub persistence: Option<PersistenceConfig>,
}

/// Failure detection strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDetectionConfig {
    /// Consecutive failure threshold
    pub consecutive_failures: Option<ConsecutiveFailureConfig>,

    /// Failure rate threshold
    pub failure_rate: Option<FailureRateConfig>,

    /// Slow call detection
    pub slow_call: Option<SlowCallConfig>,

    /// Detection strategy when multiple enabled
    pub strategy: DetectionStrategy,
}

/// Consecutive failure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConsecutiveFailureConfig {
    /// Number of consecutive failures (default: 5, min: 1, max: 100)
    #[serde(default = "default_consecutive_threshold")]
    pub threshold: u32,
}

fn default_consecutive_threshold() -> u32 {
    5
}

/// Failure rate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FailureRateConfig {
    /// Failure rate threshold 0.0-1.0 (default: 0.5)
    #[serde(default = "default_failure_rate")]
    pub threshold: f64,

    /// Minimum requests before evaluating (default: 10)
    #[serde(default = "default_min_requests")]
    pub minimum_requests: usize,

    /// Sliding window configuration
    pub window: WindowConfig,
}

fn default_failure_rate() -> f64 {
    0.5
}

fn default_min_requests() -> usize {
    10
}

/// Sliding window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WindowConfig {
    /// Count-based sliding window
    CountBased {
        /// Number of requests to track (default: 100)
        #[serde(default = "default_window_size")]
        size: usize,
    },

    /// Time-based sliding window
    TimeBased {
        /// Duration in seconds (default: 60)
        #[serde(
            default = "default_window_duration",
            with = "humantime_serde"
        )]
        duration: Duration,
    },
}

fn default_window_size() -> usize {
    100
}

fn default_window_duration() -> Duration {
    Duration::from_secs(60)
}

/// Slow call detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SlowCallConfig {
    /// Duration threshold for slow calls (default: 5s)
    #[serde(
        default = "default_slow_threshold",
        with = "humantime_serde"
    )]
    pub threshold: Duration,

    /// Slow call rate threshold 0.0-1.0 (default: 0.5)
    #[serde(default = "default_slow_rate")]
    pub rate_threshold: f64,

    /// Minimum requests before evaluating (default: 10)
    #[serde(default = "default_min_requests")]
    pub minimum_requests: usize,

    /// Window duration (default: 60s)
    #[serde(
        default = "default_window_duration",
        with = "humantime_serde"
    )]
    pub window_duration: Duration,
}

fn default_slow_threshold() -> Duration {
    Duration::from_secs(5)
}

fn default_slow_rate() -> f64 {
    0.5
}

/// Detection strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetectionStrategy {
    /// Trip if ANY detector triggers (default)
    Any,

    /// Trip if ALL detectors trigger
    All,
}

impl Default for DetectionStrategy {
    fn default() -> Self {
        Self::Any
    }
}

/// State transition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StateTransitionConfig {
    /// Open state timeout before transitioning to half-open (default: 30s)
    #[serde(
        default = "default_open_timeout",
        with = "humantime_serde"
    )]
    pub open_timeout: Duration,

    /// Enable exponential backoff for open timeout (default: true)
    #[serde(default = "default_enable_backoff")]
    pub enable_exponential_backoff: bool,

    /// Backoff multiplier (default: 2.0, min: 1.0, max: 10.0)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Maximum backoff duration (default: 5 minutes)
    #[serde(
        default = "default_max_backoff",
        with = "humantime_serde"
    )]
    pub max_backoff_duration: Duration,
}

fn default_open_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_enable_backoff() -> bool {
    true
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_max_backoff() -> Duration {
    Duration::from_secs(300)
}

/// Recovery (half-open state) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecoveryConfig {
    /// Consecutive successes to close circuit (default: 3)
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,

    /// Maximum concurrent probe requests (default: 3)
    #[serde(default = "default_max_probes")]
    pub max_concurrent_probes: u32,

    /// Failure threshold in half-open (default: 1)
    #[serde(default = "default_half_open_failure_threshold")]
    pub failure_threshold: u32,

    /// Strict mode: any failure returns to OPEN (default: false)
    #[serde(default)]
    pub strict_mode: bool,

    /// Success rate required to close (default: 1.0 = 100%)
    #[serde(default = "default_success_rate")]
    pub success_rate: f64,

    /// Minimum probes before evaluating success rate (default: 3)
    #[serde(default = "default_min_probes")]
    pub minimum_probes: u32,
}

fn default_success_threshold() -> u32 {
    3
}

fn default_max_probes() -> u32 {
    3
}

fn default_half_open_failure_threshold() -> u32 {
    1
}

fn default_success_rate() -> f64 {
    1.0
}

fn default_min_probes() -> u32 {
    3
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TimeoutConfig {
    /// Request timeout (None = no timeout)
    #[serde(
        default,
        with = "humantime_serde",
        skip_serializing_if = "Option::is_none"
    )]
    pub request_timeout: Option<Duration>,

    /// Treat timeouts as failures (default: true)
    #[serde(default = "default_timeout_as_failure")]
    pub count_timeouts_as_failures: bool,
}

fn default_timeout_as_failure() -> bool {
    true
}

/// State persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PersistenceConfig {
    /// No persistence
    None,

    /// Redis persistence
    Redis {
        /// Redis connection URL
        url: String,

        /// TTL for state entries (default: 1 hour)
        #[serde(
            default = "default_redis_ttl",
            with = "humantime_serde"
        )]
        ttl: Duration,

        /// Key prefix (default: "circuit_breaker:")
        #[serde(default = "default_redis_prefix")]
        prefix: String,
    },

    /// File-based persistence
    File {
        /// Directory path for state files
        path: PathBuf,

        /// Sync interval (default: 30s)
        #[serde(
            default = "default_file_sync_interval",
            with = "humantime_serde"
        )]
        sync_interval: Duration,
    },
}

fn default_redis_ttl() -> Duration {
    Duration::from_secs(3600)
}

fn default_redis_prefix() -> String {
    "circuit_breaker:".to_string()
}

fn default_file_sync_interval() -> Duration {
    Duration::from_secs(30)
}
```

## YAML Configuration Format

### Complete Example

```yaml
# Circuit Breaker Configuration
circuit_breakers:
  # Default configuration for all circuit breakers
  default:
    detection:
      # Consecutive failure detection
      consecutive_failures:
        threshold: 5

      # Failure rate detection (sliding window)
      failure_rate:
        threshold: 0.5  # 50%
        minimum_requests: 10
        window:
          type: time_based
          duration: 60s

      # Slow call detection
      slow_call:
        threshold: 5s
        rate_threshold: 0.5  # 50%
        minimum_requests: 10
        window_duration: 60s

      # Strategy when multiple detectors enabled
      strategy: any  # any | all

    transitions:
      # Time to wait in OPEN before HALF-OPEN
      open_timeout: 30s

      # Exponential backoff configuration
      enable_exponential_backoff: true
      backoff_multiplier: 2.0
      max_backoff_duration: 5m

    recovery:
      # Half-open state configuration
      success_threshold: 3
      max_concurrent_probes: 3
      failure_threshold: 1
      strict_mode: false
      success_rate: 0.8  # 80%
      minimum_probes: 3

    timeouts:
      request_timeout: 30s
      count_timeouts_as_failures: true

    # Optional: State persistence
    persistence:
      type: redis
      url: "redis://localhost:6379"
      ttl: 1h
      prefix: "circuit_breaker:"

  # Service-specific overrides
  services:
    # Sentinel client configuration
    sentinel_client:
      detection:
        consecutive_failures:
          threshold: 3
        failure_rate:
          threshold: 0.3
          minimum_requests: 5
          window:
            type: time_based
            duration: 30s
      transitions:
        open_timeout: 10s
      timeouts:
        request_timeout: 10s

    # Shield client configuration (lenient)
    shield_client:
      detection:
        consecutive_failures:
          threshold: 10
        failure_rate:
          threshold: 0.7
          minimum_requests: 20
      transitions:
        open_timeout: 60s
        enable_exponential_backoff: false
      recovery:
        strict_mode: false

    # Edge Agent client (aggressive)
    edge_agent_client:
      detection:
        consecutive_failures:
          threshold: 2
        failure_rate:
          threshold: 0.2
          minimum_requests: 5
        strategy: all
      transitions:
        open_timeout: 5s
      recovery:
        success_threshold: 5
        strict_mode: true

    # Governance client (balanced)
    governance_client:
      detection:
        consecutive_failures:
          threshold: 5
        slow_call:
          threshold: 10s
          rate_threshold: 0.4
      timeouts:
        request_timeout: 60s
```

### Minimal Example

```yaml
circuit_breakers:
  default:
    detection:
      consecutive_failures:
        threshold: 5
    transitions:
      open_timeout: 30s
    recovery:
      success_threshold: 3
```

### Preset Configurations

```yaml
circuit_breakers:
  # Use a preset instead of custom config
  default:
    preset: default  # default | aggressive | lenient

  services:
    critical_service:
      preset: aggressive

    background_service:
      preset: lenient
```

## TOML Configuration Format

### Complete Example

```toml
[circuit_breakers.default]

[circuit_breakers.default.detection]
strategy = "any"

[circuit_breakers.default.detection.consecutive_failures]
threshold = 5

[circuit_breakers.default.detection.failure_rate]
threshold = 0.5
minimum_requests = 10

[circuit_breakers.default.detection.failure_rate.window]
type = "time_based"
duration = "60s"

[circuit_breakers.default.detection.slow_call]
threshold = "5s"
rate_threshold = 0.5
minimum_requests = 10
window_duration = "60s"

[circuit_breakers.default.transitions]
open_timeout = "30s"
enable_exponential_backoff = true
backoff_multiplier = 2.0
max_backoff_duration = "5m"

[circuit_breakers.default.recovery]
success_threshold = 3
max_concurrent_probes = 3
failure_threshold = 1
strict_mode = false
success_rate = 0.8
minimum_probes = 3

[circuit_breakers.default.timeouts]
request_timeout = "30s"
count_timeouts_as_failures = true

[circuit_breakers.default.persistence]
type = "redis"
url = "redis://localhost:6379"
ttl = "1h"
prefix = "circuit_breaker:"

# Service-specific configuration
[circuit_breakers.services.sentinel_client]

[circuit_breakers.services.sentinel_client.detection.consecutive_failures]
threshold = 3

[circuit_breakers.services.sentinel_client.detection.failure_rate]
threshold = 0.3
minimum_requests = 5

[circuit_breakers.services.sentinel_client.detection.failure_rate.window]
type = "time_based"
duration = "30s"
```

## Validation Rules

### Configuration Validation

```rust
use validator::{Validate, ValidationError};

impl Validate for CircuitBreakerConfig {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        // Validate detection config
        self.detection.validate()?;

        // Validate transitions config
        self.transitions.validate()?;

        // Validate recovery config
        self.recovery.validate()?;

        Ok(())
    }
}

impl Validate for FailureDetectionConfig {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        // At least one detection method must be enabled
        if self.consecutive_failures.is_none()
            && self.failure_rate.is_none()
            && self.slow_call.is_none()
        {
            return Err(validator::ValidationErrors::new());
        }

        // Validate each enabled detector
        if let Some(ref config) = self.consecutive_failures {
            config.validate()?;
        }

        if let Some(ref config) = self.failure_rate {
            config.validate()?;
        }

        if let Some(ref config) = self.slow_call {
            config.validate()?;
        }

        Ok(())
    }
}

impl Validate for ConsecutiveFailureConfig {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        if self.threshold < 1 || self.threshold > 100 {
            let mut errors = validator::ValidationErrors::new();
            errors.add(
                "threshold",
                ValidationError::new("threshold must be between 1 and 100"),
            );
            return Err(errors);
        }
        Ok(())
    }
}

impl Validate for FailureRateConfig {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        if !(0.0..=1.0).contains(&self.threshold) {
            errors.add(
                "threshold",
                ValidationError::new("threshold must be between 0.0 and 1.0"),
            );
        }

        if self.minimum_requests == 0 {
            errors.add(
                "minimum_requests",
                ValidationError::new("minimum_requests must be greater than 0"),
            );
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        self.window.validate()?;
        Ok(())
    }
}

impl Validate for WindowConfig {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            WindowConfig::CountBased { size } => {
                if *size == 0 || *size > 10000 {
                    let mut errors = validator::ValidationErrors::new();
                    errors.add(
                        "size",
                        ValidationError::new("size must be between 1 and 10000"),
                    );
                    return Err(errors);
                }
            }
            WindowConfig::TimeBased { duration } => {
                if duration.as_secs() == 0 || duration.as_secs() > 3600 {
                    let mut errors = validator::ValidationErrors::new();
                    errors.add(
                        "duration",
                        ValidationError::new("duration must be between 1s and 1h"),
                    );
                    return Err(errors);
                }
            }
        }
        Ok(())
    }
}

impl Validate for StateTransitionConfig {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        if self.open_timeout.as_secs() == 0 {
            errors.add(
                "open_timeout",
                ValidationError::new("open_timeout must be greater than 0"),
            );
        }

        if !(1.0..=10.0).contains(&self.backoff_multiplier) {
            errors.add(
                "backoff_multiplier",
                ValidationError::new("backoff_multiplier must be between 1.0 and 10.0"),
            );
        }

        if self.max_backoff_duration < self.open_timeout {
            errors.add(
                "max_backoff_duration",
                ValidationError::new("max_backoff_duration must be >= open_timeout"),
            );
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(())
    }
}

impl Validate for RecoveryConfig {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        if self.success_threshold == 0 {
            errors.add(
                "success_threshold",
                ValidationError::new("success_threshold must be greater than 0"),
            );
        }

        if self.max_concurrent_probes == 0 {
            errors.add(
                "max_concurrent_probes",
                ValidationError::new("max_concurrent_probes must be greater than 0"),
            );
        }

        if !(0.0..=1.0).contains(&self.success_rate) {
            errors.add(
                "success_rate",
                ValidationError::new("success_rate must be between 0.0 and 1.0"),
            );
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(())
    }
}
```

## Configuration Loading

### From File

```rust
use config::{Config, File, FileFormat};
use std::path::Path;

pub struct CircuitBreakerConfigLoader;

impl CircuitBreakerConfigLoader {
    /// Load configuration from YAML file
    pub fn from_yaml<P: AsRef<Path>>(path: P) -> Result<CircuitBreakerConfig, config::ConfigError> {
        let settings = Config::builder()
            .add_source(File::from(path.as_ref()).format(FileFormat::Yaml))
            .build()?;

        let config: CircuitBreakerConfig = settings.try_deserialize()?;
        config.validate()
            .map_err(|e| config::ConfigError::Message(format!("Validation failed: {}", e)))?;

        Ok(config)
    }

    /// Load configuration from TOML file
    pub fn from_toml<P: AsRef<Path>>(path: P) -> Result<CircuitBreakerConfig, config::ConfigError> {
        let settings = Config::builder()
            .add_source(File::from(path.as_ref()).format(FileFormat::Toml))
            .build()?;

        let config: CircuitBreakerConfig = settings.try_deserialize()?;
        config.validate()
            .map_err(|e| config::ConfigError::Message(format!("Validation failed: {}", e)))?;

        Ok(config)
    }

    /// Load configuration with environment variable overrides
    pub fn from_file_with_env<P: AsRef<Path>>(
        path: P,
        env_prefix: &str,
    ) -> Result<CircuitBreakerConfig, config::ConfigError> {
        let settings = Config::builder()
            .add_source(File::from(path.as_ref()))
            .add_source(
                config::Environment::with_prefix(env_prefix)
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let config: CircuitBreakerConfig = settings.try_deserialize()?;
        config.validate()
            .map_err(|e| config::ConfigError::Message(format!("Validation failed: {}", e)))?;

        Ok(config)
    }
}
```

### Environment Variable Overrides

Environment variables can override configuration values using the format:
`CB__<SECTION>__<KEY>=<VALUE>`

Examples:
```bash
# Override consecutive failure threshold
CB__DETECTION__CONSECUTIVE_FAILURES__THRESHOLD=10

# Override open timeout
CB__TRANSITIONS__OPEN_TIMEOUT=60s

# Override failure rate threshold
CB__DETECTION__FAILURE_RATE__THRESHOLD=0.7
```

## Preset Configurations

### Implementation

```rust
impl CircuitBreakerConfig {
    /// Default/production preset (balanced)
    pub fn default_preset() -> Self {
        Self {
            detection: FailureDetectionConfig {
                consecutive_failures: Some(ConsecutiveFailureConfig { threshold: 5 }),
                failure_rate: Some(FailureRateConfig {
                    threshold: 0.5,
                    minimum_requests: 10,
                    window: WindowConfig::TimeBased {
                        duration: Duration::from_secs(60),
                    },
                }),
                slow_call: Some(SlowCallConfig {
                    threshold: Duration::from_secs(5),
                    rate_threshold: 0.5,
                    minimum_requests: 10,
                    window_duration: Duration::from_secs(60),
                }),
                strategy: DetectionStrategy::Any,
            },
            transitions: StateTransitionConfig {
                open_timeout: Duration::from_secs(30),
                enable_exponential_backoff: true,
                backoff_multiplier: 2.0,
                max_backoff_duration: Duration::from_secs(300),
            },
            recovery: RecoveryConfig {
                success_threshold: 3,
                max_concurrent_probes: 3,
                failure_threshold: 1,
                strict_mode: false,
                success_rate: 0.8,
                minimum_probes: 3,
            },
            timeouts: TimeoutConfig {
                request_timeout: Some(Duration::from_secs(30)),
                count_timeouts_as_failures: true,
            },
            persistence: None,
        }
    }

    /// Aggressive preset (fail fast)
    pub fn aggressive_preset() -> Self {
        Self {
            detection: FailureDetectionConfig {
                consecutive_failures: Some(ConsecutiveFailureConfig { threshold: 3 }),
                failure_rate: Some(FailureRateConfig {
                    threshold: 0.3,
                    minimum_requests: 5,
                    window: WindowConfig::TimeBased {
                        duration: Duration::from_secs(30),
                    },
                }),
                slow_call: Some(SlowCallConfig {
                    threshold: Duration::from_secs(2),
                    rate_threshold: 0.3,
                    minimum_requests: 5,
                    window_duration: Duration::from_secs(30),
                }),
                strategy: DetectionStrategy::Any,
            },
            transitions: StateTransitionConfig {
                open_timeout: Duration::from_secs(10),
                enable_exponential_backoff: true,
                backoff_multiplier: 2.0,
                max_backoff_duration: Duration::from_secs(120),
            },
            recovery: RecoveryConfig {
                success_threshold: 5,
                max_concurrent_probes: 2,
                failure_threshold: 1,
                strict_mode: true,
                success_rate: 1.0,
                minimum_probes: 5,
            },
            timeouts: TimeoutConfig {
                request_timeout: Some(Duration::from_secs(10)),
                count_timeouts_as_failures: true,
            },
            persistence: None,
        }
    }

    /// Lenient preset (graceful degradation)
    pub fn lenient_preset() -> Self {
        Self {
            detection: FailureDetectionConfig {
                consecutive_failures: Some(ConsecutiveFailureConfig { threshold: 10 }),
                failure_rate: Some(FailureRateConfig {
                    threshold: 0.7,
                    minimum_requests: 20,
                    window: WindowConfig::TimeBased {
                        duration: Duration::from_secs(120),
                    },
                }),
                slow_call: Some(SlowCallConfig {
                    threshold: Duration::from_secs(10),
                    rate_threshold: 0.7,
                    minimum_requests: 20,
                    window_duration: Duration::from_secs(120),
                }),
                strategy: DetectionStrategy::All,
            },
            transitions: StateTransitionConfig {
                open_timeout: Duration::from_secs(60),
                enable_exponential_backoff: false,
                backoff_multiplier: 1.5,
                max_backoff_duration: Duration::from_secs(600),
            },
            recovery: RecoveryConfig {
                success_threshold: 2,
                max_concurrent_probes: 5,
                failure_threshold: 3,
                strict_mode: false,
                success_rate: 0.6,
                minimum_probes: 2,
            },
            timeouts: TimeoutConfig {
                request_timeout: Some(Duration::from_secs(60)),
                count_timeouts_as_failures: false,
            },
            persistence: None,
        }
    }
}
```

## Configuration Examples by Use Case

### High-Traffic API Gateway

```yaml
detection:
  failure_rate:
    threshold: 0.3
    minimum_requests: 100
    window:
      type: time_based
      duration: 30s
  strategy: any

transitions:
  open_timeout: 5s
  enable_exponential_backoff: true
  backoff_multiplier: 2.0
  max_backoff_duration: 2m

recovery:
  success_threshold: 10
  max_concurrent_probes: 50
  strict_mode: false
  success_rate: 0.95
```

### Background Job Processor

```yaml
detection:
  consecutive_failures:
    threshold: 20
  slow_call:
    threshold: 30s
    rate_threshold: 0.8

transitions:
  open_timeout: 5m
  enable_exponential_backoff: false

recovery:
  success_threshold: 1
  strict_mode: false

timeouts:
  request_timeout: 5m
  count_timeouts_as_failures: false
```

### Real-Time Streaming Service

```yaml
detection:
  consecutive_failures:
    threshold: 2
  failure_rate:
    threshold: 0.1
    minimum_requests: 5
    window:
      type: count_based
      size: 20

transitions:
  open_timeout: 1s
  enable_exponential_backoff: true
  max_backoff_duration: 30s

recovery:
  success_threshold: 3
  strict_mode: true
  success_rate: 1.0
```

## Related Documents

- [State Machine Design](./CIRCUIT_BREAKER_STATE_MACHINE.md)
- [Core Architecture](./CIRCUIT_BREAKER_CORE_ARCHITECTURE.md)
- [Failure Detection Strategies](./CIRCUIT_BREAKER_FAILURE_DETECTION.md)
- [Integration Patterns](./CIRCUIT_BREAKER_INTEGRATION_PATTERNS.md)
