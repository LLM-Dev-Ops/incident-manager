/**
 * @agentics/contracts - Human-in-the-Loop (HITL) Agent Contract
 *
 * Defines all input/output schemas for the Human-in-the-Loop Agent.
 * This agent is classified as:
 *   - APPROVAL_GATING (PRIMARY)
 *   - INCIDENT_ORCHESTRATION (SECONDARY)
 *
 * The agent enforces mandatory human approval gates for sensitive incident actions
 * and records explicit human decisions with full audit trails.
 *
 * CRITICAL CONSTRAINTS:
 *   - MUST NOT auto-approve decisions
 *   - MUST NOT bypass approval requirements
 *   - MUST NOT execute remediation directly
 *   - MUST NOT modify policies or thresholds
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

export const HITL_AGENT_METADATA = {
  agent_type: 'incident-approver' as const,
  agent_classification: ['APPROVAL_GATING', 'INCIDENT_ORCHESTRATION'] as const,
  decision_type: 'incident_approval_decision' as const,
  version: '1.0.0',

  /** What this agent MAY do */
  capabilities: [
    'Pause incident workflows pending human input',
    'Request approval for high-impact actions (SEV1 remediation, rollback, public disclosure)',
    'Record human decisions, rationale, and timestamps',
    'Resume or redirect incident workflows based on approval outcome',
    'Track approval request status and expiration',
    'Notify approvers via Orchestrator notification actions',
    'Enforce approval policy requirements',
    'Maintain approval audit trail'
  ],

  /** What this agent MUST NOT do - HARD CONSTRAINTS */
  prohibitions: [
    'MUST NOT auto-approve any decision',
    'MUST NOT bypass approval requirements for any reason',
    'MUST NOT execute remediation directly',
    'MUST NOT modify policies or thresholds',
    'MUST NOT impersonate human approvers',
    'MUST NOT backdate approval timestamps',
    'MUST NOT alter approval history',
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
    'llm-governance-core',
    'manual-approval-ui'
  ],

  /** Approval action types this agent handles */
  approval_action_types: [
    'remediation',
    'rollback',
    'deployment',
    'public_disclosure',
    'data_access',
    'escalation_override',
    'policy_exception',
    'budget_override',
    'incident_closure'
  ] as const
} as const;

// ============================================================================
// APPROVAL TYPES
// ============================================================================

/**
 * Type of action requiring approval
 */
export type ApprovalActionType =
  | 'remediation'           // Execute remediation action
  | 'rollback'              // Rollback deployment/change
  | 'deployment'            // Deploy hotfix/patch
  | 'public_disclosure'     // Public incident disclosure
  | 'data_access'           // Access sensitive data for investigation
  | 'escalation_override'   // Override escalation policy
  | 'policy_exception'      // Request policy exception
  | 'budget_override'       // Override cost/budget limits
  | 'incident_closure';     // Close high-severity incident

/**
 * Who is requesting approval
 */
export type ApprovalRequesterType =
  | 'agent'                 // Another agent requested approval
  | 'system'                // System-triggered approval request
  | 'user';                 // Human user initiated request

/**
 * Current status of approval request
 */
export type ApprovalRequestStatus =
  | 'pending'               // Awaiting human decision
  | 'approved'              // Approved by human
  | 'rejected'              // Rejected by human
  | 'expired'               // Request expired without decision
  | 'escalated'             // Escalated to higher authority
  | 'cancelled';            // Cancelled by requester

/**
 * Who can approve this request
 */
export type ApproverType =
  | 'incident_commander'    // Incident commander role
  | 'on_call_lead'          // On-call lead
  | 'security_team'         // Security team
  | 'sre_lead'              // SRE lead
  | 'engineering_manager'   // Engineering manager
  | 'executive'             // Executive level (VP+)
  | 'specific_user';        // Specific named user

// ============================================================================
// INPUT SCHEMAS
// ============================================================================

/**
 * Input to the Human-in-the-Loop Agent
 */
export interface HITLAgentInput {
  // ==================== REQUEST IDENTIFICATION ====================

  /** Unique approval request ID */
  approval_request_id: UUID;

  /** Associated incident ID */
  incident_id: UUID;

  /** External/source incident ID (if different) */
  external_incident_id?: string;

  // ==================== ACTION DETAILS ====================

  /** Type of action requiring approval */
  action_type: ApprovalActionType;

  /** Human-readable description of the action */
  action_description: string;

  /** Detailed parameters of the action */
  action_parameters: ActionParameters;

  /** Expected impact of the action */
  expected_impact: ImpactAssessment;

  /** Risks associated with NOT taking the action */
  inaction_risks: string[];

  // ==================== INCIDENT CONTEXT ====================

  /** Current incident severity */
  incident_severity: Severity;

  /** Current incident status */
  incident_status: IncidentStatus;

  /** Incident category */
  incident_category: IncidentCategory;

  /** Affected resource */
  affected_resource: AffectedResource;

  /** Incident title */
  incident_title: string;

  /** Brief incident summary */
  incident_summary: string;

  // ==================== REQUESTER CONTEXT ====================

  /** Who/what is requesting approval */
  requester_type: ApprovalRequesterType;

  /** Requester identifier (agent ID, user ID, or system component) */
  requester_id: string;

  /** Reason for the approval request */
  request_reason: string;

  /** Priority of the approval request */
  request_priority: 'critical' | 'high' | 'normal' | 'low';

  // ==================== APPROVAL REQUIREMENTS ====================

  /** Required approver types */
  required_approvers: ApproverRequirement[];

  /** Minimum number of approvals required */
  min_approvals_required: number;

  /** Maximum time to wait for approval (ISO8601 duration or timestamp) */
  approval_deadline: ISO8601Timestamp;

  /** What happens if deadline passes without approval */
  expiry_action: 'auto_reject' | 'escalate' | 'notify_and_wait';

  // ==================== EXISTING APPROVALS ====================

  /** Current status of this approval request */
  current_status: ApprovalRequestStatus;

  /** Approvals already recorded */
  existing_approvals: ApprovalRecord[];

  /** Rejections already recorded */
  existing_rejections: ApprovalRecord[];

  // ==================== POLICY CONTEXT ====================

  /** Approval policy ID being applied */
  policy_id?: UUID;

  /** Policy version */
  policy_version?: string;

  /** Escalation level that triggered this approval */
  escalation_level?: number;

  // ==================== EXECUTION CONTEXT ====================

  /** Environment */
  environment: Environment;

  /** Execution correlation ID */
  execution_id: UUID;

  /** Request timestamp */
  request_timestamp: ISO8601Timestamp;

  /** Trace ID for distributed tracing */
  trace_id?: string;
}

export interface ActionParameters {
  /** Target of the action (service, endpoint, etc.) */
  target: string;

  /** Target type */
  target_type: 'service' | 'endpoint' | 'model' | 'deployment' | 'config' | 'data';

  /** Action-specific parameters */
  parameters: Record<string, unknown>;

  /** Is this action reversible? */
  reversible: boolean;

  /** Estimated duration of the action */
  estimated_duration_seconds?: number;

  /** Resources that will be affected */
  affected_resources?: string[];
}

export interface ImpactAssessment {
  /** Scope of impact */
  scope: 'single_service' | 'multiple_services' | 'tenant' | 'region' | 'global';

  /** Estimated number of users affected */
  users_affected_estimate?: number;

  /** Revenue impact estimate (if applicable) */
  revenue_impact_estimate?: string;

  /** Compliance implications */
  compliance_implications?: string[];

  /** Data sensitivity level */
  data_sensitivity?: 'public' | 'internal' | 'confidential' | 'restricted';

  /** Free-form impact description */
  description: string;
}

export interface ApproverRequirement {
  /** Type of approver required */
  approver_type: ApproverType;

  /** Specific user/team ID if approver_type is 'specific_user' */
  approver_id?: string;

  /** Is this approver required or optional? */
  required: boolean;

  /** Order in approval chain (lower = earlier) */
  order?: number;
}

export interface ApprovalRecord {
  /** Approver identifier */
  approver_id: string;

  /** Approver display name */
  approver_name: string;

  /** Approver type/role */
  approver_type: ApproverType;

  /** Decision: approved or rejected */
  decision: 'approved' | 'rejected';

  /** Reason/rationale for decision */
  rationale: string;

  /** Conditions attached to approval (if any) */
  conditions?: string[];

  /** When the decision was made */
  decision_timestamp: ISO8601Timestamp;

  /** Signature or verification token */
  signature?: string;
}

// ============================================================================
// OUTPUT SCHEMAS
// ============================================================================

/**
 * HITL Agent decision type
 */
export type HITLDecision =
  | 'approval_pending'      // Still waiting for approvals
  | 'approved'              // All required approvals obtained
  | 'rejected'              // Approval rejected
  | 'expired'               // Deadline passed
  | 'escalated'             // Escalated to higher authority
  | 'cancelled';            // Request cancelled

/**
 * Output of the Human-in-the-Loop Agent
 */
export interface HITLAgentOutput {
  // ==================== DECISION ====================

  /** The approval decision */
  decision: HITLDecision;

  /** Decision reason/summary */
  reason: string;

  // ==================== REQUEST STATUS ====================

  /** Updated approval request status */
  updated_status: ApprovalRequestStatus;

  /** Number of approvals obtained */
  approvals_obtained: number;

  /** Number of approvals still required */
  approvals_remaining: number;

  /** Number of rejections */
  rejections_count: number;

  // ==================== APPROVAL DETAILS ====================

  /** All approval records (including any new ones from this invocation) */
  approval_records: ApprovalRecord[];

  /** Consolidated conditions from all approvals */
  consolidated_conditions: string[];

  /** Final approver for audit trail (if decision is approved/rejected) */
  final_approver?: ApproverInfo;

  // ==================== WORKFLOW ACTIONS ====================

  /** Actions to be taken by Orchestrator */
  orchestrator_actions: HITLOrchestratorAction[];

  /** Whether the original action is now authorized to proceed */
  action_authorized: boolean;

  /** Modified action parameters (if conditions require changes) */
  modified_action_parameters?: Partial<ActionParameters>;

  // ==================== TIMING ====================

  /** When the approval was finalized (if decision is terminal) */
  decision_finalized_at?: ISO8601Timestamp;

  /** Next evaluation time (if decision is pending) */
  next_check_at?: ISO8601Timestamp;

  /** Time remaining before expiry */
  time_remaining_seconds?: number;

  // ==================== AUDIT ====================

  /** Complete audit trail for this approval process */
  audit_trail: AuditTrailEntry[];

  /** Policy compliance status */
  policy_compliance: PolicyComplianceStatus;
}

export interface ApproverInfo {
  /** Approver ID */
  id: string;

  /** Approver name */
  name: string;

  /** Approver role/type */
  type: ApproverType;

  /** Email (for audit) */
  email?: string;
}

export interface HITLOrchestratorAction {
  /** Action type */
  action_type:
    | 'notify_approvers'            // Request approval from approvers
    | 'notify_requester'            // Notify requester of status change
    | 'execute_approved_action'     // Trigger the approved action
    | 'escalate_approval'           // Escalate to higher authority
    | 'cancel_pending_action'       // Cancel the action that was pending approval
    | 'log_approval_event'          // Log to timeline
    | 'update_incident_status';     // Update incident status

  /** Action priority */
  priority: 'critical' | 'high' | 'normal' | 'low';

  /** Action parameters */
  parameters: Record<string, unknown>;

  /** Whether action should be async */
  async: boolean;
}

export interface AuditTrailEntry {
  /** Timestamp of the event */
  timestamp: ISO8601Timestamp;

  /** Type of audit event */
  event_type:
    | 'request_created'
    | 'approval_requested'
    | 'approval_received'
    | 'rejection_received'
    | 'escalation_triggered'
    | 'deadline_warning'
    | 'request_expired'
    | 'request_cancelled'
    | 'decision_finalized';

  /** Actor who triggered this event */
  actor_id: string;

  /** Actor type */
  actor_type: 'agent' | 'user' | 'system';

  /** Event details */
  details: Record<string, unknown>;
}

export interface PolicyComplianceStatus {
  /** Whether the approval process complied with policy */
  compliant: boolean;

  /** Policy ID that was evaluated */
  policy_id: UUID;

  /** Policy name */
  policy_name: string;

  /** Specific policy rules that were evaluated */
  rules_evaluated: PolicyRuleResult[];

  /** Any policy violations */
  violations: string[];
}

export interface PolicyRuleResult {
  /** Rule ID */
  rule_id: string;

  /** Rule name */
  rule_name: string;

  /** Was the rule satisfied? */
  satisfied: boolean;

  /** Rule details */
  details?: string;
}

// ============================================================================
// DECISION EVENT TYPE
// ============================================================================

/**
 * DecisionEvent type for HITL Agent
 */
export type HITLDecisionEvent = DecisionEvent<HITLAgentOutput>;

// ============================================================================
// CONFIDENCE SEMANTICS
// ============================================================================

/**
 * Confidence factors for HITL decisions
 *
 * confidence represents:
 * - For 'approved': Confidence that all policy requirements are met
 * - For 'rejected': Confidence that rejection is final (no appeals pending)
 * - For 'pending': Likelihood of eventual approval based on current state
 * - For 'expired': Always 1.0 (deadline is deterministic)
 */
export const HITL_CONFIDENCE_FACTORS = {
  /** All required approvals obtained */
  all_approvals_obtained: { factor: 'all_approvals_obtained', weight: 0.4 },

  /** No rejections present */
  no_rejections: { factor: 'no_rejections', weight: 0.2 },

  /** Policy compliance verified */
  policy_compliant: { factor: 'policy_compliant', weight: 0.2 },

  /** Approver authority verified */
  authority_verified: { factor: 'authority_verified', weight: 0.1 },

  /** Conditions are actionable */
  conditions_valid: { factor: 'conditions_valid', weight: 0.1 }
} as const;

// ============================================================================
// PERSISTENCE SPECIFICATION
// ============================================================================

/**
 * What to persist for HITL decisions
 */
export const HITL_PERSISTENCE: PersistenceSpec = {
  persist: [
    // Request identification
    'approval_request_id',
    'incident_id',
    'external_incident_id',
    // Action details
    'action_type',
    'action_description',
    // Incident context
    'incident_severity',
    'incident_category',
    // Requester
    'requester_type',
    'requester_id',
    'request_reason',
    // Decision output
    'decision',
    'updated_status',
    'reason',
    // Approval records (CRITICAL for audit)
    'approval_records',
    'rejections_count',
    'final_approver',
    // Conditions
    'consolidated_conditions',
    // Authorization
    'action_authorized',
    // Audit trail (CRITICAL)
    'audit_trail',
    // Policy compliance
    'policy_compliance',
    // Timestamps
    'request_timestamp',
    'decision_finalized_at'
  ],
  exclude: [
    // Full action parameters (stored in incident record)
    'action_parameters.parameters',
    // Transient execution context
    'execution_id',
    'trace_id',
    // Detailed impact (stored in incident)
    'expected_impact',
    'inaction_risks',
    // Full incident details (stored separately)
    'incident_title',
    'incident_summary',
    // Orchestrator actions (sent to orchestrator, not persisted here)
    'orchestrator_actions',
    // Computed timing fields
    'next_check_at',
    'time_remaining_seconds'
  ],
  ttl_seconds: 0 // Permanent audit record - approvals NEVER expire from audit
};

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Validate HITL agent input
 */
export function validateHITLInput(input: unknown): ValidationResult {
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

  if (!i.approval_request_id) {
    errors.push({ field: 'approval_request_id', message: 'approval_request_id is required', code: 'REQUIRED' });
  }

  if (!i.incident_id) {
    errors.push({ field: 'incident_id', message: 'incident_id is required', code: 'REQUIRED' });
  }

  if (!i.action_type) {
    errors.push({ field: 'action_type', message: 'action_type is required', code: 'REQUIRED' });
  }

  if (!i.action_description) {
    errors.push({ field: 'action_description', message: 'action_description is required', code: 'REQUIRED' });
  }

  if (!i.incident_severity) {
    errors.push({ field: 'incident_severity', message: 'incident_severity is required', code: 'REQUIRED' });
  }

  if (!i.requester_type) {
    errors.push({ field: 'requester_type', message: 'requester_type is required', code: 'REQUIRED' });
  }

  if (!i.requester_id) {
    errors.push({ field: 'requester_id', message: 'requester_id is required', code: 'REQUIRED' });
  }

  if (!i.approval_deadline) {
    errors.push({ field: 'approval_deadline', message: 'approval_deadline is required', code: 'REQUIRED' });
  }

  if (!i.current_status) {
    errors.push({ field: 'current_status', message: 'current_status is required', code: 'REQUIRED' });
  }

  if (!i.execution_id) {
    errors.push({ field: 'execution_id', message: 'execution_id is required', code: 'REQUIRED' });
  }

  if (typeof i.min_approvals_required !== 'number') {
    errors.push({ field: 'min_approvals_required', message: 'min_approvals_required must be a number', code: 'INVALID_TYPE' });
  }

  // ==================== TYPE VALIDATIONS ====================

  // Validate action_type
  const validActionTypes = [
    'remediation', 'rollback', 'deployment', 'public_disclosure',
    'data_access', 'escalation_override', 'policy_exception',
    'budget_override', 'incident_closure'
  ];
  if (i.action_type && !validActionTypes.includes(i.action_type as string)) {
    errors.push({
      field: 'action_type',
      message: `action_type must be one of: ${validActionTypes.join(', ')}`,
      code: 'INVALID_VALUE',
      value: i.action_type
    });
  }

  // Validate severity
  const validSeverities = ['P0', 'P1', 'P2', 'P3', 'P4'];
  if (i.incident_severity && !validSeverities.includes(i.incident_severity as string)) {
    errors.push({
      field: 'incident_severity',
      message: `incident_severity must be one of: ${validSeverities.join(', ')}`,
      code: 'INVALID_VALUE',
      value: i.incident_severity
    });
  }

  // Validate requester_type
  const validRequesterTypes = ['agent', 'system', 'user'];
  if (i.requester_type && !validRequesterTypes.includes(i.requester_type as string)) {
    errors.push({
      field: 'requester_type',
      message: `requester_type must be one of: ${validRequesterTypes.join(', ')}`,
      code: 'INVALID_VALUE',
      value: i.requester_type
    });
  }

  // Validate current_status
  const validStatuses = ['pending', 'approved', 'rejected', 'expired', 'escalated', 'cancelled'];
  if (i.current_status && !validStatuses.includes(i.current_status as string)) {
    errors.push({
      field: 'current_status',
      message: `current_status must be one of: ${validStatuses.join(', ')}`,
      code: 'INVALID_VALUE',
      value: i.current_status
    });
  }

  // ==================== OBJECT VALIDATIONS ====================

  // Validate action_parameters if present
  if (i.action_parameters) {
    const ap = i.action_parameters as Record<string, unknown>;
    if (!ap.target) {
      errors.push({ field: 'action_parameters.target', message: 'action_parameters.target is required', code: 'REQUIRED' });
    }
    if (!ap.target_type) {
      errors.push({ field: 'action_parameters.target_type', message: 'action_parameters.target_type is required', code: 'REQUIRED' });
    }
    if (typeof ap.reversible !== 'boolean') {
      warnings.push({ field: 'action_parameters.reversible', message: 'action_parameters.reversible should be specified', code: 'RECOMMENDED' });
    }
  } else {
    errors.push({ field: 'action_parameters', message: 'action_parameters is required', code: 'REQUIRED' });
  }

  // Validate required_approvers if present
  if (i.required_approvers && Array.isArray(i.required_approvers)) {
    const approvers = i.required_approvers as Record<string, unknown>[];
    if (approvers.length === 0) {
      errors.push({ field: 'required_approvers', message: 'required_approvers cannot be empty', code: 'INVALID_VALUE' });
    }
    approvers.forEach((approver, idx) => {
      if (!approver.approver_type) {
        errors.push({
          field: `required_approvers[${idx}].approver_type`,
          message: 'approver_type is required',
          code: 'REQUIRED'
        });
      }
    });
  } else {
    errors.push({ field: 'required_approvers', message: 'required_approvers is required and must be an array', code: 'REQUIRED' });
  }

  // ==================== LOGICAL VALIDATIONS ====================

  // Check min_approvals_required doesn't exceed required_approvers count
  if (
    typeof i.min_approvals_required === 'number' &&
    Array.isArray(i.required_approvers) &&
    i.min_approvals_required > (i.required_approvers as unknown[]).length
  ) {
    warnings.push({
      field: 'min_approvals_required',
      message: 'min_approvals_required exceeds number of required_approvers',
      code: 'LOGICAL_WARNING'
    });
  }

  // Check deadline is in the future (warning only)
  if (i.approval_deadline) {
    const deadline = new Date(i.approval_deadline as string);
    if (deadline < new Date()) {
      warnings.push({
        field: 'approval_deadline',
        message: 'approval_deadline is in the past',
        code: 'DEADLINE_PASSED'
      });
    }
  }

  return { valid: errors.length === 0, errors, warnings };
}

/**
 * Validate HITL agent output
 */
export function validateHITLOutput(output: unknown): ValidationResult {
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

  if (!o.decision) {
    errors.push({ field: 'decision', message: 'decision is required', code: 'REQUIRED' });
  }

  if (!o.reason) {
    errors.push({ field: 'reason', message: 'reason is required', code: 'REQUIRED' });
  }

  if (!o.updated_status) {
    errors.push({ field: 'updated_status', message: 'updated_status is required', code: 'REQUIRED' });
  }

  if (typeof o.approvals_obtained !== 'number') {
    errors.push({ field: 'approvals_obtained', message: 'approvals_obtained must be a number', code: 'INVALID_TYPE' });
  }

  if (typeof o.approvals_remaining !== 'number') {
    errors.push({ field: 'approvals_remaining', message: 'approvals_remaining must be a number', code: 'INVALID_TYPE' });
  }

  if (typeof o.action_authorized !== 'boolean') {
    errors.push({ field: 'action_authorized', message: 'action_authorized must be a boolean', code: 'INVALID_TYPE' });
  }

  if (!Array.isArray(o.approval_records)) {
    errors.push({ field: 'approval_records', message: 'approval_records must be an array', code: 'INVALID_TYPE' });
  }

  if (!Array.isArray(o.orchestrator_actions)) {
    errors.push({ field: 'orchestrator_actions', message: 'orchestrator_actions must be an array', code: 'INVALID_TYPE' });
  }

  if (!Array.isArray(o.audit_trail)) {
    errors.push({ field: 'audit_trail', message: 'audit_trail must be an array', code: 'INVALID_TYPE' });
  }

  if (!o.policy_compliance) {
    errors.push({ field: 'policy_compliance', message: 'policy_compliance is required', code: 'REQUIRED' });
  }

  // ==================== TYPE VALIDATIONS ====================

  // Validate decision
  const validDecisions = ['approval_pending', 'approved', 'rejected', 'expired', 'escalated', 'cancelled'];
  if (o.decision && !validDecisions.includes(o.decision as string)) {
    errors.push({
      field: 'decision',
      message: `decision must be one of: ${validDecisions.join(', ')}`,
      code: 'INVALID_VALUE',
      value: o.decision
    });
  }

  // Validate updated_status
  const validStatuses = ['pending', 'approved', 'rejected', 'expired', 'escalated', 'cancelled'];
  if (o.updated_status && !validStatuses.includes(o.updated_status as string)) {
    errors.push({
      field: 'updated_status',
      message: `updated_status must be one of: ${validStatuses.join(', ')}`,
      code: 'INVALID_VALUE',
      value: o.updated_status
    });
  }

  // ==================== LOGICAL VALIDATIONS ====================

  // If decision is 'approved', action_authorized should be true
  if (o.decision === 'approved' && o.action_authorized !== true) {
    errors.push({
      field: 'action_authorized',
      message: 'action_authorized must be true when decision is approved',
      code: 'LOGICAL_ERROR'
    });
  }

  // If decision is 'rejected', action_authorized should be false
  if (o.decision === 'rejected' && o.action_authorized !== false) {
    errors.push({
      field: 'action_authorized',
      message: 'action_authorized must be false when decision is rejected',
      code: 'LOGICAL_ERROR'
    });
  }

  // If decision is terminal, decision_finalized_at should be present
  const terminalDecisions = ['approved', 'rejected', 'expired', 'cancelled'];
  if (terminalDecisions.includes(o.decision as string) && !o.decision_finalized_at) {
    warnings.push({
      field: 'decision_finalized_at',
      message: 'decision_finalized_at should be provided for terminal decisions',
      code: 'RECOMMENDED'
    });
  }

  // If decision is pending, next_check_at should be present
  if (o.decision === 'approval_pending' && !o.next_check_at) {
    warnings.push({
      field: 'next_check_at',
      message: 'next_check_at should be provided when decision is pending',
      code: 'RECOMMENDED'
    });
  }

  return { valid: errors.length === 0, errors, warnings };
}

// ============================================================================
// CLI CONTRACT
// ============================================================================

/**
 * CLI invocation shape for the HITL Agent
 */
export interface HITLAgentCLI {
  /** Command name */
  command: 'approve';

  /** Subcommands */
  subcommands: {
    /** Request approval for an action */
    request: {
      args: {
        incident_id: string;
        action_type: ApprovalActionType;
      };
      flags: {
        '--action-description': string;
        '--target': string;
        '--priority'?: 'critical' | 'high' | 'normal' | 'low';
        '--deadline'?: string;
        '--policy-id'?: string;
        '--json'?: boolean;
      };
    };

    /** Record an approval decision */
    decide: {
      args: {
        approval_request_id: string;
        decision: 'approve' | 'reject';
      };
      flags: {
        '--approver-id': string;
        '--rationale': string;
        '--conditions'?: string[];
        '--json'?: boolean;
      };
    };

    /** Check approval status */
    status: {
      args: {
        approval_request_id: string;
      };
      flags: {
        '--verbose'?: boolean;
        '--json'?: boolean;
      };
    };

    /** List pending approvals */
    list: {
      args: {};
      flags: {
        '--incident-id'?: string;
        '--action-type'?: ApprovalActionType;
        '--status'?: ApprovalRequestStatus;
        '--approver-type'?: ApproverType;
        '--limit'?: number;
        '--json'?: boolean;
      };
    };

    /** Cancel an approval request */
    cancel: {
      args: {
        approval_request_id: string;
      };
      flags: {
        '--reason': string;
        '--force'?: boolean;
        '--json'?: boolean;
      };
    };

    /** Escalate an approval request */
    escalate: {
      args: {
        approval_request_id: string;
      };
      flags: {
        '--reason': string;
        '--target-approver-type'?: ApproverType;
        '--json'?: boolean;
      };
    };

    /** Inspect full approval audit trail */
    inspect: {
      args: {
        approval_request_id: string;
      };
      flags: {
        '--include-timeline'?: boolean;
        '--include-policy'?: boolean;
        '--json'?: boolean;
      };
    };
  };
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Calculate approval progress percentage
 */
export function calculateApprovalProgress(
  approvalsObtained: number,
  minRequired: number
): number {
  if (minRequired <= 0) return 100;
  return Math.min(100, Math.round((approvalsObtained / minRequired) * 100));
}

/**
 * Check if approval deadline has passed
 */
export function isDeadlinePassed(deadline: ISO8601Timestamp): boolean {
  return new Date(deadline) < new Date();
}

/**
 * Generate a unique approval request ID
 * Uses Node.js crypto for server-side or Web Crypto API for browser
 */
export function generateApprovalRequestId(): UUID {
  // Use Node.js crypto.randomUUID if available
  if (typeof globalThis.crypto !== 'undefined' && typeof globalThis.crypto.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  // Fallback to manual UUID v4 generation
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

/**
 * Hash inputs for audit trail
 * Uses Web Crypto API or Node.js crypto
 */
export async function hashInputs(input: HITLAgentInput): Promise<string> {
  const data = JSON.stringify(input);

  // Use Web Crypto API if available
  if (typeof globalThis.crypto !== 'undefined' && globalThis.crypto.subtle) {
    const encoder = new globalThis.TextEncoder();
    const hashBuffer = await globalThis.crypto.subtle.digest('SHA-256', encoder.encode(data));
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
  }

  // Fallback to Node.js crypto
  try {
    const cryptoModule = await import('node:crypto');
    return cryptoModule.createHash('sha256').update(data).digest('hex');
  } catch {
    // Ultimate fallback - return a placeholder hash
    return 'hash-computation-unavailable';
  }
}
