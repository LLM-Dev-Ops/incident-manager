/**
 * @agentics/contracts - Incident Escalation Agent Contract
 *
 * Defines all input/output schemas for the Incident Escalation Agent.
 * This agent is classified as:
 *   - INCIDENT ORCHESTRATION
 *   - ESCALATION
 *
 * The agent determines incident severity changes and triggers controlled
 * escalation across predefined escalation ladders.
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

// ============================================================================
// AGENT METADATA
// ============================================================================

export const ESCALATION_AGENT_METADATA = {
  agent_type: 'incident-escalation',
  agent_classification: ['INCIDENT_ORCHESTRATION', 'ESCALATION'] as const,
  decision_type: 'incident_escalation_decision' as const,
  version: '1.0.0',

  /** What this agent MAY do */
  capabilities: [
    'Evaluate incident signals from Sentinel, Edge-Agent, Shield, and Orchestrator',
    'Assess severity thresholds and escalation policies',
    'Transition incidents between severity levels (e.g., SEV3 -> SEV2 -> SEV1)',
    'Trigger downstream escalation actions via Orchestrator'
  ],

  /** What this agent MUST NOT do */
  prohibitions: [
    'MUST NOT perform remediation directly',
    'MUST NOT emit alerts externally (email, pager, webhook)',
    'MUST NOT modify routing or execution behavior',
    'MUST NOT alter escalation policies dynamically',
    'MUST NOT intercept runtime execution',
    'MUST NOT enforce policies (that is Shield)',
    'MUST NOT emit anomaly detections (that is Sentinel)'
  ],

  /** Which systems MAY invoke this agent */
  allowed_invokers: [
    'llm-sentinel',
    'llm-shield',
    'llm-edge-agent',
    'llm-orchestrator',
    'incident-manager-cli',
    'incident-manager-api'
  ]
} as const;

// ============================================================================
// INPUT SCHEMAS
// ============================================================================

/**
 * Source of the escalation signal
 */
export type EscalationSignalSource =
  | 'llm-sentinel'
  | 'llm-shield'
  | 'llm-edge-agent'
  | 'llm-orchestrator'
  | 'manual'
  | 'scheduled';

/**
 * Category of incident for escalation routing
 */
export type IncidentCategory =
  | 'performance'
  | 'security'
  | 'availability'
  | 'compliance'
  | 'cost'
  | 'quality'
  | 'other';

/**
 * Incident status for escalation evaluation
 */
export type IncidentStatus =
  | 'NEW'
  | 'ACKNOWLEDGED'
  | 'IN_PROGRESS'
  | 'ESCALATED'
  | 'RESOLVED'
  | 'CLOSED';

/**
 * Input to the Incident Escalation Agent
 */
export interface EscalationAgentInput {
  // ==================== INCIDENT IDENTIFICATION ====================

  /** Incident ID */
  incident_id: UUID;

  /** External/source incident ID (if different) */
  external_incident_id?: string;

  /** Fingerprint for deduplication */
  fingerprint: string;

  // ==================== CURRENT STATE ====================

  /** Current severity level */
  current_severity: Severity;

  /** Current incident status */
  current_status: IncidentStatus;

  /** Current escalation level (0 = not escalated) */
  current_escalation_level: number;

  /** Category of incident */
  category: IncidentCategory;

  // ==================== SIGNAL ====================

  /** Source of escalation signal */
  signal_source: EscalationSignalSource;

  /** Timestamp of the signal */
  signal_timestamp: ISO8601Timestamp;

  /** Signal payload (varies by source) */
  signal_payload: EscalationSignalPayload;

  // ==================== CONTEXT ====================

  /** Environment */
  environment: Environment;

  /** Incident title */
  title: string;

  /** Incident description */
  description: string;

  /** Impact description */
  impact?: string;

  /** Affected resource */
  affected_resource: AffectedResource;

  /** Tags/labels */
  tags: Record<string, string>;

  // ==================== HISTORY ====================

  /** When incident was created */
  incident_created_at: ISO8601Timestamp;

  /** When incident was last updated */
  incident_updated_at: ISO8601Timestamp;

  /** Time since last escalation (seconds), null if never escalated */
  time_since_last_escalation?: number;

  /** Previous escalation attempts */
  escalation_history: EscalationHistoryEntry[];

  // ==================== POLICY CONTEXT ====================

  /** Escalation policy ID to apply (optional - agent may select) */
  policy_id?: UUID;

  /** SLA thresholds */
  sla: SLAContext;

  // ==================== EXECUTION CONTEXT ====================

  /** Execution correlation ID */
  execution_id: UUID;

  /** Trace ID for distributed tracing */
  trace_id?: string;
}

export interface EscalationSignalPayload {
  /** Signal type */
  type: 'anomaly' | 'violation' | 'threshold_breach' | 'timeout' | 'manual' | 'scheduled';

  /** Signal severity suggestion (may differ from current) */
  suggested_severity?: Severity;

  /** Metrics that triggered the signal */
  metrics?: Record<string, number>;

  /** Raw signal data */
  raw_data?: Record<string, unknown>;

  /** Signal confidence (0-1) */
  signal_confidence?: number;
}

export interface AffectedResource {
  /** Resource type */
  type: 'service' | 'endpoint' | 'model' | 'deployment' | 'tenant' | 'infrastructure';

  /** Resource ID */
  id: string;

  /** Resource name */
  name: string;

  /** Additional metadata */
  metadata?: Record<string, unknown>;
}

export interface EscalationHistoryEntry {
  /** When this escalation occurred */
  timestamp: ISO8601Timestamp;

  /** From severity */
  from_severity: Severity;

  /** To severity */
  to_severity: Severity;

  /** From level */
  from_level: number;

  /** To level */
  to_level: number;

  /** Reason for escalation */
  reason: string;

  /** Actor (system/user) */
  actor: string;
}

export interface SLAContext {
  /** Acknowledgment deadline */
  acknowledgment_deadline?: ISO8601Timestamp;

  /** Resolution deadline */
  resolution_deadline?: ISO8601Timestamp;

  /** Whether acknowledgment SLA is breached */
  acknowledgment_breached: boolean;

  /** Whether resolution SLA is breached */
  resolution_breached: boolean;
}

// ============================================================================
// OUTPUT SCHEMAS
// ============================================================================

/**
 * Escalation decision type
 */
export type EscalationDecision =
  | 'escalate'
  | 'deescalate'
  | 'maintain'
  | 'defer';

/**
 * Output of the Incident Escalation Agent
 */
export interface EscalationAgentOutput {
  // ==================== DECISION ====================

  /** The escalation decision */
  decision: EscalationDecision;

  /** Reason for the decision */
  reason: string;

  // ==================== SEVERITY CHANGE ====================

  /** New severity (if escalate/deescalate) */
  new_severity?: Severity;

  /** New escalation level */
  new_escalation_level?: number;

  /** Severity change delta (negative = escalation, positive = deescalation) */
  severity_delta?: number;

  // ==================== ACTIONS ====================

  /** Actions to be taken by Orchestrator */
  orchestrator_actions: OrchestratorAction[];

  /** Assignment updates */
  assignment_updates?: AssignmentUpdate[];

  // ==================== TIMING ====================

  /** When to re-evaluate if decision is 'defer' */
  defer_until?: ISO8601Timestamp;

  /** Next scheduled escalation check */
  next_evaluation_at?: ISO8601Timestamp;

  // ==================== AUDIT ====================

  /** Policy that was applied */
  applied_policy: AppliedPolicy;

  /** Evaluation details for audit */
  evaluation_details: EvaluationDetails;
}

export interface OrchestratorAction {
  /** Action type */
  action_type:
    | 'notify_escalation_targets'
    | 'trigger_playbook'
    | 'update_incident_status'
    | 'request_approval'
    | 'log_timeline_event';

  /** Action priority */
  priority: 'critical' | 'high' | 'normal' | 'low';

  /** Action parameters */
  parameters: Record<string, unknown>;

  /** Whether action should be async */
  async: boolean;
}

export interface AssignmentUpdate {
  /** Update type */
  type: 'assign' | 'reassign' | 'escalate_to_team';

  /** Target (user/team ID) */
  target_id: string;

  /** Target type */
  target_type: 'user' | 'team' | 'on_call';

  /** Reason for assignment */
  reason: string;
}

export interface AppliedPolicy {
  /** Policy ID */
  policy_id: UUID;

  /** Policy name */
  policy_name: string;

  /** Policy version */
  policy_version: string;

  /** Level that was triggered */
  triggered_level?: number;

  /** Maximum level in policy */
  max_level: number;
}

export interface EvaluationDetails {
  /** Thresholds that were evaluated */
  thresholds_evaluated: ThresholdEvaluation[];

  /** Time-based factors */
  time_factors: TimeFactor[];

  /** Pattern matches */
  pattern_matches?: PatternMatch[];

  /** Raw score before normalization */
  raw_escalation_score: number;

  /** Normalized score (0-1) */
  normalized_score: number;
}

export interface ThresholdEvaluation {
  /** Threshold name */
  name: string;

  /** Threshold value */
  threshold_value: number;

  /** Actual value */
  actual_value: number;

  /** Whether threshold was breached */
  breached: boolean;

  /** Weight in final score */
  weight: number;
}

export interface TimeFactor {
  /** Factor name */
  name: 'time_since_creation' | 'time_since_last_escalation' | 'sla_remaining' | 'time_in_current_state';

  /** Value in seconds */
  value_seconds: number;

  /** Contribution to escalation score */
  score_contribution: number;
}

export interface PatternMatch {
  /** Pattern name */
  pattern: string;

  /** Match confidence */
  confidence: number;

  /** What matched */
  matched_elements: string[];
}

// ============================================================================
// DECISION EVENT TYPE
// ============================================================================

/**
 * DecisionEvent type for Escalation Agent
 */
export type EscalationDecisionEvent = DecisionEvent<EscalationAgentOutput>;

// ============================================================================
// PERSISTENCE SPECIFICATION
// ============================================================================

/**
 * What to persist for escalation decisions
 */
export const ESCALATION_PERSISTENCE: PersistenceSpec = {
  persist: [
    // Input identification
    'incident_id',
    'external_incident_id',
    'fingerprint',
    // State at time of decision
    'current_severity',
    'current_status',
    'current_escalation_level',
    'category',
    // Signal info
    'signal_source',
    'signal_timestamp',
    // Decision output
    'decision',
    'new_severity',
    'new_escalation_level',
    'reason',
    // Policy audit
    'applied_policy',
    'evaluation_details'
  ],
  exclude: [
    // Raw signal payload (may contain PII)
    'signal_payload.raw_data',
    // Transient execution context
    'execution_id',
    'trace_id',
    // Full descriptions (stored in incident record)
    'title',
    'description',
    'impact',
    // History (stored separately)
    'escalation_history',
    // Assignment updates (sent to orchestrator, not persisted here)
    'orchestrator_actions',
    'assignment_updates'
  ],
  ttl_seconds: 0 // Permanent audit record
};

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Validate escalation agent input
 */
export function validateEscalationInput(input: unknown): ValidationResult {
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

  // Required fields
  if (!i.incident_id) {
    errors.push({ field: 'incident_id', message: 'incident_id is required', code: 'REQUIRED' });
  }
  if (!i.fingerprint) {
    errors.push({ field: 'fingerprint', message: 'fingerprint is required', code: 'REQUIRED' });
  }
  if (!i.current_severity) {
    errors.push({ field: 'current_severity', message: 'current_severity is required', code: 'REQUIRED' });
  }
  if (!i.current_status) {
    errors.push({ field: 'current_status', message: 'current_status is required', code: 'REQUIRED' });
  }
  if (typeof i.current_escalation_level !== 'number') {
    errors.push({ field: 'current_escalation_level', message: 'current_escalation_level must be a number', code: 'INVALID_TYPE' });
  }
  if (!i.signal_source) {
    errors.push({ field: 'signal_source', message: 'signal_source is required', code: 'REQUIRED' });
  }
  if (!i.signal_timestamp) {
    errors.push({ field: 'signal_timestamp', message: 'signal_timestamp is required', code: 'REQUIRED' });
  }
  if (!i.execution_id) {
    errors.push({ field: 'execution_id', message: 'execution_id is required', code: 'REQUIRED' });
  }

  // Validate severity
  const validSeverities = ['P0', 'P1', 'P2', 'P3', 'P4'];
  if (i.current_severity && !validSeverities.includes(i.current_severity as string)) {
    errors.push({
      field: 'current_severity',
      message: `current_severity must be one of: ${validSeverities.join(', ')}`,
      code: 'INVALID_VALUE',
      value: i.current_severity
    });
  }

  // Validate signal source
  const validSources = ['llm-sentinel', 'llm-shield', 'llm-edge-agent', 'llm-orchestrator', 'manual', 'scheduled'];
  if (i.signal_source && !validSources.includes(i.signal_source as string)) {
    warnings.push({
      field: 'signal_source',
      message: `Unrecognized signal_source: ${i.signal_source}`,
      code: 'UNRECOGNIZED_VALUE'
    });
  }

  // Validate affected_resource if present
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

  return { valid: errors.length === 0, errors, warnings };
}

/**
 * Validate escalation agent output
 */
export function validateEscalationOutput(output: unknown): ValidationResult {
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

  // Required fields
  if (!o.decision) {
    errors.push({ field: 'decision', message: 'decision is required', code: 'REQUIRED' });
  }
  if (!o.reason) {
    errors.push({ field: 'reason', message: 'reason is required', code: 'REQUIRED' });
  }
  if (!o.applied_policy) {
    errors.push({ field: 'applied_policy', message: 'applied_policy is required', code: 'REQUIRED' });
  }
  if (!o.evaluation_details) {
    errors.push({ field: 'evaluation_details', message: 'evaluation_details is required', code: 'REQUIRED' });
  }
  if (!Array.isArray(o.orchestrator_actions)) {
    errors.push({ field: 'orchestrator_actions', message: 'orchestrator_actions must be an array', code: 'INVALID_TYPE' });
  }

  // Validate decision
  const validDecisions = ['escalate', 'deescalate', 'maintain', 'defer'];
  if (o.decision && !validDecisions.includes(o.decision as string)) {
    errors.push({
      field: 'decision',
      message: `decision must be one of: ${validDecisions.join(', ')}`,
      code: 'INVALID_VALUE',
      value: o.decision
    });
  }

  // If decision is 'escalate' or 'deescalate', new_severity should be present
  if ((o.decision === 'escalate' || o.decision === 'deescalate') && !o.new_severity) {
    warnings.push({
      field: 'new_severity',
      message: 'new_severity should be provided when decision is escalate/deescalate',
      code: 'RECOMMENDED'
    });
  }

  // If decision is 'defer', defer_until should be present
  if (o.decision === 'defer' && !o.defer_until) {
    warnings.push({
      field: 'defer_until',
      message: 'defer_until should be provided when decision is defer',
      code: 'RECOMMENDED'
    });
  }

  return { valid: errors.length === 0, errors, warnings };
}

// ============================================================================
// CLI CONTRACT
// ============================================================================

/**
 * CLI invocation shape for the Escalation Agent
 */
export interface EscalationAgentCLI {
  /** Command name */
  command: 'escalate';

  /** Subcommands */
  subcommands: {
    /** Evaluate an incident for escalation */
    evaluate: {
      args: {
        incident_id: string;
        signal_source?: EscalationSignalSource;
        policy_id?: string;
      };
      flags: {
        '--dry-run'?: boolean;
        '--verbose'?: boolean;
        '--json'?: boolean;
      };
    };

    /** Inspect escalation state */
    inspect: {
      args: {
        incident_id: string;
      };
      flags: {
        '--include-history'?: boolean;
        '--json'?: boolean;
      };
    };

    /** List active escalations */
    list: {
      args: {};
      flags: {
        '--severity'?: Severity;
        '--status'?: string;
        '--limit'?: number;
        '--json'?: boolean;
      };
    };
  };
}
