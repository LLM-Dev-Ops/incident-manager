/**
 * Health Check Endpoint
 *
 * Provides health status for the Escalation Agent.
 */

import { ESCALATION_AGENT_METADATA } from '@agentics/contracts';
import { RuvectorClient } from '@agentics/ruvector-client';
import { createAgentConfig, validateConfig } from './config.js';

// ============================================================================
// TYPES
// ============================================================================

interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  agent_id: string;
  agent_version: string;
  timestamp: string;
  checks: HealthCheck[];
  uptime_seconds?: number;
}

interface HealthCheck {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message?: string;
  duration_ms?: number;
}

// ============================================================================
// HEALTH CHECK
// ============================================================================

const startTime = Date.now();

/**
 * Perform health check
 */
export async function checkHealth(): Promise<HealthStatus> {
  const checks: HealthCheck[] = [];
  const config = createAgentConfig();

  // Check 1: Configuration validity
  const configStart = Date.now();
  const configValidation = validateConfig(config);
  checks.push({
    name: 'configuration',
    status: configValidation.valid ? 'pass' : 'fail',
    message: configValidation.valid ? 'Configuration valid' : configValidation.errors.join('; '),
    duration_ms: Date.now() - configStart
  });

  // Check 2: ruvector-service connectivity
  const ruvectorStart = Date.now();
  try {
    const ruvectorClient = new RuvectorClient({
      baseUrl: config.ruvector.baseUrl,
      apiKey: config.ruvector.apiKey,
      timeoutMs: 5000 // Shorter timeout for health check
    });

    const healthResult = await ruvectorClient.health();
    checks.push({
      name: 'ruvector_service',
      status: healthResult.success && healthResult.data.status === 'healthy' ? 'pass' :
              healthResult.success ? 'warn' : 'fail',
      message: healthResult.success ? `ruvector-service: ${healthResult.data.status}` :
               healthResult.error.message,
      duration_ms: Date.now() - ruvectorStart
    });
  } catch (err) {
    checks.push({
      name: 'ruvector_service',
      status: 'fail',
      message: `Failed to connect: ${(err as Error).message}`,
      duration_ms: Date.now() - ruvectorStart
    });
  }

  // Check 3: Memory usage
  const memUsage = process.memoryUsage();
  const memoryOk = memUsage.heapUsed < 200 * 1024 * 1024; // 200MB threshold
  checks.push({
    name: 'memory',
    status: memoryOk ? 'pass' : 'warn',
    message: `Heap used: ${Math.round(memUsage.heapUsed / 1024 / 1024)}MB`
  });

  // Determine overall status
  const failCount = checks.filter(c => c.status === 'fail').length;
  const warnCount = checks.filter(c => c.status === 'warn').length;

  let status: 'healthy' | 'degraded' | 'unhealthy';
  if (failCount > 0) {
    status = 'unhealthy';
  } else if (warnCount > 0) {
    status = 'degraded';
  } else {
    status = 'healthy';
  }

  return {
    status,
    agent_id: config.agentId,
    agent_version: ESCALATION_AGENT_METADATA.version,
    timestamp: new Date().toISOString(),
    checks,
    uptime_seconds: Math.floor((Date.now() - startTime) / 1000)
  };
}

/**
 * Health check HTTP handler
 */
export async function healthHandler(): Promise<{
  statusCode: number;
  headers: Record<string, string>;
  body: string;
}> {
  const health = await checkHealth();

  const statusCode = health.status === 'healthy' ? 200 :
                     health.status === 'degraded' ? 200 :
                     503;

  return {
    statusCode,
    headers: {
      'Content-Type': 'application/json',
      'Cache-Control': 'no-cache'
    },
    body: JSON.stringify(health, null, 2)
  };
}
