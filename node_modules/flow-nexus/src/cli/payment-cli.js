#!/usr/bin/env node

/**
 * Payment CLI Commands
 * Provides command-line interface for Stripe payments and credit management
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import prompts from 'prompts';
import { PaymentTools } from '../tools/payment-tools.js';
import { createClient } from '@supabase/supabase-js';

// Initialize Supabase client
const supabase = createClient(
  process.env.SUPABASE_URL || 'https://pklhxiuouhrcrreectbo.supabase.co',
  process.env.SUPABASE_ANON_KEY || ''
);

// Initialize payment tools
const paymentTools = new PaymentTools({
  stripeSecretKey: process.env.STRIPE_SECRET_KEY,
  supabase: supabase,
  baseUrl: process.env.BASE_URL || 'https://flow.ruv.net',
});

const program = new Command();

program
  .name('flow-pay')
  .description('Flow Cloud payment and credit management')
  .version('1.0.0');

// Pay command - create payment link
program
  .command('pay [amount]')
  .description('Create a payment link to purchase credits')
  .option('-t, --type <type>', 'Payment type (deposit/subscription/credits)', 'deposit')
  .option('-r, --return <url>', 'Return URL after payment')
  .action(async (amount, options) => {
    const spinner = ora('Creating payment link...').start();
    
    try {
      // If no amount provided, prompt for it
      if (!amount) {
        spinner.stop();
        const response = await prompts({
          type: 'number',
          name: 'amount',
          message: 'Enter amount in USD (minimum $10):',
          min: 10,
          validate: value => value >= 10 ? true : 'Minimum amount is $10',
        });
        amount = response.amount;
        
        if (!amount) {
          console.log(chalk.yellow('Payment cancelled'));
          return;
        }
        
        spinner.start('Creating payment link...');
      }

      const result = await paymentTools.createPaymentLink({
        amount: parseFloat(amount),
        type: options.type,
        returnUrl: options.return,
      });

      spinner.stop();
      
      if (result.success) {
        console.log(result.message);
        
        // Copy to clipboard if possible
        if (process.platform === 'darwin') {
          const { exec } = await import('child_process');
          exec(`echo "${result.paymentUrl}" | pbcopy`);
          console.log(chalk.green('✅ Payment link copied to clipboard!'));
        }
      } else {
        console.log(result.message);
      }
    } catch (error) {
      spinner.stop();
      console.error(chalk.red('Error:', error.message));
    }
  });

// Balance command
program
  .command('balance')
  .alias('bal')
  .description('Check your current credit balance')
  .action(async () => {
    const spinner = ora('Checking balance...').start();
    
    try {
      const result = await paymentTools.checkBalance({});
      spinner.stop();
      console.log(result.message);
      
      // If balance is low, suggest adding credits
      if (result.balance < 20) {
        const response = await prompts({
          type: 'confirm',
          name: 'addCredits',
          message: 'Would you like to add credits now?',
          initial: true,
        });
        
        if (response.addCredits) {
          const amountResponse = await prompts({
            type: 'select',
            name: 'amount',
            message: 'Select amount:',
            choices: [
              { title: '$10 (100 credits)', value: 10 },
              { title: '$25 (250 credits)', value: 25 },
              { title: '$50 (500 credits)', value: 50 },
              { title: '$100 (1100 credits - 10% bonus)', value: 100 },
              { title: 'Custom amount', value: 'custom' },
            ],
          });
          
          let amount = amountResponse.amount;
          if (amount === 'custom') {
            const customResponse = await prompts({
              type: 'number',
              name: 'amount',
              message: 'Enter custom amount (USD):',
              min: 10,
            });
            amount = customResponse.amount;
          }
          
          if (amount && amount !== 'custom') {
            const paymentResult = await paymentTools.createPaymentLink({
              amount: amount,
              type: 'deposit',
            });
            console.log(paymentResult.message);
          }
        }
      }
    } catch (error) {
      spinner.stop();
      console.error(chalk.red('Error:', error.message));
    }
  });

// Auto-refill command
program
  .command('auto-refill')
  .alias('refill')
  .description('Configure automatic credit refill')
  .option('-e, --enable', 'Enable auto-refill')
  .option('-d, --disable', 'Disable auto-refill')
  .option('-t, --threshold <amount>', 'Set refill threshold (credits)', parseFloat)
  .option('-a, --amount <amount>', 'Set refill amount (USD)', parseFloat)
  .action(async (options) => {
    const spinner = ora('Configuring auto-refill...').start();
    
    try {
      // If no options provided, show interactive menu
      if (!options.enable && !options.disable && !options.threshold && !options.amount) {
        spinner.stop();
        
        const response = await prompts([
          {
            type: 'toggle',
            name: 'enabled',
            message: 'Enable auto-refill?',
            initial: true,
            active: 'yes',
            inactive: 'no',
          },
          {
            type: prev => prev ? 'number' : null,
            name: 'threshold',
            message: 'Refill when balance drops below (credits):',
            initial: 20,
            min: 10,
          },
          {
            type: prev => prev ? 'number' : null,
            name: 'amount',
            message: 'Amount to refill (USD):',
            initial: 50,
            min: 10,
          },
        ]);
        
        if (response.enabled !== undefined) {
          spinner.start('Updating auto-refill settings...');
          const result = await paymentTools.configureAutoRefill({
            enabled: response.enabled,
            threshold: response.threshold,
            amount: response.amount,
          });
          spinner.stop();
          console.log(result.message);
        }
      } else {
        const result = await paymentTools.configureAutoRefill({
          enabled: options.enable || !options.disable,
          threshold: options.threshold,
          amount: options.amount,
        });
        spinner.stop();
        console.log(result.message);
      }
    } catch (error) {
      spinner.stop();
      console.error(chalk.red('Error:', error.message));
    }
  });

// History command
program
  .command('history')
  .description('View payment history')
  .option('-l, --limit <number>', 'Number of transactions to show', '10')
  .action(async (options) => {
    const spinner = ora('Loading payment history...').start();
    
    try {
      const result = await paymentTools.getPaymentHistory({
        limit: parseInt(options.limit),
      });
      spinner.stop();
      console.log(result.message);
    } catch (error) {
      spinner.stop();
      console.error(chalk.red('Error:', error.message));
    }
  });

// Subscribe command
program
  .command('subscribe [plan]')
  .description('Create a monthly subscription (starter/pro/enterprise)')
  .action(async (plan) => {
    const spinner = ora('Setting up subscription...').start();
    
    try {
      // If no plan provided, show interactive menu
      if (!plan) {
        spinner.stop();
        
        const response = await prompts({
          type: 'select',
          name: 'plan',
          message: 'Select subscription plan:',
          choices: [
            { 
              title: 'Starter - $29/month (1,000 credits)', 
              value: 'starter',
              description: 'Perfect for individuals and small projects',
            },
            { 
              title: 'Pro - $99/month (5,000 credits)', 
              value: 'pro',
              description: 'Great for teams and growing applications',
            },
            { 
              title: 'Enterprise - $299/month (20,000 credits)', 
              value: 'enterprise',
              description: 'For large-scale deployments',
            },
          ],
        });
        
        plan = response.plan;
        
        if (!plan) {
          console.log(chalk.yellow('Subscription cancelled'));
          return;
        }
        
        // Confirm subscription
        const confirm = await prompts({
          type: 'confirm',
          name: 'confirmed',
          message: `Subscribe to ${plan.toUpperCase()} plan?`,
          initial: true,
        });
        
        if (!confirm.confirmed) {
          console.log(chalk.yellow('Subscription cancelled'));
          return;
        }
        
        spinner.start('Creating subscription...');
      }

      const result = await paymentTools.createSubscription({
        plan: plan as 'starter' | 'pro' | 'enterprise',
      });
      
      spinner.stop();
      console.log(result.message);
      
      if (result.success && result.clientSecret) {
        console.log(chalk.cyan('\n📱 Complete subscription setup at:'));
        console.log(chalk.cyan(`${process.env.BASE_URL || 'https://flow.ruv.net'}/subscribe?secret=${result.clientSecret}`));
      }
    } catch (error) {
      spinner.stop();
      console.error(chalk.red('Error:', error.message));
    }
  });

// Quick pay shortcuts
program
  .command('pay10')
  .description('Quick payment for $10 (100 credits)')
  .action(async () => {
    const result = await paymentTools.createPaymentLink({
      amount: 10,
      type: 'deposit',
    });
    console.log(result.message);
  });

program
  .command('pay25')
  .description('Quick payment for $25 (250 credits)')
  .action(async () => {
    const result = await paymentTools.createPaymentLink({
      amount: 25,
      type: 'deposit',
    });
    console.log(result.message);
  });

program
  .command('pay50')
  .description('Quick payment for $50 (500 credits)')
  .action(async () => {
    const result = await paymentTools.createPaymentLink({
      amount: 50,
      type: 'deposit',
    });
    console.log(result.message);
  });

program
  .command('pay100')
  .description('Quick payment for $100 (1100 credits - 10% bonus)')
  .action(async () => {
    const result = await paymentTools.createPaymentLink({
      amount: 100,
      type: 'deposit',
    });
    console.log(result.message);
  });

// Help text
program.on('--help', () => {
  console.log('');
  console.log('Examples:');
  console.log('  $ flow pay 50                    # Create payment link for $50');
  console.log('  $ flow balance                    # Check credit balance');
  console.log('  $ flow auto-refill --enable       # Enable auto-refill');
  console.log('  $ flow history                    # View payment history');
  console.log('  $ flow subscribe pro              # Subscribe to Pro plan');
  console.log('');
  console.log('Quick payments:');
  console.log('  $ flow pay10                      # Pay $10 (100 credits)');
  console.log('  $ flow pay25                      # Pay $25 (250 credits)');
  console.log('  $ flow pay50                      # Pay $50 (500 credits)');
  console.log('  $ flow pay100                     # Pay $100 (1100 credits)');
});

// Parse arguments
program.parse(process.argv);

// Show help if no command provided
if (!process.argv.slice(2).length) {
  program.outputHelp();
}