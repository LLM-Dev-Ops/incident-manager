# WebSocket Client Examples

This directory contains example implementations of WebSocket clients for connecting to the LLM Incident Manager's real-time incident streaming API.

## Available Examples

### TypeScript/JavaScript Client
**File:** `typescript-client.ts`

A fully-featured TypeScript client demonstrating:
- Connection management with automatic reconnection
- Multiple subscription types
- Error handling and recovery
- Authentication with JWT tokens

**Usage:**
```bash
# Install dependencies
npm install graphql-ws ws

# Run in Node.js
npx ts-node typescript-client.ts

# Or compile and run
tsc typescript-client.ts
node typescript-client.js
```

**Environment Variables:**
```bash
export WS_URL="ws://localhost:8080/graphql/ws"
export AUTH_TOKEN="your_jwt_token"
```

---

### Python Client
**File:** `python-client.py`

An async Python client using `gql` library demonstrating:
- Async/await pattern for subscriptions
- Concurrent subscription handling
- Graceful shutdown
- Signal handling (SIGINT, SIGTERM)

**Usage:**
```bash
# Install dependencies
pip install websockets gql[websockets]

# Run the client
python python-client.py

# Or with environment variables
WS_URL="ws://localhost:8080/graphql/ws" AUTH_TOKEN="your_token" python python-client.py
```

---

### Rust Client
**File:** `rust-client.rs`

A high-performance Rust client using tokio and tungstenite:
- Zero-cost abstractions
- Type-safe message handling
- Efficient async I/O
- Production-ready error handling

**Usage:**
```bash
# Create new Cargo project
cargo new websocket-client
cd websocket-client

# Copy rust-client.rs to src/main.rs
cp ../rust-client.rs src/main.rs

# Add dependencies to Cargo.toml
cat >> Cargo.toml << EOF
tokio = { version = "1.35", features = ["full"] }
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
futures-util = "0.3"
anyhow = "1.0"
EOF

# Build and run
cargo run

# Or with environment variables
WS_URL="ws://localhost:8080/graphql/ws" AUTH_TOKEN="your_token" cargo run
```

---

## Common Features

All example clients demonstrate:

1. **Connection Management**
   - Initial WebSocket connection
   - Connection initialization with authentication
   - Automatic reconnection with exponential backoff

2. **Subscription Types**
   - Critical incidents (P0/P1)
   - Incident updates with severity filtering
   - New incident notifications

3. **Error Handling**
   - Network errors
   - GraphQL errors
   - Authentication failures
   - Graceful degradation

4. **Production Patterns**
   - Logging and monitoring
   - Health checks
   - Resource cleanup
   - Signal handling for graceful shutdown

## GraphQL Subscriptions

### Critical Incidents
Subscribe to P0 and P1 incidents only:
```graphql
subscription {
  criticalIncidents {
    id
    title
    severity
    state
    affectedResources
  }
}
```

### Incident Updates
Subscribe to incident lifecycle events:
```graphql
subscription IncidentUpdates($severities: [Severity!]) {
  incidentUpdates(severities: $severities, activeOnly: true) {
    updateType
    incidentId
    timestamp
  }
}
```

### New Incidents
Subscribe to newly created incidents:
```graphql
subscription NewIncidents($severities: [Severity!]) {
  newIncidents(severities: $severities) {
    id
    title
    description
    severity
    state
    createdAt
  }
}
```

## Configuration

### WebSocket URL
- **Local:** `ws://localhost:8080/graphql/ws`
- **Production (TLS):** `wss://api.example.com/graphql/ws`

### Authentication
All clients support JWT token authentication:
- Pass token in connection init payload
- Token should have appropriate permissions
- Tokens are typically valid for 1-24 hours

### Environment Variables
| Variable | Description | Default |
|----------|-------------|---------|
| `WS_URL` | WebSocket endpoint URL | `ws://localhost:8080/graphql/ws` |
| `AUTH_TOKEN` | JWT authentication token | Required |
| `LOG_LEVEL` | Logging verbosity | `info` |

## Testing

### Local Development
1. Start the LLM Incident Manager:
   ```bash
   cargo run --bin llm-incident-manager
   ```

2. In another terminal, run a client:
   ```bash
   # TypeScript
   npm run start

   # Python
   python python-client.py

   # Rust
   cargo run
   ```

3. Create test incidents via REST API:
   ```bash
   curl -X POST http://localhost:8080/v1/alerts \
     -H "Content-Type: application/json" \
     -d '{
       "title": "Test P0 Incident",
       "description": "Testing WebSocket notifications",
       "severity": "P0",
       "source": "test",
       "alert_type": "availability"
     }'
   ```

### Integration Testing
See `tests/integration/websocket_test.rs` for automated integration tests.

## Troubleshooting

### Connection Refused
```
Error: Connection refused
```
**Solution:** Verify the server is running and the URL is correct.

### Authentication Failed
```
Error: Unauthorized
```
**Solution:** Check that your JWT token is valid and not expired.

### No Data Received
```
Connected successfully but no data
```
**Solution:** Verify that:
1. Incidents are being created
2. Filters match incident severity
3. Subscription query is correct

### Connection Drops
```
Error: Connection closed unexpectedly
```
**Solution:** Implement reconnection logic (all examples include this).

## Best Practices

1. **Always Use TLS in Production**
   ```
   wss://api.example.com/graphql/ws  ✓
   ws://api.example.com/graphql/ws   ✗
   ```

2. **Implement Exponential Backoff**
   ```typescript
   retryDelay = Math.min(baseDelay * 2^attempts, maxDelay)
   ```

3. **Handle Authentication Expiry**
   - Refresh tokens before expiry
   - Reconnect with new token on 401

4. **Use Specific Filters**
   - Subscribe only to needed severities
   - Filter by specific incident IDs when possible

5. **Monitor Resource Usage**
   - Track memory consumption
   - Monitor connection count
   - Log errors and retries

## Performance Considerations

### Memory Usage
- Each subscription: ~1-2 KB
- Each connection: ~50 KB
- Buffer size: Configurable

### Network Bandwidth
- Heartbeat: ~100 bytes every 30s
- Incident event: ~1-5 KB per incident
- Update event: ~200-500 bytes per update

### Latency
- Message delivery: < 100ms (typical)
- End-to-end: < 500ms (including processing)

## Further Reading

- [WebSocket Streaming Guide](../../docs/WEBSOCKET_STREAMING_GUIDE.md) - Architecture overview
- [WebSocket API Reference](../../docs/WEBSOCKET_API_REFERENCE.md) - Complete API documentation
- [WebSocket Client Guide](../../docs/WEBSOCKET_CLIENT_GUIDE.md) - Detailed integration guide
- [WebSocket Deployment Guide](../../docs/WEBSOCKET_DEPLOYMENT_GUIDE.md) - Production deployment

## License

These examples are provided as-is for integration with the LLM Incident Manager.
