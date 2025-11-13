#!/usr/bin/env node

/**
 * Flow Nexus MCP Streamable HTTP Server
 * Implements MCP specification 2025-03-26 Streamable HTTP transport
 * 
 * Features:
 * - Single endpoint for POST/GET requests
 * - SSE streaming for real-time responses  
 * - Event ID tracking for stream resumption
 * - Backward compatibility with SSE transport
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import http from 'http';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Import mode configurations from main index
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

const TOOL_CATEGORIES = {
  auth: [
    { name: 'auth_status', description: 'Check authentication status and permissions' },
    { name: 'auth_init', description: 'Initialize secure authentication' }
  ],
  swarm: [
    { name: 'swarm_init', description: 'Initialize multi-agent swarm with specified topology' },
    { name: 'agent_spawn', description: 'Create specialized AI agent in swarm' },
    { name: 'task_orchestrate', description: 'Orchestrate complex task across swarm agents' }
  ],
  'app-store': [
    { name: 'app_store_list_templates', description: 'List available application templates' },
    { name: 'app_store_publish_app', description: 'Publish new application to store' },
    { name: 'app_store_complete_challenge', description: 'Mark challenge as completed for user' },
    { name: 'app_store_earn_ruv', description: 'Award rUv credits to user' }
  ],
  sandbox: [
    { name: 'sandbox_create', description: 'Create new code execution sandbox' },
    { name: 'sandbox_execute', description: 'Execute code in sandbox environment' }
  ],
  neural: [
    { name: 'neural_train', description: 'Train neural patterns with WASM acceleration' }
  ],
  github: [
    { name: 'github_repo_analyze', description: 'Analyze GitHub repository' }
  ],
  daa: [
    { name: 'daa_agent_create', description: 'Create decentralized autonomous agent' }
  ],
  workflow: [
    { name: 'workflow_create', description: 'Create custom automation workflow' }
  ]
};

class MCPStreamableServer {
  constructor(mode = 'complete', port = 3000) {
    this.mode = mode;
    this.port = port;
    this.config = MODES[mode] || MODES.complete;
    this.eventIdCounter = 0;
    this.sessions = new Map(); // Track SSE sessions
    
    // Initialize MCP server for internal processing
    this.mcpServer = new Server(
      {
        name: this.config.name + ' (Streamable)',
        version: '2.0.0',
      },
      {
        capabilities: {
          resources: {},
          tools: {},
        },
      }
    );
    
    this.setupMCPHandlers();
  }

  setupMCPHandlers() {
    this.mcpServer.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools: this.getToolsForMode()
    }));

    this.mcpServer.setRequestHandler(ListResourcesRequestSchema, async () => ({
      resources: this.getResourcesForMode()
    }));

    this.mcpServer.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;
      return await this.executeTool(name, args || {});
    });

    this.mcpServer.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;
      return await this.readResource(uri);
    });
  }

  getToolsForMode() {
    const tools = [];
    for (const category of this.config.tools) {
      if (TOOL_CATEGORIES[category]) {
        tools.push(...TOOL_CATEGORIES[category]);
      }
    }
    
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
    // Simplified resource list for demo
    return [
      {
        uri: 'flow://docs/api-reference',
        name: 'API Reference',
        description: 'Complete API documentation',
        mimeType: 'text/markdown'
      }
    ];
  }

  async executeTool(name, args) {
    // Mock implementations
    const responses = {
      auth_status: () => ({
        content: [{ 
          type: 'text', 
          text: `Authentication: Active\\nMode: ${this.mode}\\nPermissions: Full Access\\nStreamable: Enabled` 
        }]
      }),
      
      swarm_init: () => ({
        content: [{ 
          type: 'text', 
          text: `Swarm initialized via Streamable HTTP: ${args.topology || 'mesh'} topology with max ${args.maxAgents || 8} agents` 
        }]
      }),
      
      app_store_list_templates: () => ({
        content: [{ 
          type: 'text', 
          text: JSON.stringify([
            { id: 'react-stream', name: 'React Streaming App', category: 'frontend' },
            { id: 'node-sse', name: 'Node.js SSE API', category: 'backend' },
            { id: 'mcp-server', name: 'MCP Server Template', category: 'protocol' }
          ], null, 2)
        }]
      })
    };

    const handler = responses[name];
    if (!handler) {
      throw new Error(`Tool '${name}' not implemented in streamable mode`);
    }

    return handler();
  }

  async readResource(uri) {
    return {
      contents: [
        {
          uri,
          mimeType: 'text/plain',
          text: `Streamable resource content for ${uri}\\n\\nMode: ${this.mode}\\nProtocol: MCP Streamable HTTP\\nSpecification: 2025-03-26`
        }
      ]
    };
  }

  generateEventId() {
    return `event-${this.eventIdCounter++}-${Date.now()}`;
  }

  sendSSEMessage(res, data, eventId = null) {
    const id = eventId || this.generateEventId();
    res.write(`id: ${id}\\n`);
    res.write(`data: ${JSON.stringify(data)}\\n\\n`);
    return id;
  }

  async processJSONRPCRequest(request) {
    // Process JSON-RPC request through MCP server
    try {
      if (request.method === 'tools/list') {
        return {
          jsonrpc: '2.0',
          id: request.id,
          result: { tools: this.getToolsForMode() }
        };
      }
      
      if (request.method === 'resources/list') {
        return {
          jsonrpc: '2.0', 
          id: request.id,
          result: { resources: this.getResourcesForMode() }
        };
      }
      
      if (request.method === 'tools/call') {
        const result = await this.executeTool(
          request.params.name, 
          request.params.arguments || {}
        );
        return {
          jsonrpc: '2.0',
          id: request.id,
          result
        };
      }
      
      return {
        jsonrpc: '2.0',
        id: request.id,
        error: {
          code: -32601,
          message: 'Method not found',
          data: { method: request.method }
        }
      };
      
    } catch (error) {
      return {
        jsonrpc: '2.0',
        id: request.id,
        error: {
          code: -32603,
          message: 'Internal error',
          data: { error: error.message }
        }
      };
    }
  }

  createHTTPServer() {
    return http.createServer(async (req, res) => {
      // CORS headers
      res.setHeader('Access-Control-Allow-Origin', '*');
      res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Last-Event-ID');

      if (req.method === 'OPTIONS') {
        res.writeHead(200);
        res.end();
        return;
      }

      // Health check endpoint
      if (req.url === '/health' || req.url === '/') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          name: this.config.name,
          version: '2.0.0',
          mode: this.mode,
          transport: 'Streamable HTTP',
          specification: '2025-03-26',
          endpoints: {
            mcp: '/mcp',
            health: '/health'
          },
          tools: this.getToolsForMode().length,
          resources: this.getResourcesForMode().length
        }));
        return;
      }

      // MCP Streamable HTTP endpoint
      if (req.url === '/mcp') {
        
        // GET request for SSE stream resumption
        if (req.method === 'GET') {
          const lastEventId = req.headers['last-event-id'];
          
          res.writeHead(200, {
            'Content-Type': 'text/event-stream',
            'Cache-Control': 'no-cache',
            'Connection': 'keep-alive'
          });

          // Send initial connection event
          this.sendSSEMessage(res, {
            type: 'connection',
            mode: this.mode,
            lastEventId: lastEventId || null,
            timestamp: new Date().toISOString()
          });

          // Keep connection alive
          const keepAlive = setInterval(() => {
            res.write(': keepalive\\n\\n');
          }, 30000);

          req.on('close', () => {
            clearInterval(keepAlive);
          });

          return;
        }

        // POST request for JSON-RPC messages
        if (req.method === 'POST') {
          let body = '';
          req.on('data', chunk => body += chunk);
          req.on('end', async () => {
            try {
              const acceptHeader = req.headers.accept || '';
              const supportsSSE = acceptHeader.includes('text/event-stream');
              
              // Parse JSON-RPC request(s)
              const requests = JSON.parse(body);
              const requestArray = Array.isArray(requests) ? requests : [requests];
              
              if (supportsSSE) {
                // Return SSE stream
                res.writeHead(200, {
                  'Content-Type': 'text/event-stream',
                  'Cache-Control': 'no-cache',
                  'Connection': 'keep-alive'
                });

                // Process each request and stream responses
                for (const request of requestArray) {
                  const response = await this.processJSONRPCRequest(request);
                  this.sendSSEMessage(res, response);
                }

                // Send end-of-stream marker
                this.sendSSEMessage(res, { type: 'stream-end' });
                res.end();
                
              } else {
                // Return single JSON response
                res.writeHead(200, { 'Content-Type': 'application/json' });
                
                if (requestArray.length === 1) {
                  const response = await this.processJSONRPCRequest(requestArray[0]);
                  res.end(JSON.stringify(response));
                } else {
                  const responses = await Promise.all(
                    requestArray.map(req => this.processJSONRPCRequest(req))
                  );
                  res.end(JSON.stringify(responses));
                }
              }
              
            } catch (error) {
              res.writeHead(400, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                jsonrpc: '2.0',
                id: null,
                error: {
                  code: -32700,
                  message: 'Parse error',
                  data: { error: error.message }
                }
              }));
            }
          });
          return;
        }
      }

      // 404 for other routes
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Not found' }));
    });
  }

  async start() {
    const server = this.createHTTPServer();
    
    server.listen(this.port, () => {
      console.error(`🌊 ${this.config.name} (Streamable) v2.0.0 started`);
      console.error(`Mode: ${this.mode}`);
      console.error(`Port: ${this.port}`);
      console.error(`Protocol: MCP Streamable HTTP (2025-03-26)`);
      console.error(`Endpoint: http://localhost:${this.port}/mcp`);
      console.error(`Tools: ${this.getToolsForMode().length}`);
      console.error(`Resources: ${this.getResourcesForMode().length}`);
      console.error(`---`);
      console.error(`🚀 Ready for MCP Streamable HTTP requests`);
    });

    return server;
  }
}

// CLI handling
const args = process.argv.slice(2);
const modeIndex = args.findIndex(arg => ['--mode', '-m'].includes(arg));
const mode = modeIndex !== -1 && args[modeIndex + 1] ? args[modeIndex + 1] : 'complete';

const portIndex = args.findIndex(arg => ['--port', '-p'].includes(arg));
const port = portIndex !== -1 ? parseInt(args[portIndex + 1]) : 3000;

if (args.includes('--help') || args.includes('-h')) {
  console.log(`
Flow Nexus MCP Streamable HTTP Server v2.0.0
Implements MCP specification 2025-03-26

Usage: node mcp-streamable.js [options]

Options:
  --mode, -m <mode>     Server mode (default: complete)
  --port, -p <port>     HTTP server port (default: 3000)
  --help, -h            Show this help

Available Modes:
  complete    Full suite with all tools and resources
  store       App store management and publishing
  swarm       Multi-agent swarm orchestration  
  dev         Development and sandbox tools
  gamer       Gamification features only

Examples:
  node mcp-streamable.js --mode complete --port 3000
  node mcp-streamable.js --mode store --port 8080

MCP Endpoints:
  POST /mcp             Send JSON-RPC requests
  GET  /mcp             SSE stream (with Last-Event-ID support)
  GET  /health          Health check and server info
`);
  process.exit(0);
}

// Start the streamable server
const server = new MCPStreamableServer(mode, port);
server.start().catch(console.error);