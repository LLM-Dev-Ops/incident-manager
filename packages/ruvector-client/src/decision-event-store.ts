/**
 * DecisionEventStore - Specialized store for DecisionEvents
 *
 * Provides a higher-level API for working with DecisionEvents,
 * including validation and persistence specification enforcement.
 */

import type {
  DecisionEvent,
  UUID,
  AgentResult,
  PersistenceSpec
} from '@agentics/contracts';
import {
  validateDecisionEvent,
  DECISION_EVENT_PERSISTENCE
} from '@agentics/contracts';
import { RuvectorClient } from './client.js';

// ============================================================================
// DECISION EVENT STORE
// ============================================================================

export class DecisionEventStore {
  constructor(private readonly client: RuvectorClient) {}

  /**
   * Store a DecisionEvent with validation and filtering
   *
   * This method:
   * 1. Validates the DecisionEvent
   * 2. Filters out excluded fields per persistence spec
   * 3. Stores via ruvector-service
   */
  async store<T>(
    event: DecisionEvent<T>,
    persistenceSpec?: PersistenceSpec
  ): Promise<AgentResult<{ id: UUID }>> {
    // Validate
    const validation = validateDecisionEvent(event);
    if (!validation.valid) {
      return {
        success: false,
        error: {
          code: 'VALIDATION_ERROR',
          message: `DecisionEvent validation failed: ${validation.errors.join(', ')}`,
          details: { errors: validation.errors },
          retryable: false
        }
      };
    }

    // Filter out excluded fields
    const spec = persistenceSpec ?? DECISION_EVENT_PERSISTENCE;
    const filteredEvent = this.filterEvent(event, spec);

    // Store
    return this.client.storeDecisionEvent(filteredEvent);
  }

  /**
   * Retrieve a DecisionEvent by ID
   */
  async get<T>(id: UUID): Promise<AgentResult<DecisionEvent<T>>> {
    return this.client.getDecisionEvent<T>(id);
  }

  /**
   * Query DecisionEvents by agent
   */
  async queryByAgent<T>(
    agentId: string,
    options?: { limit?: number; offset?: number }
  ): Promise<AgentResult<DecisionEvent<T>[]>> {
    const result = await this.client.queryDecisionEvents<T>({
      agent_id: agentId,
      limit: options?.limit ?? 100,
      offset: options?.offset ?? 0
    });

    if (!result.success) {
      return result;
    }

    return { success: true, data: result.data.events };
  }

  /**
   * Query DecisionEvents by incident
   */
  async queryByIncident<T>(
    incidentId: UUID,
    options?: { limit?: number; offset?: number }
  ): Promise<AgentResult<DecisionEvent<T>[]>> {
    const result = await this.client.queryDecisionEvents<T>({
      incident_id: incidentId,
      limit: options?.limit ?? 100,
      offset: options?.offset ?? 0
    });

    if (!result.success) {
      return result;
    }

    return { success: true, data: result.data.events };
  }

  /**
   * Get the latest DecisionEvent for an incident
   */
  async getLatestForIncident<T>(incidentId: UUID): Promise<AgentResult<DecisionEvent<T> | null>> {
    const result = await this.client.queryDecisionEvents<T>({
      incident_id: incidentId,
      limit: 1,
      offset: 0
    });

    if (!result.success) {
      return result;
    }

    if (result.data.events.length === 0) {
      return { success: true, data: null };
    }

    return { success: true, data: result.data.events[0] };
  }

  // ============================================================================
  // PRIVATE METHODS
  // ============================================================================

  /**
   * Filter event based on persistence spec
   */
  private filterEvent<T>(
    event: DecisionEvent<T>,
    spec: PersistenceSpec
  ): DecisionEvent<T> {
    const filtered = { ...event };

    // Remove excluded fields
    for (const field of spec.exclude) {
      if (field.includes('.')) {
        // Handle nested fields like 'signal_payload.raw_data'
        const [parent, child] = field.split('.');
        if (filtered[parent as keyof DecisionEvent<T>]) {
          const parentObj = filtered[parent as keyof DecisionEvent<T>] as Record<string, unknown>;
          if (typeof parentObj === 'object' && parentObj !== null) {
            delete parentObj[child];
          }
        }
      } else {
        delete filtered[field as keyof DecisionEvent<T>];
      }
    }

    return filtered;
  }
}
