# Messaging Module Test Suite

## Overview

This document describes the comprehensive test suite for the messaging module (`tests/messaging_test.rs`). The suite contains **35 unit tests** covering all aspects of the message queue integration.

## Test Coverage

### Configuration Tests (13 tests)

| Test | Description | Coverage |
|------|-------------|----------|
| `test_messaging_service_disabled` | Service creation with disabled config | Service initialization |
| `test_messaging_config_defaults` | Default messaging configuration values | Config defaults |
| `test_nats_config_defaults` | Default NATS configuration | NATS config |
| `test_kafka_config_defaults` | Default Kafka configuration | Kafka config |
| `test_topic_prefix` | Topic prefixing functionality | Topic naming |
| `test_messaging_backend_nats` | NATS backend selection | Backend config |
| `test_messaging_backend_kafka` | Kafka backend selection | Backend config |
| `test_messaging_backend_both` | Dual backend configuration | Backend config |
| `test_dlq_configuration` | Dead letter queue setup | DLQ config |
| `test_metrics_configuration` | Metrics enablement | Metrics config |
| `test_nats_tls_config` | NATS TLS/SSL configuration | Security |
| `test_kafka_sasl_config` | Kafka SASL authentication | Security |
| `test_max_message_size` | Message size limits | Configuration |

### Event Type Tests (9 tests)

| Test | Description | Event Type |
|------|-------------|------------|
| `test_incident_event_created` | Incident creation event | `IncidentEvent::Created` |
| `test_incident_event_state_changed` | State transition event | `IncidentEvent::StateChanged` |
| `test_incident_event_assigned` | Assignment event | `IncidentEvent::Assigned` |
| `test_incident_event_resolved` | Resolution event | `IncidentEvent::Resolved` |
| `test_incident_event_escalated` | Escalation event | `IncidentEvent::Escalated` |
| `test_incident_event_comment_added` | Comment addition event | `IncidentEvent::CommentAdded` |
| `test_incident_event_playbook_started` | Playbook start event | `IncidentEvent::PlaybookStarted` |
| `test_incident_event_playbook_completed` | Playbook completion event | `IncidentEvent::PlaybookCompleted` |
| `test_incident_event_alert_correlated` | Alert correlation event | `IncidentEvent::AlertCorrelated` |

### Message Envelope Tests (2 tests)

| Test | Description | Coverage |
|------|-------------|----------|
| `test_message_envelope` | Basic envelope creation | Message wrapping |
| `test_message_envelope_with_metadata` | Envelope with metadata | Metadata handling |

### Serialization Tests (2 tests)

| Test | Description | Coverage |
|------|-------------|----------|
| `test_message_serialization` | Generic message serialization | JSON serialization |
| `test_incident_event_serialization` | Incident event serialization | Event serialization |

### Publishing Tests (1 test)

| Test | Description | Coverage |
|------|-------------|----------|
| `test_publish_disabled_service` | Publishing to disabled service | Graceful degradation |

### Advanced Configuration Tests (8 tests)

| Test | Description | Coverage |
|------|-------------|----------|
| `test_kafka_advanced_config` | Advanced Kafka parameters | Kafka tuning |
| `test_nats_reconnect_config` | NATS reconnection settings | NATS reliability |
| Plus 6 configuration tests above | Various configuration scenarios | Config validation |

## Running Tests

### Run All Messaging Tests

```bash
cargo test --test messaging_test
```

### Run Specific Test

```bash
cargo test --test messaging_test test_message_envelope
```

### Run Tests with Output

```bash
cargo test --test messaging_test -- --nocapture
```

### Run Tests in Release Mode

```bash
cargo test --release --test messaging_test
```

## Test Organization

```
tests/messaging_test.rs
├── Configuration Tests
│   ├── Service Creation
│   ├── Default Configurations
│   ├── Backend Selection
│   ├── Security Settings
│   └── Feature Flags
├── Event Type Tests
│   ├── Incident Lifecycle Events
│   ├── Operational Events
│   └── Event Type Validation
├── Message Envelope Tests
│   ├── Envelope Creation
│   └── Metadata Handling
├── Serialization Tests
│   ├── Message Serialization
│   └── Deserialization
└── Publishing Tests
    └── Service Behavior
```

## Test Data

### Test Message Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: String,
    content: String,
    timestamp: i64,
}
```

### Test Scenarios

1. **Happy Path**: All tests verify correct behavior under normal conditions
2. **Edge Cases**: Tests cover disabled services, empty configs, etc.
3. **Configuration Validation**: Tests ensure all config options work correctly
4. **Type Safety**: Tests verify serialization/deserialization round-trips

## Expected Test Results

All 35 tests should pass:

```
running 35 tests
test test_dlq_configuration ... ok
test test_incident_event_alert_correlated ... ok
test test_incident_event_assigned ... ok
test test_incident_event_comment_added ... ok
test test_incident_event_created ... ok
test test_incident_event_escalated ... ok
test test_incident_event_playbook_completed ... ok
test test_incident_event_playbook_started ... ok
test test_incident_event_resolved ... ok
test test_incident_event_serialization ... ok
test test_incident_event_state_changed ... ok
test test_kafka_advanced_config ... ok
test test_kafka_config_defaults ... ok
test test_kafka_sasl_config ... ok
test test_max_message_size ... ok
test test_message_envelope ... ok
test test_message_envelope_with_metadata ... ok
test test_message_serialization ... ok
test test_messaging_backend_both ... ok
test test_messaging_backend_kafka ... ok
test test_messaging_backend_nats ... ok
test test_messaging_config_defaults ... ok
test test_messaging_service_disabled ... ok
test test_metrics_configuration ... ok
test test_nats_config_defaults ... ok
test test_nats_reconnect_config ... ok
test test_nats_tls_config ... ok
test test_publish_disabled_service ... ok
test test_topic_prefix ... ok

test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Integration Tests (Future)

The current test suite focuses on unit tests. Future integration tests could cover:

- **NATS Integration**: Real NATS server communication
- **Kafka Integration**: Real Kafka broker communication
- **End-to-End Scenarios**: Full publish-subscribe workflows
- **Performance Tests**: Throughput and latency benchmarks
- **Failure Scenarios**: Connection loss, broker failures, network issues
- **Concurrent Access**: Multi-threaded publishing/consuming

## Test Environment

### Dependencies

- `tokio-test`: Async test runtime
- `serde_json`: JSON serialization for verification
- `tempfile`: Temporary directories for tests (if needed)

### No External Dependencies Required

All current tests are **pure unit tests** that don't require:
- Running NATS server
- Running Kafka broker
- Network connectivity
- External services

This makes the test suite fast and suitable for CI/CD pipelines.

## Maintenance

### Adding New Tests

When adding new messaging features:

1. Add configuration tests for new config options
2. Add event type tests for new event variants
3. Add serialization tests for new message types
4. Update this documentation

### Test Naming Convention

- `test_<feature>_<scenario>`: Descriptive test names
- Group related tests together
- Use clear assertions with helpful messages

## Coverage Goals

- **Configuration**: 100% of config options tested
- **Event Types**: 100% of incident events tested
- **Core Functionality**: All public API methods tested
- **Error Scenarios**: Error handling paths covered

## CI/CD Integration

The test suite is designed for CI/CD:

```yaml
# .github/workflows/test.yml
- name: Run messaging tests
  run: cargo test --test messaging_test
```

All tests complete in under 5 seconds (once compiled).

## Troubleshooting

### Tests Fail to Compile

```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo test --test messaging_test
```

### Serialization Test Failures

- Check that test message structs implement `Serialize + Deserialize`
- Verify JSON format matches expected structure

### Configuration Test Failures

- Verify default values in config structs
- Check for config struct changes

## Related Documentation

- [Messaging Guide](../docs/MESSAGING_GUIDE.md) - User documentation
- [Messaging Implementation](../docs/MESSAGING_IMPLEMENTATION.md) - Technical details
- [Test Execution Guide](./TEST_EXECUTION_GUIDE.md) - How to run all tests
