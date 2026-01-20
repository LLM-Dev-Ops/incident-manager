/**
 * Incident Escalation Agent Handler
 *
 * Google Cloud Edge Function entry point.
 *
 * ARCHITECTURAL REQUIREMENTS:
 * - Stateless execution
 * - Deterministic behavior
 * - Exactly ONE DecisionEvent per invocation
 * - All persistence via ruvector-service
 */

import { createHash } from 'crypto';
import {
  validateEscalationInput,
  validateEscalationOutput,
  DecisionEventBuilder,
  ESCALATION_AGENT_METADATA,
  ESCALATION_PERSISTENCE,
  type EscalationAgentInput,
  type EscalationAgentOutput,
  type EscalationDecisionEvent,
  type UUID,
  type AgentResult
} from '@agentics/contracts';
import {
  RuvectorClient,
  DecisionEventStore,
  TelemetryEmitter
} from '@agentics/ruvector-client';
import { createAgentConfig, validateConfig, type AgentConfig } from './config.js';
import { EscalationDecisionEngine } from './engine.js';

// ============================================================================
// TYPES
// ============================================================================

interface EdgeFunctionRequest {
  body?: string | EscalationAgentInput;
  headers?: Record<string, string>;
  method?: string;
}

interface EdgeFunctionResponse {
  statusCode: number;
  headers: Record<string, string>;
  body: string;
}

interface HandlerResult {
  decision_event: EscalationDecisionEvent;
  persisted: boolean;
  warnings: string[];
}

// ============================================================================
// HANDLER
// ============================================================================

/**
 * Google Cloud Edge Function handler
 *
 * This is the entry point for the serverless function.
 */
export async function handler(
  request: EdgeFunctionRequest
): Promise<EdgeFunctionResponse> {
  const config = createAgentConfig();
  const executionId = generateExecutionId();

  // Validate config
  const configValidation = validateConfig(config);
  if (!configValidation.valid) {
    return errorResponse(500, 'AGENT_MISCONFIGURED', configValidation.errors.join('; '), executionId);
  }

  // Initialize telemetry
  const telemetry = new TelemetryEmitter({
    endpoint: config.telemetry.endpoint,
    apiKey: config.telemetry.apiKey,
    serviceName: 'incident-escalation-agent',
    environment: config.environment,
    enabled: config.telemetry.enabled
  });

  try {
    // Emit invocation telemetry
    telemetry.emitInvocation(config.agentId, executionId);

    // Parse and validate input
    const parseResult = parseInput(request);
    if (!parseResult.success) {
      return errorResponse(400, parseResult.error.code, parseResult.error.message, executionId);
    }

    const input = parseResult.data;

    // Validate input against contract
    const inputValidation = validateEscalationInput(input);
    if (!inputValidation.valid) {
      return errorResponse(
        400,
        'INVALID_INPUT',
        inputValidation.errors.map(e => e.message).join('; '),
        executionId
      );
    }

    // Process the request
    const result = await handleRequest(input, config, executionId, telemetry);

    // Emit decision telemetry
    telemetry.emitDecision(
      config.agentId,
      executionId,
      'incident_escalation_decision',
      result.decision_event.confidence,
      {
        decision: result.decision_event.outputs.decision,
        incident_id: input.incident_id
      }
    );

    // Return success response
    return successResponse(result, executionId);

  } catch (error) {
    const err = error as Error;

    // Emit error telemetry
    telemetry.emitError(config.agentId, executionId, err);

    return errorResponse(
      500,
      'INTERNAL_ERROR',
      config.debug ? err.message : 'Internal processing error',
      executionId
    );

  } finally {
    // Flush telemetry
    await telemetry.shutdown();
  }
}

/**
 * Core request handler (also exported for direct invocation)
 */
export async function handleRequest(
  input: EscalationAgentInput,
  config: AgentConfig,
  executionId: UUID,
  telemetry?: TelemetryEmitter
): Promise<HandlerResult> {
  const warnings: string[] = [];

  // Initialize ruvector client
  const ruvectorClient = new RuvectorClient({
    baseUrl: config.ruvector.baseUrl,
    apiKey: config.ruvector.apiKey,
    timeoutMs: config.ruvector.timeoutMs
  });
  const decisionStore = new DecisionEventStore(ruvectorClient);

  // Initialize decision engine
  const engine = new EscalationDecisionEngine(config);

  // Execute decision logic
  const result = engine.evaluate(input);

  // Validate output
  const outputValidation = validateEscalationOutput(result.output);
  if (!outputValidation.valid) {
    throw new Error(`Output validation failed: ${outputValidation.errors.map(e => e.message).join('; ')}`);
  }
  if (outputValidation.warnings.length > 0) {
    warnings.push(...outputValidation.warnings.map(w => w.message));
  }

  // Calculate inputs hash for audit trail
  const inputsHash = hashInputs(input);

  // Build DecisionEvent
  const decisionEvent = new DecisionEventBuilder<EscalationAgentOutput>()
    .withId(generateEventId())
    .withAgent(
      config.agentId,
      config.agentVersion,
      'ESCALATION'
    )
    .withDecision(
      'incident_escalation_decision',
      result.output,
      inputsHash
    )
    .withConfidence(result.confidence, result.confidenceFactors)
    .withConstraints({
      severity_thresholds: result.output.evaluation_details.thresholds_evaluated.map(t => ({
        threshold: t.name as any,
        direction: t.breached ? 'below' : 'above',
        triggered: t.breached
      })),
      policy_constraints: [{
        policy_id: result.output.applied_policy.policy_id,
        policy_name: result.output.applied_policy.policy_name,
        policy_version: result.output.applied_policy.policy_version,
        satisfied: true
      }]
    })
    .withExecutionContext(executionId, config.environment)
    .build();

  // Persist DecisionEvent to ruvector-service
  // THIS IS THE CANONICAL OUTPUT - exactly ONE DecisionEvent per invocation
  let persisted = false;
  const storeResult = await decisionStore.store(decisionEvent, ESCALATION_PERSISTENCE);

  if (storeResult.success) {
    persisted = true;

    // Also update escalation state
    await ruvectorClient.updateIncidentState(input.incident_id, {
      severity: result.output.new_severity ?? input.current_severity,
      escalation_level: result.output.new_escalation_level ?? input.current_escalation_level,
      status: result.output.decision === 'escalate' ? 'ESCALATED' : undefined
    });
  } else {
    warnings.push(`Failed to persist DecisionEvent: ${storeResult.error.message}`);
  }

  // Emit metrics
  if (telemetry) {
    telemetry.emitMetrics(config.agentId, executionId, {
      processing_time_ms: result.processingTimeMs,
      confidence: result.confidence,
      thresholds_breached: result.output.evaluation_details.thresholds_evaluated.filter(t => t.breached).length,
      escalation_level: result.output.new_escalation_level ?? input.current_escalation_level
    });
  }

  return {
    decision_event: decisionEvent,
    persisted,
    warnings
  };
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

function parseInput(request: EdgeFunctionRequest): AgentResult<EscalationAgentInput> {
  try {
    let input: EscalationAgentInput;

    if (typeof request.body === 'string') {
      input = JSON.parse(request.body);
    } else if (typeof request.body === 'object' && request.body !== null) {
      input = request.body as EscalationAgentInput;
    } else {
      return {
        success: false,
        error: {
          code: 'INVALID_BODY',
          message: 'Request body must be a valid JSON object',
          details: {},
          retryable: false
        }
      };
    }

    return { success: true, data: input };
  } catch (err) {
    return {
      success: false,
      error: {
        code: 'PARSE_ERROR',
        message: `Failed to parse request body: ${(err as Error).message}`,
        details: {},
        retryable: false
      }
    };
  }
}

function hashInputs(input: EscalationAgentInput): string {
  const hash = createHash('sha256');
  hash.update(JSON.stringify(input));
  return hash.digest('hex');
}

function generateExecutionId(): UUID {
  return crypto.randomUUID();
}

function generateEventId(): UUID {
  return crypto.randomUUID();
}

function successResponse(result: HandlerResult, executionId: UUID): EdgeFunctionResponse {
  return {
    statusCode: 200,
    headers: {
      'Content-Type': 'application/json',
      'X-Execution-Id': executionId,
      'X-Agent-Id': ESCALATION_AGENT_METADATA.agent_type,
      'X-Agent-Version': ESCALATION_AGENT_METADATA.version
    },
    body: JSON.stringify({
      success: true,
      execution_id: executionId,
      decision_event: result.decision_event,
      persisted: result.persisted,
      warnings: result.warnings
    })
  };
}

function errorResponse(
  statusCode: number,
  code: string,
  message: string,
  executionId: UUID
): EdgeFunctionResponse {
  return {
    statusCode,
    headers: {
      'Content-Type': 'application/json',
      'X-Execution-Id': executionId,
      'X-Error-Code': code
    },
    body: JSON.stringify({
      success: false,
      execution_id: executionId,
      error: {
        code,
        message
      }
    })
  };
}
