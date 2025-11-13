// Comprehensive Circuit Breaker Test Suite
// This file combines unit, integration, and scenario tests for the Circuit Breaker

use llm_incident_manager::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// UNIT TESTS - State Transitions
// ============================================================================

#[tokio::test]
async fn test_circuit_breaker_starts_closed() {
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreaker::new("test_starts_closed", config);

    assert_eq!(cb.state(), CircuitBreakerState::Closed);
}

#[tokio::test]
async fn test_successful_call_stays_closed() {
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreaker::new("test_success", config);

    let result = cb
        .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
        .await
        .unwrap();

    assert_eq!(result, 42);
    assert_eq!(cb.state(), CircuitBreakerState::Closed);
}

#[tokio::test]
async fn test_circuit_opens_after_threshold_failures() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(3)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_opens", config);

    // Cause 3 failures
    for _ in 0..3 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    assert_eq!(cb.state(), CircuitBreakerState::Open);
}

#[tokio::test]
async fn test_open_circuit_rejects_calls() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_rejects", config);

    // Open the circuit
    for _ in 0..2 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    assert_eq!(cb.state(), CircuitBreakerState::Open);

    // Try to call - should be rejected
    let result = cb
        .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_circuit_transitions_to_half_open_after_timeout() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .timeout_duration(Duration::from_millis(100))
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_half_open", config);

    // Open the circuit
    for _ in 0..2 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    assert_eq!(cb.state(), CircuitBreakerState::Open);

    // Wait for timeout
    sleep(Duration::from_millis(150)).await;

    // Next call should transition to half-open
    let _ = cb
        .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
        .await;

    // Should no longer be fully open
    let current_state = cb.state();
    assert!(
        current_state == CircuitBreakerState::HalfOpen
            || current_state == CircuitBreakerState::Closed
    );
}

#[tokio::test]
async fn test_half_open_closes_after_success_threshold() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .success_threshold(3)
        .timeout_duration(Duration::from_millis(100))
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_half_open_close", config);

    // Open the circuit
    for _ in 0..2 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    // Wait for timeout
    sleep(Duration::from_millis(150)).await;

    // Make successful calls to close circuit
    for _ in 0..3 {
        let _ = cb
            .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
            .await;
    }

    assert_eq!(cb.state(), CircuitBreakerState::Closed);
}

#[tokio::test]
async fn test_half_open_reopens_on_failure() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .timeout_duration(Duration::from_millis(100))
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_half_open_reopen", config);

    // Open the circuit
    for _ in 0..2 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    // Wait for timeout
    sleep(Duration::from_millis(150)).await;

    // Try a call that fails - should reopen circuit
    let _ = cb
        .call(|| {
            Box::pin(async {
                Err::<i32, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "error",
                ))
            })
        })
        .await;

    assert_eq!(cb.state(), CircuitBreakerState::Open);
}

// ============================================================================
// FALLBACK TESTS
// ============================================================================

#[tokio::test]
async fn test_fallback_on_open_circuit() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_fallback_open", config);

    // Open the circuit
    for _ in 0..2 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    // Call with fallback
    let result = cb
        .call_with_fallback(
            || Box::pin(async { Ok::<i32, std::io::Error>(42) }),
            || Box::pin(async { 100 }),
        )
        .await
        .unwrap();

    assert_eq!(result, 100, "Should use fallback value");
}

#[tokio::test]
async fn test_fallback_not_used_on_success() {
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreaker::new("test_fallback_success", config);

    let result = cb
        .call_with_fallback(
            || Box::pin(async { Ok::<i32, std::io::Error>(42) }),
            || Box::pin(async { 100 }),
        )
        .await
        .unwrap();

    assert_eq!(result, 42, "Should use primary value");
}

// ============================================================================
// MANUAL OPERATIONS TESTS
// ============================================================================

#[tokio::test]
async fn test_force_open() {
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreaker::new("test_force_open", config);

    assert_eq!(cb.state(), CircuitBreakerState::Closed);

    cb.force_open();

    assert_eq!(cb.state(), CircuitBreakerState::Open);

    // Verify calls are rejected
    let result = cb
        .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_reset() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_reset", config);

    // Open the circuit
    for _ in 0..2 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    assert_eq!(cb.state(), CircuitBreakerState::Open);

    cb.reset();

    assert_eq!(cb.state(), CircuitBreakerState::Closed);
}

// ============================================================================
// CONCURRENCY TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_calls_closed_state() {
    let config = CircuitBreakerConfig::default();
    let cb = Arc::new(CircuitBreaker::new("test_concurrent_closed", config));

    let mut handles = vec![];

    // Make 100 concurrent calls
    for i in 0..100 {
        let cb_clone = cb.clone();
        handles.push(tokio::spawn(async move {
            cb_clone
                .call(|| Box::pin(async move { Ok::<i32, std::io::Error>(i) }))
                .await
        }));
    }

    let results = futures::future::join_all(handles).await;

    // All should succeed
    for result in results {
        assert!(result.unwrap().is_ok());
    }

    assert_eq!(cb.state(), CircuitBreakerState::Closed);
}

// ============================================================================
// CONFIGURATION TESTS
// ============================================================================

#[tokio::test]
async fn test_config_builder() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(10)
        .success_threshold(5)
        .timeout_duration(Duration::from_secs(60))
        .half_open_max_requests(10)
        .build()
        .unwrap();

    let cb = CircuitBreaker::new("test_builder", config);

    assert_eq!(cb.config().failure_threshold, 10);
    assert_eq!(cb.config().success_threshold, 5);
    assert_eq!(cb.config().timeout_duration, Duration::from_secs(60));
    assert_eq!(cb.config().half_open_max_requests, 10);
}

// ============================================================================
// STATISTICS TESTS
// ============================================================================

#[tokio::test]
async fn test_statistics() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(3)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_stats", config);

    // Initial stats
    let stats = cb.stats();
    assert_eq!(stats.name, "test_stats");
    assert_eq!(stats.state, CircuitBreakerState::Closed);
    assert_eq!(stats.consecutive_failures, 0);
    assert_eq!(stats.transition_count, 0);

    // After failures
    for _ in 0..2 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    let stats = cb.stats();
    assert_eq!(stats.consecutive_failures, 2);

    // After circuit opens
    let _ = cb
        .call(|| {
            Box::pin(async {
                Err::<i32, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "error",
                ))
            })
        })
        .await;

    let stats = cb.stats();
    assert_eq!(stats.state, CircuitBreakerState::Open);
    assert_eq!(stats.transition_count, 1);
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[tokio::test]
async fn test_exactly_at_threshold() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(5)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_exact_threshold", config);

    // 4 failures - should stay closed
    for _ in 0..4 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }
    assert_eq!(cb.state(), CircuitBreakerState::Closed);

    // 5th failure - should open
    let _ = cb
        .call(|| {
            Box::pin(async {
                Err::<i32, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "error",
                ))
            })
        })
        .await;
    assert_eq!(cb.state(), CircuitBreakerState::Open);
}

#[tokio::test]
async fn test_failure_count_resets_on_success() {
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(5)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("test_reset_failures", config);

    // 3 failures
    for _ in 0..3 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    assert_eq!(cb.state(), CircuitBreakerState::Closed);

    // 1 success should reset failure count
    let _ = cb
        .call(|| Box::pin(async { Ok::<i32, std::io::Error>(42) }))
        .await;

    // Now we need 5 more failures to open
    for _ in 0..4 {
        let _ = cb
            .call(|| {
                Box::pin(async {
                    Err::<i32, std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "error",
                    ))
                })
            })
            .await;
    }

    // Should still be closed after 4 failures
    assert_eq!(cb.state(), CircuitBreakerState::Closed);
}
