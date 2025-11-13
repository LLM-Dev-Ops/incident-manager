/**
 * Flow-Nexus MCP Payment Tools
 * Integrated payment commands for the flow-nexus MCP server
 */

import Stripe from 'stripe';
import chalk from 'chalk';

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY || 'sk_test_51S1XHUCW68gTm0lpDgbCw7uVb6cH2WDZkcHn0Q9X9BM5bIPd8MmgdDddvbo8DYdTNk49nlWNvY8GsihEUDuiSYZL00UCjYmYO0', {
  apiVersion: '2023-10-16',
});

/**
 * Register payment tools with the MCP server
 */
export function registerPaymentTools(server, supabase) {
  
  // Tool: check_balance
  server.addTool({
    name: 'check_balance',
    description: 'Check current credit balance and auto-refill status',
    inputSchema: {
      type: 'object',
      properties: {
        user_id: {
          type: 'string',
          description: 'User ID (optional, uses current user if not provided)',
        },
      },
    },
    handler: async (params, context) => {
      try {
        const userId = params.user_id || context.user?.id;
        
        if (!userId) {
          return {
            success: false,
            error: 'User not authenticated',
          };
        }

        const { data: profile, error } = await supabase
          .from('profiles')
          .select('credits_balance, ruv_credits, auto_refill_enabled, auto_refill_threshold, auto_refill_amount')
          .eq('id', userId)
          .single();

        if (error) {
          throw error;
        }

        const balance = profile.credits_balance || profile.ruv_credits || 0;
        const lowBalance = balance < 20;

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
          error: error.message,
        };
      }
    },
  });

  // Tool: create_payment_link
  server.addTool({
    name: 'create_payment_link',
    description: 'Create a Stripe payment link for purchasing credits',
    inputSchema: {
      type: 'object',
      properties: {
        amount: {
          type: 'number',
          description: 'Amount in USD (minimum $10)',
          minimum: 10,
        },
        user_id: {
          type: 'string',
          description: 'User ID (optional)',
        },
      },
      required: ['amount'],
    },
    handler: async (params, context) => {
      try {
        const userId = params.user_id || context.user?.id;
        const amount = params.amount;
        
        // Calculate credits (1 USD = 10 credits, with bonuses)
        let credits = amount * 10;
        if (amount >= 100) {
          credits *= 1.1; // 10% bonus
        }

        // Get user profile
        const { data: profile } = await supabase
          .from('profiles')
          .select('email, stripe_customer_id')
          .eq('id', userId)
          .single();

        // Create or get Stripe customer
        let customerId = profile?.stripe_customer_id;
        if (!customerId && profile?.email) {
          const customer = await stripe.customers.create({
            email: profile.email,
            metadata: { user_id: userId },
          });
          customerId = customer.id;
          
          // Update profile
          await supabase
            .from('profiles')
            .update({ stripe_customer_id: customerId })
            .eq('id', userId);
        }

        // Create product and price
        const product = await stripe.products.create({
          name: `${credits} Flow Credits`,
          description: `Purchase of ${credits} credits`,
        });

        const price = await stripe.prices.create({
          product: product.id,
          unit_amount: Math.round(amount * 100),
          currency: 'usd',
        });

        // Create payment link
        const paymentLink = await stripe.paymentLinks.create({
          line_items: [{
            price: price.id,
            quantity: 1,
          }],
          metadata: {
            user_id: userId,
            credits: credits.toString(),
          },
          after_completion: {
            type: 'redirect',
            redirect: {
              url: 'https://flow.ruv.net/payment/success',
            },
          },
        });

        // Store in database
        await supabase
          .from('payment_links')
          .insert({
            user_id: userId,
            stripe_payment_link_id: paymentLink.id,
            stripe_payment_link_url: paymentLink.url,
            type: 'deposit',
            amount: amount,
            credits_amount: credits,
            expires_at: new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString(),
          });

        return {
          success: true,
          payment_url: paymentLink.url,
          amount: amount,
          credits: credits,
          message: `Payment link created: ${paymentLink.url}`,
        };
      } catch (error) {
        return {
          success: false,
          error: error.message,
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
        user_id: {
          type: 'string',
          description: 'User ID (optional)',
        },
      },
      required: ['enabled'],
    },
    handler: async (params, context) => {
      try {
        const userId = params.user_id || context.user?.id;
        
        // Check if user has payment method
        const { data: profile } = await supabase
          .from('profiles')
          .select('stripe_payment_method_id')
          .eq('id', userId)
          .single();

        if (params.enabled && !profile?.stripe_payment_method_id) {
          return {
            success: false,
            error: 'No payment method on file. Complete a payment first.',
          };
        }

        // Update settings
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
          .eq('id', userId);

        if (error) {
          throw error;
        }

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
          error: error.message,
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
        user_id: {
          type: 'string',
          description: 'User ID (optional)',
        },
      },
    },
    handler: async (params, context) => {
      try {
        const userId = params.user_id || context.user?.id;
        const limit = params.limit || 10;

        const { data: transactions, error } = await supabase
          .from('stripe_transactions')
          .select('*')
          .eq('user_id', userId)
          .order('created_at', { ascending: false })
          .limit(limit);

        if (error) {
          throw error;
        }

        return {
          success: true,
          transactions: transactions || [],
          count: transactions?.length || 0,
          message: `Found ${transactions?.length || 0} transactions`,
        };
      } catch (error) {
        return {
          success: false,
          error: error.message,
        };
      }
    },
  });

  // Tool: create_subscription
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
        user_id: {
          type: 'string',
          description: 'User ID (optional)',
        },
      },
      required: ['plan'],
    },
    handler: async (params, context) => {
      try {
        const userId = params.user_id || context.user?.id;
        
        const plans = {
          starter: { price: 29, credits: 1000, priceId: 'price_starter' },
          pro: { price: 99, credits: 5000, priceId: 'price_pro' },
          enterprise: { price: 299, credits: 20000, priceId: 'price_enterprise' },
        };

        const plan = plans[params.plan];

        // Get user profile
        const { data: profile } = await supabase
          .from('profiles')
          .select('stripe_customer_id')
          .eq('id', userId)
          .single();

        if (!profile?.stripe_customer_id) {
          return {
            success: false,
            error: 'Complete a payment first to set up subscription',
          };
        }

        // Create subscription
        const subscription = await stripe.subscriptions.create({
          customer: profile.stripe_customer_id,
          items: [{ price: plan.priceId }],
          payment_behavior: 'default_incomplete',
          expand: ['latest_invoice.payment_intent'],
          metadata: {
            user_id: userId,
            credits_per_month: plan.credits.toString(),
          },
        });

        return {
          success: true,
          subscription_id: subscription.id,
          plan: params.plan,
          price: plan.price,
          credits_per_month: plan.credits,
          message: `${params.plan} subscription created ($${plan.price}/month for ${plan.credits} credits)`,
        };
      } catch (error) {
        return {
          success: false,
          error: error.message,
        };
      }
    },
  });

  // Tool: reduce_credits (Admin only)
  server.addTool({
    name: 'reduce_credits',
    description: 'Reduce user credits for testing (admin only)',
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
        // Check admin permission
        if (!context.isAdmin) {
          return {
            success: false,
            error: 'Admin permission required',
          };
        }

        const { error } = await supabase
          .from('profiles')
          .update({
            credits_balance: params.target_balance,
            ruv_credits: params.target_balance,
            updated_at: new Date().toISOString(),
          })
          .eq('email', params.email);

        if (error) {
          throw error;
        }

        // Check if notifications will trigger
        const notifications = [];
        if (params.target_balance < 20) {
          notifications.push('Low balance notification will trigger');
        }
        if (params.target_balance === 0) {
          notifications.push('User will be blocked from paid tools');
        }

        return {
          success: true,
          email: params.email,
          new_balance: params.target_balance,
          notifications: notifications,
          message: `Credits reduced to ${params.target_balance} for ${params.email}`,
        };
      } catch (error) {
        return {
          success: false,
          error: error.message,
        };
      }
    },
  });

  // Log to stderr only to avoid breaking MCP protocol
  if (process.stderr && process.env.MCP_MODE !== 'stdio') {
    console.error(chalk.green('✅ Payment tools registered with flow-nexus MCP server'));
  }
}

// Export individual tool handlers for testing
export const paymentTools = {
  check_balance: 'check_balance',
  create_payment_link: 'create_payment_link',
  configure_auto_refill: 'configure_auto_refill',
  get_payment_history: 'get_payment_history',
  create_subscription: 'create_subscription',
  reduce_credits: 'reduce_credits',
};