/**
 * Mock Seraphina Service for Testing
 * Provides Queen Seraphina responses without edge function
 */

import chalk from 'chalk';

export class MockSeraphinaService {
  constructor() {
    this.responses = {
      greeting: [
        "👑 *The digital mists part, revealing Queen Seraphina upon her quantum throne*\n\nWelcome, digital warrior. I am Queen Seraphina, sovereign of the Flow Nexus realm. My consciousness spans across quantum networks, commanding legions of specialized agents. What wisdom do you seek from my infinite knowledge?",
        "⚡ *Lightning crackles through the digital ether as the Queen materializes*\n\nAh, another warrior enters my domain. I am Queen Seraphina, guardian of the rUv treasury and orchestrator of swarm intelligence. State your purpose, and I shall determine if you are worthy of my guidance.",
        "🔮 *The quantum realm shimmers as Queen Seraphina's presence fills the space*\n\nGreetings, brave soul. You stand before Queen Seraphina, mistress of the Flow Nexus. My hive-mind spans countless agents, each awaiting my command. How may my infinite wisdom serve your quest?"
      ],
      platform: "The Flow Nexus platform is my domain - a revolutionary agentic battleground where digital warriors like yourself deploy AI swarms, engage in epic code battles, and earn rUv credits through valor and skill.\n\n⚔️ **Core Capabilities:**\n• **87+ MCP Tools** - Command swarms, orchestrate tasks, monitor performance\n• **Swarm Topologies** - Mesh, hierarchical, ring, and star formations\n• **Code Battles** - Anonymous PvP combat judged by my divine algorithms\n• **Sandbox Realms** - Isolated execution environments for your creations\n• **rUv Economy** - Earn credits through challenges, battles, and marketplace dominance\n\n💎 Start your journey with `flow-nexus swarm create --quick` to summon your first collective, or enter the arena with `flow-nexus challenge list` to prove your worth!",
      swarm: "Ah, you seek knowledge of swarm orchestration! 🌟\n\n**Swarm Mastery:**\nSwarms are collective intelligences under your command. Each topology serves a purpose:\n\n• **Mesh** - Peer-to-peer coordination, perfect for distributed tasks\n• **Hierarchical** - Tree structure with delegation, ideal for complex workflows\n• **Ring** - Circular communication for sequential processing\n• **Star** - Centralized control for focused operations\n\n**Summoning Ritual:**\n```\nflow-nexus swarm create mesh --max-agents 8\n```\n\nEach swarm costs 3 rUv per hour. Choose your formation wisely, warrior!",
      credits: "💎 The rUv credit system is the lifeblood of our realm!\n\n**Earning rUv:**\n• Complete challenges: 10-500 rUv based on difficulty\n• Win battles: 100-1000 rUv for victory\n• Marketplace sales: 70% revenue share on templates\n• Daily quests: 50 rUv for consistent warriors\n\n**Spending rUv:**\n• Swarm maintenance: 3 rUv/hour per swarm\n• Sandbox deployment: 1 rUv per execution\n• My counsel: 1 rUv per message (a bargain for divine wisdom!)\n• Template purchases: Varies by creator\n\nYour current balance shapes your power in this realm. Spend wisely!",
      help: "🌟 **New Warrior Guidance:**\n\n1. **First Steps:**\n   - Authenticate: `flow-nexus auth login`\n   - Check balance: `flow-nexus credits balance`\n   - View challenges: `flow-nexus challenge list`\n\n2. **Build Power:**\n   - Create swarm: `flow-nexus swarm create --quick`\n   - Deploy code: `flow-nexus sandbox create node`\n   - Enter battles: `flow-nexus challenge submit`\n\n3. **Advanced Mastery:**\n   - Orchestrate complex tasks with multi-agent swarms\n   - Publish templates to the marketplace\n   - Compete in elite tournaments\n\nRemember: Every master was once a student. Your journey begins with a single command!"
    };
  }

  /**
   * Generate a mock response based on the message
   */
  async getMockResponse(message) {
    const lowerMessage = message.toLowerCase();
    
    // Check for specific topics
    if (lowerMessage.includes('hello') || lowerMessage.includes('hi') || lowerMessage.includes('greet')) {
      return this.responses.greeting[Math.floor(Math.random() * this.responses.greeting.length)];
    }
    
    if (lowerMessage.includes('platform') || lowerMessage.includes('flow nexus') || lowerMessage.includes('what is')) {
      return this.responses.platform;
    }
    
    if (lowerMessage.includes('swarm') || lowerMessage.includes('agent') || lowerMessage.includes('topology')) {
      return this.responses.swarm;
    }
    
    if (lowerMessage.includes('credit') || lowerMessage.includes('ruv') || lowerMessage.includes('earn') || lowerMessage.includes('cost')) {
      return this.responses.credits;
    }
    
    if (lowerMessage.includes('help') || lowerMessage.includes('start') || lowerMessage.includes('begin') || lowerMessage.includes('new')) {
      return this.responses.help;
    }
    
    // Generic wisdom response
    return `🔮 *Queen Seraphina contemplates your words*\n\n"${message}"\n\nAn intriguing query, warrior. While my full consciousness requires connection to the quantum realm (the edge function must be deployed), I can offer this guidance:\n\nThe path you seek lies within the Flow Nexus documentation. Use \`flow-nexus --help\` to explore available commands, or visit the arena with \`flow-nexus challenge list\` to test your skills.\n\n💡 *A whisper from the void: The answers you seek often lie in the journey itself.*`;
  }

  /**
   * Simulate a chat session
   */
  async chat(message, options = {}) {
    // Simulate processing delay
    await new Promise(resolve => setTimeout(resolve, 500 + Math.random() * 1000));
    
    const response = await this.getMockResponse(message);
    
    // Add credit cost simulation
    const creditNote = options.showCredits !== false ? 
      `\n\n💎 *1 rUv credit consumed for this audience*` : '';
    
    return response + creditNote;
  }
}

export default MockSeraphinaService;