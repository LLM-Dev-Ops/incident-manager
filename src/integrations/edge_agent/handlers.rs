use tokio::sync::mpsc;
use tracing::{info, warn};

use super::client::EdgeAgentClient;
use super::models::*;
use crate::error::Result;

/// Edge inference handler for processing requests
pub struct EdgeInferenceHandler {
    client: EdgeAgentClient,
}

impl EdgeInferenceHandler {
    /// Create a new edge inference handler
    pub fn new(client: EdgeAgentClient) -> Self {
        Self { client }
    }

    /// Process a single inference request
    pub async fn process_request(&self, request: InferenceRequest) -> Result<InferenceResult> {
        info!(
            request_id = %request.request_id,
            model_id = %request.model_id,
            priority = ?request.priority,
            "Processing edge inference request"
        );

        // For single requests, use batch submission with one item
        let batch = BatchRequest {
            batch_id: format!("single-{}", request.request_id),
            requests: vec![request.clone()],
            priority: request.priority.clone(),
            deadline: chrono::Utc::now() + chrono::Duration::seconds(300),
        };

        let batch_response = self.client.submit_batch(batch).await?;

        info!(
            batch_id = %batch_response.batch_id,
            status = %batch_response.status,
            "Batch submitted successfully"
        );

        // For now, return a placeholder result
        // In a real implementation, we'd poll for the result
        Ok(InferenceResult {
            request_id: request.request_id,
            result: "Processing".to_string(),
            confidence: 0.0,
            status: InferenceStatus::Queued,
            metadata: std::collections::HashMap::new(),
            resource_usage: ResourceSnapshot {
                cpu_percent: 0.0,
                memory_mb: 0.0,
                gpu_percent: 0.0,
                disk_io_mb_per_sec: 0.0,
                active_requests: 0,
                queued_requests: batch_response.queued_requests,
                timestamp: chrono::Utc::now(),
            },
            processed_at: chrono::Utc::now(),
        })
    }

    /// Start streaming inference processing
    pub async fn start_streaming(
        &self,
        request_rx: mpsc::Receiver<InferenceRequest>,
        response_tx: mpsc::Sender<InferenceResult>,
    ) -> Result<()> {
        info!("Starting edge streaming inference");
        self.client.stream_inference(request_rx, response_tx).await
    }

    /// Process a batch of requests
    pub async fn process_batch(&self, batch: BatchRequest) -> Result<BatchResponse> {
        info!(
            batch_id = %batch.batch_id,
            num_requests = batch.requests.len(),
            "Processing batch request"
        );

        self.client.submit_batch(batch).await
    }

    /// Synchronize offline queue with hub
    pub async fn sync_offline_queue(&self) -> Result<SyncResponse> {
        info!("Synchronizing offline queue with hub");

        let queue = self.client.offline_queue();
        let pending_requests = queue.get_all();
        let pending_ids: Vec<String> = pending_requests
            .iter()
            .map(|r| r.request_id.clone())
            .collect();

        if pending_ids.is_empty() {
            info!("No pending requests to sync");
            return Ok(SyncResponse {
                sync_id: uuid::Uuid::new_v4().to_string(),
                synced_requests: vec![],
                failed_requests: vec![],
                sync_timestamp: chrono::Utc::now(),
            });
        }

        let sync_response = self.client.sync_with_hub(pending_ids).await?;

        // Remove successfully synced requests from queue
        let synced_set: std::collections::HashSet<String> = sync_response
            .synced_requests
            .iter()
            .cloned()
            .collect();

        let updated_queue: Vec<InferenceRequest> = pending_requests
            .into_iter()
            .filter(|r| !synced_set.contains(&r.request_id))
            .collect();

        queue.clear();
        for req in updated_queue {
            if let Err(e) = queue.enqueue(req) {
                warn!(error = %e, "Failed to re-enqueue request");
            }
        }

        info!(
            synced = sync_response.synced_requests.len(),
            failed = sync_response.failed_requests.len(),
            "Sync completed"
        );

        Ok(sync_response)
    }

    /// Get current resource usage
    pub async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        self.client.get_resource_usage().await
    }

    /// Add request to offline queue
    pub fn enqueue_offline(&self, request: InferenceRequest) -> Result<()> {
        info!(
            request_id = %request.request_id,
            "Enqueuing request for offline processing"
        );

        self.client
            .offline_queue()
            .enqueue(request)
            .map_err(|e| crate::error::AppError::Integration {
                integration_source: "EdgeAgent".to_string(),
                message: e,
            })
    }

    /// Get offline queue size
    pub fn offline_queue_size(&self) -> usize {
        self.client.offline_queue().size()
    }
}

/// Resource-aware request prioritizer
pub struct ResourceAwarePrioritizer {
    cpu_threshold: f64,
    memory_threshold: f64,
    gpu_threshold: f64,
}

impl ResourceAwarePrioritizer {
    /// Create a new resource-aware prioritizer
    pub fn new() -> Self {
        Self {
            cpu_threshold: 80.0,
            memory_threshold: 80.0,
            gpu_threshold: 90.0,
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(cpu: f64, memory: f64, gpu: f64) -> Self {
        Self {
            cpu_threshold: cpu,
            memory_threshold: memory,
            gpu_threshold: gpu,
        }
    }

    /// Determine if system can handle a new request
    pub fn can_process(&self, resource_usage: &ResourceSnapshot) -> bool {
        resource_usage.cpu_percent < self.cpu_threshold
            && resource_usage.memory_mb < self.memory_threshold
            && resource_usage.gpu_percent < self.gpu_threshold
    }

    /// Adjust request priority based on resource availability
    pub fn adjust_priority(
        &self,
        request: &mut InferenceRequest,
        resource_usage: &ResourceSnapshot,
    ) {
        if !self.can_process(resource_usage) {
            // Downgrade priority if resources are constrained
            request.priority = match request.priority {
                InferencePriority::Critical => InferencePriority::High,
                InferencePriority::High => InferencePriority::Medium,
                InferencePriority::Medium => InferencePriority::Low,
                InferencePriority::Low => InferencePriority::Low,
            };

            info!(
                request_id = %request.request_id,
                new_priority = ?request.priority,
                cpu = resource_usage.cpu_percent,
                memory = resource_usage.memory_mb,
                gpu = resource_usage.gpu_percent,
                "Adjusted request priority due to resource constraints"
            );
        }
    }

    /// Recommend queue or process based on resources
    pub fn should_queue(&self, resource_usage: &ResourceSnapshot, priority: &InferencePriority) -> bool {
        // Critical requests always process
        if *priority == InferencePriority::Critical {
            return false;
        }

        // If resources are heavily constrained, queue all non-critical
        !self.can_process(resource_usage)
    }
}

impl Default for ResourceAwarePrioritizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_prioritizer() {
        let prioritizer = ResourceAwarePrioritizer::new();

        let low_usage = ResourceSnapshot {
            cpu_percent: 30.0,
            memory_mb: 500.0,
            gpu_percent: 40.0,
            disk_io_mb_per_sec: 10.0,
            active_requests: 2,
            queued_requests: 0,
            timestamp: chrono::Utc::now(),
        };

        assert!(prioritizer.can_process(&low_usage));

        let high_usage = ResourceSnapshot {
            cpu_percent: 95.0,
            memory_mb: 7500.0,
            gpu_percent: 95.0,
            disk_io_mb_per_sec: 100.0,
            active_requests: 10,
            queued_requests: 5,
            timestamp: chrono::Utc::now(),
        };

        assert!(!prioritizer.can_process(&high_usage));
    }

    #[test]
    fn test_priority_adjustment() {
        let prioritizer = ResourceAwarePrioritizer::new();

        let mut request = InferenceRequest {
            request_id: "test-1".to_string(),
            model_id: "model-1".to_string(),
            input_data: "test".to_string(),
            parameters: std::collections::HashMap::new(),
            priority: InferencePriority::High,
            allow_offline_queue: true,
            timestamp: chrono::Utc::now(),
        };

        let constrained_resources = ResourceSnapshot {
            cpu_percent: 95.0,
            memory_mb: 7500.0,
            gpu_percent: 95.0,
            disk_io_mb_per_sec: 100.0,
            active_requests: 10,
            queued_requests: 5,
            timestamp: chrono::Utc::now(),
        };

        prioritizer.adjust_priority(&mut request, &constrained_resources);
        assert_eq!(request.priority, InferencePriority::Medium);
    }

    #[test]
    fn test_should_queue_logic() {
        let prioritizer = ResourceAwarePrioritizer::new();

        let constrained = ResourceSnapshot {
            cpu_percent: 95.0,
            memory_mb: 7500.0,
            gpu_percent: 95.0,
            disk_io_mb_per_sec: 100.0,
            active_requests: 10,
            queued_requests: 5,
            timestamp: chrono::Utc::now(),
        };

        // Critical should never queue
        assert!(!prioritizer.should_queue(&constrained, &InferencePriority::Critical));

        // Others should queue under constrained resources
        assert!(prioritizer.should_queue(&constrained, &InferencePriority::High));
        assert!(prioritizer.should_queue(&constrained, &InferencePriority::Medium));
        assert!(prioritizer.should_queue(&constrained, &InferencePriority::Low));
    }
}
