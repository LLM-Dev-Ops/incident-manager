/**
 * @agentics/contracts - Common Types
 *
 * Base types and utilities used across all Agentics platform contracts.
 * These types are fundamental to the agent infrastructure.
 */

// ============================================================================
// PRIMITIVE TYPES
// ============================================================================

/** ISO 8601 timestamp string (e.g., "2024-01-15T10:30:00.000Z") */
export type ISO8601Timestamp = string;

/** UUID v4 string */
export type UUID = string;

/** Semantic version string (e.g., "1.0.0") */
export type SemVer = string;

/** SHA-256 hash string (hex encoded) */
export type SHA256Hash = string;

// ============================================================================
// AGENT IDENTIFICATION
// ============================================================================

/**
 * Unique identifier for an agent instance
 * Format: {agent_type}:{version}:{instance_id}
 * Example: "incident-escalation:1.0.0:abc123"
 */
export type AgentId = string;

/**
 * Agent type classification
 */
export type AgentType =
  | 'incident-escalation'
  | 'incident-resolver'
  | 'incident-approver'      // Human-in-the-Loop Agent
  | 'incident-inspector'
  | 'incident-coordinator'
  | 'sentinel-consumer'
  | 'shield-consumer'
  | 'edge-agent-consumer'
  | 'governance-consumer'
  | 'hitl-agent';            // Human-in-the-Loop Agent (alias)

/**
 * Agent classification for LLM-Incident-Manager
 */
export type AgentClassification =
  | 'INCIDENT_ORCHESTRATION'
  | 'ESCALATION'
  | 'APPROVAL_GATING'
  | 'INSPECTION'
  | 'COORDINATION';

// ============================================================================
// SEVERITY & PRIORITY
// ============================================================================

/**
 * Incident severity levels (P0 = critical, P4 = informational)
 */
export type Severity = 'P0' | 'P1' | 'P2' | 'P3' | 'P4';

/**
 * Severity transition direction
 */
export type SeverityDirection = 'escalate' | 'deescalate' | 'maintain';

/**
 * Numeric severity value for calculations (lower = more severe)
 */
export const SEVERITY_NUMERIC: Record<Severity, number> = {
  'P0': 0,
  'P1': 1,
  'P2': 2,
  'P3': 3,
  'P4': 4
};

// ============================================================================
// ENVIRONMENT & CONTEXT
// ============================================================================

/**
 * Deployment environment
 */
export type Environment = 'production' | 'staging' | 'development' | 'qa';

/**
 * Agent execution context
 */
export interface ExecutionContext {
  /** Unique execution ID (correlation ID for this invocation) */
  execution_id: UUID;

  /** Agent identifier */
  agent_id: AgentId;

  /** Agent version */
  agent_version: SemVer;

  /** Deployment environment */
  environment: Environment;

  /** Region/zone where agent is executing */
  region?: string;

  /** Timestamp when execution started */
  started_at: ISO8601Timestamp;

  /** Request trace ID for distributed tracing */
  trace_id?: string;

  /** Parent span ID if part of a trace */
  parent_span_id?: string;
}

// ============================================================================
// VALIDATION UTILITIES
// ============================================================================

/**
 * Validation result for contract inputs/outputs
 */
export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

export interface ValidationError {
  field: string;
  message: string;
  code: string;
  value?: unknown;
}

export interface ValidationWarning {
  field: string;
  message: string;
  code: string;
}

/**
 * Result wrapper for agent operations
 */
export type AgentResult<T> =
  | { success: true; data: T; warnings?: ValidationWarning[] }
  | { success: false; error: AgentError };

export interface AgentError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  retryable: boolean;
}

// ============================================================================
// PERSISTENCE TYPES
// ============================================================================

/**
 * Identifies what should and should NOT be persisted
 */
export interface PersistenceSpec {
  /** Fields that MUST be persisted to ruvector-service */
  persist: string[];

  /** Fields that MUST NOT be persisted (sensitive, transient) */
  exclude: string[];

  /** TTL for persisted data in seconds (0 = no expiry) */
  ttl_seconds: number;
}

// ============================================================================
// TELEMETRY TYPES (LLM-Observatory compatible)
// ============================================================================

/**
 * Telemetry event for LLM-Observatory
 */
export interface TelemetryEvent {
  /** Event type */
  event_type: 'agent_invocation' | 'decision_made' | 'error' | 'metric';

  /** Agent identifier */
  agent_id: AgentId;

  /** Execution context */
  execution_ref: UUID;

  /** Timestamp */
  timestamp: ISO8601Timestamp;

  /** Event payload */
  payload: Record<string, unknown>;

  /** Metrics (for metric events) */
  metrics?: Record<string, number>;

  /** Tags for filtering */
  tags: Record<string, string>;
}
