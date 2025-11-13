#!/usr/bin/env node

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import fs from 'fs/promises';
import { registration } from '../services/registration.js';
import { security } from '../middleware/security.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// CLI for Flow Nexus MCP Server
class FlowNexusCLI {
  constructor() {
    this.commands = {
      'start': this.startServer,
      'register': this.registerUser,
      'login': this.loginUser,
      'upgrade': this.upgradeTier,
      'status': this.checkStatus,
      'help': this.showHelp,
      'version': this.showVersion,
      'init': this.initConfig,
      'test': this.testConnection
    };
  }

  async run(args) {
    const command = args[2] || 'help';
    const handler = this.commands[command];

    if (!handler) {
      console.error(`Unknown command: ${command}`);
      this.showHelp();
      process.exit(1);
    }

    try {
      await handler.call(this, args.slice(3));
    } catch (error) {
      console.error(`Error: ${error.message}`);
      process.exit(1);
    }
  }

  // Start MCP server
  async startServer(args) {
    const mode = args[0] || 'complete';
    const port = args[1] || 3000;

    console.log(`Starting Flow Nexus MCP Server in ${mode} mode...`);
    
    const serverPath = join(__dirname, '..', 'index.js');
    const env = {
      ...process.env,
      FLOW_NEXUS_MODE: mode,
      PORT: port
    };

    const server = spawn('node', [serverPath], {
      env,
      stdio: 'inherit'
    });

    server.on('error', (err) => {
      console.error('Failed to start server:', err);
      process.exit(1);
    });

    server.on('exit', (code) => {
      process.exit(code);
    });
  }

  // Register new user
  async registerUser(args) {
    const email = args[0];
    const password = args[1];
    
    if (!email || !password) {
      console.error('Usage: flow-nexus register <email> <password>');
      process.exit(1);
    }

    console.log('Registering new user...');
    
    const result = await registration.registerUser(email, password, {
      username: email.split('@')[0]
    });

    if (result.success) {
      console.log('\n✅ Registration successful!');
      console.log(`User ID: ${result.userId}`);
      console.log(`API Key: ${result.apiKey}`);
      
      if (result.verificationToken) {
        console.log(`Verification Token: ${result.verificationToken}`);
      }
      
      console.log('\nSave your API key securely. You will need it to authenticate.');
      
      // Save to config file
      await this.saveConfig({
        email,
        userId: result.userId,
        apiKey: result.apiKey
      });
      
    } else {
      console.error(`Registration failed: ${result.error}`);
      process.exit(1);
    }
  }

  // Login user
  async loginUser(args) {
    const email = args[0];
    const password = args[1];
    
    if (!email || !password) {
      console.error('Usage: flow-nexus login <email> <password>');
      process.exit(1);
    }

    console.log('Logging in...');
    
    const result = await registration.loginUser(email, password);

    if (result.success) {
      console.log('\n✅ Login successful!');
      console.log(`Session ID: ${result.sessionId}`);
      console.log(`API Key: ${result.apiKey}`);
      console.log(`Tier: ${result.profile.tier}`);
      console.log(`rUv Balance: ${result.profile.ruvBalance}`);
      
      // Save to config file
      await this.saveConfig({
        email: result.profile.email,
        userId: result.userId,
        apiKey: result.apiKey,
        sessionId: result.sessionId,
        token: result.token
      });
      
    } else {
      console.error(`Login failed: ${result.error}`);
      process.exit(1);
    }
  }

  // Upgrade tier
  async upgradeTier(args) {
    const tier = args[0];
    
    if (!tier || !['pro', 'enterprise'].includes(tier)) {
      console.error('Usage: flow-nexus upgrade <pro|enterprise>');
      process.exit(1);
    }

    const config = await this.loadConfig();
    if (!config || !config.userId) {
      console.error('Please login first: flow-nexus login <email> <password>');
      process.exit(1);
    }

    console.log(`Upgrading to ${tier} tier...`);
    
    const result = await registration.upgradeTier(config.userId, tier);

    if (result.success) {
      console.log(`\n✅ Upgrade successful!`);
      console.log(`New API Key: ${result.newApiKey}`);
      console.log(`Bonus Credits: ${result.bonusCredits}`);
      
      // Update config
      config.apiKey = result.newApiKey;
      config.tier = tier;
      await this.saveConfig(config);
      
    } else {
      console.error(`Upgrade failed: ${result.error}`);
      process.exit(1);
    }
  }

  // Check status
  async checkStatus() {
    const config = await this.loadConfig();
    
    if (!config) {
      console.log('Not logged in. Use: flow-nexus login <email> <password>');
      return;
    }

    console.log('\nFlow Nexus MCP Status');
    console.log('=' .repeat(50));
    console.log(`Email: ${config.email}`);
    console.log(`User ID: ${config.userId}`);
    console.log(`API Key: ${config.apiKey?.substring(0, 20)}...`);
    console.log(`Session: ${config.sessionId ? 'Active' : 'Inactive'}`);
    console.log(`Tier: ${config.tier || 'free'}`);
    
    if (config.userId) {
      const stats = await registration.getUserStats(config.userId);
      console.log('\nResource Usage:');
      console.log(`Swarms: ${stats.swarms}`);
      console.log(`Agents: ${stats.agents}`);
      console.log(`Tasks: ${stats.tasks}`);
      console.log(`Sandboxes: ${stats.sandboxes}`);
    }
  }

  // Initialize configuration
  async initConfig() {
    console.log('Initializing Flow Nexus configuration...');
    
    const configPath = await this.getConfigPath();
    const configDir = dirname(configPath);
    
    await fs.mkdir(configDir, { recursive: true });
    
    const defaultConfig = {
      email: '',
      userId: '',
      apiKey: '',
      sessionId: '',
      token: '',
      tier: 'free'
    };
    
    await fs.writeFile(configPath, JSON.stringify(defaultConfig, null, 2));
    
    console.log(`✅ Configuration file created at: ${configPath}`);
    console.log('Use "flow-nexus register" to create an account');
  }

  // Test connection
  async testConnection() {
    console.log('Testing connection to Flow Nexus services...');
    
    try {
      // Test Supabase connection
      const { db } = await import('../services/supabase.js');
      const connected = await db.testConnection();
      
      if (connected) {
        console.log('✅ Database connection: OK');
      } else {
        console.log('❌ Database connection: Failed');
      }
      
      // Test API key generation
      const testKey = security.generateApiKey('test-user', 'free');
      if (testKey && testKey.startsWith('fnx_')) {
        console.log('✅ Security module: OK');
      } else {
        console.log('❌ Security module: Failed');
      }
      
      console.log('\n✅ All systems operational');
      
    } catch (error) {
      console.error('❌ Connection test failed:', error.message);
      process.exit(1);
    }
  }

  // Show help
  showHelp() {
    console.log(`
Flow Nexus MCP Server CLI

Usage: flow-nexus <command> [options]

Commands:
  start [mode] [port]     Start MCP server (modes: complete, store, swarm, dev, gamer)
  register <email> <pwd>  Register new account
  login <email> <pwd>     Login to existing account
  upgrade <tier>          Upgrade account tier (pro, enterprise)
  status                  Check account status
  init                    Initialize configuration
  test                    Test connection to services
  version                 Show version information
  help                    Show this help message

Examples:
  flow-nexus start                    # Start server in complete mode
  flow-nexus start swarm 3001          # Start swarm mode on port 3001
  flow-nexus register user@email.com pass123
  flow-nexus login user@email.com pass123
  flow-nexus upgrade pro

Configuration:
  Config file: ~/.flow-nexus/config.json
  Environment: FLOW_NEXUS_API_KEY

Documentation:
  https://docs.flow-nexus.com
  https://github.com/flow-nexus/mcp-server
`);
  }

  // Show version
  async showVersion() {
    const packagePath = join(__dirname, '..', '..', 'package.json');
    const packageJson = JSON.parse(await fs.readFile(packagePath, 'utf8'));
    
    console.log(`Flow Nexus MCP Server v${packageJson.version}`);
    console.log(`Node.js ${process.version}`);
  }

  // Get config file path
  async getConfigPath() {
    const home = process.env.HOME || process.env.USERPROFILE;
    return join(home, '.flow-nexus', 'config.json');
  }

  // Load configuration
  async loadConfig() {
    try {
      const configPath = await this.getConfigPath();
      const data = await fs.readFile(configPath, 'utf8');
      return JSON.parse(data);
    } catch {
      return null;
    }
  }

  // Save configuration
  async saveConfig(config) {
    const configPath = await this.getConfigPath();
    const configDir = dirname(configPath);
    
    await fs.mkdir(configDir, { recursive: true });
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));
  }
}

// Run CLI
const cli = new FlowNexusCLI();
cli.run(process.argv).catch(console.error);