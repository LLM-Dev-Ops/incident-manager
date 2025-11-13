// Circuit Breaker Performance Benchmarks
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use llm_incident_manager::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

fn circuit_breaker_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreaker::new("benchmark_overhead", config);

    c.bench_function("circuit_breaker_success_call_overhead", |b| {
        b.to_async(&rt).iter(|| async {
            cb.call(|| Box::pin(async { Ok::<i32, std::io::Error>(black_box(42)) }))
                .await
        });
    });
}

fn circuit_breaker_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("circuit_breaker_throughput");

    for concurrent_requests in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrent_requests),
            concurrent_requests,
            |b, &concurrent| {
                let config = CircuitBreakerConfig::default();
                let cb = Arc::new(CircuitBreaker::new(
                    format!("benchmark_throughput_{}", concurrent),
                    config,
                ));

                b.to_async(&rt).iter(|| {
                    let cb_clone = cb.clone();
                    async move {
                        let mut handles = vec![];
                        for i in 0..concurrent {
                            let cb = cb_clone.clone();
                            handles.push(tokio::spawn(async move {
                                cb.call(|| Box::pin(async move { Ok::<i32, std::io::Error>(i) }))
                                    .await
                            }));
                        }
                        futures::future::join_all(handles).await
                    }
                });
            },
        );
    }
    group.finish();
}

fn circuit_breaker_state_transitions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("circuit_breaker_closed_to_open_transition", |b| {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(5)
            .build()
            .unwrap();
        let cb = CircuitBreaker::new("benchmark_transition", config);

        b.to_async(&rt).iter(|| async {
            cb.reset(); // Reset to closed state

            // Trigger failures to open circuit
            for _ in 0..5 {
                let _ = cb
                    .call(|| {
                        Box::pin(async {
                            Err::<i32, std::io::Error>(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "benchmark error",
                            ))
                        })
                    })
                    .await;
            }
        });
    });
}

fn circuit_breaker_fast_fail(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("benchmark_fast_fail", config);

    // Open the circuit
    rt.block_on(async {
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
    });

    c.bench_function("circuit_breaker_fast_fail_performance", |b| {
        b.to_async(&rt).iter(|| async {
            // These should fast-fail
            cb.call(|| {
                Box::pin(async {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok::<i32, std::io::Error>(42)
                })
            })
            .await
        });
    });
}

fn circuit_breaker_fallback(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(2)
        .build()
        .unwrap();
    let cb = CircuitBreaker::new("benchmark_fallback", config);

    // Open the circuit
    rt.block_on(async {
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
    });

    c.bench_function("circuit_breaker_fallback_execution", |b| {
        b.to_async(&rt).iter(|| async {
            cb.call_with_fallback(
                || Box::pin(async { Ok::<i32, std::io::Error>(42) }),
                || Box::pin(async { black_box(100) }),
            )
            .await
        });
    });
}

fn circuit_breaker_concurrent_state_reads(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = CircuitBreakerConfig::default();
    let cb = Arc::new(CircuitBreaker::new("benchmark_concurrent_reads", config));

    c.bench_function("circuit_breaker_100_concurrent_state_reads", |b| {
        b.to_async(&rt).iter(|| {
            let cb_clone = cb.clone();
            async move {
                let mut handles = vec![];
                for _ in 0..100 {
                    let cb = cb_clone.clone();
                    handles.push(tokio::spawn(async move {
                        black_box(cb.state());
                        black_box(cb.stats());
                    }));
                }
                futures::future::join_all(handles).await
            }
        });
    });
}

fn circuit_breaker_mixed_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(10)
        .build()
        .unwrap();
    let cb = Arc::new(CircuitBreaker::new("benchmark_mixed", config));

    c.bench_function("circuit_breaker_mixed_success_failure_pattern", |b| {
        b.to_async(&rt).iter(|| {
            let cb_clone = cb.clone();
            async move {
                for i in 0..20 {
                    if i % 3 == 0 {
                        let _ = cb_clone
                            .call(|| {
                                Box::pin(async {
                                    Err::<i32, std::io::Error>(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        "error",
                                    ))
                                })
                            })
                            .await;
                    } else {
                        let _ = cb_clone
                            .call(|| Box::pin(async { Ok::<i32, std::io::Error>(i) }))
                            .await;
                    }
                }
            }
        });
    });
}

criterion_group!(
    benches,
    circuit_breaker_overhead,
    circuit_breaker_throughput,
    circuit_breaker_state_transitions,
    circuit_breaker_fast_fail,
    circuit_breaker_fallback,
    circuit_breaker_concurrent_state_reads,
    circuit_breaker_mixed_operations
);
criterion_main!(benches);
