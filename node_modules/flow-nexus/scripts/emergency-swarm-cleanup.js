#!/usr/bin/env node

/**
 * Emergency Swarm Cleanup Script
 * Destroys all active swarms and terminates their sandboxes
 */

import dotenv from 'dotenv';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import SwarmCleanupService from '../src/services/swarm-cleanup-service.js';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load environment variables from root
dotenv.config({ path: join(__dirname, '../../../../.env') });

async function main() {
  console.log(chalk.red.bold('\n🚨 EMERGENCY SWARM CLEANUP\n'));
  console.log(chalk.yellow('This will destroy ALL active swarms and terminate their sandboxes.'));
  console.log(chalk.yellow('This action cannot be undone.\n'));
  
  // Verify environment
  const supabaseKey = process.env.SUPABASE_SERVICE_ROLE_KEY || process.env.SUPABASE_ANON_KEY;
  if (!process.env.SUPABASE_URL || !supabaseKey) {
    console.error(chalk.red('❌ Missing required environment variables'));
    console.log(chalk.gray('Please ensure SUPABASE_URL and SUPABASE_ANON_KEY/SERVICE_ROLE_KEY are set'));
    console.log(chalk.gray('Current env:'));
    console.log(chalk.gray(`  SUPABASE_URL: ${process.env.SUPABASE_URL ? 'Set' : 'Missing'}`));
    console.log(chalk.gray(`  SUPABASE_ANON_KEY: ${process.env.SUPABASE_ANON_KEY ? 'Set' : 'Missing'}`));
    console.log(chalk.gray(`  SUPABASE_SERVICE_ROLE_KEY: ${process.env.SUPABASE_SERVICE_ROLE_KEY ? 'Set' : 'Missing'}`));
    process.exit(1);
  }
  
  try {
    const cleanupService = new SwarmCleanupService();
    
    console.log(chalk.cyan('Starting cleanup process...\n'));
    
    const results = await cleanupService.forceDestroyAllActiveSwarms();
    
    console.log(chalk.green.bold('\n✅ Cleanup Complete!'));
    process.exit(0);
  } catch (error) {
    console.error(chalk.red('\n❌ Cleanup failed:'), error);
    process.exit(1);
  }
}

// Run the cleanup
main().catch(err => {
  console.error(chalk.red('Fatal error:'), err);
  process.exit(1);
});