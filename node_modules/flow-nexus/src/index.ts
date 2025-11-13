import { DAAWasmMCPServer, ServerConfig } from './server';
import * as dotenv from 'dotenv';

// Load environment variables
dotenv.config();

// Validate required environment variables
const requiredEnvVars = [
  'SUPABASE_URL',
  'SUPABASE_SERVICE_KEY',
  'JWT_SECRET'
];

for (const envVar of requiredEnvVars) {
  if (!process.env[envVar]) {
    console.error(`❌ Missing required environment variable: ${envVar}`);
    process.exit(1);
  }
}

// Create server configuration
const config: ServerConfig = {
  host: process.env.MCP_HOST || 'localhost',
  port: parseInt(process.env.MCP_PORT || '3001'),
  
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

// Create and start server
const server = new DAAWasmMCPServer(config);

// Handle graceful shutdown
async function gracefulShutdown(signal: string) {
  console.log(`\n🛑 Received ${signal}, shutting down gracefully...`);
  
  try {
    await server.stop();
    console.log('✅ Server shutdown completed');
    process.exit(0);
  } catch (error) {
    console.error('❌ Error during shutdown:', error);
    process.exit(1);
  }
}

// Set up signal handlers
process.on('SIGTERM', () => gracefulShutdown('SIGTERM'));
process.on('SIGINT', () => gracefulShutdown('SIGINT'));

// Handle uncaught exceptions
process.on('uncaughtException', (error) => {
  console.error('❌ Uncaught Exception:', error);
  gracefulShutdown('uncaughtException');
});

process.on('unhandledRejection', (reason, promise) => {
  console.error('❌ Unhandled Rejection at:', promise, 'reason:', reason);
  gracefulShutdown('unhandledRejection');
});

// Start the server
async function start() {
  try {
    console.log('🚀 Starting DAA WASM MCP Server...');
    console.log('Configuration:');
    console.log(`  Host: ${config.host}`);
    console.log(`  Port: ${config.port}`);
    console.log(`  WASM Memory: ${config.wasm.memoryLimit}`);
    console.log(`  SIMD Enabled: ${config.wasm.simdEnabled}`);
    console.log(`  Thread Pool: ${config.wasm.threadPoolSize}`);
    console.log(`  Redis: ${config.redis ? 'Enabled' : 'Disabled (using memory store)'}`);
    console.log(`  CORS Origins: ${config.cors.origins.join(', ')}`);
    console.log('');
    
    await server.start();
    
    console.log('');
    console.log('🎉 DAA WASM MCP Server is running!');
    console.log('');
    console.log('Available endpoints:');
    console.log(`  Health: http://${config.host}:${config.port}/health`);
    console.log(`  Metrics: http://${config.host}:${config.port}/metrics`);
    console.log(`  MCP: mcp://${config.host}:${config.port}`);
    console.log('');
    console.log('Available DAA tools:');
    console.log('  - daa_agent_spawn');
    console.log('  - daa_agent_execute');
    console.log('  - daa_agent_train');
    console.log('  - daa_assess_quality');
    console.log('  - daa_analyze_pricing');
    console.log('  - daa_security_scan');
    console.log('  - daa_generate_recommendations');
    console.log('  - daa_agent_metrics');
    console.log('  - daa_agent_list');
    console.log('  - daa_agent_terminate');
    console.log('');
    
    // Set up server event handlers
    server.on('agentSpawned', (data) => {
      console.log(`✨ Agent spawned: ${data.agentId} (${data.type})`);
    });
    
    server.on('taskCompleted', (data) => {
      console.log(`✅ Task completed: ${data.functionName} on ${data.agentId} (${data.executionTime}ms)`);
    });
    
    server.on('taskError', (data) => {
      console.error(`❌ Task error: ${data.functionName} on ${data.agentId} - ${data.error}`);
    });
    
    server.on('applicationProcessed', (data) => {
      console.log(`📝 Application processed: ${data.applicationId}`);
    });
    
    server.on('marketDataUpdated', (data) => {
      console.log(`💰 Market data updated: ${data.transactionId}`);
    });
    
    server.on('recommendationUpdated', (data) => {
      console.log(`🎯 Recommendation updated: ${data.installationId}`);
    });
    
    server.on('submissionAnalyzed', (data) => {
      console.log(`🔍 Submission analyzed: ${data.submissionId}`);
    });
    
  } catch (error) {
    console.error('❌ Failed to start server:', error);
    process.exit(1);
  }
}

// Export for use as a module
export { DAAWasmMCPServer, ServerConfig, server };

// Start server if this file is run directly
if (require.main === module) {
  start();
}