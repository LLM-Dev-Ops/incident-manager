/**
 * Phase 3 — Startup Hardening
 *
 * This module provides startup validation that MUST pass before
 * any Phase 3 Layer 1 agent can operate. Failure results in
 * crashloop behavior (process exit with non-zero code).
 *
 * CRITICAL: Misconfiguration causes CRASHLOOP
 */

import {
  PHASE3_LAYER1_CONFIG,
  PERFORMANCE_BUDGETS,
  Phase3HardFailError,
  type HardFailCondition,
} from './phase3-config.js';

// ============================================================================
// ENVIRONMENT REQUIREMENTS
// ============================================================================

/**
 * Required environment variables for Phase 3 Layer 1
 */
export const REQUIRED_ENV_VARS = [
  'AGENT_PHASE',
  'AGENT_LAYER',
  'RUVECTOR_API_KEY',
  'RUVECTOR_SERVICE_URL',
] as const;

/**
 * Optional environment variables with defaults
 */
export const OPTIONAL_ENV_VARS = {
  RUVECTOR_TIMEOUT_MS: '30000',
  RUVECTOR_RETRIES: '3',
  MAX_TOKENS: String(PERFORMANCE_BUDGETS.MAX_TOKENS),
  MAX_LATENCY_MS: String(PERFORMANCE_BUDGETS.MAX_LATENCY_MS),
  MAX_CALLS_PER_RUN: String(PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN),
  LOG_LEVEL: 'info',
  LOG_FORMAT: 'json',
} as const;

// ============================================================================
// RUVECTOR CLIENT INTERFACE
// ============================================================================

/**
 * Minimal RuVector client interface for startup validation
 * This avoids circular dependency with @agentics/ruvector-client
 */
export interface RuvectorHealthClient {
  health(): Promise<RuvectorHealthResult>;
}

export interface RuvectorHealthResult {
  success: boolean;
  data?: {
    status: 'healthy' | 'degraded' | 'unhealthy';
    version: string;
    timestamp: string;
    checks: Array<{
      name: string;
      status: 'pass' | 'fail' | 'warn';
      message?: string;
    }>;
  };
  error?: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}

/**
 * Configuration for RuVector client
 */
export interface RuvectorConfig {
  baseUrl: string;
  apiKey: string;
  timeoutMs?: number;
  retries?: number;
  debug?: boolean;
}

// ============================================================================
// STARTUP VALIDATION RESULT
// ============================================================================

export interface StartupValidationResult {
  valid: boolean;
  phase: string;
  layer: string;
  ruvector_healthy: boolean;
  ruvector_config: RuvectorConfig;
  performance_budgets: typeof PERFORMANCE_BUDGETS;
  errors: StartupError[];
  warnings: string[];
  timestamp: string;
}

export interface StartupError {
  condition: HardFailCondition;
  message: string;
  details: Record<string, unknown>;
  fatal: boolean;
}

// ============================================================================
// MINIMAL RUVECTOR HEALTH CHECK CLIENT
// ============================================================================

/**
 * Minimal client for RuVector health checks during startup
 * This is used only for validation - the full client is in @agentics/ruvector-client
 */
class MinimalRuvectorHealthClient implements RuvectorHealthClient {
  private readonly config: Required<RuvectorConfig>;

  constructor(config: RuvectorConfig) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/$/, ''),
      apiKey: config.apiKey,
      timeoutMs: config.timeoutMs ?? 30000,
      retries: config.retries ?? 3,
      debug: config.debug ?? false,
    };
  }

  async health(): Promise<RuvectorHealthResult> {
    // RuVector uses /health endpoint (not /api/v1/health)
    const url = `${this.config.baseUrl}/health`;

    if (this.config.debug) {
      console.log(`[RuvectorHealth] GET ${url}`);
    }

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.config.timeoutMs);

      const response = await fetch(url, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.config.apiKey}`,
          'X-Client': '@agentics/contracts/phase3-startup',
        },
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorBody = await response.text().catch(() => 'Unknown error');
        return {
          success: false,
          error: {
            code: 'REQUEST_FAILED',
            message: `Health check failed: ${response.status} ${response.statusText} - ${errorBody}`,
            details: { statusCode: response.status },
          },
        };
      }

      const data = await response.json() as RuvectorHealthResult['data'];
      return { success: true, data };

    } catch (err) {
      const error = err as Error;

      if (error.name === 'AbortError') {
        return {
          success: false,
          error: {
            code: 'TIMEOUT',
            message: 'Health check timed out',
          },
        };
      }

      return {
        success: false,
        error: {
          code: 'NETWORK_ERROR',
          message: error.message,
        },
      };
    }
  }
}

// ============================================================================
// STARTUP HARDENING
// ============================================================================

/**
 * Phase 3 startup hardening validator
 *
 * Validates all requirements before allowing agent execution.
 * On failure, throws Phase3HardFailError which should cause crashloop.
 */
export class Phase3StartupValidator {
  private readonly errors: StartupError[] = [];
  private readonly warnings: string[] = [];
  private ruvectorConfig: RuvectorConfig | null = null;

  constructor(private readonly env: Record<string, string | undefined> = process.env) {}

  /**
   * Run all startup validations
   *
   * @throws Phase3HardFailError on fatal configuration errors
   * @returns StartupValidationResult on success
   */
  async validate(): Promise<StartupValidationResult> {
    console.log('[PHASE3_STARTUP] Beginning startup validation...');

    // Step 1: Validate required environment variables
    this.validateRequiredEnvVars();

    // Step 2: Validate phase and layer
    this.validatePhaseAndLayer();

    // Step 3: Validate RuVector configuration
    const ruvectorHealthy = await this.validateRuvector();

    // Step 4: Validate performance budgets
    this.validatePerformanceBudgets();

    // Check for fatal errors
    const fatalErrors = this.errors.filter(e => e.fatal);
    if (fatalErrors.length > 0) {
      const firstFatal = fatalErrors[0];
      console.error('[PHASE3_STARTUP] FATAL: Startup validation failed');
      console.error('[PHASE3_STARTUP] Errors:', JSON.stringify(fatalErrors, null, 2));

      throw new Phase3HardFailError(
        firstFatal.condition,
        firstFatal.details,
        firstFatal.message
      );
    }

    // Log warnings
    if (this.warnings.length > 0) {
      console.warn('[PHASE3_STARTUP] Warnings:', this.warnings);
    }

    const result: StartupValidationResult = {
      valid: true,
      phase: this.env.AGENT_PHASE || PHASE3_LAYER1_CONFIG.AGENT_PHASE,
      layer: this.env.AGENT_LAYER || PHASE3_LAYER1_CONFIG.AGENT_LAYER,
      ruvector_healthy: ruvectorHealthy,
      ruvector_config: this.ruvectorConfig!,
      performance_budgets: PERFORMANCE_BUDGETS,
      errors: this.errors,
      warnings: this.warnings,
      timestamp: new Date().toISOString(),
    };

    console.log('[PHASE3_STARTUP] Validation successful:', JSON.stringify(result, null, 2));
    return result;
  }

  /**
   * Validate required environment variables are present
   */
  private validateRequiredEnvVars(): void {
    console.log('[PHASE3_STARTUP] Checking required environment variables...');

    for (const envVar of REQUIRED_ENV_VARS) {
      const value = this.env[envVar];

      if (!value || value.trim() === '') {
        this.errors.push({
          condition: 'missing_required_config',
          message: `Required environment variable ${envVar} is not set`,
          details: { variable: envVar },
          fatal: true,
        });
      }
    }

    // Apply defaults for optional vars
    for (const [key, defaultValue] of Object.entries(OPTIONAL_ENV_VARS)) {
      if (!this.env[key]) {
        this.warnings.push(`Optional env ${key} not set, using default: ${defaultValue}`);
      }
    }
  }

  /**
   * Validate AGENT_PHASE and AGENT_LAYER match Phase 3 Layer 1
   */
  private validatePhaseAndLayer(): void {
    console.log('[PHASE3_STARTUP] Validating phase and layer...');

    const phase = this.env.AGENT_PHASE;
    const layer = this.env.AGENT_LAYER;

    if (phase !== PHASE3_LAYER1_CONFIG.AGENT_PHASE) {
      this.errors.push({
        condition: 'missing_required_config',
        message: `AGENT_PHASE must be '${PHASE3_LAYER1_CONFIG.AGENT_PHASE}', got '${phase}'`,
        details: { expected: PHASE3_LAYER1_CONFIG.AGENT_PHASE, actual: phase },
        fatal: true,
      });
    }

    if (layer !== PHASE3_LAYER1_CONFIG.AGENT_LAYER) {
      this.errors.push({
        condition: 'missing_required_config',
        message: `AGENT_LAYER must be '${PHASE3_LAYER1_CONFIG.AGENT_LAYER}', got '${layer}'`,
        details: { expected: PHASE3_LAYER1_CONFIG.AGENT_LAYER, actual: layer },
        fatal: true,
      });
    }
  }

  /**
   * Validate RuVector is available and healthy
   *
   * CRITICAL: RuVector unavailable = HARD FAIL
   */
  private async validateRuvector(): Promise<boolean> {
    console.log('[PHASE3_STARTUP] Validating RuVector connectivity...');

    const apiKey = this.env.RUVECTOR_API_KEY;
    const serviceUrl = this.env.RUVECTOR_SERVICE_URL;

    if (!apiKey || !serviceUrl) {
      // Already caught in validateRequiredEnvVars
      return false;
    }

    this.ruvectorConfig = {
      baseUrl: serviceUrl,
      apiKey: apiKey,
      timeoutMs: parseInt(this.env.RUVECTOR_TIMEOUT_MS || OPTIONAL_ENV_VARS.RUVECTOR_TIMEOUT_MS, 10),
      retries: parseInt(this.env.RUVECTOR_RETRIES || OPTIONAL_ENV_VARS.RUVECTOR_RETRIES, 10),
      debug: this.env.LOG_LEVEL === 'debug',
    };

    const healthClient = new MinimalRuvectorHealthClient(this.ruvectorConfig);

    try {
      const healthResult = await healthClient.health();

      if (!healthResult.success) {
        this.errors.push({
          condition: 'ruvector_unavailable',
          message: `RuVector health check failed: ${healthResult.error?.message}`,
          details: { error: healthResult.error },
          fatal: true, // HARD FAIL
        });
        return false;
      }

      const health = healthResult.data!;

      if (health.status === 'unhealthy') {
        this.errors.push({
          condition: 'ruvector_unavailable',
          message: 'RuVector service is unhealthy',
          details: { health },
          fatal: true, // HARD FAIL
        });
        return false;
      }

      if (health.status === 'degraded') {
        this.warnings.push(`RuVector is degraded: ${JSON.stringify(health.checks)}`);
      }

      console.log('[PHASE3_STARTUP] RuVector healthy:', health.status);
      return true;

    } catch (err) {
      const error = err as Error;
      this.errors.push({
        condition: 'ruvector_unavailable',
        message: `RuVector connection failed: ${error.message}`,
        details: {
          error: error.message,
          serviceUrl,
        },
        fatal: true, // HARD FAIL
      });
      return false;
    }
  }

  /**
   * Validate performance budget configuration
   */
  private validatePerformanceBudgets(): void {
    console.log('[PHASE3_STARTUP] Validating performance budgets...');

    const maxTokens = parseInt(this.env.MAX_TOKENS || OPTIONAL_ENV_VARS.MAX_TOKENS, 10);
    const maxLatency = parseInt(this.env.MAX_LATENCY_MS || OPTIONAL_ENV_VARS.MAX_LATENCY_MS, 10);
    const maxCalls = parseInt(this.env.MAX_CALLS_PER_RUN || OPTIONAL_ENV_VARS.MAX_CALLS_PER_RUN, 10);

    // Warn if custom budgets exceed defaults (stricter is OK)
    if (maxTokens > PERFORMANCE_BUDGETS.MAX_TOKENS) {
      this.warnings.push(
        `Custom MAX_TOKENS (${maxTokens}) exceeds default (${PERFORMANCE_BUDGETS.MAX_TOKENS})`
      );
    }

    if (maxLatency > PERFORMANCE_BUDGETS.MAX_LATENCY_MS) {
      this.warnings.push(
        `Custom MAX_LATENCY_MS (${maxLatency}) exceeds default (${PERFORMANCE_BUDGETS.MAX_LATENCY_MS})`
      );
    }

    if (maxCalls > PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN) {
      this.warnings.push(
        `Custom MAX_CALLS_PER_RUN (${maxCalls}) exceeds default (${PERFORMANCE_BUDGETS.MAX_CALLS_PER_RUN})`
      );
    }

    // Validate reasonable ranges
    if (maxTokens <= 0 || maxLatency <= 0 || maxCalls <= 0) {
      this.errors.push({
        condition: 'missing_required_config',
        message: 'Performance budgets must be positive integers',
        details: { maxTokens, maxLatency, maxCalls },
        fatal: true,
      });
    }
  }

  /**
   * Get the RuVector configuration after successful validation
   */
  getRuvectorConfig(): RuvectorConfig {
    if (!this.ruvectorConfig) {
      throw new Phase3HardFailError(
        'ruvector_unavailable',
        {},
        'RuVector config not initialized - call validate() first'
      );
    }
    return this.ruvectorConfig;
  }
}

// ============================================================================
// CRASHLOOP HANDLER
// ============================================================================

/**
 * Handle startup failure with crashloop behavior
 *
 * This function MUST be called when Phase3HardFailError is caught.
 * It logs the error and exits with non-zero code to trigger restart.
 */
export function handleStartupFailure(error: Phase3HardFailError): never {
  console.error('='.repeat(60));
  console.error('[PHASE3_CRASHLOOP] FATAL STARTUP FAILURE');
  console.error('='.repeat(60));
  console.error(`Condition: ${error.condition}`);
  console.error(`Message: ${error.message}`);
  console.error(`Details: ${JSON.stringify(error.details, null, 2)}`);
  console.error('='.repeat(60));
  console.error('[PHASE3_CRASHLOOP] Exiting with code 1 to trigger restart...');
  console.error('='.repeat(60));

  // Exit with non-zero code to trigger container restart
  process.exit(1);
}

// ============================================================================
// MAIN STARTUP FUNCTION
// ============================================================================

/**
 * Main startup function for Phase 3 Layer 1 agents
 *
 * Call this at the beginning of your agent entry point.
 * Returns the validated config - use with @agentics/ruvector-client.
 *
 * @example
 * ```typescript
 * import { startupPhase3Layer1 } from '@agentics/contracts';
 * import { RuvectorClient } from '@agentics/ruvector-client';
 *
 * async function main() {
 *   const { ruvectorConfig, validationResult } = await startupPhase3Layer1();
 *   const ruvectorClient = new RuvectorClient(ruvectorConfig);
 *   // Agent logic here...
 * }
 *
 * main().catch(console.error);
 * ```
 */
export async function startupPhase3Layer1(
  env: Record<string, string | undefined> = process.env
): Promise<{
  ruvectorConfig: RuvectorConfig;
  validationResult: StartupValidationResult;
}> {
  const validator = new Phase3StartupValidator(env);

  try {
    const validationResult = await validator.validate();
    const ruvectorConfig = validator.getRuvectorConfig();

    console.log('[PHASE3_STARTUP] Agent ready for operation');
    console.log(`[PHASE3_STARTUP] Phase: ${validationResult.phase}, Layer: ${validationResult.layer}`);
    console.log(`[PHASE3_STARTUP] RuVector: ${validationResult.ruvector_healthy ? 'healthy' : 'unhealthy'}`);

    return { ruvectorConfig, validationResult };

  } catch (err) {
    if (err instanceof Phase3HardFailError) {
      handleStartupFailure(err);
    }

    // Unknown error - still crashloop
    console.error('[PHASE3_STARTUP] Unknown startup error:', err);
    process.exit(1);
  }
}

// Re-export is handled by the const declarations above
