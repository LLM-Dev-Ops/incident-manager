# Test Execution Guide for Prometheus Metrics

## Prerequisites

### 1. Install Rust Toolchain
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Verify Installation
```bash
rustc --version
cargo --version
```

### 3. Install Dependencies
```bash
cd /workspaces/llm-incident-manager
cargo build
```

## Running Tests

### Quick Test Run (All Tests)
```bash
# Run all metrics tests
cargo test --test prometheus_metrics_test

# Run with output
cargo test --test prometheus_metrics_test -- --nocapture

# Run with detailed output
cargo test --test prometheus_metrics_test -- --nocapture --test-threads=1
```

### Run Specific Test Modules

```bash
# Unit tests only
cargo test --test prometheus_metrics_test unit_tests

# Counter tests
cargo test --test prometheus_metrics_test counter_tests

# Gauge tests
cargo test --test prometheus_metrics_test gauge_tests

# Histogram tests
cargo test --test prometheus_metrics_test histogram_tests

# Label tests
cargo test --test prometheus_metrics_test label_tests

# Integration tests
cargo test --test prometheus_metrics_test integration_tests

# Performance tests
cargo test --test prometheus_metrics_test performance_tests

# Validation tests
cargo test --test prometheus_metrics_test validation_tests
```

### Run Specific Test Cases

```bash
# Run single test
cargo test --test prometheus_metrics_test test_counter_increment

# Run tests matching pattern
cargo test --test prometheus_metrics_test counter_
```

## Running Benchmarks

### Install Criterion
Criterion is already added to dev-dependencies, but you can verify:
```bash
cargo tree --dev | grep criterion
```

### Run All Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run benchmarks matching pattern
cargo bench counter

# Run with verbose output
cargo bench -- --verbose
```

### Run Specific Benchmarks
```bash
# Counter benchmarks only
cargo bench bench_counter_increment

# Histogram benchmarks only
cargo bench bench_histogram_observe

# Concurrent access benchmarks
cargo bench bench_concurrent_counter

# Label cardinality benchmarks
cargo bench bench_label_cardinality
```

### View Benchmark Results
```bash
# Open HTML report in browser
open target/criterion/report/index.html

# Or for Linux
xdg-open target/criterion/report/index.html

# Results are also in:
# target/criterion/<benchmark_name>/report/index.html
```

## Test Execution Status

### Before Implementation
When running tests before the Prometheus implementation is complete:

```bash
$ cargo test --test prometheus_metrics_test
```

**Expected Output:**
```
running 32 tests
test counter_tests::test_counter_concurrent_increments ... ok
test counter_tests::test_counter_creation ... ok
test counter_tests::test_counter_increment ... ok
test counter_tests::test_counter_with_labels ... ok
test gauge_tests::test_gauge_creation ... ok
test gauge_tests::test_gauge_inc_dec ... ok
test gauge_tests::test_gauge_set ... ok
test gauge_tests::test_gauge_with_labels ... ok
test histogram_tests::test_histogram_buckets ... ok
test histogram_tests::test_histogram_creation ... ok
test histogram_tests::test_histogram_observe ... ok
test histogram_tests::test_histogram_with_labels ... ok
test integration_tests::test_http_middleware_tracks_requests ... ok
test integration_tests::test_metrics_accumulation ... ok
test integration_tests::test_metrics_endpoint_prometheus_format ... ok
test integration_tests::test_metrics_endpoint_returns_200 ... ok
test integration_tests::test_metrics_with_errors ... ok
test label_tests::test_label_cardinality ... ok
test label_tests::test_label_name_validation ... ok
test label_tests::test_reserved_label_names ... ok
test performance_tests::test_concurrent_access ... ok
test performance_tests::test_counter_increment_performance ... ok
test performance_tests::test_histogram_observe_performance ... ok
test performance_tests::test_no_memory_leak ... ok
test unit_tests::test_duplicate_metric_registration ... ok
test unit_tests::test_metrics_registry_initialization ... ok
test unit_tests::test_metrics_registry_thread_safety ... ok
test validation_tests::test_counter_naming_convention ... ok
test validation_tests::test_help_and_type_comments ... ok
test validation_tests::test_label_cardinality_limits ... ok
test validation_tests::test_metric_name_conventions ... ok
test validation_tests::test_prometheus_exposition_format ... ok

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Each test will print: `Test: <name> - PENDING IMPLEMENTATION`

### After Implementation
Once the Prometheus metrics module is implemented, tests will execute fully:

1. Uncomment the test code in `/workspaces/llm-incident-manager/tests/prometheus_metrics_test.rs`
2. Uncomment imports and implementation-specific code
3. Run tests again

**Expected Output:**
```
running 32 tests
test counter_tests::test_counter_concurrent_increments ... ok (0.05s)
test counter_tests::test_counter_creation ... ok (0.00s)
test counter_tests::test_counter_increment ... ok (0.00s)
test counter_tests::test_counter_with_labels ... ok (0.00s)
...

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Test Coverage Analysis

### Install Tarpaulin (Code Coverage Tool)
```bash
cargo install cargo-tarpaulin
```

### Generate Coverage Report
```bash
# Generate HTML coverage report
cargo tarpaulin --test prometheus_metrics_test --out Html

# Generate Codecov format
cargo tarpaulin --test prometheus_metrics_test --out Xml

# Open coverage report
open tarpaulin-report.html
```

## Continuous Integration

### GitHub Actions Example

Create `.github/workflows/metrics-tests.yml`:

```yaml
name: Metrics Tests

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

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

      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Run metrics tests
        run: cargo test --test prometheus_metrics_test

      - name: Run benchmarks (quick)
        run: cargo bench -- --quick

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --test prometheus_metrics_test --out Xml

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
```

## Troubleshooting

### Tests Not Compiling
```bash
# Clean and rebuild
cargo clean
cargo build --test prometheus_metrics_test

# Check for missing dependencies
cargo tree
```

### Tests Timing Out
```bash
# Increase timeout for slow systems
cargo test --test prometheus_metrics_test -- --test-threads=1 --timeout=300
```

### Benchmarks Failing
```bash
# Run with more iterations
cargo bench -- --sample-size 10

# Run specific benchmark with debug output
cargo bench bench_counter_increment -- --verbose
```

### Permission Issues
```bash
# Ensure write permissions for criterion output
chmod -R 755 target/criterion/
```

## Performance Expectations

### Unit Tests
- **Total runtime**: < 1 second
- **Per test**: < 50ms (most < 1ms)

### Integration Tests
- **Total runtime**: < 5 seconds
- **Per test**: < 500ms

### Performance Tests
- **Total runtime**: < 10 seconds
- **Per test**: < 2 seconds

### Benchmarks
- **Total runtime**: 5-10 minutes (full suite)
- **Quick mode**: 1-2 minutes (--quick flag)

## Output Locations

### Test Results
- Console output (stdout/stderr)
- JUnit XML: Use `cargo test -- --format=junit > test-results.xml`

### Benchmark Results
- HTML reports: `target/criterion/report/index.html`
- Individual benchmarks: `target/criterion/<benchmark_name>/`
- JSON data: `target/criterion/<benchmark_name>/base/estimates.json`

### Coverage Reports
- HTML: `tarpaulin-report.html`
- XML (Cobertura): `cobertura.xml`
- JSON: Use `--out Json`

## Best Practices

1. **Run tests frequently**
   ```bash
   # Use cargo watch for automatic test running
   cargo install cargo-watch
   cargo watch -x "test --test prometheus_metrics_test"
   ```

2. **Profile slow tests**
   ```bash
   cargo test --test prometheus_metrics_test -- --nocapture --test-threads=1 | grep "ok ("
   ```

3. **Check for flaky tests**
   ```bash
   # Run tests 100 times
   for i in {1..100}; do
     cargo test --test prometheus_metrics_test || break
   done
   ```

4. **Monitor resource usage**
   ```bash
   # Monitor memory and CPU during tests
   /usr/bin/time -v cargo test --test prometheus_metrics_test
   ```

## Next Steps

1. **Wait for Implementation Engineer** to complete the Prometheus metrics module
2. **Activate Tests** by uncommenting code in test file
3. **Run Full Suite** to verify implementation
4. **Generate Reports** for documentation
5. **Set up CI/CD** for automated testing

## Support

For issues or questions:
- Review test documentation: `tests/README_METRICS_TESTS.md`
- Check implementation guide: `docs/PROMETHEUS_METRICS_SPEC.md`
- Examine test helper functions: `tests/common/mod.rs`
