/**
 * @agentics/escalation-agent
 *
 * Incident Escalation Agent for Agentics Dev platform.
 *
 * Classification:
 *   - INCIDENT ORCHESTRATION
 *   - ESCALATION
 *
 * Purpose:
 *   Determine incident severity changes and trigger controlled escalation
 *   across predefined escalation ladders.
 *
 * This agent MAY:
 *   - Evaluate incident signals from Sentinel, Edge-Agent, Shield, and Orchestrator
 *   - Assess severity thresholds and escalation policies
 *   - Transition incidents between severity levels (e.g., SEV3 → SEV2 → SEV1)
 *   - Trigger downstream escalation actions via Orchestrator
 *
 * This agent MUST NOT:
 *   - Perform remediation directly
 *   - Emit alerts externally (email, pager, webhook)
 *   - Modify routing or execution behavior
 *   - Alter escalation policies dynamically
 *   - Intercept runtime execution
 *   - Enforce policies (that is Shield)
 *   - Emit anomaly detections (that is Sentinel)
 *
 * @packageDocumentation
 */

// Export the handler for Google Cloud Functions
export { handler, handleRequest } from './handler.js';

// Export the core logic
export { EscalationDecisionEngine, type EscalationResult } from './engine.js';

// Export configuration
export { createAgentConfig, validateConfig, type AgentConfig } from './config.js';

// Export registration
export { AGENT_REGISTRATION, verifyRegistration, getDeploymentConfig } from './registration.js';

// Export health check
export { checkHealth, healthHandler } from './health.js';

// Export types from contracts
export type {
  EscalationAgentInput,
  EscalationAgentOutput,
  EscalationDecisionEvent
} from '@agentics/contracts';
