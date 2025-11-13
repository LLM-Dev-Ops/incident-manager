/**
 * Authentication Middleware
 * Checks if user is authenticated before allowing tool access
 */

import { secureConfig } from '../config/secure-config';
import { createClient } from '@supabase/supabase-js';

// Tools that don't require authentication
const PUBLIC_TOOLS = [
  'auth.init',
  'auth.register',
  'auth.login',
  'auth.checkAuth',
  'system.info',
  'system.help'
];

export interface AuthCheckResult {
  allowed: boolean;
  message?: string;
  requiresAuth?: boolean;
}

export class AuthMiddleware {
  /**
   * Check if a tool requires authentication
   */
  public static requiresAuth(toolName: string): boolean {
    // Check if tool is in public list
    if (PUBLIC_TOOLS.includes(toolName)) {
      return false;
    }

    // All other tools require authentication
    return true;
  }

  /**
   * Validate authentication for tool access
   */
  public static validateAccess(toolName: string): AuthCheckResult {
    // Check if tool requires auth
    if (!this.requiresAuth(toolName)) {
      return { allowed: true };
    }

    // Check if user is authenticated
    if (!secureConfig.isAuthenticated()) {
      return {
        allowed: false,
        requiresAuth: true,
        message: `
╔════════════════════════════════════════════════════════════╗
║                 🔒 AUTHENTICATION REQUIRED                 ║
╠════════════════════════════════════════════════════════════╣
║                                                            ║
║  The tool "${toolName}" requires authentication.          ║
║                                                            ║
║  Please authenticate first:                               ║
║    $ mcp-flow init      (first-time setup)               ║
║    $ mcp-flow login     (existing users)                 ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝
`
      };
    }

    // Check credits for paid tools
    const userConfig = secureConfig.getUserConfig();
    if (this.isPaidTool(toolName)) {
      const creditCost = this.getToolCreditCost(toolName);
      
      if ((userConfig.credits || 0) < creditCost) {
        return {
          allowed: false,
          message: `
╔════════════════════════════════════════════════════════════╗
║                  💳 INSUFFICIENT CREDITS                   ║
╠════════════════════════════════════════════════════════════╣
║                                                            ║
║  Tool: ${toolName.padEnd(51)}║
║  Cost: ${creditCost.toString().padEnd(51)}║
║  Your Balance: ${(userConfig.credits || 0).toString().padEnd(43)}║
║                                                            ║
║  Please purchase more credits to continue.                ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝
`
        };
      }
    }

    return { allowed: true };
  }

  /**
   * Check if tool requires credits
   */
  private static isPaidTool(toolName: string): boolean {
    const paidTools = [
      'swarm_init',
      'agent_spawn',
      'task_orchestrate',
      'neural_train',
      'neural_predict',
      'neural_cluster_init',
      'neural_node_deploy',
      'daa_agent_create',
      'github_pr_manage',
      'workflow_execute',
      'workflow_create',
      'sandbox_create',
      'sandbox_execute',
      'sandbox_stop',
      'sandbox_configure'
    ];

    return paidTools.some(tool => toolName.includes(tool));
  }

  /**
   * Get credit cost for a tool
   */
  private static getToolCreditCost(toolName: string): number {
    const costs: { [key: string]: number } = {
      'swarm_init': 10,
      'agent_spawn': 5,
      'task_orchestrate': 8,
      'neural_train': 15,
      'daa_agent_create': 12,
      'github_pr_manage': 3,
      'workflow_execute': 20
    };

    for (const [tool, cost] of Object.entries(costs)) {
      if (toolName.includes(tool)) {
        return cost;
      }
    }

    return 1; // Default cost
  }

  /**
   * Deduct credits after successful tool execution using meter_events
   * This triggers automatic credit deduction via database trigger
   */
  public static async deductCredits(toolName: string, metadata?: any): Promise<void> {
    try {
      // Get user config for user ID
      const userConfig = secureConfig.getUserConfig();
      if (!userConfig.userId) {
        console.error('No user ID found for credit deduction');
        return;
      }

      // Get Supabase config
      const supabaseConfig = secureConfig.getSupabaseConfig();
      if (!supabaseConfig) {
        console.error('No Supabase config available');
        return;
      }

      // Create Supabase client
      const supabase = createClient(
        supabaseConfig.url,
        supabaseConfig.anonKey
      );

      // Insert into meter_events - this triggers automatic credit deduction
      const { error } = await supabase
        .from('meter_events')
        .insert({
          user_id: userConfig.userId,
          event_name: toolName,  // Must match tool_costs.tool_name
          value: 1,  // Quantity
          metadata: metadata || {}
        });

      if (error) {
        console.error('Failed to record meter event:', error);
      } else {
        console.log(`Credit deduction triggered for ${toolName}`);
      }
    } catch (error) {
      console.error('Error in deductCredits:', error);
    }
  }

  /**
   * Get authentication status summary
   */
  public static getAuthStatus(): string {
    if (!secureConfig.isAuthenticated()) {
      return 'Not authenticated - Run: mcp-flow init';
    }

    const userConfig = secureConfig.getUserConfig();
    return `Authenticated as: ${userConfig.email} | Credits: ${userConfig.credits || 0}`;
  }
}

/**
 * Middleware wrapper for tool execution
 */
export function withAuth(toolName: string, handler: Function) {
  return async (...args: any[]) => {
    // Check authentication
    const authCheck = AuthMiddleware.validateAccess(toolName);
    
    if (!authCheck.allowed) {
      return {
        success: false,
        error: 'Authentication required',
        message: authCheck.message
      };
    }

    try {
      // Execute the tool
      const result = await handler(...args);
      
      // Deduct credits if successful
      if (result.success !== false) {
        // Pass the tool parameters as metadata for logging
        const metadata = {
          parameters: args[0] || {},
          timestamp: new Date().toISOString()
        };
        await AuthMiddleware.deductCredits(toolName, metadata);
      }
      
      return result;
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Tool execution failed'
      };
    }
  };
}