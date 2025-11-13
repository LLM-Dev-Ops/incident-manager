# GraphQL Query Examples

## Common Query Patterns and Real-World Use Cases

This document provides practical examples of GraphQL queries, mutations, and subscriptions for common incident management scenarios.

## Table of Contents

- [Basic Queries](#basic-queries)
- [Complex Nested Queries](#complex-nested-queries)
- [Pagination Examples](#pagination-examples)
- [Filtering Examples](#filtering-examples)
- [Mutation Examples](#mutation-examples)
- [Subscription Examples](#subscription-examples)
- [Analytics Queries](#analytics-queries)
- [Real-World Use Cases](#real-world-use-cases)

## Basic Queries

### Get Single Incident

```graphql
query GetIncident {
  incident(id: "inc_abc123") {
    id
    title
    severity
    status
    description
    createdAt
    updatedAt
    source
  }
}
```

### List Recent Incidents

```graphql
query RecentIncidents {
  incidents(first: 10, orderBy: { field: CREATED_AT, direction: DESC }) {
    edges {
      node {
        id
        title
        severity
        status
        createdAt
      }
    }
    totalCount
  }
}
```

### Get Critical Incidents

```graphql
query CriticalIncidents {
  incidents(
    first: 20
    filter: { severity: [P0, P1], status: [NEW, ACKNOWLEDGED, IN_PROGRESS] }
  ) {
    edges {
      node {
        id
        title
        severity
        status
        createdAt
        sla {
          resolutionDeadline
          resolutionBreached
        }
      }
    }
    totalCount
  }
}
```

### Get User Information

```graphql
query GetUser {
  user(id: "user_123") {
    id
    name
    email
    role
    teams {
      id
      name
    }
    notificationPreferences {
      channels
      quietHours {
        start
        end
      }
    }
  }
}
```

## Complex Nested Queries

### Incident with Full Context

```graphql
query IncidentFullContext {
  incident(id: "inc_abc123") {
    id
    title
    description
    severity
    status
    category
    environment

    # Timestamps
    createdAt
    updatedAt
    acknowledgedAt
    resolvedAt

    # Assignment
    assignedTo {
      id
      name
      email
      teams {
        id
        name
      }
    }
    assignedTeam {
      id
      name
      lead {
        id
        name
      }
      escalationPolicy {
        id
        name
        levels {
          level
          name
          escalateAfterSecs
        }
      }
    }

    # Resource
    resource {
      type
      id
      name
      metadata
    }

    # Metrics
    metrics {
      mttd
      mtta
      mttr
      escalationCount
    }

    # SLA
    sla {
      acknowledgmentDeadline
      resolutionDeadline
      acknowledgmentBreached
      resolutionBreached
    }

    # Related incidents
    relatedIncidents {
      id
      title
      severity
      status
    }

    # Enrichment
    enrichment {
      historical {
        similarIncidents {
          incident {
            id
            title
            resolution {
              rootCause
              resolutionNotes
            }
          }
          similarityScore
        }
        patterns
        suggestedSolutions
      }
      service {
        serviceName
        owner
        oncallTeam
      }
    }

    # Correlation
    correlationGroup {
      id
      correlationScore
      correlationType
      primaryIncident {
        id
        title
      }
    }

    # Resolution
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

    # Notifications
    notifications(first: 5) {
      edges {
        node {
          id
          channel
          recipient
          status
          sentAt
        }
      }
    }
  }
}
```

### Team Dashboard Query

```graphql
query TeamDashboard {
  team(id: "team_platform") {
    id
    name
    description

    # Members
    members {
      user {
        id
        name
        email
        status
      }
      role
    }

    # Lead
    lead {
      id
      name
      email
    }

    # Active incidents
    assignedIncidents(
      first: 20
      filter: { status: [NEW, ACKNOWLEDGED, IN_PROGRESS, ESCALATED] }
    ) {
      edges {
        node {
          id
          title
          severity
          status
          createdAt
          sla {
            resolutionDeadline
            resolutionTimeRemaining
          }
        }
      }
      totalCount
    }

    # Escalation policy
    escalationPolicy {
      id
      name
      levels {
        level
        name
        escalateAfterSecs
        targets {
          type
          identifier
        }
      }
    }

    # Metrics for last 7 days
    metrics(
      timeRange: {
        start: "2025-11-05T00:00:00Z"
        end: "2025-11-12T23:59:59Z"
      }
    ) {
      totalIncidentsHandled
      averageResponseTime
      averageResolutionTime
      incidentsBySeverity {
        severity
        count
      }
    }
  }
}
```

## Pagination Examples

### First Page

```graphql
query FirstPage {
  incidents(first: 20) {
    edges {
      cursor
      node {
        id
        title
        severity
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

**Response:**
```json
{
  "data": {
    "incidents": {
      "edges": [
        {
          "cursor": "Y3Vyc29yOjA=",
          "node": { "id": "inc_001", "title": "High CPU", "severity": "P1" }
        }
        // ... 19 more items
      ],
      "pageInfo": {
        "hasNextPage": true,
        "endCursor": "Y3Vyc29yOjE5"
      }
    }
  }
}
```

### Next Page

```graphql
query NextPage {
  incidents(first: 20, after: "Y3Vyc29yOjE5") {
    edges {
      cursor
      node {
        id
        title
        severity
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

### Load More Pattern

```graphql
query LoadMore($after: String) {
  incidents(first: 20, after: $after) {
    edges {
      cursor
      node {
        id
        title
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

**Variables (first call):**
```json
{}
```

**Variables (subsequent calls):**
```json
{
  "after": "Y3Vyc29yOjE5"
}
```

## Filtering Examples

### Filter by Severity and Status

```graphql
query FilteredIncidents {
  incidents(
    first: 20
    filter: {
      severity: [P0, P1]
      status: [NEW, ACKNOWLEDGED]
    }
  ) {
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

### Filter by Date Range

```graphql
query IncidentsLastWeek {
  incidents(
    first: 50
    filter: {
      dateRange: {
        start: "2025-11-05T00:00:00Z"
        end: "2025-11-12T23:59:59Z"
      }
    }
  ) {
    edges {
      node {
        id
        title
        createdAt
      }
    }
    totalCount
  }
}
```

### Filter by Tags

```graphql
query IncidentsByTags {
  incidents(
    first: 20
    filter: {
      tags: {
        environment: "production"
        team: "platform"
        region: "us-east-1"
      }
    }
  ) {
    edges {
      node {
        id
        title
        tags
      }
    }
  }
}
```

### Full-Text Search

```graphql
query SearchIncidents {
  searchIncidents(
    query: "database connection timeout"
    first: 20
    filter: {
      environment: [PRODUCTION]
      category: [PERFORMANCE, AVAILABILITY]
    }
  ) {
    edges {
      node {
        id
        title
        description
        category
      }
    }
    totalCount
  }
}
```

### Multiple Filters with Sorting

```graphql
query ComplexFilter {
  incidents(
    first: 20
    filter: {
      severity: [P0, P1]
      status: [NEW, ACKNOWLEDGED, IN_PROGRESS]
      environment: [PRODUCTION]
      category: [PERFORMANCE, AVAILABILITY]
      dateRange: {
        start: "2025-11-01T00:00:00Z"
        end: "2025-11-12T23:59:59Z"
      }
    }
    orderBy: [
      { field: SEVERITY, direction: ASC }
      { field: CREATED_AT, direction: DESC }
    ]
  ) {
    edges {
      node {
        id
        title
        severity
        status
        createdAt
      }
    }
  }
}
```

## Mutation Examples

### Create Incident

```graphql
mutation CreateIncident {
  createIncident(
    input: {
      event: {
        eventId: "evt_abc123"
        source: "llm-sentinel"
        title: "High CPU Usage on API Server"
        description: "CPU usage exceeded 90% on prod-api-01 for 5 minutes"
        severity: "P1"
        category: PERFORMANCE
        resource: {
          type: "service"
          id: "prod-api-01"
          name: "Production API Server"
          metadata: { region: "us-east-1", cluster: "prod-cluster-01" }
        }
        metrics: { cpu_usage: 92.5, memory_usage: 78.3 }
        tags: {
          environment: "production"
          team: "platform"
          service: "api-server"
        }
      }
      options: { skipDeduplication: false }
    }
  ) {
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

### Acknowledge Multiple Incidents

```graphql
mutation AcknowledgeMultiple {
  ack1: acknowledgeIncident(
    incidentId: "inc_001"
    actor: "user@example.com"
    notes: "Investigating CPU issue"
  ) {
    success
    incident {
      id
      status
      acknowledgedAt
    }
  }

  ack2: acknowledgeIncident(
    incidentId: "inc_002"
    actor: "user@example.com"
    notes: "Checking database connections"
  ) {
    success
    incident {
      id
      status
      acknowledgedAt
    }
  }

  ack3: acknowledgeIncident(
    incidentId: "inc_003"
    actor: "user@example.com"
    notes: "Reviewing logs"
  ) {
    success
    incident {
      id
      status
      acknowledgedAt
    }
  }
}
```

### Resolve Incident with Details

```graphql
mutation ResolveIncident {
  resolveIncident(
    input: {
      incidentId: "inc_abc123"
      resolvedBy: "user@example.com"
      method: MANUAL
      rootCause: "Memory leak in background job processor"
      notes: "Restarted service and applied hotfix"
      actions: [
        {
          type: DIAGNOSTIC
          description: "Analyzed heap dump"
          executedBy: "user@example.com"
          result: "Found memory leak in job queue"
        }
        {
          type: REMEDIATION
          description: "Restarted service"
          executedBy: "user@example.com"
          result: "Service back to normal"
        }
        {
          type: REMEDIATION
          description: "Deployed hotfix v1.2.3"
          executedBy: "user@example.com"
          result: "Memory leak fixed"
        }
      ]
      playbookUsed: "playbook_restart_service"
    }
  ) {
    success
    incident {
      id
      status
      resolvedAt
      resolution {
        rootCause
        resolutionNotes
        actionsTaken {
          type
          description
        }
      }
    }
  }
}
```

### Execute Playbook

```graphql
mutation ExecutePlaybook {
  executePlaybook(incidentId: "inc_abc123", playbookId: "playbook_001") {
    success
    execution {
      id
      status
      startedAt
      stepsExecuted {
        stepId
        status
        startedAt
        completedAt
        result
      }
    }
  }
}
```

### Escalate Incident

```graphql
mutation EscalateIncident {
  escalateIncident(
    incidentId: "inc_abc123"
    reason: "No response after 15 minutes"
    level: 2
  ) {
    success
    incident {
      id
      escalationLevel
      status
    }
    escalationState {
      currentLevel
      escalatedAt
    }
  }
}
```

### Assign Incident to Team

```graphql
mutation AssignToTeam {
  assignIncident(
    incidentId: "inc_abc123"
    teamId: "team_platform"
    actor: "oncall-scheduler"
  ) {
    success
    incident {
      id
      assignedTeam {
        id
        name
      }
    }
  }
}
```

## Subscription Examples

### Subscribe to All Incident Updates

```graphql
subscription IncidentUpdates {
  incidentUpdated {
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
      name
    }
    timestamp
  }
}
```

### Subscribe to Critical Incidents Only

```graphql
subscription CriticalIncidents {
  incidentCreated(severity: [P0, P1], environment: [PRODUCTION]) {
    id
    title
    severity
    status
    category
    description
    resource {
      type
      name
    }
    createdAt
  }
}
```

### Subscribe to Team Escalations

```graphql
subscription TeamEscalations {
  incidentEscalated(teamId: "team_platform") {
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

### Subscribe to Incident Resolutions

```graphql
subscription IncidentResolutions {
  incidentResolved(teamId: "team_platform") {
    incident {
      id
      title
      severity
    }
    resolvedAt
    resolvedBy
    resolutionTime
  }
}
```

## Analytics Queries

### System-Wide Analytics

```graphql
query SystemAnalytics {
  analytics(
    timeRange: {
      start: "2025-11-01T00:00:00Z"
      end: "2025-11-12T23:59:59Z"
    }
  ) {
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

    topAffectedServices {
      serviceName
      incidentCount
    }

    topRootCauses {
      rootCause
      count
    }
  }
}
```

### Team Performance Metrics

```graphql
query TeamPerformance {
  team(id: "team_platform") {
    id
    name

    metrics(
      timeRange: {
        start: "2025-11-01T00:00:00Z"
        end: "2025-11-12T23:59:59Z"
      }
    ) {
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

### User Performance

```graphql
query UserPerformance {
  user(id: "user_123") {
    id
    name

    metrics(
      timeRange: {
        start: "2025-11-01T00:00:00Z"
        end: "2025-11-12T23:59:59Z"
      }
    ) {
      incidentsHandled
      averageResolutionTime
      onCallHours
      incidentsByOutcome {
        outcome
        count
      }
    }

    assignedIncidents(
      first: 10
      filter: { status: [IN_PROGRESS, ESCALATED] }
    ) {
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
}
```

## Real-World Use Cases

### 1. Incident Dashboard

```graphql
query IncidentDashboard {
  # Active critical incidents
  critical: incidents(
    first: 10
    filter: {
      severity: [P0, P1]
      status: [NEW, ACKNOWLEDGED, IN_PROGRESS]
    }
  ) {
    edges {
      node {
        id
        title
        severity
        status
        createdAt
        sla {
          resolutionDeadline
          resolutionTimeRemaining
        }
        assignedTo {
          name
        }
      }
    }
    totalCount
  }

  # Incidents by status
  newIncidents: incidents(first: 1, filter: { status: [NEW] }) {
    totalCount
  }

  acknowledged: incidents(first: 1, filter: { status: [ACKNOWLEDGED] }) {
    totalCount
  }

  inProgress: incidents(first: 1, filter: { status: [IN_PROGRESS] }) {
    totalCount
  }

  # Recent resolutions
  recentResolutions: incidents(
    first: 5
    filter: { status: [RESOLVED] }
    orderBy: { field: UPDATED_AT, direction: DESC }
  ) {
    edges {
      node {
        id
        title
        severity
        resolvedAt
        metrics {
          mttr
        }
      }
    }
  }

  # SLA breaches
  slaBreaches: incidents(
    first: 10
    filter: {
      status: [NEW, ACKNOWLEDGED, IN_PROGRESS]
    }
  ) {
    edges {
      node {
        id
        title
        severity
        sla {
          resolutionBreached
          resolutionTimeRemaining
        }
      }
    }
  }
}
```

### 2. On-Call Dashboard

```graphql
query OnCallDashboard($userId: ID!) {
  user(id: $userId) {
    id
    name

    # Current assignments
    assignedIncidents(
      first: 20
      filter: { status: [NEW, ACKNOWLEDGED, IN_PROGRESS, ESCALATED] }
      orderBy: { field: SEVERITY, direction: ASC }
    ) {
      edges {
        node {
          id
          title
          severity
          status
          category
          createdAt
          sla {
            resolutionDeadline
            resolutionTimeRemaining
          }
          relatedIncidents {
            id
            title
          }
        }
      }
      totalCount
    }

    # Teams
    teams {
      id
      name
      escalationPolicy {
        name
        levels {
          level
          name
        }
      }
    }
  }
}
```

### 3. Post-Mortem Data Collection

```graphql
query PostMortemData($incidentId: ID!) {
  incident(id: $incidentId) {
    id
    title
    description
    severity
    category

    # Timeline
    createdAt
    acknowledgedAt
    resolvedAt
    closedAt

    # Metrics
    metrics {
      mttd
      mtta
      mttr
      totalDuration
    }

    # Resolution
    resolution {
      rootCause
      resolutionNotes
      resolvedBy
      actionsTaken {
        type
        description
        executedBy
        executedAt
        result
      }
    }

    # Related incidents
    relatedIncidents {
      id
      title
      severity
      createdAt
      resolvedAt
    }

    # Activity log
    resolutionLogs(first: 100) {
      edges {
        node {
          eventType
          timestamp
          actor {
            name
          }
          notes
          changes {
            field
            oldValue
            newValue
          }
        }
      }
    }

    # Notifications sent
    notifications(first: 50) {
      edges {
        node {
          channel
          recipient
          sentAt
          deliveredAt
        }
      }
    }
  }
}
```

### 4. Correlation Analysis

```graphql
query CorrelationAnalysis {
  correlationGroups(first: 20, minIncidents: 2) {
    edges {
      node {
        id
        correlationScore
        correlationType
        createdAt

        primaryIncident {
          id
          title
          severity
        }

        incidents {
          id
          title
          severity
          status
          createdAt
        }

        commonAttributes
        suggestedRootCause
      }
    }
  }
}
```

### 5. Service Health Overview

```graphql
query ServiceHealth($serviceName: String!) {
  incidents(
    first: 50
    filter: {
      tags: { service: $serviceName }
      dateRange: {
        start: "2025-11-05T00:00:00Z"
        end: "2025-11-12T23:59:59Z"
      }
    }
  ) {
    edges {
      node {
        id
        title
        severity
        status
        category
        createdAt
        resolvedAt
        metrics {
          mttr
        }
      }
    }
    totalCount
  }
}
```

## Advanced Patterns

### Conditional Fields with Directives

```graphql
query IncidentWithOptionalDetails($includeEnrichment: Boolean = false) {
  incident(id: "inc_abc123") {
    id
    title
    severity

    enrichment @include(if: $includeEnrichment) {
      historical {
        similarIncidents {
          incident {
            id
            title
          }
        }
      }
    }
  }
}
```

### Fragment Reuse

```graphql
fragment IncidentSummary on Incident {
  id
  title
  severity
  status
  createdAt
}

fragment IncidentDetails on Incident {
  ...IncidentSummary
  description
  category
  environment
  assignedTo {
    name
  }
}

query DashboardWithFragments {
  critical: incidents(first: 5, filter: { severity: [P0] }) {
    edges {
      node {
        ...IncidentDetails
      }
    }
  }

  recent: incidents(first: 10) {
    edges {
      node {
        ...IncidentSummary
      }
    }
  }
}
```

## Further Reading

- [GraphQL API Guide](./GRAPHQL_API_GUIDE.md) - Complete API documentation
- [GraphQL Schema Reference](./GRAPHQL_SCHEMA_REFERENCE.md) - Full schema reference
- [GraphQL Integration Guide](./GRAPHQL_INTEGRATION_GUIDE.md) - Client integration
- [GraphQL Development Guide](./GRAPHQL_DEVELOPMENT_GUIDE.md) - Development guide
