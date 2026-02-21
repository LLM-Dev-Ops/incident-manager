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
// POST-MORTEM GENERATOR AGENT
// ============================================================================

export {
  POSTMORTEM_AGENT_METADATA,
  POSTMORTEM_PERSISTENCE,
  validatePostMortemInput,
  validatePostMortemOutput
} from './postmortem-agent.js';

export type {
  PostMortemStatus,
  ActionItemPriority,
  ActionItemStatus,
  TimelineEntryType,
  RootCauseCategory,
  PostMortemAgentInput,
  TimelineEvent,
  EscalationHistoryRef,
  PostMortemAgentOutput,
  ReconstructedTimelineEntry,
  KeyMoment,
  RootCauseAnalysis,
  ImpactAnalysis,
  ResolutionAnalysis,
  ActionItem,
  PostMortemOrchestratorAction,
  PostMortemDecisionEvent
} from './postmortem-agent.js';

// ============================================================================
// PHASE 3 - AUTOMATION & RESILIENCE (LAYER 1)
// ============================================================================

export {
  PHASE3_LAYER1_CONFIG,
  PERFORMANCE_BUDGETS,
  REQUIRED_EXECUTION_ROLES,
  PROHIBITED_EXECUTION_ROLES,
  HARD_FAIL_CONDITIONS,
  Phase3HardFailError,
  validateSignal,
  validatePerformanceBudget,
  validateExecutionRole,
  CONFIG as PHASE3_CONFIG,
  BUDGETS as PHASE3_BUDGETS,
  REQUIRED_ROLES as PHASE3_REQUIRED_ROLES,
  PROHIBITED_ROLES as PHASE3_PROHIBITED_ROLES
} from './phase3-config.js';

export type {
  RequiredExecutionRole,
  ProhibitedExecutionRole,
  Phase3SignalType,
  ExecutionStrategySignal,
  OptimizationSignal,
  IncidentSignal,
  SignalReference,
  Phase3Signal,
  Phase3DecisionEventPayload,
  PerformanceMetrics,
  Phase3AuditRecord,
  HardFailCondition
} from './phase3-config.js';

export {
  Phase3StartupValidator,
  startupPhase3Layer1,
  handleStartupFailure,
  REQUIRED_ENV_VARS,
  OPTIONAL_ENV_VARS
} from './phase3-startup.js';

export type {
  StartupValidationResult,
  StartupError,
  RuvectorConfig,
  RuvectorHealthClient,
  RuvectorHealthResult
} from './phase3-startup.js';

export {
  Phase3ExecutionGuard,
  createExecutionGuard
} from './phase3-execution-guard.js';

export type {
  ExecutionContext as Phase3ExecutionContext
} from './phase3-execution-guard.js';

// ============================================================================
// VERSION
// ============================================================================

export const CONTRACTS_VERSION = '1.2.0';
