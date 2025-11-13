use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context for edge inference requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeContext {
    pub agent_id: String,
    pub location: String,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Inference request for edge processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub request_id: String,
    pub model_id: String,
    pub input_data: String,
    pub parameters: HashMap<String, String>,
    pub priority: InferencePriority,
    pub allow_offline_queue: bool,
    pub timestamp: DateTime<Utc>,
}

/// Priority level for inference requests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum InferencePriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl InferencePriority {
    pub fn as_i32(&self) -> i32 {
        match self {
            InferencePriority::Low => 1,
            InferencePriority::Medium => 2,
            InferencePriority::High => 3,
            InferencePriority::Critical => 4,
        }
    }

    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => InferencePriority::Low,
            2 => InferencePriority::Medium,
            3 => InferencePriority::High,
            4 => InferencePriority::Critical,
            _ => InferencePriority::Medium,
        }
    }
}

/// Result of an inference request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub request_id: String,
    pub result: String,
    pub confidence: f64,
    pub status: InferenceStatus,
    pub metadata: HashMap<String, String>,
    pub resource_usage: ResourceSnapshot,
    pub processed_at: DateTime<Utc>,
}

/// Status of an inference request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InferenceStatus {
    Completed,
    Queued,
    Processing,
    Failed,
}

impl std::fmt::Display for InferenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceStatus::Completed => write!(f, "COMPLETED"),
            InferenceStatus::Queued => write!(f, "QUEUED"),
            InferenceStatus::Processing => write!(f, "PROCESSING"),
            InferenceStatus::Failed => write!(f, "FAILED"),
        }
    }
}

/// Snapshot of resource usage at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub gpu_percent: f64,
    pub disk_io_mb_per_sec: f64,
    pub active_requests: i32,
    pub queued_requests: i32,
    pub timestamp: DateTime<Utc>,
}

/// Overall resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub agent_id: String,
    pub current_usage: ResourceSnapshot,
    pub history: Vec<ResourceSnapshot>,
}

/// Batch of inference requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub batch_id: String,
    pub requests: Vec<InferenceRequest>,
    pub priority: InferencePriority,
    pub deadline: DateTime<Utc>,
}

/// Response from batch submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    pub batch_id: String,
    pub status: String,
    pub total_requests: i32,
    pub queued_requests: i32,
    pub estimated_completion: DateTime<Utc>,
}

/// Synchronization request for edge-hub communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub agent_id: String,
    pub pending_request_ids: Vec<String>,
    pub last_sync: DateTime<Utc>,
}

/// Response from synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub sync_id: String,
    pub synced_requests: Vec<String>,
    pub failed_requests: Vec<String>,
    pub sync_timestamp: DateTime<Utc>,
}

/// Streaming message types for bidirectional communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamMessage {
    Request(InferenceRequest),
    Response(InferenceResult),
    Heartbeat { timestamp: DateTime<Utc> },
    Error { message: String },
}

/// Queue management for offline processing
#[derive(Debug, Clone)]
pub struct OfflineQueue {
    pending_requests: std::sync::Arc<parking_lot::RwLock<Vec<InferenceRequest>>>,
    max_queue_size: usize,
}

impl OfflineQueue {
    /// Create a new offline queue
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            pending_requests: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
            max_queue_size,
        }
    }

    /// Add a request to the queue
    pub fn enqueue(&self, request: InferenceRequest) -> Result<(), String> {
        let mut queue = self.pending_requests.write();
        if queue.len() >= self.max_queue_size {
            return Err("Queue is full".to_string());
        }
        queue.push(request);
        Ok(())
    }

    /// Get the next request from the queue
    pub fn dequeue(&self) -> Option<InferenceRequest> {
        let mut queue = self.pending_requests.write();
        if queue.is_empty() {
            None
        } else {
            // Sort by priority and dequeue highest priority
            queue.sort_by(|a, b| b.priority.cmp(&a.priority));
            Some(queue.remove(0))
        }
    }

    /// Get all pending requests
    pub fn get_all(&self) -> Vec<InferenceRequest> {
        self.pending_requests.read().clone()
    }

    /// Clear the queue
    pub fn clear(&self) {
        self.pending_requests.write().clear();
    }

    /// Get queue size
    pub fn size(&self) -> usize {
        self.pending_requests.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(InferencePriority::Critical > InferencePriority::High);
        assert!(InferencePriority::High > InferencePriority::Medium);
        assert!(InferencePriority::Medium > InferencePriority::Low);
    }

    #[test]
    fn test_offline_queue() {
        let queue = OfflineQueue::new(10);

        let low_priority = InferenceRequest {
            request_id: "1".to_string(),
            model_id: "model-1".to_string(),
            input_data: "test".to_string(),
            parameters: HashMap::new(),
            priority: InferencePriority::Low,
            allow_offline_queue: true,
            timestamp: Utc::now(),
        };

        let high_priority = InferenceRequest {
            request_id: "2".to_string(),
            model_id: "model-1".to_string(),
            input_data: "test".to_string(),
            parameters: HashMap::new(),
            priority: InferencePriority::High,
            allow_offline_queue: true,
            timestamp: Utc::now(),
        };

        queue.enqueue(low_priority.clone()).unwrap();
        queue.enqueue(high_priority.clone()).unwrap();

        assert_eq!(queue.size(), 2);

        // Should dequeue high priority first
        let first = queue.dequeue().unwrap();
        assert_eq!(first.request_id, "2");

        let second = queue.dequeue().unwrap();
        assert_eq!(second.request_id, "1");

        assert_eq!(queue.size(), 0);
    }

    #[test]
    fn test_queue_capacity() {
        let queue = OfflineQueue::new(2);

        let req = InferenceRequest {
            request_id: "1".to_string(),
            model_id: "model-1".to_string(),
            input_data: "test".to_string(),
            parameters: HashMap::new(),
            priority: InferencePriority::Low,
            allow_offline_queue: true,
            timestamp: Utc::now(),
        };

        assert!(queue.enqueue(req.clone()).is_ok());
        assert!(queue.enqueue(req.clone()).is_ok());
        assert!(queue.enqueue(req.clone()).is_err());
    }
}


// Type aliases for Edge Agent responses
pub type InferenceResponse = InferenceResult;
pub type ResourceMetrics = ResourceUsage;

