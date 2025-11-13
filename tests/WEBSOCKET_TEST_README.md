# WebSocket Testing Suite

## Overview

Comprehensive test coverage for the WebSocket streaming implementation through GraphQL subscriptions.

## Test Files

### 1. `websocket_unit_test.rs` - Unit Tests

Tests for core WebSocket message handling and serialization:

- **Message Serialization** (10 tests)
  - GraphQL subscription message formats
  - Connection init/ack messages
  - Subscribe/stop/terminate messages
  - Data message serialization
  - Error message handling
  - Invalid message handling
  - Large message handling

- **Message Validation** (4 tests)
  - Subscription query validation
  - Severity enum validation
  - UUID format validation
  - Timestamp format validation

- **Subscription Filters** (6 tests)
  - Filter by incident IDs
  - Filter by severities
  - Filter by active status
  - Filter by source
  - Combined filters
  - Filter logic testing

- **Connection State** (2 tests)
  - Connection state tracking
  - Concurrent subscription tracking

- **Session Management** (2 tests)
  - Session creation and validation
  - Multiple sessions handling

**Total: 24 unit tests**

### 2. `websocket_integration_test.rs` - Integration Tests

Tests for WebSocket connection lifecycle and end-to-end functionality:

- **Connection Lifecycle** (6 tests)
  - WebSocket endpoint configuration
  - GraphQL-WS protocol messages
  - Connection initialization flow
  - Subscription lifecycle
  - Multiple concurrent subscriptions
  - Graceful disconnection

- **Subscription Operations** (6 tests)
  - Incident updates subscription
  - New incidents subscription
  - Critical incidents subscription
  - Incident state changes subscription
  - Alerts subscription
  - Filtered subscriptions with variables

- **Event Streaming** (6 tests)
  - Heartbeat stream generation
  - Incident event streams
  - Filtered event streams
  - Event ordering verification
  - Stream backpressure handling

- **Error Handling** (7 tests)
  - Invalid GraphQL query format
  - Missing subscription ID
  - Invalid message types
  - Oversized message detection
  - Unauthorized topic access
  - Connection timeout simulation
  - Error message propagation

- **Security** (4 tests)
  - Authentication headers
  - Rate limiting tracking
  - Message size limits
  - Connection limit tracking

- **Reliability** (5 tests)
  - Connection recovery simulation
  - Message delivery with retries
  - Session persistence across reconnects
  - Graceful shutdown
  - Stream resumption after interruption

**Total: 34 integration tests**

### 3. `websocket_performance_test.rs` - Performance Tests

Tests for scalability, throughput, and performance:

- **Concurrent Connections** (3 tests)
  - 100 concurrent connections
  - 1000 concurrent connections scalability
  - Connection pool management

- **Message Throughput** (4 tests)
  - 1000+ messages/second throughput
  - Sustained throughput over time
  - Multi-subscriber broadcast
  - Message batching efficiency

- **Latency Measurements** (2 tests)
  - Message latency tracking (p50, p95, p99)
  - End-to-end latency measurement

- **Backpressure** (3 tests)
  - Bounded channel backpressure
  - Stream backpressure handling
  - Buffer overflow handling

- **Memory Usage** (3 tests)
  - Memory-efficient message handling
  - Subscription cleanup
  - Message pool reuse

- **Scalability** (2 tests)
  - Horizontal scaling simulation
  - Load balancing distribution

**Total: 17 performance tests**

### 4. `websocket_graphql_subscription_test.rs` - GraphQL Subscription Tests

Comprehensive tests for GraphQL subscription functionality:

- **GraphQL Subscription Queries** (5 tests)
  - Incident updates subscription structure
  - New incidents subscription structure
  - Critical incidents subscription structure
  - Incident state changes subscription structure
  - Alerts subscription structure

- **Subscription Message Flow** (5 tests)
  - Complete subscription flow
  - Subscriptions with variables
  - Error handling flow
  - Keepalive ping/pong
  - Protocol compliance

- **Subscription Filtering** (5 tests)
  - Filter by severity
  - Filter by incident IDs
  - Filter by active status
  - Filter by alert source
  - Combined filters

- **Update Types** (7 tests)
  - CREATED updates
  - UPDATED updates
  - STATE_CHANGED updates
  - RESOLVED updates
  - ASSIGNED updates
  - COMMENT_ADDED updates
  - HEARTBEAT updates

- **Subscription Lifecycle Management** (3 tests)
  - Subscription registry
  - Multiple subscriptions per connection
  - Cleanup on disconnect

- **GraphQL Protocol Compliance** (4 tests)
  - Subprotocol validation
  - Message type validity
  - Payload structure
  - Subscription ID format

- **Edge Cases** (5 tests)
  - Empty filter arrays
  - Null optional parameters
  - Subscriptions without filters
  - Rapid subscribe/unsubscribe
  - Duplicate subscription IDs

**Total: 34 GraphQL subscription tests**

## Test Coverage Summary

| Category | Test Count | Coverage |
|----------|-----------|----------|
| Unit Tests | 24 | Message handling, validation, filters |
| Integration Tests | 34 | Lifecycle, streaming, security |
| Performance Tests | 17 | Load, throughput, latency |
| GraphQL Tests | 34 | Subscriptions, protocol, filtering |
| **TOTAL** | **109** | **Comprehensive coverage** |

## Running the Tests

### All Tests
```bash
cargo test --all-targets
```

### Unit Tests Only
```bash
cargo test --test websocket_unit_test
```

### Integration Tests Only
```bash
cargo test --test websocket_integration_test
```

### Performance Tests Only
```bash
cargo test --test websocket_performance_test
```

### GraphQL Subscription Tests Only
```bash
cargo test --test websocket_graphql_subscription_test
```

### Specific Test Module
```bash
cargo test --test websocket_unit_test message_serialization
cargo test --test websocket_integration_test connection_lifecycle
cargo test --test websocket_performance_test concurrent_connections
```

### Run with Output
```bash
cargo test --test websocket_unit_test -- --nocapture
```

### Run in Release Mode (for performance tests)
```bash
cargo test --release --test websocket_performance_test
```

## Performance Benchmarks

### Target Metrics

| Metric | Target | Test Coverage |
|--------|--------|---------------|
| Concurrent Connections | 100+ | Yes (1000 tested) |
| Message Throughput | 1000+ msg/s | Yes |
| Message Latency (p95) | < 10ms | Yes |
| Memory Usage | Efficient | Yes |
| Backpressure Handling | Robust | Yes |

### Expected Results

- **Concurrent Connections**: Should handle 1000+ simultaneous WebSocket connections
- **Throughput**: Should process 1000+ messages/second per connection
- **Latency**: p95 latency should be under 10ms for message processing
- **Memory**: Should maintain reasonable memory usage under load
- **Reliability**: Should handle disconnections, errors, and recovery gracefully

## Test Categories

### 1. Message Handling Tests
- Serialization/deserialization
- Protocol compliance
- Message validation
- Error handling

### 2. Connection Management Tests
- Connection lifecycle
- Authentication
- Session management
- Graceful shutdown

### 3. Subscription Tests
- Query validation
- Filter application
- Multiple subscriptions
- Subscription cleanup

### 4. Event Streaming Tests
- Event delivery
- Ordering guarantees
- Filtering
- Backpressure

### 5. Performance Tests
- Load testing
- Throughput testing
- Latency measurement
- Scalability testing

### 6. Security Tests
- Authentication enforcement
- Authorization checks
- Rate limiting
- Message size limits

### 7. Reliability Tests
- Connection recovery
- Message delivery guarantees
- Session persistence
- Error recovery

## CI/CD Integration

### GitHub Actions Example

```yaml
name: WebSocket Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run unit tests
        run: cargo test --test websocket_unit_test
      - name: Run integration tests
        run: cargo test --test websocket_integration_test
      - name: Run performance tests
        run: cargo test --release --test websocket_performance_test
```

## Test Data

### Sample WebSocket Messages

#### Connection Init
```json
{
  "type": "connection_init",
  "payload": {}
}
```

#### Subscribe
```json
{
  "id": "1",
  "type": "subscribe",
  "payload": {
    "query": "subscription { incidentUpdates { updateType incidentId timestamp } }"
  }
}
```

#### Data Message
```json
{
  "id": "1",
  "type": "next",
  "payload": {
    "data": {
      "incidentUpdates": {
        "updateType": "CREATED",
        "incidentId": "550e8400-e29b-41d4-a716-446655440000",
        "timestamp": "2025-11-12T00:00:00Z"
      }
    }
  }
}
```

## Troubleshooting

### Common Issues

1. **Connection Timeout**
   - Check network connectivity
   - Verify server is running
   - Check firewall rules

2. **Subscription Not Receiving Updates**
   - Verify query syntax
   - Check filters are correct
   - Ensure events are being generated

3. **High Latency**
   - Check server load
   - Review network conditions
   - Verify backpressure handling

4. **Memory Issues**
   - Check subscription cleanup
   - Verify message pooling
   - Review connection limits

## Additional Resources

- [GraphQL WebSocket Protocol](https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md)
- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [Axum WebSocket Guide](https://docs.rs/axum/latest/axum/extract/ws/index.html)

## Test Maintenance

- Tests should be run before every commit
- Performance benchmarks should be tracked over time
- Test coverage should remain above 80%
- New features should include corresponding tests
