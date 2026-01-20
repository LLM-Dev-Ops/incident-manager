/**
 * @agentics/contracts - Human-in-the-Loop (HITL) Agent Handler
 *
 * Edge Function handler for the HITL Agent.
 * This handler processes approval requests and manages human approval workflows.
 *
 * DEPLOYMENT: Google Cloud Edge Function
 * EXECUTION: Stateless, deterministic
 * PERSISTENCE: Via ruvector-service ONLY (no direct SQL)
 */

import type {
  UUID,
  ISO8601Timestamp,
  AgentId,
  Environment,
  AgentResult,
  TelemetryEvent
} from './common.js';

import type {
  DecisionEvent,
  DecisionConstraints,
  ConfidenceFactor
} from './decision-event.js';

import {
  DecisionEventBuilder
} from './decision-event.js';

import type {
  HITLAgentInput,
  HITLAgentOutput,
  HITLDecision,
  ApprovalRecord,
  HITLOrchestratorAction,
  AuditTrailEntry,
  PolicyComplianceStatus,
  ApproverInfo,
  ApprovalRequestStatus
} from './hitl-agent.js';

import {
  HITL_AGENT_METADATA,
  HITL_CONFIDENCE_FACTORS,
  validateHITLInput,
  validateHITLOutput,
  isDeadlinePassed,
  hashInputs
} from './hitl-agent.js';

// ============================================================================
// CONSTANTS
// ============================================================================

const AGENT_ID_PREFIX = 'incident-approver';
const INSTANCE_ID = (typeof process !== 'undefined' && process.env?.INSTANCE_ID) || 'default';

/**
 * Build the full agent ID
 */
function buildAgentId(): AgentId {
  return `${AGENT_ID_PREFIX}:${HITL_AGENT_METADATA.version}:${INSTANCE_ID}`;
}

// ============================================================================
// RUVECTOR SERVICE CLIENT INTERFACE
// ============================================================================

/**
 * Interface for ruvector-service client
 * All persistence MUST go through this interface - NO DIRECT SQL ACCESS
 */
export interface RuVectorClient {
  /**
   * Persist a DecisionEvent
   */
  persistDecisionEvent(event: DecisionEvent<HITLAgentOutput>): Promise<void>;

  /**
   * Get existing approval request state
   */
  getApprovalRequest(requestId: UUID): Promise<ApprovalRequestState | null>;

  /**
   * Update approval request state
   */
  updateApprovalRequest(requestId: UUID, state: ApprovalRequestState): Promise<void>;

  /**
   * Emit telemetry event to LLM-Observatory
   */
  emitTelemetry(event: TelemetryEvent): Promise<void>;
}

/**
 * Approval request state stored in ruvector-service
 */
export interface ApprovalRequestState {
  request_id: UUID;
  incident_id: UUID;
  status: ApprovalRequestStatus;
  approvals: ApprovalRecord[];
  rejections: ApprovalRecord[];
  created_at: ISO8601Timestamp;
  updated_at: ISO8601Timestamp;
  deadline: ISO8601Timestamp;
  audit_trail: AuditTrailEntry[];
}

// ============================================================================
// HANDLER CONFIGURATION
// ============================================================================

export interface HITLHandlerConfig {
  /** ruvector-service client */
  ruvectorClient: RuVectorClient;

  /** Environment */
  environment: Environment;

  /** Region */
  region?: string;

  /** Default deadline if not specified (seconds) */
  defaultDeadlineSeconds?: number;

  /** Enable strict policy enforcement */
  strictPolicyEnforcement?: boolean;
}

// ============================================================================
// CORE HANDLER
// ============================================================================

/**
 * Human-in-the-Loop Agent Handler
 *
 * This is the main entry point for the Edge Function.
 * It processes approval requests and emits DecisionEvents.
 *
 * IMPORTANT CONSTRAINTS:
 * - This handler MUST NOT auto-approve any decision
 * - This handler MUST NOT bypass approval requirements
 * - This handler MUST NOT execute remediation directly
 * - This handler MUST NOT modify policies or thresholds
 */
export async function handleHITLRequest(
  input: unknown,
  config: HITLHandlerConfig
): Promise<AgentResult<HITLAgentOutput>> {
  const startTime = Date.now();
  const agentId = buildAgentId();

  // ==================== INPUT VALIDATION ====================

  const validation = validateHITLInput(input);
  if (!validation.valid) {
    await emitErrorTelemetry(config, agentId, 'validation_error', validation.errors);
    return {
      success: false,
      error: {
        code: 'VALIDATION_ERROR',
        message: `Input validation failed: ${validation.errors.map(e => e.message).join(', ')}`,
        details: { errors: validation.errors },
        retryable: false
      }
    };
  }

  const typedInput = input as HITLAgentInput;

  // ==================== HASH INPUTS FOR AUDIT ====================

  let inputsHash: string;
  try {
    inputsHash = await hashInputs(typedInput);
  } catch (err) {
    inputsHash = 'hash-computation-failed';
  }

  // ==================== CORE DECISION LOGIC ====================

  try {
    // Get existing state if this is a follow-up invocation
    const _existingState = await config.ruvectorClient.getApprovalRequest(
      typedInput.approval_request_id
    );

    // Compute the decision
    const decision = computeApprovalDecision(typedInput, _existingState);

    // Build orchestrator actions
    const orchestratorActions = buildOrchestratorActions(typedInput, decision);

    // Build audit trail entry for this invocation
    const auditEntry = buildAuditEntry(typedInput, decision);

    // Merge audit trails
    const fullAuditTrail = [
      ...(_existingState?.audit_trail || []),
      ...(typedInput.existing_approvals.length > 0 || typedInput.existing_rejections.length > 0
        ? [] // Don't duplicate existing entries
        : []),
      auditEntry
    ];

    // Build policy compliance status
    const policyCompliance = evaluatePolicyCompliance(typedInput, decision);

    // Calculate confidence
    const { confidence, factors } = calculateConfidence(typedInput, decision, policyCompliance);

    // Build output
    const output: HITLAgentOutput = {
      decision: decision.decision,
      reason: decision.reason,
      updated_status: decision.updatedStatus,
      approvals_obtained: countApprovals(typedInput),
      approvals_remaining: Math.max(0, typedInput.min_approvals_required - countApprovals(typedInput)),
      rejections_count: typedInput.existing_rejections.length,
      approval_records: [...typedInput.existing_approvals, ...typedInput.existing_rejections],
      consolidated_conditions: extractConditions(typedInput.existing_approvals),
      final_approver: decision.finalApprover,
      orchestrator_actions: orchestratorActions,
      action_authorized: decision.decision === 'approved',
      modified_action_parameters: decision.modifiedParams,
      decision_finalized_at: isTerminalDecision(decision.decision) ? new Date().toISOString() : undefined,
      next_check_at: decision.decision === 'approval_pending' ? computeNextCheckTime(typedInput) : undefined,
      time_remaining_seconds: computeTimeRemaining(typedInput.approval_deadline),
      audit_trail: fullAuditTrail,
      policy_compliance: policyCompliance
    };

    // ==================== VALIDATE OUTPUT ====================

    const outputValidation = validateHITLOutput(output);
    if (!outputValidation.valid) {
      await emitErrorTelemetry(config, agentId, 'output_validation_error', outputValidation.errors);
      return {
        success: false,
        error: {
          code: 'OUTPUT_VALIDATION_ERROR',
          message: `Output validation failed: ${outputValidation.errors.map(e => e.message).join(', ')}`,
          details: { errors: outputValidation.errors },
          retryable: true
        }
      };
    }

    // ==================== BUILD DECISION EVENT ====================

    const constraints = buildConstraints(typedInput, decision);

    const eventId = typeof globalThis.crypto !== 'undefined' && typeof globalThis.crypto.randomUUID === 'function'
      ? globalThis.crypto.randomUUID()
      : `event-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

    const decisionEvent = new DecisionEventBuilder<HITLAgentOutput>()
      .withId(eventId)
      .withAgent(
        agentId,
        HITL_AGENT_METADATA.version,
        'APPROVAL_GATING'
      )
      .withDecision(
        HITL_AGENT_METADATA.decision_type,
        output,
        inputsHash
      )
      .withConfidence(confidence, factors)
      .withConstraints(constraints)
      .withExecutionContext(
        typedInput.execution_id,
        config.environment,
        config.region
      )
      .requiresReview(isTerminalDecision(decision.decision) ? undefined : typedInput.approval_deadline)
      .withAuditMetadata({
        sources: [typedInput.requester_type],
        policies_evaluated: typedInput.policy_id ? [typedInput.policy_id] : [],
        manual_overrides: undefined
      })
      .build();

    // ==================== PERSIST DECISION EVENT ====================

    await config.ruvectorClient.persistDecisionEvent(decisionEvent);

    // ==================== UPDATE APPROVAL STATE ====================

    await config.ruvectorClient.updateApprovalRequest(typedInput.approval_request_id, {
      request_id: typedInput.approval_request_id,
      incident_id: typedInput.incident_id,
      status: output.updated_status,
      approvals: typedInput.existing_approvals,
      rejections: typedInput.existing_rejections,
      created_at: typedInput.request_timestamp,
      updated_at: new Date().toISOString(),
      deadline: typedInput.approval_deadline,
      audit_trail: fullAuditTrail
    });

    // ==================== EMIT TELEMETRY ====================

    await emitSuccessTelemetry(config, agentId, typedInput, output, Date.now() - startTime);

    return {
      success: true,
      data: output,
      warnings: outputValidation.warnings
    };

  } catch (err) {
    const error = err instanceof Error ? err : new Error(String(err));
    await emitErrorTelemetry(config, agentId, 'execution_error', { message: error.message });

    return {
      success: false,
      error: {
        code: 'EXECUTION_ERROR',
        message: error.message,
        details: { stack: error.stack },
        retryable: true
      }
    };
  }
}

// ============================================================================
// DECISION LOGIC
// ============================================================================

interface ApprovalDecisionResult {
  decision: HITLDecision;
  reason: string;
  updatedStatus: ApprovalRequestStatus;
  finalApprover?: ApproverInfo;
  modifiedParams?: Record<string, unknown>;
}

/**
 * Core decision logic for approval evaluation
 *
 * CRITICAL: This function MUST NOT auto-approve
 * It only evaluates the current state and determines the appropriate decision
 */
function computeApprovalDecision(
  input: HITLAgentInput,
  _existingState: ApprovalRequestState | null
): ApprovalDecisionResult {
  // Check if already in a terminal state
  if (['approved', 'rejected', 'expired', 'cancelled'].includes(input.current_status)) {
    return {
      decision: input.current_status as HITLDecision,
      reason: `Request is already in terminal state: ${input.current_status}`,
      updatedStatus: input.current_status
    };
  }

  // Check if deadline has passed
  if (isDeadlinePassed(input.approval_deadline)) {
    const expiryDecision = handleExpiry(input);
    return expiryDecision;
  }

  // Check for rejections
  if (input.existing_rejections.length > 0) {
    const latestRejection = input.existing_rejections[input.existing_rejections.length - 1];
    return {
      decision: 'rejected',
      reason: `Request rejected by ${latestRejection.approver_name}: ${latestRejection.rationale}`,
      updatedStatus: 'rejected',
      finalApprover: {
        id: latestRejection.approver_id,
        name: latestRejection.approver_name,
        type: latestRejection.approver_type
      }
    };
  }

  // Count approvals
  const approvalsObtained = countApprovals(input);

  // Check if we have enough approvals
  if (approvalsObtained >= input.min_approvals_required) {
    // Verify all required approvers have approved
    const allRequiredSatisfied = checkRequiredApprovers(input);

    if (allRequiredSatisfied) {
      const latestApproval = input.existing_approvals[input.existing_approvals.length - 1];
      return {
        decision: 'approved',
        reason: `All required approvals obtained (${approvalsObtained}/${input.min_approvals_required})`,
        updatedStatus: 'approved',
        finalApprover: {
          id: latestApproval.approver_id,
          name: latestApproval.approver_name,
          type: latestApproval.approver_type
        },
        modifiedParams: consolidateConditionsToParams(input.existing_approvals)
      };
    }
  }

  // Still pending
  return {
    decision: 'approval_pending',
    reason: `Awaiting approvals: ${approvalsObtained}/${input.min_approvals_required} obtained`,
    updatedStatus: 'pending'
  };
}

/**
 * Handle expiry based on expiry_action setting
 */
function handleExpiry(input: HITLAgentInput): ApprovalDecisionResult {
  switch (input.expiry_action) {
    case 'auto_reject':
      return {
        decision: 'expired',
        reason: 'Approval deadline passed without sufficient approvals',
        updatedStatus: 'expired'
      };

    case 'escalate':
      return {
        decision: 'escalated',
        reason: 'Approval deadline passed, escalating to higher authority',
        updatedStatus: 'escalated'
      };

    case 'notify_and_wait':
      return {
        decision: 'approval_pending',
        reason: 'Approval deadline passed but configured to wait. Notifications sent.',
        updatedStatus: 'pending'
      };

    default:
      return {
        decision: 'expired',
        reason: 'Approval deadline passed',
        updatedStatus: 'expired'
      };
  }
}

/**
 * Count valid approvals
 */
function countApprovals(input: HITLAgentInput): number {
  return input.existing_approvals.filter(a => a.decision === 'approved').length;
}

/**
 * Check if all required approvers have approved
 */
function checkRequiredApprovers(input: HITLAgentInput): boolean {
  const requiredApprovers = input.required_approvers.filter(a => a.required);

  for (const required of requiredApprovers) {
    const hasApproval = input.existing_approvals.some(approval => {
      if (required.approver_type === 'specific_user') {
        return approval.approver_id === required.approver_id && approval.decision === 'approved';
      }
      return approval.approver_type === required.approver_type && approval.decision === 'approved';
    });

    if (!hasApproval) {
      return false;
    }
  }

  return true;
}

/**
 * Extract conditions from all approvals
 */
function extractConditions(approvals: ApprovalRecord[]): string[] {
  const conditions: string[] = [];
  for (const approval of approvals) {
    if (approval.conditions) {
      conditions.push(...approval.conditions);
    }
  }
  return [...new Set(conditions)]; // Deduplicate
}

/**
 * Convert conditions to modified action parameters
 */
function consolidateConditionsToParams(approvals: ApprovalRecord[]): Record<string, unknown> | undefined {
  const conditions = extractConditions(approvals);
  if (conditions.length === 0) return undefined;

  return {
    approval_conditions: conditions,
    conditions_acknowledged: true
  };
}

/**
 * Check if decision is terminal
 */
function isTerminalDecision(decision: HITLDecision): boolean {
  return ['approved', 'rejected', 'expired', 'cancelled'].includes(decision);
}

/**
 * Compute next check time for pending requests
 */
function computeNextCheckTime(input: HITLAgentInput): ISO8601Timestamp {
  const deadline = new Date(input.approval_deadline);
  const now = new Date();
  const timeRemaining = deadline.getTime() - now.getTime();

  // Check every 5 minutes or half the remaining time, whichever is less
  const checkInterval = Math.min(5 * 60 * 1000, timeRemaining / 2);
  return new Date(now.getTime() + checkInterval).toISOString();
}

/**
 * Compute time remaining until deadline
 */
function computeTimeRemaining(deadline: ISO8601Timestamp): number {
  const remaining = new Date(deadline).getTime() - Date.now();
  return Math.max(0, Math.floor(remaining / 1000));
}

// ============================================================================
// ORCHESTRATOR ACTIONS
// ============================================================================

/**
 * Build orchestrator actions based on decision
 */
function buildOrchestratorActions(
  input: HITLAgentInput,
  decision: ApprovalDecisionResult
): HITLOrchestratorAction[] {
  const actions: HITLOrchestratorAction[] = [];

  switch (decision.decision) {
    case 'approved':
      // Execute the approved action
      actions.push({
        action_type: 'execute_approved_action',
        priority: input.request_priority === 'critical' ? 'critical' : 'high',
        parameters: {
          action_type: input.action_type,
          action_parameters: input.action_parameters,
          conditions: extractConditions(input.existing_approvals),
          approval_request_id: input.approval_request_id
        },
        async: true
      });

      // Notify requester
      actions.push({
        action_type: 'notify_requester',
        priority: 'normal',
        parameters: {
          requester_id: input.requester_id,
          message: `Action approved: ${input.action_description}`,
          approval_request_id: input.approval_request_id
        },
        async: true
      });

      // Log to timeline
      actions.push({
        action_type: 'log_approval_event',
        priority: 'low',
        parameters: {
          incident_id: input.incident_id,
          event: 'approval_granted',
          approvers: input.existing_approvals.map(a => a.approver_name)
        },
        async: true
      });
      break;

    case 'rejected':
      // Cancel the pending action
      actions.push({
        action_type: 'cancel_pending_action',
        priority: 'high',
        parameters: {
          action_type: input.action_type,
          approval_request_id: input.approval_request_id,
          reason: decision.reason
        },
        async: false
      });

      // Notify requester
      actions.push({
        action_type: 'notify_requester',
        priority: 'high',
        parameters: {
          requester_id: input.requester_id,
          message: `Action rejected: ${input.action_description}. Reason: ${decision.reason}`,
          approval_request_id: input.approval_request_id
        },
        async: true
      });

      // Log to timeline
      actions.push({
        action_type: 'log_approval_event',
        priority: 'low',
        parameters: {
          incident_id: input.incident_id,
          event: 'approval_rejected',
          rejector: decision.finalApprover?.name
        },
        async: true
      });
      break;

    case 'escalated':
      // Escalate to higher authority
      actions.push({
        action_type: 'escalate_approval',
        priority: 'critical',
        parameters: {
          approval_request_id: input.approval_request_id,
          incident_id: input.incident_id,
          reason: decision.reason,
          original_deadline: input.approval_deadline
        },
        async: false
      });

      // Notify current approvers
      actions.push({
        action_type: 'notify_approvers',
        priority: 'high',
        parameters: {
          message: `Approval request escalated: ${input.action_description}`,
          approval_request_id: input.approval_request_id
        },
        async: true
      });
      break;

    case 'expired':
      // Cancel the pending action
      actions.push({
        action_type: 'cancel_pending_action',
        priority: 'normal',
        parameters: {
          action_type: input.action_type,
          approval_request_id: input.approval_request_id,
          reason: 'Approval request expired'
        },
        async: false
      });

      // Notify requester
      actions.push({
        action_type: 'notify_requester',
        priority: 'normal',
        parameters: {
          requester_id: input.requester_id,
          message: `Approval request expired: ${input.action_description}`,
          approval_request_id: input.approval_request_id
        },
        async: true
      });
      break;

    case 'approval_pending':
      // Notify approvers if this is a new request or reminder
      actions.push({
        action_type: 'notify_approvers',
        priority: mapRequestPriority(input.request_priority),
        parameters: {
          required_approvers: input.required_approvers,
          action_description: input.action_description,
          incident_id: input.incident_id,
          incident_severity: input.incident_severity,
          deadline: input.approval_deadline,
          approval_request_id: input.approval_request_id
        },
        async: true
      });
      break;
  }

  return actions;
}

function mapRequestPriority(priority: string): 'critical' | 'high' | 'normal' | 'low' {
  switch (priority) {
    case 'critical': return 'critical';
    case 'high': return 'high';
    case 'normal': return 'normal';
    case 'low': return 'low';
    default: return 'normal';
  }
}

// ============================================================================
// AUDIT TRAIL
// ============================================================================

/**
 * Build audit trail entry for this invocation
 */
function buildAuditEntry(
  input: HITLAgentInput,
  decision: ApprovalDecisionResult
): AuditTrailEntry {
  let eventType: AuditTrailEntry['event_type'];

  switch (decision.decision) {
    case 'approved':
      eventType = 'decision_finalized';
      break;
    case 'rejected':
      eventType = 'rejection_received';
      break;
    case 'expired':
      eventType = 'request_expired';
      break;
    case 'escalated':
      eventType = 'escalation_triggered';
      break;
    case 'cancelled':
      eventType = 'request_cancelled';
      break;
    case 'approval_pending':
    default:
      eventType = 'approval_requested';
      break;
  }

  return {
    timestamp: new Date().toISOString(),
    event_type: eventType,
    actor_id: buildAgentId(),
    actor_type: 'agent',
    details: {
      decision: decision.decision,
      reason: decision.reason,
      approvals_count: countApprovals(input),
      required_count: input.min_approvals_required
    }
  };
}

// ============================================================================
// POLICY COMPLIANCE
// ============================================================================

/**
 * Evaluate policy compliance
 */
function evaluatePolicyCompliance(
  input: HITLAgentInput,
  decision: ApprovalDecisionResult
): PolicyComplianceStatus {
  const rules: PolicyComplianceStatus['rules_evaluated'] = [];
  const violations: string[] = [];

  // Rule: Minimum approvals required
  const minApprovalsRule = {
    rule_id: 'min_approvals',
    rule_name: 'Minimum Approvals Required',
    satisfied: countApprovals(input) >= input.min_approvals_required || decision.decision !== 'approved',
    details: `${countApprovals(input)}/${input.min_approvals_required} approvals`
  };
  rules.push(minApprovalsRule);
  if (!minApprovalsRule.satisfied) {
    violations.push('Minimum approvals requirement not met');
  }

  // Rule: Required approvers present
  const requiredApproversRule = {
    rule_id: 'required_approvers',
    rule_name: 'Required Approvers Present',
    satisfied: checkRequiredApprovers(input) || decision.decision !== 'approved',
    details: 'All required approver types have approved'
  };
  rules.push(requiredApproversRule);
  if (!requiredApproversRule.satisfied) {
    violations.push('Not all required approver types have approved');
  }

  // Rule: Deadline not exceeded
  const deadlineRule = {
    rule_id: 'deadline_compliance',
    rule_name: 'Approval Deadline',
    satisfied: !isDeadlinePassed(input.approval_deadline) || ['expired', 'escalated'].includes(decision.decision),
    details: `Deadline: ${input.approval_deadline}`
  };
  rules.push(deadlineRule);
  if (!deadlineRule.satisfied) {
    violations.push('Approval deadline exceeded');
  }

  // Rule: No auto-approval (always satisfied by design)
  const noAutoApprovalRule = {
    rule_id: 'no_auto_approval',
    rule_name: 'No Auto-Approval',
    satisfied: true, // HITL agent NEVER auto-approves by design
    details: 'All approvals are from human approvers'
  };
  rules.push(noAutoApprovalRule);

  return {
    compliant: violations.length === 0,
    policy_id: input.policy_id || 'default-approval-policy',
    policy_name: 'Approval Policy',
    rules_evaluated: rules,
    violations
  };
}

// ============================================================================
// CONFIDENCE CALCULATION
// ============================================================================

/**
 * Calculate confidence score based on approval state
 */
function calculateConfidence(
  input: HITLAgentInput,
  decision: ApprovalDecisionResult,
  compliance: PolicyComplianceStatus
): { confidence: number; factors: ConfidenceFactor[] } {
  const factors: ConfidenceFactor[] = [];
  let totalScore = 0;

  // Factor: All approvals obtained
  const approvalsObtained = countApprovals(input) >= input.min_approvals_required;
  const approvalsFactor: ConfidenceFactor = {
    factor: HITL_CONFIDENCE_FACTORS.all_approvals_obtained.factor,
    weight: HITL_CONFIDENCE_FACTORS.all_approvals_obtained.weight,
    contribution: approvalsObtained ? HITL_CONFIDENCE_FACTORS.all_approvals_obtained.weight : 0,
    explanation: approvalsObtained ? 'All required approvals obtained' : 'Still awaiting approvals'
  };
  factors.push(approvalsFactor);
  totalScore += approvalsFactor.contribution;

  // Factor: No rejections
  const noRejections = input.existing_rejections.length === 0;
  const rejectionsFactor: ConfidenceFactor = {
    factor: HITL_CONFIDENCE_FACTORS.no_rejections.factor,
    weight: HITL_CONFIDENCE_FACTORS.no_rejections.weight,
    contribution: noRejections ? HITL_CONFIDENCE_FACTORS.no_rejections.weight : 0,
    explanation: noRejections ? 'No rejections recorded' : 'Rejections present'
  };
  factors.push(rejectionsFactor);
  totalScore += rejectionsFactor.contribution;

  // Factor: Policy compliant
  const policyFactor: ConfidenceFactor = {
    factor: HITL_CONFIDENCE_FACTORS.policy_compliant.factor,
    weight: HITL_CONFIDENCE_FACTORS.policy_compliant.weight,
    contribution: compliance.compliant ? HITL_CONFIDENCE_FACTORS.policy_compliant.weight : 0,
    explanation: compliance.compliant ? 'Policy requirements met' : 'Policy violations present'
  };
  factors.push(policyFactor);
  totalScore += policyFactor.contribution;

  // Factor: Authority verified
  const authorityVerified = checkRequiredApprovers(input);
  const authorityFactor: ConfidenceFactor = {
    factor: HITL_CONFIDENCE_FACTORS.authority_verified.factor,
    weight: HITL_CONFIDENCE_FACTORS.authority_verified.weight,
    contribution: authorityVerified ? HITL_CONFIDENCE_FACTORS.authority_verified.weight : 0,
    explanation: authorityVerified ? 'All required authorities approved' : 'Missing required authority'
  };
  factors.push(authorityFactor);
  totalScore += authorityFactor.contribution;

  // Factor: Conditions valid
  const conditions = extractConditions(input.existing_approvals);
  const conditionsValid = conditions.every(c => c.trim().length > 0);
  const conditionsFactor: ConfidenceFactor = {
    factor: HITL_CONFIDENCE_FACTORS.conditions_valid.factor,
    weight: HITL_CONFIDENCE_FACTORS.conditions_valid.weight,
    contribution: conditionsValid ? HITL_CONFIDENCE_FACTORS.conditions_valid.weight : 0,
    explanation: conditions.length === 0 ? 'No conditions specified' : 'Conditions are actionable'
  };
  factors.push(conditionsFactor);
  totalScore += conditionsFactor.contribution;

  // Special cases for terminal states
  if (decision.decision === 'expired') {
    return { confidence: 1.0, factors }; // Deadline is deterministic
  }
  if (decision.decision === 'rejected') {
    return { confidence: 1.0, factors }; // Rejection is final
  }
  if (decision.decision === 'cancelled') {
    return { confidence: 1.0, factors }; // Cancellation is final
  }

  return { confidence: Math.min(1.0, totalScore), factors };
}

// ============================================================================
// CONSTRAINTS BUILDING
// ============================================================================

/**
 * Build decision constraints
 */
function buildConstraints(
  input: HITLAgentInput,
  _decision: ApprovalDecisionResult
): DecisionConstraints {
  const constraints: DecisionConstraints = {};

  // Approval requirements
  constraints.approval_requirements = input.required_approvers.map(req => ({
    approver_type: req.approver_type === 'specific_user' ? 'user' : 'team',
    approver_id: req.approver_id || req.approver_type,
    required: req.required,
    obtained: input.existing_approvals.some(a =>
      req.approver_type === 'specific_user'
        ? a.approver_id === req.approver_id
        : a.approver_type === req.approver_type
    )
  }));

  // Time constraints
  constraints.time_constraints = [{
    type: 'timeout' as const,
    deadline: input.approval_deadline,
    breached: isDeadlinePassed(input.approval_deadline)
  }];

  // Severity thresholds (approval required for certain severities)
  if (['P0', 'P1'].includes(input.incident_severity)) {
    constraints.severity_thresholds = [{
      threshold: input.incident_severity,
      direction: 'equal' as const,
      triggered: true
    }];
  }

  return constraints;
}

// ============================================================================
// TELEMETRY
// ============================================================================

/**
 * Emit success telemetry
 */
async function emitSuccessTelemetry(
  config: HITLHandlerConfig,
  agentId: AgentId,
  input: HITLAgentInput,
  output: HITLAgentOutput,
  durationMs: number
): Promise<void> {
  const event: TelemetryEvent = {
    event_type: 'decision_made',
    agent_id: agentId,
    execution_ref: input.execution_id,
    timestamp: new Date().toISOString(),
    payload: {
      approval_request_id: input.approval_request_id,
      incident_id: input.incident_id,
      action_type: input.action_type,
      decision: output.decision,
      action_authorized: output.action_authorized
    },
    metrics: {
      duration_ms: durationMs,
      approvals_obtained: output.approvals_obtained,
      approvals_remaining: output.approvals_remaining,
      rejections_count: output.rejections_count
    },
    tags: {
      agent_type: HITL_AGENT_METADATA.agent_type,
      environment: config.environment,
      decision_type: HITL_AGENT_METADATA.decision_type,
      incident_severity: input.incident_severity
    }
  };

  await config.ruvectorClient.emitTelemetry(event);
}

/**
 * Emit error telemetry
 */
async function emitErrorTelemetry(
  config: HITLHandlerConfig,
  agentId: AgentId,
  errorType: string,
  details: unknown
): Promise<void> {
  const event: TelemetryEvent = {
    event_type: 'error',
    agent_id: agentId,
    execution_ref: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    payload: {
      error_type: errorType,
      details
    },
    tags: {
      agent_type: HITL_AGENT_METADATA.agent_type,
      environment: config.environment,
      error_type: errorType
    }
  };

  await config.ruvectorClient.emitTelemetry(event);
}

// ============================================================================
// EDGE FUNCTION ENTRY POINT
// ============================================================================

/**
 * Google Cloud Edge Function entry point
 *
 * This is the exported handler that Google Cloud will invoke.
 * It wraps the core handler with request/response handling.
 */
export interface EdgeFunctionRequest {
  body: unknown;
  headers: Record<string, string>;
}

export interface EdgeFunctionResponse {
  statusCode: number;
  body: string;
  headers: Record<string, string>;
}

export async function edgeFunctionHandler(
  request: EdgeFunctionRequest,
  ruvectorClient: RuVectorClient
): Promise<EdgeFunctionResponse> {
  const config: HITLHandlerConfig = {
    ruvectorClient,
    environment: (process.env.ENVIRONMENT as Environment) || 'production',
    region: process.env.REGION,
    defaultDeadlineSeconds: 3600, // 1 hour default
    strictPolicyEnforcement: true
  };

  const result = await handleHITLRequest(request.body, config);

  if (result.success) {
    return {
      statusCode: 200,
      body: JSON.stringify({
        success: true,
        data: result.data,
        warnings: result.warnings
      }),
      headers: {
        'Content-Type': 'application/json',
        'X-Agent-Id': buildAgentId(),
        'X-Agent-Version': HITL_AGENT_METADATA.version
      }
    };
  } else {
    const statusCode = result.error.code === 'VALIDATION_ERROR' ? 400 : 500;
    return {
      statusCode,
      body: JSON.stringify({
        success: false,
        error: result.error
      }),
      headers: {
        'Content-Type': 'application/json',
        'X-Agent-Id': buildAgentId(),
        'X-Agent-Version': HITL_AGENT_METADATA.version
      }
    };
  }
}
