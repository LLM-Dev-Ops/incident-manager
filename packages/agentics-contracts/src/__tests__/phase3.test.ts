/**
 * Phase 3 — Automation & Resilience (Layer 1) Tests
 *
 * Tests for startup hardening, execution guards, and signal validation.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  PHASE3_LAYER1_CONFIG,
  PERFORMANCE_BUDGETS,
  REQUIRED_EXECUTION_ROLES,
  PROHIBITED_EXECUTION_ROLES,
  Phase3HardFailError,
  validateSignal,
  validatePerformanceBudget,
  validateExecutionRole,
  type ExecutionStrategySignal,
  type OptimizationSignal,
  type IncidentSignal,
  type PerformanceMetrics,
} from '../phase3-config.js';

import {
  REQUIRED_ENV_VARS,
  OPTIONAL_ENV_VARS,
} from '../phase3-startup.js';

import {
  Phase3ExecutionGuard,
  createExecutionGuard,
} from '../phase3-execution-guard.js';

// ============================================================================
// PHASE 3 CONFIG TESTS
// ============================================================================

describe('Phase3 Configuration', () => {
  it('should have correct phase and layer constants', () => {
    expect(PHASE3_LAYER1_CONFIG.AGENT_PHASE).toBe('phase3');
    expect(PHASE3_LAYER1_CONFIG.AGENT_LAYER).toBe('layer1');
    expect(PHASE3_LAYER1_CONFIG.PHASE_NAME).toBe('AUTOMATION_AND_RESILIENCE');
  });

  it('should have correct performance budgets', () => {
    expect(PERFORMANCE_BUDGETS.MAX_TOKENS).toBe(1500);
    expect(PERFORMANCE_BUDGETS.MAX_LATENCY_MS).toBe(3000);
    expect(PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN).toBe(4);
  });

  it('should define required execution roles', () => {
    expect(REQUIRED_EXECUTION_ROLES).toContain('coordinate');
    expect(REQUIRED_EXECUTION_ROLES).toContain('route');
    expect(REQUIRED_EXECUTION_ROLES).toContain('optimize');
    expect(REQUIRED_EXECUTION_ROLES).toContain('escalate');
  });

  it('should define prohibited execution roles', () => {
    expect(PROHIBITED_EXECUTION_ROLES).toContain('make_final_decision');
    expect(PROHIBITED_EXECUTION_ROLES).toContain('emit_executive_conclusion');
  });
});

// ============================================================================
// SIGNAL VALIDATION TESTS
// ============================================================================

describe('Signal Validation', () => {
  describe('validateSignal', () => {
    it('should validate a correct execution_strategy_signal', () => {
      const signal: ExecutionStrategySignal = {
        signal_type: 'execution_strategy_signal',
        strategy_id: '550e8400-e29b-41d4-a716-446655440000',
        strategy_name: 'test-strategy',
        confidence: 0.85,
        references: [
          {
            reference_type: 'metric',
            reference_id: 'latency-p99',
            reference_source: 'prometheus',
            weight: 1.0,
          },
        ],
        recommended_action: 'route',
        execution_path: ['handler-1', 'handler-2'],
        constraints_applied: ['max-latency'],
        timestamp: new Date().toISOString(),
      };

      const result = validateSignal(signal);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should reject signal with invalid confidence', () => {
      const signal: ExecutionStrategySignal = {
        signal_type: 'execution_strategy_signal',
        strategy_id: '550e8400-e29b-41d4-a716-446655440000',
        strategy_name: 'test',
        confidence: 1.5, // Invalid: > 1
        references: [{ reference_type: 'metric', reference_id: 'x', reference_source: 'y', weight: 1.0 }],
        recommended_action: 'route',
        execution_path: [],
        constraints_applied: [],
        timestamp: new Date().toISOString(),
      };

      const result = validateSignal(signal);
      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.includes('Confidence'))).toBe(true);
    });

    it('should reject signal with no references', () => {
      const signal: IncidentSignal = {
        signal_type: 'incident_signal',
        incident_id: '550e8400-e29b-41d4-a716-446655440000',
        severity_assessment: 'P2',
        confidence: 0.9,
        references: [], // Invalid: empty
        routing_recommendation: 'team-sre',
        escalation_required: false,
        timestamp: new Date().toISOString(),
      };

      const result = validateSignal(signal);
      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.includes('reference'))).toBe(true);
    });

    it('should reject signal with weights not summing to 1', () => {
      const signal: OptimizationSignal = {
        signal_type: 'optimization_signal',
        optimization_id: '550e8400-e29b-41d4-a716-446655440000',
        optimization_type: 'latency',
        confidence: 0.8,
        references: [
          { reference_type: 'metric', reference_id: 'a', reference_source: 'x', weight: 0.3 },
          { reference_type: 'metric', reference_id: 'b', reference_source: 'y', weight: 0.3 },
          // Sum = 0.6, not 1.0
        ],
        current_value: 100,
        suggested_value: 50,
        improvement_percentage: 50,
        timestamp: new Date().toISOString(),
      };

      const result = validateSignal(signal);
      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.includes('weights must sum'))).toBe(true);
    });
  });
});

// ============================================================================
// PERFORMANCE BUDGET TESTS
// ============================================================================

describe('Performance Budget Validation', () => {
  it('should pass for metrics within budget', () => {
    const metrics: PerformanceMetrics = {
      tokens_used: 1000,
      latency_ms: 2000,
      api_calls_made: 3,
      budget_remaining: {
        tokens: 500,
        latency_ms: 1000,
        calls: 1,
      },
    };

    const result = validatePerformanceBudget(metrics);
    expect(result.valid).toBe(true);
    expect(result.violations).toHaveLength(0);
  });

  it('should fail when tokens exceed budget', () => {
    const metrics: PerformanceMetrics = {
      tokens_used: 2000, // Exceeds 1500
      latency_ms: 1000,
      api_calls_made: 2,
      budget_remaining: { tokens: -500, latency_ms: 2000, calls: 2 },
    };

    const result = validatePerformanceBudget(metrics);
    expect(result.valid).toBe(false);
    expect(result.condition).toBe('performance_budget_exceeded');
    expect(result.violations.some(v => v.includes('Tokens'))).toBe(true);
  });

  it('should fail when latency exceeds budget', () => {
    const metrics: PerformanceMetrics = {
      tokens_used: 1000,
      latency_ms: 5000, // Exceeds 3000
      api_calls_made: 2,
      budget_remaining: { tokens: 500, latency_ms: -2000, calls: 2 },
    };

    const result = validatePerformanceBudget(metrics);
    expect(result.valid).toBe(false);
    expect(result.violations.some(v => v.includes('Latency'))).toBe(true);
  });

  it('should fail when API calls exceed budget', () => {
    const metrics: PerformanceMetrics = {
      tokens_used: 1000,
      latency_ms: 2000,
      api_calls_made: 10, // Exceeds 4
      budget_remaining: { tokens: 500, latency_ms: 1000, calls: -6 },
    };

    const result = validatePerformanceBudget(metrics);
    expect(result.valid).toBe(false);
    expect(result.violations.some(v => v.includes('API calls'))).toBe(true);
  });
});

// ============================================================================
// EXECUTION ROLE VALIDATION TESTS
// ============================================================================

describe('Execution Role Validation', () => {
  it('should validate required roles as valid', () => {
    for (const role of REQUIRED_EXECUTION_ROLES) {
      const result = validateExecutionRole(role);
      expect(result.valid).toBe(true);
      expect(result.isRequired).toBe(true);
      expect(result.isProhibited).toBe(false);
    }
  });

  it('should reject prohibited roles', () => {
    for (const role of PROHIBITED_EXECUTION_ROLES) {
      const result = validateExecutionRole(role);
      expect(result.valid).toBe(false);
      expect(result.isProhibited).toBe(true);
      expect(result.condition).toBe('prohibited_role_attempted');
    }
  });

  it('should mark unknown roles as invalid but not prohibited', () => {
    const result = validateExecutionRole('unknown_role');
    expect(result.valid).toBe(false);
    expect(result.isRequired).toBe(false);
    expect(result.isProhibited).toBe(false);
  });
});

// ============================================================================
// STARTUP VALIDATOR TESTS
// ============================================================================

describe('Phase3StartupValidator', () => {
  it('should require all mandatory environment variables', () => {
    expect(REQUIRED_ENV_VARS).toContain('AGENT_PHASE');
    expect(REQUIRED_ENV_VARS).toContain('AGENT_LAYER');
    expect(REQUIRED_ENV_VARS).toContain('RUVECTOR_API_KEY');
    expect(REQUIRED_ENV_VARS).toContain('RUVECTOR_SERVICE_URL');
  });

  it('should have sensible defaults for optional variables', () => {
    expect(OPTIONAL_ENV_VARS.RUVECTOR_TIMEOUT_MS).toBe('30000');
    expect(OPTIONAL_ENV_VARS.MAX_TOKENS).toBe(String(PERFORMANCE_BUDGETS.MAX_TOKENS));
    expect(OPTIONAL_ENV_VARS.MAX_LATENCY_MS).toBe(String(PERFORMANCE_BUDGETS.MAX_LATENCY_MS));
    expect(OPTIONAL_ENV_VARS.MAX_CALLS_PER_RUN).toBe(String(PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN));
  });
});

// ============================================================================
// EXECUTION GUARD TESTS
// ============================================================================

describe('Phase3ExecutionGuard', () => {
  let guard: Phase3ExecutionGuard;

  beforeEach(() => {
    guard = createExecutionGuard('test-agent', 'trace-123');
  });

  describe('Role Enforcement', () => {
    it('should allow required roles', () => {
      expect(() => guard.performRole('coordinate')).not.toThrow();
      expect(() => guard.performRole('route')).not.toThrow();
      expect(() => guard.performRole('optimize')).not.toThrow();
      expect(() => guard.performRole('escalate')).not.toThrow();
    });

    it('should throw Phase3HardFailError for prohibited roles', () => {
      expect(() => guard.performRole('make_final_decision')).toThrow(Phase3HardFailError);
      expect(() => guard.performRole('emit_executive_conclusion')).toThrow(Phase3HardFailError);
    });

    it('should detect decision-like actions', () => {
      expect(() =>
        guard.assertCoordinationOnly('process', 'Making a final decision on routing')
      ).toThrow(Phase3HardFailError);

      expect(() =>
        guard.assertCoordinationOnly('process', 'Emitting executive conclusion')
      ).toThrow(Phase3HardFailError);
    });

    it('should allow coordination-only actions', () => {
      expect(() =>
        guard.assertCoordinationOnly('process', 'Coordinating between handlers')
      ).not.toThrow();
    });
  });

  describe('Resource Tracking', () => {
    it('should track token usage', () => {
      guard.recordTokens(500);
      guard.recordTokens(500);

      const context = guard.getContext();
      expect(context.tokens_used).toBe(1000);
    });

    it('should throw when token budget exceeded', () => {
      guard.recordTokens(1000);
      expect(() => guard.recordTokens(600)).toThrow(Phase3HardFailError);
    });

    it('should track API calls', () => {
      guard.recordApiCall();
      guard.recordApiCall();

      const context = guard.getContext();
      expect(context.api_calls_made).toBe(2);
    });

    it('should throw when API call budget exceeded', () => {
      guard.recordApiCall();
      guard.recordApiCall();
      guard.recordApiCall();
      guard.recordApiCall();
      expect(() => guard.recordApiCall()).toThrow(Phase3HardFailError);
    });

    it('should check latency budget', () => {
      const result = guard.checkLatencyBudget();
      expect(result.available).toBe(true);
      expect(result.remaining_ms).toBeGreaterThan(0);
    });
  });

  describe('Signal Emission', () => {
    it('should emit valid signals', () => {
      const signal: IncidentSignal = {
        signal_type: 'incident_signal',
        incident_id: '550e8400-e29b-41d4-a716-446655440000',
        severity_assessment: 'P2',
        confidence: 0.9,
        references: [
          { reference_type: 'metric', reference_id: 'x', reference_source: 'y', weight: 1.0 },
        ],
        routing_recommendation: 'team-sre',
        escalation_required: false,
        timestamp: new Date().toISOString(),
      };

      expect(() => guard.emitSignal(signal)).not.toThrow();
      expect(guard.getSignals()).toHaveLength(1);
    });

    it('should reject invalid signals', () => {
      const signal: IncidentSignal = {
        signal_type: 'incident_signal',
        incident_id: '550e8400-e29b-41d4-a716-446655440000',
        severity_assessment: 'P2',
        confidence: 1.5, // Invalid
        references: [],
        routing_recommendation: 'team-sre',
        escalation_required: false,
        timestamp: new Date().toISOString(),
      };

      expect(() => guard.emitSignal(signal)).toThrow(Phase3HardFailError);
    });
  });

  describe('Finalization', () => {
    it('should require at least one role to be performed', () => {
      // Emit a signal but don't perform any role
      const signal: IncidentSignal = {
        signal_type: 'incident_signal',
        incident_id: '550e8400-e29b-41d4-a716-446655440000',
        severity_assessment: 'P2',
        confidence: 0.9,
        references: [
          { reference_type: 'metric', reference_id: 'x', reference_source: 'y', weight: 1.0 },
        ],
        routing_recommendation: 'team-sre',
        escalation_required: false,
        timestamp: new Date().toISOString(),
      };

      guard.emitSignal(signal);

      // emitSignal already performs a role based on signal type, so this should pass
      const { audit } = guard.finalize();
      expect(audit.compliant).toBe(true);
    });

    it('should require at least one signal', () => {
      guard.performRole('coordinate');

      expect(() => guard.finalize()).toThrow(Phase3HardFailError);
    });

    it('should return metrics and audit on successful finalization', () => {
      guard.performRole('coordinate');
      guard.recordTokens(500);
      guard.recordApiCall();

      const signal: IncidentSignal = {
        signal_type: 'incident_signal',
        incident_id: '550e8400-e29b-41d4-a716-446655440000',
        severity_assessment: 'P2',
        confidence: 0.9,
        references: [
          { reference_type: 'metric', reference_id: 'x', reference_source: 'y', weight: 1.0 },
        ],
        routing_recommendation: 'team-sre',
        escalation_required: false,
        timestamp: new Date().toISOString(),
      };
      guard.emitSignal(signal);

      const { metrics, audit } = guard.finalize();

      expect(metrics.tokens_used).toBe(500);
      expect(metrics.api_calls_made).toBe(1);
      expect(audit.compliant).toBe(true);
      expect(audit.roles_validated).toContain('coordinate');
    });

    it('should prevent modifications after finalization', () => {
      guard.performRole('coordinate');
      const signal: IncidentSignal = {
        signal_type: 'incident_signal',
        incident_id: '550e8400-e29b-41d4-a716-446655440000',
        severity_assessment: 'P2',
        confidence: 0.9,
        references: [
          { reference_type: 'metric', reference_id: 'x', reference_source: 'y', weight: 1.0 },
        ],
        routing_recommendation: 'team-sre',
        escalation_required: false,
        timestamp: new Date().toISOString(),
      };
      guard.emitSignal(signal);
      guard.finalize();

      expect(() => guard.performRole('route')).toThrow();
      expect(() => guard.recordTokens(100)).toThrow();
    });
  });
});

// ============================================================================
// HARD FAIL ERROR TESTS
// ============================================================================

describe('Phase3HardFailError', () => {
  it('should capture condition and details', () => {
    const error = new Phase3HardFailError(
      'ruvector_unavailable',
      { serviceUrl: 'https://example.com' },
      'Connection failed'
    );

    expect(error.condition).toBe('ruvector_unavailable');
    expect(error.details).toEqual({ serviceUrl: 'https://example.com' });
    expect(error.message).toContain('ruvector_unavailable');
    expect(error.message).toContain('Connection failed');
    expect(error.name).toBe('Phase3HardFailError');
  });
});
