/**
 * @agentics/contracts
 *
 * Contract schemas and types for Agentics Dev platform agents.
 * This package defines ALL input/output schemas, DecisionEvents, and validation rules
 * used by agents in the LLM-Incident-Manager repository.
 *
 * ARCHITECTURAL REQUIREMENTS:
 * - All agents MUST import schemas from this package
 * - All inputs/outputs MUST be validated against these contracts
 * - All agents MUST emit exactly ONE DecisionEvent per invocation
 *
 * @packageDocumentation
 */

// ============================================================================
// COMMON TYPES
// ============================================================================

export type {
  ISO8601Timestamp,
  UUID,
  SemVer,
  SHA256Hash,
  AgentId,
  AgentType,
  AgentClassification,
  Severity,
  SeverityDirection,
  Environment,
  ExecutionContext,
  ValidationResult,
  ValidationError,
  ValidationWarning,
  AgentResult,
  AgentError,
  PersistenceSpec,
  TelemetryEvent
} from './common.js';

export { SEVERITY_NUMERIC } from './common.js';

// ============================================================================
// DECISION EVENT
// ============================================================================

export type {
  DecisionType,
  DecisionConstraints,
  SeverityThreshold,
  PolicyConstraint,
  ApprovalRequirement,
  TimeConstraint,
  RateLimitConstraint,
  DecisionEvent,
  ConfidenceFactor,
  AuditMetadata,
  ManualOverride
} from './decision-event.js';

export {
  DECISION_EVENT_PERSISTENCE,
  validateDecisionEvent,
  DecisionEventBuilder
} from './decision-event.js';

// ============================================================================
// ESCALATION AGENT
// ============================================================================

export {
  ESCALATION_AGENT_METADATA,
  ESCALATION_PERSISTENCE,
  validateEscalationInput,
  validateEscalationOutput
} from './escalation-agent.js';

export type {
  EscalationSignalSource,
  IncidentCategory,
  IncidentStatus,
  EscalationAgentInput,
  EscalationSignalPayload,
  AffectedResource,
  EscalationHistoryEntry,
  SLAContext,
  EscalationDecision,
  EscalationAgentOutput,
  OrchestratorAction,
  AssignmentUpdate,
  AppliedPolicy,
  EvaluationDetails,
  ThresholdEvaluation,
  TimeFactor,
  PatternMatch,
  EscalationDecisionEvent,
  EscalationAgentCLI
} from './escalation-agent.js';

// ============================================================================
// HUMAN-IN-THE-LOOP (HITL) AGENT
// ============================================================================

export {
  HITL_AGENT_METADATA,
  HITL_PERSISTENCE,
  HITL_CONFIDENCE_FACTORS,
  validateHITLInput,
  validateHITLOutput,
  calculateApprovalProgress,
  isDeadlinePassed,
  generateApprovalRequestId,
  hashInputs
} from './hitl-agent.js';

export type {
  ApprovalActionType,
  ApprovalRequesterType,
  ApprovalRequestStatus,
  ApproverType,
  HITLAgentInput,
  ActionParameters,
  ImpactAssessment,
  ApproverRequirement,
  ApprovalRecord,
  HITLDecision,
  HITLAgentOutput,
  ApproverInfo,
  HITLOrchestratorAction,
  AuditTrailEntry,
  PolicyComplianceStatus,
  PolicyRuleResult,
  HITLDecisionEvent,
  HITLAgentCLI
} from './hitl-agent.js';

// ============================================================================
// HITL HANDLER
// ============================================================================

export {
  handleHITLRequest,
  edgeFunctionHandler
} from './hitl-handler.js';

export type {
  RuVectorClient,
  ApprovalRequestState,
  HITLHandlerConfig,
  EdgeFunctionRequest,
  EdgeFunctionResponse
} from './hitl-handler.js';

// ============================================================================
// VERSION
// ============================================================================

export const CONTRACTS_VERSION = '1.1.0';
