use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Span type within the execution hierarchy: Core -> Repo -> Agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    Repo,
    Agent,
}

/// Terminal status of a completed span
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpanStatus {
    Ok,
    Failed,
}

/// An artifact produced by an agent during execution.
/// Must include a stable reference (ID, URI, hash, or filename).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub artifact_type: String,
    pub reference: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// A single execution span in the hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpan {
    pub span_id: Uuid,
    pub parent_span_id: Uuid,
    pub span_type: SpanType,
    pub name: String,
    pub repo_name: String,
    pub status: SpanStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub artifacts: Vec<Artifact>,
    pub metadata: serde_json::Value,
    pub children: Vec<ExecutionSpan>,
}

/// The complete execution graph returned with API responses.
/// Append-only, causally ordered via parent_span_id, JSON-serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGraph {
    pub execution_id: Uuid,
    pub repo_span: ExecutionSpan,
}

pub const REPO_NAME: &str = "incident-manager";
