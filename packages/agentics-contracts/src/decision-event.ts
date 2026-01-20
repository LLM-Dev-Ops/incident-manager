/**
 * @agentics/contracts - DecisionEvent Schema
 *
 * DecisionEvent is the CANONICAL output format for ALL Agentics platform agents.
 * Every agent invocation MUST emit exactly ONE DecisionEvent to ruvector-service.
 *
 * This is a NON-NEGOTIABLE architectural requirement.
 */

import type {
  ISO8601Timestamp,
  UUID,
  SemVer,
  SHA256Hash,
  AgentId,
  AgentClassification,
  Severity,
  Environment,
  PersistenceSpec
} from './common.js';

// ============================================================================
// DECISION TYPES
// ============================================================================

/**
 * Type of decision made by an agent
 */
export type DecisionType =
  // Incident lifecycle decisions
  | 'incident_open'
  | 'incident_escalation_decision'
  | 'incident_approval_decision'
  | 'incident_resolution_decision'
  | 'incident_inspection'
  | 'incident_close'
  // Workflow decisions
  | 'remediation_trigger'
  | 'notification_dispatch'
  | 'policy_evaluation'
  // Meta decisions
  | 'no_action_required'
  | 'deferred_decision';

// ============================================================================
// CONSTRAINTS
// ============================================================================

/**
 * Constraints applied during decision making
 */
export interface DecisionConstraints {
  /** Severity thresholds that were evaluated */
  severity_thresholds?: SeverityThreshold[];

  /** Policy constraints that were applied */
  policy_constraints?: PolicyConstraint[];

  /** Approval requirements */
  approval_requirements?: ApprovalRequirement[];

  /** Time-based constraints */
  time_constraints?: TimeConstraint[];

  /** Rate limiting constraints */
  rate_limits?: RateLimitConstraint[];
}

export interface SeverityThreshold {
  threshold: Severity;
  direction: 'above' | 'below' | 'equal';
  triggered: boolean;
}

export interface PolicyConstraint {
  policy_id: string;
  policy_name: string;
  policy_version: string;
  satisfied: boolean;
  reason?: string;
}

export interface ApprovalRequirement {
  approver_type: 'user' | 'team' | 'system';
  approver_id: string;
  required: boolean;
  obtained: boolean;
}

export interface TimeConstraint {
  type: 'sla' | 'timeout' | 'schedule';
  deadline: ISO8601Timestamp;
  breached: boolean;
}

export interface RateLimitConstraint {
  limit_type: 'escalations_per_hour' | 'notifications_per_incident';
  limit_value: number;
  current_value: number;
  exceeded: boolean;
}

// ============================================================================
// DECISION EVENT SCHEMA
// ============================================================================

/**
 * DecisionEvent - The canonical output for ALL Agentics agents
 *
 * REQUIREMENTS (from PROMPT 0):
 * - agent_id: Unique identifier for the agent
 * - agent_version: Semantic version of the agent
 * - decision_type: Type of decision made
 * - inputs_hash: SHA-256 hash of all inputs for auditability
 * - outputs: The decision outputs
 * - confidence: Certainty score (0.0 - 1.0)
 * - constraints_applied: What constraints influenced the decision
 * - execution_ref: Reference to the execution context
 * - timestamp: UTC timestamp
 */
export interface DecisionEvent<TOutput = unknown> {
  // ==================== IDENTIFICATION ====================

  /** Unique ID for this decision event */
  id: UUID;

  /** Agent identifier (format: {type}:{version}:{instance}) */
  agent_id: AgentId;

  /** Agent semantic version */
  agent_version: SemVer;

  /** Agent classification */
  agent_classification: AgentClassification;

  // ==================== DECISION ====================

  /** Type of decision made */
  decision_type: DecisionType;

  /** SHA-256 hash of all inputs for audit trail */
  inputs_hash: SHA256Hash;

  /** The decision outputs (type depends on decision_type) */
  outputs: TOutput;

  // ==================== CONFIDENCE ====================

  /**
   * Confidence score (0.0 - 1.0)
   * For escalation: severity certainty / escalation confidence
   * For approval: approval confidence
   * For resolution: resolution certainty
   */
  confidence: number;

  /** Factors that influenced the confidence score */
  confidence_factors?: ConfidenceFactor[];

  // ==================== CONSTRAINTS ====================

  /** Constraints that were applied during decision making */
  constraints_applied: DecisionConstraints;

  // ==================== EXECUTION CONTEXT ====================

  /** Reference to execution context (correlation ID) */
  execution_ref: UUID;

  /** UTC timestamp when decision was made */
  timestamp: ISO8601Timestamp;

  /** Environment where decision was made */
  environment: Environment;

  /** Region where agent executed */
  region?: string;

  // ==================== TRACING ====================

  /** Distributed trace ID */
  trace_id?: string;

  /** Parent span ID */
  parent_span_id?: string;

  /** Span ID for this decision */
  span_id?: string;

  // ==================== AUDIT ====================

  /** Whether this decision requires human review */
  requires_review: boolean;

  /** Review deadline if requires_review is true */
  review_deadline?: ISO8601Timestamp;

  /** Audit metadata */
  audit_metadata?: AuditMetadata;
}

export interface ConfidenceFactor {
  factor: string;
  weight: number;
  contribution: number;
  explanation?: string;
}

export interface AuditMetadata {
  /** Source systems that contributed to this decision */
  sources: string[];

  /** Policies that were evaluated */
  policies_evaluated: string[];

  /** Any manual overrides */
  manual_overrides?: ManualOverride[];
}

export interface ManualOverride {
  field: string;
  original_value: unknown;
  override_value: unknown;
  reason: string;
  approved_by: string;
}

// ============================================================================
// PERSISTENCE SPECIFICATION
// ============================================================================

/**
 * Persistence specification for DecisionEvent
 * Defines what MUST and MUST NOT be persisted to ruvector-service
 */
export const DECISION_EVENT_PERSISTENCE: PersistenceSpec = {
  persist: [
    'id',
    'agent_id',
    'agent_version',
    'agent_classification',
    'decision_type',
    'inputs_hash',
    'outputs',
    'confidence',
    'constraints_applied',
    'execution_ref',
    'timestamp',
    'environment',
    'requires_review',
    'audit_metadata'
  ],
  exclude: [
    // Transient tracing data (stored elsewhere)
    'trace_id',
    'parent_span_id',
    'span_id',
    // Computed fields
    'confidence_factors',
    // Region is derived from execution context
    'region'
  ],
  ttl_seconds: 0 // No expiry - decisions are permanent audit records
};

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Validate a DecisionEvent
 */
export function validateDecisionEvent(event: unknown): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if (!event || typeof event !== 'object') {
    return { valid: false, errors: ['DecisionEvent must be an object'] };
  }

  const e = event as Record<string, unknown>;

  // Required fields
  if (!e.id) errors.push('id is required');
  if (!e.agent_id) errors.push('agent_id is required');
  if (!e.agent_version) errors.push('agent_version is required');
  if (!e.agent_classification) errors.push('agent_classification is required');
  if (!e.decision_type) errors.push('decision_type is required');
  if (!e.inputs_hash) errors.push('inputs_hash is required');
  if (e.outputs === undefined) errors.push('outputs is required');
  if (typeof e.confidence !== 'number') errors.push('confidence must be a number');
  if (!e.constraints_applied) errors.push('constraints_applied is required');
  if (!e.execution_ref) errors.push('execution_ref is required');
  if (!e.timestamp) errors.push('timestamp is required');
  if (typeof e.requires_review !== 'boolean') errors.push('requires_review must be a boolean');

  // Confidence range
  if (typeof e.confidence === 'number' && (e.confidence < 0 || e.confidence > 1)) {
    errors.push('confidence must be between 0 and 1');
  }

  return { valid: errors.length === 0, errors };
}

// ============================================================================
// BUILDER
// ============================================================================

/**
 * Builder for creating DecisionEvents with proper defaults
 */
export class DecisionEventBuilder<TOutput = unknown> {
  private event: Partial<DecisionEvent<TOutput>> = {
    timestamp: new Date().toISOString(),
    environment: 'production',
    requires_review: false,
    constraints_applied: {}
  };

  withId(id: UUID): this {
    this.event.id = id;
    return this;
  }

  withAgent(agentId: AgentId, version: SemVer, classification: AgentClassification): this {
    this.event.agent_id = agentId;
    this.event.agent_version = version;
    this.event.agent_classification = classification;
    return this;
  }

  withDecision(type: DecisionType, outputs: TOutput, inputsHash: SHA256Hash): this {
    this.event.decision_type = type;
    this.event.outputs = outputs;
    this.event.inputs_hash = inputsHash;
    return this;
  }

  withConfidence(confidence: number, factors?: ConfidenceFactor[]): this {
    this.event.confidence = confidence;
    this.event.confidence_factors = factors;
    return this;
  }

  withConstraints(constraints: DecisionConstraints): this {
    this.event.constraints_applied = constraints;
    return this;
  }

  withExecutionContext(executionRef: UUID, environment?: Environment, region?: string): this {
    this.event.execution_ref = executionRef;
    if (environment) this.event.environment = environment;
    if (region) this.event.region = region;
    return this;
  }

  withTracing(traceId: string, spanId: string, parentSpanId?: string): this {
    this.event.trace_id = traceId;
    this.event.span_id = spanId;
    this.event.parent_span_id = parentSpanId;
    return this;
  }

  requiresReview(deadline?: ISO8601Timestamp): this {
    this.event.requires_review = true;
    this.event.review_deadline = deadline;
    return this;
  }

  withAuditMetadata(metadata: AuditMetadata): this {
    this.event.audit_metadata = metadata;
    return this;
  }

  build(): DecisionEvent<TOutput> {
    const validation = validateDecisionEvent(this.event);
    if (!validation.valid) {
      throw new Error(`Invalid DecisionEvent: ${validation.errors.join(', ')}`);
    }
    return this.event as DecisionEvent<TOutput>;
  }
}
