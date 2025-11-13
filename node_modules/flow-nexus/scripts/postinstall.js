#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.join(__dirname, '..');
const ENV_PATH = path.join(ROOT_DIR, '.env');

function checkUserSetup() {
  try {
    if (fs.existsSync(ENV_PATH)) {
      const envContent = fs.readFileSync(ENV_PATH, 'utf8');
      return envContent.includes('FLOW_NEXUS_USER_EMAIL');
    }
  } catch {
    // Silent fail
  }
  return false;
}

const isConfigured = checkUserSetup();

if (isConfigured) {
  console.log(`
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║     ✅ Flow Nexus MCP Server Installed Successfully! ✅     ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

Your user account is already configured.

📚 Quick Start:
  • Run the server:     npx flow-nexus
  • Use specific mode:  npx flow-nexus --mode store
  • Get help:          npx flow-nexus --help

🎯 Available Modes:
  • complete - All features enabled
  • swarm    - Multi-agent swarm coordination
  • store    - App store and gamification
  • dev      - Development sandbox environment
  • gamer    - Gaming and achievements

🔧 For Claude Desktop integration:
  {
    "mcpServers": {
      "flow-nexus": {
        "command": "npx",
        "args": ["flow-nexus", "mcp"]
      }
    }
  }

Enjoy building with Flow Nexus! 🚀
`);
} else {
  console.log(`
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║        🎮 Flow Nexus MCP Server - Setup Required 🎮         ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

⚠️  USER AUTHENTICATION REQUIRED

To use Flow Nexus, you must create your personal account.
This ensures your data is isolated and secure.

🚀 Run the setup command:
   npm run init-user

This will:
  ✓ Create your Flow Nexus account
  ✓ Generate a secure API key
  ✓ Configure your .env file
  ✓ Grant you 2560 rUv credits to start

📋 After setup, you can:
  • Run the server:    npx flow-nexus
  • Test your setup:   npm run test-user
  • View all modes:    npx flow-nexus --help

🔒 Security Notes:
  • Each user's data is completely isolated
  • Your API key is stored locally in .env
  • Never share your API key with others

📖 Documentation: https://flow-nexus.com/docs
🐛 Issues: https://github.com/ruvnet/flow-nexus/issues

Created by ruv (ruv@ruv.net)
`);
  
  // Exit with code 0 to not break npm install
  process.exit(0);
}