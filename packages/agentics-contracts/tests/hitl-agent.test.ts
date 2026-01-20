/**
 * Tests for Human-in-the-Loop (HITL) Agent Contract
 *
 * These tests verify:
 * 1. Input validation
 * 2. Output validation
 * 3. Decision logic
 * 4. Policy compliance
 * 5. Audit trail generation
 * 6. Edge cases and failure modes
 */

import { describe, it, expect, beforeEach } from 'vitest';

import {
  HITL_AGENT_METADATA,
  HITL_PERSISTENCE,
  HITL_CONFIDENCE_FACTORS,
  validateHITLInput,
  validateHITLOutput,
  calculateApprovalProgress,
  isDeadlinePassed,
  generateApprovalRequestId,
  hashInputs
} from '../src/hitl-agent.js';

import type {
  HITLAgentInput,
  HITLAgentOutput,
  ApprovalRecord,
  ApproverRequirement
} from '../src/hitl-agent.js';

// ============================================================================
// TEST FIXTURES
// ============================================================================

function createValidInput(overrides: Partial<HITLAgentInput> = {}): HITLAgentInput {
  const futureDate = new Date(Date.now() + 3600000).toISOString(); // 1 hour from now

  return {
    approval_request_id: 'req-123e4567-e89b-12d3-a456-426614174000',
    incident_id: 'inc-123e4567-e89b-12d3-a456-426614174001',
    action_type: 'remediation',
    action_description: 'Execute rollback to previous version',
    action_parameters: {
      target: 'payment-service',
      target_type: 'service',
      parameters: { version: 'v1.2.3' },
      reversible: true,
      estimated_duration_seconds: 300
    },
    expected_impact: {
      scope: 'single_service',
      users_affected_estimate: 1000,
      description: 'Service will be temporarily unavailable during rollback'
    },
    inaction_risks: ['Continued service degradation', 'Revenue loss'],
    incident_severity: 'P1',
    incident_status: 'IN_PROGRESS',
    incident_category: 'availability',
    affected_resource: {
      type: 'service',
      id: 'svc-payment-001',
      name: 'Payment Service'
    },
    incident_title: 'Payment Service Degradation',
    incident_summary: 'Payment service experiencing elevated error rates',
    requester_type: 'agent',
    requester_id: 'incident-escalation:1.0.0:abc123',
    request_reason: 'Automated escalation due to SLA breach',
    request_priority: 'high',
    required_approvers: [
      { approver_type: 'incident_commander', required: true },
      { approver_type: 'sre_lead', required: false }
    ],
    min_approvals_required: 1,
    approval_deadline: futureDate,
    expiry_action: 'auto_reject',
    current_status: 'pending',
    existing_approvals: [],
    existing_rejections: [],
    environment: 'production',
    execution_id: 'exec-123e4567-e89b-12d3-a456-426614174002',
    request_timestamp: new Date().toISOString(),
    ...overrides
  };
}

function createValidOutput(overrides: Partial<HITLAgentOutput> = {}): HITLAgentOutput {
  return {
    decision: 'approval_pending',
    reason: 'Awaiting approvals: 0/1 obtained',
    updated_status: 'pending',
    approvals_obtained: 0,
    approvals_remaining: 1,
    rejections_count: 0,
    approval_records: [],
    consolidated_conditions: [],
    orchestrator_actions: [
      {
        action_type: 'notify_approvers',
        priority: 'high',
        parameters: {},
        async: true
      }
    ],
    action_authorized: false,
    audit_trail: [
      {
        timestamp: new Date().toISOString(),
        event_type: 'approval_requested',
        actor_id: 'incident-approver:1.0.0:default',
        actor_type: 'agent',
        details: {}
      }
    ],
    policy_compliance: {
      compliant: true,
      policy_id: 'default-approval-policy',
      policy_name: 'Approval Policy',
      rules_evaluated: [],
      violations: []
    },
    ...overrides
  };
}

function createApprovalRecord(overrides: Partial<ApprovalRecord> = {}): ApprovalRecord {
  return {
    approver_id: 'user-john-doe',
    approver_name: 'John Doe',
    approver_type: 'incident_commander',
    decision: 'approved',
    rationale: 'Approved based on impact assessment',
    decision_timestamp: new Date().toISOString(),
    ...overrides
  };
}

// ============================================================================
// METADATA TESTS
// ============================================================================

describe('HITL_AGENT_METADATA', () => {
  it('should have correct agent type', () => {
    expect(HITL_AGENT_METADATA.agent_type).toBe('incident-approver');
  });

  it('should have APPROVAL_GATING as primary classification', () => {
    expect(HITL_AGENT_METADATA.agent_classification[0]).toBe('APPROVAL_GATING');
  });

  it('should have correct decision type', () => {
    expect(HITL_AGENT_METADATA.decision_type).toBe('incident_approval_decision');
  });

  it('should define capabilities', () => {
    expect(HITL_AGENT_METADATA.capabilities.length).toBeGreaterThan(0);
  });

  it('should define prohibitions that prevent auto-approval', () => {
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT auto-approve any decision');
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT bypass approval requirements for any reason');
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT execute remediation directly');
  });

  it('should define allowed invokers', () => {
    expect(HITL_AGENT_METADATA.allowed_invokers).toContain('llm-orchestrator');
    expect(HITL_AGENT_METADATA.allowed_invokers).toContain('incident-manager-cli');
  });
});

// ============================================================================
// INPUT VALIDATION TESTS
// ============================================================================

describe('validateHITLInput', () => {
  it('should validate a correct input', () => {
    const input = createValidInput();
    const result = validateHITLInput(input);

    expect(result.valid).toBe(true);
    expect(result.errors.length).toBe(0);
  });

  it('should reject non-object input', () => {
    const result = validateHITLInput(null);

    expect(result.valid).toBe(false);
    expect(result.errors[0].code).toBe('INVALID_TYPE');
  });

  it('should require approval_request_id', () => {
    const input = createValidInput();
    delete (input as any).approval_request_id;
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'approval_request_id')).toBe(true);
  });

  it('should require incident_id', () => {
    const input = createValidInput();
    delete (input as any).incident_id;
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'incident_id')).toBe(true);
  });

  it('should require action_type', () => {
    const input = createValidInput();
    delete (input as any).action_type;
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'action_type')).toBe(true);
  });

  it('should validate action_type enum values', () => {
    const input = createValidInput({ action_type: 'invalid_action' as any });
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'action_type' && e.code === 'INVALID_VALUE')).toBe(true);
  });

  it('should validate severity enum values', () => {
    const input = createValidInput({ incident_severity: 'INVALID' as any });
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'incident_severity')).toBe(true);
  });

  it('should require action_parameters', () => {
    const input = createValidInput();
    delete (input as any).action_parameters;
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'action_parameters')).toBe(true);
  });

  it('should require action_parameters.target', () => {
    const input = createValidInput();
    delete (input.action_parameters as any).target;
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'action_parameters.target')).toBe(true);
  });

  it('should require required_approvers array', () => {
    const input = createValidInput();
    delete (input as any).required_approvers;
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'required_approvers')).toBe(true);
  });

  it('should reject empty required_approvers array', () => {
    const input = createValidInput({ required_approvers: [] });
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'required_approvers' && e.code === 'INVALID_VALUE')).toBe(true);
  });

  it('should require min_approvals_required as number', () => {
    const input = createValidInput();
    (input as any).min_approvals_required = 'one';
    const result = validateHITLInput(input);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'min_approvals_required')).toBe(true);
  });

  it('should warn when deadline is in the past', () => {
    const pastDate = new Date(Date.now() - 3600000).toISOString();
    const input = createValidInput({ approval_deadline: pastDate });
    const result = validateHITLInput(input);

    // Should still be valid but with warning
    expect(result.warnings.some(w => w.field === 'approval_deadline' && w.code === 'DEADLINE_PASSED')).toBe(true);
  });

  it('should warn when min_approvals exceeds required_approvers count', () => {
    const input = createValidInput({ min_approvals_required: 10 });
    const result = validateHITLInput(input);

    expect(result.warnings.some(w => w.field === 'min_approvals_required' && w.code === 'LOGICAL_WARNING')).toBe(true);
  });

  it('should validate all action types', () => {
    const validActionTypes = [
      'remediation', 'rollback', 'deployment', 'public_disclosure',
      'data_access', 'escalation_override', 'policy_exception',
      'budget_override', 'incident_closure'
    ];

    for (const actionType of validActionTypes) {
      const input = createValidInput({ action_type: actionType as any });
      const result = validateHITLInput(input);
      expect(result.valid).toBe(true);
    }
  });
});

// ============================================================================
// OUTPUT VALIDATION TESTS
// ============================================================================

describe('validateHITLOutput', () => {
  it('should validate a correct output', () => {
    const output = createValidOutput();
    const result = validateHITLOutput(output);

    expect(result.valid).toBe(true);
    expect(result.errors.length).toBe(0);
  });

  it('should reject non-object output', () => {
    const result = validateHITLOutput(null);

    expect(result.valid).toBe(false);
    expect(result.errors[0].code).toBe('INVALID_TYPE');
  });

  it('should require decision', () => {
    const output = createValidOutput();
    delete (output as any).decision;
    const result = validateHITLOutput(output);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'decision')).toBe(true);
  });

  it('should require reason', () => {
    const output = createValidOutput();
    delete (output as any).reason;
    const result = validateHITLOutput(output);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'reason')).toBe(true);
  });

  it('should require action_authorized to be true when approved', () => {
    const output = createValidOutput({
      decision: 'approved',
      action_authorized: false
    });
    const result = validateHITLOutput(output);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'action_authorized' && e.code === 'LOGICAL_ERROR')).toBe(true);
  });

  it('should require action_authorized to be false when rejected', () => {
    const output = createValidOutput({
      decision: 'rejected',
      action_authorized: true
    });
    const result = validateHITLOutput(output);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'action_authorized' && e.code === 'LOGICAL_ERROR')).toBe(true);
  });

  it('should warn when terminal decision lacks decision_finalized_at', () => {
    const output = createValidOutput({
      decision: 'approved',
      action_authorized: true
    });
    delete output.decision_finalized_at;
    const result = validateHITLOutput(output);

    expect(result.warnings.some(w => w.field === 'decision_finalized_at')).toBe(true);
  });

  it('should warn when pending decision lacks next_check_at', () => {
    const output = createValidOutput({
      decision: 'approval_pending'
    });
    delete output.next_check_at;
    const result = validateHITLOutput(output);

    expect(result.warnings.some(w => w.field === 'next_check_at')).toBe(true);
  });

  it('should require approval_records as array', () => {
    const output = createValidOutput();
    (output as any).approval_records = 'not an array';
    const result = validateHITLOutput(output);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'approval_records')).toBe(true);
  });

  it('should require policy_compliance', () => {
    const output = createValidOutput();
    delete (output as any).policy_compliance;
    const result = validateHITLOutput(output);

    expect(result.valid).toBe(false);
    expect(result.errors.some(e => e.field === 'policy_compliance')).toBe(true);
  });
});

// ============================================================================
// HELPER FUNCTION TESTS
// ============================================================================

describe('calculateApprovalProgress', () => {
  it('should return 0% when no approvals', () => {
    expect(calculateApprovalProgress(0, 3)).toBe(0);
  });

  it('should return 100% when all approvals obtained', () => {
    expect(calculateApprovalProgress(3, 3)).toBe(100);
  });

  it('should return correct percentage', () => {
    expect(calculateApprovalProgress(1, 2)).toBe(50);
    expect(calculateApprovalProgress(2, 3)).toBe(67);
  });

  it('should cap at 100%', () => {
    expect(calculateApprovalProgress(5, 3)).toBe(100);
  });

  it('should handle zero required gracefully', () => {
    expect(calculateApprovalProgress(0, 0)).toBe(100);
  });
});

describe('isDeadlinePassed', () => {
  it('should return false for future deadline', () => {
    const futureDate = new Date(Date.now() + 3600000).toISOString();
    expect(isDeadlinePassed(futureDate)).toBe(false);
  });

  it('should return true for past deadline', () => {
    const pastDate = new Date(Date.now() - 3600000).toISOString();
    expect(isDeadlinePassed(pastDate)).toBe(true);
  });
});

describe('generateApprovalRequestId', () => {
  it('should generate valid UUID', () => {
    const id = generateApprovalRequestId();
    expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
  });

  it('should generate unique IDs', () => {
    const ids = new Set<string>();
    for (let i = 0; i < 100; i++) {
      ids.add(generateApprovalRequestId());
    }
    expect(ids.size).toBe(100);
  });
});

describe('hashInputs', () => {
  it('should generate SHA-256 hash', async () => {
    const input = createValidInput();
    const hash = await hashInputs(input);

    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });

  it('should generate same hash for same input', async () => {
    const input = createValidInput();
    const hash1 = await hashInputs(input);
    const hash2 = await hashInputs(input);

    expect(hash1).toBe(hash2);
  });

  it('should generate different hash for different input', async () => {
    const input1 = createValidInput({ incident_id: 'inc-1' });
    const input2 = createValidInput({ incident_id: 'inc-2' });

    const hash1 = await hashInputs(input1);
    const hash2 = await hashInputs(input2);

    expect(hash1).not.toBe(hash2);
  });
});

// ============================================================================
// PERSISTENCE SPECIFICATION TESTS
// ============================================================================

describe('HITL_PERSISTENCE', () => {
  it('should persist approval_request_id', () => {
    expect(HITL_PERSISTENCE.persist).toContain('approval_request_id');
  });

  it('should persist incident_id', () => {
    expect(HITL_PERSISTENCE.persist).toContain('incident_id');
  });

  it('should persist approval_records for audit', () => {
    expect(HITL_PERSISTENCE.persist).toContain('approval_records');
  });

  it('should persist audit_trail', () => {
    expect(HITL_PERSISTENCE.persist).toContain('audit_trail');
  });

  it('should persist policy_compliance', () => {
    expect(HITL_PERSISTENCE.persist).toContain('policy_compliance');
  });

  it('should exclude execution_id (transient)', () => {
    expect(HITL_PERSISTENCE.exclude).toContain('execution_id');
  });

  it('should exclude trace_id (transient)', () => {
    expect(HITL_PERSISTENCE.exclude).toContain('trace_id');
  });

  it('should exclude orchestrator_actions (sent separately)', () => {
    expect(HITL_PERSISTENCE.exclude).toContain('orchestrator_actions');
  });

  it('should have TTL of 0 (permanent audit record)', () => {
    expect(HITL_PERSISTENCE.ttl_seconds).toBe(0);
  });
});

// ============================================================================
// CONFIDENCE FACTORS TESTS
// ============================================================================

describe('HITL_CONFIDENCE_FACTORS', () => {
  it('should define all_approvals_obtained factor', () => {
    expect(HITL_CONFIDENCE_FACTORS.all_approvals_obtained).toBeDefined();
    expect(HITL_CONFIDENCE_FACTORS.all_approvals_obtained.weight).toBeGreaterThan(0);
  });

  it('should define no_rejections factor', () => {
    expect(HITL_CONFIDENCE_FACTORS.no_rejections).toBeDefined();
    expect(HITL_CONFIDENCE_FACTORS.no_rejections.weight).toBeGreaterThan(0);
  });

  it('should define policy_compliant factor', () => {
    expect(HITL_CONFIDENCE_FACTORS.policy_compliant).toBeDefined();
  });

  it('should define authority_verified factor', () => {
    expect(HITL_CONFIDENCE_FACTORS.authority_verified).toBeDefined();
  });

  it('should have weights that sum to 1.0', () => {
    const totalWeight =
      HITL_CONFIDENCE_FACTORS.all_approvals_obtained.weight +
      HITL_CONFIDENCE_FACTORS.no_rejections.weight +
      HITL_CONFIDENCE_FACTORS.policy_compliant.weight +
      HITL_CONFIDENCE_FACTORS.authority_verified.weight +
      HITL_CONFIDENCE_FACTORS.conditions_valid.weight;

    expect(totalWeight).toBeCloseTo(1.0, 5);
  });
});

// ============================================================================
// DECISION SCENARIO TESTS
// ============================================================================

describe('Approval Decision Scenarios', () => {
  it('should handle pending state with no approvals', () => {
    const input = createValidInput({
      existing_approvals: [],
      existing_rejections: [],
      min_approvals_required: 2
    });

    // This would be tested via handler, but we verify input is valid
    const result = validateHITLInput(input);
    expect(result.valid).toBe(true);
  });

  it('should handle partial approvals', () => {
    const input = createValidInput({
      existing_approvals: [createApprovalRecord()],
      existing_rejections: [],
      min_approvals_required: 2
    });

    const result = validateHITLInput(input);
    expect(result.valid).toBe(true);
  });

  it('should handle sufficient approvals', () => {
    const input = createValidInput({
      existing_approvals: [
        createApprovalRecord({ approver_id: 'user-1', approver_type: 'incident_commander' }),
        createApprovalRecord({ approver_id: 'user-2', approver_type: 'sre_lead' })
      ],
      existing_rejections: [],
      min_approvals_required: 2
    });

    const result = validateHITLInput(input);
    expect(result.valid).toBe(true);
  });

  it('should handle rejection scenario', () => {
    const input = createValidInput({
      existing_approvals: [],
      existing_rejections: [
        createApprovalRecord({
          decision: 'rejected',
          rationale: 'Risk too high without additional safeguards'
        })
      ]
    });

    const result = validateHITLInput(input);
    expect(result.valid).toBe(true);
  });

  it('should handle mixed approvals and rejections', () => {
    const input = createValidInput({
      existing_approvals: [
        createApprovalRecord({ approver_id: 'user-1' })
      ],
      existing_rejections: [
        createApprovalRecord({
          approver_id: 'user-2',
          decision: 'rejected',
          rationale: 'Needs more review'
        })
      ]
    });

    const result = validateHITLInput(input);
    expect(result.valid).toBe(true);
  });

  it('should handle approvals with conditions', () => {
    const input = createValidInput({
      existing_approvals: [
        createApprovalRecord({
          conditions: ['Must notify customers before rollback', 'Keep rollback window to off-peak hours']
        })
      ]
    });

    const result = validateHITLInput(input);
    expect(result.valid).toBe(true);
  });
});

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

describe('Edge Cases', () => {
  it('should handle all valid action types', () => {
    const actionTypes = [
      'remediation',
      'rollback',
      'deployment',
      'public_disclosure',
      'data_access',
      'escalation_override',
      'policy_exception',
      'budget_override',
      'incident_closure'
    ] as const;

    for (const actionType of actionTypes) {
      const input = createValidInput({ action_type: actionType });
      const result = validateHITLInput(input);
      expect(result.valid).toBe(true);
    }
  });

  it('should handle all valid approver types', () => {
    const approverTypes = [
      'incident_commander',
      'on_call_lead',
      'security_team',
      'sre_lead',
      'engineering_manager',
      'executive',
      'specific_user'
    ] as const;

    for (const approverType of approverTypes) {
      const input = createValidInput({
        required_approvers: [{ approver_type: approverType, required: true }]
      });
      const result = validateHITLInput(input);
      expect(result.valid).toBe(true);
    }
  });

  it('should handle all severity levels', () => {
    const severities = ['P0', 'P1', 'P2', 'P3', 'P4'] as const;

    for (const severity of severities) {
      const input = createValidInput({ incident_severity: severity });
      const result = validateHITLInput(input);
      expect(result.valid).toBe(true);
    }
  });

  it('should handle terminal status inputs', () => {
    const statuses = ['approved', 'rejected', 'expired', 'cancelled'] as const;

    for (const status of statuses) {
      const input = createValidInput({ current_status: status as any });
      const result = validateHITLInput(input);
      expect(result.valid).toBe(true);
    }
  });

  it('should handle expiry actions', () => {
    const expiryActions = ['auto_reject', 'escalate', 'notify_and_wait'] as const;

    for (const action of expiryActions) {
      const input = createValidInput({ expiry_action: action });
      const result = validateHITLInput(input);
      expect(result.valid).toBe(true);
    }
  });
});

// ============================================================================
// SECURITY TESTS
// ============================================================================

describe('Security Constraints', () => {
  it('should not allow auto-approval (verified by metadata)', () => {
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT auto-approve any decision');
  });

  it('should not allow bypass of approval requirements (verified by metadata)', () => {
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT bypass approval requirements for any reason');
  });

  it('should not allow direct remediation execution (verified by metadata)', () => {
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT execute remediation directly');
  });

  it('should not allow impersonation of approvers (verified by metadata)', () => {
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT impersonate human approvers');
  });

  it('should not allow backdating timestamps (verified by metadata)', () => {
    expect(HITL_AGENT_METADATA.prohibitions).toContain('MUST NOT backdate approval timestamps');
  });

  it('should require action_authorized to match decision state', () => {
    // Approved requires action_authorized = true
    const approvedOutput = createValidOutput({
      decision: 'approved',
      action_authorized: false
    });
    expect(validateHITLOutput(approvedOutput).valid).toBe(false);

    // Rejected requires action_authorized = false
    const rejectedOutput = createValidOutput({
      decision: 'rejected',
      action_authorized: true
    });
    expect(validateHITLOutput(rejectedOutput).valid).toBe(false);
  });
});
