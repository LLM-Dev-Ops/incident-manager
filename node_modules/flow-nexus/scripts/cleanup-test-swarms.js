#!/usr/bin/env node

/**
 * Cleanup Test Swarms
 * Destroys all test swarms created during development
 */

import { createClient } from '@supabase/supabase-js';
import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import chalk from 'chalk';
import SwarmCleanupService from '../src/services/swarm-cleanup-service.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load environment
dotenv.config({ path: path.join(__dirname, '../../../.env') });

async function cleanupTestSwarms() {
  console.log(chalk.blue.bold('\n🧹 Cleaning Up Test Swarms\n'));
  
  const supabase = createClient(
    process.env.SUPABASE_URL,
    process.env.SUPABASE_ANON_KEY
  );
  
  // Get all active swarms for the test user
  const TEST_USER_ID = '54fd58c0-d5d9-403b-abd5-740bd3e99758';
  
  console.log(chalk.yellow('Fetching active swarms...'));
  
  const { data: swarms, error } = await supabase
    .from('user_swarms')
    .select('*')
    .eq('user_id', TEST_USER_ID)
    .eq('status', 'active');
  
  if (error) {
    console.error(chalk.red('Failed to fetch swarms:'), error);
    return;
  }
  
  console.log(chalk.cyan(`Found ${swarms.length} active swarms\n`));
  
  const cleanupService = new SwarmCleanupService();
  
  let destroyed = 0;
  let failed = 0;
  
  for (const swarm of swarms) {
    console.log(chalk.yellow(`Destroying ${swarm.id}...`));
    
    try {
      const result = await cleanupService.destroySwarm(swarm.id, TEST_USER_ID);
      
      if (result.errors.length === 0) {
        console.log(chalk.green(`  ✅ Destroyed successfully`));
        console.log(chalk.gray(`     Sandboxes: ${result.sandboxesTerminated}`));
        console.log(chalk.gray(`     Cost: ${result.finalCost?.toFixed(2) || 0} rUv`));
        destroyed++;
      } else {
        console.log(chalk.yellow(`  ⚠️ Destroyed with warnings:`));
        result.errors.forEach(err => console.log(chalk.gray(`     - ${err}`)));
        destroyed++;
      }
    } catch (err) {
      console.log(chalk.red(`  ❌ Failed: ${err.message}`));
      failed++;
    }
  }
  
  console.log(chalk.blue('\n📊 Cleanup Summary:'));
  console.log(chalk.green(`  Destroyed: ${destroyed}`));
  console.log(chalk.red(`  Failed: ${failed}`));
  
  // Verify final state
  const { data: remaining } = await supabase
    .from('user_swarms')
    .select('id')
    .eq('user_id', TEST_USER_ID)
    .eq('status', 'active');
  
  console.log(chalk.cyan(`  Remaining active: ${remaining?.length || 0}`));
  
  if (remaining && remaining.length === 0) {
    console.log(chalk.green.bold('\n✅ All test swarms cleaned up!'));
    console.log(chalk.gray('No more billing for test swarms.'));
  } else {
    console.log(chalk.yellow(`\n⚠️ ${remaining?.length} swarms still active`));
    console.log(chalk.gray('Run again or check manually.'));
  }
}

// Run cleanup
cleanupTestSwarms()
  .then(() => process.exit(0))
  .catch(err => {
    console.error(chalk.red('Fatal error:'), err);
    process.exit(1);
  });