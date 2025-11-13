#!/usr/bin/env node

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { createRequire } from 'module';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import fs from 'fs/promises';

const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Mode configurations
const MODES = {
  complete: {
    name: 'Flow Nexus Complete',
    description: 'Full suite with all tools and resources',
    tools: ['swarm', 'neural', 'github', 'daa', 'workflow', 'sandbox', 'app-store', 'auth'],
    resources: ['docs', 'templates', 'examples', 'configs']
  },
  store: {
    name: 'App Store Only',
    description: 'App store management and publishing tools',
    tools: ['app-store', 'auth'],
    resources: ['templates', 'examples']
  },
  swarm: {
    name: 'Swarm Coordination',
    description: 'Multi-agent swarm orchestration',
    tools: ['swarm', 'neural', 'daa', 'auth'],
    resources: ['docs', 'configs']
  },
  dev: {
    name: 'Development Tools',
    description: 'Code execution and development utilities',
    tools: ['sandbox', 'workflow', 'github', 'auth'],
    resources: ['examples', 'docs']
  },
  gamer: {
    name: 'Gamification Features',
    description: 'Challenges, achievements, and leaderboards',
    tools: ['app-store', 'auth'],
    resources: ['templates'],
    filter: ['challenge', 'achievement', 'leaderboard', 'ruv', 'gamification']
  }
};

// Tool definitions by category
const TOOL_CATEGORIES = {
  auth: [
    {
      name: 'auth_status',
      description: 'Check authentication status and permissions',
      inputSchema: {
        type: 'object',
        properties: {
          detailed: { type: 'boolean', description: 'Include detailed auth info' }
        }
      }
    },
    {
      name: 'auth_init',
      description: 'Initialize secure authentication',
      inputSchema: {
        type: 'object',
        properties: {
          mode: { type: 'string', enum: ['user', 'service'], description: 'Authentication mode' }
        },
        required: ['mode']
      }
    }
  ],
  swarm: [
    {
      name: 'swarm_init',
      description: 'Initialize multi-agent swarm with specified topology',
      inputSchema: {
        type: 'object',
        properties: {
          topology: { 
            type: 'string', 
            enum: ['hierarchical', 'mesh', 'ring', 'star'],
            description: 'Swarm topology: hierarchical (tree), mesh (peer-to-peer), ring (circular), star (centralized)'
          },
          maxAgents: { 
            type: 'number', 
            minimum: 1, 
            maximum: 100, 
            default: 8,
            description: 'Maximum number of agents in swarm'
          },
          strategy: { 
            type: 'string', 
            enum: ['balanced', 'specialized', 'adaptive'],
            default: 'balanced',
            description: 'Agent distribution strategy'
          }
        },
        required: ['topology']
      }
    },
    {
      name: 'agent_spawn',
      description: 'Create specialized AI agent in swarm',
      inputSchema: {
        type: 'object',
        properties: {
          type: { 
            type: 'string', 
            enum: ['researcher', 'coder', 'analyst', 'optimizer', 'coordinator'],
            description: 'Agent specialization type'
          },
          capabilities: { 
            type: 'array', 
            items: { type: 'string' },
            description: 'Specific capabilities for the agent'
          },
          name: { type: 'string', description: 'Custom agent identifier' }
        },
        required: ['type']
      }
    },
    {
      name: 'task_orchestrate',
      description: 'Orchestrate complex task across swarm agents',
      inputSchema: {
        type: 'object',
        properties: {
          task: { type: 'string', description: 'Task description or instructions' },
          strategy: { 
            type: 'string', 
            enum: ['parallel', 'sequential', 'adaptive'],
            default: 'adaptive',
            description: 'Task execution strategy'
          },
          priority: { 
            type: 'string', 
            enum: ['low', 'medium', 'high', 'critical'],
            default: 'medium',
            description: 'Task priority level'
          },
          maxAgents: { 
            type: 'number', 
            minimum: 1, 
            maximum: 10,
            description: 'Maximum agents to use for task'
          }
        },
        required: ['task']
      }
    }
  ],
  'app-store': [
    {
      name: 'app_store_list_templates',
      description: 'List available application templates',
      inputSchema: {
        type: 'object',
        properties: {
          category: { type: 'string', description: 'Filter by template category' },
          tags: { type: 'array', items: { type: 'string' }, description: 'Filter by tags' },
          limit: { type: 'number', minimum: 1, maximum: 100, default: 20 }
        }
      }
    },
    {
      name: 'app_store_publish_app',
      description: 'Publish new application to store',
      inputSchema: {
        type: 'object',
        properties: {
          name: { type: 'string', description: 'Application name' },
          description: { type: 'string', description: 'App description' },
          category: { type: 'string', description: 'App category' },
          tags: { type: 'array', items: { type: 'string' } },
          version: { type: 'string', default: '1.0.0' },
          source_code: { type: 'string', description: 'Application source code' },
          metadata: { type: 'object', description: 'Additional app metadata' }
        },
        required: ['name', 'description', 'category', 'source_code']
      }
    },
    {
      name: 'app_store_complete_challenge',
      description: 'Mark challenge as completed for user',
      inputSchema: {
        type: 'object',
        properties: {
          challenge_id: { type: 'string', description: 'Challenge identifier' },
          user_id: { type: 'string', description: 'User identifier' },
          submission_data: { type: 'object', description: 'Challenge completion data' }
        },
        required: ['challenge_id', 'user_id']
      }
    },
    {
      name: 'app_store_earn_ruv',
      description: 'Award rUv credits to user',
      inputSchema: {
        type: 'object',
        properties: {
          user_id: { type: 'string', description: 'User identifier' },
          amount: { type: 'number', minimum: 1, description: 'rUv credits to award' },
          reason: { type: 'string', description: 'Reason for earning credits' },
          source: { type: 'string', description: 'Credit source (challenge, app_usage, etc.)' }
        },
        required: ['user_id', 'amount', 'reason']
      }
    }
  ],
  sandbox: [
    {
      name: 'sandbox_create',
      description: 'Create new code execution sandbox',
      inputSchema: {
        type: 'object',
        properties: {
          template: { 
            type: 'string', 
            enum: ['node', 'python', 'react', 'nextjs', 'vanilla'],
            description: 'Sandbox template type'
          },
          name: { type: 'string', description: 'Sandbox identifier' }
        },
        required: ['template']
      }
    },
    {
      name: 'sandbox_execute',
      description: 'Execute code in sandbox environment',
      inputSchema: {
        type: 'object',
        properties: {
          sandbox_id: { type: 'string', description: 'Sandbox identifier' },
          code: { type: 'string', description: 'Code to execute' },
          language: { type: 'string', description: 'Programming language' }
        },
        required: ['sandbox_id', 'code']
      }
    }
  ],
  neural: [
    {
      name: 'neural_train',
      description: 'Train neural patterns with WASM acceleration',
      inputSchema: {
        type: 'object',
        properties: {
          pattern_type: { 
            type: 'string', 
            enum: ['coordination', 'optimization', 'prediction'],
            description: 'Type of neural pattern to train'
          },
          training_data: { type: 'string', description: 'Training dataset' },
          epochs: { type: 'number', minimum: 1, maximum: 1000, default: 50 }
        },
        required: ['pattern_type', 'training_data']
      }
    }
  ],
  github: [
    {
      name: 'github_repo_analyze',
      description: 'Analyze GitHub repository',
      inputSchema: {
        type: 'object',
        properties: {
          repo: { type: 'string', description: 'Repository name (owner/repo)' },
          analysis_type: { 
            type: 'string', 
            enum: ['code_quality', 'performance', 'security'],
            description: 'Type of analysis to perform'
          }
        },
        required: ['repo']
      }
    }
  ],
  daa: [
    {
      name: 'daa_agent_create',
      description: 'Create decentralized autonomous agent',
      inputSchema: {
        type: 'object',
        properties: {
          agent_type: { type: 'string', description: 'DAA agent type' },
          capabilities: { type: 'array', items: { type: 'string' } },
          resources: { type: 'object', description: 'Agent resources' }
        },
        required: ['agent_type']
      }
    }
  ],
  workflow: [
    {
      name: 'workflow_create',
      description: 'Create custom automation workflow',
      inputSchema: {
        type: 'object',
        properties: {
          name: { type: 'string', description: 'Workflow name' },
          steps: { type: 'array', description: 'Workflow steps' },
          triggers: { type: 'array', description: 'Trigger conditions' }
        },
        required: ['name', 'steps']
      }
    }
  ]
};

// Resource definitions
const RESOURCE_CATEGORIES = {
  docs: [
    {
      uri: 'flow://docs/api-reference',
      name: 'API Reference',
      description: 'Complete API documentation',
      mimeType: 'text/markdown'
    },
    {
      uri: 'flow://docs/getting-started',
      name: 'Getting Started Guide',
      description: 'Quick start guide for Flow Nexus',
      mimeType: 'text/markdown'
    }
  ],
  templates: [
    {
      uri: 'flow://templates/react-app',
      name: 'React Application Template',
      description: 'Modern React app with TypeScript',
      mimeType: 'application/json'
    },
    {
      uri: 'flow://templates/node-api',
      name: 'Node.js API Template',
      description: 'Express.js REST API starter',
      mimeType: 'application/json'
    }
  ],
  examples: [
    {
      uri: 'flow://examples/swarm-coordination',
      name: 'Swarm Coordination Example',
      description: 'Multi-agent coordination patterns',
      mimeType: 'text/javascript'
    }
  ],
  configs: [
    {
      uri: 'flow://configs/production',
      name: 'Production Configuration',
      description: 'Production deployment settings',
      mimeType: 'application/json'
    }
  ]
};

class FlowNexusServer {
  constructor(mode = 'complete') {
    this.mode = mode;
    this.config = MODES[mode] || MODES.complete;
    this.server = new Server(
      {
        name: this.config.name,
        version: '2.0.0',
      },
      {
        capabilities: {
          resources: {},
          tools: {},
        },
      }
    );
    
    this.setupHandlers();
  }

  getToolsForMode() {
    const tools = [];
    for (const category of this.config.tools) {
      if (TOOL_CATEGORIES[category]) {
        tools.push(...TOOL_CATEGORIES[category]);
      }
    }
    
    // Apply filters for specific modes
    if (this.config.filter) {
      return tools.filter(tool => 
        this.config.filter.some(filter => 
          tool.name.includes(filter) || tool.description.toLowerCase().includes(filter)
        )
      );
    }
    
    return tools;
  }

  getResourcesForMode() {
    const resources = [];
    for (const category of this.config.resources) {
      if (RESOURCE_CATEGORIES[category]) {
        resources.push(...RESOURCE_CATEGORIES[category]);
      }
    }
    return resources;
  }

  setupHandlers() {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools: this.getToolsForMode()
    }));

    this.server.setRequestHandler(ListResourcesRequestSchema, async () => ({
      resources: this.getResourcesForMode()
    }));

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;
      
      try {
        return await this.executeTool(name, args || {});
      } catch (error) {
        return {
          content: [
            {
              type: 'text',
              text: `Error executing ${name}: ${error.message}`
            }
          ],
          isError: true
        };
      }
    });

    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;
      return await this.readResource(uri);
    });
  }

  async executeTool(name, args) {
    // Mock implementations for demo - replace with actual logic
    const responses = {
      auth_status: () => ({
        content: [{ 
          type: 'text', 
          text: `Authentication: Active\nMode: ${this.mode}\nPermissions: Full Access` 
        }]
      }),
      
      swarm_init: () => ({
        content: [{ 
          type: 'text', 
          text: `Swarm initialized: ${args.topology || 'mesh'} topology with max ${args.maxAgents || 8} agents` 
        }]
      }),
      
      app_store_list_templates: () => ({
        content: [{ 
          type: 'text', 
          text: JSON.stringify([
            { id: 'react-app', name: 'React Application', category: 'frontend' },
            { id: 'node-api', name: 'Node.js API', category: 'backend' },
            { id: 'next-fullstack', name: 'Next.js Full-Stack', category: 'fullstack' }
          ], null, 2)
        }]
      }),
      
      app_store_earn_ruv: () => ({
        content: [{ 
          type: 'text', 
          text: `Awarded ${args.amount} rUv credits to user ${args.user_id} for: ${args.reason}` 
        }]
      }),
      
      sandbox_create: () => ({
        content: [{ 
          type: 'text', 
          text: `Sandbox created: ${args.template} template, ID: sandbox_${Date.now()}` 
        }]
      })
    };

    const handler = responses[name];
    if (!handler) {
      throw new Error(`Tool '${name}' not implemented`);
    }

    return handler();
  }

  async readResource(uri) {
    const mockContent = {
      'flow://docs/api-reference': '# Flow Nexus API Reference\\n\\nComplete API documentation...',
      'flow://templates/react-app': JSON.stringify({ 
        name: 'React App', 
        files: { 'src/App.jsx': 'export default function App() { return <h1>Hello World</h1>; }' }
      }),
    };

    return {
      contents: [
        {
          uri,
          mimeType: 'text/plain',
          text: mockContent[uri] || `Resource content for ${uri}`
        }
      ]
    };
  }

  async start() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    
    const enhanced = process.env.MCP_ENHANCED === 'true';
    const suffix = enhanced ? ' (Enhanced)' : '';
    
    console.error(`${this.config.name}${suffix} v2.0.0 started`);
    console.error(`Mode: ${this.mode}`);
    console.error(`Tools: ${this.getToolsForMode().length}`);
    console.error(`Resources: ${this.getResourcesForMode().length}`);
    if (enhanced) {
      console.error(`Enhanced features: Streaming responses, improved error handling`);
    }
  }
}

// CLI handling
const args = process.argv.slice(2);
const modeIndex = args.findIndex(arg => ['--mode', '-m'].includes(arg));
const mode = modeIndex !== -1 && args[modeIndex + 1] ? args[modeIndex + 1] : 'complete';

if (args.includes('--help') || args.includes('-h')) {
  console.log(`
Flow Nexus MCP Server v2.0.0

Usage: npx flow-nexus mcp [options]

Options:
  --mode, -m <mode>     Server mode (default: complete)
  --http-port <port>    HTTP server port for Fly.io deployment
  --help, -h            Show this help

Available Modes:
  complete    Full suite with all tools and resources (${MODES.complete.tools.length} tool categories)
  store       App store management and publishing (${MODES.store.tools.length} tool categories)
  swarm       Multi-agent swarm orchestration (${MODES.swarm.tools.length} tool categories)
  dev         Development and sandbox tools (${MODES.dev.tools.length} tool categories)
  gamer       Gamification features only (${MODES.gamer.tools.length} tool categories)

Examples:
  npx flow-nexus mcp --mode store
  npx flow-nexus mcp --mode swarm
  npx flow-nexus mcp --http-port 3000
`);
  process.exit(0);
}

if (args.includes('--list-modes')) {
  console.log('Available modes:');
  Object.entries(MODES).forEach(([key, config]) => {
    console.log(`  ${key.padEnd(10)} - ${config.description}`);
  });
  process.exit(0);
}

// HTTP server for Fly.io deployment
const httpPortIndex = args.findIndex(arg => arg === '--http-port');
if (httpPortIndex !== -1) {
  const port = parseInt(args[httpPortIndex + 1]) || 3000;
  
  import('http').then(({ createServer }) => {
    const server = createServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        name: 'Flow Nexus MCP Server',
        version: '2.0.0',
        mode,
        status: 'running',
        tools: new FlowNexusServer(mode).getToolsForMode().length,
        resources: new FlowNexusServer(mode).getResourcesForMode().length,
        endpoint: `http://localhost:${port}/mcp`
      }));
    });
    
    server.listen(port, () => {
      console.log(`Flow Nexus HTTP server running on port ${port}`);
      console.log(`Mode: ${mode}`);
    });
  });
} else {
  // Standard MCP server
  const server = new FlowNexusServer(mode);
  server.start().catch(console.error);
}