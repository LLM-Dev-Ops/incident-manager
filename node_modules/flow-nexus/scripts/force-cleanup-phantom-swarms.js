#!/usr/bin/env node

/**
 * Force Cleanup Phantom Swarms
 * Removes swarms that appear in CLI but not in database
 */

import dotenv from 'dotenv';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load environment
dotenv.config({ path: join(__dirname, '../../../../.env') });

const phantomSwarmIds = [
  'swarm_1755963285423',
  'swarm_1755963259709',
  'swarm_1755963249341',
  'swarm_1755911406225',
  'swarm_1755911355784',
  'swarm_1755909604588',
  'swarm_1755905504126',
  'swarm_1755894415436',
  'swarm_1755890522666',
  'swarm_1755885065553',
  'swarm_1755885050389',
  'swarm_1755884838981'
];

async function cleanupPhantomSwarms() {
  console.log(chalk.red.bold('\n🔥 FORCE CLEANUP PHANTOM SWARMS\n'));
  
  const supabase = createClient(
    process.env.SUPABASE_URL,
    process.env.SUPABASE_ANON_KEY
  );
  
  for (const swarmId of phantomSwarmIds) {
    console.log(chalk.yellow(`\nProcessing ${swarmId}...`));
    
    try {
      // Try to insert as destroyed
      const { error: insertError } = await supabase
        .from('user_swarms')
        .insert({
          id: swarmId,
          user_id: 'a1b2c3d4-e5f6-7890-abcd-ef1234567890', // System user
          topology: 'mesh',
          max_agents: 0,
          status: 'destroyed',
          destroyed_at: new Date().toISOString(),
          metadata: { cleanup: 'phantom' }
        });
      
      if (!insertError) {
        console.log(chalk.green(`  ✅ Marked as destroyed`));
      } else {
        // Try to update if exists
        const { error: updateError } = await supabase
          .from('user_swarms')
          .update({
            status: 'destroyed',
            destroyed_at: new Date().toISOString()
          })
          .eq('id', swarmId);
        
        if (!updateError) {
          console.log(chalk.green(`  ✅ Updated to destroyed`));
        } else {
          console.log(chalk.gray(`  ⚠️ Not found in database`));
        }
      }
      
      // Clean up any agents
      await supabase
        .from('user_swarm_agents')
        .delete()
        .eq('swarm_id', swarmId);
      
      // Clean up any billing
      await supabase
        .from('user_swarm_billing')
        .update({ status: 'finalized' })
        .eq('swarm_id', swarmId);
      
    } catch (error) {
      console.error(chalk.red(`  ❌ Error: ${error.message}`));
    }
  }
  
  console.log(chalk.green.bold('\n✅ Phantom cleanup complete'));
  
  // Check current status
  const { data: activeSwarms } = await supabase
    .from('user_swarms')
    .select('id, status')
    .eq('status', 'active');
  
  console.log(chalk.cyan('\n📊 Current Status:'));
  console.log(chalk.gray(`  Active swarms in database: ${activeSwarms?.length || 0}`));
  
  if (activeSwarms && activeSwarms.length > 0) {
    activeSwarms.forEach(s => console.log(chalk.gray(`    - ${s.id}`)));
  }
}

// Run cleanup
cleanupPhantomSwarms()
  .then(() => {
    console.log(chalk.green('\n✨ Complete'));
    process.exit(0);
  })
  .catch(err => {
    console.error(chalk.red('Fatal:'), err);
    process.exit(1);
  });