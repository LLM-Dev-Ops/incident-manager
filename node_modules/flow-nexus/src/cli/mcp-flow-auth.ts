#!/usr/bin/env node

/**
 * MCP Flow Authentication CLI
 * Command-line interface for user authentication
 */

import { Command } from 'commander';
import * as readline from 'readline';
import { SecureMCPAuthTool } from '../tools/auth-secure';
import { secureConfig } from '../config/secure-config';
import * as fs from 'fs';
import * as path from 'path';

const program = new Command();
const auth = new SecureMCPAuthTool();

// Helper to get password input
async function getPassword(prompt: string): Promise<string> {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });

  return new Promise((resolve) => {
    rl.question(prompt, (password) => {
      rl.close();
      resolve(password);
    });
  });
}

// Helper to display banner
function displayBanner() {
  console.log(`
╔════════════════════════════════════════════════════════════╗
║                    FLOW NEXUS MCP AUTH                     ║
║                   Secure Authentication CLI                ║
╚════════════════════════════════════════════════════════════╝
`);
}

program
  .name('mcp-flow')
  .description('Flow Nexus MCP Authentication CLI')
  .version('2.0.0');

// Init command
program
  .command('init')
  .description('Initialize MCP authentication for first-time users')
  .action(async () => {
    displayBanner();
    const result = await auth.init();
    console.log(result.message);
  });

// Check auth status
program
  .command('status')
  .description('Check authentication status')
  .action(async () => {
    displayBanner();
    const result = auth.checkAuth();
    
    if (result.authenticated) {
      console.log('✅ ' + result.message);
      
      const session = auth.getSession();
      if (session.user) {
        console.log('\n📊 Account Details:');
        console.log(`  • User ID: ${session.user.id}`);
        console.log(`  • Email: ${session.user.email}`);
        console.log(`  • API Key: ${session.user.apiKey}`);
        console.log(`  • Credits: ${session.user.credits}`);
      }
    } else {
      console.log(result.message);
    }
  });

// Register command
program
  .command('register')
  .description('Register a new account')
  .option('-e, --email <email>', 'Email address')
  .option('-p, --password <password>', 'Password (min 8 characters)')
  .option('-u, --username <username>', 'Username (optional)')
  .action(async (options) => {
    displayBanner();
    
    // Interactive mode if no options provided
    let email = options.email;
    let password = options.password;
    let username = options.username;

    if (!email) {
      const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
      });
      
      email = await new Promise((resolve) => {
        rl.question('📧 Email: ', resolve);
      });
      rl.close();
    }

    if (!password) {
      password = await getPassword('🔑 Password (min 8 chars): ');
    }

    if (!username) {
      const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
      });
      
      username = await new Promise((resolve) => {
        rl.question('👤 Username (optional, press Enter to skip): ', resolve);
      });
      rl.close();
    }

    const result = await auth.register({
      email,
      password,
      username: username || undefined
    });

    if (result.success) {
      console.log(result.message);
    } else {
      console.error('❌ Registration failed:', result.error);
    }
  });

// Login command
program
  .command('login')
  .description('Login to existing account')
  .option('-e, --email <email>', 'Email address')
  .option('-p, --password <password>', 'Password')
  .action(async (options) => {
    displayBanner();
    
    // Interactive mode if no options provided
    let email = options.email;
    let password = options.password;

    if (!email) {
      const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
      });
      
      email = await new Promise((resolve) => {
        rl.question('📧 Email: ', resolve);
      });
      rl.close();
    }

    if (!password) {
      password = await getPassword('🔑 Password: ');
    }

    const result = await auth.login({
      email,
      password
    });

    if (result.success) {
      console.log(result.message);
    } else {
      console.error('❌ Login failed:', result.error);
    }
  });

// Logout command
program
  .command('logout')
  .description('Logout and clear credentials')
  .action(async () => {
    displayBanner();
    
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout
    });
    
    const confirm = await new Promise<string>((resolve) => {
      rl.question('⚠️  Are you sure you want to logout? (y/N): ', resolve);
    });
    rl.close();

    if (confirm.toLowerCase() === 'y') {
      const result = await auth.logout();
      console.log(result.message);
    } else {
      console.log('Logout cancelled.');
    }
  });

// Credits command
program
  .command('credits')
  .description('Check credit balance')
  .action(async () => {
    displayBanner();
    const result = auth.checkCredits();
    
    if (result.success) {
      console.log(`
💰 Credit Balance
─────────────────
Current Balance: ${result.credits} credits
`);
    } else {
      console.error('❌', result.message);
    }
  });

// Template command
program
  .command('template')
  .description('Create .env.user template file')
  .action(() => {
    displayBanner();
    secureConfig.createUserEnvTemplate();
    console.log(`
✅ Template created!

Next steps:
1. Edit .env.user.template with your credentials
2. Rename to .env.user
3. Restart MCP server
`);
  });

// Parse command line arguments
program.parse(process.argv);

// Show help if no command provided
if (!process.argv.slice(2).length) {
  displayBanner();
  console.log(`
Welcome to Flow Nexus MCP Authentication!

Quick Start:
  $ mcp-flow init       - First-time setup
  $ mcp-flow register   - Create new account
  $ mcp-flow login      - Login to existing account
  $ mcp-flow status     - Check authentication status
  $ mcp-flow credits    - Check credit balance
  $ mcp-flow logout     - Logout and clear credentials

For more options, use: mcp-flow --help
`);
}