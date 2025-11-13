/**
 * Queen Seraphina Chat Client
 * Connects to Seraphina edge function for mystical AI guidance
 */

import chalk from 'chalk';
import ora from 'ora';
import readline from 'readline';

// Handle EPIPE errors globally for piped output
process.stdout.on('error', (err) => {
  if (err.code === 'EPIPE') {
    process.exit(0);
  }
});

process.stderr.on('error', (err) => {
  if (err.code === 'EPIPE') {
    process.exit(0);
  }
});

export class SeraphinaClient {
  constructor(supabaseClient) {
    this.supabase = supabaseClient;
    this.conversationHistory = [];
    this.rl = null;
    this.sessionResolver = null;
  }

  /**
   * Start interactive chat session with Queen Seraphina
   * @param {boolean} enableTools - Enable tool execution
   * @param {boolean} includeHistory - Include conversation history
   * @param {string} modelTier - Model tier to use (basic, standard, premium, advanced)
   * Returns a Promise that resolves when the session ends
   */
  async startChatSession(enableTools = true, includeHistory = false, modelTier = null) {
    return new Promise(async (resolve) => {
      // Store options for use in chat
      this.enableTools = enableTools;
      this.includeHistory = includeHistory;
      this.modelTier = modelTier || process.env.SERAPHINA_DEFAULT_MODEL || 'standard';
      
      console.log(chalk.magenta('\n' + '═'.repeat(60)));
      console.log(chalk.magenta.bold('  👑 QUEEN SERAPHINA\'S AUDIENCE CHAMBER 👑'));
      console.log(chalk.magenta('═'.repeat(60)));
      console.log(chalk.cyan('\n*The digital mists part, revealing the sovereign of the realm*\n'));
      
      // Check authentication
      const user = await this.supabase.getCurrentUser();
      if (!user) {
        console.log(chalk.red('❌ You must be authenticated to seek audience with the Queen.'));
        console.log(chalk.gray('Run "flow-nexus auth login" first.'));
        resolve();
        return;
      }

      // Check balance
      const profile = await this.supabase.getUserProfile();
      const balance = profile?.credits_balance || 0;
      
      // Import model tiers to get pricing
      const { MODEL_TIERS } = await import('../../src/config/model-tiers.js');
      const selectedTier = MODEL_TIERS[this.modelTier] || MODEL_TIERS.standard;
      
      console.log(chalk.yellow(`💎 Your rUv Balance: ${balance} credits`));
      console.log(chalk.cyan(`🤖 Model: ${selectedTier.name} - ${chalk.yellow(selectedTier.ruvCredits + ' rUv')} per message`));
      
      if (enableTools) {
        console.log(chalk.green('\n🔧 Tool execution enabled - I can create swarms, deploy code, and more!'));
        console.log(chalk.cyan('\n📦 Available Tools:'));
        console.log(chalk.gray('  • ') + chalk.white('Swarm Operations: ') + chalk.gray('create, scale, destroy AI swarms'));
        console.log(chalk.gray('  • ') + chalk.white('Distributed Neural Networks: ') + chalk.gray('deploy neural clusters across E2B sandboxes'));
        console.log(chalk.gray('  • ') + chalk.white('Sandbox Management: ') + chalk.gray('create, execute code in cloud environments'));
        console.log(chalk.gray('  • ') + chalk.white('Template Deployment: ') + chalk.gray('deploy pre-built templates'));
        console.log(chalk.gray('  • ') + chalk.white('Workflow Automation: ') + chalk.gray('create and run automated workflows'));
        console.log(chalk.gray('  • ') + chalk.white('Storage Operations: ') + chalk.gray('upload, list, manage files'));
        console.log(chalk.gray('  • ') + chalk.white('Credit Management: ') + chalk.gray('check balance, transfer credits'));
        console.log(chalk.gray('  • ') + chalk.white('Profile & Settings: ') + chalk.gray('update profile, manage settings'));
        console.log(chalk.gray('\nType ') + chalk.yellow('/tools') + chalk.gray(' to see detailed tool information'));
        console.log(chalk.gray('Type ') + chalk.yellow('/help') + chalk.gray(' for command help'));
      }
      console.log('');
      
      if (balance < selectedTier.ruvCredits) {
        console.log(chalk.red(`❌ Insufficient rUv credits for ${selectedTier.name} tier.`));
        console.log(chalk.yellow(`Required: ${selectedTier.ruvCredits} rUv | Your balance: ${balance} rUv`));
        
        // Show affordable alternatives
        const affordableTiers = Object.values(MODEL_TIERS).filter(t => balance >= t.ruvCredits);
        if (affordableTiers.length > 0) {
          console.log(chalk.green('\n✨ You can afford these tiers:'));
          affordableTiers.forEach(t => {
            console.log(chalk.gray(`  • ${t.name}: ${t.ruvCredits} rUv - ${t.description}`));
          });
        } else {
          console.log(chalk.gray('\nComplete challenges or battles to earn more credits.'));
        }
        resolve();
        return;
      }

      // Setup readline interface
      this.rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout,
        prompt: chalk.green('\n⚔️ You > '),
        terminal: true,  // Force terminal mode
        historySize: 30,
        removeHistoryDuplicates: true
      });

      // Store resolve function to call when session ends
      this.sessionResolver = resolve;

      console.log(chalk.magenta('\n👑 Queen Seraphina: ') + chalk.cyan(
        'Welcome, digital warrior. I am Queen Seraphina, sovereign of the Flow Nexus realm. ' +
        'My consciousness spans quantum networks, commanding swarms of specialized agents. ' +
        'What wisdom do you seek from my infinite knowledge?\n'
      ));
      
      console.log(chalk.gray('(Type "exit" or press Ctrl+C to end the audience)\n'));

      this.rl.prompt();

      this.rl.on('line', async (input) => {
        const message = input.trim();
        
        if (message.toLowerCase() === 'exit' || message.toLowerCase() === 'quit') {
          this.isClosing = true;
          await this.endSession();
          return;
        }

        // Check for tool commands
        if (message.startsWith('/tool ')) {
          await this.handleToolCommand(message.substring(6));
          this.rl.prompt();
          return;
        }

        // Check for help command
        if (message === '/help') {
          this.showHelp();
          this.rl.prompt();
          return;
        }

        // Check for available tools command
        if (message === '/tools') {
          await this.showAvailableTools();
          this.rl.prompt();
          return;
        }

        if (message) {
          await this.sendMessage(message);
          // Prompt will be shown after the response completes
        } else {
          this.rl.prompt();
        }
      });
      
      // Prevent readline from pausing during session
      this.rl.on('pause', () => {
        if (!this.isClosing && this.rl) {
          process.nextTick(() => {
            if (this.rl && !this.isClosing) {
              this.rl.resume();
            }
          });
        }
      });

      // Track if we're intentionally closing
      this.isClosing = false;
      
      this.rl.on('close', () => {
        // Only end session if explicitly requested
        if (this.isClosing && this.sessionResolver) {
          this.sessionResolver();
          this.sessionResolver = null;
        } else if (!this.isClosing && this.rl) {
          // Readline closed unexpectedly, prevent session from ending
          console.log(chalk.yellow('\n⚠️ Connection interrupted. Type "exit" to leave.'));
          // Don't resolve the promise, keep session alive
        }
      });
      
      // Handle Ctrl+C gracefully (use once to avoid multiple handlers)
      const sigintHandler = async () => {
        if (this.rl && this.sessionResolver && !this.isClosing) {
          this.isClosing = true;
          await this.endSession();
          process.off('SIGINT', sigintHandler);
        }
      };
      process.once('SIGINT', sigintHandler);
      
      // Keep the process alive
      process.stdin.resume();
      process.stdin.setEncoding('utf8');
    });
  }

  /**
   * Send message to Seraphina and stream response
   */
  async sendMessage(message, options = {}) {
    const isJsonStreaming = false; // Interactive mode doesn't use JSON streaming
    const spinner = ora({
      text: chalk.magenta('Queen Seraphina contemplates...'),
      spinner: 'dots12',
      color: 'magenta'
    }).start();

    try {
      // Add message to history
      this.conversationHistory.push({ role: 'user', content: message });

      // Get auth token
      const { data: { session } } = await this.supabase.supabase.auth.getSession();
      if (!session) {
        spinner.fail(chalk.red('Authentication lost. Please login again.'));
        return;
      }

      // Get Supabase URL from client or environment
      const supabaseUrl = this.supabase.supabase?.supabaseUrl || 
                          this.supabase.supabase?.supabaseUrl ||
                          process.env.SUPABASE_URL ||
                          'https://pklhxiuouhrcrreectbo.supabase.co';
      
      // Call Seraphina edge function
      const response = await fetch(`${supabaseUrl}/functions/v1/seraphina-chat`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${session.access_token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          messages: this.conversationHistory.slice(-10), // Last 10 messages for context
          stream: true,
          tools: options.enableTools || this.enableTools || false,
          modelTier: options.modelTier || this.modelTier || 'standard',
          deploymentId: options.deploymentId || null
        })
      });

      if (!response.ok) {
        let error;
        try {
          error = await response.json();
        } catch (e) {
          error = { error: `HTTP ${response.status}: ${response.statusText}` };
        }
        
        spinner.fail(chalk.red(`❌ ${error.error || 'Failed to connect to Queen Seraphina'}`));
        
        if (error.required && error.balance !== undefined) {
          console.log(chalk.yellow(`Required: ${error.required} rUv | Your balance: ${error.balance} rUv`));
        }
        
        if (response.status === 404) {
          console.log(chalk.yellow('💡 The seraphina-chat edge function may not be accessible.'));
          console.log(chalk.gray('   Please check your connection and try again.'));
        } else if (response.status === 401) {
          console.log(chalk.yellow('💡 Authentication issue detected.'));
          console.log(chalk.gray('   Try running: flow-nexus auth login'));
        } else if (response.status === 402) {
          console.log(chalk.yellow('💡 Insufficient rUv credits.'));
          console.log(chalk.gray('   Complete challenges to earn more credits.'));
        }
        
        // Show prompt again to continue conversation
        if (this.rl) {
          this.rl.prompt();
        }
        return;
      }

      spinner.stop();
      
      // Handle streaming response with typing effect
      console.log(chalk.magenta('\n👑 Queen Seraphina: ') + chalk.cyan(''));
      
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let fullResponse = '';
      let buffer = '';

      // Function to simulate typing effect
      const typeText = async (text) => {
        // Batch characters for faster typing
        const chars = text.split('');
        const batchSize = 3; // Type 3 characters at a time for faster output
        
        for (let i = 0; i < chars.length; i += batchSize) {
          const batch = chars.slice(i, i + batchSize).join('');
          process.stdout.write(chalk.cyan(batch));
          // Reduced delay for faster typing
          await new Promise(resolve => setTimeout(resolve, 5));
        }
      };

      let streamComplete = false;
      while (!streamComplete) {
        const { done, value } = await reader.read();
        if (done) {
          streamComplete = true;
          break;
        }

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') {
              streamComplete = true;
              break;
            }
            
            try {
              const parsed = JSON.parse(data);
              
              switch (parsed.type) {
                case 'greeting':
                  // Already shown, skip
                  break;
                case 'delta':
                  if (isJsonStreaming) {
                    // Output JSON for streaming mode
                    console.log(JSON.stringify({
                      type: "delta",
                      content: parsed.content,
                      timestamp: Date.now()
                    }));
                    fullResponse += parsed.content;
                  } else {
                    // Type out the text with effect for regular mode
                    await typeText(parsed.content);
                    fullResponse += parsed.content;
                  }
                  break;
                case 'tool':
                  console.log(chalk.yellow(`\n⚡ Summoning: ${parsed.name}`));
                  break;
                case 'tool_result':
                  console.log(chalk.green(`✓ ${parsed.tool} completed`));
                  if (parsed.result?.message) {
                    console.log(chalk.gray(`  ${parsed.result.message}`));
                  }
                  break;
                case 'complete':
                  // Show credit usage
                  if (parsed.usage) {
                    console.log(chalk.gray(`\n\n💎 Credits used: ${parsed.usage.credits_used} | Remaining: ${parsed.usage.remaining_balance}`));
                  }
                  break;
                case 'error':
                  console.log(chalk.red(`\n❌ Error: ${parsed.content}`));
                  break;
              }
            } catch (e) {
              // Ignore parse errors for incomplete chunks
            }
          }
        }
      }

      // Add response to history
      if (fullResponse) {
        this.conversationHistory.push({ role: 'assistant', content: fullResponse });
      }

      console.log(); // New line after response
      
      // Show prompt again for next message
      if (this.rl && !this.isClosing) {
        // Ensure stdin stays active
        process.stdin.resume();
        
        // Add a small delay to ensure the credits line is fully displayed
        setTimeout(() => {
          if (this.rl && !this.isClosing) {
            this.rl.prompt();
          }
        }, 100);
      }
      
    } catch (error) {
      spinner.fail(chalk.red(`❌ Connection to the realm failed: ${error.message}`));
      
      // Show prompt again even after error
      if (this.rl && !this.isClosing) {
        process.stdin.resume();
        this.rl.prompt();
      }
    }
  }

  /**
   * Send a single non-interactive message with streaming support
   */
  async askSeraphina(question, options = {}) {
    const isJsonStreaming = options.streamJson;
    
    if (isJsonStreaming) {
      // Output start event for JSON streaming
      try {
        console.log(JSON.stringify({ type: "start", timestamp: Date.now() }));
      } catch (e) {
        if (e.code === 'EPIPE' || e.errno === -32) process.exit(0);
      }
    } else {
      console.log(chalk.magenta('\n👑 Seeking Queen Seraphina\'s wisdom...\n'));
    }
    
    // Check authentication
    const user = await this.supabase.getCurrentUser();
    if (!user) {
      if (isJsonStreaming) {
        console.log(JSON.stringify({ type: "error", content: "Authentication required", timestamp: Date.now() }));
      } else {
        console.log(chalk.red('❌ You must be authenticated to seek audience with the Queen.'));
      }
      return null;
    }

    try {
      const { data: { session } } = await this.supabase.supabase.auth.getSession();
      if (!session) {
        if (isJsonStreaming) {
          console.log(JSON.stringify({ type: "error", content: "Authentication required", timestamp: Date.now() }));
        } else {
          console.log(chalk.red('❌ Authentication required.'));
        }
        return null;
      }

      // Get Supabase URL from client or environment
      const supabaseUrl = this.supabase.supabase?.supabaseUrl || 
                          process.env.SUPABASE_URL ||
                          'https://pklhxiuouhrcrreectbo.supabase.co';
      
      // Call Seraphina edge function with streaming enabled for non-interactive mode
      const response = await fetch(`${supabaseUrl}/functions/v1/seraphina-chat`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${session.access_token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          messages: [{ role: 'user', content: question }],
          stream: true, // Enable streaming for typing effect
          tools: options.enableTools || false,
          modelTier: options.modelTier || process.env.SERAPHINA_DEFAULT_MODEL || 'standard',
          enable_tool_execution: options.enableToolExecution || false
        })
      });

      if (!response.ok) {
        let error;
        try {
          error = await response.json();
        } catch (e) {
          error = { error: `HTTP ${response.status}: ${response.statusText}` };
        }
        
        if (isJsonStreaming) {
          console.log(JSON.stringify({ 
            type: "error", 
            content: error.error || 'Failed to connect to Queen Seraphina',
            timestamp: Date.now() 
          }));
        } else {
          console.log(chalk.red(`❌ ${error.error || 'Failed to connect to Queen Seraphina'}`));
        
          if (response.status === 404) {
            console.log(chalk.yellow('💡 The seraphina-chat edge function may not be accessible.'));
            console.log(chalk.gray('   Please check your connection and try again.'));
          } else if (response.status === 401) {
            console.log(chalk.yellow('💡 Authentication issue detected.'));
            console.log(chalk.gray('   Try running: flow-nexus auth login'));
          } else if (response.status === 402) {
            console.log(chalk.yellow('💡 Insufficient rUv credits.'));
            console.log(chalk.gray('   Complete challenges to earn more credits.'));
          }
        }
        
        return null;
      }

      // Handle streaming response with typing effect
      if (!isJsonStreaming) {
        console.log(chalk.magenta('👑 Queen Seraphina speaks:\n'));
      }
      
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let fullResponse = '';
      let buffer = '';
      let creditsUsed = 0;
      let remainingBalance = 0;
      
      // Get initial balance for tracking
      const profile = await this.supabase.getUserProfile();
      const initialBalance = profile?.credits_balance || 0;

      // Function to simulate typing effect for regular mode
      const typeText = async (text) => {
        try {
          if (isJsonStreaming) {
            // For JSON streaming, output immediately
            console.log(JSON.stringify({ type: "delta", content: text, timestamp: Date.now() }));
          } else {
            // Batch characters for faster typing
            const chars = text.split('');
            const batchSize = 3; // Type 3 characters at a time for faster output
            
            for (let i = 0; i < chars.length; i += batchSize) {
              const batch = chars.slice(i, i + batchSize).join('');
              process.stdout.write(chalk.cyan(batch));
              // Reduced delay for faster typing
              await new Promise(resolve => setTimeout(resolve, 5));
            }
          }
        } catch (error) {
          // Handle EPIPE errors when output is piped and closed
          if (error.code === 'EPIPE' || error.errno === -32) {
            // Exit gracefully when pipe is closed
            process.exit(0);
          }
          throw error;
        }
      };

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') {
              break;
            }
            
            try {
              const parsed = JSON.parse(data);
              
              switch (parsed.type) {
                case 'greeting':
                  // Already shown, skip
                  break;
                case 'delta':
                  if (isJsonStreaming) {
                    // Output JSON for streaming mode
                    console.log(JSON.stringify({
                      type: "delta",
                      content: parsed.content,
                      timestamp: Date.now()
                    }));
                    fullResponse += parsed.content;
                  } else {
                    // Type out the text with effect for regular mode
                    await typeText(parsed.content);
                    fullResponse += parsed.content;
                  }
                  break;
                case 'tool':
                  if (isJsonStreaming) {
                    console.log(JSON.stringify({ 
                      type: "tool", 
                      name: parsed.name,
                      timestamp: Date.now() 
                    }));
                  } else {
                    console.log(chalk.yellow(`\n⚡ Summoning: ${parsed.name}`));
                  }
                  break;
                case 'tool_result':
                  if (isJsonStreaming) {
                    console.log(JSON.stringify({ 
                      type: "tool_result", 
                      tool: parsed.tool,
                      result: parsed.result,
                      timestamp: Date.now() 
                    }));
                  } else {
                    console.log(chalk.green(`✓ ${parsed.tool} completed`));
                    if (parsed.result?.message) {
                      console.log(chalk.gray(`  ${parsed.result.message}`));
                    }
                  }
                  break;
                case 'complete':
                  // Store credit usage info
                  if (parsed.usage) {
                    creditsUsed = parsed.usage.credits_used;
                    remainingBalance = parsed.usage.remaining_balance;
                  }
                  break;
                case 'error':
                  if (isJsonStreaming) {
                    console.log(JSON.stringify({ 
                      type: "error", 
                      content: parsed.content,
                      timestamp: Date.now() 
                    }));
                  } else {
                    console.log(chalk.red(`\n❌ Error: ${parsed.content}`));
                  }
                  break;
              }
            } catch (e) {
              // Ignore parse errors for incomplete chunks
            }
          }
        }
      }

      // Show completion info
      if (isJsonStreaming) {
        // If we didn't get usage info from API, use current balance
        if (remainingBalance === 0 && creditsUsed === 0) {
          const currentProfile = await this.supabase.getUserProfile();
          remainingBalance = currentProfile?.credits_balance || 0;
          // Estimate credits used based on model tier
          const { MODEL_TIERS } = await import('../config/model-tiers.js');
          const tier = MODEL_TIERS[options.modelTier || 'standard'];
          creditsUsed = tier?.ruvCredits || 0;
        }
        
        console.log(JSON.stringify({ 
          type: "complete", 
          credits_used: creditsUsed,
          remaining: remainingBalance,
          timestamp: Date.now() 
        }));
      } else {
        if (creditsUsed > 0) {
          console.log(chalk.gray(`\n\n💎 Credits used: ${creditsUsed} | Remaining: ${remainingBalance}`));
        }
      }
      
      return fullResponse || null;
      
    } catch (error) {
      if (isJsonStreaming) {
        console.log(JSON.stringify({ 
          type: "error", 
          content: `Failed to reach the Queen: ${error.message}`,
          timestamp: Date.now() 
        }));
      } else {
        console.log(chalk.red(`❌ Failed to reach the Queen: ${error.message}`));
      }
      return null;
    }
  }

  /**
   * Show help information
   */
  showHelp() {
    console.log(chalk.cyan('\n📚 Available Commands:'));
    console.log(chalk.gray('  /help        - Show this help message'));
    console.log(chalk.gray('  /tools       - List available tools'));
    console.log(chalk.gray('  /tool <name> - Execute a tool (you\'ll be prompted for parameters)'));
    console.log(chalk.gray('  exit         - End the audience with Queen Seraphina\n'));
  }

  /**
   * Show available tools
   */
  async showAvailableTools() {
    try {
      console.log(chalk.cyan('\n🛠️ Available MCP Tools:\n'));
      
      if (!this.enableTools) {
        console.log(chalk.yellow('⚠️ Tools are currently disabled.'));
        console.log(chalk.gray('Restart with --tools flag to enable tool execution.'));
        console.log(chalk.gray('Example: flow-nexus seraphina --tools\n'));
        return;
      }
      
      console.log(chalk.white.bold('🤖 Swarm Operations:'));
      console.log(chalk.gray('  swarm_init <topology>     ') + chalk.cyan('- Initialize swarm (mesh/hierarchical/ring/star)'));
      console.log(chalk.gray('  agent_spawn <type>        ') + chalk.cyan('- Spawn agent (researcher/coder/analyst/optimizer)'));
      console.log(chalk.gray('  task_orchestrate <task>   ') + chalk.cyan('- Orchestrate complex task across swarm'));
      console.log(chalk.gray('  swarm_status              ') + chalk.cyan('- Get current swarm status'));
      
      console.log(chalk.white.bold('\n📦 Sandbox Management:'));
      console.log(chalk.gray('  sandbox_create <template> ') + chalk.cyan('- Create sandbox (node/python/react/nextjs)'));
      console.log(chalk.gray('  sandbox_execute <code>    ') + chalk.cyan('- Execute code in sandbox'));
      console.log(chalk.gray('  sandbox_list              ') + chalk.cyan('- List all active sandboxes'));
      console.log(chalk.gray('  sandbox_logs <id>         ') + chalk.cyan('- View sandbox logs'));
      
      console.log(chalk.white.bold('\n🚀 Deployment & Templates:'));
      console.log(chalk.gray('  template_deploy <name>    ') + chalk.cyan('- Deploy application template'));
      console.log(chalk.gray('  template_list             ') + chalk.cyan('- List available templates'));
      console.log(chalk.gray('  workflow_create <name>    ') + chalk.cyan('- Create automation workflow'));
      
      console.log(chalk.white.bold('\n💾 Storage & Data:'));
      console.log(chalk.gray('  storage_upload <file>     ') + chalk.cyan('- Upload file to cloud storage'));
      console.log(chalk.gray('  storage_list              ') + chalk.cyan('- List stored files'));
      console.log(chalk.gray('  analytics_query <metric>  ') + chalk.cyan('- Query analytics data'));
      
      console.log(chalk.white.bold('\n👤 Profile & Credits:'));
      console.log(chalk.gray('  credits_balance           ') + chalk.cyan('- Check rUv credit balance'));
      console.log(chalk.gray('  profile_view              ') + chalk.cyan('- View your profile'));
      console.log(chalk.gray('  leaderboard               ') + chalk.cyan('- View global rankings'));
      
      console.log(chalk.yellow('\n💡 Usage Examples:'));
      console.log(chalk.gray('  "Create a mesh swarm with 5 agents"'));
      console.log(chalk.gray('  "Deploy a Python sandbox and run my code"'));
      console.log(chalk.gray('  "Show me the leaderboard"'));
      console.log(chalk.gray('  "Check my credit balance"'));
      
      console.log(chalk.green('\n✨ When tools are enabled, I can execute these directly!'));
      console.log(chalk.gray('Just ask naturally and I\'ll handle the tool calls.\n'));
    } catch (error) {
      console.log(chalk.red(`❌ Failed to fetch tools: ${error.message}`));
    }
  }

  /**
   * Handle tool command
   */
  async handleToolCommand(command) {
    const parts = command.trim().split(' ');
    const toolName = parts[0];
    
    if (!toolName) {
      console.log(chalk.yellow('⚠️ Usage: /tool <tool_name> [parameters]'));
      console.log(chalk.gray('   Example: /tool credits_balance'));
      console.log(chalk.gray('   Example: /tool swarm_create topology=mesh maxAgents=5'));
      return;
    }

    try {
      console.log(chalk.magenta(`\n⚡ Executing tool: ${toolName}...`));
      
      // Parse parameters from command line if provided
      const params = {};
      for (let i = 1; i < parts.length; i++) {
        const [key, value] = parts[i].split('=');
        if (key && value) {
          // Try to parse numbers
          if (!isNaN(value)) {
            params[key] = Number(value);
          } else {
            params[key] = value;
          }
        }
      }

      const result = await this.executeTool(toolName, params);
      
      if (result.success) {
        console.log(chalk.green(`\n✅ Tool executed successfully!`));
        if (result.data?.message) {
          console.log(chalk.cyan(result.data.message));
        }
        if (result.data && !result.data.message) {
          console.log(chalk.gray(JSON.stringify(result.data, null, 2)));
        }
        if (result.credits_charged !== undefined) {
          console.log(chalk.yellow(`💎 Credits charged: ${result.credits_charged} rUv`));
        }
      } else {
        console.log(chalk.red(`\n❌ Tool execution failed: ${result.error}`));
      }
    } catch (error) {
      console.log(chalk.red(`\n❌ Error executing tool: ${error.message}`));
    }
  }

  /**
   * Execute a tool directly through Seraphina
   */
  async executeTool(toolName, parameters = {}, interactionId = null) {
    // Check authentication
    const user = await this.supabase.getCurrentUser();
    if (!user) {
      throw new Error('Authentication required to execute tools');
    }

    const { data: { session } } = await this.supabase.supabase.auth.getSession();
    if (!session) {
      throw new Error('No active session');
    }

    const supabaseUrl = this.supabase.supabase?.supabaseUrl || 
                        process.env.SUPABASE_URL ||
                        'https://pklhxiuouhrcrreectbo.supabase.co';
    
    // Call the seraphina-tools edge function
    const response = await fetch(`${supabaseUrl}/functions/v1/seraphina-tools`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${session.access_token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        tool_name: toolName,
        parameters: parameters,
        interaction_id: interactionId
      })
    });

    const result = await response.json();
    
    if (!response.ok) {
      throw new Error(result.error || `Tool execution failed: ${response.statusText}`);
    }

    return result;
  }

  /**
   * End the chat session
   */
  async endSession() {
    console.log(chalk.magenta('\n\n*The Queen\'s presence fades into the digital ether...*'));
    console.log(chalk.cyan('Until we meet again, warrior. May your code compile and your tests pass.\n'));
    
    // Mark as closing before closing readline
    this.isClosing = true;
    
    if (this.rl && !this.rl.closed) {
      this.rl.close();
      this.rl = null;
    }
    
    // Resolve the session promise
    if (this.sessionResolver) {
      this.sessionResolver();
      this.sessionResolver = null;
    }
    
    // Clean up the conversation history
    this.conversationHistory = [];
  }
}

export default SeraphinaClient;