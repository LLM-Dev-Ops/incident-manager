/**
 * @llm-dev-ops/incident-manager-types
 *
 * TypeScript type definitions for LLM Incident Manager
 * Enterprise-grade incident management system for LLM operations
 *
 * @version 1.0.1
 * @license MIT OR Apache-2.0
 */

// Re-export all data model types
export * from './data-models.js';

// Re-export LLM client types (excluding duplicates)
export type {
  // Core LLM Types
  LLMRequest,
  Message,
  ContentBlock,
  TextBlock,
  ImageBlock,
  DocumentBlock,
  FunctionCall,
  FunctionDefinition,
  JSONSchema,
  LLMResponse,
  TokenUsage,
  CompletionChunk,
  CompletionRequest,
  CompletionResponse,
  EmbeddingRequest,
  EmbeddingResponse,

  // Error Types
  LLMErrorType,

  // Configuration Types
  LLMClientConfig,
  ProviderConfig,
  OpenAIConfig,
  AnthropicConfig,
  AzureConfig,
  VertexConfig,
  ResilienceConfig,
  RetryConfig,
  CircuitBreakerConfig,
  RateLimitConfig,
  TimeoutConfig,
  ObservabilityConfig,
  LoggingConfig,
  MetricsConfig,
  TracingConfig,
  AuditConfig,
  FeatureConfig,

  // Sentinel Types
  SentinelLLMConfig,
  AnomalyEvent,
  MetricData,
  AnomalyAnalysis,
  SeverityClassification,
  ImpactAssessment,
  Insight,
  ImpactPrediction,

  // Shield Types
  ShieldLLMConfig,
  SecurityEvent,
  SecurityContext,
  ThreatAnalysis,
  RiskAssessment,
  Vulnerability,
  MaliciousPattern,
  MitigationPlan,
  MitigationStep,
  ComplianceStatus,

  // Edge-Agent Types
  EdgeAgentLLMConfig,
  ResourceLimits,
  EdgeContext,
  ResourceInfo as EdgeResourceInfo,
  EdgeProcessingResult,
  ResourceUsage,
  InferenceRequest,
  InferenceResult,
  EdgeData,
  SyncResult,
  QueuedRequest,

  // Governance Types
  GovernanceLLMConfig,
  PolicyEngineConfig,
  ComplianceRequest,
  GovernanceContext,
  ComplianceResult,
  PolicyViolation,
  Policy,
  PolicyRule,
  AuditEntry,
  AuditRequest,
  AuditFilters,
  AuditReport,
  AuditSummary,
  ComplianceReport,
  ComplianceSection,

  // Provider Types
  ProviderCapabilities,
  ModelInfo,
  CostEstimate,

  // Middleware Types
  RequestContext,
  IMiddleware,

  // Observability Types
  ILogger,
  IMetrics,
  ITracer,
  ISpan,
  RequestMetrics,

  // Resilience Types
  CircuitState,
  CircuitBreakerStats,
  RateLimiterStats,
  ErrorRecoveryAction,

  // Validation Types
  ValidationResult,
  IContentFilter,
  ISchemaValidator,

  // Security Types
  PIIFinding,
  RedactionResult,
  ApiKey,
  IKeyStore,
  KeyRotationPolicy,

  // Caching Types
  CacheEntry,
  CacheStats,

  // Testing Types
  MockProviderConfig,
  LoadTestConfig,
  LoadTestReport,

  // Utility Types
  AsyncIterableIterator,
  DeepPartial,
  Promisable,
  Optional,
  RequiredFields
} from './llm-client-types.js';

// Re-export the LLMError class
export { LLMError, DEFAULT_CONFIG, PROVIDER_LIMITS } from './llm-client-types.js';
