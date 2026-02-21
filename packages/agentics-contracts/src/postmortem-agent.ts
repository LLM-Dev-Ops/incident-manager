/**
 * @agentics/contracts - Post-Mortem Generator Agent Contract
 *
 * Defines all input/output schemas for the Post-Mortem Generator Agent.
 * This agent is classified as:
 *   - DOCUMENTATION (PRIMARY)
 *   - INCIDENT_ANALYSIS (SECONDARY - POST-RESOLUTION ONLY)
 *
 * The agent produces authoritative post-incident records that capture the
 * complete incident lifecycle, including timeline reconstruction, root cause
 * analysis, impact assessment, and actionable follow-up items.
 *
 * CRITICAL CONSTRAINTS:
 *   - MUST NOT modify incident state (read-only analysis)
 *   - MUST NOT trigger remediation actions
 *   - MUST NOT reassign severity
 *   - MUST NOT alter historical timeline
 *   - MUST NOT generate during active incidents (only resolved/closed)
 */

import type {
  ISO8601Timestamp,
  UUID,
  Severity,
  Environment,
  ValidationResult,
  PersistenceSpec
} from './common.js';

import type {
  DecisionEvent
} from './decision-event.js';

import type {
  IncidentCategory,
  IncidentStatus,
  AffectedResource
} from './escalation-agent.js';

// ============================================================================
// AGENT METADATA
// ============================================================================

export const POSTMORTEM_AGENT_METADATA = {
  agent_type: 'incident-postmortem' as const,
  agent_classification: ['DOCUMENTATION', 'INCIDENT_ANALYSIS'] as const,
  decision_type: 'incident_postmortem_generated' as const,
  version: '1.0.0',

  /** What this agent MAY do */
  capabilities: [
    'Reconstruct incident timelines from immutable event records',
    'Perform root cause analysis on resolved incidents',
    'Assess incident impact across services, users, and revenue',
    'Generate actionable follow-up items with priority and ownership',
    'Produce structured post-mortem documents for review',
    'Persist post-mortem records via ruvector-service'
  ],

  /** What this agent MUST NOT do - HARD CONSTRAINTS */
  prohibitions: [
    'MUST NOT modify incident state (post-mortems are read-only)',
    'MUST NOT trigger remediation actions',
    'MUST NOT reassign severity levels',
    'MUST NOT alter historical timeline entries',
    'MUST NOT generate for active (unresolved) incidents',
    'MUST NOT intercept runtime execution',
    'MUST NOT emit alerts externally (email, pager, webhook) - delegates to Orchestrator',
    'MUST NOT evaluate model performance (that is Sentinel)',
    'MUST NOT enforce security policies (that is Shield)'
  ],

  /** Which systems MAY invoke this agent */
  allowed_invokers: [
    'llm-orchestrator',
    'llm-incident-manager-api',
    'incident-manager-cli',
    'llm-governance-core'
  ]
} as const;

// ============================================================================
// ENUMS & TYPES
// ============================================================================

/**
 * Status of the post-mortem document
 */
export type PostMortemStatus =
  | 'draft'
  | 'in_review'
  | 'approved'
  | 'published';

/**
 * Priority of an action item
 */
export type ActionItemPriority =
  | 'critical'
  | 'high'
  | 'medium'
  | 'low';

/**
 * Status of an action item
 */
export type ActionItemStatus =
  | 'open'
  | 'in_progress'
  | 'completed'
  | 'wont_fix';

/**
 * Type of timeline entry
 */
export type TimelineEntryType =
  | 'detection'
  | 'notification'
  | 'acknowledgment'
  | 'escalation'
  | 'investigation'
  | 'remediation'
  | 'mitigation'
  | 'resolution'
  | 'communication'
  | 'decision';

/**
 * Root cause category
 */
export type RootCauseCategory =
  | 'code_defect'
  | 'configuration_error'
  | 'infrastructure_failure'
  | 'dependency_failure'
  | 'capacity_exhaustion'
  | 'security_breach'
  | 'human_error'
  | 'process_gap'
  | 'unknown';

// ============================================================================
// INPUT SCHEMAS
// ============================================================================

/**
 * Input to the Post-Mortem Generator Agent
 */
export interface PostMortemAgentInput {
  // ==================== INCIDENT IDENTIFICATION ====================

  /** Incident ID */
  incident_id: UUID;

  /** External/source incident ID (if different) */
  external_incident_id?: string;

  // ==================== INCIDENT CONTEXT ====================

  /** Incident title */
  title: string;

  /** Incident description */
  description: string;

  /** Incident category */
  category: IncidentCategory;

  /** Final severity at resolution */
  severity: Severity;

  /** Peak severity during incident */
  peak_severity: Severity;

  /** Current status (must be RESOLVED or CLOSED) */
  status: IncidentStatus;

  /** Environment */
  environment: Environment;

  /** Affected resource */
  affected_resource: AffectedResource;

  // ==================== TIMELINE ====================

  /** When the incident was created/detected */
  incident_created_at: ISO8601Timestamp;

  /** When the incident was resolved */
  incident_resolved_at: ISO8601Timestamp;

  /** When the incident was closed (if different from resolved) */
  incident_closed_at?: ISO8601Timestamp;

  /** Total duration in seconds */
  duration_seconds: number;

  /** Time to detect in seconds */
  time_to_detect_seconds?: number;

  /** Time to acknowledge in seconds */
  time_to_acknowledge_seconds?: number;

  /** Time to mitigate in seconds */
  time_to_mitigate_seconds?: number;

  /** Time to resolve in seconds */
  time_to_resolve_seconds: number;

  /** Ordered timeline of incident events */
  timeline_events: TimelineEvent[];

  // ==================== ESCALATION HISTORY ====================

  /** Escalation history entries */
  escalation_history: EscalationHistoryRef[];

  /** Maximum escalation level reached */
  max_escalation_level: number;

  // ==================== RESOLUTION ====================

  /** How the incident was resolved */
  resolution_summary: string;

  /** Actions taken to resolve */
  resolution_actions: string[];

  /** Who resolved the incident */
  resolved_by: string;

  // ==================== POLICY CONTEXT ====================

  /** Post-mortem policy ID */
  policy_id?: UUID;

  // ==================== EXECUTION CONTEXT ====================

  /** Execution correlation ID */
  execution_id: UUID;

  /** Trace ID for distributed tracing */
  trace_id?: string;
}

export interface TimelineEvent {
  /** Event timestamp */
  timestamp: ISO8601Timestamp;

  /** Type of event */
  type: TimelineEntryType;

  /** Human-readable description */
  description: string;

  /** Actor (user, system, or agent) */
  actor: string;

  /** Actor type */
  actor_type: 'user' | 'system' | 'agent';

  /** Additional event metadata */
  metadata?: Record<string, unknown>;
}

export interface EscalationHistoryRef {
  /** When escalation occurred */
  timestamp: ISO8601Timestamp;

  /** From severity */
  from_severity: Severity;

  /** To severity */
  to_severity: Severity;

  /** Reason */
  reason: string;

  /** Actor */
  actor: string;
}

// ============================================================================
// OUTPUT SCHEMAS
// ============================================================================

/**
 * Output of the Post-Mortem Generator Agent
 */
export interface PostMortemAgentOutput {
  // ==================== POST-MORTEM DOCUMENT ====================

  /** Generated post-mortem ID */
  postmortem_id: UUID;

  /** Post-mortem status */
  status: PostMortemStatus;

  /** Executive summary */
  summary: string;

  // ==================== TIMELINE RECONSTRUCTION ====================

  /** Reconstructed and annotated timeline */
  timeline: ReconstructedTimelineEntry[];

  /** Key moments identified */
  key_moments: KeyMoment[];

  // ==================== ROOT CAUSE ANALYSIS ====================

  /** Root cause analysis */
  root_cause: RootCauseAnalysis;

  // ==================== IMPACT ASSESSMENT ====================

  /** Impact analysis */
  impact: ImpactAnalysis;

  // ==================== RESOLUTION ANALYSIS ====================

  /** Resolution analysis */
  resolution: ResolutionAnalysis;

  // ==================== ACTION ITEMS ====================

  /** Follow-up action items */
  action_items: ActionItem[];

  // ==================== WORKFLOW ACTIONS ====================

  /** Actions for the Orchestrator */
  orchestrator_actions: PostMortemOrchestratorAction[];

  // ==================== LESSONS LEARNED ====================

  /** What went well */
  what_went_well: string[];

  /** What could be improved */
  what_could_be_improved: string[];

  /** Lessons learned */
  lessons_learned: string[];
}

export interface ReconstructedTimelineEntry {
  /** Event timestamp */
  timestamp: ISO8601Timestamp;

  /** Event type */
  type: TimelineEntryType;

  /** Description */
  description: string;

  /** Actor */
  actor: string;

  /** Duration to next event (seconds) */
  gap_to_next_seconds?: number;

  /** Annotation/analysis */
  annotation?: string;
}

export interface KeyMoment {
  /** Timestamp */
  timestamp: ISO8601Timestamp;

  /** Label */
  label: string;

  /** Significance */
  significance: string;
}

export interface RootCauseAnalysis {
  /** Primary root cause category */
  category: RootCauseCategory;

  /** Root cause description */
  description: string;

  /** Contributing factors */
  contributing_factors: string[];

  /** Was this root cause preventable? */
  preventable: boolean;

  /** Detection gap analysis */
  detection_gap?: string;
}

export interface ImpactAnalysis {
  /** Impact scope */
  scope: 'single_service' | 'multiple_services' | 'tenant' | 'region' | 'global';

  /** Services affected */
  services_affected: string[];

  /** Estimated users affected */
  users_affected_estimate?: number;

  /** Duration of user-facing impact (seconds) */
  user_impact_duration_seconds?: number;

  /** Revenue impact estimate */
  revenue_impact_estimate?: string;

  /** SLA impact */
  sla_impact?: string;

  /** Data impact */
  data_impact?: string;
}

export interface ResolutionAnalysis {
  /** Resolution approach */
  approach: string;

  /** Was the resolution permanent or temporary? */
  resolution_type: 'permanent' | 'temporary' | 'workaround';

  /** Effectiveness assessment */
  effectiveness: 'fully_resolved' | 'partially_resolved' | 'mitigated';

  /** Remaining risks */
  remaining_risks: string[];

  /** Follow-up required */
  follow_up_required: boolean;
}

export interface ActionItem {
  /** Action item ID */
  id: UUID;

  /** Title */
  title: string;

  /** Description */
  description: string;

  /** Priority */
  priority: ActionItemPriority;

  /** Status */
  status: ActionItemStatus;

  /** Assigned to */
  assigned_to?: string;

  /** Due date */
  due_date?: ISO8601Timestamp;

  /** Category */
  category: 'prevention' | 'detection' | 'response' | 'process' | 'tooling';
}

export interface PostMortemOrchestratorAction {
  /** Action type */
  action_type:
    | 'notify_stakeholders'
    | 'schedule_review_meeting'
    | 'create_action_items'
    | 'update_knowledge_base'
    | 'log_postmortem_event';

  /** Priority */
  priority: 'critical' | 'high' | 'normal' | 'low';

  /** Parameters */
  parameters: Record<string, unknown>;

  /** Async execution */
  async: boolean;
}

// ============================================================================
// DECISION EVENT TYPE
// ============================================================================

/**
 * DecisionEvent type for Post-Mortem Agent
 */
export type PostMortemDecisionEvent = DecisionEvent<PostMortemAgentOutput>;

// ============================================================================
// PERSISTENCE SPECIFICATION
// ============================================================================

/**
 * What to persist for post-mortem decisions
 */
export const POSTMORTEM_PERSISTENCE: PersistenceSpec = {
  persist: [
    // Incident identification
    'incident_id',
    'external_incident_id',
    // Post-mortem document
    'postmortem_id',
    'status',
    'summary',
    // Root cause
    'root_cause',
    // Impact
    'impact',
    // Resolution
    'resolution',
    // Action items
    'action_items',
    // Timeline
    'timeline',
    'key_moments',
    // Lessons
    'what_went_well',
    'what_could_be_improved',
    'lessons_learned',
    // Context
    'category',
    'severity',
    'peak_severity',
    'duration_seconds'
  ],
  exclude: [
    // Transient execution context
    'execution_id',
    'trace_id',
    // Raw timeline events (persisted as reconstructed timeline)
    'timeline_events',
    // Orchestrator actions (sent, not persisted here)
    'orchestrator_actions',
    // Incident details (stored in incident record)
    'title',
    'description',
    'resolution_summary',
    'resolution_actions'
  ],
  ttl_seconds: 0 // Permanent audit record
};

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Validate post-mortem agent input
 */
export function validatePostMortemInput(input: unknown): ValidationResult {
  const errors: { field: string; message: string; code: string; value?: unknown }[] = [];
  const warnings: { field: string; message: string; code: string }[] = [];

  if (!input || typeof input !== 'object') {
    return {
      valid: false,
      errors: [{ field: 'root', message: 'Input must be an object', code: 'INVALID_TYPE' }],
      warnings: []
    };
  }

  const i = input as Record<string, unknown>;

  // ==================== REQUIRED FIELDS ====================

  if (!i.incident_id) {
    errors.push({ field: 'incident_id', message: 'incident_id is required', code: 'REQUIRED' });
  }

  if (!i.title) {
    errors.push({ field: 'title', message: 'title is required', code: 'REQUIRED' });
  }

  if (!i.description) {
    errors.push({ field: 'description', message: 'description is required', code: 'REQUIRED' });
  }

  if (!i.severity) {
    errors.push({ field: 'severity', message: 'severity is required', code: 'REQUIRED' });
  }

  if (!i.peak_severity) {
    errors.push({ field: 'peak_severity', message: 'peak_severity is required', code: 'REQUIRED' });
  }

  if (!i.status) {
    errors.push({ field: 'status', message: 'status is required', code: 'REQUIRED' });
  }

  if (!i.incident_created_at) {
    errors.push({ field: 'incident_created_at', message: 'incident_created_at is required', code: 'REQUIRED' });
  }

  if (!i.incident_resolved_at) {
    errors.push({ field: 'incident_resolved_at', message: 'incident_resolved_at is required', code: 'REQUIRED' });
  }

  if (typeof i.duration_seconds !== 'number') {
    errors.push({ field: 'duration_seconds', message: 'duration_seconds must be a number', code: 'INVALID_TYPE' });
  }

  if (typeof i.time_to_resolve_seconds !== 'number') {
    errors.push({ field: 'time_to_resolve_seconds', message: 'time_to_resolve_seconds must be a number', code: 'INVALID_TYPE' });
  }

  if (!i.resolution_summary) {
    errors.push({ field: 'resolution_summary', message: 'resolution_summary is required', code: 'REQUIRED' });
  }

  if (!i.resolved_by) {
    errors.push({ field: 'resolved_by', message: 'resolved_by is required', code: 'REQUIRED' });
  }

  if (!i.execution_id) {
    errors.push({ field: 'execution_id', message: 'execution_id is required', code: 'REQUIRED' });
  }

  // ==================== TYPE VALIDATIONS ====================

  // Validate severity
  const validSeverities = ['P0', 'P1', 'P2', 'P3', 'P4'];
  if (i.severity && !validSeverities.includes(i.severity as string)) {
    errors.push({
      field: 'severity',
      message: `severity must be one of: ${validSeverities.join(', ')}`,
      code: 'INVALID_VALUE',
      value: i.severity
    });
  }

  if (i.peak_severity && !validSeverities.includes(i.peak_severity as string)) {
    errors.push({
      field: 'peak_severity',
      message: `peak_severity must be one of: ${validSeverities.join(', ')}`,
      code: 'INVALID_VALUE',
      value: i.peak_severity
    });
  }

  // Validate status - MUST be resolved or closed
  const allowedStatuses = ['RESOLVED', 'CLOSED'];
  if (i.status && !allowedStatuses.includes(i.status as string)) {
    errors.push({
      field: 'status',
      message: `status must be one of: ${allowedStatuses.join(', ')} (post-mortems only for resolved/closed incidents)`,
      code: 'INVALID_VALUE',
      value: i.status
    });
  }

  // Validate affected_resource
  if (i.affected_resource) {
    const ar = i.affected_resource as Record<string, unknown>;
    if (!ar.type) {
      errors.push({ field: 'affected_resource.type', message: 'affected_resource.type is required', code: 'REQUIRED' });
    }
    if (!ar.id) {
      errors.push({ field: 'affected_resource.id', message: 'affected_resource.id is required', code: 'REQUIRED' });
    }
  } else {
    errors.push({ field: 'affected_resource', message: 'affected_resource is required', code: 'REQUIRED' });
  }

  // Validate timeline_events
  if (i.timeline_events && Array.isArray(i.timeline_events)) {
    if ((i.timeline_events as unknown[]).length === 0) {
      warnings.push({ field: 'timeline_events', message: 'timeline_events is empty', code: 'EMPTY_TIMELINE' });
    }
  } else {
    errors.push({ field: 'timeline_events', message: 'timeline_events must be an array', code: 'INVALID_TYPE' });
  }

  // ==================== LOGICAL VALIDATIONS ====================

  if (i.incident_created_at && i.incident_resolved_at) {
    const created = new Date(i.incident_created_at as string);
    const resolved = new Date(i.incident_resolved_at as string);
    if (resolved < created) {
      errors.push({
        field: 'incident_resolved_at',
        message: 'incident_resolved_at cannot be before incident_created_at',
        code: 'LOGICAL_ERROR'
      });
    }
  }

  return { valid: errors.length === 0, errors, warnings };
}

/**
 * Validate post-mortem agent output
 */
export function validatePostMortemOutput(output: unknown): ValidationResult {
  const errors: { field: string; message: string; code: string; value?: unknown }[] = [];
  const warnings: { field: string; message: string; code: string }[] = [];

  if (!output || typeof output !== 'object') {
    return {
      valid: false,
      errors: [{ field: 'root', message: 'Output must be an object', code: 'INVALID_TYPE' }],
      warnings: []
    };
  }

  const o = output as Record<string, unknown>;

  // ==================== REQUIRED FIELDS ====================

  if (!o.postmortem_id) {
    errors.push({ field: 'postmortem_id', message: 'postmortem_id is required', code: 'REQUIRED' });
  }

  if (!o.status) {
    errors.push({ field: 'status', message: 'status is required', code: 'REQUIRED' });
  }

  if (!o.summary) {
    errors.push({ field: 'summary', message: 'summary is required', code: 'REQUIRED' });
  }

  if (!Array.isArray(o.timeline)) {
    errors.push({ field: 'timeline', message: 'timeline must be an array', code: 'INVALID_TYPE' });
  }

  if (!o.root_cause) {
    errors.push({ field: 'root_cause', message: 'root_cause is required', code: 'REQUIRED' });
  }

  if (!o.impact) {
    errors.push({ field: 'impact', message: 'impact is required', code: 'REQUIRED' });
  }

  if (!o.resolution) {
    errors.push({ field: 'resolution', message: 'resolution is required', code: 'REQUIRED' });
  }

  if (!Array.isArray(o.action_items)) {
    errors.push({ field: 'action_items', message: 'action_items must be an array', code: 'INVALID_TYPE' });
  }

  if (!Array.isArray(o.orchestrator_actions)) {
    errors.push({ field: 'orchestrator_actions', message: 'orchestrator_actions must be an array', code: 'INVALID_TYPE' });
  }

  // ==================== TYPE VALIDATIONS ====================

  const validStatuses = ['draft', 'in_review', 'approved', 'published'];
  if (o.status && !validStatuses.includes(o.status as string)) {
    errors.push({
      field: 'status',
      message: `status must be one of: ${validStatuses.join(', ')}`,
      code: 'INVALID_VALUE',
      value: o.status
    });
  }

  // Validate root_cause structure
  if (o.root_cause && typeof o.root_cause === 'object') {
    const rc = o.root_cause as Record<string, unknown>;
    if (!rc.category) {
      errors.push({ field: 'root_cause.category', message: 'root_cause.category is required', code: 'REQUIRED' });
    }
    if (!rc.description) {
      errors.push({ field: 'root_cause.description', message: 'root_cause.description is required', code: 'REQUIRED' });
    }
  }

  // Validate action_items structure
  if (Array.isArray(o.action_items)) {
    (o.action_items as Record<string, unknown>[]).forEach((item, idx) => {
      if (!item.title) {
        errors.push({ field: `action_items[${idx}].title`, message: 'title is required', code: 'REQUIRED' });
      }
      if (!item.priority) {
        errors.push({ field: `action_items[${idx}].priority`, message: 'priority is required', code: 'REQUIRED' });
      }
    });
  }

  // Warn if no action items
  if (Array.isArray(o.action_items) && (o.action_items as unknown[]).length === 0) {
    warnings.push({
      field: 'action_items',
      message: 'No action items generated - consider adding preventive measures',
      code: 'NO_ACTION_ITEMS'
    });
  }

  return { valid: errors.length === 0, errors, warnings };
}
