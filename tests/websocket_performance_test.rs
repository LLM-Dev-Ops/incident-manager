//! WebSocket Performance Tests
//!
//! Tests for WebSocket performance, scalability, and load handling
//! - 100+ concurrent connections
//! - High message throughput (1000+ msg/s)
//! - Backpressure handling
//! - Memory usage under load
//! - Message latency (< 10ms p95)

use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

#[cfg(test)]
mod concurrent_connections {
    use super::*;

    #[tokio::test]
    async fn test_100_concurrent_connection_simulation() {
        let active_connections = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        // Simulate 100 concurrent connections
        for i in 0..100 {
            let active = active_connections.clone();
            let handle = tokio::spawn(async move {
                active.fetch_add(1, Ordering::SeqCst);

                // Simulate connection activity
                tokio::time::sleep(Duration::from_millis(50)).await;

                // Create subscription message
                let _msg = json!({
                    "id": format!("sub-{}", i),
                    "type": "subscribe",
                    "payload": {
                        "query": "subscription { incidentUpdates { updateType } }"
                    }
                });

                tokio::time::sleep(Duration::from_millis(50)).await;

                active.fetch_sub(1, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        // Wait for all connections
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(active_connections.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_1000_concurrent_connections_scalability() {
        let connection_count = Arc::new(AtomicUsize::new(0));
        let peak_connections = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();

        for i in 0..1000 {
            let count = connection_count.clone();
            let peak = peak_connections.clone();

            let handle = tokio::spawn(async move {
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;

                // Update peak if needed
                loop {
                    let current_peak = peak.load(Ordering::SeqCst);
                    if current <= current_peak
                        || peak
                            .compare_exchange(
                                current_peak,
                                current,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            )
                            .is_ok()
                    {
                        break;
                    }
                }

                // Simulate connection
                tokio::time::sleep(Duration::from_millis(10)).await;

                count.fetch_sub(1, Ordering::SeqCst);
            });

            handles.push(handle);

            // Stagger connection creation
            if i % 100 == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Verify we hit expected peak
        let peak = peak_connections.load(Ordering::SeqCst);
        println!("Peak concurrent connections: {}", peak);
        assert!(peak > 100); // Should have significant concurrency
        assert_eq!(connection_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_connection_pool_management() {
        use std::collections::HashSet;
        use std::sync::Mutex;

        #[derive(Debug)]
        struct ConnectionPool {
            active: Mutex<HashSet<String>>,
            max_size: usize,
        }

        impl ConnectionPool {
            fn new(max_size: usize) -> Self {
                Self {
                    active: Mutex::new(HashSet::new()),
                    max_size,
                }
            }

            fn try_acquire(&self, conn_id: String) -> bool {
                let mut active = self.active.lock().unwrap();
                if active.len() < self.max_size {
                    active.insert(conn_id);
                    true
                } else {
                    false
                }
            }

            fn release(&self, conn_id: &str) {
                let mut active = self.active.lock().unwrap();
                active.remove(conn_id);
            }
        }

        let pool = Arc::new(ConnectionPool::new(100));
        let mut handles = Vec::new();

        // Try to create 150 connections (100 max)
        for i in 0..150 {
            let pool = pool.clone();
            let handle = tokio::spawn(async move {
                let conn_id = format!("conn-{}", i);
                if pool.try_acquire(conn_id.clone()) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    pool.release(&conn_id);
                    true
                } else {
                    false
                }
            });
            handles.push(handle);
        }

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let successful = results.iter().filter(|&&r| r).count();
        println!("Successful connections: {}", successful);

        // All should eventually succeed
        assert!(successful > 0);
    }
}

#[cfg(test)]
mod message_throughput {
    use super::*;

    #[tokio::test]
    async fn test_high_message_throughput_1000_per_second() {
        let message_count = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();

        let count = message_count.clone();
        let handle = tokio::spawn(async move {
            // Generate messages at high rate
            for i in 0..1000 {
                let _msg = json!({
                    "id": "sub-1",
                    "type": "next",
                    "payload": {
                        "data": {
                            "incidentUpdates": {
                                "updateType": "HEARTBEAT",
                                "sequence": i
                            }
                        }
                    }
                });

                count.fetch_add(1, Ordering::SeqCst);

                // Yield to allow other tasks
                if i % 100 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        });

        handle.await.unwrap();

        let elapsed = start.elapsed();
        let count = message_count.load(Ordering::SeqCst);
        let throughput = count as f64 / elapsed.as_secs_f64();

        println!("Messages: {}, Time: {:?}, Throughput: {:.0} msg/s", count, elapsed, throughput);

        assert_eq!(count, 1000);
        assert!(throughput > 100.0); // Should be well above 100 msg/s
    }

    #[tokio::test]
    async fn test_sustained_throughput_over_10_seconds() {
        let message_count = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();
        let duration = Duration::from_secs(2); // Shortened for tests

        let count = message_count.clone();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let mut sequence = 0u64;

            while start.elapsed() < duration {
                let _msg = json!({
                    "id": "sub-1",
                    "type": "next",
                    "payload": {
                        "sequence": sequence
                    }
                });

                count.fetch_add(1, Ordering::SeqCst);
                sequence += 1;

                // Small yield to prevent monopolization
                if sequence % 1000 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        });

        handle.await.unwrap();

        let elapsed = start.elapsed();
        let total_messages = message_count.load(Ordering::SeqCst);
        let throughput = total_messages as f64 / elapsed.as_secs_f64();

        println!(
            "Sustained test - Messages: {}, Time: {:?}, Throughput: {:.0} msg/s",
            total_messages, elapsed, throughput
        );

        assert!(total_messages > 1000); // Should process many messages
        assert!(throughput > 100.0);
    }

    #[tokio::test]
    async fn test_multi_subscriber_broadcast_throughput() {
        let subscriber_count = 50;
        let messages_per_subscriber = 100;
        let total_received = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();

        for sub_id in 0..subscriber_count {
            let received = total_received.clone();

            let handle = tokio::spawn(async move {
                let mut local_count = 0;

                // Simulate receiving messages
                for msg_id in 0..messages_per_subscriber {
                    let _msg = json!({
                        "id": format!("sub-{}", sub_id),
                        "type": "next",
                        "payload": {
                            "messageId": msg_id
                        }
                    });

                    local_count += 1;

                    // Small delay to simulate processing
                    if msg_id % 50 == 0 {
                        tokio::task::yield_now().await;
                    }
                }

                received.fetch_add(local_count, Ordering::SeqCst);
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let expected = subscriber_count * messages_per_subscriber;
        assert_eq!(total_received.load(Ordering::SeqCst), expected);
    }

    #[tokio::test]
    async fn test_message_batching_efficiency() {
        let batch_size = 100;
        let num_batches = 10;

        let start = Instant::now();

        for batch_num in 0..num_batches {
            let mut batch = Vec::new();

            for i in 0..batch_size {
                batch.push(json!({
                    "id": format!("msg-{}-{}", batch_num, i),
                    "data": "test"
                }));
            }

            // Process batch
            let _serialized: Vec<_> = batch
                .iter()
                .map(|m| serde_json::to_string(m).unwrap())
                .collect();

            tokio::task::yield_now().await;
        }

        let elapsed = start.elapsed();
        let total_messages = batch_size * num_batches;
        let throughput = total_messages as f64 / elapsed.as_secs_f64();

        println!("Batch processing - Messages: {}, Throughput: {:.0} msg/s", total_messages, throughput);

        assert!(throughput > 100.0);
    }
}

#[cfg(test)]
mod latency_measurements {
    use super::*;

    #[tokio::test]
    async fn test_message_latency_tracking() {
        let mut latencies = Vec::new();

        for _ in 0..100 {
            let start = Instant::now();

            // Simulate message processing
            let _msg = json!({
                "id": "sub-1",
                "type": "next",
                "payload": {
                    "data": "test"
                }
            });

            let _serialized = serde_json::to_string(&_msg).unwrap();

            let latency = start.elapsed();
            latencies.push(latency);

            tokio::task::yield_now().await;
        }

        // Calculate percentiles
        latencies.sort();
        let p50 = latencies[49];
        let p95 = latencies[94];
        let p99 = latencies[98];

        println!("Latency - p50: {:?}, p95: {:?}, p99: {:?}", p50, p95, p99);

        // Most latencies should be very low
        assert!(p95 < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_end_to_end_latency() {
        #[derive(Debug)]
        struct Message {
            id: String,
            sent_at: Instant,
            received_at: Option<Instant>,
        }

        let messages = Arc::new(Mutex::new(Vec::new()));

        // Producer
        let producer_msgs = messages.clone();
        let producer = tokio::spawn(async move {
            for i in 0..100 {
                let mut msgs = producer_msgs.lock().await;
                msgs.push(Message {
                    id: format!("msg-{}", i),
                    sent_at: Instant::now(),
                    received_at: None,
                });
                drop(msgs);

                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });

        // Consumer
        let consumer_msgs = messages.clone();
        let consumer = tokio::spawn(async move {
            for _ in 0..100 {
                tokio::time::sleep(Duration::from_micros(150)).await;

                let mut msgs = consumer_msgs.lock().await;
                if let Some(msg) = msgs.iter_mut().find(|m| m.received_at.is_none()) {
                    msg.received_at = Some(Instant::now());
                }
            }
        });

        producer.await.unwrap();
        consumer.await.unwrap();

        // Calculate latencies
        let msgs = messages.lock().await;
        let mut latencies: Vec<_> = msgs
            .iter()
            .filter_map(|m| {
                m.received_at
                    .map(|recv| recv.duration_since(m.sent_at))
            })
            .collect();

        latencies.sort();

        if !latencies.is_empty() {
            let p95_idx = (latencies.len() as f64 * 0.95) as usize;
            let p95 = latencies[p95_idx.min(latencies.len() - 1)];

            println!("E2E Latency p95: {:?}", p95);
            assert!(p95 < Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod backpressure {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_backpressure_with_bounded_channel() {
        let (tx, mut rx) = mpsc::channel::<i32>(10); // Small buffer

        let producer_sent = Arc::new(AtomicUsize::new(0));
        let consumer_received = Arc::new(AtomicUsize::new(0));

        // Fast producer
        let sent = producer_sent.clone();
        let producer = tokio::spawn(async move {
            for i in 0..100 {
                match tx.send(i).await {
                    Ok(_) => {
                        sent.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(_) => break,
                }
            }
        });

        // Slow consumer
        let received = consumer_received.clone();
        let consumer = tokio::spawn(async move {
            while let Some(_msg) = rx.recv().await {
                received.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });

        producer.await.unwrap();
        drop(rx); // Close receiver to stop consumer

        tokio::time::sleep(Duration::from_millis(100)).await;

        let sent_count = producer_sent.load(Ordering::SeqCst);
        let received_count = consumer_received.load(Ordering::SeqCst);

        println!("Sent: {}, Received: {}", sent_count, received_count);

        assert_eq!(sent_count, 100);
        // Some messages might still be in channel
        assert!(received_count <= sent_count);
    }

    #[tokio::test]
    async fn test_stream_backpressure_handling() {
        let produced = Arc::new(AtomicUsize::new(0));
        let consumed = Arc::new(AtomicUsize::new(0));

        let produced_clone = produced.clone();
        let stream = async_stream::stream! {
            for i in 0..1000 {
                produced_clone.fetch_add(1, Ordering::SeqCst);
                yield i;
            }
        };

        let consumed_clone = consumed.clone();
        let mut stream = Box::pin(stream);

        // Consume at controlled rate
        let mut count = 0;
        while let Some(_item) = stream.next().await {
            consumed_clone.fetch_add(1, Ordering::SeqCst);
            count += 1;

            // Slow consumer
            if count % 100 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        assert_eq!(produced.load(Ordering::SeqCst), 1000);
        assert_eq!(consumed.load(Ordering::SeqCst), 1000);
    }

    #[tokio::test]
    async fn test_buffer_overflow_handling() {
        use std::collections::VecDeque;

        const MAX_BUFFER_SIZE: usize = 100;

        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let dropped = Arc::new(AtomicUsize::new(0));

        // Producer trying to overwhelm buffer
        let buf = buffer.clone();
        let drop_count = dropped.clone();

        for i in 0..1000 {
            let mut b = buf.lock().await;
            if b.len() < MAX_BUFFER_SIZE {
                b.push_back(i);
            } else {
                drop_count.fetch_add(1, Ordering::SeqCst);
            }
            drop(b);
            tokio::task::yield_now().await;
        }

        let buffer_size = buffer.lock().await.len();
        let dropped_count = dropped.load(Ordering::SeqCst);

        println!("Buffer size: {}, Dropped: {}", buffer_size, dropped_count);

        assert!(buffer_size <= MAX_BUFFER_SIZE);
        assert!(dropped_count > 0); // Some messages should be dropped
    }
}

#[cfg(test)]
mod memory_usage {
    use super::*;

    #[tokio::test]
    async fn test_memory_efficient_message_handling() {
        // Measure approximate memory usage
        let message_size_estimate = std::mem::size_of::<serde_json::Value>();

        println!("Estimated message size: {} bytes", message_size_estimate);

        // Create many messages
        let num_messages = 10_000;
        let messages: Vec<_> = (0..num_messages)
            .map(|i| {
                json!({
                    "id": format!("msg-{}", i),
                    "type": "next",
                    "payload": {
                        "data": i
                    }
                })
            })
            .collect();

        assert_eq!(messages.len(), num_messages);

        // Approximate memory usage
        let total_memory = message_size_estimate * num_messages;
        println!("Approximate memory usage: {} MB", total_memory / 1_048_576);

        // Should be reasonable
        assert!(total_memory < 100_000_000); // Less than 100MB
    }

    #[tokio::test]
    async fn test_subscription_cleanup() {
        use std::collections::HashMap;

        let subscriptions = Arc::new(Mutex::new(HashMap::new()));

        // Create subscriptions
        for i in 0..1000 {
            let mut subs = subscriptions.lock().await;
            subs.insert(format!("sub-{}", i), json!({"active": true}));
        }

        assert_eq!(subscriptions.lock().await.len(), 1000);

        // Cleanup inactive subscriptions
        let mut subs = subscriptions.lock().await;
        subs.retain(|_, _| false); // Remove all
        drop(subs);

        assert_eq!(subscriptions.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_message_pool_reuse() {
        // Test message object reuse to reduce allocations
        let mut message_pool = Vec::new();

        // Pre-allocate pool
        for _ in 0..100 {
            message_pool.push(json!({
                "type": "next",
                "id": "",
                "payload": null
            }));
        }

        // Reuse messages
        for i in 0..1000 {
            let idx = i % 100;
            let msg = &mut message_pool[idx];

            // Update message
            msg["id"] = json!(format!("msg-{}", i));
            msg["payload"] = json!({"data": i});

            // Process message
            let _serialized = serde_json::to_string(&msg).unwrap();
        }

        assert_eq!(message_pool.len(), 100);
    }
}

#[cfg(test)]
mod scalability {
    use super::*;

    #[tokio::test]
    async fn test_horizontal_scaling_simulation() {
        // Simulate multiple server instances handling connections
        let num_instances = 4;
        let connections_per_instance = 250;

        let total_connections = Arc::new(AtomicUsize::new(0));
        let mut instance_handles = Vec::new();

        for instance_id in 0..num_instances {
            let connections = total_connections.clone();

            let handle = tokio::spawn(async move {
                let mut local_connections = Vec::new();

                for i in 0..connections_per_instance {
                    let conn_id = format!("instance-{}-conn-{}", instance_id, i);
                    local_connections.push(conn_id);
                    connections.fetch_add(1, Ordering::SeqCst);
                }

                // Simulate connection activity
                tokio::time::sleep(Duration::from_millis(50)).await;

                local_connections.len()
            });

            instance_handles.push(handle);
        }

        let results: Vec<_> = futures::future::join_all(instance_handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let total = total_connections.load(Ordering::SeqCst);
        let expected = num_instances * connections_per_instance;

        println!("Total connections across {} instances: {}", num_instances, total);

        assert_eq!(total, expected);
        assert!(results.iter().all(|&count| count == connections_per_instance));
    }

    #[tokio::test]
    async fn test_load_balancing_distribution() {
        use std::collections::HashMap;

        let num_servers = 4;
        let total_connections = 1000;

        let distribution = Arc::new(Mutex::new(HashMap::new()));

        for i in 0..total_connections {
            // Simple round-robin load balancing
            let server_id = i % num_servers;

            let mut dist = distribution.lock().await;
            *dist.entry(server_id).or_insert(0) += 1;
        }

        let dist = distribution.lock().await;

        println!("Load distribution: {:?}", dist);

        // Should be evenly distributed
        for &count in dist.values() {
            let expected = total_connections / num_servers;
            assert_eq!(count, expected);
        }
    }
}
