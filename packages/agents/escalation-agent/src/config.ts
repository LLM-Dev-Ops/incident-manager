/**
 * Agent Configuration
 *
 * Configuration management for the Escalation Agent.
 */

import { ESCALATION_AGENT_METADATA } from '@agentics/contracts';

// ============================================================================
// CONFIGURATION
// ============================================================================

export interface AgentConfig {
  /** Agent identification */
  agentId: string;
  agentVersion: string;

  /** ruvector-service configuration */
  ruvector: {
    baseUrl: string;
    apiKey: string;
    timeoutMs: number;
  };

  /** Telemetry configuration */
  telemetry: {
    endpoint: string;
    apiKey?: string;
    enabled: boolean;
  };

  /** Environment */
  environment: 'production' | 'staging' | 'development' | 'qa';

  /** Escalation configuration */
  escalation: {
    /** Default policy ID if none specified */
    defaultPolicyId?: string;

    /** Maximum escalation level */
    maxEscalationLevel: number;

    /** Minimum time between escalations (seconds) */
    minEscalationInterval: number;

    /** Confidence threshold for automatic escalation */
    autoEscalationConfidenceThreshold: number;

    /** Whether to require approval for P0 escalations */
    requireApprovalForP0: boolean;
  };

  /** Logging */
  debug: boolean;
}

// ============================================================================
// DEFAULT CONFIGURATION
// ============================================================================

const DEFAULT_CONFIG: Partial<AgentConfig> = {
  agentVersion: ESCALATION_AGENT_METADATA.version,
  environment: 'production',
  escalation: {
    maxEscalationLevel: 5,
    minEscalationInterval: 300, // 5 minutes
    autoEscalationConfidenceThreshold: 0.8,
    requireApprovalForP0: true
  },
  debug: false
};

// ============================================================================
// CONFIGURATION FACTORY
// ============================================================================

/**
 * Create agent configuration from environment variables
 */
export function createAgentConfig(overrides?: Partial<AgentConfig>): AgentConfig {
  const env = process.env;

  const agentId = overrides?.agentId ??
    env.AGENT_ID ??
    `incident-escalation:${ESCALATION_AGENT_METADATA.version}:${generateInstanceId()}`;

  const config: AgentConfig = {
    agentId,
    agentVersion: overrides?.agentVersion ?? ESCALATION_AGENT_METADATA.version,

    ruvector: {
      baseUrl: overrides?.ruvector?.baseUrl ?? env.RUVECTOR_BASE_URL ?? 'http://localhost:8080',
      apiKey: overrides?.ruvector?.apiKey ?? env.RUVECTOR_API_KEY ?? '',
      timeoutMs: overrides?.ruvector?.timeoutMs ?? parseInt(env.RUVECTOR_TIMEOUT_MS ?? '30000', 10)
    },

    telemetry: {
      endpoint: overrides?.telemetry?.endpoint ?? env.TELEMETRY_ENDPOINT ?? 'http://localhost:9090',
      apiKey: overrides?.telemetry?.apiKey ?? env.TELEMETRY_API_KEY,
      enabled: overrides?.telemetry?.enabled ?? (env.TELEMETRY_ENABLED !== 'false')
    },

    environment: (overrides?.environment ?? env.ENVIRONMENT ?? 'production') as AgentConfig['environment'],

    escalation: {
      defaultPolicyId: overrides?.escalation?.defaultPolicyId ?? env.DEFAULT_POLICY_ID,
      maxEscalationLevel: overrides?.escalation?.maxEscalationLevel ??
        parseInt(env.MAX_ESCALATION_LEVEL ?? '5', 10),
      minEscalationInterval: overrides?.escalation?.minEscalationInterval ??
        parseInt(env.MIN_ESCALATION_INTERVAL ?? '300', 10),
      autoEscalationConfidenceThreshold: overrides?.escalation?.autoEscalationConfidenceThreshold ??
        parseFloat(env.AUTO_ESCALATION_THRESHOLD ?? '0.8'),
      requireApprovalForP0: overrides?.escalation?.requireApprovalForP0 ??
        (env.REQUIRE_APPROVAL_P0 !== 'false')
    },

    debug: overrides?.debug ?? (env.DEBUG === 'true')
  };

  return config;
}

/**
 * Generate a unique instance ID
 */
function generateInstanceId(): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 8);
  return `${timestamp}-${random}`;
}

/**
 * Validate configuration
 */
export function validateConfig(config: AgentConfig): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  if (!config.agentId) {
    errors.push('agentId is required');
  }

  if (!config.ruvector.baseUrl) {
    errors.push('ruvector.baseUrl is required');
  }

  if (config.escalation.maxEscalationLevel < 1) {
    errors.push('escalation.maxEscalationLevel must be at least 1');
  }

  if (config.escalation.minEscalationInterval < 0) {
    errors.push('escalation.minEscalationInterval cannot be negative');
  }

  if (config.escalation.autoEscalationConfidenceThreshold < 0 ||
      config.escalation.autoEscalationConfidenceThreshold > 1) {
    errors.push('escalation.autoEscalationConfidenceThreshold must be between 0 and 1');
  }

  return { valid: errors.length === 0, errors };
}
