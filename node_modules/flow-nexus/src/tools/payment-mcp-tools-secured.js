/**
 * Flow-Nexus MCP Payment Tools - SECURED VERSION
 * Enhanced with rate limiting, input validation, and secure vault access
 * PCI DSS SAQ-A compliant - No card data stored or processed
 */

import chalk from 'chalk';
import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';
import { rateLimiter } from '../middleware/rate-limiter.js';
import { inputValidator } from '../middleware/input-validator.js';
import { vaultSecurity } from '../middleware/vault-security.js';

// Load environment variables from root .env (suppress dotenv output)
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Temporarily suppress console.log to hide dotenv message
const originalLog = console.log;
console.log = () => {};
dotenv.config({ path: path.resolve(__dirname, '../../../../.env') });
console.log = originalLog;

// Security constants
const MIN_AMOUNT = 10; // Minimum $10 to prevent micro-transaction abuse
const MAX_AMOUNT = 10000; // Maximum $10,000 for fraud prevention
const CREDIT_RATE = 10; // 1 USD = 10 credits
const BONUS_THRESHOLD = 100; // $100+ gets bonus
const BONUS_RATE = 1.1; // 10% bonus

/**
 * Enhanced authentication validation with session expiry
 */
function validateAuth(context, requiredRole = null) {
  // Check if user is authenticated
  if (!context?.user?.id || !context?.session) {
    return {
      valid: false,
      error: 'Authentication required. Please log in.',
    };
  }
  
  // Check session expiry
  if (context.session?.expires_at) {
    const sessionExpiry = new Date(context.session.expires_at);
    if (sessionExpiry < new Date()) {
      return {
        valid: false,
        error: 'Session expired. Please log in again.',
      };
    }
  }
  
  // Check email confirmation
  if (!context.user?.email_confirmed_at) {
    return {
      valid: false,
      error: 'Please confirm your email address.',
    };
  }
  
  // Check if user is banned
  if (context.user?.banned === true) {
    return {
      valid: false,
      error: 'Account suspended.',
    };
  }
  
  // Check admin role with additional verification
  if (requiredRole === 'admin') {
    const isAdmin = 
      context.user?.app_metadata?.role === 'admin' &&
      context.user?.app_metadata?.admin_verified === true;
    
    const isSuperAdmin = context.user?.email === 'ruv@ruv.net';
    
    if (!isAdmin && !isSuperAdmin) {
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
    sessionId: context.session?.id,
  };
}

/**
 * Calculate credits with bonus logic
 */
function calculateCredits(amount) {
  let credits = amount * CREDIT_RATE;
  if (amount >= BONUS_THRESHOLD) {
    credits *= BONUS_RATE;
  }
  return Math.round(credits * 100) / 100;
}

/**
 * Enhanced Supabase function call with vault security
 */
async function callSecureSupabaseFunction(supabase, functionName, payload, userId, context) {
  try {
    // Generate vault token for this operation
    const vaultToken = await vaultSecurity.generateVaultToken(
      userId,
      'payment_vault_access',
      context
    );
    
    // Call edge function with enhanced security
    const { data, error } = await supabase.functions.invoke(functionName, {
      body: {
        ...payload,
        user_id: userId,
        vault_token: vaultToken,
      },
      headers: {
        Authorization: `Bearer ${context.session?.access_token}`,
        'X-Request-ID': crypto.randomUUID(),
        'X-User-Fingerprint': vaultSecurity.generateFingerprint(context),
      },
    });
    
    if (error) throw error;
    return data;
  } catch (error) {
    console.error(chalk.red(`Secure function error: ${error.message}`));
    throw error;
  }
}

/**
 * Register secured payment tools with the MCP server
 */
export function registerSecuredPaymentTools(server, supabase) {
  
  // Tool: check_balance (with rate limiting)
  server.addTool({
    name: 'check_balance',
    description: 'Check current credit balance and auto-refill status',
    inputSchema: {
      type: 'object',
      properties: {},
    },
    handler: async (params, context) => {
      try {
        // Rate limiting
        const rateLimit = rateLimiter.checkLimit('check_balance', context, context.ip);
        if (!rateLimit.allowed) {
          return {
            success: false,
            error: rateLimit.reason,
            retryAfter: rateLimit.retryAfter,
          };
        }
        
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        // Use Supabase RLS to get user's own profile
        const { data: profile, error } = await supabase
          .from('profiles')
          .select('credits_balance, ruv_credits, auto_refill_enabled, auto_refill_threshold, auto_refill_amount')
          .eq('id', auth.userId)
          .single();
        
        if (error) {
          throw new Error('Failed to fetch balance');
        }
        
        const balance = profile.credits_balance || profile.ruv_credits || 0;
        const lowBalance = balance < 20;
        
        // Check if auto-refill should trigger
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
          rateLimit: rateLimit.headers,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to check balance. Please try again.',
        };
      }
    },
  });
  
  // Tool: create_payment_link (with validation and rate limiting)
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
        // Input validation
        const validation = inputValidator.validate('create_payment_link', params);
        if (!validation.valid) {
          return {
            success: false,
            error: 'Validation failed',
            errors: validation.errors,
          };
        }
        
        // Rate limiting
        const rateLimit = rateLimiter.checkLimit('create_payment_link', context, context.ip);
        if (!rateLimit.allowed) {
          return {
            success: false,
            error: rateLimit.reason,
            retryAfter: rateLimit.retryAfter,
          };
        }
        
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        const amount = validation.sanitized.amount;
        
        // Additional amount validation
        if (amount < MIN_AMOUNT || amount > MAX_AMOUNT) {
          return {
            success: false,
            error: `Amount must be between $${MIN_AMOUNT} and $${MAX_AMOUNT}`,
          };
        }
        
        // Calculate credits with bonus
        const credits = calculateCredits(amount);
        
        // Call secure edge function with vault token
        const result = await callSecureSupabaseFunction(
          supabase,
          'create-payment-link',
          {
            amount: amount,
            credits: credits,
            type: 'deposit',
          },
          auth.userId,
          context
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
          rateLimit: rateLimit.headers,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to create payment link. Please try again.',
        };
      }
    },
  });
  
  // Tool: configure_auto_refill (with validation)
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
        // Input validation
        const validation = inputValidator.validate('configure_auto_refill', params);
        if (!validation.valid) {
          return {
            success: false,
            error: 'Validation failed',
            errors: validation.errors,
          };
        }
        
        // Rate limiting
        const rateLimit = rateLimiter.checkLimit('configure_auto_refill', context, context.ip);
        if (!rateLimit.allowed) {
          return {
            success: false,
            error: rateLimit.reason,
            retryAfter: rateLimit.retryAfter,
          };
        }
        
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        // Check if user has payment method
        const { data: profile } = await supabase
          .from('profiles')
          .select('stripe_payment_method_id')
          .eq('id', auth.userId)
          .single();
        
        if (validation.sanitized.enabled && !profile?.stripe_payment_method_id) {
          return {
            success: false,
            error: 'No payment method on file. Complete a payment first.',
          };
        }
        
        // Update settings using Supabase with RLS
        const updates = {
          auto_refill_enabled: validation.sanitized.enabled,
          updated_at: new Date().toISOString(),
        };
        
        if (validation.sanitized.threshold !== undefined) {
          updates.auto_refill_threshold = validation.sanitized.threshold;
        }
        if (validation.sanitized.amount !== undefined) {
          updates.auto_refill_amount = validation.sanitized.amount;
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
              enabled: validation.sanitized.enabled,
              threshold: validation.sanitized.threshold,
              amount: validation.sanitized.amount,
            },
          });
        
        return {
          success: true,
          enabled: validation.sanitized.enabled,
          threshold: validation.sanitized.threshold,
          amount: validation.sanitized.amount,
          message: `Auto-refill ${validation.sanitized.enabled ? 'enabled' : 'disabled'}`,
          rateLimit: rateLimit.headers,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to configure auto-refill. Please try again.',
        };
      }
    },
  });
  
  // Tool: get_payment_history (with validation and rate limiting)
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
        // Input validation
        const validation = inputValidator.validate('get_payment_history', params);
        if (!validation.valid) {
          return {
            success: false,
            error: 'Validation failed',
            errors: validation.errors,
          };
        }
        
        // Rate limiting
        const rateLimit = rateLimiter.checkLimit('get_payment_history', context, context.ip);
        if (!rateLimit.allowed) {
          return {
            success: false,
            error: rateLimit.reason,
            retryAfter: rateLimit.retryAfter,
          };
        }
        
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        const limit = validation.sanitized.limit || 10;
        
        // Validate SQL params to prevent injection
        const sqlCheck = inputValidator.validateSQLParams({ limit });
        if (!sqlCheck.valid) {
          return {
            success: false,
            error: sqlCheck.error,
          };
        }
        
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
          rateLimit: rateLimit.headers,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to fetch payment history.',
        };
      }
    },
  });
  
  // Tool: create_subscription (with enhanced validation)
  server.addTool({
    name: 'create_subscription',
    description: 'Create a monthly subscription for automatic credits',
    inputSchema: {
      type: 'object',
      properties: {
        plan: {
          type: 'string',
          enum: ['starter', 'pro', 'enterprise'],
          description: 'Subscription plan',
        },
      },
      required: ['plan'],
    },
    handler: async (params, context) => {
      try {
        // Input validation
        const validation = inputValidator.validate('create_subscription', params);
        if (!validation.valid) {
          return {
            success: false,
            error: 'Validation failed',
            errors: validation.errors,
          };
        }
        
        // Rate limiting (strict for subscription creation)
        const rateLimit = rateLimiter.checkLimit('create_subscription', context, context.ip);
        if (!rateLimit.allowed) {
          return {
            success: false,
            error: rateLimit.reason,
            retryAfter: rateLimit.retryAfter,
          };
        }
        
        // Validate authentication
        const auth = validateAuth(context);
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        // Get user profile with payment info
        const { data: profile } = await supabase
          .from('profiles')
          .select('stripe_customer_id')
          .eq('id', auth.userId)
          .single();
        
        if (!profile?.stripe_customer_id) {
          return {
            success: false,
            error: 'Complete a payment first to set up subscription',
          };
        }
        
        // Call secure edge function
        const result = await callSecureSupabaseFunction(
          supabase,
          'create-subscription',
          {
            plan: validation.sanitized.plan,
            customer_id: profile.stripe_customer_id,
          },
          auth.userId,
          context
        );
        
        if (!result.success) {
          throw new Error(result.error || 'Failed to create subscription');
        }
        
        return {
          success: true,
          subscription_id: result.subscription_id,
          plan: validation.sanitized.plan,
          price: result.price,
          credits_per_month: result.credits,
          message: result.message,
          rateLimit: rateLimit.headers,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Failed to create subscription. Please try again.',
        };
      }
    },
  });
  
  // Tool: reduce_credits (Admin only with strict validation)
  server.addTool({
    name: 'reduce_credits',
    description: 'Reduce user credits for alpha testing (admin only)',
    inputSchema: {
      type: 'object',
      properties: {
        email: {
          type: 'string',
          description: 'User email address',
        },
        target_balance: {
          type: 'number',
          description: 'Target credit balance',
          minimum: 0,
        },
      },
      required: ['email', 'target_balance'],
    },
    handler: async (params, context) => {
      try {
        // Input validation
        const validation = inputValidator.validate('reduce_credits', params);
        if (!validation.valid) {
          return {
            success: false,
            error: 'Validation failed',
            errors: validation.errors,
          };
        }
        
        // Rate limiting for admin operations
        const rateLimit = rateLimiter.checkLimit('reduce_credits', context, context.ip);
        if (!rateLimit.allowed) {
          return {
            success: false,
            error: rateLimit.reason,
            retryAfter: rateLimit.retryAfter,
          };
        }
        
        // Validate admin authentication with enhanced checks
        const auth = validateAuth(context, 'admin');
        if (!auth.valid) {
          return { success: false, error: auth.error };
        }
        
        // Generate vault token for admin operation
        const vaultToken = await vaultSecurity.generateVaultToken(
          auth.userId,
          'admin_vault_access',
          context
        );
        
        // Call Supabase RPC function for admin operations
        const { data: result, error } = await supabase.rpc('admin_reduce_credits', {
          p_admin_id: auth.userId,
          p_target_email: validation.sanitized.email,
          p_target_balance: validation.sanitized.target_balance,
          p_vault_token: vaultToken,
        });
        
        if (error) {
          throw new Error('Admin operation failed');
        }
        
        // Check if notifications will trigger
        const notifications = [];
        if (validation.sanitized.target_balance < 20) {
          notifications.push('Low balance notification will trigger');
        }
        if (validation.sanitized.target_balance === 0) {
          notifications.push('User will be blocked from paid tools');
        }
        
        // Enhanced audit logging
        await supabase
          .from('audit_logs')
          .insert({
            user_id: auth.userId,
            action: 'admin_credit_reduction',
            severity: 'high',
            details: {
              target_email: validation.sanitized.email,
              target_balance: validation.sanitized.target_balance,
              reason: 'alpha_testing',
              admin_email: auth.email,
              session_id: auth.sessionId,
            },
            ip_address: context.ip,
            user_agent: context.userAgent,
          });
        
        return {
          success: true,
          email: validation.sanitized.email,
          new_balance: validation.sanitized.target_balance,
          notifications: notifications,
          message: `Credits reduced to ${validation.sanitized.target_balance} for ${validation.sanitized.email}`,
          rateLimit: rateLimit.headers,
        };
      } catch (error) {
        return {
          success: false,
          error: 'Admin operation failed. Check permissions.',
        };
      }
    },
  });
  
  // Log to stderr only to avoid breaking MCP protocol
  if (process.stderr && process.env.MCP_MODE !== 'stdio') {
    console.error(chalk.green('✅ SECURED Payment tools registered with flow-nexus MCP server'));
    console.error(chalk.gray('  - Rate limiting enabled'));
    console.error(chalk.gray('  - Input validation active'));
    console.error(chalk.gray('  - Vault security enhanced'));
    console.error(chalk.gray('  - Session expiry checking'));
    console.error(chalk.gray('  - SQL injection prevention'));
    console.error(chalk.gray('  - Enhanced audit logging'));
  }
}

// Export individual tool names for testing
export const securedPaymentTools = {
  check_balance: 'check_balance',
  create_payment_link: 'create_payment_link',
  configure_auto_refill: 'configure_auto_refill',
  get_payment_history: 'get_payment_history',
  create_subscription: 'create_subscription',
  reduce_credits: 'reduce_credits',
};