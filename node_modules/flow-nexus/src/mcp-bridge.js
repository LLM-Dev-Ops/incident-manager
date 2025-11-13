#!/usr/bin/env node

/**
 * MCP Bridge Server - Provides HTTP interface for Flow Nexus MCP Server
 * This creates an HTTP bridge that can communicate with the MCP protocol
 */

import { spawn } from 'child_process';
import http from 'http';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

class MCPBridge {
  constructor(port = 3000, mode = 'complete') {
    this.port = port;
    this.mode = mode;
    this.mcpProcess = null;
    this.server = null;
  }

  startMCPServer() {
    return new Promise((resolve, reject) => {
      const mcpPath = join(__dirname, 'index.js');
      this.mcpProcess = spawn('node', [mcpPath, '--mode', this.mode], {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: {
          ...process.env,
          FLOW_NEXUS_MODE: 'production',
          NODE_ENV: 'production'
        }
      });

      this.mcpProcess.on('error', reject);
      
      // Wait for server to start
      setTimeout(() => {
        if (this.mcpProcess && !this.mcpProcess.killed) {
          resolve(this.mcpProcess);
        } else {
          reject(new Error('MCP server failed to start'));
        }
      }, 1000);
    });
  }

  async sendMCPRequest(request) {
    return new Promise((resolve, reject) => {
      if (!this.mcpProcess) {
        reject(new Error('MCP server not running'));
        return;
      }

      const timeout = setTimeout(() => {
        reject(new Error('MCP request timeout'));
      }, 10000);

      let responseData = '';
      
      const onData = (data) => {
        responseData += data.toString();
        try {
          const response = JSON.parse(responseData);
          clearTimeout(timeout);
          this.mcpProcess.stdout.off('data', onData);
          resolve(response);
        } catch (e) {
          // Still receiving data
        }
      };

      this.mcpProcess.stdout.on('data', onData);
      this.mcpProcess.stdin.write(JSON.stringify(request) + '\n');
    });
  }

  createHTTPServer() {
    this.server = http.createServer(async (req, res) => {
      // CORS headers
      res.setHeader('Access-Control-Allow-Origin', '*');
      res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

      if (req.method === 'OPTIONS') {
        res.writeHead(200);
        res.end();
        return;
      }

      // Health check endpoint
      if (req.url === '/' || req.url === '/health') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          name: 'Flow Nexus MCP Bridge',
          version: '2.0.0',
          mode: this.mode,
          status: 'running',
          mcp_process: this.mcpProcess ? 'active' : 'inactive',
          endpoints: {
            health: '/',
            tools: '/tools',
            resources: '/resources',
            execute: '/execute'
          }
        }));
        return;
      }

      // List tools endpoint
      if (req.url === '/tools' && req.method === 'GET') {
        try {
          const response = await this.sendMCPRequest({
            jsonrpc: '2.0',
            id: Date.now(),
            method: 'tools/list'
          });
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify(response));
        } catch (error) {
          res.writeHead(500, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: error.message }));
        }
        return;
      }

      // List resources endpoint
      if (req.url === '/resources' && req.method === 'GET') {
        try {
          const response = await this.sendMCPRequest({
            jsonrpc: '2.0',
            id: Date.now(),
            method: 'resources/list'
          });
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify(response));
        } catch (error) {
          res.writeHead(500, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: error.message }));
        }
        return;
      }

      // Execute tool endpoint
      if (req.url === '/execute' && req.method === 'POST') {
        let body = '';
        req.on('data', chunk => body += chunk);
        req.on('end', async () => {
          try {
            const { tool, arguments: args } = JSON.parse(body);
            const response = await this.sendMCPRequest({
              jsonrpc: '2.0',
              id: Date.now(),
              method: 'tools/call',
              params: {
                name: tool,
                arguments: args || {}
              }
            });
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify(response));
          } catch (error) {
            res.writeHead(500, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: error.message }));
          }
        });
        return;
      }

      // 404 for other routes
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Not found' }));
    });
  }

  async start() {
    try {
      console.error(`🌉 Starting MCP Bridge Server`);
      console.error(`Mode: ${this.mode}`);
      console.error(`Port: ${this.port}`);
      console.error(`---`);

      // Start MCP server
      await this.startMCPServer();
      console.error(`✅ MCP server started`);

      // Create HTTP server
      this.createHTTPServer();
      
      // Start listening
      this.server.listen(this.port, () => {
        console.error(`🚀 HTTP Bridge running on http://localhost:${this.port}`);
        console.error(`📡 MCP Bridge ready for requests`);
      });

    } catch (error) {
      console.error(`❌ Bridge startup failed: ${error.message}`);
      process.exit(1);
    }
  }

  stop() {
    if (this.mcpProcess) {
      this.mcpProcess.kill();
    }
    if (this.server) {
      this.server.close();
    }
  }
}

// CLI handling
const args = process.argv.slice(2);
const portIndex = args.findIndex(arg => arg === '--port');
const port = portIndex !== -1 ? parseInt(args[portIndex + 1]) : 3000;

const modeIndex = args.findIndex(arg => arg === '--mode');
const mode = modeIndex !== -1 ? args[modeIndex + 1] : 'complete';

if (args.includes('--help')) {
  console.log(`
Flow Nexus MCP Bridge v2.0.0

Usage: node mcp-bridge.js [options]

Options:
  --port <port>     HTTP server port (default: 3000)
  --mode <mode>     MCP server mode (default: complete)
  --help            Show this help

Available modes: complete, store, swarm, dev, gamer

Examples:
  node mcp-bridge.js --port 3000 --mode complete
  node mcp-bridge.js --port 8080 --mode store

Endpoints:
  GET  /            Health check
  GET  /tools       List available tools
  GET  /resources   List available resources
  POST /execute     Execute tool (JSON: {"tool": "name", "arguments": {}})
`);
  process.exit(0);
}

// Start the bridge
const bridge = new MCPBridge(port, mode);

// Graceful shutdown
process.on('SIGINT', () => {
  console.error(`\n🛑 Shutting down MCP Bridge...`);
  bridge.stop();
  process.exit(0);
});

bridge.start().catch(console.error);