/**
 * LLM-Incident-Manager Data Models
 * Comprehensive TypeScript type definitions for the incident management system
 */

// ============================================================================
// CORE TYPES
// ============================================================================

export type ISO8601Timestamp = string;
export type UUID = string;

export type Severity = 'P0' | 'P1' | 'P2' | 'P3' | 'P4';

export type IncidentStatus =
  | 'NEW'
  | 'ACKNOWLEDGED'
  | 'IN_PROGRESS'
  | 'ESCALATED'
  | 'RESOLVED'
  | 'CLOSED';

export type Category =
  | 'performance'
  | 'security'
  | 'availability'
  | 'compliance'
  | 'cost'
  | 'other';

export type Environment = 'production' | 'staging' | 'development' | 'qa';

export type Channel =
  | 'email'
  | 'slack'
  | 'teams'
  | 'pagerduty'
  | 'opsgenie'
  | 'sms'
  | 'webhook';

// ============================================================================
// EVENT MODELS
// ============================================================================

/**
 * Raw event from source systems (Sentinel, Shield, Edge-Agent)
 */
export interface RawEvent {
  // Event identification
  event_id: string;
  source: 'llm-sentinel' | 'llm-shield' | 'llm-edge-agent' | 'llm-governance-core' | string;
  source_version: string;

  // Event metadata
  timestamp: ISO8601Timestamp;
  received_at?: ISO8601Timestamp;

  // Event classification
  event_type: 'anomaly' | 'violation' | 'alert' | 'error' | 'warning' | string;
  category: Category;

  // Event data
  title: string;
  description: string;
  severity: string;

  // Context
  resource: ResourceInfo;

  // Metrics & details
  metrics: Record<string, number>;
  tags: Record<string, string>;
  labels?: string[];

  // Correlation
  correlation_id?: string;
  parent_event_id?: string;

  // Custom payload
  payload: Record<string, any>;
}

/**
 * Normalized incident event after validation and enrichment
 */
export interface IncidentEvent extends RawEvent {
  // Incident management fields
  incident_id?: string;
  fingerprint: string;
  normalized_severity: Severity;

  // Enrichment
  enrichment: EventEnrichment;

  // Validation
  schema_version: string;
  validated_at: ISO8601Timestamp;
}

export interface EventEnrichment {
  geo_location?: string;
  environment: Environment;
  tenant_id?: string;
  application_id?: string;
  cluster_id?: string;
  additional: Record<string, any>;
}

export interface ResourceInfo {
  type: 'service' | 'endpoint' | 'model' | 'deployment' | 'tenant' | string;
  id: string;
  name: string;
  metadata: Record<string, any>;
}

// ============================================================================
// INCIDENT MODELS
// ============================================================================

/**
 * Core incident record
 */
export interface Incident {
  // Identification
  id: string;
  external_id?: string;
  fingerprint: string;

  // Classification
  severity: Severity;
  status: IncidentStatus;
  category: Category;

  // Description
  title: string;
  description: string;
  impact: string;

  // Source
  source: string;
  source_event_id: string;

  // Assignment
  assigned_to?: string;
  assigned_team?: string;
  escalation_level: number;

  // Timestamps
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
  acknowledged_at?: ISO8601Timestamp;
  resolved_at?: ISO8601Timestamp;
  closed_at?: ISO8601Timestamp;

  // SLA tracking
  sla: SLATracking;

  // Related incidents
  related_incidents: string[];
  parent_incident?: string;
  duplicate_of?: string;

  // Metrics
  metrics: IncidentMetrics;

  // Context
  resource: ResourceInfo;
  environment: Environment;
  tags: Record<string, string>;
  labels: string[];

  // Resolution
  resolution: ResolutionInfo;

  // Metadata
  metadata: Record<string, any>;
}

export interface SLATracking {
  acknowledgment_deadline: ISO8601Timestamp;
  resolution_deadline: ISO8601Timestamp;
  acknowledgment_breached: boolean;
  resolution_breached: boolean;
}

export interface IncidentMetrics {
  mttd: number;  // Mean time to detect (seconds)
  mtta: number;  // Mean time to acknowledge (seconds)
  mttr: number;  // Mean time to resolve (seconds)
}

export interface ResolutionInfo {
  root_cause?: string;
  resolution_notes?: string;
  resolved_by?: string;
  playbook_used?: string;
  actions_taken: Action[];
}

export interface Action {
  id: string;
  type: 'diagnostic' | 'remediation' | 'notification' | 'escalation' | 'integration';
  description: string;
  executed_by: string;
  executed_at: ISO8601Timestamp;
  result: string;
  metadata?: Record<string, any>;
}

// ============================================================================
// ESCALATION POLICY MODELS
// ============================================================================

/**
 * Escalation policy configuration
 */
export interface EscalationPolicy {
  id: string;
  name: string;
  description: string;

  // Matching criteria
  conditions: EscalationConditions;

  // Escalation levels
  levels: EscalationLevel[];

  // Timers
  timers: EscalationTimers;

  // Active periods
  schedule?: EscalationSchedule;

  // Metadata
  enabled: boolean;
  priority: number;
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
  created_by: string;
}

export interface EscalationConditions {
  severity?: Severity[];
  category?: Category[];
  source?: string[];
  tags?: Record<string, string>;
  environment?: Environment[];
}

export interface EscalationLevel {
  level: number;
  name: string;

  // Targets
  targets: EscalationTarget[];

  // Timing
  escalate_after: number;

  // Notification
  notification_channels: Channel[];
  notification_template: string;

  // Actions
  actions?: EscalationAction[];
}

export interface EscalationTarget {
  type: 'user' | 'team' | 'on_call' | 'external';
  id: string;
  fallback?: EscalationTarget;
}

export interface EscalationAction {
  type: 'webhook' | 'playbook' | 'integration';
  config: Record<string, any>;
}

export interface EscalationTimers {
  acknowledgment_timeout: number;
  resolution_timeout: number;
  escalation_interval: number;
}

export interface EscalationSchedule {
  timezone: string;
  active_hours?: {
    start: string;  // HH:MM
    end: string;    // HH:MM
  };
  active_days?: number[];  // 0-6 (Sunday-Saturday)
}

// ============================================================================
// NOTIFICATION MODELS
// ============================================================================

/**
 * Notification template configuration
 */
export interface NotificationTemplate {
  id: string;
  name: string;
  description: string;

  // Template type
  type: NotificationType;

  // Channel-specific templates
  channels: Partial<Record<Channel, ChannelTemplate>>;

  // Variables available in templates
  variables: string[];

  // Conditions for template selection
  conditions?: TemplateConditions;

  // Metadata
  enabled: boolean;
  priority: number;
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
}

export type NotificationType =
  | 'incident_created'
  | 'incident_updated'
  | 'escalated'
  | 'resolved'
  | 'custom';

export interface ChannelTemplate {
  // Email
  subject?: string;
  body_html?: string;
  body_text?: string;

  // Slack/Teams
  blocks?: any[];

  // SMS
  message?: string;

  // Webhook
  payload?: Record<string, any>;

  // Common
  attachments?: Attachment[];
}

export interface Attachment {
  type: 'image' | 'file' | 'link';
  url: string;
  name: string;
}

export interface TemplateConditions {
  severity?: Severity[];
  category?: Category[];
  tags?: Record<string, string>;
}

export interface Notification {
  id: string;
  incident_id: string;
  template_id: string;
  channel: Channel;
  recipient: string;
  status: 'pending' | 'sent' | 'failed' | 'delivered';
  sent_at?: ISO8601Timestamp;
  delivered_at?: ISO8601Timestamp;
  error?: string;
  metadata: Record<string, any>;
}

// ============================================================================
// AUDIT & LOGGING MODELS
// ============================================================================

/**
 * Resolution log entry
 */
export interface ResolutionLog {
  id: string;
  incident_id: string;

  // Event info
  event_type: LogEventType;
  event_data: Record<string, any>;

  // Actor
  actor: Actor;

  // Changes
  changes?: FieldChange[];

  // Timestamp
  timestamp: ISO8601Timestamp;

  // Additional context
  notes?: string;
  metadata?: Record<string, any>;
}

export type LogEventType =
  | 'incident_created'
  | 'incident_updated'
  | 'state_changed'
  | 'assigned'
  | 'acknowledged'
  | 'escalated'
  | 'resolved'
  | 'closed'
  | 'reopened'
  | 'comment_added'
  | 'action_executed'
  | 'notification_sent'
  | 'sla_breached';

export interface Actor {
  type: 'user' | 'system' | 'integration';
  id: string;
  name: string;
}

export interface FieldChange {
  field: string;
  old_value: any;
  new_value: any;
}

/**
 * Post-mortem document
 */
export interface PostMortem {
  id: string;
  incident_id: string;

  // Summary
  title: string;
  summary: string;

  // Timeline
  timeline: TimelineEvent[];

  // Analysis
  root_cause: RootCauseAnalysis;

  // Impact
  impact: ImpactAnalysis;

  // Resolution
  resolution: ResolutionAnalysis;

  // Prevention
  action_items: ActionItem[];
  lessons_learned: string[];

  // Metadata
  created_by: string;
  created_at: ISO8601Timestamp;
  reviewed_by?: string[];
  status: 'draft' | 'review' | 'published';
}

export interface TimelineEvent {
  timestamp: ISO8601Timestamp;
  description: string;
  type: 'detection' | 'response' | 'communication' | 'mitigation' | 'resolution';
}

export interface RootCauseAnalysis {
  description: string;
  contributing_factors: string[];
  why_undetected?: string;
}

export interface ImpactAnalysis {
  description: string;
  affected_users?: number;
  affected_services?: string[];
  duration: number;
  estimated_cost?: number;
}

export interface ResolutionAnalysis {
  description: string;
  actions_taken: string[];
  effective_actions: string[];
}

export interface ActionItem {
  id: string;
  description: string;
  owner: string;
  priority: 'high' | 'medium' | 'low';
  due_date?: ISO8601Timestamp;
  status: 'open' | 'in_progress' | 'completed' | 'cancelled';
  tracking_url?: string;
}

// ============================================================================
// ROUTING MODELS
// ============================================================================

/**
 * Routing rule configuration
 */
export interface RoutingRule {
  id: string;
  name: string;
  description: string;

  // Matching conditions
  conditions: RoutingConditions;

  // Route targets
  targets: RouteTarget[];

  // Strategy
  strategy: RoutingStrategy;

  // Metadata
  enabled: boolean;
  priority: number;
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
}

export interface RoutingConditions {
  severity?: Severity[];
  category?: Category[];
  source?: string[];
  tags?: Record<string, string>;
  environment?: Environment[];
  time_of_day?: {
    start: string;
    end: string;
  };
}

export interface RouteTarget {
  type: 'team' | 'user' | 'on_call' | 'external_system';
  id: string;
  weight?: number;  // For load balancing
}

export type RoutingStrategy =
  | 'direct'          // Direct assignment
  | 'on_call'         // Based on on-call schedule
  | 'load_balanced'   // Distribute across teams
  | 'skill_based'     // Route by required skills
  | 'geo_based';      // Route by geographic region

/**
 * On-call schedule
 */
export interface OnCallSchedule {
  id: string;
  team_id: string;
  name: string;
  timezone: string;

  // Rotation
  rotations: OnCallRotation[];

  // Overrides
  overrides?: OnCallOverride[];

  // Metadata
  enabled: boolean;
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
}

export interface OnCallRotation {
  id: string;
  name: string;
  users: string[];
  rotation_type: 'daily' | 'weekly' | 'custom';
  start_date: ISO8601Timestamp;
  rotation_interval_days: number;
}

export interface OnCallOverride {
  id: string;
  user_id: string;
  start_date: ISO8601Timestamp;
  end_date: ISO8601Timestamp;
  reason?: string;
}

// ============================================================================
// INTEGRATION MODELS
// ============================================================================

/**
 * Integration configuration
 */
export interface Integration {
  id: string;
  name: string;
  type: IntegrationType;

  // Connection
  endpoint: string;
  authentication: IntegrationAuth;

  // Behavior
  enabled: boolean;
  retry_policy: RetryPolicy;
  timeout_ms: number;

  // Mapping
  severity_mapping?: Record<string, Severity>;
  field_mapping?: Record<string, string>;

  // Metadata
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
}

export type IntegrationType =
  | 'llm-sentinel'
  | 'llm-shield'
  | 'llm-edge-agent'
  | 'llm-governance-core'
  | 'jira'
  | 'servicenow'
  | 'datadog'
  | 'prometheus'
  | 'custom';

export interface IntegrationAuth {
  type: 'api_key' | 'oauth2' | 'basic' | 'mtls';
  credentials: Record<string, string>;
}

export interface RetryPolicy {
  max_attempts: number;
  backoff: 'linear' | 'exponential';
  base_delay_ms: number;
  max_delay_ms: number;
}

// ============================================================================
// PLAYBOOK MODELS
// ============================================================================

/**
 * Automated playbook for incident response
 */
export interface Playbook {
  id: string;
  name: string;
  description: string;

  // Trigger conditions
  triggers: PlaybookTrigger[];

  // Steps
  steps: PlaybookStep[];

  // Execution settings
  execution: PlaybookExecution;

  // Metadata
  enabled: boolean;
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
  created_by: string;
}

export interface PlaybookTrigger {
  type: 'incident_created' | 'status_changed' | 'escalated' | 'manual';
  conditions?: Record<string, any>;
}

export interface PlaybookStep {
  id: string;
  name: string;
  type: 'action' | 'decision' | 'notification' | 'integration';
  config: Record<string, any>;
  on_success?: string;  // Next step ID
  on_failure?: string;  // Next step ID
  timeout_ms?: number;
}

export interface PlaybookExecution {
  mode: 'automatic' | 'manual_approval' | 'dry_run';
  concurrency: number;
  timeout_ms: number;
  rollback_on_failure: boolean;
}

/**
 * Playbook execution record
 */
export interface PlaybookExecution {
  id: string;
  playbook_id: string;
  incident_id: string;

  // Execution
  status: 'running' | 'completed' | 'failed' | 'cancelled';
  started_at: ISO8601Timestamp;
  completed_at?: ISO8601Timestamp;

  // Results
  steps_executed: StepExecution[];
  error?: string;

  // Metadata
  triggered_by: Actor;
}

export interface StepExecution {
  step_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped';
  started_at: ISO8601Timestamp;
  completed_at?: ISO8601Timestamp;
  result?: any;
  error?: string;
}

// ============================================================================
// USER & TEAM MODELS
// ============================================================================

/**
 * User model
 */
export interface User {
  id: string;
  email: string;
  name: string;
  role: UserRole;
  teams: string[];

  // Contact
  phone?: string;
  slack_id?: string;
  pagerduty_id?: string;

  // Preferences
  notification_preferences: NotificationPreferences;

  // Status
  status: 'active' | 'inactive';
  timezone: string;

  // Metadata
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
}

export type UserRole = 'admin' | 'responder' | 'viewer';

export interface NotificationPreferences {
  channels: Channel[];
  quiet_hours?: {
    start: string;
    end: string;
  };
  severity_filter?: Severity[];
}

/**
 * Team model
 */
export interface Team {
  id: string;
  name: string;
  description: string;

  // Members
  members: TeamMember[];
  lead_id?: string;

  // Configuration
  escalation_policy_id?: string;
  on_call_schedule_id?: string;

  // Metadata
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
}

export interface TeamMember {
  user_id: string;
  role: 'lead' | 'member';
  joined_at: ISO8601Timestamp;
}

// ============================================================================
// METRICS & ANALYTICS MODELS
// ============================================================================

/**
 * Aggregated incident metrics
 */
export interface IncidentAnalytics {
  time_range: {
    start: ISO8601Timestamp;
    end: ISO8601Timestamp;
  };

  // Volume
  total_incidents: number;
  incidents_by_severity: Record<Severity, number>;
  incidents_by_category: Record<Category, number>;
  incidents_by_status: Record<IncidentStatus, number>;

  // Performance
  average_mttd: number;
  average_mtta: number;
  average_mttr: number;
  p50_mttr: number;
  p95_mttr: number;
  p99_mttr: number;

  // SLA
  sla_acknowledgment_compliance: number;  // Percentage
  sla_resolution_compliance: number;      // Percentage
  total_sla_breaches: number;

  // Trends
  incident_trend: DataPoint[];
  mttr_trend: DataPoint[];
}

export interface DataPoint {
  timestamp: ISO8601Timestamp;
  value: number;
}

/**
 * Team performance metrics
 */
export interface TeamMetrics {
  team_id: string;
  time_range: {
    start: ISO8601Timestamp;
    end: ISO8601Timestamp;
  };

  // Volume
  total_incidents_handled: number;
  incidents_by_severity: Record<Severity, number>;

  // Performance
  average_response_time: number;
  average_resolution_time: number;

  // Quality
  reopened_incidents: number;
  escalated_incidents: number;

  // Individual contributions
  user_metrics: UserMetrics[];
}

export interface UserMetrics {
  user_id: string;
  incidents_handled: number;
  average_resolution_time: number;
  on_call_hours: number;
}

// ============================================================================
// CONFIGURATION MODELS
// ============================================================================

/**
 * System configuration
 */
export interface SystemConfig {
  // General
  instance_id: string;
  deployment_mode: 'standalone' | 'distributed' | 'sidecar';

  // Database
  database: DatabaseConfig;

  // Message queue
  message_queue: MessageQueueConfig;

  // Cache
  cache: CacheConfig;

  // Integrations
  integrations: Record<string, Integration>;

  // Notification
  notification: NotificationConfig;

  // Features
  features: FeatureFlags;

  // Limits
  limits: SystemLimits;
}

export interface DatabaseConfig {
  type: 'postgresql' | 'mongodb' | 'sqlite';
  url: string;
  pool_size: number;
  timeout_ms: number;
}

export interface MessageQueueConfig {
  type: 'redis' | 'rabbitmq' | 'kafka' | 'nats';
  brokers: string[];
  topics: Record<string, TopicConfig>;
}

export interface TopicConfig {
  partitions: number;
  replication_factor: number;
  retention_ms: number;
}

export interface CacheConfig {
  type: 'redis' | 'memcached';
  url: string;
  ttl_seconds: number;
}

export interface NotificationConfig {
  default_channels: Channel[];
  rate_limits: Record<Channel, RateLimit>;
  templates_path: string;
}

export interface RateLimit {
  requests_per_minute: number;
  burst: number;
}

export interface FeatureFlags {
  ml_classification_enabled: boolean;
  auto_remediation_enabled: boolean;
  playbook_execution_enabled: boolean;
  multi_region_enabled: boolean;
}

export interface SystemLimits {
  max_events_per_minute: number;
  max_concurrent_workers: number;
  max_incident_age_days: number;
  max_audit_log_age_days: number;
}

// ============================================================================
// API REQUEST/RESPONSE MODELS
// ============================================================================

/**
 * API request to create incident
 */
export interface CreateIncidentRequest {
  event: RawEvent;
  options?: {
    skip_deduplication?: boolean;
    force_severity?: Severity;
    assign_to?: string;
  };
}

export interface CreateIncidentResponse {
  status: 'created' | 'duplicate';
  incident_id: string;
  duplicate_of?: string;
  message: string;
}

/**
 * API request to update incident
 */
export interface UpdateIncidentRequest {
  incident_id: string;
  updates: Partial<Incident>;
  actor: Actor;
  notes?: string;
}

export interface UpdateIncidentResponse {
  status: 'success' | 'error';
  incident: Incident;
  message: string;
}

/**
 * API query for incidents
 */
export interface QueryIncidentsRequest {
  filters?: {
    severity?: Severity[];
    status?: IncidentStatus[];
    category?: Category[];
    environment?: Environment[];
    date_range?: {
      start: ISO8601Timestamp;
      end: ISO8601Timestamp;
    };
    tags?: Record<string, string>;
    search?: string;
  };
  pagination?: {
    page: number;
    page_size: number;
  };
  sort?: {
    field: string;
    order: 'asc' | 'desc';
  };
}

export interface QueryIncidentsResponse {
  incidents: Incident[];
  total_count: number;
  page: number;
  page_size: number;
  has_more: boolean;
}
