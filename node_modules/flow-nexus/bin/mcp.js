#!/usr/bin/env node

/**
 * MCP Server Direct Entry Point
 * For Model Context Protocol stdio transport
 */

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const serverPath = join(__dirname, '..', 'src', 'index.js');

// Run MCP server in stdio mode
const mcp = spawn('node', [serverPath, ...process.argv.slice(2)], {
  stdio: 'inherit',
  env: { ...process.env, MCP_MODE: 'stdio' }
});

mcp.on('error', (error) => {
  console.error('MCP server error:', error);
  process.exit(1);
});

mcp.on('exit', (code) => {
  process.exit(code || 0);
});