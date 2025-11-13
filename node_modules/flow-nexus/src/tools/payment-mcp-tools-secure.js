/**
 * Flow-Nexus MCP Payment Tools
 * Secure implementation with Supabase auth and vault integration
 * PCI DSS SAQ-A compliant - No card data stored or processed
 */

import chalk from 'chalk';
import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';

// Load environment variables from root .env (suppress dotenv output)
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Temporarily suppress console.log to hide dotenv message
const originalLog = console.log;
console.log = () => {};
dotenv.config({ path: path.resolve(__dirname, '../../../../.env') });
console.log = originalLog;

// Security constants - All pricing from database
const MIN_AMOUNT = 10; // Minimum $10 to prevent micro-transaction abuse
const MAX_AMOUNT = 10000; // Maximum $10,000 for fraud prevention
// REMOVED CREDIT_RATE - All pricing now fetched from database
// Conversion rate, bonus threshold, and bonus rate are stored in database

/**
 * Validate user authentication using Supabase session
 * @param {Object} context - Request context with user info
 * @param {string} requiredRole - Required role for operation
 * @returns {Object} Validation result
 */
function validateAuth(context, requiredRole = null) {
  // Check if user is authenticated via Supabase
  if (!context?.user?.id) {
    return {
      valid: false,
      error: 'User not authenticated. Please log in via flow-nexus auth.',
    };
  }

  // Check admin role in user metadata
  if (requiredRole === 'admin') {
    const isAdmin = context.user?.app_metadata?.role === 'admin' ||
                    context.user?.user_metadata?.is_admin === true;
    
    if (!isAdmin) {
      return {
        valid: false,
        error: 'Admin permission required for this operation.',
      };
    }
  }

  return { 
    valid: true, 
    userId: context.user.id,
    email: context.user.email,
  };
}

/**
 * Calculate credits using database function
 * @param {Object} supabase - Supabase client
 * @param {number} amount - USD amount
 * @returns {Promise<number>} Credit amount with bonuses applied
 */
async function calculateCredits(supabase, amount) {
  try {
    // Use database function for consistent calculation
    const { data, error } = await supabase.rpc('calculate_credits_from_usd', {
      amount_usd: amount,
      apply_bonus: true
    });
    
    if (error) throw error;
    return data || (amount * 100); // Fallback: 1 USD = 100 credits
  } catch (err) {
    console.error('Failed to calculate credits from database:', err);
    // Safe fallback: Use correct rate of 100 credits per USD
    return amount >= 100 ? Math.floor(amount * 100 * 1.1) : amount * 100;
  }
}

/**
 * Call Supabase Edge Function for payment processing
 * Uses Supabase vault for Stripe keys
 */
async function callSupabaseFunction(supabase, functionName, payload, userId) {
  try {
    // Call edge function with user context
    const { data, error } = await supabase.functions.invoke(functionName, {
      body: {
        ...payload,
        user_id: userId,
      },
      headers: {
        // Pass user JWT for RLS
        Authorization: `Bearer ${supabase.auth.session()?.access_token}`,
      },
    });

    if (error) throw error;
    return data;
  } catch (error) {
    console.error(chalk.red(`Edge function error: ${error.message}`));
    throw error;
  }
}

/**
 * Register payment tools with the MCP server
 * All sensitive operations delegated to Supabase edge functions
 */
export function registerPaymentTools(server, supabase) {
  
  // Tool: check_balance
  server.addTool({
    name: 'check_balance',
    description: 'Check current credit balance and auto-refill status',
    inputSchema: {
      type: 'object',
      properties: {},
    },
    handler: async (params, context) => {
      try {
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }

        // Use database function to get accurate balance from ruv_transactions
        const { data: calculatedBalance, error: calcError } = await supabase
          .rpc('calculate_user_balance', { p_user_id: auth.userId });
        
        // Also get profile for auto-refill settings
        const { data: profile, error } = await supabase
          .from('profiles')
          .select('credits_balance, ruv_credits, auto_refill_enabled, auto_refill_threshold, auto_refill_amount')
          .eq('id', auth.userId)
          .single();

        if (error || calcError) {
          throw new Error('Failed to fetch balance');
        }

        // Use calculated balance as source of truth, fallback to profile if needed
        const balance = calculatedBalance !== null ? calculatedBalance : (profile.credits_balance || profile.ruv_credits || 0);
        const lowBalance = balance < 20;

        // Check if auto-refill should trigger via Supabase function
        if (lowBalance && profile.auto_refill_enabled) {
          await supabase.rpc('check_auto_refill', {
            p_user_id: auth.userId,
          });
        }

        return {
          success: true,
          balance: balance,
          auto_refill_enabled: profile.auto_refill_enabled,
          auto_refill_threshold: profile.auto_refill_threshold,
          auto_refill_amount: profile.auto_refill_amount,
          low_balance_warning: lowBalance,
          message: `Current balance: ${balance} credits${lowBalance ? ' (⚠️ LOW BALANCE)' : ''}`,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to check balance. Please try again.',
        };
      }
    },
  });

  // Tool: create_payment_link
  server.addTool({
    name: 'create_payment_link',
    description: 'Create a secure payment link for purchasing credits',
    inputSchema: {
      type: 'object',
      properties: {
        amount: {
          type: 'number',
          description: 'Amount in USD (minimum $10)',
          minimum: 10,
          maximum: 10000,
        },
      },
      required: ['amount'],
    },
    handler: async (params, context) => {
      try {
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        const amount = params.amount;
        
        // Validate amount
        if (amount < MIN_AMOUNT || amount > MAX_AMOUNT) {
          return {
            success: false,
            error: `Amount must be between $${MIN_AMOUNT} and $${MAX_AMOUNT}`,
          };
        }
        
        // Calculate credits using database function
        const credits = await calculateCredits(supabase, amount);

        // Call Supabase edge function to create payment link
        // This keeps Stripe API key in Supabase vault
        const result = await callSupabaseFunction(
          supabase,
          'create-payment-link',
          {
            amount: amount,
            credits: credits,
            type: 'deposit',
          },
          auth.userId
        );

        if (!result.success) {
          throw new Error(result.error || 'Failed to create payment link');
        }

        return {
          success: true,
          payment_url: result.payment_url,
          amount: amount,
          credits: credits,
          expires_at: result.expires_at,
          message: `Payment link created: ${result.payment_url}`,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to create payment link. Please try again.',
        };
      }
    },
  });

  // Tool: configure_auto_refill
  server.addTool({
    name: 'configure_auto_refill',
    description: 'Configure automatic credit refill settings',
    inputSchema: {
      type: 'object',
      properties: {
        enabled: {
          type: 'boolean',
          description: 'Enable or disable auto-refill',
        },
        threshold: {
          type: 'number',
          description: 'Credit threshold to trigger refill',
          minimum: 10,
        },
        amount: {
          type: 'number',
          description: 'Amount in USD to refill',
          minimum: 10,
        },
      },
      required: ['enabled'],
    },
    handler: async (params, context) => {
      try {
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        // Check if user has payment method via Supabase
        const { data: profile } = await supabase
          .from('profiles')
          .select('stripe_payment_method_id')
          .eq('id', auth.userId)
          .single();

        if (params.enabled && !profile?.stripe_payment_method_id) {
          return {
            success: false,
            error: 'No payment method on file. Complete a payment first.',
          };
        }

        // Update settings using Supabase with RLS
        const updates = {
          auto_refill_enabled: params.enabled,
          updated_at: new Date().toISOString(),
        };

        if (params.threshold !== undefined) {
          updates.auto_refill_threshold = params.threshold;
        }
        if (params.amount !== undefined) {
          updates.auto_refill_amount = params.amount;
        }

        const { error } = await supabase
          .from('profiles')
          .update(updates)
          .eq('id', auth.userId);

        if (error) {
          throw new Error('Failed to update settings');
        }

        // Log configuration change
        await supabase
          .from('audit_logs')
          .insert({
            user_id: auth.userId,
            action: 'auto_refill_config',
            details: {
              enabled: params.enabled,
              threshold: params.threshold,
              amount: params.amount,
            },
          });

        return {
          success: true,
          enabled: params.enabled,
          threshold: params.threshold,
          amount: params.amount,
          message: `Auto-refill ${params.enabled ? 'enabled' : 'disabled'}`,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to configure auto-refill. Please try again.',
        };
      }
    },
  });

  // Tool: get_payment_history
  server.addTool({
    name: 'get_payment_history',
    description: 'Get recent payment and transaction history',
    inputSchema: {
      type: 'object',
      properties: {
        limit: {
          type: 'number',
          description: 'Number of transactions to return',
          minimum: 1,
          maximum: 100,
          default: 10,
        },
      },
    },
    handler: async (params, context) => {
      try {
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        const limit = params.limit || 10;

        // Use Supabase with RLS to get user's own transactions
        const { data: transactions, error } = await supabase
          .from('stripe_transactions')
          .select('id, type, status, amount, credits_purchased, created_at')
          .eq('user_id', auth.userId)
          .order('created_at', { ascending: false })
          .limit(limit);

        if (error) {
          throw new Error('Failed to fetch history');
        }

        // Sanitize transaction data
        const sanitized = transactions?.map(t => ({
          id: t.id,
          type: t.type,
          status: t.status,
          amount: t.amount,
          credits: t.credits_purchased,
          date: t.created_at,
        })) || [];

        return {
          success: true,
          transactions: sanitized,
          count: sanitized.length,
          message: `Found ${sanitized.length} transactions`,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to fetch payment history.',
        };
      }
    },
  });



  // Log registration to stderr only (not stdout) to avoid breaking MCP protocol
  if (process.stderr && process.env.MCP_MODE !== 'stdio') {
    console.error(chalk.green('✅ Payment tools registered with flow-nexus MCP server'));
    console.error(chalk.gray('  - PCI DSS SAQ-A compliant (no card data)'));
    console.error(chalk.gray('  - Supabase auth & RLS integrated'));
    console.error(chalk.gray('  - Stripe keys stored in Supabase vault'));
    console.error(chalk.gray('  - Edge functions handle sensitive operations'));
  }
}

// Export individual tool names for testing
export const paymentTools = {
  check_balance: 'check_balance',
  create_payment_link: 'create_payment_link',
  configure_auto_refill: 'configure_auto_refill',
  get_payment_history: 'get_payment_history',
};