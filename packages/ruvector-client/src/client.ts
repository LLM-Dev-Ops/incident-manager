/**
 * RuvectorClient - Core client for ruvector-service
 *
 * This is the ONLY permitted way to persist data from Agentics agents.
 */

import type { DecisionEvent, AgentResult, UUID } from '@agentics/contracts';

// ============================================================================
// CONFIGURATION
// ============================================================================

export interface RuvectorConfig {
  /** Base URL of ruvector-service */
  baseUrl: string;

  /** API key for authentication */
  apiKey: string;

  /** Request timeout in milliseconds */
  timeoutMs?: number;

  /** Number of retries on failure */
  retries?: number;

  /** Enable debug logging */
  debug?: boolean;
}

export interface RuvectorClientOptions {
  /** Custom fetch implementation (for testing) */
  fetch?: typeof fetch;

  /** Custom headers to include in all requests */
  headers?: Record<string, string>;
}

// ============================================================================
// ERROR TYPES
// ============================================================================

export class RuvectorError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly statusCode?: number,
    public readonly retryable: boolean = false
  ) {
    super(message);
    this.name = 'RuvectorError';
  }
}

// ============================================================================
// CLIENT
// ============================================================================

export class RuvectorClient {
  private readonly config: Required<RuvectorConfig>;
  private readonly fetchFn: typeof fetch;
  private readonly headers: Record<string, string>;

  constructor(config: RuvectorConfig, options?: RuvectorClientOptions) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/$/, ''), // Remove trailing slash
      apiKey: config.apiKey,
      timeoutMs: config.timeoutMs ?? 30000,
      retries: config.retries ?? 3,
      debug: config.debug ?? false
    };

    this.fetchFn = options?.fetch ?? globalThis.fetch;
    this.headers = {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${this.config.apiKey}`,
      'X-Client': '@agentics/ruvector-client',
      'X-Client-Version': '1.0.0',
      ...options?.headers
    };
  }

  // ============================================================================
  // DECISION EVENTS
  // ============================================================================

  /**
   * Store a DecisionEvent in ruvector-service
   * This is called exactly ONCE per agent invocation.
   */
  async storeDecisionEvent<T>(event: DecisionEvent<T>): Promise<AgentResult<{ id: UUID }>> {
    return this.post<{ id: UUID }>('/api/v1/decision-events', event);
  }

  /**
   * Retrieve a DecisionEvent by ID
   */
  async getDecisionEvent<T>(id: UUID): Promise<AgentResult<DecisionEvent<T>>> {
    return this.get<DecisionEvent<T>>(`/api/v1/decision-events/${id}`);
  }

  /**
   * Query DecisionEvents
   */
  async queryDecisionEvents<T>(query: DecisionEventQuery): Promise<AgentResult<DecisionEventQueryResult<T>>> {
    return this.post<DecisionEventQueryResult<T>>('/api/v1/decision-events/query', query);
  }

  // ============================================================================
  // INCIDENT STATE
  // ============================================================================

  /**
   * Get incident state from ruvector-service
   */
  async getIncidentState(incidentId: UUID): Promise<AgentResult<IncidentState>> {
    return this.get<IncidentState>(`/api/v1/incidents/${incidentId}/state`);
  }

  /**
   * Update incident state via ruvector-service
   */
  async updateIncidentState(incidentId: UUID, update: IncidentStateUpdate): Promise<AgentResult<IncidentState>> {
    return this.patch<IncidentState>(`/api/v1/incidents/${incidentId}/state`, update);
  }

  // ============================================================================
  // ESCALATION STATE
  // ============================================================================

  /**
   * Get escalation state for an incident
   */
  async getEscalationState(incidentId: UUID): Promise<AgentResult<EscalationStateRecord | null>> {
    return this.get<EscalationStateRecord | null>(`/api/v1/incidents/${incidentId}/escalation`);
  }

  /**
   * Store escalation state
   */
  async storeEscalationState(state: EscalationStateRecord): Promise<AgentResult<{ id: UUID }>> {
    return this.post<{ id: UUID }>('/api/v1/escalation-states', state);
  }

  /**
   * Update escalation state
   */
  async updateEscalationState(
    incidentId: UUID,
    update: Partial<EscalationStateRecord>
  ): Promise<AgentResult<EscalationStateRecord>> {
    return this.patch<EscalationStateRecord>(`/api/v1/incidents/${incidentId}/escalation`, update);
  }

  // ============================================================================
  // HEALTH
  // ============================================================================

  /**
   * Check ruvector-service health
   */
  async health(): Promise<AgentResult<HealthStatus>> {
    return this.get<HealthStatus>('/api/v1/health');
  }

  // ============================================================================
  // PRIVATE METHODS
  // ============================================================================

  private async get<T>(path: string): Promise<AgentResult<T>> {
    return this.request<T>('GET', path);
  }

  private async post<T>(path: string, body: unknown): Promise<AgentResult<T>> {
    return this.request<T>('POST', path, body);
  }

  private async patch<T>(path: string, body: unknown): Promise<AgentResult<T>> {
    return this.request<T>('PATCH', path, body);
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    attempt = 1
  ): Promise<AgentResult<T>> {
    const url = `${this.config.baseUrl}${path}`;

    if (this.config.debug) {
      console.log(`[RuvectorClient] ${method} ${url}`);
    }

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.config.timeoutMs);

      const response = await this.fetchFn(url, {
        method,
        headers: this.headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorBody = await response.text().catch(() => 'Unknown error');
        const error = new RuvectorError(
          `Request failed: ${response.status} ${response.statusText} - ${errorBody}`,
          'REQUEST_FAILED',
          response.status,
          response.status >= 500 || response.status === 429
        );

        // Retry on retryable errors
        if (error.retryable && attempt < this.config.retries) {
          const delay = Math.min(1000 * Math.pow(2, attempt - 1), 10000);
          await this.sleep(delay);
          return this.request<T>(method, path, body, attempt + 1);
        }

        return {
          success: false,
          error: {
            code: error.code,
            message: error.message,
            details: { statusCode: error.statusCode },
            retryable: error.retryable
          }
        };
      }

      const data = await response.json() as T;
      return { success: true, data };

    } catch (err) {
      const error = err as Error;

      if (error.name === 'AbortError') {
        const ruvectorError = new RuvectorError(
          'Request timed out',
          'TIMEOUT',
          undefined,
          true
        );

        if (attempt < this.config.retries) {
          const delay = Math.min(1000 * Math.pow(2, attempt - 1), 10000);
          await this.sleep(delay);
          return this.request<T>(method, path, body, attempt + 1);
        }

        return {
          success: false,
          error: {
            code: ruvectorError.code,
            message: ruvectorError.message,
            details: {},
            retryable: ruvectorError.retryable
          }
        };
      }

      return {
        success: false,
        error: {
          code: 'NETWORK_ERROR',
          message: error.message,
          details: {},
          retryable: true
        }
      };
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// ============================================================================
// TYPES
// ============================================================================

export interface DecisionEventQuery {
  agent_id?: string;
  decision_type?: string;
  incident_id?: UUID;
  from_timestamp?: string;
  to_timestamp?: string;
  limit?: number;
  offset?: number;
}

export interface DecisionEventQueryResult<T> {
  events: DecisionEvent<T>[];
  total: number;
  limit: number;
  offset: number;
}

export interface IncidentState {
  incident_id: UUID;
  status: string;
  severity: string;
  escalation_level: number;
  assigned_to?: string;
  assigned_team?: string;
  acknowledged_at?: string;
  resolved_at?: string;
  updated_at: string;
  metadata: Record<string, unknown>;
}

export interface IncidentStateUpdate {
  status?: string;
  severity?: string;
  escalation_level?: number;
  assigned_to?: string;
  assigned_team?: string;
  acknowledged_at?: string;
  resolved_at?: string;
  metadata?: Record<string, unknown>;
}

export interface EscalationStateRecord {
  id: UUID;
  incident_id: UUID;
  policy_id: UUID;
  current_level: number;
  status: 'active' | 'acknowledged' | 'completed' | 'resolved' | 'cancelled';
  started_at: string;
  level_reached_at: string;
  next_escalation_at?: string;
  acknowledged: boolean;
  acknowledged_at?: string;
  acknowledged_by?: string;
  repeat_count: number;
  notification_history: NotificationRecord[];
  created_at: string;
  updated_at: string;
}

export interface NotificationRecord {
  sent_at: string;
  level: number;
  target: string;
  channel: string;
  success: boolean;
  error?: string;
}

export interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version: string;
  timestamp: string;
  checks: HealthCheck[];
}

export interface HealthCheck {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message?: string;
  duration_ms?: number;
}
