/**
 * Phase 3 — Automation & Resilience (Layer 1) Configuration
 *
 * This module defines the execution constraints, performance budgets,
 * and signal types for Phase 3 agents operating at Layer 1.
 *
 * CRITICAL: Agents at this phase MUST:
 *   - Coordinate
 *   - Route
 *   - Optimize
 *   - Escalate
 *
 * Agents MUST NOT:
 *   - Make final decisions
 *   - Emit executive conclusions
 */

import type { UUID, ISO8601Timestamp } from './common.js';

// ============================================================================
// PHASE & LAYER CONSTANTS
// ============================================================================

export const PHASE3_LAYER1_CONFIG = {
  AGENT_PHASE: 'phase3' as const,
  AGENT_LAYER: 'layer1' as const,
  PHASE_NAME: 'AUTOMATION_AND_RESILIENCE' as const,
  LAYER_NAME: 'COORDINATION_LAYER' as const,
} as const;

// ============================================================================
// PERFORMANCE BUDGETS (HARD LIMITS)
// ============================================================================

export const PERFORMANCE_BUDGETS = {
  /** Maximum tokens allowed per agent invocation */
  MAX_TOKENS: 1500,

  /** Maximum latency in milliseconds */
  MAX_LATENCY_MS: 3000,

  /** Maximum number of API calls per run */
  MAX_CALLS_PER_RUN: 4,
} as const;

export type PerformanceBudget = typeof PERFORMANCE_BUDGETS;

// ============================================================================
// EXECUTION ROLES
// ============================================================================

/**
 * Actions that Phase 3 Layer 1 agents MUST perform
 */
export const REQUIRED_EXECUTION_ROLES = [
  'coordinate',
  'route',
  'optimize',
  'escalate',
] as const;

export type RequiredExecutionRole = typeof REQUIRED_EXECUTION_ROLES[number];

/**
 * Actions that Phase 3 Layer 1 agents MUST NOT perform
 * Violation of these results in HARD FAIL
 */
export const PROHIBITED_EXECUTION_ROLES = [
  'make_final_decision',
  'emit_executive_conclusion',
  'direct_remediation',
  'policy_override',
  'external_alert_emission',
] as const;

export type ProhibitedExecutionRole = typeof PROHIBITED_EXECUTION_ROLES[number];

// ============================================================================
// DECISION EVENT SIGNALS
// ============================================================================

/**
 * Signal types that Phase 3 Layer 1 agents MUST emit
 */
export type Phase3SignalType =
  | 'execution_strategy_signal'
  | 'optimization_signal'
  | 'incident_signal';

/**
 * Execution strategy signal - emitted when routing or coordinating execution
 */
export interface ExecutionStrategySignal {
  signal_type: 'execution_strategy_signal';
  strategy_id: UUID;
  strategy_name: string;
  confidence: number; // 0.0 - 1.0
  references: SignalReference[];
  recommended_action: 'route' | 'coordinate' | 'defer' | 'escalate';
  execution_path: string[];
  constraints_applied: string[];
  timestamp: ISO8601Timestamp;
}

/**
 * Optimization signal - emitted when suggesting performance improvements
 */
export interface OptimizationSignal {
  signal_type: 'optimization_signal';
  optimization_id: UUID;
  optimization_type: 'latency' | 'throughput' | 'cost' | 'reliability';
  confidence: number; // 0.0 - 1.0
  references: SignalReference[];
  current_value: number;
  suggested_value: number;
  improvement_percentage: number;
  timestamp: ISO8601Timestamp;
}

/**
 * Incident signal - emitted when detecting or routing incidents
 */
export interface IncidentSignal {
  signal_type: 'incident_signal';
  incident_id: UUID;
  severity_assessment: 'P0' | 'P1' | 'P2' | 'P3' | 'P4';
  confidence: number; // 0.0 - 1.0
  references: SignalReference[];
  routing_recommendation: string;
  escalation_required: boolean;
  timestamp: ISO8601Timestamp;
}

/**
 * Reference for signal audit trail
 */
export interface SignalReference {
  reference_type: 'metric' | 'event' | 'policy' | 'historical' | 'external';
  reference_id: string;
  reference_source: string;
  weight: number; // Contribution to confidence
}

/**
 * Union type for all Phase 3 signals
 */
export type Phase3Signal =
  | ExecutionStrategySignal
  | OptimizationSignal
  | IncidentSignal;

// ============================================================================
// DECISION EVENT WRAPPER
// ============================================================================

/**
 * DecisionEvent payload for Phase 3 Layer 1
 */
export interface Phase3DecisionEventPayload {
  /** Phase and layer identification */
  phase: typeof PHASE3_LAYER1_CONFIG.AGENT_PHASE;
  layer: typeof PHASE3_LAYER1_CONFIG.AGENT_LAYER;

  /** The primary signal emitted */
  primary_signal: Phase3Signal;

  /** Additional signals (optional) */
  secondary_signals?: Phase3Signal[];

  /** Execution role performed */
  execution_role: RequiredExecutionRole;

  /** Performance metrics for this invocation */
  performance_metrics: PerformanceMetrics;

  /** Audit trail */
  audit: Phase3AuditRecord;
}

export interface PerformanceMetrics {
  tokens_used: number;
  latency_ms: number;
  api_calls_made: number;
  budget_remaining: {
    tokens: number;
    latency_ms: number;
    calls: number;
  };
}

export interface Phase3AuditRecord {
  /** Unique execution ID */
  execution_id: UUID;

  /** Trace ID for distributed tracing */
  trace_id?: string;

  /** Execution roles validated */
  roles_validated: RequiredExecutionRole[];

  /** Prohibited roles checked (should be empty if compliant) */
  prohibited_roles_attempted: ProhibitedExecutionRole[];

  /** Whether execution was compliant */
  compliant: boolean;

  /** Compliance failure reason (if any) */
  compliance_failure_reason?: string;
}

// ============================================================================
// FAILURE SEMANTICS
// ============================================================================

/**
 * Conditions that MUST result in HARD FAIL
 */
export const HARD_FAIL_CONDITIONS = [
  'ruvector_unavailable',
  'execution_guard_violated',
  'performance_budget_exceeded',
  'prohibited_role_attempted',
  'missing_required_config',
] as const;

export type HardFailCondition = typeof HARD_FAIL_CONDITIONS[number];

/**
 * Error thrown when a hard fail condition is met
 */
export class Phase3HardFailError extends Error {
  constructor(
    public readonly condition: HardFailCondition,
    public readonly details: Record<string, unknown>,
    message: string
  ) {
    super(`[PHASE3_HARD_FAIL] ${condition}: ${message}`);
    this.name = 'Phase3HardFailError';
  }
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

/**
 * Validate that a signal includes required confidence and references
 */
export function validateSignal(signal: Phase3Signal): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if (signal.confidence < 0 || signal.confidence > 1) {
    errors.push(`Confidence must be between 0 and 1, got ${signal.confidence}`);
  }

  if (!signal.references || signal.references.length === 0) {
    errors.push('Signal must include at least one reference');
  }

  // Validate reference weights sum
  if (signal.references) {
    const totalWeight = signal.references.reduce((sum, ref) => sum + ref.weight, 0);
    if (Math.abs(totalWeight - 1.0) > 0.01) {
      errors.push(`Reference weights must sum to 1.0, got ${totalWeight}`);
    }
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Validate performance metrics against budgets
 */
export function validatePerformanceBudget(metrics: PerformanceMetrics): {
  valid: boolean;
  violations: string[];
  condition?: HardFailCondition;
} {
  const violations: string[] = [];

  if (metrics.tokens_used > PERFORMANCE_BUDGETS.MAX_TOKENS) {
    violations.push(
      `Tokens exceeded: ${metrics.tokens_used} > ${PERFORMANCE_BUDGETS.MAX_TOKENS}`
    );
  }

  if (metrics.latency_ms > PERFORMANCE_BUDGETS.MAX_LATENCY_MS) {
    violations.push(
      `Latency exceeded: ${metrics.latency_ms}ms > ${PERFORMANCE_BUDGETS.MAX_LATENCY_MS}ms`
    );
  }

  if (metrics.api_calls_made > PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN) {
    violations.push(
      `API calls exceeded: ${metrics.api_calls_made} > ${PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN}`
    );
  }

  return {
    valid: violations.length === 0,
    violations,
    condition: violations.length > 0 ? 'performance_budget_exceeded' : undefined,
  };
}

/**
 * Validate that execution role is allowed
 */
export function validateExecutionRole(role: string): {
  valid: boolean;
  isRequired: boolean;
  isProhibited: boolean;
  condition?: HardFailCondition;
} {
  const isRequired = REQUIRED_EXECUTION_ROLES.includes(role as RequiredExecutionRole);
  const isProhibited = PROHIBITED_EXECUTION_ROLES.includes(role as ProhibitedExecutionRole);

  return {
    valid: isRequired && !isProhibited,
    isRequired,
    isProhibited,
    condition: isProhibited ? 'prohibited_role_attempted' : undefined,
  };
}

// ============================================================================
// ALIASES (for convenience)
// ============================================================================

export const CONFIG = PHASE3_LAYER1_CONFIG;
export const BUDGETS = PERFORMANCE_BUDGETS;
export const REQUIRED_ROLES = REQUIRED_EXECUTION_ROLES;
export const PROHIBITED_ROLES = PROHIBITED_EXECUTION_ROLES;
