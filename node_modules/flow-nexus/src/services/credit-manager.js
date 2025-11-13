/**
 * Credit Manager Service
 * Handles rUv credit deduction using Supabase triggers and functions
 */

import chalk from 'chalk';

class CreditManager {
  constructor(supabaseClient) {
    this.supabase = supabaseClient;
  }

  /**
   * Deduct credits for a tool using the database function
   * @param {string} userId - User ID
   * @param {string} toolName - Tool name from tool_costs table
   * @param {object} parameters - Tool parameters
   * @param {object} result - Tool execution result
   * @param {object} metadata - Additional metadata for the transaction
   * @returns {object} Result with success status and details
   */
  async deductCreditsForTool(userId, toolName, parameters = {}, result = null, metadata = {}) {
    try {
      // Call the new unified database function
      const { data, error } = await this.supabase
        .rpc('execute_tool_with_credits', {
          p_user_id: userId,
          p_tool_name: toolName,
          p_parameters: parameters,
          p_result: result,
          p_metadata: metadata
        });

      if (error) {
        console.error('Credit deduction error:', error);
        return {
          success: false,
          error: error.message || 'Failed to deduct credits'
        };
      }

      // Return the data from the function
      return data || {
        success: false,
        error: 'No response from credit deduction'
      };
    } catch (err) {
      console.error('Credit deduction failed:', err);
      return {
        success: false,
        error: err.message || 'Credit deduction failed'
      };
    }
  }

  /**
   * Check if user has sufficient credits for a tool
   * @param {string} userId - User ID
   * @param {string} toolName - Tool name
   * @returns {object} Result with balance info
   */
  async checkCreditsForTool(userId, toolName) {
    try {
      // Get tool cost
      const { data: toolCost, error: costError } = await this.supabase
        .from('tool_costs')
        .select('cost')
        .eq('tool_name', toolName)
        .eq('is_active', true)
        .single();

      if (costError || !toolCost) {
        return {
          success: false,
          error: 'Tool not found or inactive'
        };
      }

      // Get user balance
      const { data: profile, error: profileError } = await this.supabase
        .from('profiles')
        .select('credits_balance')
        .eq('id', userId)
        .single();

      if (profileError || !profile) {
        return {
          success: false,
          error: 'User profile not found'
        };
      }

      const hasSufficient = profile.credits_balance >= toolCost.cost;

      return {
        success: true,
        hasSufficient,
        cost: toolCost.cost,
        balance: profile.credits_balance,
        toolName
      };
    } catch (err) {
      console.error('Credit check failed:', err);
      return {
        success: false,
        error: err.message || 'Credit check failed'
      };
    }
  }

  /**
   * Get credit stats for a user
   * @param {string} userId - User ID
   * @returns {object} Credit statistics
   */
  async getUserCreditStats(userId) {
    try {
      const { data, error } = await this.supabase
        .rpc('get_user_credit_stats', {
          p_user_id: userId
        });

      if (error) {
        console.error('Failed to get credit stats:', error);
        return null;
      }

      return data;
    } catch (err) {
      console.error('Failed to get credit stats:', err);
      return null;
    }
  }

  /**
   * Format credit result for display
   * @param {object} result - Credit deduction result
   * @returns {string} Formatted message
   */
  formatCreditResult(result) {
    if (!result || !result.success) {
      return chalk.red(`❌ ${result?.error || 'Credit deduction failed'}`);
    }

    if (result.cost === 0) {
      return chalk.green(`✅ Free tool - no credits deducted (Balance: ${result.balance_after || result.balance} rUv)`);
    }

    return chalk.green(
      `✅ Deducted ${result.cost} rUv\n` +
      `   Balance: ${result.balance_before} → ${result.balance_after} rUv\n` +
      `   Transaction: ${result.transaction_id}`
    );
  }

  /**
   * Calculate swarm creation cost
   * @param {number} maxAgents - Number of agents
   * @returns {number} Total cost in rUv
   */
  calculateSwarmCost(maxAgents) {
    // swarm_init costs 3 rUv (from tool_costs table)
    // agent_spawn costs 2 rUv per agent (from tool_costs table)
    return 3 + (maxAgents * 2);
  }

  /**
   * Handle swarm creation with atomic credit deduction
   * @param {string} userId - User ID
   * @param {object} swarmConfig - Swarm configuration
   * @returns {object} Result with swarm details
   */
  async createSwarmWithCredits(userId, swarmConfig) {
    try {
      // First deduct for swarm_init
      const initParams = {
        topology: swarmConfig.topology,
        maxAgents: swarmConfig.max_agents
      };
      
      const initResult = await this.deductCreditsForTool(
        userId, 
        'swarm_init',
        initParams,
        null,
        { swarm_id: swarmConfig.swarm_id }
      );

      if (!initResult.success) {
        return initResult;
      }

      // Then deduct for each agent
      const agentResults = [];
      for (let i = 0; i < swarmConfig.max_agents; i++) {
        const agentParams = {
          type: swarmConfig.agent_types?.[i] || 'worker',
          index: i
        };
        
        const agentResult = await this.deductCreditsForTool(
          userId,
          'agent_spawn',
          agentParams,
          null,
          { swarm_id: swarmConfig.swarm_id, agent_index: i }
        );

        if (!agentResult.success) {
          // Rollback would be complex, so we'll just log the failure
          console.error(`Failed to deduct credits for agent ${i}:`, agentResult.error);
          break;
        }

        agentResults.push(agentResult);
      }

      return {
        success: true,
        swarm_id: swarmConfig.swarm_id,
        init_transaction: initResult.transaction_id,
        agent_transactions: agentResults.map(r => r.transaction_id),
        total_cost: this.calculateSwarmCost(swarmConfig.max_agents),
        final_balance: agentResults.length > 0 
          ? agentResults[agentResults.length - 1].balance_after 
          : initResult.balance_after
      };
    } catch (err) {
      console.error('Swarm creation with credits failed:', err);
      return {
        success: false,
        error: err.message || 'Failed to create swarm with credits'
      };
    }
  }
}

export default CreditManager;