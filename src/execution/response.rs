use serde::Serialize;

use super::types::ExecutionGraph;

/// Wraps any API response with the execution graph.
///
/// When `execution` is `Some`, the response includes the full span hierarchy.
/// When `None` (no execution context provided), the response is just the data.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResponse<T: Serialize> {
    #[serde(flatten)]
    pub data: T,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionGraph>,
}

impl<T: Serialize> ExecutionResponse<T> {
    pub fn new(data: T, graph: Option<ExecutionGraph>) -> Self {
        Self {
            data,
            execution: graph,
        }
    }

    pub fn without_execution(data: T) -> Self {
        Self {
            data,
            execution: None,
        }
    }
}
