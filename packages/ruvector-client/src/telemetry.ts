/**
 * TelemetryEmitter - Emit telemetry events to LLM-Observatory
 *
 * This provides LLM-Observatory compatible telemetry emission
 * for Agentics agents.
 */

import type { TelemetryEvent, AgentId, UUID } from '@agentics/contracts';

// ============================================================================
// CONFIGURATION
// ============================================================================

export interface TelemetryConfig {
  /** LLM-Observatory endpoint */
  endpoint: string;

  /** API key for authentication */
  apiKey?: string;

  /** Service name for identification */
  serviceName: string;

  /** Environment */
  environment: 'production' | 'staging' | 'development' | 'qa';

  /** Enable telemetry (default: true) */
  enabled?: boolean;

  /** Batch size before flush (default: 10) */
  batchSize?: number;

  /** Flush interval in ms (default: 5000) */
  flushIntervalMs?: number;

  /** Custom fetch implementation */
  fetch?: typeof fetch;
}

// ============================================================================
// TELEMETRY EMITTER
// ============================================================================

export class TelemetryEmitter {
  private readonly config: Required<Omit<TelemetryConfig, 'fetch'>> & { fetch?: typeof fetch };
  private readonly buffer: TelemetryEvent[] = [];
  private flushTimer: ReturnType<typeof setInterval> | null = null;

  constructor(config: TelemetryConfig) {
    this.config = {
      endpoint: config.endpoint.replace(/\/$/, ''),
      apiKey: config.apiKey ?? '',
      serviceName: config.serviceName,
      environment: config.environment,
      enabled: config.enabled ?? true,
      batchSize: config.batchSize ?? 10,
      flushIntervalMs: config.flushIntervalMs ?? 5000,
      fetch: config.fetch
    };

    if (this.config.enabled) {
      this.startFlushTimer();
    }
  }

  /**
   * Emit an agent invocation event
   */
  emitInvocation(
    agentId: AgentId,
    executionRef: UUID,
    metadata?: Record<string, unknown>
  ): void {
    this.emit({
      event_type: 'agent_invocation',
      agent_id: agentId,
      execution_ref: executionRef,
      timestamp: new Date().toISOString(),
      payload: metadata ?? {},
      tags: {
        service: this.config.serviceName,
        environment: this.config.environment
      }
    });
  }

  /**
   * Emit a decision made event
   */
  emitDecision(
    agentId: AgentId,
    executionRef: UUID,
    decisionType: string,
    confidence: number,
    metadata?: Record<string, unknown>
  ): void {
    this.emit({
      event_type: 'decision_made',
      agent_id: agentId,
      execution_ref: executionRef,
      timestamp: new Date().toISOString(),
      payload: {
        decision_type: decisionType,
        confidence,
        ...metadata
      },
      metrics: {
        confidence
      },
      tags: {
        service: this.config.serviceName,
        environment: this.config.environment,
        decision_type: decisionType
      }
    });
  }

  /**
   * Emit an error event
   */
  emitError(
    agentId: AgentId,
    executionRef: UUID,
    error: Error | string,
    metadata?: Record<string, unknown>
  ): void {
    const errorMessage = error instanceof Error ? error.message : error;
    const errorStack = error instanceof Error ? error.stack : undefined;

    this.emit({
      event_type: 'error',
      agent_id: agentId,
      execution_ref: executionRef,
      timestamp: new Date().toISOString(),
      payload: {
        error: errorMessage,
        stack: errorStack,
        ...metadata
      },
      tags: {
        service: this.config.serviceName,
        environment: this.config.environment,
        error_type: error instanceof Error ? error.name : 'string'
      }
    });
  }

  /**
   * Emit a metric event
   */
  emitMetrics(
    agentId: AgentId,
    executionRef: UUID,
    metrics: Record<string, number>,
    metadata?: Record<string, unknown>
  ): void {
    this.emit({
      event_type: 'metric',
      agent_id: agentId,
      execution_ref: executionRef,
      timestamp: new Date().toISOString(),
      payload: metadata ?? {},
      metrics,
      tags: {
        service: this.config.serviceName,
        environment: this.config.environment
      }
    });
  }

  /**
   * Emit a raw telemetry event
   */
  emit(event: TelemetryEvent): void {
    if (!this.config.enabled) {
      return;
    }

    this.buffer.push(event);

    if (this.buffer.length >= this.config.batchSize) {
      this.flush().catch(console.error);
    }
  }

  /**
   * Flush all buffered events
   */
  async flush(): Promise<void> {
    if (this.buffer.length === 0) {
      return;
    }

    const events = this.buffer.splice(0, this.buffer.length);

    try {
      const fetchFn = this.config.fetch ?? globalThis.fetch;

      await fetchFn(`${this.config.endpoint}/api/v1/telemetry/events`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(this.config.apiKey && {
            'Authorization': `Bearer ${this.config.apiKey}`
          }),
          'X-Service': this.config.serviceName
        },
        body: JSON.stringify({ events })
      });
    } catch (error) {
      // Log but don't throw - telemetry should never break the agent
      console.error('[TelemetryEmitter] Failed to flush events:', error);
      // Re-add events to buffer for retry
      this.buffer.unshift(...events);
    }
  }

  /**
   * Stop the telemetry emitter
   */
  async shutdown(): Promise<void> {
    if (this.flushTimer) {
      clearInterval(this.flushTimer);
      this.flushTimer = null;
    }

    await this.flush();
  }

  // ============================================================================
  // PRIVATE METHODS
  // ============================================================================

  private startFlushTimer(): void {
    this.flushTimer = setInterval(() => {
      this.flush().catch(console.error);
    }, this.config.flushIntervalMs);

    // Don't let this timer prevent Node from exiting
    if (this.flushTimer.unref) {
      this.flushTimer.unref();
    }
  }
}

/**
 * Create a no-op telemetry emitter (for testing or when disabled)
 */
export function createNoOpTelemetryEmitter(): TelemetryEmitter {
  return new TelemetryEmitter({
    endpoint: 'http://localhost',
    serviceName: 'noop',
    environment: 'development',
    enabled: false
  });
}
