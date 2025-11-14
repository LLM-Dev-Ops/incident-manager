/**
 * LLM Client Architecture - TypeScript Type Definitions
 *
 * This file contains all type definitions for the enterprise LLM client architecture.
 * Implementation teams should use these types as the canonical reference.
 *
 * @version 1.0.0
 * @date 2025-11-12
 */

// ============================================================================
// CORE TYPES
// ============================================================================

export type ISO8601Timestamp = string;
export type UUID = string;

// ============================================================================
// LLM REQUEST/RESPONSE TYPES
// ============================================================================

/**
 * Base LLM request structure
 */
export interface LLMRequest {
  id: string;
  timestamp: ISO8601Timestamp;
  model: string;
  messages: Message[];
  temperature?: number;
  maxTokens?: number;
  topP?: number;
  frequencyPenalty?: number;
  presencePenalty?: number;
  stopSequences?: string[];
  functions?: FunctionDefinition[];
  metadata?: Record<string, any>;
}

/**
 * Message structure for chat-based interactions
 */
export interface Message {
  role: 'system' | 'user' | 'assistant' | 'function';
  content: string | ContentBlock[];
  name?: string;
  functionCall?: FunctionCall;
}

/**
 * Content block for multimodal inputs
 */
export type ContentBlock = TextBlock | ImageBlock | DocumentBlock;

export interface TextBlock {
  type: 'text';
  text: string;
}

export interface ImageBlock {
  type: 'image';
  source: {
    type: 'url' | 'base64';
    data: string;
  };
}

export interface DocumentBlock {
  type: 'document';
  source: {
    type: 'url' | 'base64';
    data: string;
  };
  mimeType: string;
}

/**
 * Function calling support
 */
export interface FunctionCall {
  name: string;
  arguments: Record<string, any>;
}

export interface FunctionDefinition {
  name: string;
  description: string;
  parameters: JSONSchema;
}

export interface JSONSchema {
  type: string;
  properties?: Record<string, any>;
  required?: string[];
  [key: string]: any;
}

/**
 * LLM response structure
 */
export interface LLMResponse {
  id: string;
  requestId: string;
  timestamp: ISO8601Timestamp;
  model: string;
  content: string;
  role: 'assistant';
  functionCall?: FunctionCall;
  finishReason: 'stop' | 'length' | 'function_call' | 'content_filter';
  usage: TokenUsage;
  metadata?: Record<string, any>;
}

/**
 * Token usage tracking
 */
export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  estimatedCost?: number;
}

/**
 * Streaming response chunk
 */
export interface CompletionChunk {
  id: string;
  delta: {
    role?: 'assistant';
    content?: string;
    functionCall?: Partial<FunctionCall>;
  };
  finishReason?: string;
}

/**
 * Generic completion request
 */
export interface CompletionRequest extends LLMRequest {
  stream?: boolean;
}

/**
 * Generic completion response
 */
export interface CompletionResponse extends LLMResponse {}

/**
 * Embedding request
 */
export interface EmbeddingRequest {
  id: string;
  model: string;
  input: string | string[];
  metadata?: Record<string, any>;
}

/**
 * Embedding response
 */
export interface EmbeddingResponse {
  embeddings: number[][];
  usage: {
    promptTokens: number;
    totalTokens: number;
  };
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/**
 * Error categorization
 */
export enum LLMErrorType {
  // Retryable errors
  RATE_LIMIT = 'RATE_LIMIT',
  TIMEOUT = 'TIMEOUT',
  SERVICE_UNAVAILABLE = 'SERVICE_UNAVAILABLE',
  NETWORK_ERROR = 'NETWORK_ERROR',

  // Non-retryable errors
  AUTHENTICATION_ERROR = 'AUTHENTICATION_ERROR',
  INVALID_REQUEST = 'INVALID_REQUEST',
  CONTENT_FILTER = 'CONTENT_FILTER',
  MODEL_NOT_FOUND = 'MODEL_NOT_FOUND',
  QUOTA_EXCEEDED = 'QUOTA_EXCEEDED',

  // Circuit breaker
  CIRCUIT_OPEN = 'CIRCUIT_OPEN',

  // Unknown
  UNKNOWN_ERROR = 'UNKNOWN_ERROR',
}

/**
 * LLM error with metadata
 */
export class LLMError extends Error {
  constructor(
    public type: LLMErrorType,
    public message: string,
    public statusCode?: number,
    public retryable: boolean = false,
    public metadata?: Record<string, any>
  ) {
    super(message);
    this.name = 'LLMError';
    Object.setPrototypeOf(this, LLMError.prototype);
  }

  static fromProviderError(_error: any, _provider: string): LLMError {
    // Provider-specific error mapping
    throw new Error('Not implemented');
  }
}

// ============================================================================
// CONFIGURATION TYPES
// ============================================================================

/**
 * Base LLM client configuration
 */
export interface LLMClientConfig {
  // Client identification
  clientId: string;
  clientName: string;
  version: string;

  // Provider configuration
  provider: ProviderConfig;

  // Resilience settings
  resilience: ResilienceConfig;

  // Observability settings
  observability: ObservabilityConfig;

  // Feature flags
  features: FeatureConfig;

  // Environment
  environment: 'production' | 'staging' | 'development' | 'test';
}

/**
 * Provider configuration
 */
export interface ProviderConfig {
  provider: 'openai' | 'anthropic' | 'azure' | 'vertex';
  apiKey: string;
  baseUrl?: string;
  organization?: string;
  apiVersion?: string;

  // Model configuration
  defaultModel: string;
  availableModels: string[];

  // Request defaults
  defaultTemperature: number;
  defaultMaxTokens: number;
  defaultTopP: number;
}

/**
 * OpenAI-specific configuration
 */
export interface OpenAIConfig extends ProviderConfig {
  provider: 'openai';
  organization?: string;
  azureDeployment?: string;
}

/**
 * Anthropic-specific configuration
 */
export interface AnthropicConfig extends ProviderConfig {
  provider: 'anthropic';
  anthropicVersion: string;
}

/**
 * Azure OpenAI configuration
 */
export interface AzureConfig extends ProviderConfig {
  provider: 'azure';
  azureDeployment: string;
  azureEndpoint: string;
  apiVersion: string;
}

/**
 * Vertex AI configuration
 */
export interface VertexConfig extends ProviderConfig {
  provider: 'vertex';
  projectId: string;
  location: string;
  credentials?: any;
}

/**
 * Resilience configuration
 */
export interface ResilienceConfig {
  // Retry configuration
  retry: RetryConfig;

  // Circuit breaker configuration
  circuitBreaker: CircuitBreakerConfig;

  // Rate limiting
  rateLimit: RateLimitConfig;

  // Timeout configuration
  timeout: TimeoutConfig;
}

/**
 * Retry configuration with exponential backoff
 */
export interface RetryConfig {
  enabled: boolean;
  maxAttempts: number;
  baseDelayMs: number;
  maxDelayMs: number;
  backoffMultiplier: number;
  jitterMs: number;
  retryableErrors: LLMErrorType[];
}

/**
 * Circuit breaker configuration
 */
export interface CircuitBreakerConfig {
  enabled: boolean;
  failureThreshold: number;
  successThreshold: number;
  timeout: number;
  halfOpenMaxAttempts: number;
  volumeThreshold: number;
}

/**
 * Rate limiting configuration
 */
export interface RateLimitConfig {
  enabled: boolean;
  requestsPerMinute: number;
  requestsPerHour: number;
  tokensPerMinute: number;
  tokensPerDay: number;
  burstAllowance: number;
}

/**
 * Timeout configuration
 */
export interface TimeoutConfig {
  requestTimeoutMs: number;
  connectionTimeoutMs: number;
  streamTimeoutMs: number;
}

/**
 * Observability configuration
 */
export interface ObservabilityConfig {
  // Logging
  logging: LoggingConfig;

  // Metrics
  metrics: MetricsConfig;

  // Tracing
  tracing: TracingConfig;

  // Auditing
  auditing: AuditConfig;
}

/**
 * Logging configuration
 */
export interface LoggingConfig {
  enabled: boolean;
  level: 'debug' | 'info' | 'warn' | 'error';
  logRequests: boolean;
  logResponses: boolean;
  redactPII: boolean;
  redactApiKeys: boolean;
  maxBodyLength: number;
}

/**
 * Metrics configuration
 */
export interface MetricsConfig {
  enabled: boolean;
  prefix: string;
  labels: Record<string, string>;
  exportInterval: number;
  exportEndpoint?: string;
}

/**
 * Tracing configuration
 */
export interface TracingConfig {
  enabled: boolean;
  serviceName: string;
  samplingRate: number;
  exportEndpoint?: string;
}

/**
 * Audit configuration
 */
export interface AuditConfig {
  enabled: boolean;
  logAllRequests: boolean;
  logFailedRequests: boolean;
  retentionDays: number;
  storageBackend: 'file' | 'database' | 's3';
}

/**
 * Feature flags
 */
export interface FeatureConfig {
  streamingEnabled: boolean;
  functionCallingEnabled: boolean;
  cachingEnabled: boolean;
  embeddingsEnabled: boolean;
  visionEnabled: boolean;
}

// ============================================================================
// SPECIALIZED CLIENT TYPES - SENTINEL
// ============================================================================

/**
 * Sentinel LLM Client Configuration
 */
export interface SentinelLLMConfig extends LLMClientConfig {
  // Sentinel-specific configuration
  anomalyDetection: {
    confidenceThreshold: number;
    severityMapping: Record<string, number>;
    enablePatternLearning: boolean;
  };
}

/**
 * Anomaly event input
 */
export interface AnomalyEvent {
  eventId: string;
  timestamp: ISO8601Timestamp;
  source: string;
  metrics: MetricData;
  context: Record<string, any>;
}

/**
 * Metric data structure
 */
export interface MetricData {
  [key: string]: number | string;
}

/**
 * Anomaly analysis result
 */
export interface AnomalyAnalysis {
  isAnomaly: boolean;
  confidence: number;
  severity: 'P0' | 'P1' | 'P2' | 'P3' | 'P4';
  rootCause?: string;
  recommendations: string[];
  relatedIncidents?: string[];
  impactAssessment?: ImpactAssessment;
}

/**
 * Severity classification
 */
export interface SeverityClassification {
  severity: 'P0' | 'P1' | 'P2' | 'P3' | 'P4';
  confidence: number;
  reasoning: string;
  factors: string[];
}

/**
 * Impact assessment
 */
export interface ImpactAssessment {
  affectedServices: string[];
  estimatedUserImpact: number;
  businessImpact: 'critical' | 'high' | 'medium' | 'low';
  estimatedDowntime?: number;
}

/**
 * Insight generation
 */
export interface Insight {
  type: 'trend' | 'pattern' | 'correlation' | 'prediction';
  description: string;
  confidence: number;
  supportingData: any[];
  actionable: boolean;
  recommendations?: string[];
}

/**
 * Impact prediction
 */
export interface ImpactPrediction {
  likelihood: number;
  severity: string;
  timeToImpact?: number;
  affectedResources: string[];
  mitigationOptions: string[];
}

// ============================================================================
// SPECIALIZED CLIENT TYPES - SHIELD
// ============================================================================

/**
 * Shield LLM Client Configuration
 */
export interface ShieldLLMConfig extends LLMClientConfig {
  // Shield-specific configuration
  securityAnalysis: {
    threatIntelligence: boolean;
    enableMalwareDetection: boolean;
    riskThreshold: number;
  };
}

/**
 * Security event input
 */
export interface SecurityEvent {
  eventId: string;
  timestamp: ISO8601Timestamp;
  eventType: string;
  payload: string;
  context: SecurityContext;
}

/**
 * Security context
 */
export interface SecurityContext {
  userId?: string;
  ipAddress?: string;
  userAgent?: string;
  apiKey?: string;
  requestPath?: string;
  headers?: Record<string, string>;
  geolocation?: string;
}

/**
 * Threat analysis result
 */
export interface ThreatAnalysis {
  isThreat: boolean;
  threatLevel: 'critical' | 'high' | 'medium' | 'low' | 'none';
  confidence: number;
  threatType?: string[];
  indicators: string[];
  mitigationSteps: string[];
  cvssScore?: number;
  attackVector?: string;
}

/**
 * Risk assessment
 */
export interface RiskAssessment {
  riskScore: number;
  riskLevel: 'critical' | 'high' | 'medium' | 'low';
  vulnerabilities: Vulnerability[];
  recommendations: string[];
  compliance: ComplianceStatus[];
}

/**
 * Vulnerability information
 */
export interface Vulnerability {
  id: string;
  name: string;
  description: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  cvssScore?: number;
  cveId?: string;
  affectedComponents: string[];
  patchAvailable: boolean;
  exploitAvailable: boolean;
}

/**
 * Malicious pattern detection
 */
export interface MaliciousPattern {
  patternType: string;
  description: string;
  confidence: number;
  location: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
}

/**
 * Mitigation plan
 */
export interface MitigationPlan {
  threatId: string;
  steps: MitigationStep[];
  estimatedTime: number;
  priority: 'immediate' | 'high' | 'medium' | 'low';
  automatable: boolean;
}

/**
 * Mitigation step
 */
export interface MitigationStep {
  id: string;
  description: string;
  type: 'preventive' | 'detective' | 'corrective';
  automated: boolean;
  command?: string;
  expectedOutcome: string;
}

/**
 * Compliance status
 */
export interface ComplianceStatus {
  framework: string;
  compliant: boolean;
  violations: string[];
  score: number;
}

// ============================================================================
// SPECIALIZED CLIENT TYPES - EDGE-AGENT
// ============================================================================

/**
 * Edge-Agent LLM Client Configuration
 */
export interface EdgeAgentLLMConfig extends LLMClientConfig {
  // Edge-specific configuration
  edge: {
    localCacheEnabled: boolean;
    offlineQueueSize: number;
    syncInterval: number;
    resourceLimits: ResourceLimits;
  };
}

/**
 * Resource limits for edge processing
 */
export interface ResourceLimits {
  maxCpuPercent: number;
  maxMemoryMB: number;
  maxDiskMB: number;
  maxNetworkKbps: number;
}

/**
 * Edge context
 */
export interface EdgeContext {
  nodeId: string;
  region: string;
  availableResources: ResourceInfo;
  networkQuality: 'high' | 'medium' | 'low';
  isOffline?: boolean;
}

/**
 * Resource information
 */
export interface ResourceInfo {
  cpuPercent: number;
  memoryMB: number;
  diskGB: number;
  networkKbps?: number;
}

/**
 * Edge processing result
 */
export interface EdgeProcessingResult {
  processedLocally: boolean;
  fallbackToCentral: boolean;
  result: any;
  latency: number;
  resourceUsage: ResourceUsage;
  cacheHit?: boolean;
}

/**
 * Resource usage tracking
 */
export interface ResourceUsage {
  cpuPercent: number;
  memoryMB: number;
  networkKbps: number;
  duration: number;
}

/**
 * Inference request for edge processing
 */
export interface InferenceRequest {
  requestId: string;
  model: string;
  input: string;
  context: EdgeContext;
  priority?: 'high' | 'medium' | 'low';
}

/**
 * Inference result
 */
export interface InferenceResult {
  requestId: string;
  output: string;
  confidence: number;
  metadata: Record<string, any>;
}

/**
 * Edge data sync
 */
export interface EdgeData {
  nodeId: string;
  timestamp: ISO8601Timestamp;
  metrics: Record<string, number>;
  events: any[];
  cachedResponses: number;
}

/**
 * Sync result
 */
export interface SyncResult {
  success: boolean;
  synced: number;
  failed: number;
  errors?: string[];
}

/**
 * Queued request for offline processing
 */
export interface QueuedRequest {
  id: string;
  request: LLMRequest;
  queuedAt: ISO8601Timestamp;
  priority: number;
  retryCount: number;
}

// ============================================================================
// SPECIALIZED CLIENT TYPES - GOVERNANCE
// ============================================================================

/**
 * Governance LLM Client Configuration
 */
export interface GovernanceLLMConfig extends LLMClientConfig {
  // Governance-specific configuration
  governance: {
    policyEngine: PolicyEngineConfig;
    complianceFrameworks: string[];
    auditLevel: 'strict' | 'standard' | 'minimal';
  };
}

/**
 * Policy engine configuration
 */
export interface PolicyEngineConfig {
  enabled: boolean;
  strictMode: boolean;
  policyPath: string;
  updateInterval: number;
}

/**
 * Compliance request
 */
export interface ComplianceRequest {
  requestId: string;
  action: string;
  context: GovernanceContext;
  policies: string[];
  data?: any;
}

/**
 * Governance context
 */
export interface GovernanceContext {
  userId: string;
  tenantId: string;
  environment: 'production' | 'staging' | 'development';
  dataClassification?: 'public' | 'internal' | 'confidential' | 'restricted';
  region?: string;
}

/**
 * Compliance result
 */
export interface ComplianceResult {
  compliant: boolean;
  violations: PolicyViolation[];
  warnings: string[];
  auditTrail: AuditEntry[];
  score: number;
}

/**
 * Policy violation
 */
export interface PolicyViolation {
  policyId: string;
  policyName: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  description: string;
  remediation: string;
  affectedData?: string[];
}

/**
 * Policy definition
 */
export interface Policy {
  id: string;
  name: string;
  description: string;
  version: string;
  rules: PolicyRule[];
  enabled: boolean;
}

/**
 * Policy rule
 */
export interface PolicyRule {
  id: string;
  condition: string;
  action: 'allow' | 'deny' | 'warn';
  message: string;
}

/**
 * Audit entry
 */
export interface AuditEntry {
  timestamp: ISO8601Timestamp;
  action: string;
  actor: string;
  result: 'success' | 'failure' | 'warning';
  details: Record<string, any>;
}

/**
 * Audit request
 */
export interface AuditRequest {
  requestId: string;
  startDate: ISO8601Timestamp;
  endDate: ISO8601Timestamp;
  filters?: AuditFilters;
}

/**
 * Audit filters
 */
export interface AuditFilters {
  userId?: string;
  action?: string;
  result?: 'success' | 'failure' | 'warning';
  policyId?: string;
}

/**
 * Audit report
 */
export interface AuditReport {
  reportId: string;
  generatedAt: ISO8601Timestamp;
  period: {
    start: ISO8601Timestamp;
    end: ISO8601Timestamp;
  };
  summary: AuditSummary;
  entries: AuditEntry[];
  violations: PolicyViolation[];
}

/**
 * Audit summary
 */
export interface AuditSummary {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  warningCount: number;
  violationCount: number;
  complianceScore: number;
}

/**
 * Compliance report
 */
export interface ComplianceReport {
  reportId: string;
  generatedAt: ISO8601Timestamp;
  framework: string;
  overallCompliance: number;
  sections: ComplianceSection[];
  recommendations: string[];
}

/**
 * Compliance section
 */
export interface ComplianceSection {
  id: string;
  name: string;
  compliant: boolean;
  score: number;
  findings: string[];
  evidence: string[];
}

// ============================================================================
// PROVIDER INTERFACE TYPES
// ============================================================================

/**
 * Provider capabilities
 */
export interface ProviderCapabilities {
  streaming: boolean;
  functionCalling: boolean;
  vision: boolean;
  embeddings?: boolean;
  jsonMode?: boolean;
  maxTokens: number;
  maxContextWindow: number;
}

/**
 * Model information
 */
export interface ModelInfo {
  id: string;
  name: string;
  description?: string;
  contextWindow: number;
  maxOutputTokens: number;
  supportsVision?: boolean;
  supportsFunctions?: boolean;
}

/**
 * Cost estimate
 */
export interface CostEstimate {
  estimatedCost: number;
  currency: string;
  breakdown?: {
    promptCost: number;
    completionCost: number;
  };
}

// ============================================================================
// MIDDLEWARE TYPES
// ============================================================================

/**
 * Request context for middleware
 */
export interface RequestContext {
  requestId: string;
  retryCount: number;
  startTime: number;
  config: LLMClientConfig;
  metadata: Record<string, any>;
  traceId?: string;
  spanId?: string;
}

/**
 * Middleware interface
 */
export interface IMiddleware {
  readonly name: string;
  readonly priority: number;

  onRequest(request: LLMRequest, context: RequestContext): Promise<LLMRequest>;
  onResponse(response: LLMResponse, context: RequestContext): Promise<LLMResponse>;
  onError(error: Error, context: RequestContext): Promise<void>;
}

// ============================================================================
// OBSERVABILITY TYPES
// ============================================================================

/**
 * Logger interface
 */
export interface ILogger {
  debug(message: string, context?: Record<string, any>): void;
  info(message: string, context?: Record<string, any>): void;
  warn(message: string, context?: Record<string, any>): void;
  error(message: string, context?: Record<string, any>): void;
}

/**
 * Metrics interface
 */
export interface IMetrics {
  increment(metric: string, labels?: Record<string, string>, value?: number): void;
  gauge(metric: string, value: number, labels?: Record<string, string>): void;
  histogram(metric: string, value: number, labels?: Record<string, string>): void;
  timer(metric: string): () => void;
}

/**
 * Tracer interface
 */
export interface ITracer {
  startSpan(name: string, context?: any): ISpan;
}

/**
 * Span interface
 */
export interface ISpan {
  setTag(key: string, value: any): void;
  log(event: string, payload?: any): void;
  finish(): void;
  context(): any;
}

/**
 * Request metrics
 */
export interface RequestMetrics {
  requestId: string;
  model: string;
  durationMs: number;
  tokenUsage: TokenUsage;
  success: boolean;
  errorType?: string;
  retryCount: number;
}

// ============================================================================
// RESILIENCE TYPES
// ============================================================================

/**
 * Circuit breaker states
 */
export enum CircuitState {
  CLOSED = 'CLOSED',
  OPEN = 'OPEN',
  HALF_OPEN = 'HALF_OPEN',
}

/**
 * Circuit breaker statistics
 */
export interface CircuitBreakerStats {
  state: CircuitState;
  failureCount: number;
  successCount: number;
  requestCount: number;
  lastFailureTime: number;
}

/**
 * Rate limiter statistics
 */
export interface RateLimiterStats {
  requestBucket: {
    tokens: number;
    capacity: number;
    refillRate: number;
  };
  tokenBucket: {
    tokens: number;
    capacity: number;
    refillRate: number;
  };
}

/**
 * Error recovery action
 */
export interface ErrorRecoveryAction {
  action: 'retry' | 'circuit_open' | 'fail';
  delayMs?: number;
  error?: LLMError;
  message?: string;
}

// ============================================================================
// VALIDATION TYPES
// ============================================================================

/**
 * Validation result
 */
export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings?: string[];
}

/**
 * Content filter interface
 */
export interface IContentFilter {
  containsMaliciousContent(content: string): Promise<boolean>;
  filterContent(content: string): Promise<string>;
}

/**
 * Schema validator interface
 */
export interface ISchemaValidator {
  validate(data: any): boolean;
  getErrors(): any[];
}

// ============================================================================
// SECURITY TYPES
// ============================================================================

/**
 * PII finding
 */
export interface PIIFinding {
  type: string;
  count: number;
  positions: number[];
}

/**
 * Redaction result
 */
export interface RedactionResult {
  original: string;
  redacted: string;
  findings: PIIFinding[];
  containsPII: boolean;
}

/**
 * API key with metadata
 */
export interface ApiKey {
  value: string;
  createdAt: number;
  expiresAt?: number;
  provider: string;
  environment: string;
}

/**
 * Key store interface
 */
export interface IKeyStore {
  retrieve(provider: string): Promise<ApiKey>;
  store(provider: string, key: string): Promise<void>;
  delete(provider: string): Promise<void>;
  rotate(provider: string): Promise<ApiKey>;
}

/**
 * Key rotation policy
 */
export interface KeyRotationPolicy {
  maxAgeMs: number;
  rotationWarningMs: number;
  autoRotate: boolean;
}

// ============================================================================
// CACHING TYPES
// ============================================================================

/**
 * Cache entry
 */
export interface CacheEntry {
  response: LLMResponse;
  timestamp: number;
  hitCount: number;
}

/**
 * Cache statistics
 */
export interface CacheStats {
  size: number;
  hitRate: number;
  missRate: number;
  evictionCount: number;
}

// ============================================================================
// TESTING TYPES
// ============================================================================

/**
 * Mock provider configuration
 */
export interface MockProviderConfig {
  responses: Map<string, CompletionResponse>;
  errors: Map<string, Error>;
  latencyMs?: number;
}

/**
 * Load test configuration
 */
export interface LoadTestConfig {
  durationSeconds: number;
  requestsPerSecond: number;
  concurrentRequests: number;
}

/**
 * Load test report
 */
export interface LoadTestReport {
  totalRequests: number;
  successCount: number;
  failureCount: number;
  successRate: number;
  latency: {
    min: number;
    max: number;
    mean: number;
    p50: number;
    p95: number;
    p99: number;
  };
  errors: Record<string, number>;
}

// ============================================================================
// UTILITY TYPES
// ============================================================================

/**
 * Async iterable for streaming
 */
export type AsyncIterableIterator<T> = {
  [Symbol.asyncIterator](): AsyncIterator<T>;
  next(): Promise<IteratorResult<T>>;
};

/**
 * Deep partial type
 */
export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

/**
 * Promisable type
 */
export type Promisable<T> = T | Promise<T>;

/**
 * Optional fields type
 */
export type Optional<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>;

/**
 * Required fields type
 */
export type RequiredFields<T, K extends keyof T> = T & Required<Pick<T, K>>;

// ============================================================================
// CONSTANTS
// ============================================================================

/**
 * Default configuration values
 */
export const DEFAULT_CONFIG = {
  RETRY_MAX_ATTEMPTS: 3,
  RETRY_BASE_DELAY_MS: 1000,
  RETRY_MAX_DELAY_MS: 30000,
  RETRY_BACKOFF_MULTIPLIER: 2,
  RETRY_JITTER_MS: 100,

  CIRCUIT_BREAKER_FAILURE_THRESHOLD: 5,
  CIRCUIT_BREAKER_SUCCESS_THRESHOLD: 2,
  CIRCUIT_BREAKER_TIMEOUT_MS: 60000,
  CIRCUIT_BREAKER_VOLUME_THRESHOLD: 10,

  RATE_LIMIT_REQUESTS_PER_MINUTE: 50,
  RATE_LIMIT_REQUESTS_PER_HOUR: 2000,
  RATE_LIMIT_TOKENS_PER_MINUTE: 100000,
  RATE_LIMIT_TOKENS_PER_DAY: 5000000,

  TIMEOUT_REQUEST_MS: 30000,
  TIMEOUT_CONNECTION_MS: 5000,
  TIMEOUT_STREAM_MS: 120000,

  CACHE_TTL_MS: 3600000,
  CACHE_MAX_SIZE: 1000,

  LOG_LEVEL: 'info' as const,
  LOG_MAX_BODY_LENGTH: 10000,
} as const;

/**
 * Provider-specific constants
 */
export const PROVIDER_LIMITS = {
  openai: {
    'gpt-4': {
      contextWindow: 8192,
      maxTokens: 4096,
    },
    'gpt-4-32k': {
      contextWindow: 32768,
      maxTokens: 4096,
    },
    'gpt-3.5-turbo': {
      contextWindow: 4096,
      maxTokens: 4096,
    },
  },
  anthropic: {
    'claude-3-opus-20240229': {
      contextWindow: 200000,
      maxTokens: 4096,
    },
    'claude-3-5-sonnet-20241022': {
      contextWindow: 200000,
      maxTokens: 8192,
    },
  },
} as const;
