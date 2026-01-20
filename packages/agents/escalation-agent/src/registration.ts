/**
 * Agent Registration
 *
 * Registration metadata for the Incident Escalation Agent.
 * This file provides the canonical registration entry for the agent
 * in the agentics-contracts registry.
 */

import { ESCALATION_AGENT_METADATA } from '@agentics/contracts';

// ============================================================================
// REGISTRATION METADATA
// ============================================================================

/**
 * Agent registration entry for the incident manager platform
 */
export const AGENT_REGISTRATION = {
  // Identification
  id: 'incident-escalation-agent',
  name: 'Incident Escalation Agent',
  version: ESCALATION_AGENT_METADATA.version,

  // Classification
  type: ESCALATION_AGENT_METADATA.agent_type,
  classifications: ESCALATION_AGENT_METADATA.agent_classification,
  decision_type: ESCALATION_AGENT_METADATA.decision_type,

  // Capabilities (what this agent MAY do)
  capabilities: ESCALATION_AGENT_METADATA.capabilities,

  // Prohibitions (what this agent MUST NOT do)
  prohibitions: ESCALATION_AGENT_METADATA.prohibitions,

  // Allowed invokers
  allowed_invokers: ESCALATION_AGENT_METADATA.allowed_invokers,

  // Endpoint configuration
  endpoints: {
    // Google Cloud Edge Function endpoint
    edge_function: {
      path: '/api/v1/agents/escalation/evaluate',
      method: 'POST',
      content_type: 'application/json'
    },

    // CLI endpoint
    cli: {
      command: 'escalate',
      subcommands: ['evaluate', 'inspect', 'list']
    },

    // Health check
    health: {
      path: '/api/v1/agents/escalation/health',
      method: 'GET'
    }
  },

  // Dependencies
  dependencies: {
    // Required external services
    ruvector_service: {
      required: true,
      version: '>=1.0.0'
    },
    llm_observatory: {
      required: false,
      version: '>=1.0.0'
    }
  },

  // Configuration schema
  configuration: {
    environment_variables: [
      { name: 'RUVECTOR_BASE_URL', required: true, description: 'ruvector-service base URL' },
      { name: 'RUVECTOR_API_KEY', required: true, description: 'API key for ruvector-service' },
      { name: 'RUVECTOR_TIMEOUT_MS', required: false, default: '30000', description: 'Request timeout' },
      { name: 'TELEMETRY_ENDPOINT', required: false, description: 'LLM-Observatory endpoint' },
      { name: 'TELEMETRY_API_KEY', required: false, description: 'Telemetry API key' },
      { name: 'TELEMETRY_ENABLED', required: false, default: 'true', description: 'Enable telemetry' },
      { name: 'ENVIRONMENT', required: false, default: 'production', description: 'Deployment environment' },
      { name: 'MAX_ESCALATION_LEVEL', required: false, default: '5', description: 'Maximum escalation level' },
      { name: 'MIN_ESCALATION_INTERVAL', required: false, default: '300', description: 'Minimum seconds between escalations' },
      { name: 'AUTO_ESCALATION_THRESHOLD', required: false, default: '0.8', description: 'Confidence threshold for auto-escalation' },
      { name: 'REQUIRE_APPROVAL_P0', required: false, default: 'true', description: 'Require approval for P0 escalations' },
      { name: 'DEBUG', required: false, default: 'false', description: 'Enable debug mode' }
    ]
  },

  // Deployment specification
  deployment: {
    platform: 'google-cloud-functions',
    runtime: 'nodejs20',
    memory: '256MB',
    timeout: '30s',
    max_instances: 100,
    min_instances: 0,
    vpc_connector: null, // Can be configured for private VPC access
    service_account: null // Use default or specify custom
  },

  // Documentation
  documentation: {
    readme: 'https://github.com/globalbusinessadvisors/llm-incident-manager/tree/main/packages/agents/escalation-agent',
    api_spec: 'https://github.com/globalbusinessadvisors/llm-incident-manager/blob/main/packages/agentics-contracts/src/escalation-agent.ts',
    examples: 'https://github.com/globalbusinessadvisors/llm-incident-manager/tree/main/examples/escalation-agent'
  }
} as const;

/**
 * Verify registration compliance
 */
export function verifyRegistration(): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  // Verify required fields
  if (!AGENT_REGISTRATION.id) errors.push('Agent ID is required');
  if (!AGENT_REGISTRATION.version) errors.push('Version is required');
  if (!AGENT_REGISTRATION.type) errors.push('Type is required');
  if (AGENT_REGISTRATION.classifications.length === 0) {
    errors.push('At least one classification is required');
  }

  // Verify classification compliance
  const validClassifications = ['INCIDENT_ORCHESTRATION', 'ESCALATION', 'APPROVAL_GATING'];
  for (const c of AGENT_REGISTRATION.classifications) {
    if (!validClassifications.includes(c)) {
      errors.push(`Invalid classification: ${c}`);
    }
  }

  // Verify endpoints
  if (!AGENT_REGISTRATION.endpoints.edge_function) {
    errors.push('Edge function endpoint is required');
  }
  if (!AGENT_REGISTRATION.endpoints.cli) {
    errors.push('CLI endpoint is required');
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Get deployment configuration for Google Cloud Functions
 */
export function getDeploymentConfig(): Record<string, string> {
  return {
    'gcloud-functions-name': AGENT_REGISTRATION.id,
    'gcloud-functions-runtime': AGENT_REGISTRATION.deployment.runtime,
    'gcloud-functions-memory': AGENT_REGISTRATION.deployment.memory,
    'gcloud-functions-timeout': AGENT_REGISTRATION.deployment.timeout,
    'gcloud-functions-max-instances': String(AGENT_REGISTRATION.deployment.max_instances),
    'gcloud-functions-min-instances': String(AGENT_REGISTRATION.deployment.min_instances)
  };
}
