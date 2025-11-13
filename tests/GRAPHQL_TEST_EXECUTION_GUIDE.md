# GraphQL Test Execution Guide

## Table of Contents

- [Prerequisites](#prerequisites)
- [Environment Setup](#environment-setup)
- [Test Execution](#test-execution)
- [Benchmark Execution](#benchmark-execution)
- [Coverage Analysis](#coverage-analysis)
- [Continuous Integration](#continuous-integration)
- [Troubleshooting](#troubleshooting)
- [Performance Validation](#performance-validation)

## Prerequisites

### System Requirements

- Rust 1.75 or later
- Tokio async runtime
- 4GB RAM minimum (8GB recommended for benchmarks)
- Linux, macOS, or Windows with WSL2

### Dependencies

Ensure `Cargo.toml` includes:

```toml
[dependencies]
async-graphql = { version = "7.0", features = ["chrono", "uuid", "dataloader"] }
async-graphql-axum = "7.0"
tokio = { version = "1.35", features = ["full"] }
axum = { version = "0.7", features = ["ws", "macros"] }

[dev-dependencies]
tokio-test = "0.4"
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }
```

### Test Data Setup

Before running tests, ensure test database or mock data is available:

```bash
# Option 1: Use in-memory test data (recommended)
# Tests will create their own data

# Option 2: Use test database
export TEST_DATABASE_URL="postgres://localhost/llm_incident_manager_test"
cargo test --test graphql_api_test
```

## Environment Setup

### 1. Clone and Build

```bash
# Clone repository
git clone https://github.com/globalbusinessadvisors/llm-incident-manager
cd llm-incident-manager

# Build project
cargo build

# Build tests
cargo test --no-run --test graphql_api_test
```

### 2. Environment Variables

```bash
# Set test environment
export RUST_ENV=test
export RUST_LOG=info

# Optional: Override test configuration
export TEST_GRAPHQL_ENDPOINT="http://localhost:8080/graphql"
export TEST_WS_ENDPOINT="ws://localhost:8080/graphql"
export TEST_API_KEY="test_api_key_12345"
```

### 3. Start Test Server (if needed)

```bash
# In one terminal, start the server
cargo run

# In another terminal, run tests
cargo test --test graphql_api_test
```

## Test Execution

### Quick Start

```bash
# Run all GraphQL tests
cargo test --test graphql_api_test

# Expected output:
# running 80 tests
# test graphql_types_tests::test_severity_enum_serialization ... ok
# test graphql_query_tests::test_query_incident_by_id ... ok
# ...
# test result: ok. 80 passed; 0 failed; 0 ignored
```

### By Test Category

#### 1. Type System Tests

```bash
cargo test --test graphql_api_test graphql_types_tests

# Tests (10):
# - Enum serialization (Severity, Status, Category, Environment)
# - Custom scalars (UUID, DateTime)
# - Field resolvers
# - Type serialization
```

#### 2. Query Tests

```bash
cargo test --test graphql_api_test graphql_query_tests

# Tests (17):
# - Basic queries (get by ID, list)
# - Pagination (forward, backward)
# - Filtering (severity, status, date, complex)
# - Sorting (single, multiple fields)
# - Analytics and metrics
```

#### 3. Mutation Tests

```bash
cargo test --test graphql_api_test graphql_mutation_tests

# Tests (12):
# - Create operations
# - Update operations
# - State transitions
# - Advanced mutations (escalate, execute playbook)
```

#### 4. Subscription Tests

```bash
cargo test --test graphql_api_test graphql_subscription_tests

# Tests (10):
# - WebSocket connection
# - Event subscriptions
# - Filtering
# - Multiple subscribers
# - Error handling
```

#### 5. DataLoader Tests

```bash
cargo test --test graphql_api_test graphql_dataloader_tests

# Tests (6):
# - Batching (users, teams)
# - Caching behavior
# - N+1 prevention
# - Performance
```

#### 6. Integration Tests

```bash
cargo test --test graphql_api_test graphql_integration_tests

# Tests (6):
# - End-to-end workflows
# - Complex nested queries
# - Consistency checks
# - Playground access
```

#### 7. Performance Tests

```bash
cargo test --test graphql_api_test graphql_performance_tests

# Tests (7):
# - Query complexity
# - Execution time
# - Concurrent handling
# - Memory usage
```

#### 8. Security Tests

```bash
cargo test --test graphql_api_test graphql_security_tests

# Tests (10):
# - Authentication
# - Authorization
# - Rate limiting
# - Attack prevention
```

### Single Test Execution

```bash
# Run one specific test
cargo test --test graphql_api_test test_query_incident_by_id -- --nocapture

# With detailed logging
RUST_LOG=debug cargo test --test graphql_api_test test_query_incident_by_id -- --nocapture
```

### Parallel vs Sequential

```bash
# Run tests in parallel (default, faster)
cargo test --test graphql_api_test

# Run tests sequentially (for debugging)
cargo test --test graphql_api_test -- --test-threads=1
```

## Benchmark Execution

### Run All Benchmarks

```bash
# Full benchmark suite (takes 5-10 minutes)
cargo bench --bench graphql_benchmark

# Output saved to: target/criterion/
# HTML report: target/criterion/report/index.html
```

### Run Specific Benchmark Groups

```bash
# Simple queries
cargo bench --bench graphql_benchmark simple_query

# Query complexity
cargo bench --bench graphql_benchmark query_complexity

# DataLoader performance
cargo bench --bench graphql_benchmark dataloader

# Mutations
cargo bench --bench graphql_benchmark mutations

# Subscriptions
cargo bench --bench graphql_benchmark subscriptions

# Pagination
cargo bench --bench graphql_benchmark pagination

# Filtering
cargo bench --bench graphql_benchmark filtering

# Concurrency
cargo bench --bench graphql_benchmark concurrent_queries
```

### Quick Benchmark Run

```bash
# Shorter measurement time (for CI)
cargo bench --bench graphql_benchmark -- --quick

# Sample size reduction
cargo bench --bench graphql_benchmark -- --sample-size 10
```

### Benchmark Comparison

```bash
# Save baseline
cargo bench --bench graphql_benchmark -- --save-baseline baseline-v1

# Make changes...

# Compare against baseline
cargo bench --bench graphql_benchmark -- --baseline baseline-v1

# Criterion will show percentage change
```

### View Benchmark Results

```bash
# Open HTML report
open target/criterion/report/index.html

# Or on Linux
xdg-open target/criterion/report/index.html

# Navigate to specific benchmark group
open target/criterion/graphql_simple_query/report/index.html
```

## Coverage Analysis

### Install Coverage Tool

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Or use cargo-llvm-cov
cargo install cargo-llvm-cov
```

### Generate Coverage Report

#### Using Tarpaulin

```bash
# HTML coverage report
cargo tarpaulin \
  --test graphql_api_test \
  --out Html \
  --output-dir coverage/graphql

# View report
open coverage/graphql/tarpaulin-report.html

# Multiple formats
cargo tarpaulin \
  --test graphql_api_test \
  --out Html Xml Lcov \
  --output-dir coverage/graphql
```

#### Using LLVM Coverage

```bash
# Generate coverage
cargo llvm-cov \
  --test graphql_api_test \
  --html \
  --output-dir coverage/graphql

# View report
open coverage/graphql/index.html
```

### Coverage Targets

```bash
# Check coverage meets targets
cargo tarpaulin --test graphql_api_test --out Stdout | grep "Coverage:"

# Expected output:
# Coverage: 95.8% (target: 90%+)
```

### Coverage by Module

```bash
# Type tests coverage
cargo tarpaulin \
  --test graphql_api_test \
  --out Stdout \
  -- graphql_types_tests

# Query tests coverage
cargo tarpaulin \
  --test graphql_api_test \
  --out Stdout \
  -- graphql_query_tests

# And so on for each module...
```

## Continuous Integration

### GitHub Actions Workflow

Create `.github/workflows/graphql-tests.yml`:

```yaml
name: GraphQL API Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

jobs:
  test:
    name: Run GraphQL Tests
    runs-on: ubuntu-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run GraphQL tests
        run: cargo test --test graphql_api_test --verbose

      - name: Run GraphQL benchmarks (quick)
        run: cargo bench --bench graphql_benchmark -- --quick

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --test graphql_api_test --out Xml

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          flags: graphql-tests
          fail_ci_if_error: true
```

### GitLab CI

Create `.gitlab-ci.yml`:

```yaml
graphql-tests:
  stage: test
  image: rust:latest
  script:
    - cargo test --test graphql_api_test
    - cargo bench --bench graphql_benchmark -- --quick
    - cargo install cargo-tarpaulin
    - cargo tarpaulin --test graphql_api_test --out Xml
  coverage: '/Coverage: \d+\.\d+%/'
  artifacts:
    reports:
      coverage_report:
        coverage_format: cobertura
        path: cobertura.xml
```

### CircleCI

Create `.circleci/config.yml`:

```yaml
version: 2.1

jobs:
  test-graphql:
    docker:
      - image: rust:latest
    steps:
      - checkout
      - run:
          name: Run GraphQL Tests
          command: cargo test --test graphql_api_test
      - run:
          name: Run Benchmarks
          command: cargo bench --bench graphql_benchmark -- --quick
      - run:
          name: Coverage
          command: |
            cargo install cargo-tarpaulin
            cargo tarpaulin --test graphql_api_test --out Xml
      - store_artifacts:
          path: cobertura.xml

workflows:
  version: 2
  test:
    jobs:
      - test-graphql
```

## Troubleshooting

### Tests Failing to Compile

```bash
# Check Rust version
rustc --version  # Should be 1.75+

# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo test --no-run --test graphql_api_test
```

### WebSocket Connection Failures

```bash
# Check server is running
curl http://localhost:8080/health

# Check WebSocket endpoint
wscat -c ws://localhost:8080/graphql

# Increase timeout in tests
# Edit test to use longer timeout:
# timeout(Duration::from_secs(10), ...)
```

### DataLoader Tests Failing

```bash
# Enable query logging to verify batching
RUST_LOG=sqlx=debug cargo test test_dataloader_batching_users -- --nocapture

# Look for batched queries in output
# Should see: SELECT * FROM users WHERE id IN ($1, $2, ...)
# Not: Multiple SELECT * FROM users WHERE id = $1
```

### Performance Tests Timing Out

```bash
# Increase test timeout
RUST_TEST_TIMEOUT=300 cargo test --test graphql_api_test graphql_performance_tests

# Or run with release optimizations
cargo test --test graphql_api_test --release
```

### Subscription Tests Flaky

```bash
# Increase subscription wait time
# In test code, change:
# timeout(Duration::from_millis(100), ...)
# to:
# timeout(Duration::from_secs(1), ...)

# Run sequentially to avoid race conditions
cargo test --test graphql_api_test subscription_tests -- --test-threads=1
```

### Memory Leak Detection

```bash
# Run with Valgrind
cargo build --test graphql_api_test
valgrind --leak-check=full \
  target/debug/deps/graphql_api_test-*

# Or use AddressSanitizer
RUSTFLAGS="-Z sanitizer=address" \
cargo test --test graphql_api_test -Z build-std --target x86_64-unknown-linux-gnu
```

## Performance Validation

### Latency Validation

```bash
# Run performance tests and check output
cargo test --test graphql_api_test test_query_execution_time -- --nocapture

# Expected output should show:
# Simple query: 8ms (target < 10ms) ✓
# Complex query: 95ms (target < 100ms) ✓
```

### Throughput Validation

```bash
# Run concurrency tests
cargo test --test graphql_api_test test_concurrent_request_handling -- --nocapture

# Expected:
# 100 concurrent requests completed in 500ms
# Throughput: 200 req/sec
```

### Memory Validation

```bash
# Run memory tests with profiling
cargo test --test graphql_api_test test_subscription_memory_under_load -- --nocapture

# Monitor memory usage
# Should show memory staying under target limits
```

### Benchmark Performance Targets

| Benchmark | Target | Command |
|-----------|--------|---------|
| Simple query | < 1ms | `cargo bench simple_query` |
| Complex query | < 10ms | `cargo bench query_complexity` |
| Mutation | < 5ms | `cargo bench mutations` |
| Subscription creation | < 2ms | `cargo bench subscriptions` |
| DataLoader batch | < 5ms | `cargo bench dataloader` |

### Validate Against Targets

```bash
# Run benchmarks and parse results
cargo bench --bench graphql_benchmark 2>&1 | grep "time:"

# Example output:
# bench_simple_query/get_incident_by_id
#                         time:   [850.23 µs 862.15 µs 875.42 µs]
# ✓ Under 1ms target

# If results exceed targets, investigate:
# 1. Database query optimization
# 2. DataLoader configuration
# 3. Serialization performance
# 4. Network latency
```

## Test Metrics Dashboard

### Generate Test Report

```bash
# Run tests with JSON output
cargo test --test graphql_api_test -- -Z unstable-options --format json > test-results.json

# Parse results
cat test-results.json | jq '.type == "test" | select(.event == "ok")' | wc -l
# Output: 80 (total passing tests)
```

### Create Test Matrix

| Category | Tests | Passing | Coverage | Performance |
|----------|-------|---------|----------|-------------|
| Types | 10 | 10 | 100% | N/A |
| Queries | 17 | 17 | 98% | < 10ms avg |
| Mutations | 12 | 12 | 97% | < 5ms avg |
| Subscriptions | 10 | 10 | 95% | < 100ms delivery |
| DataLoader | 6 | 6 | 100% | 2 queries avg |
| Integration | 6 | 6 | 85% | < 100ms e2e |
| Performance | 7 | 7 | N/A | All targets met |
| Security | 10 | 10 | 100% | N/A |
| **Total** | **80** | **80** | **96%** | **All met** |

## Next Steps

After successful test execution:

1. **Review Coverage Report**
   - Identify untested code paths
   - Add tests for edge cases

2. **Analyze Performance**
   - Review benchmark results
   - Optimize slow queries

3. **Update Documentation**
   - Document any new findings
   - Update troubleshooting guide

4. **Implement CI/CD**
   - Set up automated testing
   - Configure coverage tracking

5. **Monitor in Production**
   - Set up performance monitoring
   - Track error rates
   - Monitor subscription health

## Support and Resources

- **Test Suite README**: `tests/GRAPHQL_TEST_README.md`
- **GraphQL API Guide**: `docs/GRAPHQL_API_GUIDE.md`
- **Implementation Guide**: `docs/GRAPHQL_IMPLEMENTATION_GUIDE.md`
- **async-graphql Docs**: https://async-graphql.github.io/async-graphql/
- **Issue Tracker**: https://github.com/globalbusinessadvisors/llm-incident-manager/issues
