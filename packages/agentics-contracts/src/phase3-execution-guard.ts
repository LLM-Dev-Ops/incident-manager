/**
 * Phase 3 — Execution Guard
 *
 * Runtime enforcement of execution role constraints.
 * Prevents agents from performing prohibited actions.
 *
 * CRITICAL: Violation of execution guard = HARD FAIL
 */

import type { UUID, ISO8601Timestamp } from './common.js';
import {
  REQUIRED_EXECUTION_ROLES,
  PROHIBITED_EXECUTION_ROLES,
  PERFORMANCE_BUDGETS,
  Phase3HardFailError,
  type RequiredExecutionRole,
  type ProhibitedExecutionRole,
  type Phase3Signal,
  type PerformanceMetrics,
  type Phase3AuditRecord,
  validateSignal,
  validateExecutionRole,
} from './phase3-config.js';

// ============================================================================
// EXECUTION CONTEXT
// ============================================================================

/**
 * Execution context for tracking agent behavior
 */
export interface ExecutionContext {
  execution_id: UUID;
  trace_id?: string;
  started_at: ISO8601Timestamp;
  phase: 'phase3';
  layer: 'layer1';
  agent_type: string;

  // Metrics tracking
  tokens_used: number;
  api_calls_made: number;

  // Role tracking
  roles_performed: RequiredExecutionRole[];
  prohibited_attempts: ProhibitedExecutionRole[];

  // Signal tracking
  signals_emitted: Phase3Signal[];
}

// ============================================================================
// EXECUTION GUARD
// ============================================================================

/**
 * Phase 3 Execution Guard
 *
 * Monitors and enforces execution constraints at runtime.
 * Must be instantiated at the start of each agent invocation.
 */
export class Phase3ExecutionGuard {
  private readonly context: ExecutionContext;
  private readonly startTime: number;
  private finalized: boolean = false;

  constructor(
    executionId: UUID,
    agentType: string,
    traceId?: string
  ) {
    this.startTime = Date.now();
    this.context = {
      execution_id: executionId,
      trace_id: traceId,
      started_at: new Date().toISOString(),
      phase: 'phase3',
      layer: 'layer1',
      agent_type: agentType,
      tokens_used: 0,
      api_calls_made: 0,
      roles_performed: [],
      prohibited_attempts: [],
      signals_emitted: [],
    };
  }

  // ============================================================================
  // ROLE ENFORCEMENT
  // ============================================================================

  /**
   * Record that an execution role is being performed
   *
   * @throws Phase3HardFailError if role is prohibited
   */
  performRole(role: string): void {
    this.ensureNotFinalized();

    const validation = validateExecutionRole(role);

    if (validation.isProhibited) {
      this.context.prohibited_attempts.push(role as ProhibitedExecutionRole);

      throw new Phase3HardFailError(
        'execution_guard_violated',
        {
          attempted_role: role,
          prohibited_roles: PROHIBITED_EXECUTION_ROLES,
          context: this.context,
        },
        `Attempted prohibited role: ${role}. Agents in Phase 3 Layer 1 MUST NOT ${role}.`
      );
    }

    if (validation.isRequired) {
      if (!this.context.roles_performed.includes(role as RequiredExecutionRole)) {
        this.context.roles_performed.push(role as RequiredExecutionRole);
      }
    }
  }

  /**
   * Assert that an action is a coordination action (not a decision)
   */
  assertCoordinationOnly(action: string, description: string): void {
    this.ensureNotFinalized();

    // Check for decision-like patterns
    const decisionPatterns = [
      /final.*(decision|conclusion)/i,
      /executive.*(decision|conclusion)/i,
      /definitive.*(answer|result)/i,
      /authoritative.*(decision|ruling)/i,
    ];

    for (const pattern of decisionPatterns) {
      if (pattern.test(description)) {
        this.context.prohibited_attempts.push('emit_executive_conclusion');

        throw new Phase3HardFailError(
          'execution_guard_violated',
          {
            action,
            description,
            pattern: pattern.toString(),
            context: this.context,
          },
          `Action "${action}" appears to be a final decision, not coordination. ` +
          `Phase 3 Layer 1 agents MUST NOT make final decisions.`
        );
      }
    }

    // Record as coordination
    this.performRole('coordinate');
  }

  // ============================================================================
  // RESOURCE TRACKING
  // ============================================================================

  /**
   * Record token usage
   *
   * @throws Phase3HardFailError if budget exceeded
   */
  recordTokens(count: number): void {
    this.ensureNotFinalized();

    this.context.tokens_used += count;

    if (this.context.tokens_used > PERFORMANCE_BUDGETS.MAX_TOKENS) {
      throw new Phase3HardFailError(
        'performance_budget_exceeded',
        {
          budget: PERFORMANCE_BUDGETS.MAX_TOKENS,
          used: this.context.tokens_used,
          context: this.context,
        },
        `Token budget exceeded: ${this.context.tokens_used} > ${PERFORMANCE_BUDGETS.MAX_TOKENS}`
      );
    }
  }

  /**
   * Record an API call
   *
   * @throws Phase3HardFailError if budget exceeded
   */
  recordApiCall(): void {
    this.ensureNotFinalized();

    this.context.api_calls_made += 1;

    if (this.context.api_calls_made > PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN) {
      throw new Phase3HardFailError(
        'performance_budget_exceeded',
        {
          budget: PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN,
          used: this.context.api_calls_made,
          context: this.context,
        },
        `API call budget exceeded: ${this.context.api_calls_made} > ${PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN}`
      );
    }
  }

  /**
   * Check if latency budget is still available
   */
  checkLatencyBudget(): { available: boolean; remaining_ms: number } {
    const elapsed = Date.now() - this.startTime;
    const remaining = PERFORMANCE_BUDGETS.MAX_LATENCY_MS - elapsed;

    return {
      available: remaining > 0,
      remaining_ms: Math.max(0, remaining),
    };
  }

  // ============================================================================
  // SIGNAL EMISSION
  // ============================================================================

  /**
   * Emit a Phase 3 signal
   *
   * @throws Phase3HardFailError if signal is invalid
   */
  emitSignal(signal: Phase3Signal): void {
    this.ensureNotFinalized();

    const validation = validateSignal(signal);

    if (!validation.valid) {
      throw new Phase3HardFailError(
        'execution_guard_violated',
        {
          signal,
          errors: validation.errors,
          context: this.context,
        },
        `Invalid signal: ${validation.errors.join(', ')}`
      );
    }

    this.context.signals_emitted.push(signal);

    // Map signal type to execution role
    const roleMap: Record<string, RequiredExecutionRole> = {
      execution_strategy_signal: 'route',
      optimization_signal: 'optimize',
      incident_signal: 'escalate',
    };

    const role = roleMap[signal.signal_type];
    if (role) {
      this.performRole(role);
    }
  }

  // ============================================================================
  // FINALIZATION
  // ============================================================================

  /**
   * Finalize execution and generate audit record
   *
   * @throws Phase3HardFailError if constraints violated
   */
  finalize(): { metrics: PerformanceMetrics; audit: Phase3AuditRecord } {
    if (this.finalized) {
      throw new Error('Execution already finalized');
    }

    this.finalized = true;
    const latencyMs = Date.now() - this.startTime;

    // Check latency budget
    if (latencyMs > PERFORMANCE_BUDGETS.MAX_LATENCY_MS) {
      throw new Phase3HardFailError(
        'performance_budget_exceeded',
        {
          budget: PERFORMANCE_BUDGETS.MAX_LATENCY_MS,
          actual: latencyMs,
          context: this.context,
        },
        `Latency budget exceeded: ${latencyMs}ms > ${PERFORMANCE_BUDGETS.MAX_LATENCY_MS}ms`
      );
    }

    // Validate at least one required role was performed
    if (this.context.roles_performed.length === 0) {
      throw new Phase3HardFailError(
        'execution_guard_violated',
        {
          required_roles: REQUIRED_EXECUTION_ROLES,
          performed_roles: this.context.roles_performed,
          context: this.context,
        },
        `No required execution roles performed. Agent must: ${REQUIRED_EXECUTION_ROLES.join(', ')}`
      );
    }

    // Validate at least one signal was emitted
    if (this.context.signals_emitted.length === 0) {
      throw new Phase3HardFailError(
        'execution_guard_violated',
        {
          context: this.context,
        },
        'No signals emitted. Phase 3 agents must emit at least one signal.'
      );
    }

    const metrics: PerformanceMetrics = {
      tokens_used: this.context.tokens_used,
      latency_ms: latencyMs,
      api_calls_made: this.context.api_calls_made,
      budget_remaining: {
        tokens: PERFORMANCE_BUDGETS.MAX_TOKENS - this.context.tokens_used,
        latency_ms: PERFORMANCE_BUDGETS.MAX_LATENCY_MS - latencyMs,
        calls: PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN - this.context.api_calls_made,
      },
    };

    const audit: Phase3AuditRecord = {
      execution_id: this.context.execution_id,
      trace_id: this.context.trace_id,
      roles_validated: this.context.roles_performed,
      prohibited_roles_attempted: this.context.prohibited_attempts,
      compliant: this.context.prohibited_attempts.length === 0,
      compliance_failure_reason: this.context.prohibited_attempts.length > 0
        ? `Attempted prohibited roles: ${this.context.prohibited_attempts.join(', ')}`
        : undefined,
    };

    return { metrics, audit };
  }

  // ============================================================================
  // GETTERS
  // ============================================================================

  /**
   * Get current execution context (read-only)
   */
  getContext(): Readonly<ExecutionContext> {
    return { ...this.context };
  }

  /**
   * Get emitted signals
   */
  getSignals(): readonly Phase3Signal[] {
    return [...this.context.signals_emitted];
  }

  // ============================================================================
  // PRIVATE METHODS
  // ============================================================================

  private ensureNotFinalized(): void {
    if (this.finalized) {
      throw new Error('Cannot modify execution after finalization');
    }
  }
}

// ============================================================================
// FACTORY FUNCTION
// ============================================================================

/**
 * Create a new execution guard for an agent invocation
 *
 * @example
 * ```typescript
 * const guard = createExecutionGuard('my-agent', traceId);
 *
 * // Perform coordination
 * guard.performRole('coordinate');
 * guard.recordApiCall();
 *
 * // Emit signal
 * guard.emitSignal({
 *   signal_type: 'incident_signal',
 *   incident_id: '...',
 *   // ...
 * });
 *
 * // Finalize and get audit record
 * const { metrics, audit } = guard.finalize();
 * ```
 */
export function createExecutionGuard(
  agentType: string,
  traceId?: string
): Phase3ExecutionGuard {
  const executionId = crypto.randomUUID() as UUID;
  return new Phase3ExecutionGuard(executionId, agentType, traceId);
}

// ExecutionContext is exported via the interface declaration above
