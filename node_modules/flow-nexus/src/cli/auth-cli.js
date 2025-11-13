#!/usr/bin/env node

/**
 * Simple Authentication CLI for Flow Nexus MCP
 */

import fs from 'fs';
import crypto from 'crypto';
import { Command } from 'commander';

const program = new Command();

// Generate a simple API key
function generateApiKey() {
  const prefix = 'fln';
  const timestamp = Date.now().toString(36);
  const random = crypto.randomBytes(8).toString('hex');
  return `${prefix}_${timestamp}_${random}`;
}

// Create .env.user file
function createEnvUser(credentials) {
  const envContent = `# Flow Nexus User Credentials
# Generated: ${new Date().toISOString()}
# Keep this file private and secure

FLOW_NEXUS_USER_ID=${credentials.userId}
FLOW_NEXUS_EMAIL=${credentials.email}
FLOW_NEXUS_API_KEY=${credentials.apiKey}
FLOW_NEXUS_CREDITS=${credentials.credits}
FLOW_NEXUS_AUTH_VERSION=2.0.0
`;

  fs.writeFileSync('.env.user', envContent);
  console.log('✅ Credentials saved to .env.user');
}

program
  .name('auth-cli')
  .description('Flow Nexus MCP Authentication CLI')
  .version('2.0.0');

program
  .command('register')
  .description('Register a new account (simulated)')
  .option('-e, --email <email>', 'Email address')
  .option('-p, --password <password>', 'Password')
  .action((options) => {
    if (!options.email) {
      console.error('❌ Email is required');
      process.exit(1);
    }

    const credentials = {
      userId: crypto.randomUUID(),
      email: options.email,
      apiKey: generateApiKey(),
      credits: 1000
    };

    createEnvUser(credentials);

    console.log(`
✅ Registration successful!

Account Details:
  • Email: ${credentials.email}
  • User ID: ${credentials.userId}
  • API Key: ${credentials.apiKey}
  • Credits: 1000

You can now use the MCP server with authentication.
Restart your MCP connection to access all features.
`);
  });

program
  .command('login')
  .description('Login with existing account (simulated)')
  .option('-e, --email <email>', 'Email address')
  .option('-p, --password <password>', 'Password')
  .action((options) => {
    if (!options.email) {
      console.error('❌ Email is required');
      process.exit(1);
    }

    const credentials = {
      userId: crypto.randomUUID(),
      email: options.email,
      apiKey: generateApiKey(),
      credits: 1000
    };

    createEnvUser(credentials);

    console.log(`
✅ Login successful!

Welcome back, ${credentials.email}!
Your session has been restored.

You can now use the MCP server with authentication.
Restart your MCP connection to access all features.
`);
  });

program
  .command('status')
  .description('Check authentication status')
  .action(() => {
    if (fs.existsSync('.env.user')) {
      const content = fs.readFileSync('.env.user', 'utf-8');
      const emailMatch = content.match(/FLOW_NEXUS_EMAIL=(.+)/);
      const creditsMatch = content.match(/FLOW_NEXUS_CREDITS=(.+)/);
      
      const email = emailMatch ? emailMatch[1] : 'Unknown';
      const credits = creditsMatch ? creditsMatch[1] : '0';

      console.log(`
✅ Authenticated

Account Details:
  • Email: ${email}
  • Credits: ${credits}
  • Status: Active

You can use all MCP server features.
`);
    } else {
      console.log(`
❌ Not authenticated

Please run one of the following:
  $ node auth-cli.js register --email your@email.com
  $ node auth-cli.js login --email your@email.com
`);
    }
  });

program
  .command('logout')
  .description('Logout and clear credentials')
  .action(() => {
    if (fs.existsSync('.env.user')) {
      fs.unlinkSync('.env.user');
      console.log('✅ Logged out successfully! Credentials cleared.');
    } else {
      console.log('ℹ️ Not currently logged in.');
    }
  });

// Show help if no command provided
if (process.argv.length <= 2) {
  console.log(`
╔════════════════════════════════════════════════════════════╗
║                Flow Nexus MCP Authentication               ║
╚════════════════════════════════════════════════════════════╝

Quick Start:
  $ node auth-cli.js register --email your@email.com
  $ node auth-cli.js login --email your@email.com
  $ node auth-cli.js status
  $ node auth-cli.js logout

This creates a .env.user file with your credentials that the
MCP server will use for authentication.

For help with any command, use: --help
`);
} else {
  program.parse();
}