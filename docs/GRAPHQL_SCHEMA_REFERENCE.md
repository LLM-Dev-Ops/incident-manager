# GraphQL Schema Reference

## Complete Schema Documentation

This document provides comprehensive documentation for all types, queries, mutations, and subscriptions in the LLM Incident Manager GraphQL API.

## Table of Contents

- [Root Types](#root-types)
- [Object Types](#object-types)
- [Input Types](#input-types)
- [Enums](#enums)
- [Scalars](#scalars)
- [Interfaces](#interfaces)
- [Unions](#unions)

## Root Types

### Query

Root query type for all read operations.

```graphql
type Query {
  """
  Get a single incident by ID
  """
  incident(id: ID!): Incident

  """
  List incidents with filtering, sorting, and pagination
  """
  incidents(
    first: Int
    after: String
    last: Int
    before: String
    filter: IncidentFilterInput
    orderBy: [IncidentOrderByInput!]
  ): IncidentConnection!

  """
  Get incident analytics for a time range
  """
  analytics(timeRange: TimeRangeInput!): IncidentAnalytics!

  """
  Search incidents with full-text search
  """
  searchIncidents(
    query: String!
    first: Int
    after: String
    filter: IncidentFilterInput
  ): IncidentConnection!

  """
  Get correlation groups
  """
  correlationGroups(
    first: Int
    after: String
    minIncidents: Int
  ): CorrelationGroupConnection!

  """
  Get a single user by ID
  """
  user(id: ID!): User

  """
  List all users
  """
  users(first: Int, after: String): UserConnection!

  """
  Get a single team by ID
  """
  team(id: ID!): Team

  """
  List all teams
  """
  teams(first: Int, after: String): TeamConnection!

  """
  Get team metrics for a time range
  """
  teamMetrics(teamId: ID!, timeRange: TimeRangeInput!): TeamMetrics!

  """
  Get escalation policy by ID
  """
  escalationPolicy(id: ID!): EscalationPolicy

  """
  List all escalation policies
  """
  escalationPolicies(first: Int, after: String): EscalationPolicyConnection!

  """
  Get playbook by ID
  """
  playbook(id: ID!): Playbook

  """
  List all playbooks
  """
  playbooks(first: Int, after: String, enabled: Boolean): PlaybookConnection!

  """
  Get notification template by ID
  """
  notificationTemplate(id: ID!): NotificationTemplate

  """
  List notification templates
  """
  notificationTemplates(first: Int, after: String): NotificationTemplateConnection!

  """
  Get system health status
  """
  health: HealthStatus!

  """
  Get system configuration
  """
  systemConfig: SystemConfig!
}
```

### Mutation

Root mutation type for all write operations.

```graphql
type Mutation {
  """
  Create a new incident from an alert/event
  """
  createIncident(input: CreateIncidentInput!): CreateIncidentResponse!

  """
  Update an existing incident
  """
  updateIncident(input: UpdateIncidentInput!): UpdateIncidentResponse!

  """
  Acknowledge an incident
  """
  acknowledgeIncident(
    incidentId: ID!
    actor: String!
    notes: String
  ): AcknowledgeIncidentResponse!

  """
  Resolve an incident
  """
  resolveIncident(input: ResolveIncidentInput!): ResolveIncidentResponse!

  """
  Close an incident
  """
  closeIncident(
    incidentId: ID!
    actor: String!
    notes: String
  ): CloseIncidentResponse!

  """
  Reopen a closed incident
  """
  reopenIncident(
    incidentId: ID!
    actor: String!
    reason: String!
  ): ReopenIncidentResponse!

  """
  Escalate an incident
  """
  escalateIncident(
    incidentId: ID!
    reason: String!
    level: Int
  ): EscalateIncidentResponse!

  """
  Assign an incident to a user or team
  """
  assignIncident(
    incidentId: ID!
    userId: ID
    teamId: ID
    actor: String!
  ): AssignIncidentResponse!

  """
  Add a comment to an incident
  """
  addComment(
    incidentId: ID!
    comment: String!
    actor: String!
  ): AddCommentResponse!

  """
  Execute a playbook for an incident
  """
  executePlaybook(
    incidentId: ID!
    playbookId: ID!
    dryRun: Boolean
  ): ExecutePlaybookResponse!

  """
  Cancel a playbook execution
  """
  cancelPlaybookExecution(executionId: ID!): CancelPlaybookResponse!

  """
  Merge duplicate incidents
  """
  mergeIncidents(
    sourceId: ID!
    targetId: ID!
    actor: String!
  ): MergeIncidentsResponse!

  """
  Create or update an escalation policy
  """
  upsertEscalationPolicy(input: EscalationPolicyInput!): EscalationPolicy!

  """
  Delete an escalation policy
  """
  deleteEscalationPolicy(id: ID!): DeleteResponse!

  """
  Create or update a playbook
  """
  upsertPlaybook(input: PlaybookInput!): Playbook!

  """
  Delete a playbook
  """
  deletePlaybook(id: ID!): DeleteResponse!

  """
  Create or update a notification template
  """
  upsertNotificationTemplate(input: NotificationTemplateInput!): NotificationTemplate!

  """
  Delete a notification template
  """
  deleteNotificationTemplate(id: ID!): DeleteResponse!

  """
  Provide feedback for ML classification
  """
  provideClassificationFeedback(
    incidentId: ID!
    correctSeverity: Severity!
    notes: String
  ): FeedbackResponse!
}
```

### Subscription

Root subscription type for real-time updates.

```graphql
type Subscription {
  """
  Subscribe to incident updates
  """
  incidentUpdated(filter: IncidentFilterInput): IncidentUpdateEvent!

  """
  Subscribe to new incident creation
  """
  incidentCreated(
    severity: [Severity!]
    environment: [Environment!]
    category: [Category!]
  ): Incident!

  """
  Subscribe to incident escalations
  """
  incidentEscalated(teamId: ID, userId: ID): EscalationEvent!

  """
  Subscribe to incident resolutions
  """
  incidentResolved(teamId: ID): ResolutionEvent!

  """
  Subscribe to correlation group updates
  """
  correlationGroupUpdated: CorrelationGroupEvent!

  """
  Subscribe to playbook execution updates
  """
  playbookExecutionUpdated(
    incidentId: ID
    playbookId: ID
  ): PlaybookExecutionEvent!

  """
  Subscribe to notification events
  """
  notificationSent(incidentId: ID): NotificationEvent!

  """
  Subscribe to system health updates
  """
  healthUpdated: HealthStatus!
}
```

## Object Types

### Incident

Core incident object representing a managed incident.

```graphql
"""
An incident represents a problem or event that requires attention and resolution.
"""
type Incident {
  """Unique incident identifier"""
  id: ID!

  """External ID from source system"""
  externalId: String

  """Fingerprint for deduplication"""
  fingerprint: String!

  """Incident severity (P0-P4)"""
  severity: Severity!

  """Current incident status"""
  status: IncidentStatus!

  """Incident category"""
  category: Category!

  """Short title describing the incident"""
  title: String!

  """Detailed description"""
  description: String!

  """Impact description"""
  impact: String

  """Source system that generated the incident"""
  source: String!

  """Source event ID"""
  sourceEventId: String!

  """User assigned to the incident"""
  assignedTo: User

  """Team assigned to the incident"""
  assignedTeam: Team

  """Current escalation level (0 = not escalated)"""
  escalationLevel: Int!

  """Timestamp when incident was created"""
  createdAt: DateTime!

  """Timestamp when incident was last updated"""
  updatedAt: DateTime!

  """Timestamp when incident was acknowledged"""
  acknowledgedAt: DateTime

  """Timestamp when incident was resolved"""
  resolvedAt: DateTime

  """Timestamp when incident was closed"""
  closedAt: DateTime

  """SLA tracking information"""
  sla: SLATracking!

  """Related incidents"""
  relatedIncidents: [Incident!]!

  """Parent incident (if this is a sub-incident)"""
  parentIncident: Incident

  """Incident this is a duplicate of"""
  duplicateOf: Incident

  """Performance metrics"""
  metrics: IncidentMetrics!

  """Affected resource"""
  resource: ResourceInfo!

  """Environment where incident occurred"""
  environment: Environment!

  """Key-value tags"""
  tags: JSON!

  """Label array"""
  labels: [String!]!

  """Resolution information"""
  resolution: ResolutionInfo

  """Additional metadata"""
  metadata: JSON

  """Enrichment context"""
  enrichment: EnrichmentContext

  """Correlation group this incident belongs to"""
  correlationGroup: CorrelationGroup

  """Notification history"""
  notifications(first: Int, after: String): NotificationConnection!

  """Resolution log entries"""
  resolutionLogs(first: Int, after: String): ResolutionLogConnection!

  """Comments on this incident"""
  comments(first: Int, after: String): CommentConnection!

  """Playbook executions for this incident"""
  playbookExecutions: [PlaybookExecution!]!

  """Post-mortem document (if available)"""
  postMortem: PostMortem
}
```

### SLATracking

```graphql
"""
Service Level Agreement tracking for an incident.
"""
type SLATracking {
  """Deadline for acknowledging the incident"""
  acknowledgmentDeadline: DateTime!

  """Deadline for resolving the incident"""
  resolutionDeadline: DateTime!

  """Whether acknowledgment SLA was breached"""
  acknowledgmentBreached: Boolean!

  """Whether resolution SLA was breached"""
  resolutionBreached: Boolean!

  """Time remaining until acknowledgment deadline (seconds)"""
  acknowledgmentTimeRemaining: Int

  """Time remaining until resolution deadline (seconds)"""
  resolutionTimeRemaining: Int
}
```

### IncidentMetrics

```graphql
"""
Performance metrics for an incident.
"""
type IncidentMetrics {
  """Mean Time To Detect (seconds)"""
  mttd: Float!

  """Mean Time To Acknowledge (seconds)"""
  mtta: Float!

  """Mean Time To Resolve (seconds)"""
  mttr: Float!

  """Total time incident was open (seconds)"""
  totalDuration: Float

  """Number of escalations"""
  escalationCount: Int!

  """Number of notifications sent"""
  notificationCount: Int!

  """Number of related incidents"""
  correlationCount: Int!
}
```

### ResourceInfo

```graphql
"""
Information about the affected resource.
"""
type ResourceInfo {
  """Resource type (service, endpoint, model, etc.)"""
  type: String!

  """Resource identifier"""
  id: String!

  """Resource name"""
  name: String!

  """Additional resource metadata"""
  metadata: JSON
}
```

### ResolutionInfo

```graphql
"""
Information about incident resolution.
"""
type ResolutionInfo {
  """Root cause analysis"""
  rootCause: String

  """Resolution notes"""
  resolutionNotes: String

  """User who resolved the incident"""
  resolvedBy: String

  """Playbook used for resolution"""
  playbookUsed: String

  """Actions taken to resolve"""
  actionsTaken: [Action!]!
}
```

### Action

```graphql
"""
An action taken during incident resolution.
"""
type Action {
  """Action identifier"""
  id: ID!

  """Action type"""
  type: ActionType!

  """Action description"""
  description: String!

  """Who executed the action"""
  executedBy: String!

  """When the action was executed"""
  executedAt: DateTime!

  """Action result"""
  result: String!

  """Additional metadata"""
  metadata: JSON
}
```

### EnrichmentContext

```graphql
"""
Enrichment context added to an incident.
"""
type EnrichmentContext {
  """Historical incident data"""
  historical: HistoricalEnrichment

  """Service catalog information"""
  service: ServiceEnrichment

  """Team information"""
  team: TeamEnrichment

  """External API data"""
  external: ExternalEnrichment

  """When enrichment was performed"""
  enrichedAt: DateTime!

  """Enrichment performance metrics"""
  metrics: EnrichmentMetrics
}
```

### HistoricalEnrichment

```graphql
"""
Historical incident analysis enrichment.
"""
type HistoricalEnrichment {
  """Similar past incidents"""
  similarIncidents: [SimilarIncident!]!

  """Common patterns found"""
  patterns: [String!]!

  """Suggested solutions based on history"""
  suggestedSolutions: [String!]!

  """Average resolution time for similar incidents"""
  avgResolutionTime: Float
}
```

### SimilarIncident

```graphql
"""
A similar past incident.
"""
type SimilarIncident {
  """The similar incident"""
  incident: Incident!

  """Similarity score (0.0 - 1.0)"""
  similarityScore: Float!

  """Common attributes"""
  commonAttributes: [String!]!
}
```

### CorrelationGroup

```graphql
"""
A group of correlated incidents.
"""
type CorrelationGroup {
  """Group identifier"""
  id: ID!

  """Incidents in this group"""
  incidents: [Incident!]!

  """Correlation score"""
  correlationScore: Float!

  """Type of correlation"""
  correlationType: CorrelationType!

  """When the group was created"""
  createdAt: DateTime!

  """When the group was last updated"""
  updatedAt: DateTime!

  """Common attributes across incidents"""
  commonAttributes: JSON!

  """Suggested root cause"""
  suggestedRootCause: String

  """Primary incident (highest severity)"""
  primaryIncident: Incident!
}
```

### User

```graphql
"""
A user in the system.
"""
type User {
  """User identifier"""
  id: ID!

  """Email address"""
  email: String!

  """Full name"""
  name: String!

  """User role"""
  role: UserRole!

  """Teams the user belongs to"""
  teams: [Team!]!

  """Phone number"""
  phone: String

  """Slack user ID"""
  slackId: String

  """PagerDuty user ID"""
  pagerdutyId: String

  """Notification preferences"""
  notificationPreferences: NotificationPreferences!

  """User status"""
  status: UserStatus!

  """User timezone"""
  timezone: String!

  """When user was created"""
  createdAt: DateTime!

  """When user was last updated"""
  updatedAt: DateTime!

  """Incidents assigned to user"""
  assignedIncidents(
    first: Int
    after: String
    filter: IncidentFilterInput
  ): IncidentConnection!

  """User performance metrics"""
  metrics(timeRange: TimeRangeInput!): UserMetrics!
}
```

### Team

```graphql
"""
A team in the system.
"""
type Team {
  """Team identifier"""
  id: ID!

  """Team name"""
  name: String!

  """Team description"""
  description: String

  """Team members"""
  members: [TeamMember!]!

  """Team lead"""
  lead: User

  """Escalation policy for this team"""
  escalationPolicy: EscalationPolicy

  """On-call schedule"""
  onCallSchedule: OnCallSchedule

  """When team was created"""
  createdAt: DateTime!

  """When team was last updated"""
  updatedAt: DateTime!

  """Incidents assigned to team"""
  assignedIncidents(
    first: Int
    after: String
    filter: IncidentFilterInput
  ): IncidentConnection!

  """Team performance metrics"""
  metrics(timeRange: TimeRangeInput!): TeamMetrics!
}
```

### EscalationPolicy

```graphql
"""
An escalation policy configuration.
"""
type EscalationPolicy {
  """Policy identifier"""
  id: ID!

  """Policy name"""
  name: String!

  """Policy description"""
  description: String

  """Escalation levels"""
  levels: [EscalationLevel!]!

  """Conditions for policy activation"""
  conditions: EscalationConditions

  """Timer configuration"""
  timers: EscalationTimers!

  """Schedule for policy activation"""
  schedule: EscalationSchedule

  """Whether policy is enabled"""
  enabled: Boolean!

  """Policy priority"""
  priority: Int!

  """When policy was created"""
  createdAt: DateTime!

  """When policy was last updated"""
  updatedAt: DateTime!

  """Who created the policy"""
  createdBy: String!
}
```

### Playbook

```graphql
"""
An automated playbook for incident response.
"""
type Playbook {
  """Playbook identifier"""
  id: ID!

  """Playbook name"""
  name: String!

  """Playbook description"""
  description: String

  """Trigger conditions"""
  triggers: [PlaybookTrigger!]!

  """Playbook steps"""
  steps: [PlaybookStep!]!

  """Execution configuration"""
  executionConfig: PlaybookExecutionConfig!

  """Whether playbook is enabled"""
  enabled: Boolean!

  """When playbook was created"""
  createdAt: DateTime!

  """When playbook was last updated"""
  updatedAt: DateTime!

  """Who created the playbook"""
  createdBy: String!

  """Execution history"""
  executions(first: Int, after: String): PlaybookExecutionConnection!
}
```

### PlaybookExecution

```graphql
"""
A playbook execution instance.
"""
type PlaybookExecution {
  """Execution identifier"""
  id: ID!

  """Playbook being executed"""
  playbook: Playbook!

  """Incident this execution is for"""
  incident: Incident!

  """Execution status"""
  status: ExecutionStatus!

  """When execution started"""
  startedAt: DateTime!

  """When execution completed"""
  completedAt: DateTime

  """Steps executed"""
  stepsExecuted: [StepExecution!]!

  """Error message if failed"""
  error: String

  """Who triggered the execution"""
  triggeredBy: Actor!

  """Execution metadata"""
  metadata: JSON
}
```

### IncidentAnalytics

```graphql
"""
Aggregated incident analytics.
"""
type IncidentAnalytics {
  """Time range for analytics"""
  timeRange: TimeRange!

  """Total number of incidents"""
  totalIncidents: Int!

  """Incidents grouped by severity"""
  incidentsBySeverity: [SeverityCount!]!

  """Incidents grouped by category"""
  incidentsByCategory: [CategoryCount!]!

  """Incidents grouped by status"""
  incidentsByStatus: [StatusCount!]!

  """Performance metrics"""
  performance: PerformanceMetrics!

  """SLA compliance metrics"""
  slaMetrics: SLAMetrics!

  """Trend data"""
  trends: TrendData!

  """Top affected services"""
  topAffectedServices: [ServiceIncidentCount!]!

  """Most common root causes"""
  topRootCauses: [RootCauseCount!]!
}
```

### HealthStatus

```graphql
"""
System health status.
"""
type HealthStatus {
  """Overall system status"""
  status: SystemStatus!

  """Individual component statuses"""
  components: [ComponentStatus!]!

  """System version"""
  version: String!

  """System uptime (seconds)"""
  uptime: Float!

  """Last health check time"""
  timestamp: DateTime!

  """Active incidents count"""
  activeIncidents: Int!

  """Processing throughput (events/sec)"""
  throughput: Float!

  """Average processing latency (ms)"""
  latency: Float!
}
```

## Input Types

### CreateIncidentInput

```graphql
"""
Input for creating a new incident.
"""
input CreateIncidentInput {
  """Raw event data"""
  event: RawEventInput!

  """Creation options"""
  options: IncidentCreationOptions
}
```

### RawEventInput

```graphql
"""
Raw event/alert data.
"""
input RawEventInput {
  """Event identifier"""
  eventId: String!

  """Source system"""
  source: String!

  """Event title"""
  title: String!

  """Event description"""
  description: String!

  """Event severity"""
  severity: String!

  """Event category"""
  category: Category!

  """Affected resource"""
  resource: ResourceInfoInput!

  """Event metrics"""
  metrics: JSON

  """Event tags"""
  tags: JSON

  """Event labels"""
  labels: [String!]

  """Additional payload"""
  payload: JSON
}
```

### IncidentFilterInput

```graphql
"""
Filter criteria for incidents.
"""
input IncidentFilterInput {
  """Filter by severity"""
  severity: [Severity!]

  """Filter by status"""
  status: [IncidentStatus!]

  """Filter by category"""
  category: [Category!]

  """Filter by environment"""
  environment: [Environment!]

  """Filter by date range"""
  dateRange: DateRangeInput

  """Filter by tags"""
  tags: JSON

  """Full-text search"""
  search: String

  """Filter by assigned user"""
  assignedTo: ID

  """Filter by assigned team"""
  assignedTeam: ID

  """Filter by source"""
  source: String

  """Filter by escalation level"""
  escalationLevel: Int
}
```

### IncidentOrderByInput

```graphql
"""
Sort order for incidents.
"""
input IncidentOrderByInput {
  """Field to sort by"""
  field: IncidentSortField!

  """Sort direction"""
  direction: SortDirection!
}
```

### UpdateIncidentInput

```graphql
"""
Input for updating an incident.
"""
input UpdateIncidentInput {
  """Incident ID"""
  incidentId: ID!

  """New severity"""
  severity: Severity

  """New status"""
  status: IncidentStatus

  """New title"""
  title: String

  """New description"""
  description: String

  """New impact"""
  impact: String

  """Assign to user"""
  assignedTo: ID

  """Assign to team"""
  assignedTeam: ID

  """Update tags"""
  tags: JSON

  """Update labels"""
  labels: [String!]

  """Update metadata"""
  metadata: JSON

  """Actor making the change"""
  actor: String!

  """Update notes"""
  notes: String
}
```

### ResolveIncidentInput

```graphql
"""
Input for resolving an incident.
"""
input ResolveIncidentInput {
  """Incident ID"""
  incidentId: ID!

  """Who resolved the incident"""
  resolvedBy: String!

  """Resolution method"""
  method: ResolutionMethod!

  """Root cause"""
  rootCause: String

  """Resolution notes"""
  notes: String

  """Actions taken"""
  actions: [ActionInput!]

  """Playbook used"""
  playbookUsed: ID
}
```

### TimeRangeInput

```graphql
"""
Time range specification.
"""
input TimeRangeInput {
  """Start timestamp"""
  start: DateTime!

  """End timestamp"""
  end: DateTime!
}
```

## Enums

### Severity

```graphql
"""
Incident severity levels.
"""
enum Severity {
  """Critical - System down, major impact"""
  P0

  """High - Significant impact, immediate attention required"""
  P1

  """Medium - Moderate impact, attention needed soon"""
  P2

  """Low - Minor impact, can be scheduled"""
  P3

  """Informational - Minimal or no impact"""
  P4
}
```

### IncidentStatus

```graphql
"""
Incident lifecycle status.
"""
enum IncidentStatus {
  """Newly created incident"""
  NEW

  """Incident has been acknowledged"""
  ACKNOWLEDGED

  """Work in progress"""
  IN_PROGRESS

  """Incident has been escalated"""
  ESCALATED

  """Incident has been resolved"""
  RESOLVED

  """Incident is closed"""
  CLOSED
}
```

### Category

```graphql
"""
Incident categories.
"""
enum Category {
  """Performance-related issues"""
  PERFORMANCE

  """Security incidents"""
  SECURITY

  """Availability problems"""
  AVAILABILITY

  """Compliance violations"""
  COMPLIANCE

  """Cost-related issues"""
  COST

  """Other uncategorized incidents"""
  OTHER
}
```

### Environment

```graphql
"""
Deployment environments.
"""
enum Environment {
  """Production environment"""
  PRODUCTION

  """Staging environment"""
  STAGING

  """Development environment"""
  DEVELOPMENT

  """QA/Testing environment"""
  QA
}
```

### NotificationChannel

```graphql
"""
Available notification channels.
"""
enum NotificationChannel {
  EMAIL
  SLACK
  TEAMS
  PAGERDUTY
  OPSGENIE
  SMS
  WEBHOOK
}
```

### UserRole

```graphql
"""
User roles in the system.
"""
enum UserRole {
  """System administrator"""
  ADMIN

  """Incident responder"""
  RESPONDER

  """Read-only viewer"""
  VIEWER
}
```

### CorrelationType

```graphql
"""
Types of incident correlation.
"""
enum CorrelationType {
  """Correlated by source system"""
  SOURCE

  """Correlated by incident type"""
  TYPE

  """Correlated by similarity"""
  SIMILARITY

  """Correlated by tags"""
  TAGS

  """Correlated by affected service"""
  SERVICE
}
```

### ActionType

```graphql
"""
Types of actions taken on incidents.
"""
enum ActionType {
  """Diagnostic action"""
  DIAGNOSTIC

  """Remediation action"""
  REMEDIATION

  """Notification sent"""
  NOTIFICATION

  """Escalation performed"""
  ESCALATION

  """Integration action"""
  INTEGRATION
}
```

### ExecutionStatus

```graphql
"""
Playbook execution status.
"""
enum ExecutionStatus {
  """Execution in progress"""
  RUNNING

  """Execution completed successfully"""
  COMPLETED

  """Execution failed"""
  FAILED

  """Execution cancelled"""
  CANCELLED
}
```

### SystemStatus

```graphql
"""
Overall system health status.
"""
enum SystemStatus {
  """All systems operational"""
  HEALTHY

  """Some degradation"""
  DEGRADED

  """System unavailable"""
  UNHEALTHY
}
```

### IncidentSortField

```graphql
"""
Fields available for sorting incidents.
"""
enum IncidentSortField {
  CREATED_AT
  UPDATED_AT
  SEVERITY
  STATUS
  ESCALATION_LEVEL
  MTTA
  MTTR
}
```

### SortDirection

```graphql
"""
Sort direction.
"""
enum SortDirection {
  """Ascending order"""
  ASC

  """Descending order"""
  DESC
}
```

## Scalars

### DateTime

```graphql
"""
ISO 8601 date-time string (e.g., "2025-11-12T10:30:00Z").
"""
scalar DateTime
```

### JSON

```graphql
"""
Arbitrary JSON data.
"""
scalar JSON
```

## Interfaces

### Node

```graphql
"""
Base interface for all entities with an ID.
"""
interface Node {
  """Unique identifier"""
  id: ID!
}
```

### Timestamped

```graphql
"""
Interface for entities with timestamps.
"""
interface Timestamped {
  """Creation timestamp"""
  createdAt: DateTime!

  """Last update timestamp"""
  updatedAt: DateTime!
}
```

## Unions

### SearchResult

```graphql
"""
Unified search result type.
"""
union SearchResult = Incident | User | Team | Playbook
```

## Connection Types (Relay Pagination)

### IncidentConnection

```graphql
type IncidentConnection {
  edges: [IncidentEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type IncidentEdge {
  cursor: String!
  node: Incident!
}
```

### PageInfo

```graphql
"""
Pagination information.
"""
type PageInfo {
  """Whether there are more items"""
  hasNextPage: Boolean!

  """Whether there are previous items"""
  hasPreviousPage: Boolean!

  """Cursor to the first item"""
  startCursor: String

  """Cursor to the last item"""
  endCursor: String
}
```

## Response Types

### CreateIncidentResponse

```graphql
type CreateIncidentResponse {
  """Created incident"""
  incident: Incident!

  """Creation status"""
  status: CreateStatus!

  """Status message"""
  message: String!

  """If duplicate, the incident it's a duplicate of"""
  duplicateOf: ID
}

enum CreateStatus {
  CREATED
  DUPLICATE
}
```

### UpdateIncidentResponse

```graphql
type UpdateIncidentResponse {
  """Updated incident"""
  incident: Incident!

  """Success status"""
  status: ResponseStatus!

  """Response message"""
  message: String!
}
```

### DeleteResponse

```graphql
type DeleteResponse {
  """Whether deletion was successful"""
  success: Boolean!

  """Response message"""
  message: String!
}
```

### FeedbackResponse

```graphql
type FeedbackResponse {
  """Whether feedback was accepted"""
  accepted: Boolean!

  """Response message"""
  message: String!
}
```

## Event Types (Subscriptions)

### IncidentUpdateEvent

```graphql
type IncidentUpdateEvent {
  """Updated incident"""
  incident: Incident!

  """Type of update"""
  updateType: UpdateType!

  """Fields that changed"""
  changedFields: [String!]!

  """Actor who made the change"""
  actor: Actor!

  """When the update occurred"""
  timestamp: DateTime!
}

enum UpdateType {
  CREATED
  UPDATED
  STATUS_CHANGED
  ASSIGNED
  ESCALATED
  RESOLVED
  CLOSED
}
```

### EscalationEvent

```graphql
type EscalationEvent {
  """Escalated incident"""
  incident: Incident!

  """New escalation level"""
  escalationLevel: EscalationLevel!

  """When escalation occurred"""
  escalatedAt: DateTime!

  """Reason for escalation"""
  reason: String
}
```

### NotificationEvent

```graphql
type NotificationEvent {
  """Notification record"""
  notification: Notification!

  """Related incident"""
  incident: Incident!

  """When notification was sent"""
  sentAt: DateTime!
}
```

## Full SDL Schema

For the complete schema in SDL format suitable for code generation tools:

```bash
# Download complete schema
curl http://localhost:8080/graphql/schema > schema.graphql

# Or use introspection query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ __schema { types { name description fields { name description type { name kind ofType { name kind } } } } } }"}'
```

## Schema Visualization

Generate a visual schema diagram:

```bash
# Using GraphQL Voyager
npx graphql-voyager --from-url http://localhost:8080/graphql

# Using graphdoc
npx graphdoc --schema-url http://localhost:8080/graphql --output ./docs/schema
```

## Further Reading

- [GraphQL API Guide](./GRAPHQL_API_GUIDE.md) - Usage guide
- [GraphQL Integration Guide](./GRAPHQL_INTEGRATION_GUIDE.md) - Client integration
- [GraphQL Development Guide](./GRAPHQL_DEVELOPMENT_GUIDE.md) - Development guide
- [GraphQL Examples](./GRAPHQL_EXAMPLES.md) - Query examples
