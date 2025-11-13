#!/usr/bin/env node

/**
 * Cleanup Running Sandboxes Script
 * Pauses all running sandboxes to prevent billing overruns
 */

import { createClient } from '@supabase/supabase-js';
import SandboxLifecycleManager from '../src/services/sandbox-lifecycle-manager.js';
import chalk from 'chalk';
import ora from 'ora';
import dotenv from 'dotenv';

dotenv.config({ path: '/workspaces/flow-cloud/.env' });

const SUPABASE_URL = process.env.SUPABASE_URL;
const SUPABASE_SERVICE_KEY = process.env.SUPABASE_SERVICE_ROLE_KEY || process.env.SUPABASE_ANON_KEY;

async function cleanupSandboxes() {
  console.log(chalk.cyan.bold('\n🧹 Sandbox Cleanup & Billing Protection\n'));
  console.log(chalk.gray('=' .repeat(60)));
  
  const supabase = createClient(SUPABASE_URL, SUPABASE_SERVICE_KEY);
  const lifecycleManager = new SandboxLifecycleManager();
  
  try {
    // Get all active sandboxes
    const spinner = ora('Fetching active sandboxes...').start();
    
    const { data: activeSandboxes, error } = await supabase
      .from('sandbox_evaluations')
      .select(`
        *,
        sandbox_billing!inner(*)
      `)
      .eq('sandbox_status', 'running');
    
    if (error) {
      spinner.fail('Failed to fetch sandboxes');
      throw error;
    }
    
    spinner.succeed(`Found ${activeSandboxes?.length || 0} active sandboxes`);
    
    if (!activeSandboxes || activeSandboxes.length === 0) {
      console.log(chalk.green('✅ No active sandboxes found'));
      return;
    }
    
    console.log(chalk.yellow('\n📊 Active Sandboxes:\n'));
    
    let totalCost = 0;
    let totalRuntime = 0;
    
    for (const sandbox of activeSandboxes) {
      const billing = sandbox.sandbox_billing[0];
      const runtimeMinutes = Math.floor(
        (Date.now() - new Date(sandbox.started_at).getTime()) / 60000
      );
      const cost = (runtimeMinutes / 60) * 3; // 3 rUv per hour
      
      totalRuntime += runtimeMinutes;
      totalCost += cost;
      
      console.log(chalk.cyan(`  Sandbox: ${sandbox.sandbox_id}`));
      console.log(chalk.gray(`    Submission: ${sandbox.submission_id}`));
      console.log(chalk.gray(`    Runtime: ${runtimeMinutes} minutes`));
      console.log(chalk.gray(`    Cost: ${cost.toFixed(2)} rUv`));
      console.log(chalk.gray(`    Status: ${sandbox.sandbox_status}`));
      
      // Check if should be paused
      if (runtimeMinutes > 60) {
        console.log(chalk.yellow(`    ⚠️  Running for over 1 hour!`));
      }
      
      if (runtimeMinutes > 24 * 60) {
        console.log(chalk.red(`    ❌ Exceeded 24-hour limit!`));
      }
    }
    
    console.log(chalk.yellow('\n💰 Billing Summary:'));
    console.log(chalk.gray(`  Total Sandboxes: ${activeSandboxes.length}`));
    console.log(chalk.gray(`  Total Runtime: ${totalRuntime} minutes`));
    console.log(chalk.gray(`  Total Cost: ${totalCost.toFixed(2)} rUv`));
    console.log(chalk.gray(`  Hourly Rate: 3 rUv/hour`));
    
    // Ask for confirmation
    console.log(chalk.yellow('\n⚠️  Action Required:'));
    console.log(chalk.gray('  These sandboxes will be paused to prevent further charges.'));
    console.log(chalk.gray('  State will be preserved for judge review.'));
    
    const pauseSpinner = ora('Pausing sandboxes...').start();
    
    let pausedCount = 0;
    let failedCount = 0;
    
    for (const sandbox of activeSandboxes) {
      try {
        // Pause the sandbox
        await lifecycleManager.pauseSandbox(
          sandbox.sandbox_id,
          'Manual cleanup - billing protection'
        );
        
        // Preserve state for review
        if (sandbox.submission_id) {
          await lifecycleManager.preserveStateForReview(
            sandbox.sandbox_id,
            sandbox.submission_id
          );
        }
        
        pausedCount++;
      } catch (err) {
        console.error(chalk.red(`Failed to pause ${sandbox.sandbox_id}:`, err.message));
        failedCount++;
      }
    }
    
    pauseSpinner.succeed(`Paused ${pausedCount} sandboxes`);
    
    if (failedCount > 0) {
      console.log(chalk.red(`❌ Failed to pause ${failedCount} sandboxes`));
    }
    
    // Clean up old sandboxes
    console.log(chalk.cyan('\n🗑️ Cleaning up old sandboxes...'));
    
    const { data: oldSandboxes, error: oldError } = await supabase
      .from('sandbox_evaluations')
      .update({
        sandbox_status: 'terminated',
        pause_reason: 'Cleanup - 48hr limit'
      })
      .in('sandbox_status', ['running', 'paused'])
      .lt('started_at', new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString())
      .select();
    
    if (oldSandboxes && oldSandboxes.length > 0) {
      console.log(chalk.green(`✅ Terminated ${oldSandboxes.length} old sandboxes`));
    }
    
    // Generate cost report
    console.log(chalk.cyan('\n📊 Cost Report:\n'));
    
    const { data: costData } = await supabase
      .from('sandbox_billing')
      .select('user_id, SUM(total_cost_ruv), COUNT(*)')
      .group('user_id')
      .order('SUM(total_cost_ruv)', { ascending: false })
      .limit(10);
    
    if (costData && costData.length > 0) {
      console.log(chalk.gray('Top 10 Users by Sandbox Costs:'));
      costData.forEach((user, index) => {
        console.log(chalk.gray(`  ${index + 1}. User ${user.user_id}: ${user.sum} rUv`));
      });
    }
    
    console.log(chalk.green('\n✅ Cleanup complete!'));
    console.log(chalk.cyan('\nRecommendations:'));
    console.log(chalk.gray('  1. Enable automatic monitoring with lifecycle manager'));
    console.log(chalk.gray('  2. Set up pg_cron for scheduled checks'));
    console.log(chalk.gray('  3. Configure alerts for long-running sandboxes'));
    console.log(chalk.gray('  4. Review billing policies with users'));
    
  } catch (error) {
    console.error(chalk.red('\n❌ Cleanup failed:'), error);
    process.exit(1);
  }
}

// Run cleanup
cleanupSandboxes()
  .then(() => {
    console.log(chalk.green('\n✨ All sandboxes processed'));
    process.exit(0);
  })
  .catch(err => {
    console.error(chalk.red('Fatal error:'), err);
    process.exit(1);
  });