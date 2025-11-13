import { z } from 'zod';
import { WasmDAARuntime } from '../wasm/runtime';
import { AgentType } from '../types';

// Validation schemas
const SpawnAgentSchema = z.object({
  type: z.enum(['curator', 'pricing', 'security', 'recommendation']),
  config: z.record(z.any()).optional(),
  name: z.string().optional()
});

const ExecuteFunctionSchema = z.object({
  agentId: z.string(),
  functionName: z.string(),
  params: z.record(z.any()).optional()
});

const TrainModelSchema = z.object({
  agentId: z.string(),
  trainingData: z.array(z.any()),
  epochs: z.number().min(1).max(1000).optional(),
  learningRate: z.number().min(0.0001).max(1).optional()
});

const AssessQualitySchema = z.object({
  repositoryUrl: z.string().url(),
  applicationData: z.object({
    name: z.string(),
    description: z.string(),
    category: z.string()
  }),
  criteria: z.object({
    codeQuality: z.boolean().optional(),
    documentation: z.boolean().optional(),
    security: z.boolean().optional(),
    performance: z.boolean().optional()
  }).optional()
});

export class DAAAgentTools {
  constructor(private runtime: WasmDAARuntime) {}

  getTools() {
    return [
      {
        name: 'daa_agent_spawn',
        description: 'Spawn a new DAA agent with specified configuration',
        inputSchema: {
          type: 'object',
          properties: {
            type: {
              type: 'string',
              enum: ['curator', 'pricing', 'security', 'recommendation'],
              description: 'Type of agent to spawn'
            },
            config: {
              type: 'object',
              description: 'Agent-specific configuration parameters',
              additionalProperties: true
            },
            name: {
              type: 'string',
              description: 'Optional human-readable name for the agent'
            }
          },
          required: ['type']
        },
        handler: this.spawnAgent.bind(this)
      },
      {
        name: 'daa_agent_execute',
        description: 'Execute a function on a specific DAA agent',
        inputSchema: {
          type: 'object',
          properties: {
            agentId: {
              type: 'string',
              description: 'ID of the agent to execute function on'
            },
            functionName: {
              type: 'string',
              description: 'Name of the function to execute'
            },
            params: {
              type: 'object',
              description: 'Parameters to pass to the function',
              additionalProperties: true
            }
          },
          required: ['agentId', 'functionName']
        },
        handler: this.executeFunction.bind(this)
      },
      {
        name: 'daa_agent_train',
        description: 'Train ML models within DAA agents',
        inputSchema: {
          type: 'object',
          properties: {
            agentId: {
              type: 'string',
              description: 'ID of the agent to train'
            },
            trainingData: {
              type: 'array',
              description: 'Training dataset',
              items: { type: 'object' }
            },
            epochs: {
              type: 'number',
              minimum: 1,
              maximum: 1000,
              description: 'Number of training epochs'
            },
            learningRate: {
              type: 'number',
              minimum: 0.0001,
              maximum: 1,
              description: 'Learning rate for training'
            }
          },
          required: ['agentId', 'trainingData']
        },
        handler: this.trainModel.bind(this)
      },
      {
        name: 'daa_assess_quality',
        description: 'Assess application quality using curator agents',
        inputSchema: {
          type: 'object',
          properties: {
            repositoryUrl: {
              type: 'string',
              format: 'uri',
              description: 'URL of the repository to assess'
            },
            applicationData: {
              type: 'object',
              properties: {
                name: { type: 'string' },
                description: { type: 'string' },
                category: { type: 'string' }
              },
              required: ['name', 'description', 'category']
            },
            criteria: {
              type: 'object',
              properties: {
                codeQuality: { type: 'boolean' },
                documentation: { type: 'boolean' },
                security: { type: 'boolean' },
                performance: { type: 'boolean' }
              }
            }
          },
          required: ['repositoryUrl', 'applicationData']
        },
        handler: this.assessQuality.bind(this)
      },
      {
        name: 'daa_analyze_pricing',
        description: 'Analyze optimal pricing for applications using pricing agents',
        inputSchema: {
          type: 'object',
          properties: {
            applicationId: {
              type: 'string',
              description: 'Application ID to analyze pricing for'
            },
            marketData: {
              type: 'object',
              description: 'Current market conditions and competitor data'
            },
            currentPrice: {
              type: 'number',
              minimum: 0,
              description: 'Current price of the application'
            }
          },
          required: ['applicationId']
        },
        handler: this.analyzePricing.bind(this)
      },
      {
        name: 'daa_security_scan',
        description: 'Perform security scanning using security agents',
        inputSchema: {
          type: 'object',
          properties: {
            repositoryUrl: {
              type: 'string',
              format: 'uri',
              description: 'Repository URL to scan'
            },
            scanDepth: {
              type: 'string',
              enum: ['basic', 'comprehensive', 'thorough'],
              description: 'Depth of security scan'
            },
            includeCompliance: {
              type: 'boolean',
              description: 'Include compliance checks'
            }
          },
          required: ['repositoryUrl']
        },
        handler: this.securityScan.bind(this)
      },
      {
        name: 'daa_generate_recommendations',
        description: 'Generate personalized recommendations using recommendation agents',
        inputSchema: {
          type: 'object',
          properties: {
            userId: {
              type: 'string',
              description: 'User ID to generate recommendations for'
            },
            category: {
              type: 'string',
              description: 'Optional category filter'
            },
            limit: {
              type: 'number',
              minimum: 1,
              maximum: 50,
              description: 'Maximum number of recommendations'
            },
            context: {
              type: 'object',
              description: 'Additional context for recommendations'
            }
          },
          required: ['userId']
        },
        handler: this.generateRecommendations.bind(this)
      },
      {
        name: 'daa_agent_metrics',
        description: 'Get performance metrics for DAA agents',
        inputSchema: {
          type: 'object',
          properties: {
            agentId: {
              type: 'string',
              description: 'Specific agent ID (optional)'
            },
            agentType: {
              type: 'string',
              enum: ['curator', 'pricing', 'security', 'recommendation'],
              description: 'Filter by agent type'
            },
            includePerformance: {
              type: 'boolean',
              description: 'Include detailed performance metrics'
            }
          }
        },
        handler: this.getAgentMetrics.bind(this)
      },
      {
        name: 'daa_agent_list',
        description: 'List all active DAA agents',
        inputSchema: {
          type: 'object',
          properties: {
            status: {
              type: 'string',
              enum: ['all', 'idle', 'busy', 'error'],
              description: 'Filter agents by status'
            },
            type: {
              type: 'string',
              enum: ['curator', 'pricing', 'security', 'recommendation'],
              description: 'Filter by agent type'
            }
          }
        },
        handler: this.listAgents.bind(this)
      },
      {
        name: 'daa_agent_terminate',
        description: 'Terminate a specific DAA agent',
        inputSchema: {
          type: 'object',
          properties: {
            agentId: {
              type: 'string',
              description: 'ID of the agent to terminate'
            },
            force: {
              type: 'boolean',
              description: 'Force termination even if agent is busy'
            }
          },
          required: ['agentId']
        },
        handler: this.terminateAgent.bind(this)
      }
    ];
  }

  private async spawnAgent(params: any): Promise<any> {
    try {
      const validated = SpawnAgentSchema.parse(params);
      
      if (!this.runtime.isInitialized) {
        throw new Error('WASM DAA Runtime not initialized');
      }

      const agentId = await this.runtime.spawnAgent(
        validated.type as AgentType,
        validated.config || {}
      );

      return {
        success: true,
        agentId,
        type: validated.type,
        name: validated.name || `${validated.type}-agent`,
        status: 'idle',
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error occurred'
      };
    }
  }

  private async executeFunction(params: any): Promise<any> {
    try {
      const validated = ExecuteFunctionSchema.parse(params);
      
      const result = await this.runtime.executeFunction(
        validated.agentId,
        validated.functionName,
        validated.params || {}
      );

      return {
        success: true,
        agentId: validated.agentId,
        functionName: validated.functionName,
        result,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error occurred'
      };
    }
  }

  private async trainModel(params: any): Promise<any> {
    try {
      const validated = TrainModelSchema.parse(params);
      
      const result = await this.runtime.executeFunction(
        validated.agentId,
        'train_model',
        {
          trainingData: validated.trainingData,
          epochs: validated.epochs || 10,
          learningRate: validated.learningRate || 0.01
        }
      );

      return {
        success: true,
        agentId: validated.agentId,
        trainingResult: result,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Training failed'
      };
    }
  }

  private async assessQuality(params: any): Promise<any> {
    try {
      const validated = AssessQualitySchema.parse(params);
      
      // Find available curator agent
      const curatorAgents = this.runtime.getAgentsByType('curator');
      const availableAgent = curatorAgents.find(agent => agent.status === 'idle');
      
      if (!availableAgent) {
        // Spawn a new curator agent if none available
        const agentId = await this.runtime.spawnAgent('curator', {
          qualityThreshold: 0.8,
          autoApprove: false
        });
        
        const result = await this.runtime.executeFunction(agentId, 'assess_quality', {
          repositoryUrl: validated.repositoryUrl,
          applicationData: validated.applicationData,
          criteria: validated.criteria || {
            codeQuality: true,
            documentation: true,
            security: true,
            performance: true
          }
        });

        return {
          success: true,
          assessment: result,
          agentId,
          timestamp: new Date().toISOString()
        };
      }

      const result = await this.runtime.executeFunction(
        availableAgent.id,
        'assess_quality',
        {
          repositoryUrl: validated.repositoryUrl,
          applicationData: validated.applicationData,
          criteria: validated.criteria
        }
      );

      return {
        success: true,
        assessment: result,
        agentId: availableAgent.id,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Quality assessment failed'
      };
    }
  }

  private async analyzePricing(params: any): Promise<any> {
    try {
      // Find available pricing agent
      const pricingAgents = this.runtime.getAgentsByType('pricing');
      let agent = pricingAgents.find(a => a.status === 'idle');
      
      if (!agent) {
        const agentId = await this.runtime.spawnAgent('pricing', {
          priceElasticity: 0.3,
          marketCap: 1000000
        });
        agent = this.runtime.getAgent(agentId);
      }

      if (!agent) {
        throw new Error('Failed to get pricing agent');
      }

      const result = await this.runtime.executeFunction(
        agent.id,
        'analyze_pricing',
        params
      );

      return {
        success: true,
        pricingAnalysis: result,
        agentId: agent.id,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Pricing analysis failed'
      };
    }
  }

  private async securityScan(params: any): Promise<any> {
    try {
      // Find available security agent
      const securityAgents = this.runtime.getAgentsByType('security');
      let agent = securityAgents.find(a => a.status === 'idle');
      
      if (!agent) {
        const agentId = await this.runtime.spawnAgent('security', {
          scanDepth: params.scanDepth || 'comprehensive',
          cveDatabase: 'latest'
        });
        agent = this.runtime.getAgent(agentId);
      }

      if (!agent) {
        throw new Error('Failed to get security agent');
      }

      const result = await this.runtime.executeFunction(
        agent.id,
        'scan_repository',
        params
      );

      return {
        success: true,
        securityScan: result,
        agentId: agent.id,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Security scan failed'
      };
    }
  }

  private async generateRecommendations(params: any): Promise<any> {
    try {
      // Find available recommendation agent
      const recommendationAgents = this.runtime.getAgentsByType('recommendation');
      let agent = recommendationAgents.find(a => a.status === 'idle');
      
      if (!agent) {
        const agentId = await this.runtime.spawnAgent('recommendation', {
          modelType: 'collaborative_filtering',
          embeddingDim: 128
        });
        agent = this.runtime.getAgent(agentId);
      }

      if (!agent) {
        throw new Error('Failed to get recommendation agent');
      }

      const result = await this.runtime.executeFunction(
        agent.id,
        'generate_recommendations',
        {
          userId: params.userId,
          category: params.category,
          limit: params.limit || 10,
          context: params.context || {}
        }
      );

      return {
        success: true,
        recommendations: result,
        agentId: agent.id,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Recommendation generation failed'
      };
    }
  }

  private async getAgentMetrics(params: any): Promise<any> {
    try {
      const metrics = await this.runtime.getMetrics(params.agentId);
      
      return {
        success: true,
        metrics,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to get metrics'
      };
    }
  }

  private async listAgents(params: any): Promise<any> {
    try {
      const allAgents = Array.from({ length: this.runtime.totalAgents }, (_, i) => {
        // This is a simplified version - in reality we'd iterate through actual agents
        return this.runtime.getAgentsByType('curator')[0]; // Placeholder
      });

      let filteredAgents = allAgents;

      if (params.status && params.status !== 'all') {
        filteredAgents = filteredAgents.filter(agent => agent?.status === params.status);
      }

      if (params.type) {
        filteredAgents = filteredAgents.filter(agent => agent?.type === params.type);
      }

      const agentList = filteredAgents.map(agent => ({
        id: agent?.id,
        type: agent?.type,
        status: agent?.status,
        lastActivity: agent?.lastActivity,
        performance: agent?.performance
      })).filter(Boolean);

      return {
        success: true,
        agents: agentList,
        total: agentList.length,
        active: this.runtime.activeAgents,
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to list agents'
      };
    }
  }

  private async terminateAgent(params: any): Promise<any> {
    try {
      const agent = this.runtime.getAgent(params.agentId);
      if (!agent) {
        throw new Error(`Agent ${params.agentId} not found`);
      }

      if (agent.status === 'busy' && !params.force) {
        throw new Error('Agent is busy. Use force=true to terminate anyway');
      }

      await this.runtime.terminateAgent(params.agentId);

      return {
        success: true,
        agentId: params.agentId,
        message: 'Agent terminated successfully',
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to terminate agent'
      };
    }
  }
}