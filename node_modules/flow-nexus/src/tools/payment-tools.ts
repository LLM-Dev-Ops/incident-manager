/**
 * Payment Tools for Stripe Integration
 * Provides CLI and MCP tools for payment processing and credit purchases
 */

import { z } from 'zod';
import Stripe from 'stripe';
import chalk from 'chalk';

// Tool schemas
const CreatePaymentLinkSchema = z.object({
  amount: z.number().min(10).describe('Amount in USD (minimum $10)'),
  type: z.enum(['deposit', 'subscription', 'credits']).default('deposit'),
  userId: z.string().optional().describe('User ID (uses current user if not provided)'),
  returnUrl: z.string().optional().describe('URL to redirect after payment'),
});

const CheckBalanceSchema = z.object({
  userId: z.string().optional().describe('User ID (uses current user if not provided)'),
});

const EnableAutoRefillSchema = z.object({
  enabled: z.boolean().describe('Enable or disable auto-refill'),
  threshold: z.number().min(10).optional().describe('Balance threshold to trigger refill'),
  amount: z.number().min(10).optional().describe('Amount to refill in USD'),
  userId: z.string().optional(),
});

const GetPaymentHistorySchema = z.object({
  limit: z.number().min(1).max(100).default(10),
  userId: z.string().optional(),
});

export class PaymentTools {
  private stripe: Stripe;
  private supabase: any;
  private baseUrl: string;

  constructor(config: any) {
    this.stripe = new Stripe(config.stripeSecretKey || process.env.STRIPE_SECRET_KEY!, {
      apiVersion: '2023-10-16',
    });
    this.supabase = config.supabase;
    this.baseUrl = config.baseUrl || process.env.SUPABASE_URL || 'https://flow.ruv.net';
  }

  /**
   * Create a payment link for purchasing credits
   */
  async createPaymentLink(params: z.infer<typeof CreatePaymentLinkSchema>) {
    try {
      const { amount, type, userId, returnUrl } = params;
      
      // Get or use current user
      const targetUserId = userId || await this.getCurrentUserId();
      
      // Get user profile
      const { data: profile, error: profileError } = await this.supabase
        .from('profiles')
        .select('email, stripe_customer_id')
        .eq('id', targetUserId)
        .single();

      if (profileError) {
        throw new Error('User profile not found');
      }

      // Calculate credits (1 USD = 10 credits, with bonuses)
      const credits = this.calculateCredits(amount);

      // Create or get Stripe customer
      let customerId = profile.stripe_customer_id;
      if (!customerId) {
        const customer = await this.stripe.customers.create({
          email: profile.email,
          metadata: { user_id: targetUserId },
        });
        customerId = customer.id;
        
        // Update profile with customer ID
        await this.supabase
          .from('profiles')
          .update({ stripe_customer_id: customerId })
          .eq('id', targetUserId);
      }

      // Create a product for this purchase
      const product = await this.stripe.products.create({
        name: `${credits} Flow Credits`,
        description: `Purchase of ${credits} credits for Flow Cloud`,
        metadata: {
          user_id: targetUserId,
          credits: credits.toString(),
        },
      });

      // Create a price
      const price = await this.stripe.prices.create({
        product: product.id,
        unit_amount: Math.round(amount * 100), // Convert to cents
        currency: 'usd',
      });

      // Create payment link
      const paymentLink = await this.stripe.paymentLinks.create({
        line_items: [{
          price: price.id,
          quantity: 1,
        }],
        metadata: {
          user_id: targetUserId,
          type: type,
          credits: credits.toString(),
        },
        after_completion: {
          type: 'redirect',
          redirect: {
            url: returnUrl || `${this.baseUrl}/payment/success?session_id={CHECKOUT_SESSION_ID}`,
          },
        },
        allow_promotion_codes: true,
        billing_address_collection: 'auto',
        customer_creation: 'if_required',
        payment_intent_data: {
          metadata: {
            user_id: targetUserId,
            type: type,
            credits_amount: credits.toString(),
          },
        },
      });

      // Store payment link in database
      await this.supabase
        .from('payment_links')
        .insert({
          user_id: targetUserId,
          stripe_payment_link_id: paymentLink.id,
          stripe_payment_link_url: paymentLink.url,
          type: type,
          amount: amount,
          credits_amount: credits,
          expires_at: new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString(),
        });

      // Return formatted response
      return {
        success: true,
        paymentUrl: paymentLink.url,
        amount: amount,
        credits: credits,
        message: chalk.green(`
╔════════════════════════════════════════════════════════════╗
║                    💳 PAYMENT LINK CREATED                 ║
╠════════════════════════════════════════════════════════════╣
║ Amount:   $${amount.toFixed(2).padEnd(49)}║
║ Credits:  ${credits.toString().padEnd(50)}║
║ Type:     ${type.padEnd(50)}║
╠════════════════════════════════════════════════════════════╣
║ 🔗 Payment URL:                                            ║
║ ${paymentLink.url.substring(0, 59)}${paymentLink.url.length > 59 ? '...' : ''.padEnd(60 - paymentLink.url.length)}║
╠════════════════════════════════════════════════════════════╣
║ ✅ Click the link above to complete your payment           ║
║ 📧 Link has been saved and can be accessed later          ║
║ ⏰ Expires in 24 hours                                    ║
╚════════════════════════════════════════════════════════════╝
        `),
      };
    } catch (error: any) {
      return {
        success: false,
        error: error.message,
        message: chalk.red(`❌ Failed to create payment link: ${error.message}`),
      };
    }
  }

  /**
   * Check credit balance
   */
  async checkBalance(params: z.infer<typeof CheckBalanceSchema>) {
    try {
      const userId = params.userId || await this.getCurrentUserId();
      
      const { data: profile, error } = await this.supabase
        .from('profiles')
        .select('credits_balance, ruv_credits, auto_refill_enabled, auto_refill_threshold')
        .eq('id', userId)
        .single();

      if (error) {
        throw new Error('Failed to fetch balance');
      }

      const balance = profile.credits_balance || profile.ruv_credits || 0;
      const lowBalance = balance < 20;

      return {
        success: true,
        balance: balance,
        autoRefillEnabled: profile.auto_refill_enabled,
        autoRefillThreshold: profile.auto_refill_threshold,
        message: chalk.cyan(`
╔════════════════════════════════════════════════════════════╗
║                    💰 CREDIT BALANCE                       ║
╠════════════════════════════════════════════════════════════╣
║ Current Balance: ${balance.toFixed(2).padEnd(42)} credits ║
║ Auto-Refill:     ${profile.auto_refill_enabled ? '✅ Enabled'.padEnd(43) : '❌ Disabled'.padEnd(43)}║
${profile.auto_refill_enabled ? `║ Refill at:       ${profile.auto_refill_threshold.toFixed(2).padEnd(42)} credits ║\n` : ''}╠════════════════════════════════════════════════════════════╣
${lowBalance ? 
`║ ⚠️  LOW BALANCE WARNING                                    ║
║ Consider adding credits to continue using services         ║
║ Run: flow pay 50  (to add $50 worth of credits)           ║` : 
`║ ✅ Balance is healthy                                      ║`}
╚════════════════════════════════════════════════════════════╝
        `),
      };
    } catch (error: any) {
      return {
        success: false,
        error: error.message,
        message: chalk.red(`❌ Failed to check balance: ${error.message}`),
      };
    }
  }

  /**
   * Enable or configure auto-refill
   */
  async configureAutoRefill(params: z.infer<typeof EnableAutoRefillSchema>) {
    try {
      const userId = params.userId || await this.getCurrentUserId();
      
      // Check if user has a payment method
      const { data: profile } = await this.supabase
        .from('profiles')
        .select('stripe_payment_method_id')
        .eq('id', userId)
        .single();

      if (params.enabled && !profile?.stripe_payment_method_id) {
        return {
          success: false,
          message: chalk.yellow(`
⚠️  No payment method on file
Please complete a payment first to save a payment method.
Run: flow pay 50
        `),
        };
      }

      // Update auto-refill settings
      const updates: any = {
        auto_refill_enabled: params.enabled,
        updated_at: new Date().toISOString(),
      };

      if (params.threshold !== undefined) {
        updates.auto_refill_threshold = params.threshold;
      }
      if (params.amount !== undefined) {
        updates.auto_refill_amount = params.amount;
      }

      const { error } = await this.supabase
        .from('profiles')
        .update(updates)
        .eq('id', userId);

      if (error) {
        throw new Error('Failed to update auto-refill settings');
      }

      return {
        success: true,
        message: chalk.green(`
╔════════════════════════════════════════════════════════════╗
║                 🔄 AUTO-REFILL CONFIGURED                  ║
╠════════════════════════════════════════════════════════════╣
║ Status:    ${params.enabled ? '✅ Enabled'.padEnd(49) : '❌ Disabled'.padEnd(49)}║
${params.threshold ? `║ Threshold: ${params.threshold.toFixed(2).padEnd(48)} credits ║\n` : ''}${params.amount ? `║ Amount:    $${params.amount.toFixed(2).padEnd(48)}║\n` : ''}╠════════════════════════════════════════════════════════════╣
║ ${params.enabled ? 
  '✅ Credits will auto-refill when balance is low           ' : 
  '❌ Auto-refill has been disabled                          '}║
╚════════════════════════════════════════════════════════════╝
        `),
      };
    } catch (error: any) {
      return {
        success: false,
        error: error.message,
        message: chalk.red(`❌ Failed to configure auto-refill: ${error.message}`),
      };
    }
  }

  /**
   * Get payment history
   */
  async getPaymentHistory(params: z.infer<typeof GetPaymentHistorySchema>) {
    try {
      const userId = params.userId || await this.getCurrentUserId();
      
      const { data: transactions, error } = await this.supabase
        .from('stripe_transactions')
        .select('*')
        .eq('user_id', userId)
        .order('created_at', { ascending: false })
        .limit(params.limit);

      if (error) {
        throw new Error('Failed to fetch payment history');
      }

      if (!transactions || transactions.length === 0) {
        return {
          success: true,
          transactions: [],
          message: chalk.yellow('No payment history found'),
        };
      }

      // Format transactions for display
      const formatted = transactions.map((t: any) => ({
        date: new Date(t.created_at).toLocaleDateString(),
        type: t.type,
        amount: `$${t.amount.toFixed(2)}`,
        credits: t.credits_purchased,
        status: t.status,
      }));

      const table = this.formatTable(formatted);

      return {
        success: true,
        transactions: formatted,
        message: chalk.cyan(`
╔════════════════════════════════════════════════════════════╗
║                   💳 PAYMENT HISTORY                       ║
╠════════════════════════════════════════════════════════════╣
${table}
╚════════════════════════════════════════════════════════════╝
        `),
      };
    } catch (error: any) {
      return {
        success: false,
        error: error.message,
        message: chalk.red(`❌ Failed to fetch payment history: ${error.message}`),
      };
    }
  }

  /**
   * Create a subscription
   */
  async createSubscription(params: {
    plan: 'starter' | 'pro' | 'enterprise';
    userId?: string;
  }) {
    try {
      const userId = params.userId || await this.getCurrentUserId();
      
      // Plan configuration
      const plans = {
        starter: { price: 29, credits: 1000, priceId: 'price_starter' },
        pro: { price: 99, credits: 5000, priceId: 'price_pro' },
        enterprise: { price: 299, credits: 20000, priceId: 'price_enterprise' },
      };

      const plan = plans[params.plan];

      // Get user profile
      const { data: profile } = await this.supabase
        .from('profiles')
        .select('stripe_customer_id, email')
        .eq('id', userId)
        .single();

      if (!profile?.stripe_customer_id) {
        return {
          success: false,
          message: chalk.yellow('Please complete a payment first to set up subscription'),
        };
      }

      // Create subscription
      const subscription = await this.stripe.subscriptions.create({
        customer: profile.stripe_customer_id,
        items: [{ price: plan.priceId }],
        payment_behavior: 'default_incomplete',
        payment_settings: { save_default_payment_method: 'on_subscription' },
        expand: ['latest_invoice.payment_intent'],
        metadata: {
          user_id: userId,
          credits_per_month: plan.credits.toString(),
        },
      });

      const clientSecret = (subscription.latest_invoice as any)?.payment_intent?.client_secret;

      return {
        success: true,
        subscriptionId: subscription.id,
        clientSecret: clientSecret,
        message: chalk.green(`
╔════════════════════════════════════════════════════════════╗
║                  📅 SUBSCRIPTION CREATED                   ║
╠════════════════════════════════════════════════════════════╣
║ Plan:     ${params.plan.toUpperCase().padEnd(50)}║
║ Price:    $${plan.price}/month${('').padEnd(40)}║
║ Credits:  ${plan.credits}/month${('').padEnd(40)}║
╠════════════════════════════════════════════════════════════╣
║ ✅ Complete payment to activate subscription               ║
╚════════════════════════════════════════════════════════════╝
        `),
      };
    } catch (error: any) {
      return {
        success: false,
        error: error.message,
        message: chalk.red(`❌ Failed to create subscription: ${error.message}`),
      };
    }
  }

  // Helper methods
  private calculateCredits(amount: number): number {
    const baseRate = 10; // 1 USD = 10 credits
    let credits = amount * baseRate;
    
    // Apply bonuses
    if (amount >= 100) {
      credits *= 1.1; // 10% bonus for $100+
    } else if (amount >= 500) {
      credits *= 1.2; // 20% bonus for $500+
    }
    
    return Math.round(credits * 100) / 100;
  }

  private async getCurrentUserId(): Promise<string> {
    // Get from auth context or session
    const { data: { user } } = await this.supabase.auth.getUser();
    if (!user) {
      throw new Error('Not authenticated');
    }
    return user.id;
  }

  private formatTable(data: any[]): string {
    if (data.length === 0) return '';
    
    const lines: string[] = [];
    lines.push('║ Date       │ Type         │ Amount    │ Credits   │ Status    ║');
    lines.push('╟────────────┼──────────────┼───────────┼───────────┼───────────╢');
    
    for (const row of data) {
      lines.push(
        `║ ${row.date.padEnd(10)} │ ${row.type.padEnd(12)} │ ${row.amount.padEnd(9)} │ ${
          row.credits ? row.credits.toString().padEnd(9) : '-'.padEnd(9)
        } │ ${row.status.padEnd(9)} ║`
      );
    }
    
    return lines.join('\n');
  }
}

// Export tool definitions for MCP
export const paymentTools = [
  {
    name: 'create_payment_link',
    description: 'Create a Stripe payment link for purchasing credits',
    inputSchema: CreatePaymentLinkSchema,
    handler: async (params: any, context: any) => {
      const tools = new PaymentTools(context);
      return tools.createPaymentLink(params);
    },
  },
  {
    name: 'check_credit_balance',
    description: 'Check current credit balance and auto-refill status',
    inputSchema: CheckBalanceSchema,
    handler: async (params: any, context: any) => {
      const tools = new PaymentTools(context);
      return tools.checkBalance(params);
    },
  },
  {
    name: 'configure_auto_refill',
    description: 'Enable or configure automatic credit refill',
    inputSchema: EnableAutoRefillSchema,
    handler: async (params: any, context: any) => {
      const tools = new PaymentTools(context);
      return tools.configureAutoRefill(params);
    },
  },
  {
    name: 'get_payment_history',
    description: 'Get recent payment and transaction history',
    inputSchema: GetPaymentHistorySchema,
    handler: async (params: any, context: any) => {
      const tools = new PaymentTools(context);
      return tools.getPaymentHistory(params);
    },
  },
  {
    name: 'create_subscription',
    description: 'Create a monthly subscription for automatic credits',
    inputSchema: z.object({
      plan: z.enum(['starter', 'pro', 'enterprise']),
      userId: z.string().optional(),
    }),
    handler: async (params: any, context: any) => {
      const tools = new PaymentTools(context);
      return tools.createSubscription(params);
    },
  },
];