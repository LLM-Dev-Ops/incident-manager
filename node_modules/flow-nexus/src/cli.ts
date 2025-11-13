#!/usr/bin/env node

import { Command } from 'commander';
import { DAAWasmMCPServer, ServerConfig } from './server';
import { AuthMiddleware } from './middleware/auth';
import * as fs from 'fs';
import * as path from 'path';
import * as dotenv from 'dotenv';

const program = new Command();

program
  .name('daa-wasm-mcp')
  .description('DAA WASM MCP Server for Flow Cloud App Store')
  .version('1.0.0');

// Start server command
program
  .command('start')
  .description('Start the DAA WASM MCP server')
  .option('-p, --port <port>', 'Port to listen on', '3001')
  .option('-h, --host <host>', 'Host to bind to', 'localhost')
  .option('--config <file>', 'Configuration file path')
  .option('--env <file>', 'Environment file path', '.env')
  .action(async (options) => {
    try {
      // Load environment file
      if (options.env && fs.existsSync(options.env)) {
        dotenv.config({ path: options.env });
      } else {
        dotenv.config();
      }

      let config: ServerConfig;

      // Load configuration from file if specified
      if (options.config) {
        if (!fs.existsSync(options.config)) {
          console.error(`❌ Configuration file not found: ${options.config}`);
          process.exit(1);
        }
        
        const configContent = fs.readFileSync(options.config, 'utf-8');
        config = JSON.parse(configContent);
      } else {
        // Use environment variables
        config = {
          host: options.host || process.env.MCP_HOST || 'localhost',
          port: parseInt(options.port || process.env.MCP_PORT || '3001'),
          
          supabase: {
            url: process.env.SUPABASE_URL!,
            serviceKey: process.env.SUPABASE_SERVICE_KEY!,
            anonKey: process.env.SUPABASE_ANON_KEY
          },
          
          wasm: {
            wasmPath: process.env.WASM_PATH || './dist/daa-agents.wasm',
            memoryLimit: process.env.WASM_MEMORY_LIMIT || '256MB',
            simdEnabled: process.env.WASM_SIMD_ENABLED !== 'false',
            threadPoolSize: parseInt(process.env.WASM_THREAD_POOL_SIZE || '4')
          },
          
          auth: {
            jwtSecret: process.env.JWT_SECRET!
          },
          
          redis: process.env.REDIS_URL ? {
            url: process.env.REDIS_URL
          } : undefined,
          
          cors: {
            origins: process.env.ALLOWED_ORIGINS?.split(',') || ['http://localhost:5173']
          }
        };
      }

      // Validate required configuration
      if (!config.supabase.url || !config.supabase.serviceKey || !config.auth.jwtSecret) {
        console.error('❌ Missing required configuration. Please check your environment variables or config file.');
        console.error('Required: SUPABASE_URL, SUPABASE_SERVICE_KEY, JWT_SECRET');
        process.exit(1);
      }

      const server = new DAAWasmMCPServer(config);
      await server.start();
      
    } catch (error) {
      console.error('❌ Failed to start server:', error);
      process.exit(1);
    }
  });

// Generate config command
program
  .command('init')
  .description('Initialize configuration files')
  .option('--force', 'Overwrite existing files')
  .action((options) => {
    const envFile = '.env';
    const configFile = 'daa-mcp-config.json';

    // Generate .env file
    if (!fs.existsSync(envFile) || options.force) {
      const envContent = `# DAA WASM MCP Server Configuration
MCP_HOST=localhost
MCP_PORT=3001
NODE_ENV=development

# Supabase Configuration
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_KEY=your-service-key
SUPABASE_ANON_KEY=your-anon-key

# Authentication
JWT_SECRET=your-jwt-secret-key

# Redis Configuration (optional)
# REDIS_URL=redis://localhost:6379

# WASM Configuration
WASM_PATH=./dist/daa-agents.wasm
WASM_MEMORY_LIMIT=256MB
WASM_THREAD_POOL_SIZE=4
WASM_SIMD_ENABLED=true

# Monitoring
PROMETHEUS_PORT=9090
LOG_LEVEL=info

# CORS
ALLOWED_ORIGINS=http://localhost:5173,http://localhost:3000

# Agent Configuration
MAX_CURATOR_AGENTS=3
MAX_PRICING_AGENTS=2
MAX_SECURITY_AGENTS=4
MAX_RECOMMENDATION_AGENTS=2

# Performance Settings
REQUEST_TIMEOUT=30000
MAX_CONCURRENT_REQUESTS=100
RATE_LIMIT_REQUESTS_PER_HOUR=1000
`;

      fs.writeFileSync(envFile, envContent);
      console.log(`✅ Created ${envFile}`);
    } else {
      console.log(`⚠️  ${envFile} already exists. Use --force to overwrite.`);
    }

    // Generate config file
    if (!fs.existsSync(configFile) || options.force) {
      const config = {
        host: "localhost",
        port: 3001,
        supabase: {
          url: "https://your-project.supabase.co",
          serviceKey: "your-service-key"
        },
        wasm: {
          wasmPath: "./dist/daa-agents.wasm",
          memoryLimit: "256MB",
          simdEnabled: true,
          threadPoolSize: 4
        },
        auth: {
          jwtSecret: "your-jwt-secret"
        },
        cors: {
          origins: ["http://localhost:5173"]
        }
      };

      fs.writeFileSync(configFile, JSON.stringify(config, null, 2));
      console.log(`✅ Created ${configFile}`);
    } else {
      console.log(`⚠️  ${configFile} already exists. Use --force to overwrite.`);
    }

    console.log('');
    console.log('🎉 Configuration files generated!');
    console.log('');
    console.log('Next steps:');
    console.log('1. Update the configuration files with your actual values');
    console.log('2. Run: npx daa-wasm-mcp start');
  });

// Health check command
program
  .command('health')
  .description('Check server health')
  .option('--url <url>', 'Server URL', 'http://localhost:3001')
  .action(async (options) => {
    try {
      const response = await fetch(`${options.url}/health`);
      const health = await response.json() as any;
      
      console.log('🏥 Server Health Status:');
      console.log(JSON.stringify(health, null, 2));
      
      if (health.status === 'healthy') {
        console.log('✅ Server is healthy');
        process.exit(0);
      } else {
        console.log('❌ Server is unhealthy');
        process.exit(1);
      }
    } catch (error) {
      console.error('❌ Failed to check health:', error);
      process.exit(1);
    }
  });

// Metrics command
program
  .command('metrics')
  .description('Get server metrics')
  .option('--url <url>', 'Server URL', 'http://localhost:3001')
  .option('--format <format>', 'Output format (json|prometheus)', 'json')
  .action(async (options) => {
    try {
      const endpoint = options.format === 'prometheus' ? 'metrics' : 'metrics';
      const response = await fetch(`${options.url}/${endpoint}`);
      
      if (options.format === 'prometheus') {
        const metrics = await response.text();
        console.log(metrics);
      } else {
        const metrics = await response.json();
        console.log(JSON.stringify(metrics, null, 2));
      }
    } catch (error) {
      console.error('❌ Failed to get metrics:', error);
      process.exit(1);
    }
  });

// Token generation command (for testing)
program
  .command('generate-token')
  .description('Generate a JWT token for testing')
  .requiredOption('--user-id <id>', 'User ID')
  .option('--username <name>', 'Username', 'test-user')
  .option('--level <level>', 'Developer level', '3')
  .option('--secret <secret>', 'JWT secret (or set JWT_SECRET env var)')
  .option('--expires <time>', 'Expiration time', '24h')
  .action((options) => {
    dotenv.config();
    
    const secret = options.secret || process.env.JWT_SECRET;
    if (!secret) {
      console.error('❌ JWT secret is required. Use --secret or set JWT_SECRET environment variable.');
      process.exit(1);
    }

    const user = {
      id: options.userId,
      username: options.username,
      developer_level: parseInt(options.level),
      permissions: []
    };

    const token = AuthMiddleware.createToken(user, secret, options.expires);
    
    console.log('🎫 Generated JWT Token:');
    console.log(token);
    console.log('');
    console.log('User Info:');
    console.log(`  ID: ${user.id}`);
    console.log(`  Username: ${user.username}`);
    console.log(`  Level: ${user.developer_level}`);
    console.log(`  Expires: ${options.expires}`);
  });

// Version command
program
  .command('version')
  .description('Show version information')
  .action(() => {
    const packageJson = JSON.parse(fs.readFileSync(path.join(__dirname, '../package.json'), 'utf-8'));
    console.log(`DAA WASM MCP Server v${packageJson.version}`);
    console.log(`Node.js ${process.version}`);
    console.log(`Platform: ${process.platform}`);
  });

// Parse command line arguments
program.parse();

// If no command specified, show help
if (!process.argv.slice(2).length) {
  program.outputHelp();
}