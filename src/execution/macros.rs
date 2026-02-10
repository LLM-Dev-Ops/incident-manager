/// Instruments a block of code as an agent execution span.
///
/// The block must return a `Result<T, E>` where `E: std::fmt::Display`.
/// On `Ok`, the span is marked as completed successfully.
/// On `Err`, the span is marked as failed with the error message.
///
/// The variable `_agent_guard` is available inside the block for calling
/// `add_artifact()` during execution.
///
/// # Usage
///
/// ```ignore
/// let result = execute_agent!(ctx, "DeduplicationEngine", {
///     self.dedup_engine.find_duplicate(&alert).await
/// });
/// ```
///
/// With artifact attachment:
/// ```ignore
/// let result = execute_agent!(ctx, "CorrelationEngine", {
///     let r = self.correlation_engine.analyze_incident(&incident).await;
///     if let Ok(ref res) = r {
///         _agent_guard.add_artifact(Artifact { ... });
///     }
///     r
/// });
/// ```
#[macro_export]
macro_rules! execute_agent {
    ($ctx:expr, $agent_name:expr, $body:expr) => {{
        let mut _agent_guard = $ctx.start_agent_span($agent_name);
        let result = { $body };
        match &result {
            Ok(_) => _agent_guard.complete_ok(vec![]),
            Err(e) => _agent_guard.complete_err(format!("{}", e)),
        }
        result
    }};
}

/// Instruments a block of code as an agent execution span for infallible operations.
///
/// The block can return any type (not necessarily Result). The span is always
/// marked as completed successfully.
///
/// # Usage
///
/// ```ignore
/// let executions = execute_agent_infallible!(ctx, "PlaybookService", {
///     playbook_service.auto_execute_for_incident(&incident).await
/// });
/// ```
#[macro_export]
macro_rules! execute_agent_infallible {
    ($ctx:expr, $agent_name:expr, $body:expr) => {{
        let mut _agent_guard = $ctx.start_agent_span($agent_name);
        let result = { $body };
        _agent_guard.complete_ok(vec![]);
        result
    }};
}
