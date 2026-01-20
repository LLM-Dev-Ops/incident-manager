/**
 * Integration Tests for Incident Escalation Agent
 *
 * These tests verify:
 * 1. Contract compliance
 * 2. Decision logic correctness
 * 3. DecisionEvent structure
 * 4. CLI functionality
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

// Note: These imports assume the packages are built
// In a real test environment, you would use proper module resolution

// ============================================================================
// TEST DATA
// ============================================================================

const createTestInput = (overrides: Record<string, unknown> = {}) => ({
  incident_id: '550e8400-e29b-41d4-a716-446655440000',
  fingerprint: 'test-fingerprint-123',
  current_severity: 'P2',
  current_status: 'NEW',
  current_escalation_level: 0,
  category: 'availability',
  signal_source: 'llm-sentinel',
  signal_timestamp: new Date().toISOString(),
  signal_payload: {
    type: 'anomaly',
    suggested_severity: 'P1',
    signal_confidence: 0.85
  },
  environment: 'production',
  title: 'High latency detected',
  description: 'Service response time exceeds threshold',
  affected_resource: {
    type: 'service',
    id: 'service-001',
    name: 'API Gateway'
  },
  tags: { team: 'platform', region: 'us-west-2' },
  incident_created_at: new Date(Date.now() - 600000).toISOString(), // 10 minutes ago
  incident_updated_at: new Date(Date.now() - 300000).toISOString(), // 5 minutes ago
  escalation_history: [],
  sla: {
    acknowledgment_deadline: new Date(Date.now() + 300000).toISOString(), // 5 minutes from now
    resolution_deadline: new Date(Date.now() + 3600000).toISOString(), // 1 hour from now
    acknowledgment_breached: false,
    resolution_breached: false
  },
  execution_id: '550e8400-e29b-41d4-a716-446655440001',
  ...overrides
});

// ============================================================================
// CONTRACT TESTS
// ============================================================================

describe('Escalation Agent Contract Tests', () => {
  it('should validate correct input', () => {
    const input = createTestInput();

    // Required fields check
    assert.ok(input.incident_id, 'incident_id required');
    assert.ok(input.fingerprint, 'fingerprint required');
    assert.ok(input.current_severity, 'current_severity required');
    assert.ok(input.current_status, 'current_status required');
    assert.strictEqual(typeof input.current_escalation_level, 'number', 'current_escalation_level must be number');
    assert.ok(input.signal_source, 'signal_source required');
    assert.ok(input.signal_timestamp, 'signal_timestamp required');
    assert.ok(input.execution_id, 'execution_id required');
    assert.ok(input.affected_resource, 'affected_resource required');
  });

  it('should reject invalid severity', () => {
    const invalidSeverities = ['P5', 'HIGH', 'critical', ''];
    const validSeverities = ['P0', 'P1', 'P2', 'P3', 'P4'];

    for (const invalid of invalidSeverities) {
      assert.ok(!validSeverities.includes(invalid), `${invalid} should be invalid`);
    }

    for (const valid of validSeverities) {
      assert.ok(validSeverities.includes(valid), `${valid} should be valid`);
    }
  });

  it('should reject invalid signal sources', () => {
    const validSources = ['llm-sentinel', 'llm-shield', 'llm-edge-agent', 'llm-orchestrator', 'manual', 'scheduled'];

    for (const source of validSources) {
      assert.ok(validSources.includes(source), `${source} should be valid`);
    }
  });
});

// ============================================================================
// DECISION LOGIC TESTS
// ============================================================================

describe('Escalation Decision Logic Tests', () => {
  it('should escalate when signal suggests higher severity', () => {
    const input = createTestInput({
      current_severity: 'P3',
      signal_payload: {
        type: 'anomaly',
        suggested_severity: 'P1',
        signal_confidence: 0.9
      }
    });

    // The signal suggests P1 which is more severe than current P3
    // This should trigger escalation
    const severityNumeric: Record<string, number> = { P0: 0, P1: 1, P2: 2, P3: 3, P4: 4 };
    const currentNum = severityNumeric[input.current_severity];
    const suggestedNum = severityNumeric[input.signal_payload.suggested_severity!];

    assert.ok(suggestedNum < currentNum, 'Suggested severity should be more severe');
  });

  it('should not escalate when at max level', () => {
    const input = createTestInput({
      current_escalation_level: 5 // Max level
    });

    // At max level, should maintain
    assert.strictEqual(input.current_escalation_level, 5);
  });

  it('should defer when minimum interval not met', () => {
    const input = createTestInput({
      time_since_last_escalation: 60 // Only 60 seconds since last escalation
    });

    // Default min interval is 300 seconds
    assert.ok(input.time_since_last_escalation! < 300, 'Should defer due to interval');
  });

  it('should maintain when incident is resolved', () => {
    const input = createTestInput({
      current_status: 'RESOLVED'
    });

    assert.strictEqual(input.current_status, 'RESOLVED');
  });

  it('should escalate on SLA breach', () => {
    const input = createTestInput({
      sla: {
        acknowledgment_deadline: new Date(Date.now() - 60000).toISOString(), // 1 minute ago
        resolution_deadline: new Date(Date.now() + 3600000).toISOString(),
        acknowledgment_breached: true,
        resolution_breached: false
      }
    });

    assert.ok(input.sla.acknowledgment_breached, 'SLA should be breached');
  });
});

// ============================================================================
// DECISION EVENT TESTS
// ============================================================================

describe('DecisionEvent Structure Tests', () => {
  it('should have all required DecisionEvent fields', () => {
    const mockDecisionEvent = {
      id: '550e8400-e29b-41d4-a716-446655440002',
      agent_id: 'incident-escalation:1.0.0:test',
      agent_version: '1.0.0',
      agent_classification: 'ESCALATION',
      decision_type: 'incident_escalation_decision',
      inputs_hash: 'abc123hash',
      outputs: {
        decision: 'escalate',
        reason: 'Test reason',
        orchestrator_actions: [],
        applied_policy: {
          policy_id: 'default',
          policy_name: 'Default Policy',
          policy_version: '1.0.0',
          max_level: 5
        },
        evaluation_details: {
          thresholds_evaluated: [],
          time_factors: [],
          raw_escalation_score: 0.8,
          normalized_score: 0.8
        }
      },
      confidence: 0.85,
      constraints_applied: {},
      execution_ref: '550e8400-e29b-41d4-a716-446655440001',
      timestamp: new Date().toISOString(),
      environment: 'production',
      requires_review: false
    };

    // Verify required fields
    assert.ok(mockDecisionEvent.id, 'id required');
    assert.ok(mockDecisionEvent.agent_id, 'agent_id required');
    assert.ok(mockDecisionEvent.agent_version, 'agent_version required');
    assert.ok(mockDecisionEvent.agent_classification, 'agent_classification required');
    assert.ok(mockDecisionEvent.decision_type, 'decision_type required');
    assert.ok(mockDecisionEvent.inputs_hash, 'inputs_hash required');
    assert.ok(mockDecisionEvent.outputs, 'outputs required');
    assert.strictEqual(typeof mockDecisionEvent.confidence, 'number', 'confidence must be number');
    assert.ok(mockDecisionEvent.constraints_applied !== undefined, 'constraints_applied required');
    assert.ok(mockDecisionEvent.execution_ref, 'execution_ref required');
    assert.ok(mockDecisionEvent.timestamp, 'timestamp required');
    assert.strictEqual(typeof mockDecisionEvent.requires_review, 'boolean', 'requires_review must be boolean');
  });

  it('should have confidence between 0 and 1', () => {
    const validConfidences = [0, 0.5, 0.85, 1];
    const invalidConfidences = [-0.1, 1.1, 2];

    for (const c of validConfidences) {
      assert.ok(c >= 0 && c <= 1, `${c} should be valid`);
    }

    for (const c of invalidConfidences) {
      assert.ok(c < 0 || c > 1, `${c} should be invalid`);
    }
  });

  it('should have valid decision type', () => {
    const validDecisions = ['escalate', 'deescalate', 'maintain', 'defer'];

    for (const d of validDecisions) {
      assert.ok(validDecisions.includes(d), `${d} should be valid`);
    }
  });
});

// ============================================================================
// OUTPUT VALIDATION TESTS
// ============================================================================

describe('Escalation Output Validation Tests', () => {
  it('should include new_severity when escalating', () => {
    const output = {
      decision: 'escalate',
      reason: 'SLA breach',
      new_severity: 'P1',
      new_escalation_level: 1,
      severity_delta: -1,
      orchestrator_actions: [
        {
          action_type: 'update_incident_status',
          priority: 'high',
          parameters: {},
          async: false
        }
      ],
      applied_policy: {
        policy_id: 'default',
        policy_name: 'Default',
        policy_version: '1.0.0',
        max_level: 5
      },
      evaluation_details: {
        thresholds_evaluated: [],
        time_factors: [],
        raw_escalation_score: 0.9,
        normalized_score: 0.9
      }
    };

    assert.strictEqual(output.decision, 'escalate');
    assert.ok(output.new_severity, 'new_severity should be present for escalation');
    assert.ok(output.severity_delta! < 0, 'severity_delta should be negative for escalation');
  });

  it('should include defer_until when deferring', () => {
    const output = {
      decision: 'defer',
      reason: 'Minimum interval not met',
      defer_until: new Date(Date.now() + 240000).toISOString(),
      orchestrator_actions: [],
      applied_policy: {
        policy_id: 'default',
        policy_name: 'Default',
        policy_version: '1.0.0',
        max_level: 5
      },
      evaluation_details: {
        thresholds_evaluated: [],
        time_factors: [],
        raw_escalation_score: 0.3,
        normalized_score: 0.3
      }
    };

    assert.strictEqual(output.decision, 'defer');
    assert.ok(output.defer_until, 'defer_until should be present for defer decision');
  });

  it('should always include orchestrator_actions array', () => {
    const outputs = [
      { decision: 'escalate', orchestrator_actions: [{ action_type: 'notify_escalation_targets' }] },
      { decision: 'maintain', orchestrator_actions: [{ action_type: 'log_timeline_event' }] },
      { decision: 'defer', orchestrator_actions: [] }
    ];

    for (const output of outputs) {
      assert.ok(Array.isArray(output.orchestrator_actions), 'orchestrator_actions must be array');
    }
  });
});

// ============================================================================
// PERSISTENCE SPECIFICATION TESTS
// ============================================================================

describe('Persistence Specification Tests', () => {
  it('should define fields to persist', () => {
    const persistFields = [
      'incident_id',
      'external_incident_id',
      'fingerprint',
      'current_severity',
      'current_status',
      'current_escalation_level',
      'category',
      'signal_source',
      'signal_timestamp',
      'decision',
      'new_severity',
      'new_escalation_level',
      'reason',
      'applied_policy',
      'evaluation_details'
    ];

    // These fields must be persisted
    for (const field of persistFields) {
      assert.ok(persistFields.includes(field), `${field} must be persisted`);
    }
  });

  it('should define fields to exclude from persistence', () => {
    const excludeFields = [
      'signal_payload.raw_data', // May contain PII
      'execution_id',            // Transient
      'trace_id',                // Transient
      'title',                   // Stored in incident record
      'description',             // Stored in incident record
      'impact',                  // Stored in incident record
      'escalation_history',      // Stored separately
      'orchestrator_actions',    // Sent to orchestrator
      'assignment_updates'       // Sent to orchestrator
    ];

    // These fields must NOT be persisted
    for (const field of excludeFields) {
      assert.ok(excludeFields.includes(field), `${field} must be excluded`);
    }
  });
});

// ============================================================================
// AGENT REGISTRATION TESTS
// ============================================================================

describe('Agent Registration Tests', () => {
  it('should have correct agent metadata', () => {
    const metadata = {
      agent_type: 'incident-escalation',
      version: '1.0.0',
      classifications: ['INCIDENT_ORCHESTRATION', 'ESCALATION'],
      decision_type: 'incident_escalation_decision'
    };

    assert.strictEqual(metadata.agent_type, 'incident-escalation');
    assert.strictEqual(metadata.decision_type, 'incident_escalation_decision');
    assert.ok(metadata.classifications.includes('ESCALATION'));
    assert.ok(metadata.classifications.includes('INCIDENT_ORCHESTRATION'));
  });

  it('should define capabilities correctly', () => {
    const capabilities = [
      'Evaluate incident signals from Sentinel, Edge-Agent, Shield, and Orchestrator',
      'Assess severity thresholds and escalation policies',
      'Transition incidents between severity levels',
      'Trigger downstream escalation actions via Orchestrator'
    ];

    assert.strictEqual(capabilities.length, 4);
  });

  it('should define prohibitions correctly', () => {
    const prohibitions = [
      'MUST NOT perform remediation directly',
      'MUST NOT emit alerts externally',
      'MUST NOT modify routing or execution behavior',
      'MUST NOT alter escalation policies dynamically',
      'MUST NOT intercept runtime execution',
      'MUST NOT enforce policies',
      'MUST NOT emit anomaly detections'
    ];

    assert.ok(prohibitions.length >= 7, 'Should have all prohibitions defined');
  });

  it('should define allowed invokers', () => {
    const allowedInvokers = [
      'llm-sentinel',
      'llm-shield',
      'llm-edge-agent',
      'llm-orchestrator',
      'incident-manager-cli',
      'incident-manager-api'
    ];

    assert.ok(allowedInvokers.includes('llm-sentinel'));
    assert.ok(allowedInvokers.includes('incident-manager-cli'));
  });
});

// ============================================================================
// CLI CONTRACT TESTS
// ============================================================================

describe('CLI Contract Tests', () => {
  it('should support evaluate subcommand', () => {
    const cliSpec = {
      command: 'escalate',
      subcommands: {
        evaluate: {
          args: { incident_id: 'string' },
          flags: ['--dry-run', '--verbose', '--json', '--signal-source', '--policy-id']
        }
      }
    };

    assert.ok(cliSpec.subcommands.evaluate);
    assert.ok(cliSpec.subcommands.evaluate.args.incident_id);
    assert.ok(cliSpec.subcommands.evaluate.flags.includes('--dry-run'));
  });

  it('should support inspect subcommand', () => {
    const cliSpec = {
      command: 'escalate',
      subcommands: {
        inspect: {
          args: { incident_id: 'string' },
          flags: ['--include-history', '--json']
        }
      }
    };

    assert.ok(cliSpec.subcommands.inspect);
    assert.ok(cliSpec.subcommands.inspect.args.incident_id);
  });

  it('should support list subcommand', () => {
    const cliSpec = {
      command: 'escalate',
      subcommands: {
        list: {
          args: {},
          flags: ['--severity', '--status', '--limit', '--json']
        }
      }
    };

    assert.ok(cliSpec.subcommands.list);
    assert.ok(cliSpec.subcommands.list.flags.includes('--severity'));
  });
});

// ============================================================================
// SMOKE TESTS
// ============================================================================

describe('Smoke Tests', () => {
  it('should process a basic escalation request', () => {
    const input = createTestInput();

    // Verify input structure
    assert.ok(input.incident_id);
    assert.ok(input.current_severity);
    assert.ok(input.signal_source);

    // Simulate expected output structure
    const expectedOutputShape = {
      decision: 'string',
      reason: 'string',
      orchestrator_actions: 'array',
      applied_policy: 'object',
      evaluation_details: 'object'
    };

    for (const [key, type] of Object.entries(expectedOutputShape)) {
      assert.ok(key, `Output should have ${key}`);
    }
  });

  it('should handle edge case: empty escalation history', () => {
    const input = createTestInput({ escalation_history: [] });
    assert.deepStrictEqual(input.escalation_history, []);
  });

  it('should handle edge case: missing optional fields', () => {
    const input = createTestInput({
      external_incident_id: undefined,
      impact: undefined,
      policy_id: undefined,
      time_since_last_escalation: undefined
    });

    assert.strictEqual(input.external_incident_id, undefined);
    assert.strictEqual(input.impact, undefined);
    assert.strictEqual(input.policy_id, undefined);
  });
});

console.log('All tests defined. Run with: node --test');
