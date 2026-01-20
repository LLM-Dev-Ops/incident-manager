/**
 * @agentics/ruvector-client
 *
 * Client for ruvector-service - the ONLY permitted persistence layer
 * for Agentics platform agents.
 *
 * ARCHITECTURAL REQUIREMENTS (from PROMPT 0):
 * - LLM-Incident-Manager does NOT own persistence
 * - ALL incident states, transitions, approvals, and artifacts
 *   are persisted via ruvector-service
 * - ruvector-service is backed by Google SQL (Postgres)
 * - LLM-Incident-Manager NEVER connects directly to Google SQL
 * - LLM-Incident-Manager NEVER executes SQL
 * - All persistence occurs via ruvector-service client calls ONLY
 *
 * @packageDocumentation
 */

export { RuvectorClient } from './client.js';
export type { RuvectorConfig, RuvectorClientOptions } from './client.js';
export { DecisionEventStore } from './decision-event-store.js';
export { TelemetryEmitter } from './telemetry.js';
export type { TelemetryConfig } from './telemetry.js';

// Re-export relevant types from contracts
export type {
  DecisionEvent,
  DecisionType,
  TelemetryEvent
} from '@agentics/contracts';
