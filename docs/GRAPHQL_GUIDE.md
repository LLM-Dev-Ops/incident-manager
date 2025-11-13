# GraphQL API Guide

## Introduction

This guide covers the GraphQL API implementation for the LLM Incident Manager. The GraphQL API provides a flexible, efficient alternative to the REST API with real-time capabilities.

## Quick Start

### Starting the Server

The GraphQL API starts automatically with the main application:

```bash
cargo run --release
```

The server will log:
```
🚀 HTTP API server listening on http://0.0.0.0:8080
   GraphQL API: http://0.0.0.0:8080/graphql
   GraphQL Playground: http://0.0.0.0:8080/graphql/playground
   GraphQL WebSocket: ws://0.0.0.0:8080/graphql/ws
```

### Accessing the Playground

Open your browser to:
```
http://localhost:8080/graphql/playground
```

This provides an interactive IDE for:
- Writing and testing queries
- Exploring the schema
- Viewing documentation
- Testing subscriptions

## Core Concepts

### Queries

Queries read data from the system. They are cacheable and idempotent.

**Example: Get an incident by ID**
```graphql
query GetIncident {
  incident(id: "550e8400-e29b-41d4-a716-446655440000") {
    id
    title
    severity
    state
    description
  }
}
```

### Mutations

Mutations modify data. They are not cacheable and may have side effects.

**Example: Create an incident**
```graphql
mutation CreateIncident {
  createIncident(
    input: {
      source: "llm-sentinel"
      title: "Database Connection Pool Exhausted"
      description: "All connections in use, new requests timing out"
      severity: P1
      incidentType: Infrastructure
    }
  ) {
    id
    createdAt
    state
  }
}
```

### Subscriptions

Subscriptions provide real-time updates via WebSocket.

**Example: Watch for new critical incidents**
```graphql
subscription WatchCritical {
  criticalIncidents {
    id
    title
    severity
    createdAt
  }
}
```

## Schema Documentation

### Query Type

#### health
Returns system health information.

```graphql
type Query {
  health: HealthInfo!
}

type HealthInfo {
  status: String!
  version: String!
}
```

#### incident
Get a single incident by ID.

```graphql
type Query {
  incident(id: UUID!): Incident!
}
```

**Example:**
```graphql
{
  incident(id: "550e8400-e29b-41d4-a716-446655440000") {
    title
    severity
    state
  }
}
```

#### incidents
List incidents with optional filtering, sorting, and pagination.

```graphql
type Query {
  incidents(
    filter: IncidentFilterInput
    pagination: PaginationInput
    sort: IncidentSortInput
  ): IncidentConnection!
}
```

**Example:**
```graphql
{
  incidents(
    filter: {
      severities: [P0, P1]
      states: [Investigating, Remediating]
      activeOnly: true
    }
    pagination: {
      page: 0
      pageSize: 20
    }
  ) {
    incidents {
      id
      title
      severity
    }
    pageInfo {
      totalCount
      hasNextPage
    }
  }
}
```

#### activeIncidents
Convenience query for active incidents only.

```graphql
type Query {
  activeIncidents(pagination: PaginationInput): IncidentConnection!
}
```

#### criticalIncidents
Convenience query for P0 and P1 incidents only.

```graphql
type Query {
  criticalIncidents(pagination: PaginationInput): IncidentConnection!
}
```

#### searchIncidents
Full-text search across incidents.

```graphql
type Query {
  searchIncidents(
    query: String!
    pagination: PaginationInput
  ): IncidentConnection!
}
```

**Example:**
```graphql
{
  searchIncidents(query: "database timeout") {
    incidents {
      id
      title
      description
    }
  }
}
```

#### incidentStats
Aggregate statistics about incidents.

```graphql
type Query {
  incidentStats: IncidentStats!
}

type IncidentStats {
  total: Int!
  active: Int!
  resolved: Int!
  bySeverity: SeverityStats!
}
```

**Example:**
```graphql
{
  incidentStats {
    total
    active
    resolved
    bySeverity {
      p0
      p1
      p2
      p3
      p4
    }
  }
}
```

#### playbook
Get a single playbook by ID.

```graphql
type Query {
  playbook(id: UUID!): Playbook!
}
```

#### playbooks
List all playbooks.

```graphql
type Query {
  playbooks: [Playbook!]!
}
```

### Mutation Type

#### submitAlert
Submit an alert for processing.

```graphql
type Mutation {
  submitAlert(input: SubmitAlertInput!): AlertAck!
}
```

**Example:**
```graphql
mutation SubmitAlert {
  submitAlert(
    input: {
      source: "llm-shield"
      title: "Unusual LLM Behavior Detected"
      description: "Model outputs contain sensitive information"
      severity: P0
      alertType: Security
      labels: { model: "gpt-4", environment: "production" }
      affectedServices: ["chat-api"]
    }
  ) {
    alertId
    incidentId
    status
    message
  }
}
```

#### createIncident
Create an incident directly (bypassing alert processing).

```graphql
type Mutation {
  createIncident(input: CreateIncidentInput!): Incident!
}
```

#### updateIncident
Update an existing incident.

```graphql
type Mutation {
  updateIncident(
    id: UUID!
    input: UpdateIncidentInput!
  ): Incident!
}
```

**Example:**
```graphql
mutation UpdateIncident {
  updateIncident(
    id: "550e8400-e29b-41d4-a716-446655440000"
    input: {
      state: Investigating
      assignees: ["sre-oncall@example.com"]
      addLabels: { investigation_started: "2025-11-12T10:30:00Z" }
    }
  ) {
    id
    state
    assignees
    updatedAt
  }
}
```

#### resolveIncident
Resolve an incident.

```graphql
type Mutation {
  resolveIncident(
    id: UUID!
    input: ResolveIncidentInput!
  ): Incident!
}
```

**Example:**
```graphql
mutation ResolveIncident {
  resolveIncident(
    id: "550e8400-e29b-41d4-a716-446655440000"
    input: {
      method: Manual
      notes: "Restarted connection pool, increased max connections"
      rootCause: "Connection leak in user service version 2.3.1"
    }
  ) {
    id
    state
    resolution {
      resolvedAt
      resolvedBy
      rootCause
      notes
    }
  }
}
```

#### addComment
Add a comment to an incident.

```graphql
type Mutation {
  addComment(
    incidentId: UUID!
    comment: String!
  ): Incident!
}
```

**Example:**
```graphql
mutation AddComment {
  addComment(
    incidentId: "550e8400-e29b-41d4-a716-446655440000"
    comment: "Rolled back deployment to v2.3.0, monitoring for 30 minutes"
  ) {
    timeline {
      timestamp
      eventType
      description
      actor
    }
  }
}
```

#### assignIncident
Assign users to an incident.

```graphql
type Mutation {
  assignIncident(
    incidentId: UUID!
    assignees: [String!]!
  ): Incident!
}
```

#### linkIncidents
Link two related incidents.

```graphql
type Mutation {
  linkIncidents(
    incidentId: UUID!
    relatedId: UUID!
  ): Incident!
}
```

#### escalateIncident
Escalate an incident to higher severity.

```graphql
type Mutation {
  escalateIncident(
    incidentId: UUID!
    newSeverity: Severity!
    reason: String!
  ): Incident!
}
```

**Example:**
```graphql
mutation EscalateIncident {
  escalateIncident(
    incidentId: "550e8400-e29b-41d4-a716-446655440000"
    newSeverity: P0
    reason: "Customer CEO reported on Twitter, major revenue impact"
  ) {
    id
    severity
    timeline {
      timestamp
      eventType
      description
    }
  }
}
```

### Subscription Type

#### incidentUpdates
Subscribe to incident update events.

```graphql
type Subscription {
  incidentUpdates(
    incidentIds: [UUID!]
    severities: [Severity!]
    activeOnly: Boolean
  ): IncidentUpdate!
}
```

**Example:**
```graphql
subscription WatchIncidents {
  incidentUpdates(severities: [P0, P1], activeOnly: true) {
    updateType
    incidentId
    timestamp
  }
}
```

#### newIncidents
Subscribe to new incident creation events.

```graphql
type Subscription {
  newIncidents(severities: [Severity!]): Incident!
}
```

#### criticalIncidents
Subscribe to critical incident creation events (P0/P1 only).

```graphql
type Subscription {
  criticalIncidents: Incident!
}
```

#### incidentStateChanges
Subscribe to incident state change events.

```graphql
type Subscription {
  incidentStateChanges(incidentId: UUID): IncidentStateChange!
}
```

#### alerts
Subscribe to alert submission events.

```graphql
type Subscription {
  alerts(sources: [String!]): Alert!
}
```

## Advanced Usage

### Using Variables

Variables make queries reusable and safer:

```graphql
query GetIncidentById($incidentId: UUID!) {
  incident(id: $incidentId) {
    id
    title
    severity
  }
}
```

**Variables:**
```json
{
  "incidentId": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Fragments

Fragments reduce duplication:

```graphql
fragment IncidentSummary on Incident {
  id
  title
  severity
  state
  createdAt
}

query GetIncidents {
  activeIncidents {
    incidents {
      ...IncidentSummary
    }
  }

  criticalIncidents {
    incidents {
      ...IncidentSummary
      assignees
    }
  }
}
```

### Aliases

Aliases allow multiple queries with different arguments:

```graphql
query MultipleQueries {
  p0: incidents(filter: { severities: [P0] }) {
    pageInfo { totalCount }
  }

  p1: incidents(filter: { severities: [P1] }) {
    pageInfo { totalCount }
  }

  p2: incidents(filter: { severities: [P2] }) {
    pageInfo { totalCount }
  }
}
```

### Directives

Use `@include` and `@skip` for conditional fields:

```graphql
query GetIncident($id: UUID!, $includeTimeline: Boolean!) {
  incident(id: $id) {
    id
    title
    severity
    timeline @include(if: $includeTimeline) {
      timestamp
      description
    }
  }
}
```

## Performance Optimization

### Field Selection

Only request fields you need:

**Bad:**
```graphql
{
  incidents {
    incidents {
      id
      title
      # ... all fields
    }
  }
}
```

**Good:**
```graphql
{
  incidents {
    incidents {
      id
      title  # Only what you need
    }
  }
}
```

### Pagination

Always use pagination for lists:

```graphql
{
  incidents(pagination: { page: 0, pageSize: 20 }) {
    incidents { id title }
    pageInfo {
      totalCount
      hasNextPage
    }
  }
}
```

### DataLoader Benefits

The implementation automatically batches requests. This query:

```graphql
{
  incidents {
    incidents {
      relatedIncidents {
        id
        title
      }
    }
  }
}
```

...executes 2 database queries instead of N+1:
1. Load all incidents
2. Batch load all related incidents

## Error Handling

GraphQL errors are returned in the response:

```json
{
  "data": null,
  "errors": [
    {
      "message": "Incident not found",
      "locations": [{ "line": 2, "column": 3 }],
      "path": ["incident"]
    }
  ]
}
```

### Common Error Types

1. **Validation Errors** - Invalid input
2. **Not Found** - Resource doesn't exist
3. **Permission Denied** - Unauthorized access
4. **Internal Error** - Server error

## Client Integration

### JavaScript/TypeScript

```typescript
import { GraphQLClient } from 'graphql-request';

const client = new GraphQLClient('http://localhost:8080/graphql');

// Query
const data = await client.request(`
  query GetIncidents {
    incidents(pagination: { page: 0, pageSize: 10 }) {
      incidents {
        id
        title
        severity
      }
    }
  }
`);

// Mutation
const result = await client.request(`
  mutation CreateIncident($input: CreateIncidentInput!) {
    createIncident(input: $input) {
      id
      createdAt
    }
  }
`, {
  input: {
    source: "api-client",
    title: "Test Incident",
    description: "Testing GraphQL",
    severity: "P3",
    incidentType: "Infrastructure"
  }
});

// Subscription (using graphql-ws)
import { createClient } from 'graphql-ws';

const wsClient = createClient({
  url: 'ws://localhost:8080/graphql/ws',
});

wsClient.subscribe({
  query: `
    subscription {
      criticalIncidents {
        id
        title
      }
    }
  `,
}, {
  next: (data) => console.log('New incident:', data),
  error: (err) => console.error('Error:', err),
  complete: () => console.log('Complete'),
});
```

### Python

```python
from gql import gql, Client
from gql.transport.requests import RequestsHTTPTransport

# HTTP transport
transport = RequestsHTTPTransport(
    url='http://localhost:8080/graphql',
    headers={'Content-Type': 'application/json'}
)

client = Client(transport=transport, fetch_schema_from_transport=True)

# Query
query = gql("""
    query GetIncidents {
        incidents(pagination: { page: 0, pageSize: 10 }) {
            incidents {
                id
                title
                severity
            }
        }
    }
""")

result = client.execute(query)
print(result)

# Mutation
mutation = gql("""
    mutation CreateIncident($input: CreateIncidentInput!) {
        createIncident(input: $input) {
            id
            createdAt
        }
    }
""")

params = {
    "input": {
        "source": "python-client",
        "title": "Test Incident",
        "description": "Testing GraphQL from Python",
        "severity": "P3",
        "incidentType": "Infrastructure"
    }
}

result = client.execute(mutation, variable_values=params)
print(result)
```

### Rust

```rust
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "query.graphql",
    response_derives = "Debug"
)]
pub struct GetIncidents;

async fn query_incidents() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let variables = get_incidents::Variables {
        pagination: Some(get_incidents::PaginationInput {
            page: 0,
            page_size: 10,
        }),
    };

    let request_body = GetIncidents::build_query(variables);

    let res = client
        .post("http://localhost:8080/graphql")
        .json(&request_body)
        .send()
        .await?;

    let response_body: graphql_client::Response<get_incidents::ResponseData> =
        res.json().await?;

    println!("{:#?}", response_body.data);

    Ok(())
}
```

## Monitoring and Metrics

### Prometheus Metrics

All GraphQL operations are tracked:

```promql
# Query rate
rate(llm_incident_manager_graphql_queries_total[5m])

# Query duration p95
histogram_quantile(0.95,
  rate(llm_incident_manager_graphql_query_duration_seconds_bucket[5m])
)

# Error rate
rate(llm_incident_manager_graphql_errors_total[5m])

# Active subscriptions
llm_incident_manager_graphql_subscriptions_active
```

### Logging

GraphQL operations are logged with:
- Operation name
- Duration
- Error details
- User/actor

Example log:
```
INFO GraphQL query execution completed successfully operation=GetIncidents duration_ms=45 has_data=true
```

## Best Practices

### 1. Use Specific Queries

Create specific queries for specific use cases:

```graphql
# Good
query DashboardData {
  activeIncidents(pagination: { pageSize: 5 }) {
    incidents {
      id
      title
      severity
      createdAt
    }
  }

  incidentStats {
    active
    bySeverity { p0 p1 }
  }
}

# Bad - too generic
query Everything {
  incidents {
    incidents {
      # All fields, even if not needed
    }
  }
}
```

### 2. Leverage Fragments

```graphql
fragment IncidentCard on Incident {
  id
  title
  severity
  state
  createdAt
  affectedResources
}

query Dashboard {
  critical: criticalIncidents(pagination: { pageSize: 5 }) {
    incidents { ...IncidentCard }
  }

  recent: incidents(pagination: { pageSize: 10 }) {
    incidents { ...IncidentCard }
  }
}
```

### 3. Use Pagination

Always paginate list queries to avoid performance issues:

```graphql
{
  incidents(pagination: { page: 0, pageSize: 20 }) {
    incidents { id title }
    pageInfo {
      hasNextPage
      totalCount
    }
  }
}
```

### 4. Handle Errors

Check for both data and errors in responses:

```typescript
const response = await client.request(query);

if (response.errors) {
  console.error('GraphQL errors:', response.errors);
  // Handle errors
}

if (response.data) {
  // Process data
}
```

### 5. Use Subscriptions Wisely

Subscriptions maintain persistent connections. Use them for:
- Real-time dashboards
- Notification systems
- Live monitoring

Don't use them for:
- One-time data fetching (use queries)
- Rare updates (poll instead)

## Troubleshooting

### Query Too Complex

**Error:** "Query complexity exceeds maximum (100)"

**Solution:** Break query into multiple queries or reduce nesting.

### Query Too Deep

**Error:** "Query depth exceeds maximum (10)"

**Solution:** Reduce nesting levels or refactor data model.

### Slow Queries

**Check:**
1. Use DataLoaders (automatic)
2. Add database indexes
3. Reduce field selection
4. Add pagination

### Subscription Not Receiving Updates

**Check:**
1. WebSocket connection established
2. Filters match the events
3. Events are being published (check logs)

## Schema Evolution

### Adding Fields

Safe - clients ignore unknown fields:

```graphql
type Incident {
  id: UUID!
  title: String!
  newField: String  # Safe to add
}
```

### Deprecating Fields

Use `@deprecated`:

```graphql
type Incident {
  id: UUID!
  oldField: String @deprecated(reason: "Use newField instead")
  newField: String
}
```

### Breaking Changes

Avoid:
- Removing fields
- Changing field types
- Making nullable fields required

Instead:
- Add new fields
- Deprecate old fields
- Version the API if necessary

## Resources

- [GraphQL Specification](https://spec.graphql.org/)
- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)

## Support

For issues or questions:
1. Check the GraphQL Playground documentation (built-in)
2. Review error messages in responses
3. Check Prometheus metrics
4. Review application logs
