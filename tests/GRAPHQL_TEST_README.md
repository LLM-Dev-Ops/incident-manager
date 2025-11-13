# GraphQL API Test Suite

## Overview

This directory contains comprehensive test coverage for the LLM Incident Manager GraphQL API implementation. The test suite is designed to validate all aspects of the GraphQL implementation using `async-graphql` 7.0+.

## Test Structure

### Test Files

| File | Purpose | Test Count |
|------|---------|------------|
| `graphql_api_test.rs` | Main test suite with all test categories | 80+ tests |
| `common/graphql_helpers.rs` | Test utilities and helper functions | N/A |

### Benchmark Files

| File | Purpose | Benchmark Count |
|------|---------|-----------------|
| `benches/graphql_benchmark.rs` | Performance benchmarks using Criterion | 13 benchmark groups |

## Test Categories

### 1. Type Tests (10 tests)

Tests for GraphQL type system components:

- **Enum Serialization** (4 tests)
  - Severity: P0, P1, P2, P3, P4
  - IncidentStatus: NEW, ACKNOWLEDGED, IN_PROGRESS, ESCALATED, RESOLVED, CLOSED
  - Category: PERFORMANCE, SECURITY, AVAILABILITY, COMPLIANCE, COST, OTHER
  - Environment: PRODUCTION, STAGING, DEVELOPMENT, QA

- **Custom Scalars** (2 tests)
  - UUID validation and serialization
  - DateTime (ISO 8601) formatting

- **Field Resolvers** (2 tests)
  - Incident metrics (MTTD, MTTA, MTTR)
  - Related incidents (via correlation)

- **Type Serialization** (2 tests)
  - Complex object serialization
  - Nested type resolution

### 2. Query Tests (17 tests)

Tests for all GraphQL query operations:

- **Basic Queries** (3 tests)
  - Get incident by ID
  - List incidents
  - Handle not found errors

- **Pagination** (2 tests)
  - Cursor-based forward pagination
  - Cursor-based backward pagination

- **Filtering** (5 tests)
  - Single field filters (severity, status)
  - Date range filtering
  - Complex multi-field filters
  - Text search

- **Sorting** (2 tests)
  - Single field sorting
  - Multi-field sorting

- **Analytics** (2 tests)
  - Incident analytics query
  - Team metrics query

- **Advanced** (3 tests)
  - Nested field resolution
  - DataLoader integration
  - Complex queries

### 3. Mutation Tests (12 tests)

Tests for all GraphQL mutation operations:

- **Create Operations** (3 tests)
  - Successful incident creation
  - Input validation errors
  - Deduplication handling

- **Update Operations** (2 tests)
  - Update incident fields
  - Handle not found

- **State Transitions** (4 tests)
  - Acknowledge incident
  - Invalid state transition
  - Resolve incident
  - Resolve with playbook

- **Advanced Mutations** (3 tests)
  - Escalate incident
  - Execute playbook
  - Batch mutations
  - Idempotency

### 4. Subscription Tests (10 tests)

Tests for GraphQL subscription (WebSocket) operations:

- **WebSocket Connection** (2 tests)
  - Connection establishment
  - Authentication via connectionParams

- **Event Subscriptions** (4 tests)
  - incidentCreated subscription
  - incidentUpdated subscription
  - incidentEscalated subscription
  - correlationGroupUpdated subscription

- **Filtering** (1 test)
  - Filter subscriptions by criteria

- **Reliability** (3 tests)
  - Multiple subscribers
  - Disconnection handling
  - Error scenarios
  - Keep-alive (ping/pong)

### 5. DataLoader Tests (6 tests)

Tests for N+1 query prevention and batching:

- **Batching** (2 tests)
  - User batching
  - Team batching

- **Caching** (1 test)
  - Per-request cache behavior

- **N+1 Prevention** (1 test)
  - Verify query count reduction

- **Error Handling** (1 test)
  - Partial batch failures

- **Performance** (1 test)
  - Large batch efficiency

### 6. Integration Tests (6 tests)

End-to-end workflow tests:

- Complete incident lifecycle
- Complex nested queries
- Mutation → Query consistency
- Subscription delivery guarantees
- GraphQL Playground access
- Schema introspection

### 7. Performance Tests (7 tests)

Performance and scalability tests:

- Query complexity calculation
- Complexity limit enforcement
- Query depth limiting
- Query execution time
- Concurrent request handling
- Subscription memory usage
- DataLoader efficiency metrics

### 8. Security Tests (10 tests)

Security and authorization tests:

- Authentication enforcement (3 tests)
- Field-level authorization (2 tests)
- Query depth attack prevention
- Query cost attack prevention
- Rate limiting (per-user and per-IP)
- Input sanitization
- Introspection control
- Error information disclosure

## Running Tests

### Prerequisites

```bash
# Ensure GraphQL implementation is complete
# The following dependencies must be in Cargo.toml:
# - async-graphql = { version = "7.0", features = ["chrono", "uuid", "dataloader"] }
# - async-graphql-axum = "7.0"
```

### Run All Tests

```bash
# Run the complete GraphQL test suite
cargo test --test graphql_api_test

# Run with output
cargo test --test graphql_api_test -- --nocapture

# Run specific test module
cargo test --test graphql_api_test graphql_query_tests

# Run specific test
cargo test --test graphql_api_test test_query_incident_by_id
```

### Run Tests by Category

```bash
# Type tests only
cargo test --test graphql_api_test graphql_types_tests

# Query tests only
cargo test --test graphql_api_test graphql_query_tests

# Mutation tests only
cargo test --test graphql_api_test graphql_mutation_tests

# Subscription tests only
cargo test --test graphql_api_test graphql_subscription_tests

# DataLoader tests only
cargo test --test graphql_api_test graphql_dataloader_tests

# Integration tests only
cargo test --test graphql_api_test graphql_integration_tests

# Performance tests only
cargo test --test graphql_api_test graphql_performance_tests

# Security tests only
cargo test --test graphql_api_test graphql_security_tests
```

### Run Benchmarks

```bash
# Run all GraphQL benchmarks
cargo bench --bench graphql_benchmark

# Run specific benchmark group
cargo bench --bench graphql_benchmark simple_query

# Generate HTML reports (in target/criterion/)
cargo bench --bench graphql_benchmark
open target/criterion/report/index.html

# Quick benchmark (shorter measurement time)
cargo bench --bench graphql_benchmark -- --quick
```

### Generate Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin \
  --test graphql_api_test \
  --out Html \
  --output-dir coverage/graphql

# View coverage
open coverage/graphql/index.html

# Generate JSON for CI
cargo tarpaulin \
  --test graphql_api_test \
  --out Json \
  --output-dir coverage/graphql
```

## Test Implementation Status

### ✅ Completed

- [x] Test structure created
- [x] All test stubs implemented
- [x] Test documentation written
- [x] Benchmark suite created
- [x] Helper utilities defined

### ⏳ Pending (awaiting GraphQL implementation)

- [ ] Implement test helper functions
- [ ] Add actual GraphQL schema creation
- [ ] Implement WebSocket client utilities
- [ ] Add database query counting utilities
- [ ] Execute and validate all tests
- [ ] Generate coverage reports
- [ ] Tune performance benchmarks

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Simple query | < 10ms | P95 latency |
| Complex query | < 100ms | P95 latency |
| Mutation | < 50ms | P95 latency |
| Subscription creation | < 5ms | Average |
| Event broadcast (100 subscribers) | < 100ms | Total time |
| Query complexity calculation | < 1ms | Average |
| DataLoader batch | 2 queries | For N incidents + relations |
| Concurrent queries (100) | No errors | Success rate |
| Memory per subscription | < 10KB | Average |

## Coverage Goals

| Category | Target | Priority |
|----------|--------|----------|
| Type system | 100% | High |
| Queries | 95%+ | High |
| Mutations | 95%+ | High |
| Subscriptions | 90%+ | High |
| DataLoaders | 95%+ | High |
| Error handling | 90%+ | Medium |
| Security | 100% | Critical |
| Integration | 80%+ | Medium |

## Testing Best Practices

### 1. Isolation

Each test should:
- Create its own test data
- Not depend on other tests
- Clean up after execution
- Use unique identifiers

### 2. Assertions

Use specific assertions:
```rust
// Good
assert_eq!(incident.severity, Severity::P0);
assert!(response.errors.is_none());

// Avoid
assert!(true);
```

### 3. Error Testing

Always test error cases:
```rust
// Test successful case
let result = execute_query(valid_query).await;
assert_no_errors(&result);

// Test error case
let result = execute_query(invalid_query).await;
assert_error_code(&result, "VALIDATION_ERROR");
```

### 4. Async Handling

Properly handle async operations:
```rust
#[tokio::test]
async fn test_name() {
    let result = async_operation().await;
    // assertions
}
```

### 5. Subscription Testing

For subscriptions, use timeouts:
```rust
use tokio::time::{timeout, Duration};

let event = timeout(
    Duration::from_secs(5),
    subscription_stream.next()
).await.expect("Timeout waiting for event");
```

## Debugging Failed Tests

### 1. Enable Detailed Logging

```bash
RUST_LOG=debug cargo test --test graphql_api_test test_name -- --nocapture
```

### 2. Inspect GraphQL Errors

```rust
if let Some(errors) = response.errors {
    for error in errors {
        eprintln!("Error: {:?}", error);
        eprintln!("Path: {:?}", error.path);
        eprintln!("Extensions: {:?}", error.extensions);
    }
}
```

### 3. Validate Schema

```bash
# Test introspection to verify schema
cargo test test_schema_introspection -- --nocapture
```

### 4. Check DataLoader Batching

```rust
let query_count = count_database_queries(|| {
    execute_query(query_with_relations).await
}).await;

println!("Queries executed: {}", query_count);
assert_eq!(query_count, 2, "Should use batching");
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: GraphQL Tests

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

      - name: Run GraphQL Tests
        run: cargo test --test graphql_api_test

      - name: Run GraphQL Benchmarks
        run: cargo bench --bench graphql_benchmark -- --quick

      - name: Generate Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --test graphql_api_test --out Xml

      - name: Upload Coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          flags: graphql-tests
```

## Common Issues and Solutions

### Issue: WebSocket Connection Fails

**Solution**: Ensure server is running and WebSocket endpoint is correct
```rust
// Check server is listening
let health = reqwest::get("http://localhost:8080/health").await;
assert!(health.is_ok());

// Then try WebSocket
let ws_client = create_websocket_client().await;
```

### Issue: DataLoader Not Batching

**Solution**: Verify requests are within same execution context
```rust
// All these should batch together
let incidents = query_incidents(first: 10).await;
for edge in incidents.edges {
    // These user loads will batch
    let user = edge.node.assigned_to;
}
```

### Issue: Subscription Not Receiving Events

**Solution**: Ensure subscription is established before triggering event
```rust
// Subscribe first
let mut stream = subscribe("incidentCreated").await;

// Give time to establish
tokio::time::sleep(Duration::from_millis(100)).await;

// Then trigger event
create_incident(data).await;

// Now receive
let event = stream.next().await;
```

### Issue: Query Complexity Rejection

**Solution**: Reduce query depth or increase complexity limit
```rust
// Instead of deep nesting
query {
  incidents {
    edges {
      node {
        relatedIncidents {
          relatedIncidents { # Too deep
            ...
          }
        }
      }
    }
  }
}

// Use separate queries
query {
  incident(id: $id) {
    relatedIncidents {
      id
    }
  }
}
```

## Resources

- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [GraphQL Specification](https://spec.graphql.org/)
- [Relay Cursor Specification](https://relay.dev/graphql/connections.htm)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)
- [DataLoader Pattern](https://github.com/graphql/dataloader)

## Support

For issues or questions about the GraphQL test suite:

1. Check test output and logs
2. Review GraphQL API Guide: `/docs/GRAPHQL_API_GUIDE.md`
3. Check implementation guide: `/docs/GRAPHQL_IMPLEMENTATION_GUIDE.md`
4. Open an issue with test failure details

## Maintenance

### Adding New Tests

1. Add test function to appropriate module in `graphql_api_test.rs`
2. Follow naming convention: `test_<category>_<specific_behavior>`
3. Add TODO comments explaining what to test
4. Update this README with test count
5. Update coverage goals if needed

### Updating Benchmarks

1. Add benchmark function to `graphql_benchmark.rs`
2. Add to `criterion_group!` macro
3. Document expected performance
4. Update performance targets in this README

### Test Review Checklist

- [ ] Test name is descriptive
- [ ] Test has clear assertions
- [ ] Test handles async correctly
- [ ] Test cleans up resources
- [ ] Test is documented with TODO
- [ ] Test added to appropriate module
- [ ] README updated if needed
