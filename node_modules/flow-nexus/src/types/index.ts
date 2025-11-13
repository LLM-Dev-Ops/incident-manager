export interface AgentInstance {
  id: string;
  type: AgentType;
  status: 'idle' | 'busy' | 'error';
  lastActivity: Date;
  config: AgentConfig;
  performance: {
    tasksCompleted: number;
    averageResponseTime: number;
    errorRate: number;
  };
}

export type AgentType = 'curator' | 'pricing' | 'security' | 'recommendation';

export interface AgentConfig {
  [key: string]: any;
}

export interface CuratorConfig extends AgentConfig {
  qualityThreshold: number;
  autoApprove: boolean;
  learningRate: number;
  batchSize: number;
}

export interface PricingConfig extends AgentConfig {
  priceElasticity: number;
  marketCap: number;
  updateInterval: number;
  volatilityThreshold: number;
}

export interface SecurityConfig extends AgentConfig {
  scanDepth: 'basic' | 'comprehensive' | 'thorough';
  cveDatabase: string;
  aiModelPath: string;
  parallelScans: number;
}

export interface RecommendationConfig extends AgentConfig {
  modelType: 'collaborative_filtering' | 'content_based' | 'hybrid';
  embeddingDim: number;
  updateFrequency: number;
  minUserInteractions: number;
}

export interface TaskResult {
  success: boolean;
  data?: any;
  error?: string;
  executionTime: number;
  agentId: string;
}

export interface WasmModule {
  initialize(): Promise<void>;
  spawnAgent(type: AgentType, config: AgentConfig): Promise<string>;
  executeFunction(agentId: string, functionName: string, params: any): Promise<any>;
  terminateAgent(agentId: string): Promise<void>;
  getMetrics(agentId?: string): Promise<any>;
  cleanup(): Promise<void>;
}

export interface DatabaseConfig {
  url: string;
  serviceKey: string;
  anonKey?: string;
}

export interface ServerConfig {
  host: string;
  port: number;
  cors: {
    origin: string[];
    credentials: boolean;
  };
}

export interface AuthConfig {
  jwtSecret: string;
  issuer: string;
  audience: string;
}

export interface MCPRequest {
  method: string;
  params?: any;
  id?: string;
}

export interface MCPResponse {
  jsonrpc: string;
  id?: string;
  result?: any;
  error?: {
    code: number;
    message: string;
    data?: any;
  };
}

export interface UserProfile {
  id: string;
  username: string;
  developer_level: number;
  permissions: string[];
}

export interface AuthResult {
  success: boolean;
  user?: UserProfile;
  error?: string;
}

export interface RateLimit {
  requests: number;
  window: number; // seconds
}

export interface ApplicationData {
  id: string;
  name: string;
  description: string;
  repository_url: string;
  status: 'pending' | 'approved' | 'rejected';
  quality_score?: number;
  assessment_metadata?: any;
}

export interface QualityAssessmentResult {
  qualityScore: number;
  autoApprove: boolean;
  metadata: {
    codeQuality: number;
    documentation: number;
    security: number;
    performance: number;
    issues: string[];
    recommendations: string[];
  };
}

export interface PricingAnalysisResult {
  recommendedPrice: number;
  marketPosition: 'low' | 'medium' | 'high';
  elasticity: number;
  competitorAnalysis: {
    averagePrice: number;
    priceRange: [number, number];
    marketShare: number;
  };
}

export interface SecurityScanResult {
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
  vulnerabilities: Array<{
    severity: string;
    type: string;
    description: string;
    file?: string;
    line?: number;
    fix?: string;
  }>;
  compliance: {
    score: number;
    issues: string[];
  };
}

export interface RecommendationResult {
  applications: Array<{
    id: string;
    name: string;
    relevanceScore: number;
    reason: string;
  }>;
  confidence: number;
  modelVersion: string;
}