/**
 * Example Phase 3 Layer 1 Agent Implementation
 *
 * This example demonstrates how to implement a compliant Phase 3 agent
 * that adheres to all startup, execution, and signal requirements.
 *
 * USAGE:
 *   AGENT_PHASE=phase3 AGENT_LAYER=layer1 \
 *   RUVECTOR_API_KEY=<key> RUVECTOR_SERVICE_URL=<url> \
 *   npx ts-node phase3-agent-example.ts
 */

import {
  startupPhase3Layer1,
  createExecutionGuard,
  Phase3HardFailError,
  handleStartupFailure,
  type IncidentSignal,
  type Phase3DecisionEventPayload,
  type RuvectorConfig,
} from '../index.js';

import type { DecisionEvent } from '../decision-event.js';
import type { UUID } from '../common.js';

// Import RuvectorClient from the sibling package
// In production: import { RuvectorClient } from '@agentics/ruvector-client';
// For this example, we define a minimal interface
interface MockRuvectorClient {
  storeDecisionEvent<T>(event: DecisionEvent<T>): Promise<{ success: boolean; data?: { id: UUID }; error?: { message: string } }>;
}

function createRuvectorClient(_config: RuvectorConfig): MockRuvectorClient {
  // In production, use: new RuvectorClient(config) from @agentics/ruvector-client
  return {
    async storeDecisionEvent<T>(event: DecisionEvent<T>) {
      // Mock implementation for example
      console.log('[MOCK] Storing decision event:', event.id);
      return { success: true, data: { id: event.id } };
    }
  };
}

// ============================================================================
// AGENT IMPLEMENTATION
// ============================================================================

interface IncidentInput {
  incident_id: UUID;
  severity: 'P0' | 'P1' | 'P2' | 'P3' | 'P4';
  description: string;
  metrics: Record<string, number>;
}

interface AgentOutput {
  routing_recommendation: string;
  escalation_required: boolean;
  confidence: number;
}

/**
 * Phase 3 Layer 1 Incident Routing Agent
 *
 * This agent:
 * - Coordinates incident routing
 * - Emits incident signals with confidence + references
 * - Does NOT make final decisions
 * - Does NOT emit executive conclusions
 */
async function runAgent(input: IncidentInput): Promise<AgentOutput> {
  // -------------------------------------------------------------------------
  // STARTUP VALIDATION (Required - Causes crashloop on failure)
  // -------------------------------------------------------------------------
  const { ruvectorConfig, validationResult } = await startupPhase3Layer1();

  console.log(`[AGENT] Phase: ${validationResult.phase}, Layer: ${validationResult.layer}`);
  console.log(`[AGENT] RuVector: ${validationResult.ruvector_healthy ? 'connected' : 'disconnected'}`);

  // Create RuVector client from validated config
  const ruvectorClient = createRuvectorClient(ruvectorConfig);

  // -------------------------------------------------------------------------
  // CREATE EXECUTION GUARD
  // -------------------------------------------------------------------------
  const guard = createExecutionGuard('incident-routing-agent', `trace-${Date.now()}`);

  try {
    // -----------------------------------------------------------------------
    // PERFORM COORDINATION (Required role)
    // -----------------------------------------------------------------------
    guard.assertCoordinationOnly(
      'analyze-incident',
      'Analyzing incident metrics for routing coordination'
    );

    // Simulate API call (e.g., fetching additional context)
    guard.recordApiCall();
    guard.recordTokens(250); // Simulated token usage

    // -----------------------------------------------------------------------
    // ANALYZE AND ROUTE (Coordination, not decision)
    // -----------------------------------------------------------------------
    const routingRecommendation = analyzeForRouting(input);

    guard.performRole('route');
    guard.recordTokens(300);

    // -----------------------------------------------------------------------
    // CHECK ESCALATION NEED (Escalation role)
    // -----------------------------------------------------------------------
    const needsEscalation = checkEscalationNeed(input);

    if (needsEscalation) {
      guard.performRole('escalate');
    }

    // -----------------------------------------------------------------------
    // EMIT INCIDENT SIGNAL (Required)
    // -----------------------------------------------------------------------
    const signal: IncidentSignal = {
      signal_type: 'incident_signal',
      incident_id: input.incident_id,
      severity_assessment: input.severity,
      confidence: calculateConfidence(input),
      references: [
        {
          reference_type: 'metric',
          reference_id: 'error_rate',
          reference_source: 'prometheus',
          weight: 0.4,
        },
        {
          reference_type: 'metric',
          reference_id: 'latency_p99',
          reference_source: 'prometheus',
          weight: 0.3,
        },
        {
          reference_type: 'historical',
          reference_id: 'similar_incidents',
          reference_source: 'ruvector',
          weight: 0.3,
        },
      ],
      routing_recommendation: routingRecommendation,
      escalation_required: needsEscalation,
      timestamp: new Date().toISOString(),
    };

    guard.emitSignal(signal);
    guard.recordTokens(200);

    // -----------------------------------------------------------------------
    // FINALIZE EXECUTION
    // -----------------------------------------------------------------------
    const { metrics, audit } = guard.finalize();

    console.log(`[AGENT] Execution metrics:`, metrics);
    console.log(`[AGENT] Audit: compliant=${audit.compliant}`);

    // -----------------------------------------------------------------------
    // STORE DECISION EVENT IN RUVECTOR
    // -----------------------------------------------------------------------
    const decisionPayload: Phase3DecisionEventPayload = {
      phase: 'phase3',
      layer: 'layer1',
      primary_signal: signal,
      execution_role: 'route',
      performance_metrics: metrics,
      audit,
    };

    const decisionEvent: DecisionEvent<Phase3DecisionEventPayload> = {
      id: crypto.randomUUID() as UUID,
      agent_id: 'incident-routing-agent:1.0.0:phase3',
      agent_version: '1.0.0',
      agent_classification: 'INCIDENT_ORCHESTRATION',
      decision_type: 'incident_escalation_decision',
      timestamp: new Date().toISOString(),
      inputs_hash: hashInput(input),
      outputs: decisionPayload,
      confidence: signal.confidence,
      constraints_applied: {},
      execution_ref: audit.execution_id,
      environment: (process.env.PLATFORM_ENV || 'development') as 'development' | 'staging' | 'production',
      requires_review: false,
      audit_metadata: {
        sources: ['incident-routing-agent'],
        policies_evaluated: ['phase3-execution-guard', 'performance-budget'],
      },
    };

    const storeResult = await ruvectorClient.storeDecisionEvent(decisionEvent);

    if (!storeResult.success) {
      console.error('[AGENT] Failed to store decision event:', storeResult.error);
      // Note: This is a soft failure - agent completed but persistence failed
    } else if (storeResult.data) {
      console.log(`[AGENT] Decision event stored: ${storeResult.data.id}`);
    }

    // -----------------------------------------------------------------------
    // RETURN OUTPUT (Signal, not decision)
    // -----------------------------------------------------------------------
    return {
      routing_recommendation: routingRecommendation,
      escalation_required: needsEscalation,
      confidence: signal.confidence,
    };

  } catch (error) {
    if (error instanceof Phase3HardFailError) {
      console.error(`[AGENT] Hard fail: ${error.condition}`);
      throw error; // Re-throw to trigger crashloop
    }
    throw error;
  }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

function analyzeForRouting(input: IncidentInput): string {
  // This is COORDINATION, not a final decision
  // The actual routing decision is made by the orchestrator

  if (input.severity === 'P0' || input.severity === 'P1') {
    return 'team-sre-oncall';
  }

  if (input.metrics['error_rate'] > 0.1) {
    return 'team-platform';
  }

  return 'team-general';
}

function checkEscalationNeed(input: IncidentInput): boolean {
  // Check if escalation is recommended (not decided)
  return input.severity === 'P0' ||
         input.severity === 'P1' ||
         input.metrics['error_rate'] > 0.5;
}

function calculateConfidence(input: IncidentInput): number {
  // Calculate confidence based on available data quality
  let confidence = 0.5;

  // More metrics = higher confidence
  const metricCount = Object.keys(input.metrics).length;
  confidence += Math.min(metricCount * 0.1, 0.3);

  // Known severity = higher confidence
  if (input.severity) {
    confidence += 0.1;
  }

  return Math.min(confidence, 0.95);
}

function hashInput(input: IncidentInput): string {
  // Simple hash for demo purposes
  const str = JSON.stringify(input);
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash;
  }
  return Math.abs(hash).toString(16);
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

async function main() {
  console.log('='.repeat(60));
  console.log('Phase 3 Layer 1 - Incident Routing Agent');
  console.log('='.repeat(60));

  const testInput: IncidentInput = {
    incident_id: '550e8400-e29b-41d4-a716-446655440000' as UUID,
    severity: 'P2',
    description: 'Elevated error rate in payment service',
    metrics: {
      error_rate: 0.15,
      latency_p99: 500,
      request_count: 10000,
    },
  };

  try {
    const output = await runAgent(testInput);
    console.log('='.repeat(60));
    console.log('Agent Output (Signal, not Decision):');
    console.log(JSON.stringify(output, null, 2));
    console.log('='.repeat(60));
  } catch (error) {
    if (error instanceof Phase3HardFailError) {
      handleStartupFailure(error);
    }
    console.error('Agent failed:', error);
    process.exit(1);
  }
}

// Only run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { runAgent, type IncidentInput, type AgentOutput };
