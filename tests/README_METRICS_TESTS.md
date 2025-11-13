# Prometheus Metrics Test Suite Documentation

## Overview

This comprehensive test suite provides complete coverage for the Prometheus metrics implementation in the LLM Incident Manager system. The tests are designed to validate functionality, performance, and compliance with Prometheus best practices.

## Test Structure

### 1. Unit Tests (`tests/prometheus_metrics_test.rs`)

Located in: `/workspaces/llm-incident-manager/tests/prometheus_metrics_test.rs`

#### Registry Tests
- **`test_metrics_registry_initialization`**: Validates registry can be created and starts empty
- **`test_metrics_registry_thread_safety`**: Ensures registry is safe for concurrent access
- **`test_duplicate_metric_registration`**: Verifies proper handling of duplicate registrations

#### Counter Tests
- **`test_counter_creation`**: Validates counter initialization
- **`test_counter_increment`**: Tests `inc()` and `inc_by()` operations
- **`test_counter_with_labels`**: Verifies label-based counter variants work correctly
- **`test_counter_concurrent_increments`**: Ensures counters are thread-safe (10,000 concurrent increments)

#### Gauge Tests
- **`test_gauge_creation`**: Validates gauge initialization
- **`test_gauge_set`**: Tests `set()` operation
- **`test_gauge_inc_dec`**: Verifies `inc()`, `inc_by()`, `dec()`, `dec_by()` operations
- **`test_gauge_with_labels`**: Tests label-based gauge variants

#### Histogram Tests
- **`test_histogram_creation`**: Validates histogram initialization with buckets
- **`test_histogram_observe`**: Tests `observe()` operation and sample tracking
- **`test_histogram_buckets`**: Verifies correct bucket assignment
- **`test_histogram_with_labels`**: Tests label-based histogram variants

#### Label Tests
- **`test_label_name_validation`**: Ensures label names follow Prometheus conventions
- **`test_label_cardinality`**: Verifies reasonable limits on unique label values
- **`test_reserved_label_names`**: Tests rejection of reserved names (e.g., `__name__`)

### 2. Integration Tests (`tests/prometheus_metrics_test.rs`)

#### Endpoint Tests
- **`test_metrics_endpoint_returns_200`**: Validates `/metrics` endpoint availability
- **`test_metrics_endpoint_prometheus_format`**: Verifies correct Content-Type header
  - Expected: `text/plain; version=0.0.4; charset=utf-8`

#### Middleware Tests
- **`test_http_middleware_tracks_requests`**: Ensures HTTP requests are automatically tracked
- **`test_metrics_accumulation`**: Validates metrics accumulate over multiple requests
- **`test_metrics_with_errors`**: Ensures error cases don't break metrics collection

### 3. Performance Tests (`tests/prometheus_metrics_test.rs`)

#### Overhead Tests
- **`test_counter_increment_performance`**: Validates < 1ms per operation (target: < 1μs average)
- **`test_histogram_observe_performance`**: Ensures histogram observations are fast

#### Resource Tests
- **`test_no_memory_leak`**: Validates no unbounded memory growth over 1M operations
- **`test_concurrent_access`**: Tests 100,000 concurrent operations complete in < 1s

### 4. Validation Tests (`tests/prometheus_metrics_test.rs`)

#### Naming Convention Tests
- **`test_metric_name_conventions`**: Validates names follow `[a-zA-Z_:][a-zA-Z0-9_:]*`
- **`test_counter_naming_convention`**: Ensures counters end with `_total` suffix

#### Format Tests
- **`test_prometheus_exposition_format`**: Validates output format
- **`test_help_and_type_comments`**: Ensures proper `# HELP` and `# TYPE` comments
- **`test_label_cardinality_limits`**: Validates reasonable label value limits

### 5. Benchmark Suite (`benches/metrics_benchmark.rs`)

Located in: `/workspaces/llm-incident-manager/benches/metrics_benchmark.rs`

Uses Criterion for detailed performance analysis:

- **`bench_counter_increment`**: Measures counter `inc()` and `inc_by()` performance
- **`bench_gauge_operations`**: Measures gauge `set()`, `inc()`, `dec()` performance
- **`bench_histogram_observe`**: Measures histogram observation across different buckets
- **`bench_counter_with_labels`**: Measures label lookup and caching performance
- **`bench_metrics_export`**: Measures registry export performance with 100 metrics
- **`bench_concurrent_counter`**: Measures throughput with 1, 2, 4, 8, 16 threads
- **`bench_label_cardinality`**: Measures impact of 10, 100, 1000 unique label values
- **`bench_mixed_operations`**: Simulates realistic workload mixing all metric types

## Running Tests

### Run All Tests
```bash
cargo test --test prometheus_metrics_test
```

### Run Specific Test Module
```bash
# Unit tests only
cargo test --test prometheus_metrics_test unit_tests

# Counter tests only
cargo test --test prometheus_metrics_test counter_tests

# Integration tests only
cargo test --test prometheus_metrics_test integration_tests

# Performance tests only
cargo test --test prometheus_metrics_test performance_tests

# Validation tests only
cargo test --test prometheus_metrics_test validation_tests
```

### Run Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench counter_increment

# Run with additional output
cargo bench -- --verbose
```

### Generate Benchmark Reports
Benchmark results are saved in:
- `target/criterion/`: Detailed HTML reports
- `target/criterion/*/report/index.html`: Interactive charts

## Test Coverage Goals

### Functional Coverage
- ✅ Registry initialization and management
- ✅ Counter operations (inc, inc_by)
- ✅ Gauge operations (set, inc, dec, inc_by, dec_by)
- ✅ Histogram operations (observe, buckets)
- ✅ Label handling and validation
- ✅ Metric export format

### Performance Coverage
- ✅ < 1ms overhead per operation
- ✅ Memory leak detection
- ✅ Concurrent access safety
- ✅ Scalability with many metrics

### Validation Coverage
- ✅ Prometheus naming conventions
- ✅ Exposition format compliance
- ✅ Label cardinality limits
- ✅ Reserved name rejection

## Expected Performance Targets

Based on Prometheus best practices:

### Latency Targets
- Counter increment: < 100ns
- Gauge operations: < 100ns
- Histogram observe: < 500ns
- Label lookup (cached): < 50ns
- Metrics export (100 metrics): < 10ms

### Throughput Targets
- Counter increments: > 10M ops/sec (single thread)
- Concurrent increments: > 50M ops/sec (16 threads)

### Memory Targets
- Base registry overhead: < 1MB
- Per-metric overhead: < 1KB
- Per-label-value overhead: < 100 bytes

## Prometheus Format Requirements

### Metric Naming
- Format: `[a-zA-Z_:][a-zA-Z0-9_:]*`
- Counters MUST end with `_total`
- Histograms MUST end with `_bucket`, `_sum`, `_count`
- Reserved prefixes: `__` (double underscore)

### Label Naming
- Format: `[a-zA-Z_][a-zA-Z0-9_]*`
- Reserved labels: `__name__`, `job`, `instance`
- Case-sensitive

### Exposition Format
```
# HELP <metric_name> <help_text>
# TYPE <metric_name> <counter|gauge|histogram|summary>
<metric_name>{label1="value1",label2="value2"} <value> <timestamp?>
```

## Integration with CI/CD

### Pre-commit Checks
```bash
# Fast test suite (unit tests only)
cargo test --test prometheus_metrics_test unit_tests --quiet
```

### Pull Request Checks
```bash
# Full test suite
cargo test --test prometheus_metrics_test

# Quick benchmarks (10% of full suite)
cargo bench -- --quick
```

### Release Checks
```bash
# Full test suite with output
cargo test --test prometheus_metrics_test -- --nocapture

# Full benchmark suite
cargo bench

# Generate coverage report
cargo tarpaulin --test prometheus_metrics_test
```

## Troubleshooting

### Tests Not Running
If tests show "PENDING IMPLEMENTATION", the core metrics module hasn't been implemented yet. Tests are designed to compile but will print status messages until the implementation is complete.

### Benchmark Not Found
Ensure Criterion is in dev-dependencies:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }
```

### Performance Tests Failing
Performance tests may fail on constrained systems. Adjust thresholds in test code if running on:
- Containerized environments
- CI/CD systems with shared resources
- Systems with < 4 CPU cores

## Future Enhancements

### Planned Additions
1. **Exemplar Support**: Test exemplar tracking in histograms
2. **Remote Write Tests**: Validate remote write protocol compatibility
3. **Service Discovery Tests**: Test metric discovery and scraping
4. **Alert Rule Tests**: Validate metrics work with PromQL queries
5. **Grafana Integration Tests**: Verify dashboard compatibility

### Additional Benchmarks
1. **Memory profiling**: Track allocation patterns
2. **CPU profiling**: Identify hot paths
3. **Cache efficiency**: Measure label lookup cache hit rates
4. **Export optimization**: Test streaming export for large metric sets

## Contributing

When adding new tests:

1. **Follow naming conventions**: `test_<component>_<behavior>`
2. **Add documentation**: Explain what is being tested and why
3. **Include TODOs**: Mark implementation-dependent code
4. **Update this README**: Document new test modules
5. **Add benchmarks**: For performance-critical paths

## References

- [Prometheus Exposition Formats](https://prometheus.io/docs/instrumenting/exposition_formats/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/naming/)
- [Prometheus Client Guidelines](https://prometheus.io/docs/instrumenting/writing_clientlibs/)
- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)

## Test Status

| Category | Tests | Status |
|----------|-------|--------|
| Registry | 3 | ⏳ Pending Implementation |
| Counter | 4 | ⏳ Pending Implementation |
| Gauge | 4 | ⏳ Pending Implementation |
| Histogram | 4 | ⏳ Pending Implementation |
| Labels | 3 | ⏳ Pending Implementation |
| Integration | 5 | ⏳ Pending Implementation |
| Performance | 4 | ⏳ Pending Implementation |
| Validation | 5 | ⏳ Pending Implementation |
| Benchmarks | 8 | ⏳ Pending Implementation |
| **Total** | **40** | **Ready for Implementation** |

---

**Note**: All tests are structured and ready to be activated once the Prometheus metrics implementation is complete. Each test includes detailed TODO comments explaining exactly what needs to be tested and how to implement it.
