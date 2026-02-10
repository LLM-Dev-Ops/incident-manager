use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

use super::types::{Artifact, ExecutionGraph, ExecutionSpan, SpanStatus, SpanType, REPO_NAME};

/// Thread-safe execution context that collects spans across async tasks.
///
/// All clones share the same inner span collector via Arc.
/// Uses parking_lot::Mutex (not tokio::sync::Mutex) so that AgentSpanGuard
/// can record spans in its synchronous Drop implementation.
#[derive(Clone)]
pub struct ExecutionContext {
    inner: Arc<ExecutionContextInner>,
}

struct ExecutionContextInner {
    execution_id: Uuid,
    parent_span_id: Uuid,
    repo_span_id: Uuid,
    repo_started_at: DateTime<Utc>,
    agent_spans: Mutex<Vec<ExecutionSpan>>,
}

impl ExecutionContext {
    /// Create a new execution context. Automatically initializes the repo-level span.
    ///
    /// - `execution_id`: correlation ID for this execution (from the Core)
    /// - `parent_span_id`: the Core's span ID that this repo is nested under
    pub fn new(execution_id: Uuid, parent_span_id: Uuid) -> Self {
        Self {
            inner: Arc::new(ExecutionContextInner {
                execution_id,
                parent_span_id,
                repo_span_id: Uuid::new_v4(),
                repo_started_at: Utc::now(),
                agent_spans: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn execution_id(&self) -> Uuid {
        self.inner.execution_id
    }

    pub fn repo_span_id(&self) -> Uuid {
        self.inner.repo_span_id
    }

    /// Start a new agent-level execution span.
    /// Returns an AgentSpanGuard that MUST be completed via `complete_ok` or `complete_err`.
    /// If dropped without completion, the span is marked as FAILED.
    pub fn start_agent_span(&self, agent_name: &str) -> AgentSpanGuard {
        AgentSpanGuard {
            ctx: self.clone(),
            span_id: Uuid::new_v4(),
            agent_name: agent_name.to_string(),
            started_at: Utc::now(),
            artifacts: Vec::new(),
            completed: false,
        }
    }

    /// Check whether any agent spans have been recorded.
    pub fn has_agent_spans(&self) -> bool {
        !self.inner.agent_spans.lock().is_empty()
    }

    /// Finalize the execution context into an ExecutionGraph.
    ///
    /// - Assembles repo span with nested agent spans as children
    /// - Sets repo status to FAILED if `overall_error` is provided or no agent spans exist
    /// - Computes duration_ms for the repo span
    pub fn finalize(self, overall_error: Option<&str>) -> ExecutionGraph {
        let ended_at = Utc::now();
        let agent_spans = self.inner.agent_spans.lock().clone();

        let has_failure = overall_error.is_some() || agent_spans.is_empty();

        let repo_status = if has_failure {
            SpanStatus::Failed
        } else {
            SpanStatus::Ok
        };

        let error = if agent_spans.is_empty() && overall_error.is_none() {
            Some("No agent-level spans were emitted during execution".to_string())
        } else {
            overall_error.map(|s| s.to_string())
        };

        let duration_ms =
            (ended_at - self.inner.repo_started_at).num_milliseconds().max(0) as u64;

        let repo_span = ExecutionSpan {
            span_id: self.inner.repo_span_id,
            parent_span_id: self.inner.parent_span_id,
            span_type: SpanType::Repo,
            name: REPO_NAME.to_string(),
            repo_name: REPO_NAME.to_string(),
            status: repo_status,
            started_at: self.inner.repo_started_at,
            ended_at: Some(ended_at),
            duration_ms: Some(duration_ms),
            error,
            artifacts: Vec::new(),
            metadata: serde_json::json!({}),
            children: agent_spans,
        };

        ExecutionGraph {
            execution_id: self.inner.execution_id,
            repo_span,
        }
    }

    /// Record a completed agent span. Called by AgentSpanGuard.
    fn record_agent_span(&self, span: ExecutionSpan) {
        self.inner.agent_spans.lock().push(span);
    }
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("execution_id", &self.inner.execution_id)
            .field("repo_span_id", &self.inner.repo_span_id)
            .finish()
    }
}

/// RAII guard for an agent execution span.
///
/// On explicit completion (`complete_ok` / `complete_err`), records the span
/// with the appropriate status. If dropped without completion, records a
/// FAILED span with "span abandoned" error (safety net).
pub struct AgentSpanGuard {
    ctx: ExecutionContext,
    span_id: Uuid,
    agent_name: String,
    started_at: DateTime<Utc>,
    artifacts: Vec<Artifact>,
    completed: bool,
}

impl AgentSpanGuard {
    /// Add an artifact to this agent span (can be called during execution).
    pub fn add_artifact(&mut self, artifact: Artifact) {
        self.artifacts.push(artifact);
    }

    /// Mark the agent span as successfully completed.
    pub fn complete_ok(mut self, mut artifacts: Vec<Artifact>) {
        self.artifacts.append(&mut artifacts);
        self.record_span(SpanStatus::Ok, None);
        self.completed = true;
    }

    /// Mark the agent span as failed with an error message.
    pub fn complete_err(mut self, error: String) {
        self.record_span(SpanStatus::Failed, Some(error));
        self.completed = true;
    }

    fn record_span(&self, status: SpanStatus, error: Option<String>) {
        let ended_at = Utc::now();
        let duration_ms = (ended_at - self.started_at).num_milliseconds().max(0) as u64;

        let span = ExecutionSpan {
            span_id: self.span_id,
            parent_span_id: self.ctx.repo_span_id(),
            span_type: SpanType::Agent,
            name: self.agent_name.clone(),
            repo_name: REPO_NAME.to_string(),
            status,
            started_at: self.started_at,
            ended_at: Some(ended_at),
            duration_ms: Some(duration_ms),
            error,
            artifacts: self.artifacts.clone(),
            metadata: serde_json::json!({}),
            children: Vec::new(),
        };

        self.ctx.record_agent_span(span);
    }
}

impl Drop for AgentSpanGuard {
    fn drop(&mut self) {
        if !self.completed {
            // Safety net: record a failed span if the guard was dropped without completion.
            // Uses parking_lot::Mutex which supports synchronous locking in Drop.
            self.record_span(SpanStatus::Failed, Some("Agent span abandoned without completion".to_string()));
            self.completed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let exec_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let ctx = ExecutionContext::new(exec_id, parent_id);

        assert_eq!(ctx.execution_id(), exec_id);
        assert!(!ctx.has_agent_spans());
    }

    #[test]
    fn test_agent_span_ok() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let guard = ctx.start_agent_span("TestAgent");
        guard.complete_ok(vec![]);

        assert!(ctx.has_agent_spans());

        let graph = ctx.finalize(None);
        assert_eq!(graph.repo_span.status, SpanStatus::Ok);
        assert_eq!(graph.repo_span.children.len(), 1);
        assert_eq!(graph.repo_span.children[0].name, "TestAgent");
        assert_eq!(graph.repo_span.children[0].status, SpanStatus::Ok);
    }

    #[test]
    fn test_agent_span_err() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let guard = ctx.start_agent_span("FailingAgent");
        guard.complete_err("something went wrong".to_string());

        let graph = ctx.finalize(None);
        assert_eq!(graph.repo_span.status, SpanStatus::Ok);
        assert_eq!(graph.repo_span.children.len(), 1);
        assert_eq!(graph.repo_span.children[0].status, SpanStatus::Failed);
        assert_eq!(
            graph.repo_span.children[0].error.as_deref(),
            Some("something went wrong")
        );
    }

    #[test]
    fn test_no_agent_spans_means_failed() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());
        let graph = ctx.finalize(None);

        assert_eq!(graph.repo_span.status, SpanStatus::Failed);
        assert!(graph.repo_span.error.as_ref().unwrap().contains("No agent-level spans"));
    }

    #[test]
    fn test_overall_error_marks_failed() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());
        let guard = ctx.start_agent_span("Agent");
        guard.complete_ok(vec![]);

        let graph = ctx.finalize(Some("operation failed"));
        assert_eq!(graph.repo_span.status, SpanStatus::Failed);
        assert_eq!(graph.repo_span.error.as_deref(), Some("operation failed"));
    }

    #[test]
    fn test_abandoned_span_marked_failed() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        {
            let _guard = ctx.start_agent_span("AbandonedAgent");
            // guard dropped without completion
        }

        let graph = ctx.finalize(None);
        assert_eq!(graph.repo_span.children.len(), 1);
        assert_eq!(graph.repo_span.children[0].status, SpanStatus::Failed);
        assert!(graph.repo_span.children[0]
            .error
            .as_ref()
            .unwrap()
            .contains("abandoned"));
    }

    #[test]
    fn test_multiple_agents() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let g1 = ctx.start_agent_span("Agent1");
        g1.complete_ok(vec![]);

        let g2 = ctx.start_agent_span("Agent2");
        g2.complete_err("failed".to_string());

        let g3 = ctx.start_agent_span("Agent3");
        g3.complete_ok(vec![Artifact {
            name: "result".to_string(),
            artifact_type: "test".to_string(),
            reference: "test-ref-123".to_string(),
            data: serde_json::json!({"key": "value"}),
            created_at: Utc::now(),
        }]);

        let graph = ctx.finalize(None);
        assert_eq!(graph.repo_span.children.len(), 3);
        assert_eq!(graph.repo_span.children[0].name, "Agent1");
        assert_eq!(graph.repo_span.children[1].name, "Agent2");
        assert_eq!(graph.repo_span.children[2].name, "Agent3");
        assert_eq!(graph.repo_span.children[2].artifacts.len(), 1);
    }

    #[test]
    fn test_graph_is_json_serializable() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());
        let guard = ctx.start_agent_span("SerAgent");
        guard.complete_ok(vec![]);

        let graph = ctx.finalize(None);
        let json = serde_json::to_string_pretty(&graph).expect("must serialize");
        assert!(json.contains("incident-manager"));
        assert!(json.contains("SerAgent"));

        // Round-trip
        let _: ExecutionGraph = serde_json::from_str(&json).expect("must deserialize");
    }

    #[test]
    fn test_hierarchy_structure() {
        let core_span_id = Uuid::new_v4();
        let ctx = ExecutionContext::new(Uuid::new_v4(), core_span_id);
        let repo_span_id = ctx.repo_span_id();

        let guard = ctx.start_agent_span("MyAgent");
        guard.complete_ok(vec![]);

        let graph = ctx.finalize(None);

        // Repo span parent is the Core span
        assert_eq!(graph.repo_span.parent_span_id, core_span_id);
        // Agent span parent is the repo span
        assert_eq!(graph.repo_span.children[0].parent_span_id, repo_span_id);
    }
}
