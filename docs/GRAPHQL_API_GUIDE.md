# GraphQL API Guide

## Overview

The LLM Incident Manager GraphQL API provides a flexible, type-safe, and efficient way to interact with the incident management system. Built with modern GraphQL best practices, it offers real-time subscriptions, comprehensive querying capabilities, and powerful mutations for managing incidents, escalations, and integrations.

## Table of Contents

- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Authentication](#authentication)
- [Schema Overview](#schema-overview)
- [Queries](#queries)
- [Mutations](#mutations)
- [Subscriptions](#subscriptions)
- [Pagination](#pagination)
- [Filtering and Sorting](#filtering-and-sorting)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [Query Complexity](#query-complexity)
- [Best Practices](#best-practices)

## Architecture

### Technology Stack

- **Framework**: async-graphql 7.0+
- **Runtime**: Tokio async runtime
- **Web Server**: Axum integration
- **Transport**: HTTP POST + WebSocket for subscriptions
- **Schema Language**: GraphQL SDL with introspection

### System Integration

```
┌─────────────────────────────────────────────────────────────┐
│                       Client Applications                    │
│  (Apollo Client, Relay, urql, GraphQL Playground)           │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    GraphQL API Layer                         │
│  ┌───────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │   Queries     │  │  Mutations   │  │  Subscriptions  │  │
│  └───────┬───────┘  └──────┬───────┘  └────────┬────────┘  │
│          │                  │                    │           │
│          └──────────────────┼────────────────────┘           │
│                             ▼                                │
│                   ┌──────────────────┐                       │
│                   │   DataLoaders    │                       │
│                   │  (N+1 Prevention)│                       │
│                   └─────────┬────────┘                       │
└──────────────────────────────┼──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    Business Logic Layer                      │
│  ┌────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │ Incident   │  │ Escalation   │  │   Enrichment       │  │
│  │ Processor  │  │   Engine     │  │    Service         │  │
│  └────────────┘  └──────────────┘  └────────────────────┘  │
│  ┌────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │Correlation │  │ ML Service   │  │  Notification      │  │
│  │  Engine    │  │              │  │    Service         │  │
│  └────────────┘  └──────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Endpoints

| Endpoint | Purpose | Protocol |
|----------|---------|----------|
| `/graphql` | GraphQL queries and mutations | HTTP POST |
| `/graphql` | GraphQL subscriptions | WebSocket |
| `/graphql/playground` | GraphQL Playground IDE | HTTP GET |
| `/graphql/schema` | Schema introspection | HTTP GET |

## Quick Start

### Using cURL

```bash
# Simple query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "query": "{ incidents(first: 10) { edges { node { id title severity status } } } }"
  }'
```

### Using GraphQL Playground

1. Navigate to `http://localhost:8080/graphql/playground`
2. Set authorization header in the bottom-left panel
3. Explore schema docs in the right panel
4. Write and execute queries in the main editor

### Basic Query Example

```graphql
query GetRecentIncidents {
  incidents(
    first: 10
    orderBy: { field: CREATED_AT, direction: DESC }
  ) {
    edges {
      node {
        id
        title
        severity
        status
        createdAt
        source
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

## Authentication

### API Key Authentication

Include your API key in the Authorization header:

```
Authorization: Bearer YOUR_API_KEY
```

### JWT Token Authentication

For user-specific operations:

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### WebSocket Authentication

For subscriptions, pass the token during connection initialization:

```javascript
const wsClient = new WebSocket('ws://localhost:8080/graphql', {
  connectionParams: {
    authorization: 'Bearer YOUR_API_KEY'
  }
});
```

## Schema Overview

### Core Types

The GraphQL schema mirrors the TypeScript data models:

- **Incident**: Core incident entity
- **Alert**: Raw alert/event data
- **EscalationPolicy**: Escalation configuration
- **EnrichmentContext**: Additional context data
- **CorrelationGroup**: Related incident groups
- **User**: User accounts
- **Team**: Team information
- **Notification**: Notification records
- **Playbook**: Automation playbooks

### Enums

- `Severity`: P0, P1, P2, P3, P4
- `IncidentStatus`: NEW, ACKNOWLEDGED, IN_PROGRESS, ESCALATED, RESOLVED, CLOSED
- `Category`: PERFORMANCE, SECURITY, AVAILABILITY, COMPLIANCE, COST, OTHER
- `Environment`: PRODUCTION, STAGING, DEVELOPMENT, QA
- `NotificationChannel`: EMAIL, SLACK, TEAMS, PAGERDUTY, OPSGENIE, SMS, WEBHOOK

## Queries

### Incident Queries

#### Get Incident by ID

```graphql
query GetIncident($id: ID!) {
  incident(id: $id) {
    id
    title
    description
    severity
    status
    category
    environment
    createdAt
    updatedAt
    resolvedAt
    source
    assignedTo {
      id
      name
      email
    }
    assignedTeam {
      id
      name
    }
    tags
    metrics {
      mttd
      mtta
      mttr
    }
    relatedIncidents {
      id
      title
      severity
    }
  }
}
```

#### List Incidents with Filters

```graphql
query ListIncidents(
  $first: Int = 20
  $after: String
  $severity: [Severity!]
  $status: [IncidentStatus!]
  $category: [Category!]
  $environment: [Environment!]
  $dateRange: DateRangeInput
) {
  incidents(
    first: $first
    after: $after
    filter: {
      severity: $severity
      status: $status
      category: $category
      environment: $environment
      dateRange: $dateRange
    }
    orderBy: { field: CREATED_AT, direction: DESC }
  ) {
    edges {
      cursor
      node {
        id
        title
        severity
        status
        category
        environment
        createdAt
        source
      }
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

### Analytics Queries

```graphql
query GetIncidentAnalytics($timeRange: TimeRangeInput!) {
  analytics(timeRange: $timeRange) {
    totalIncidents
    incidentsBySeverity {
      severity
      count
    }
    incidentsByCategory {
      category
      count
    }
    incidentsByStatus {
      status
      count
    }
    performance {
      averageMttd
      averageMtta
      averageMttr
      p50Mttr
      p95Mttr
      p99Mttr
    }
    slaMetrics {
      acknowledgmentCompliance
      resolutionCompliance
      totalBreaches
    }
    trends {
      incidentTrend {
        timestamp
        value
      }
      mttrTrend {
        timestamp
        value
      }
    }
  }
}
```

### Team Performance Queries

```graphql
query GetTeamMetrics($teamId: ID!, $timeRange: TimeRangeInput!) {
  team(id: $teamId) {
    id
    name
    metrics(timeRange: $timeRange) {
      totalIncidentsHandled
      incidentsBySeverity {
        severity
        count
      }
      averageResponseTime
      averageResolutionTime
      reopenedIncidents
      escalatedIncidents
      userMetrics {
        user {
          id
          name
        }
        incidentsHandled
        averageResolutionTime
        onCallHours
      }
    }
  }
}
```

## Mutations

### Create Incident

```graphql
mutation CreateIncident($input: CreateIncidentInput!) {
  createIncident(input: $input) {
    incident {
      id
      title
      severity
      status
      createdAt
    }
    status
    message
    duplicateOf
  }
}
```

**Input Variables:**

```json
{
  "input": {
    "event": {
      "eventId": "ext-12345",
      "source": "monitoring",
      "title": "High CPU Usage Detected",
      "description": "CPU usage exceeded 90% threshold on prod-api-01",
      "severity": "P1",
      "category": "PERFORMANCE",
      "resource": {
        "type": "service",
        "id": "prod-api-01",
        "name": "Production API Server",
        "metadata": {
          "region": "us-east-1",
          "cluster": "prod-cluster-01"
        }
      },
      "metrics": {
        "cpu_usage": 92.5,
        "memory_usage": 78.3
      },
      "tags": {
        "environment": "production",
        "team": "platform"
      }
    },
    "options": {
      "skipDeduplication": false,
      "assignTo": "user-123"
    }
  }
}
```

### Update Incident

```graphql
mutation UpdateIncident($input: UpdateIncidentInput!) {
  updateIncident(input: $input) {
    incident {
      id
      status
      severity
      assignedTo {
        id
        name
      }
      updatedAt
    }
    status
    message
  }
}
```

### Acknowledge Incident

```graphql
mutation AcknowledgeIncident($incidentId: ID!, $actor: String!, $notes: String) {
  acknowledgeIncident(incidentId: $incidentId, actor: $actor, notes: $notes) {
    incident {
      id
      status
      acknowledgedAt
    }
    success
    message
  }
}
```

### Resolve Incident

```graphql
mutation ResolveIncident($input: ResolveIncidentInput!) {
  resolveIncident(input: $input) {
    incident {
      id
      status
      resolvedAt
      resolution {
        rootCause
        resolutionNotes
        resolvedBy
        playbookUsed
        actionsTaken {
          id
          type
          description
          executedBy
          executedAt
        }
      }
    }
    success
    message
  }
}
```

### Execute Playbook

```graphql
mutation ExecutePlaybook($incidentId: ID!, $playbookId: ID!) {
  executePlaybook(incidentId: $incidentId, playbookId: $playbookId) {
    execution {
      id
      playbookId
      incidentId
      status
      startedAt
      stepsExecuted {
        stepId
        status
        startedAt
        completedAt
        result
        error
      }
    }
    success
    message
  }
}
```

### Escalate Incident

```graphql
mutation EscalateIncident($incidentId: ID!, $reason: String!, $level: Int) {
  escalateIncident(incidentId: $incidentId, reason: $reason, level: $level) {
    incident {
      id
      escalationLevel
      status
    }
    escalationState {
      incidentId
      currentLevel
      escalatedAt
      reason
    }
    success
    message
  }
}
```

## Subscriptions

### Real-time Incident Updates

```graphql
subscription IncidentUpdates($filter: IncidentFilterInput) {
  incidentUpdated(filter: $filter) {
    incident {
      id
      title
      severity
      status
      updatedAt
    }
    updateType
    changedFields
    actor {
      type
      id
      name
    }
  }
}
```

### New Incidents Stream

```graphql
subscription NewIncidents($severity: [Severity!], $environment: [Environment!]) {
  incidentCreated(severity: $severity, environment: $environment) {
    id
    title
    severity
    status
    category
    environment
    source
    createdAt
  }
}
```

### Escalation Notifications

```graphql
subscription EscalationNotifications($teamId: ID, $userId: ID) {
  incidentEscalated(teamId: $teamId, userId: $userId) {
    incident {
      id
      title
      severity
      escalationLevel
    }
    escalationLevel {
      level
      name
      targets {
        type
        identifier
      }
    }
    escalatedAt
  }
}
```

### Correlation Updates

```graphql
subscription CorrelationUpdates {
  correlationGroupUpdated {
    groupId
    incidentIds
    correlationScore
    correlationType
    updatedAt
  }
}
```

## Pagination

### Cursor-based Pagination

The API uses Relay-style cursor-based pagination for scalability:

```graphql
query PaginatedIncidents($first: Int!, $after: String) {
  incidents(first: $first, after: $after) {
    edges {
      cursor
      node {
        id
        title
      }
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

**First Page:**
```json
{
  "first": 20
}
```

**Next Page:**
```json
{
  "first": 20,
  "after": "Y3Vyc29yOjIw"
}
```

### Backward Pagination

```graphql
query PreviousPage($last: Int!, $before: String!) {
  incidents(last: $last, before: $before) {
    edges {
      cursor
      node {
        id
        title
      }
    }
    pageInfo {
      hasPreviousPage
      startCursor
    }
  }
}
```

## Filtering and Sorting

### Complex Filters

```graphql
query ComplexFilter($filter: IncidentFilterInput!) {
  incidents(filter: $filter) {
    edges {
      node {
        id
        title
      }
    }
  }
}
```

**Filter Variables:**
```json
{
  "filter": {
    "severity": ["P0", "P1"],
    "status": ["NEW", "ACKNOWLEDGED"],
    "category": ["PERFORMANCE", "AVAILABILITY"],
    "environment": ["PRODUCTION"],
    "dateRange": {
      "start": "2025-11-01T00:00:00Z",
      "end": "2025-11-12T23:59:59Z"
    },
    "tags": {
      "team": "platform",
      "region": "us-east-1"
    },
    "search": "high cpu"
  }
}
```

### Multiple Sort Options

```graphql
query SortedIncidents($orderBy: [IncidentOrderByInput!]) {
  incidents(orderBy: $orderBy) {
    edges {
      node {
        id
        title
        severity
        createdAt
      }
    }
  }
}
```

**Sort Variables:**
```json
{
  "orderBy": [
    { "field": "SEVERITY", "direction": "ASC" },
    { "field": "CREATED_AT", "direction": "DESC" }
  ]
}
```

## Error Handling

### Error Response Format

```json
{
  "errors": [
    {
      "message": "Incident not found",
      "locations": [{ "line": 2, "column": 3 }],
      "path": ["incident"],
      "extensions": {
        "code": "NOT_FOUND",
        "incidentId": "inc_123456",
        "timestamp": "2025-11-12T10:30:00Z"
      }
    }
  ],
  "data": {
    "incident": null
  }
}
```

### Error Codes

| Code | Description | Retry |
|------|-------------|-------|
| `UNAUTHENTICATED` | Missing or invalid authentication | No |
| `UNAUTHORIZED` | Insufficient permissions | No |
| `NOT_FOUND` | Resource not found | No |
| `VALIDATION_ERROR` | Invalid input data | No |
| `CONFLICT` | Resource conflict (e.g., duplicate) | No |
| `RATE_LIMITED` | Too many requests | Yes |
| `INTERNAL_ERROR` | Server error | Yes |
| `SERVICE_UNAVAILABLE` | Dependency unavailable | Yes |

### Partial Errors

GraphQL supports partial responses with errors:

```json
{
  "errors": [
    {
      "message": "Failed to enrich incident",
      "path": ["incident", "enrichment"],
      "extensions": {
        "code": "SERVICE_UNAVAILABLE",
        "service": "enrichment-service"
      }
    }
  ],
  "data": {
    "incident": {
      "id": "inc_123",
      "title": "High CPU",
      "enrichment": null
    }
  }
}
```

## Rate Limiting

### Rate Limit Headers

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1699804800
```

### Rate Limit Tiers

| Tier | Requests/Hour | Concurrent Connections |
|------|---------------|------------------------|
| Free | 1,000 | 5 |
| Pro | 10,000 | 20 |
| Enterprise | 100,000 | 100 |

### Rate Limit Error

```json
{
  "errors": [
    {
      "message": "Rate limit exceeded",
      "extensions": {
        "code": "RATE_LIMITED",
        "limit": 1000,
        "reset": 1699804800,
        "retryAfter": 600
      }
    }
  ]
}
```

## Query Complexity

### Complexity Calculation

Each field has a complexity cost:
- Simple scalar field: 1
- Object field: 1 + nested complexity
- Connection field: multiplier * (first/last) + nested complexity
- Mutations: 10 base cost

### Complexity Limits

| Tier | Max Complexity |
|------|----------------|
| Free | 1,000 |
| Pro | 10,000 |
| Enterprise | 100,000 |

### Complexity Error

```json
{
  "errors": [
    {
      "message": "Query complexity exceeds maximum",
      "extensions": {
        "code": "COMPLEXITY_LIMITED",
        "complexity": 15000,
        "maxComplexity": 10000
      }
    }
  ]
}
```

### Optimizing Complexity

**High Complexity (Bad):**
```graphql
query {
  incidents(first: 100) {
    edges {
      node {
        relatedIncidents {
          relatedIncidents {
            relatedIncidents {
              id
            }
          }
        }
      }
    }
  }
}
```

**Optimized (Good):**
```graphql
query {
  incidents(first: 20) {
    edges {
      node {
        id
        relatedIncidents {
          id
          title
        }
      }
    }
  }
}
```

## Best Practices

### 1. Use Fragments for Reusability

```graphql
fragment IncidentFields on Incident {
  id
  title
  severity
  status
  createdAt
}

query GetIncidents {
  recent: incidents(first: 10, filter: { status: [NEW] }) {
    edges {
      node {
        ...IncidentFields
      }
    }
  }
  critical: incidents(first: 10, filter: { severity: [P0, P1] }) {
    edges {
      node {
        ...IncidentFields
      }
    }
  }
}
```

### 2. Request Only Needed Fields

**Bad:**
```graphql
query {
  incidents(first: 10) {
    edges {
      node {
        id
        title
        description
        enrichment {
          historical {
            similarIncidents {
              id
              title
              description
              enrichment {
                # ...too deep
              }
            }
          }
        }
      }
    }
  }
}
```

**Good:**
```graphql
query {
  incidents(first: 10) {
    edges {
      node {
        id
        title
        severity
        status
      }
    }
  }
}
```

### 3. Use Variables for Dynamic Queries

**Bad:**
```graphql
query {
  incident(id: "inc_123") { id }
}
```

**Good:**
```graphql
query GetIncident($id: ID!) {
  incident(id: $id) { id }
}
```

### 4. Leverage DataLoader for N+1 Prevention

The API automatically batches related queries:

```graphql
query {
  incidents(first: 10) {
    edges {
      node {
        id
        assignedTo {  # Batched loading
          id
          name
        }
        assignedTeam {  # Batched loading
          id
          name
        }
      }
    }
  }
}
```

### 5. Use Subscriptions for Real-time Data

Instead of polling:

```graphql
# Bad: Polling every 5 seconds
setInterval(() => {
  query { incidents { edges { node { id status } } } }
}, 5000)

# Good: Subscription
subscription {
  incidentUpdated {
    incident { id status }
  }
}
```

### 6. Handle Errors Gracefully

```javascript
const result = await client.query({ query: GET_INCIDENT });

if (result.errors) {
  result.errors.forEach(error => {
    console.error(`Error: ${error.message}`);
    if (error.extensions.code === 'RATE_LIMITED') {
      // Handle rate limiting
      setTimeout(retry, error.extensions.retryAfter * 1000);
    }
  });
}

if (result.data) {
  // Handle partial success
  console.log(result.data);
}
```

### 7. Use Persisted Queries (Production)

Pre-register queries for better performance and security:

```javascript
const QUERY_ID = "abc123...";

fetch('/graphql', {
  method: 'POST',
  body: JSON.stringify({
    id: QUERY_ID,
    variables: { incidentId: "inc_123" }
  })
});
```

## Advanced Features

### Field-Level Authorization

Some fields require specific permissions:

```graphql
query {
  incident(id: "inc_123") {
    id
    title
    sensitiveData  # Requires 'admin' role
  }
}
```

### Batch Mutations

Execute multiple mutations in a single request:

```graphql
mutation BatchUpdate {
  ack1: acknowledgeIncident(incidentId: "inc_1", actor: "user1") {
    success
  }
  ack2: acknowledgeIncident(incidentId: "inc_2", actor: "user1") {
    success
  }
  ack3: acknowledgeIncident(incidentId: "inc_3", actor: "user1") {
    success
  }
}
```

### Query Directives

```graphql
query GetIncident($id: ID!, $includeEnrichment: Boolean = false) {
  incident(id: $id) {
    id
    title
    enrichment @include(if: $includeEnrichment) {
      historical {
        similarIncidents {
          id
          title
        }
      }
    }
  }
}
```

## Performance Tips

1. **Use pagination** - Always paginate large result sets
2. **Limit depth** - Avoid deeply nested queries
3. **Cache responses** - Use HTTP caching and Apollo cache
4. **Batch requests** - Combine multiple queries when possible
5. **Monitor complexity** - Track query complexity in production
6. **Use CDN** - Cache static schema responses
7. **Connection pooling** - Reuse WebSocket connections for subscriptions

## Support

- **Documentation**: [GRAPHQL_SCHEMA_REFERENCE.md](./GRAPHQL_SCHEMA_REFERENCE.md)
- **Integration Guide**: [GRAPHQL_INTEGRATION_GUIDE.md](./GRAPHQL_INTEGRATION_GUIDE.md)
- **Development Guide**: [GRAPHQL_DEVELOPMENT_GUIDE.md](./GRAPHQL_DEVELOPMENT_GUIDE.md)
- **Examples**: [GRAPHQL_EXAMPLES.md](./GRAPHQL_EXAMPLES.md)
- **API Support**: support@example.com
- **Community**: https://github.com/globalbusinessadvisors/llm-incident-manager/discussions
