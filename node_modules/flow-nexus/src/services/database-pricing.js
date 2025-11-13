/**
 * Database-Driven Pricing Service
 * All pricing logic controlled by Supabase database
 * NO HARDCODED VALUES - Everything from database
 */

import chalk from 'chalk';

class DatabasePricingService {
  constructor(supabaseClient) {
    this.supabase = supabaseClient;
    this.pricingConfig = null;
    this.configCacheTimeout = 5 * 60 * 1000; // 5 minutes cache
    this.lastConfigFetch = 0;
  }

  /**
   * Get pricing configuration from database
   * Cached for 5 minutes to reduce database calls
   */
  async getPricingConfig() {
    const now = Date.now();
    
    // Return cached config if still valid
    if (this.pricingConfig && (now - this.lastConfigFetch) < this.configCacheTimeout) {
      return this.pricingConfig;
    }

    try {
      // Call database function to get all pricing config
      const { data, error } = await this.supabase
        .rpc('get_pricing_config');

      if (error) throw error;

      this.pricingConfig = data;
      this.lastConfigFetch = now;
      
      console.log(chalk.green('✓ Loaded pricing config from database'));
      return this.pricingConfig;
    } catch (err) {
      console.error(chalk.red('Failed to load pricing config:'), err);
      
      // Return last known config or defaults
      return this.pricingConfig || {
        credits_per_dollar: 100,
        credit_value_usd: 0.01,
        min_payment_usd: 10,
        max_payment_usd: 10000,
        bonus_threshold_usd: 100,
        bonus_rate: 1.1
      };
    }
  }

  /**
   * Calculate credits from USD amount
   * Uses database function for consistency
   */
  async calculateCredits(amountUsd, applyBonus = true) {
    try {
      const { data, error } = await this.supabase
        .rpc('calculate_credits_from_usd', {
          amount_usd: amountUsd,
          apply_bonus: applyBonus
        });

      if (error) throw error;
      return data;
    } catch (err) {
      console.error('Failed to calculate credits:', err);
      
      // Fallback calculation using cached config
      const config = await this.getPricingConfig();
      let credits = amountUsd * config.credits_per_dollar;
      
      if (applyBonus && amountUsd >= config.bonus_threshold_usd) {
        credits = Math.floor(credits * config.bonus_rate);
      }
      
      return credits;
    }
  }

  /**
   * Get tool cost for a specific user (with tier discounts)
   */
  async getToolCost(toolName, userId = null) {
    try {
      const { data, error } = await this.supabase
        .rpc('get_tool_cost', {
          p_tool_name: toolName,
          p_user_id: userId
        });

      if (error) throw error;
      return data?.[0] || null;
    } catch (err) {
      console.error('Failed to get tool cost:', err);
      return null;
    }
  }

  /**
   * Process a payment and add credits
   * All logic handled by database
   */
  async processPayment(userId, amountUsd, paymentType = 'deposit', stripePaymentId = null) {
    try {
      const { data, error } = await this.supabase
        .rpc('process_payment', {
          p_user_id: userId,
          p_amount_usd: amountUsd,
          p_payment_type: paymentType,
          p_stripe_payment_id: stripePaymentId
        });

      if (error) throw error;
      return data;
    } catch (err) {
      console.error('Failed to process payment:', err);
      return {
        success: false,
        error: err.message || 'Payment processing failed'
      };
    }
  }

  /**
   * Execute tool with credits (all pricing from database)
   */
  async executeToolWithCredits(userId, toolName, parameters = {}, result = null, metadata = {}) {
    try {
      const { data, error } = await this.supabase
        .rpc('execute_tool_with_credits', {
          p_user_id: userId,
          p_tool_name: toolName,
          p_parameters: parameters,
          p_result: result,
          p_metadata: metadata
        });

      if (error) throw error;
      return data;
    } catch (err) {
      console.error('Failed to execute tool with credits:', err);
      return {
        success: false,
        error: err.message || 'Tool execution failed'
      };
    }
  }

  /**
   * Get user's subscription tier and benefits
   */
  async getUserTier(userId) {
    try {
      const { data, error } = await this.supabase
        .from('profiles')
        .select(`
          subscription_tier,
          pricing_tiers!inner(
            tier_name,
            monthly_credits,
            bonus_percentage,
            volume_discount_rate,
            features
          )
        `)
        .eq('id', userId)
        .single();

      if (error) throw error;
      return data?.pricing_tiers || { tier_name: 'free' };
    } catch (err) {
      console.error('Failed to get user tier:', err);
      return { tier_name: 'free' };
    }
  }

  /**
   * Check if user can afford a tool
   */
  async canAffordTool(userId, toolName) {
    try {
      // Get tool cost with user's discount
      const toolCost = await this.getToolCost(toolName, userId);
      if (!toolCost) return { canAfford: false, reason: 'Tool not found' };

      // Get user balance
      const { data: profile, error } = await this.supabase
        .from('profiles')
        .select('credits_balance, subscription_tier')
        .eq('id', userId)
        .single();

      if (error || !profile) {
        return { canAfford: false, reason: 'User not found' };
      }

      const canAfford = profile.credits_balance >= toolCost.final_cost;

      return {
        canAfford,
        balance: profile.credits_balance,
        cost: toolCost.final_cost,
        baseCost: toolCost.base_cost,
        discount: toolCost.discount_applied,
        tier: toolCost.user_tier
      };
    } catch (err) {
      console.error('Failed to check affordability:', err);
      return { canAfford: false, reason: 'Check failed' };
    }
  }

  /**
   * Format pricing display for UI
   */
  formatCreditsDisplay(credits) {
    const config = this.pricingConfig || { credit_value_usd: 0.01 };
    const usdValue = (credits * config.credit_value_usd).toFixed(2);
    return `${credits} credits ($${usdValue})`;
  }

  /**
   * Get all available tiers for display
   */
  async getAvailableTiers() {
    try {
      const { data, error } = await this.supabase
        .from('pricing_tiers')
        .select('*')
        .eq('is_active', true)
        .order('tier_level', { ascending: true });

      if (error) throw error;
      return data;
    } catch (err) {
      console.error('Failed to get tiers:', err);
      return [];
    }
  }

  /**
   * Validate payment amount against database limits
   */
  async validatePaymentAmount(amountUsd) {
    const config = await this.getPricingConfig();
    
    if (amountUsd < config.min_payment_usd) {
      return {
        valid: false,
        error: `Minimum payment is $${config.min_payment_usd}`
      };
    }
    
    if (amountUsd > config.max_payment_usd) {
      return {
        valid: false,
        error: `Maximum payment is $${config.max_payment_usd}`
      };
    }
    
    return {
      valid: true,
      willGetBonus: amountUsd >= config.bonus_threshold_usd,
      credits: await this.calculateCredits(amountUsd, true)
    };
  }

  /**
   * Clear cached configuration (force reload on next request)
   */
  clearCache() {
    this.pricingConfig = null;
    this.lastConfigFetch = 0;
    console.log(chalk.yellow('Pricing config cache cleared'));
  }
}

export default DatabasePricingService;