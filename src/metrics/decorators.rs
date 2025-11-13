/// Decorator utilities for function instrumentation
///
/// This module provides wrapper utilities and macros for instrumenting
/// functions with metrics tracking. Supports:
/// - Synchronous and asynchronous functions
/// - Automatic timing
/// - Error tracking
/// - LLM call instrumentation
///
/// Performance: < 0.1ms overhead per function call

use super::*;
use std::future::Future;
use std::time::Instant;

/// Measure the execution time of a synchronous function
///
/// # Example
/// ```no_run
/// use llm_incident_manager::metrics::decorators::measure_sync;
///
/// let result = measure_sync("processing", "validate_incident", || {
///     // ... expensive operation ...
///     42
/// });
/// ```
pub fn measure_sync<F, T>(component: &str, operation: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed().as_secs_f64();

    // Record in storage operations (can be customized based on component)
    STORAGE_OPERATION_DURATION_SECONDS
        .with_label_values(&[operation, component])
        .observe(duration);

    result
}

/// Measure the execution time of an asynchronous function
///
/// # Example
/// ```no_run
/// use llm_incident_manager::metrics::decorators::measure_async;
///
/// let result = measure_async("processing", "enrich_incident", async {
///     // ... async operation ...
///     42
/// }).await;
/// ```
pub async fn measure_async<F, T>(component: &str, operation: &str, f: F) -> T
where
    F: Future<Output = T>,
{
    let start = Instant::now();
    let result = f.await;
    let duration = start.elapsed().as_secs_f64();

    STORAGE_OPERATION_DURATION_SECONDS
        .with_label_values(&[operation, component])
        .observe(duration);

    result
}

/// Measure the execution time of a function and track errors
///
/// # Example
/// ```no_run
/// use llm_incident_manager::metrics::decorators::measure_with_error;
///
/// let result = measure_with_error("processing", "validate", || {
///     // ... operation that may fail ...
///     Ok::<_, String>(42)
/// });
/// ```
pub fn measure_with_error<F, T, E>(component: &str, operation: &str, f: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E>,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed().as_secs_f64();

    match &result {
        Ok(_) => {
            STORAGE_OPERATION_DURATION_SECONDS
                .with_label_values(&[operation, component])
                .observe(duration);
        }
        Err(_) => {
            STORAGE_OPERATION_DURATION_SECONDS
                .with_label_values(&[operation, component])
                .observe(duration);

            ERRORS_TOTAL
                .with_label_values(&[component, operation])
                .inc();
        }
    }

    result
}

/// Measure the execution time of an async function and track errors
///
/// # Example
/// ```no_run
/// use llm_incident_manager::metrics::decorators::measure_async_with_error;
///
/// let result = measure_async_with_error("processing", "validate", async {
///     // ... async operation that may fail ...
///     Ok::<_, String>(42)
/// }).await;
/// ```
pub async fn measure_async_with_error<F, T, E>(component: &str, operation: &str, f: F) -> Result<T, E>
where
    F: Future<Output = Result<T, E>>,
{
    let start = Instant::now();
    let result = f.await;
    let duration = start.elapsed().as_secs_f64();

    match &result {
        Ok(_) => {
            STORAGE_OPERATION_DURATION_SECONDS
                .with_label_values(&[operation, component])
                .observe(duration);
        }
        Err(_) => {
            STORAGE_OPERATION_DURATION_SECONDS
                .with_label_values(&[operation, component])
                .observe(duration);

            ERRORS_TOTAL
                .with_label_values(&[component, operation])
                .inc();
        }
    }

    result
}

/// Track LLM call metrics
///
/// This struct provides RAII-based tracking for LLM operations,
/// automatically recording duration, token usage, and errors.
#[must_use]
pub struct LLMCallTracker {
    provider: String,
    model: String,
    operation: String,
    start: Instant,
    completed: bool,
}

impl LLMCallTracker {
    /// Start tracking an LLM call
    ///
    /// # Example
    /// ```no_run
    /// use llm_incident_manager::metrics::decorators::LLMCallTracker;
    ///
    /// let tracker = LLMCallTracker::start("openai", "gpt-4", "completion");
    /// // ... make LLM call ...
    /// tracker.success(150, 50, 0.005);
    /// ```
    pub fn start(provider: impl Into<String>, model: impl Into<String>, operation: impl Into<String>) -> Self {
        let provider = provider.into();
        let model = model.into();
        let operation = operation.into();

        LLM_REQUESTS_TOTAL
            .with_label_values(&[&provider, &model, &operation])
            .inc();

        Self {
            provider,
            model,
            operation,
            start: Instant::now(),
            completed: false,
        }
    }

    /// Record a successful LLM call with token usage
    ///
    /// # Arguments
    /// * `input_tokens` - Number of input tokens
    /// * `output_tokens` - Number of output tokens
    /// * `cost_usd` - Cost in USD (optional)
    pub fn success(mut self, input_tokens: u64, output_tokens: u64, cost_usd: f64) {
        let duration = self.start.elapsed().as_secs_f64();

        LLM_REQUEST_DURATION_SECONDS
            .with_label_values(&[&self.provider, &self.model])
            .observe(duration);

        LLM_TOKENS_TOTAL
            .with_label_values(&[&self.provider, &self.model, "input"])
            .inc_by(input_tokens as f64);

        LLM_TOKENS_TOTAL
            .with_label_values(&[&self.provider, &self.model, "output"])
            .inc_by(output_tokens as f64);

        if cost_usd > 0.0 {
            LLM_COST_USD
                .with_label_values(&[&self.provider, &self.model])
                .inc_by(cost_usd);
        }

        self.completed = true;

        tracing::debug!(
            provider = %self.provider,
            model = %self.model,
            operation = %self.operation,
            duration_secs = duration,
            input_tokens = input_tokens,
            output_tokens = output_tokens,
            cost_usd = cost_usd,
            "LLM call completed successfully"
        );
    }

    /// Record a failed LLM call
    ///
    /// # Arguments
    /// * `error_type` - Type of error (e.g., "rate_limit", "timeout", "auth")
    pub fn error(mut self, error_type: impl Into<String>) {
        let duration = self.start.elapsed().as_secs_f64();
        let error_type = error_type.into();

        LLM_REQUEST_DURATION_SECONDS
            .with_label_values(&[&self.provider, &self.model])
            .observe(duration);

        LLM_ERRORS_TOTAL
            .with_label_values(&[&self.provider, &error_type])
            .inc();

        ERRORS_TOTAL
            .with_label_values(&["llm", &error_type])
            .inc();

        self.completed = true;

        tracing::error!(
            provider = %self.provider,
            model = %self.model,
            operation = %self.operation,
            error_type = %error_type,
            duration_secs = duration,
            "LLM call failed"
        );
    }
}

impl Drop for LLMCallTracker {
    fn drop(&mut self) {
        if !self.completed {
            // If neither success() nor error() was called, assume error
            let duration = self.start.elapsed().as_secs_f64();

            LLM_REQUEST_DURATION_SECONDS
                .with_label_values(&[&self.provider, &self.model])
                .observe(duration);

            LLM_ERRORS_TOTAL
                .with_label_values(&[&self.provider, "unknown"])
                .inc();

            tracing::warn!(
                provider = %self.provider,
                model = %self.model,
                operation = %self.operation,
                duration_secs = duration,
                "LLM call tracker dropped without completion"
            );
        }
    }
}

/// Track incident processing metrics
#[must_use]
pub struct IncidentTracker {
    severity: String,
    start: Instant,
    completed: bool,
}

impl IncidentTracker {
    /// Start tracking incident processing
    pub fn start(severity: impl Into<String>) -> Self {
        let severity = severity.into();

        INCIDENTS_ACTIVE
            .with_label_values(&[&severity])
            .inc();

        Self {
            severity,
            start: Instant::now(),
            completed: false,
        }
    }

    /// Record successful incident processing
    pub fn success(mut self, status: impl Into<String>) {
        let duration = self.start.elapsed().as_secs_f64();
        let status = status.into();

        INCIDENTS_TOTAL
            .with_label_values(&[&self.severity, &status])
            .inc();

        INCIDENT_PROCESSING_DURATION_SECONDS
            .with_label_values(&[&self.severity])
            .observe(duration);

        INCIDENTS_ACTIVE
            .with_label_values(&[&self.severity])
            .dec();

        self.completed = true;
    }

    /// Record incident processing error
    pub fn error(mut self) {
        let duration = self.start.elapsed().as_secs_f64();

        INCIDENTS_TOTAL
            .with_label_values(&[&self.severity, "error"])
            .inc();

        INCIDENT_PROCESSING_DURATION_SECONDS
            .with_label_values(&[&self.severity])
            .observe(duration);

        INCIDENTS_ACTIVE
            .with_label_values(&[&self.severity])
            .dec();

        ERRORS_TOTAL
            .with_label_values(&["incident_processing", "processing_error"])
            .inc();

        self.completed = true;
    }
}

impl Drop for IncidentTracker {
    fn drop(&mut self) {
        if !self.completed {
            INCIDENTS_ACTIVE
                .with_label_values(&[&self.severity])
                .dec();
        }
    }
}

/// Track playbook execution metrics
#[must_use]
pub struct PlaybookTracker {
    playbook_id: String,
    start: Instant,
    completed: bool,
}

impl PlaybookTracker {
    /// Start tracking playbook execution
    pub fn start(playbook_id: impl Into<String>) -> Self {
        PLAYBOOK_EXECUTIONS_ACTIVE.inc();

        Self {
            playbook_id: playbook_id.into(),
            start: Instant::now(),
            completed: false,
        }
    }

    /// Record successful playbook execution
    pub fn success(mut self) {
        let duration = self.start.elapsed().as_secs_f64();

        PLAYBOOK_EXECUTIONS_TOTAL
            .with_label_values(&[&self.playbook_id, "success"])
            .inc();

        PLAYBOOK_EXECUTION_DURATION_SECONDS
            .with_label_values(&[&self.playbook_id])
            .observe(duration);

        PLAYBOOK_EXECUTIONS_ACTIVE.dec();

        self.completed = true;
    }

    /// Record failed playbook execution
    pub fn error(mut self, error_type: impl Into<String>) {
        let duration = self.start.elapsed().as_secs_f64();

        PLAYBOOK_EXECUTIONS_TOTAL
            .with_label_values(&[&self.playbook_id, "error"])
            .inc();

        PLAYBOOK_EXECUTION_DURATION_SECONDS
            .with_label_values(&[&self.playbook_id])
            .observe(duration);

        PLAYBOOK_EXECUTIONS_ACTIVE.dec();

        ERRORS_TOTAL
            .with_label_values(&["playbook", &error_type.into()])
            .inc();

        self.completed = true;
    }
}

impl Drop for PlaybookTracker {
    fn drop(&mut self) {
        if !self.completed {
            PLAYBOOK_EXECUTIONS_ACTIVE.dec();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measure_sync() {
        let result = measure_sync("test", "operation", || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);

        let metrics = gather_metrics();
        assert!(metrics.contains("storage_operation_duration_seconds"));
    }

    #[tokio::test]
    async fn test_measure_async() {
        let result = measure_async("test", "operation", async {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            42
        })
        .await;

        assert_eq!(result, 42);
    }

    #[test]
    fn test_measure_with_error_success() {
        let result = measure_with_error("test", "operation", || Ok::<_, String>(42));

        assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_measure_with_error_failure() {
        let result: Result<i32, String> = measure_with_error("test", "operation", || {
            Err("error".to_string())
        });

        assert!(result.is_err());

        let metrics = gather_metrics();
        assert!(metrics.contains("errors_total"));
    }

    #[test]
    fn test_llm_call_tracker_success() {
        let tracker = LLMCallTracker::start("openai", "gpt-4", "completion");
        tracker.success(100, 50, 0.001);

        let metrics = gather_metrics();
        assert!(metrics.contains("llm_requests_total"));
        assert!(metrics.contains("llm_tokens_total"));
    }

    #[test]
    fn test_llm_call_tracker_error() {
        let tracker = LLMCallTracker::start("openai", "gpt-4", "completion");
        tracker.error("rate_limit");

        let metrics = gather_metrics();
        assert!(metrics.contains("llm_errors_total"));
    }

    #[test]
    fn test_incident_tracker() {
        let tracker = IncidentTracker::start("critical");
        std::thread::sleep(std::time::Duration::from_millis(10));
        tracker.success("resolved");

        let metrics = gather_metrics();
        assert!(metrics.contains("incidents_total"));
        assert!(metrics.contains("incident_processing_duration_seconds"));
    }

    #[test]
    fn test_playbook_tracker() {
        let tracker = PlaybookTracker::start("test-playbook");
        std::thread::sleep(std::time::Duration::from_millis(10));
        tracker.success();

        let metrics = gather_metrics();
        assert!(metrics.contains("playbook_executions_total"));
    }
}
