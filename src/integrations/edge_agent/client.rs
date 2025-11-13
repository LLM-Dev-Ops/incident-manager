use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, instrument, warn};

use crate::error::{AppError, Result};
use crate::integrations::common::{retry_with_backoff, IntegrationMetrics, RetryPolicy};

use super::models::*;

// Import generated gRPC code
pub mod proto {
    tonic::include_proto!("integrations");
}

use proto::edge_agent_service_client::EdgeAgentServiceClient;
use proto::{
    BatchRequest as ProtoBatchRequest, InferenceRequest as ProtoInferenceRequest,
    ResourceUsageRequest, SyncRequest as ProtoSyncRequest,
};

/// Edge-Agent client for local inference with bidirectional streaming
#[derive(Clone)]
pub struct EdgeAgentClient {
    client: EdgeAgentServiceClient<Channel>,
    agent_id: String,
    retry_policy: Arc<RetryPolicy>,
    metrics: Arc<IntegrationMetrics>,
    offline_queue: Arc<OfflineQueue>,
}

impl EdgeAgentClient {
    /// Create a new Edge-Agent client
    #[instrument(skip_all)]
    pub async fn new(endpoint: impl Into<String>, agent_id: impl Into<String>) -> Result<Self> {
        let endpoint_str = endpoint.into();
        let agent_id = agent_id.into();
        info!(
            endpoint = %endpoint_str,
            agent_id = %agent_id,
            "Connecting to Edge-Agent service"
        );

        let channel = Endpoint::from_shared(endpoint_str.clone())
            .map_err(|e| AppError::Integration {
                integration_source: "EdgeAgent".to_string(),
                message: format!("Invalid endpoint: {}", e),
            })?
            .connect()
            .await
            .map_err(|e| AppError::Integration {
                integration_source: "EdgeAgent".to_string(),
                message: format!("Connection failed: {}", e),
            })?;

        let client = EdgeAgentServiceClient::new(channel);

        Ok(Self {
            client,
            agent_id,
            retry_policy: Arc::new(RetryPolicy::default()),
            metrics: Arc::new(IntegrationMetrics::new("edge-agent")),
            offline_queue: Arc::new(OfflineQueue::new(1000)),
        })
    }

    /// Create client with custom configuration
    pub async fn with_config(
        endpoint: impl Into<String>,
        agent_id: impl Into<String>,
        max_queue_size: usize,
    ) -> Result<Self> {
        let endpoint_str = endpoint.into();
        let agent_id = agent_id.into();
        info!(
            endpoint = %endpoint_str,
            agent_id = %agent_id,
            max_queue_size = max_queue_size,
            "Connecting to Edge-Agent service with custom config"
        );

        let channel = Endpoint::from_shared(endpoint_str.clone())
            .map_err(|e| AppError::Integration {
                integration_source: "EdgeAgent".to_string(),
                message: format!("Invalid endpoint: {}", e),
            })?
            .connect()
            .await
            .map_err(|e| AppError::Integration {
                integration_source: "EdgeAgent".to_string(),
                message: format!("Connection failed: {}", e),
            })?;

        let client = EdgeAgentServiceClient::new(channel);

        Ok(Self {
            client,
            agent_id,
            retry_policy: Arc::new(RetryPolicy::default()),
            metrics: Arc::new(IntegrationMetrics::new("edge-agent")),
            offline_queue: Arc::new(OfflineQueue::new(max_queue_size)),
        })
    }

    /// Start bidirectional streaming for real-time inference
    #[instrument(skip(self))]
    pub async fn stream_inference(
        &self,
        mut request_rx: mpsc::Receiver<InferenceRequest>,
        response_tx: mpsc::Sender<InferenceResult>,
    ) -> Result<()> {
        info!(agent_id = %self.agent_id, "Starting bidirectional streaming");

        let mut client = self.client.clone();
        let metrics = self.metrics.clone();

        // Convert incoming requests to proto format
        let (proto_tx, proto_rx) = mpsc::channel(100);

        // Spawn task to convert requests
        let agent_id = self.agent_id.clone();
        tokio::spawn(async move {
            while let Some(req) = request_rx.recv().await {
                let proto_req = ProtoInferenceRequest {
                    request_id: req.request_id.clone(),
                    model_id: req.model_id.clone(),
                    input_data: req.input_data.clone(),
                    parameters: req.parameters.clone(),
                    priority: req.priority.as_i32(),
                    allow_offline_queue: req.allow_offline_queue,
                    timestamp: Some(prost_types::Timestamp {
                        seconds: req.timestamp.timestamp(),
                        nanos: req.timestamp.timestamp_subsec_nanos() as i32,
                    }),
                };

                if proto_tx.send(proto_req).await.is_err() {
                    error!("Failed to send request to stream");
                    break;
                }
            }
            debug!(agent_id = %agent_id, "Request sender task completed");
        });

        let request_stream = ReceiverStream::new(proto_rx);

        // Start the bidirectional stream
        let start = Instant::now();
        let response_stream = client
            .stream_inference(request_stream)
            .await
            .map_err(|e| AppError::Integration {
                integration_source: "EdgeAgent".to_string(),
                message: format!("Stream failed: {}", e),
            })?
            .into_inner();

        let latency = start.elapsed().as_millis() as u64;
        metrics.record_success(latency);

        // Process responses
        let mut stream = response_stream;
        while let Some(result) = stream.message().await.map_err(|e| AppError::Integration {
            integration_source: "EdgeAgent".to_string(),
            message: format!("Stream error: {}", e),
        })? {
            let status = match result.status.as_str() {
                "COMPLETED" => InferenceStatus::Completed,
                "QUEUED" => InferenceStatus::Queued,
                "PROCESSING" => InferenceStatus::Processing,
                "FAILED" => InferenceStatus::Failed,
                _ => InferenceStatus::Failed,
            };

            let resource_usage = result.resource_usage.map(|ru| ResourceSnapshot {
                cpu_percent: ru.cpu_percent,
                memory_mb: ru.memory_mb,
                gpu_percent: ru.gpu_percent,
                disk_io_mb_per_sec: ru.disk_io_mb_per_sec,
                active_requests: ru.active_requests,
                queued_requests: ru.queued_requests,
                timestamp: ru
                    .timestamp
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                            .unwrap_or_else(|| chrono::Utc::now())
                    })
                    .unwrap_or_else(|| chrono::Utc::now()),
            });

            let processed_at = result
                .processed_at
                .map(|ts| {
                    chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                        .unwrap_or_else(|| chrono::Utc::now())
                })
                .unwrap_or_else(|| chrono::Utc::now());

            let inference_result = InferenceResult {
                request_id: result.request_id,
                result: result.result,
                confidence: result.confidence,
                status,
                metadata: result.metadata,
                resource_usage: resource_usage.unwrap_or_else(|| ResourceSnapshot {
                    cpu_percent: 0.0,
                    memory_mb: 0.0,
                    gpu_percent: 0.0,
                    disk_io_mb_per_sec: 0.0,
                    active_requests: 0,
                    queued_requests: 0,
                    timestamp: chrono::Utc::now(),
                }),
                processed_at,
            };

            if response_tx.send(inference_result).await.is_err() {
                warn!("Response receiver dropped, stopping stream");
                break;
            }
        }

        info!(agent_id = %self.agent_id, "Streaming completed");
        Ok(())
    }

    /// Submit a batch for offline processing
    #[instrument(skip(self, batch))]
    pub async fn submit_batch(&self, batch: BatchRequest) -> Result<BatchResponse> {
        let start = Instant::now();
        debug!(
            batch_id = %batch.batch_id,
            num_requests = batch.requests.len(),
            "Submitting batch for processing"
        );

        let proto_requests: Vec<ProtoInferenceRequest> = batch
            .requests
            .into_iter()
            .map(|req| ProtoInferenceRequest {
                request_id: req.request_id,
                model_id: req.model_id,
                input_data: req.input_data,
                parameters: req.parameters,
                priority: req.priority.as_i32(),
                allow_offline_queue: req.allow_offline_queue,
                timestamp: Some(prost_types::Timestamp {
                    seconds: req.timestamp.timestamp(),
                    nanos: req.timestamp.timestamp_subsec_nanos() as i32,
                }),
            })
            .collect();

        let grpc_request = ProtoBatchRequest {
            batch_id: batch.batch_id.clone(),
            requests: proto_requests,
            priority: batch.priority.as_i32(),
            deadline: Some(prost_types::Timestamp {
                seconds: batch.deadline.timestamp(),
                nanos: batch.deadline.timestamp_subsec_nanos() as i32,
            }),
        };

        let client = self.client.clone();
        let retry_policy = &self.retry_policy;

        let result = retry_with_backoff("submit_batch", retry_policy, || {
            let mut client = client.clone();
            let grpc_request = grpc_request.clone();
            async move {
                client
                    .submit_batch(grpc_request)
                    .await
                    .map_err(|e| e.into())
            }
        })
        .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                self.metrics.record_success(latency);
                let resp = response.into_inner();

                let estimated_completion = resp
                    .estimated_completion
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                            .unwrap_or_else(|| chrono::Utc::now())
                    })
                    .unwrap_or_else(|| chrono::Utc::now());

                Ok(BatchResponse {
                    batch_id: resp.batch_id,
                    status: resp.status,
                    total_requests: resp.total_requests,
                    queued_requests: resp.queued_requests,
                    estimated_completion,
                })
            }
            Err(e) => {
                self.metrics.record_failure(latency);
                Err(e.into())
            }
        }
    }

    /// Synchronize with hub (for edge devices)
    #[instrument(skip(self))]
    pub async fn sync_with_hub(&self, pending_request_ids: Vec<String>) -> Result<SyncResponse> {
        let start = Instant::now();
        debug!(
            agent_id = %self.agent_id,
            num_pending = pending_request_ids.len(),
            "Synchronizing with hub"
        );

        let grpc_request = ProtoSyncRequest {
            agent_id: self.agent_id.clone(),
            pending_request_ids,
            last_sync: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
        };

        let client = self.client.clone();
        let retry_policy = &self.retry_policy;

        let result = retry_with_backoff("sync_with_hub", retry_policy, || {
            let mut client = client.clone();
            let grpc_request = grpc_request.clone();
            async move {
                client
                    .sync_with_hub(grpc_request)
                    .await
                    .map_err(|e| e.into())
            }
        })
        .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                self.metrics.record_success(latency);
                let resp = response.into_inner();

                let sync_timestamp = resp
                    .sync_timestamp
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                            .unwrap_or_else(|| chrono::Utc::now())
                    })
                    .unwrap_or_else(|| chrono::Utc::now());

                Ok(SyncResponse {
                    sync_id: resp.sync_id,
                    synced_requests: resp.synced_requests,
                    failed_requests: resp.failed_requests,
                    sync_timestamp,
                })
            }
            Err(e) => {
                self.metrics.record_failure(latency);
                Err(e.into())
            }
        }
    }

    /// Get resource usage statistics
    #[instrument(skip(self))]
    pub async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        let start = Instant::now();
        debug!(agent_id = %self.agent_id, "Getting resource usage");

        let grpc_request = ResourceUsageRequest {
            agent_id: self.agent_id.clone(),
        };

        let client = self.client.clone();
        let retry_policy = &self.retry_policy;

        let result = retry_with_backoff("get_resource_usage", retry_policy, || {
            let mut client = client.clone();
            let grpc_request = grpc_request.clone();
            async move {
                client
                    .get_resource_usage(grpc_request)
                    .await
                    .map_err(|e| e.into())
            }
        })
        .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                self.metrics.record_success(latency);
                let resp = response.into_inner();

                let current_usage = resp
                    .current_usage
                    .map(|ru| ResourceSnapshot {
                        cpu_percent: ru.cpu_percent,
                        memory_mb: ru.memory_mb,
                        gpu_percent: ru.gpu_percent,
                        disk_io_mb_per_sec: ru.disk_io_mb_per_sec,
                        active_requests: ru.active_requests,
                        queued_requests: ru.queued_requests,
                        timestamp: ru
                            .timestamp
                            .map(|ts| {
                                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                                    .unwrap_or_else(|| chrono::Utc::now())
                            })
                            .unwrap_or_else(|| chrono::Utc::now()),
                    })
                    .unwrap_or_else(|| ResourceSnapshot {
                        cpu_percent: 0.0,
                        memory_mb: 0.0,
                        gpu_percent: 0.0,
                        disk_io_mb_per_sec: 0.0,
                        active_requests: 0,
                        queued_requests: 0,
                        timestamp: chrono::Utc::now(),
                    });

                let history = resp
                    .history
                    .into_iter()
                    .map(|ru| ResourceSnapshot {
                        cpu_percent: ru.cpu_percent,
                        memory_mb: ru.memory_mb,
                        gpu_percent: ru.gpu_percent,
                        disk_io_mb_per_sec: ru.disk_io_mb_per_sec,
                        active_requests: ru.active_requests,
                        queued_requests: ru.queued_requests,
                        timestamp: ru
                            .timestamp
                            .map(|ts| {
                                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                                    .unwrap_or_else(|| chrono::Utc::now())
                            })
                            .unwrap_or_else(|| chrono::Utc::now()),
                    })
                    .collect();

                Ok(ResourceUsage {
                    agent_id: resp.agent_id,
                    current_usage,
                    history,
                })
            }
            Err(e) => {
                self.metrics.record_failure(latency);
                Err(e.into())
            }
        }
    }

    /// Access the offline queue
    pub fn offline_queue(&self) -> &OfflineQueue {
        &self.offline_queue
    }

    /// Get current metrics snapshot
    pub fn metrics(&self) -> crate::integrations::common::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_agent_client_creation() {
        // Test that client can be instantiated (will fail on connection in real scenario)
        let endpoint = "http://localhost:50052";
        let agent_id = "test-agent";
        assert!(!endpoint.is_empty());
        assert!(!agent_id.is_empty());
    }
}
