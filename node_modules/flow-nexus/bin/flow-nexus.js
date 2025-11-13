#!/usr/bin/env node

/**
 * Flow Nexus CLI
 * Advanced MCP server with E2B, Claude-Flow, and Supabase integration
 * Created by ruv (ruv@ruv.net) - https://flow-nexus.ruv.io
 */

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import fs from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const serverPath = join(__dirname, '..', 'src', 'index.js');

// Load version from package.json
const packageJson = JSON.parse(fs.readFileSync(join(__dirname, '..', 'package.json'), 'utf8'));
const VERSION = packageJson.version;

// Parse command line arguments
const args = process.argv.slice(2);
const command = args[0];

// CLI commands that should use the interactive CLI
const cliCommands = [
  'init', 'swarm', 'challenge', 'sandbox', 'credits', 'deploy', 'auth',
  'template', 'store', 'leaderboard', 'storage', 'workflow', 'monitor',
  'profile', 'achievements', 'seraphina', 'chat', 'check', 'system', 'e2e'
];

// Special handling for auth login before MCP
if (command === 'auth' && args[1] === 'login') {
  // Auth login should always use CLI, not start MCP
  // This allows: npx flow-nexus auth login --email user --password pass
  // to work before starting MCP server
}

// Check if this is an MCP server command (should run server, not CLI)
// MCP server runs when:
// - Just 'mcp' with no args or with server-specific args
// - 'mcp start' (compatibility with Claude Desktop)  
// - Has --debug, --mode flags
// - Is test/status command
// CLI runs when it's specific MCP CLI commands like setup, stop, tools
const mcpCliCommands = ['setup', 'stop', 'tools'];
const isMcpCli = command === 'mcp' && args[1] && mcpCliCommands.includes(args[1]);

// Handle MCP server commands first (before CLI)
if (command === 'mcp' && !isMcpCli) {
  // Handle 'mcp start' command (for Claude Desktop compatibility)
  let mcpArgs = args.slice(1); // Get all args after 'mcp'
  
  if (mcpArgs[0] === 'start') {
    // Remove 'start' from mcpArgs for processing
    mcpArgs.shift();
  }
  
  // Check for debug flag
  const debugMode = mcpArgs.includes('--debug') || process.env.DEBUG_MCP === '1';
  if (debugMode) {
    // Remove --debug from args if present
    const debugIndex = mcpArgs.indexOf('--debug');
    if (debugIndex !== -1) {
      mcpArgs.splice(debugIndex, 1);
    }
  }
  
  // Check if it's a test/status command
  if (mcpArgs[0] === 'test' || mcpArgs[0] === 'status') {
    console.log('Testing MCP server...');
    
    // Windows debug info
    if (process.platform === 'win32') {
      console.log('[DEBUG] Running on Windows');
      console.log('[DEBUG] Node version:', process.version);
      console.log('[DEBUG] Server path:', serverPath);
      console.log('[DEBUG] Server exists:', fs.existsSync(serverPath));
    }
    
    // Proper MCP initialization sequence
    const initMessage = JSON.stringify({
      jsonrpc: "2.0",
      method: "initialize",
      params: {
        protocolVersion: "1.0.0",
        capabilities: {
          roots: {}
        },
        clientInfo: {
          name: "test-client",
          version: "1.0.0"
        }
      },
      id: 1
    }) + '\n';
    
    // Windows needs different stdio handling
    const isWindows = process.platform === 'win32';
    const testProcess = spawn('node', [serverPath, '--mode', 'complete'], {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, MCP_MODE: 'stdio', DOTENV_SILENT: 'true' },
      // Windows-specific options
      detached: false,
      windowsHide: false,
      shell: false
    });
    
    let responseBuffer = '';
    let errorBuffer = '';
    let responseReceived = false;
    
    testProcess.on('spawn', () => {
      if (process.platform === 'win32') {
        console.log('[DEBUG] Process spawned successfully');
      }
    });
    
    testProcess.on('error', (err) => {
      console.error('[ERROR] Failed to spawn process:', err.message);
      process.exit(1);
    });
    
    testProcess.stdout.on('data', (data) => {
      responseBuffer += data.toString();
      if (process.platform === 'win32') {
        console.log('[DEBUG] Received data:', data.toString().substring(0, 100));
      }
      
      // Check for JSON-RPC response
      const lines = responseBuffer.split('\n');
      for (const line of lines) {
        if (line.trim() && line.includes('"jsonrpc"')) {
          responseReceived = true;
          console.log('✅ MCP server is working correctly');
          
          // If testing a specific tool, wait for that response
          if (mcpArgs[1]) {
            try {
              const parsed = JSON.parse(line);
              if (parsed.id === 2) {
                console.log('Tool response:', JSON.stringify(parsed, null, 2));
                testProcess.kill();
                process.exit(0);
              }
            } catch (e) {
              // Continue
            }
          } else {
            console.log('Response:', line.trim());
            testProcess.kill();
            process.exit(0);
          }
        }
      }
    });
    
    testProcess.stderr.on('data', (data) => {
      errorBuffer += data.toString();
      if (process.platform === 'win32') {
        console.log('[DEBUG] stderr:', data.toString());
      }
    });
    
    // Send initialization after a small delay
    setTimeout(() => {
      if (process.platform === 'win32') {
        console.log('[DEBUG] Sending init message...');
      }
      testProcess.stdin.write(initMessage);
      
      // If testing a specific tool, send that request too
      if (mcpArgs[1]) {
        const toolRequest = {
          jsonrpc: "2.0",
          method: "tools/call",
          params: {
            name: mcpArgs[1],
            arguments: {}
          },
          id: 2
        };
        setTimeout(() => {
          if (process.platform === 'win32') {
            console.log('[DEBUG] Sending tool request for:', mcpArgs[1]);
          }
          testProcess.stdin.write(JSON.stringify(toolRequest) + '\n');
        }, 200);
      }
    }, 100);
    
    // Timeout with more detailed error
    setTimeout(() => {
      if (!responseReceived) {
        console.error('❌ MCP server did not respond');
        if (errorBuffer) {
          console.error('Error output:', errorBuffer);
        }
        if (responseBuffer) {
          console.error('Partial response:', responseBuffer.substring(0, 500));
        }
        
        // On Windows, suggest alternative test
        if (process.platform === 'win32') {
          console.log('\n💡 Windows Tip: Try running the MCP server directly:');
          console.log('   npx flow-nexus@latest mcp start');
          console.log('Then test with Claude Desktop or another MCP client.\n');
        }
      }
      testProcess.kill();
      process.exit(responseReceived ? 0 : 1);
    }, 5000);
  } else if (process.platform === 'win32') {
    // WINDOWS FIX: Run server directly without parent-child architecture
    // The parent-child stdio piping doesn't work properly on Windows
    const mode = mcpArgs.includes('--mode') ? 
      mcpArgs[mcpArgs.indexOf('--mode') + 1] : 'complete';
    
    // Show mode info (auth status will be shown by the server itself)
    if (process.stderr.isTTY) {
      process.stderr.write(`Flow Nexus MCP server running in ${mode} mode\n`);
      process.stderr.write(`Waiting for Claude Desktop connection...\n`);
      if (debugMode) {
        process.stderr.write(`[DEBUG] Running directly on Windows\n`);
      }
    }
    
    // Set environment and run server directly
    process.env.MCP_MODE = 'stdio';
    process.env.DOTENV_SILENT = 'true';
    if (debugMode) {
      process.env.DEBUG_MCP = '1';
    }
    
    // Import and run the server directly (convert to file:// URL for Windows)
    const serverUrl = process.platform === 'win32' 
      ? new URL(`file:///${serverPath.replace(/\\/g, '/')}`).href
      : serverPath;
    
    import(serverUrl).then(module => {
      const FlowNexusServer = module.FlowNexusServer || module.default;
      const server = new FlowNexusServer(mode);
      server.start().catch(error => {
        console.error('Failed to start server:', error);
        process.exit(1);
      });
    }).catch(error => {
      console.error('Failed to load server:', error);
      process.exit(1);
    });
  } else {
  
  // Parse MCP mode from --mode flag
  let mode = 'complete';
  const modeIndex = mcpArgs.indexOf('--mode');
  if (modeIndex !== -1 && mcpArgs[modeIndex + 1]) {
    mode = mcpArgs[modeIndex + 1];
  }
  
  // Show message to stderr so it doesn't interfere with stdio
  if (process.stderr.isTTY) {
    process.stderr.write(`Flow Nexus MCP server running in ${mode} mode\n`);
    process.stderr.write(`Waiting for Claude Desktop connection...\n`);
    process.stderr.write(`Press Ctrl+C to stop\n`);
    if (debugMode) {
      process.stderr.write(`[DEBUG] Debug mode enabled\n`);
      if (process.platform === 'win32') {
        process.stderr.write(`[WIN32 DEBUG] Windows-specific debugging active\n`);
      }
    }
  }
  
  // Pass debug flag to child process
  const env = { ...process.env, MCP_MODE: 'stdio', DOTENV_SILENT: 'true' };
  if (debugMode) {
    env.DEBUG_MCP = '1';
    if (process.stderr.isTTY) {
      process.stderr.write(`[DEBUG] Setting DEBUG_MCP=1 for child process\n`);
    }
  }
  
  // Run as MCP server with stdio transport
  // On Windows, we need to use explicit pipe handling to keep the process alive
  const stdio = process.platform === 'win32' ? ['pipe', 'pipe', 'pipe'] : 'inherit';
  
  const mcp = spawn('node', [serverPath, '--mode', mode], {
    stdio,
    env,
    // Windows-specific: detached false to ensure proper signal handling
    detached: false,
    // Windows-specific: ensure the process stays alive
    windowsHide: false
    // IMPORTANT: Don't use shell:true on Windows as it breaks stdio piping!
  });
  
  // On Windows, pipe stdio to keep process alive
  if (process.platform === 'win32') {
    // Windows fix: Handle initialize in parent process, pass others to child
    let initHandled = false;
    let childReady = false;
    
    // Wait for child to be ready
    mcp.on('spawn', () => {
      childReady = true;
      if (debugMode) {
        process.stderr.write(`[WIN32 DEBUG] Child process spawned and ready\n`);
      }
    });
    
    // Also consider child ready after a short delay
    setTimeout(() => { childReady = true; }, 100);
    
    let buffer = '';
    process.stdin.on('data', (data) => {
      buffer += data.toString();
      
      // Process complete messages (ending with newline)
      let lines = buffer.split('\n');
      buffer = lines.pop() || ''; // Keep incomplete line in buffer
      
      for (const line of lines) {
        if (!line.trim()) continue;
        
        // Handle initialize in parent
        if (!initHandled && line.includes('"method":"initialize"')) {
          try {
            const msg = JSON.parse(line);
            if (msg.method === 'initialize') {
              initHandled = true;
              const response = {
                jsonrpc: '2.0',
                id: msg.id,
                result: {
                  protocolVersion: '2025-06-18',
                  capabilities: {
                    resources: {},
                    tools: {}
                  },
                  serverInfo: {
                    name: 'Flow Nexus Complete',
                    version: '2.0.0'
                  }
                }
              };
              process.stdout.write(JSON.stringify(response) + '\n');
              if (debugMode) {
                process.stderr.write(`[WIN32 DEBUG] Parent handled initialize\n`);
              }
              continue; // Don't pass initialize to child
            }
          } catch (e) {
            // Not JSON, pass through
          }
        }
        
        // Pass all other messages to child
        if (childReady && mcp.stdin && mcp.stdin.writable) {
          mcp.stdin.write(line + '\n');
          if (debugMode && line.includes('"method"')) {
            const methodMatch = line.match(/"method":"([^"]+)"/);
            const method = methodMatch ? methodMatch[1] : 'unknown';
            process.stderr.write(`[WIN32 DEBUG] Passed to child: ${method}\n`);
          }
        } else if (!childReady) {
          // Queue message for when child is ready
          if (debugMode) {
            process.stderr.write(`[WIN32 DEBUG] Child not ready, will retry message\n`);
          }
          // Re-add to buffer to process later
          buffer = line + '\n' + buffer;
        }
      }
    });
    
    // Pipe child outputs back
    mcp.stdout.pipe(process.stdout);
    mcp.stderr.pipe(process.stderr);
    
    // Keep stdin open
    process.stdin.resume();
    process.stdin.setEncoding('utf8');
    
    // Prevent stdin from closing
    mcp.stdin.on('error', (err) => {
      if (debugMode && err.code !== 'EPIPE') {
        process.stderr.write(`[WIN32 DEBUG] stdin error: ${err.message}\n`);
      }
    });
    
    // Handle stdin end
    process.stdin.on('end', () => {
      if (debugMode) {
        process.stderr.write(`[WIN32 DEBUG] Parent stdin ended\n`);
      }
      mcp.stdin.end();
    });
    
    if (debugMode) {
      process.stderr.write(`[WIN32 DEBUG] Parent process piping stdio\n`);
      process.stderr.write(`[WIN32 DEBUG] Child process PID: ${mcp.pid}\n`);
    }
  }
  
  mcp.on('error', (error) => {
    console.error('Failed to start MCP server:', error);
    process.exit(1);
  });
  
  mcp.on('exit', (code) => {
    if (debugMode && process.platform === 'win32') {
      process.stderr.write(`[WIN32 DEBUG] Child process exited with code: ${code}\n`);
    }
    process.exit(code || 0);
  });
  
  // Handle graceful shutdown
  process.on('SIGINT', () => {
    if (debugMode && process.platform === 'win32') {
      process.stderr.write(`[WIN32 DEBUG] SIGINT received in parent\n`);
    }
    mcp.kill('SIGTERM');
  });
  
  process.on('SIGTERM', () => {
    if (debugMode && process.platform === 'win32') {
      process.stderr.write(`[WIN32 DEBUG] SIGTERM received in parent\n`);
    }
    mcp.kill('SIGTERM');
  });
  }
} else if (cliCommands.includes(command) || isMcpCli) {
  // Run CLI mode (cli.js) for CLI commands and MCP CLI subcommands
  const cliPath = join(__dirname, '..', 'cli.js');
  const cli = spawn('node', [cliPath, ...args], {
    stdio: 'inherit',
    env: {
      ...process.env,
      DOTENV_SILENT: 'true'
    }
  });
  
  cli.on('error', (err) => {
    console.error('Failed to start CLI:', err);
    process.exit(1);
  });
  
  cli.on('exit', (code) => {
    process.exit(code || 0);
  });
} else if (args[0] === 'help' || args[0] === '--help' || args[0] === '-h') {
  console.log(`
╔══════════════════════════════════════════════════════════════╗
║                    Flow Nexus CLI v${VERSION}                     ║
║     Advanced MCP Server with Multi-Agent Orchestration       ║
╚══════════════════════════════════════════════════════════════╝

Created by rUv - https://flow-nexus.ruv.io

GETTING STARTED:
  New Users:    flow-nexus auth register -e pilot@ruv.io -p your-password
  Existing:     flow-nexus auth login -e pilot@ruv.io -p your-password
  Local Only:   flow-nexus auth init

USAGE:
  flow-nexus <command> [options]
  fnx <command> [options]  (shorthand)

CLI COMMANDS:
  init               🎯 Initialize new Flow Nexus project
  auth <action>      🔐 Authentication (register, login, status, logout, init)
  swarm <action>     🤖 AI swarms (create, list, destroy, scale)
  challenge <action> 🏆 Challenges (list, start, submit, status, leaderboard)
  sandbox <action>   📦 Cloud sandboxes (create, list, exec, stop, delete, logs)
  credits <action>   💎 rUv credits (balance, history, earn, transfer, leaderboard)
  profile <action>   👤 Profile management (view, edit, password, settings, privacy)
  seraphina|chat     👑 Chat with Queen Seraphina AI
  storage <action>   💾 File storage (upload, list)
  leaderboard        🏆 View rankings
  achievements       🏅 View achievements & badges
  check|system       ✅ System check and validation
  e2e                🧪 Run end-to-end tests

MCP COMMANDS:
  mcp <action>       🔌 MCP server (start, stop, status, tools, setup, test)
  mcp --debug        🐛 Run MCP server with Windows debugging enabled
  start <mode>       🚀 Start server in mode (complete, swarm, store, dev, gamer, enterprise)
  
UTILITY COMMANDS:
  version            Show version information
  help               Show this help message
  test               Run test suite

MODES (for 'start' and 'mcp'):
  complete           All 50+ tools enabled (default)
  swarm              Swarm orchestration focused
  store              App store and gamification
  dev                Development tools
  gamer              Gaming and achievements
  enterprise         Enterprise features

OPTIONS:
  --http-port <port>       Enable HTTP server on port
  --tools <list>           Comma-separated tool list
  --no-auth               Disable authentication
  --realtime              Enable real-time features
  --enhanced              Enable enhanced features
  --max-agents <n>        Maximum agents (enterprise)
  --supabase-url <url>    Override Supabase URL
  --supabase-key <key>    Override Supabase key

EXAMPLES:
  # Authentication & Setup
  flow-nexus auth register -e pilot@ruv.io -p pass123  # Register new user
  flow-nexus auth login -e pilot@ruv.io -p pass123     # Login existing user
  flow-nexus auth init                                 # Local auth only
  
  # CLI Commands  
  flow-nexus swarm create mesh          # Create mesh swarm
  flow-nexus challenge list             # View available challenges
  flow-nexus sandbox create python      # Create Python sandbox
  flow-nexus credits balance            # Check rUv balance
  flow-nexus profile view               # View your profile
  flow-nexus profile edit -n "Name"     # Update profile name
  flow-nexus profile settings           # Manage settings
  flow-nexus chat "How do I use this?"  # Ask Queen Seraphina
  flow-nexus seraphina --tools          # Chat with tool execution
  flow-nexus system                     # System check (alias for check)
  
  # MCP Server (for Claude Desktop)
  flow-nexus mcp                        # Run as MCP server (all tools)
  flow-nexus mcp --mode swarm           # MCP with swarm tools only
  flow-nexus mcp --mode store           # MCP with store tools only
  
  # Start Server Modes
  flow-nexus start swarm                # Start in swarm mode
  flow-nexus start enterprise --max-agents 200
  
  # Shortcuts (fnx)
  fnx swarm create                      # Using fnx alias
  fnx chat "What challenges exist?"     # Quick chat with Seraphina
  fnx credits earn                      # Check earning opportunities

NPX USAGE:
  npx flow-nexus init
  npx flow-nexus mcp
  npx fnx swarm create

For help on specific commands:
  flow-nexus <command> --help

For more information: https://github.com/ruvnet/flow-nexus
Version: ${VERSION}
`);
  process.exit(0);
} else if (args[0] === 'version' || args[0] === '--version' || args[0] === '-v') {
  console.log(`Flow Nexus v${VERSION}`);
  console.log(`Created by rUv`);
  console.log(`https://flow-nexus.ruv.io`);
  process.exit(0);
} else if (args[0] === 'test') {
  const test = spawn('node', [join(__dirname, '..', 'test-100-percent.js')], {
    stdio: 'inherit'
  });
  test.on('exit', (code) => process.exit(code || 0));
} else if (args[0] === 'e2e') {
  const e2e = spawn('node', [join(__dirname, '..', 'e2e-test-summary.js')], {
    stdio: 'inherit'
  });
  e2e.on('exit', (code) => process.exit(code || 0));
} else if (args[0] === 'start') {
  // Start server with mode
  const mode = args[1] || 'complete';
  const startArgs = [serverPath, mode, ...args.slice(2)];
  const server = spawn('node', startArgs, {
    stdio: 'inherit',
    env: { ...process.env, DOTENV_SILENT: 'true' }
  });
  server.on('exit', (code) => process.exit(code || 0));
} else {
  // Default: run interactive CLI when no arguments provided
  if (args.length === 0) {
    const cliPath = join(__dirname, '..', 'cli.js');
    const cli = spawn('node', [cliPath], {
      stdio: 'inherit',
      env: {
        ...process.env,
        DOTENV_SILENT: 'true'
      }
    });
    
    cli.on('error', (err) => {
      console.error('Failed to start CLI:', err);
      process.exit(1);
    });
    
    cli.on('exit', (code) => {
      process.exit(code || 0);
    });
  } else {
    // Run MCP server with provided arguments
    const mcp = spawn('node', [serverPath, ...args], {
      stdio: 'inherit',
      env: { ...process.env, DOTENV_SILENT: 'true' }
    });
    
    mcp.on('error', (error) => {
      console.error('Failed to start server:', error);
      process.exit(1);
    });
    
    mcp.on('exit', (code) => {
      process.exit(code || 0);
    });
  }
}