# WebSocket Test Execution Guide

## Prerequisites

Before running the WebSocket tests, ensure:

1. Rust toolchain is installed (1.75+)
2. All dependencies are compiled
3. Test infrastructure is set up

```bash
rustc --version  # Should show 1.75+
cargo --version
```

## Quick Start

### Run All WebSocket Tests

```bash
# From project root
cd /workspaces/llm-incident-manager

# Run all WebSocket tests
cargo test websocket --no-fail-fast

# Run with detailed output
cargo test websocket -- --nocapture --test-threads=1
```

## Individual Test Suites

### 1. Unit Tests (Fast - ~2 seconds)

```bash
# Run all unit tests
cargo test --test websocket_unit_test

# Run specific module
cargo test --test websocket_unit_test message_serialization
cargo test --test websocket_unit_test message_validation
cargo test --test websocket_unit_test subscription_filters
cargo test --test websocket_unit_test connection_state
cargo test --test websocket_unit_test session_management

# Run single test
cargo test --test websocket_unit_test test_graphql_subscription_message_serialization
```

### 2. Integration Tests (Medium - ~5-10 seconds)

```bash
# Run all integration tests
cargo test --test websocket_integration_test

# Run specific module
cargo test --test websocket_integration_test connection_lifecycle
cargo test --test websocket_integration_test subscription_operations
cargo test --test websocket_integration_test event_streaming
cargo test --test websocket_integration_test error_handling
cargo test --test websocket_integration_test security
cargo test --test websocket_integration_test reliability

# Run with verbose output
cargo test --test websocket_integration_test -- --nocapture
```

### 3. Performance Tests (Slow - ~30-60 seconds)

```bash
# Run all performance tests
cargo test --test websocket_performance_test

# Run in release mode for accurate performance measurements
cargo test --release --test websocket_performance_test

# Run specific performance test
cargo test --test websocket_performance_test concurrent_connections
cargo test --test websocket_performance_test message_throughput
cargo test --test websocket_performance_test latency_measurements
cargo test --test websocket_performance_test backpressure
cargo test --test websocket_performance_test memory_usage

# Run with performance metrics
cargo test --release --test websocket_performance_test -- --nocapture
```

### 4. GraphQL Subscription Tests (Medium - ~5 seconds)

```bash
# Run all GraphQL subscription tests
cargo test --test websocket_graphql_subscription_test

# Run specific module
cargo test --test websocket_graphql_subscription_test graphql_subscription_queries
cargo test --test websocket_graphql_subscription_test subscription_message_flow
cargo test --test websocket_graphql_subscription_test subscription_filtering
cargo test --test websocket_graphql_subscription_test update_types
cargo test --test websocket_graphql_subscription_test edge_cases
```

## Test Execution Patterns

### Development Mode

Run frequently during development:

```bash
# Fast feedback loop (unit tests only)
cargo test --test websocket_unit_test

# Medium feedback (unit + integration)
cargo test websocket_unit_test websocket_integration_test
```

### Pre-Commit

Run before committing code:

```bash
# All tests except slow performance tests
cargo test --test websocket_unit_test --test websocket_integration_test --test websocket_graphql_subscription_test

# Or use test filter
cargo test websocket --no-fail-fast
```

### CI/CD Pipeline

Complete test suite:

```bash
# All tests in release mode
cargo test --release websocket --no-fail-fast -- --test-threads=4

# Generate coverage report
cargo tarpaulin --test websocket_unit_test --test websocket_integration_test --out Html
```

### Performance Validation

For performance benchmarking:

```bash
# Release mode is essential for accurate performance metrics
cargo test --release --test websocket_performance_test -- --nocapture

# Run multiple times for statistical significance
for i in {1..5}; do
  echo "Run $i:"
  cargo test --release --test websocket_performance_test -- --nocapture
done
```

## Test Output Interpretation

### Successful Test Run

```
running 24 tests
test message_serialization::test_graphql_subscription_message_serialization ... ok
test message_serialization::test_subscription_start_message ... ok
...
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Performance Test Output

```
Messages: 1000, Time: 850ms, Throughput: 1176 msg/s
Peak concurrent connections: 985
Latency - p50: 1.2ms, p95: 8.5ms, p99: 15.3ms
```

### Failed Test

```
---- connection_lifecycle::test_websocket_upgrade_endpoint_exists stdout ----
thread 'connection_lifecycle::test_websocket_upgrade_endpoint_exists' panicked at 'assertion failed'
```

## Debugging Failed Tests

### Enable Logging

```bash
# Set log level
RUST_LOG=debug cargo test --test websocket_integration_test -- --nocapture

# More verbose
RUST_LOG=trace cargo test --test websocket_integration_test test_specific_test -- --nocapture
```

### Run Single Test

```bash
# Isolate failing test
cargo test --test websocket_integration_test test_websocket_upgrade_endpoint_exists -- --nocapture --test-threads=1
```

### Enable Backtraces

```bash
# Full backtrace
RUST_BACKTRACE=full cargo test --test websocket_integration_test
```

## Common Test Scenarios

### Testing Connection Limits

```bash
# Run connection scalability test
cargo test --release --test websocket_performance_test test_1000_concurrent_connections_scalability -- --nocapture
```

Expected output:
```
Peak concurrent connections: 985
test concurrent_connections::test_1000_concurrent_connections_scalability ... ok
```

### Testing Message Throughput

```bash
# Run throughput test
cargo test --release --test websocket_performance_test test_high_message_throughput_1000_per_second -- --nocapture
```

Expected output:
```
Messages: 1000, Time: 850ms, Throughput: 1176 msg/s
test message_throughput::test_high_message_throughput_1000_per_second ... ok
```

### Testing Latency

```bash
# Run latency test
cargo test --release --test websocket_performance_test test_message_latency_tracking -- --nocapture
```

Expected output:
```
Latency - p50: 1.2ms, p95: 8.5ms, p99: 15.3ms
test latency_measurements::test_message_latency_tracking ... ok
```

### Testing Error Handling

```bash
# Run error handling tests
cargo test --test websocket_integration_test error_handling -- --nocapture
```

## Performance Benchmarks

### Baseline Metrics (Release Mode)

| Test | Expected Result |
|------|----------------|
| 100 Concurrent Connections | < 1s |
| 1000 Concurrent Connections | < 5s |
| 1000 msg/s Throughput | Pass |
| p95 Latency | < 10ms |
| Memory per Connection | < 1MB |

### Running Benchmarks

```bash
# Run all performance tests in release mode
cargo test --release --test websocket_performance_test -- --nocapture 2>&1 | tee performance_results.txt

# Extract metrics
grep -E "(Messages:|Throughput:|Latency|Peak)" performance_results.txt
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: WebSocket Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Run unit tests
        run: cargo test --test websocket_unit_test

      - name: Run integration tests
        run: cargo test --test websocket_integration_test

      - name: Run GraphQL tests
        run: cargo test --test websocket_graphql_subscription_test

      - name: Run performance tests
        run: cargo test --release --test websocket_performance_test
```

## Test Coverage

### Generate Coverage Report

```bash
# Install tarpaulin if not already installed
cargo install cargo-tarpaulin

# Generate coverage for WebSocket tests
cargo tarpaulin \
  --test websocket_unit_test \
  --test websocket_integration_test \
  --test websocket_graphql_subscription_test \
  --out Html \
  --output-dir ./coverage

# Open coverage report
open coverage/index.html
```

### Expected Coverage

- Unit Tests: > 95%
- Integration Tests: > 85%
- Overall WebSocket Module: > 90%

## Troubleshooting

### Test Hangs

If tests hang:

```bash
# Set timeout
cargo test --test websocket_integration_test -- --test-threads=1 --nocapture &
sleep 30
kill %1
```

### Port Conflicts

If you get port binding errors:

```bash
# Tests use random ports, but if issues persist:
lsof -ti:8080 | xargs kill -9
lsof -ti:9000 | xargs kill -9
```

### Memory Issues

For large-scale tests:

```bash
# Increase stack size
RUST_MIN_STACK=8388608 cargo test --release --test websocket_performance_test
```

### Temporary File Cleanup

Tests create temporary databases:

```bash
# Clean up test artifacts
rm -rf ./data/test-ws-*
```

## Best Practices

1. **Always run in release mode for performance tests**
   ```bash
   cargo test --release --test websocket_performance_test
   ```

2. **Use `--nocapture` for debugging**
   ```bash
   cargo test --test websocket_integration_test -- --nocapture
   ```

3. **Run tests serially when debugging**
   ```bash
   cargo test --test websocket_integration_test -- --test-threads=1
   ```

4. **Clean build for accurate results**
   ```bash
   cargo clean
   cargo test --release --test websocket_performance_test
   ```

## Metrics Collection

### Automated Metrics Tracking

```bash
#!/bin/bash
# run_websocket_benchmarks.sh

echo "WebSocket Test Metrics - $(date)"
echo "================================"

# Run performance tests and extract metrics
cargo test --release --test websocket_performance_test -- --nocapture 2>&1 | \
  tee /tmp/ws_perf.log

echo ""
echo "Summary:"
echo "--------"
grep -E "Peak concurrent|Throughput:|Latency" /tmp/ws_perf.log

# Save to metrics file
echo "$(date),$(grep -oP 'Throughput: \K[0-9.]+' /tmp/ws_perf.log | head -1)" >> metrics.csv
```

## Next Steps

After tests pass:

1. Review coverage report
2. Address any performance regressions
3. Update documentation if needed
4. Commit changes
5. Create PR with test results

## Resources

- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [async-graphql Testing](https://async-graphql.github.io/async-graphql/testing.html)
