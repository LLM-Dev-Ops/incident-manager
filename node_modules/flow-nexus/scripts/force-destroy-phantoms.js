#!/usr/bin/env node

/**
 * Force Destroy Phantom Swarms
 * Inserts phantom swarms into database as destroyed
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

const PHANTOM_SWARMS = [
  { id: 'swarm_1755963285423', topology: 'mesh', agents: 3 },
  { id: 'swarm_1755963259709', topology: 'mesh', agents: 2 },
  { id: 'swarm_1755963249341', topology: 'mesh', agents: 2 },
  { id: 'swarm_1755911406225', topology: 'mesh', agents: 8 },
  { id: 'swarm_1755911355784', topology: 'mesh', agents: 8 },
  { id: 'swarm_1755909604588', topology: 'mesh', agents: 8 },
  { id: 'swarm_1755905504126', topology: 'mesh', agents: 8 },
  { id: 'swarm_1755894415436', topology: 'mesh', agents: 2 },
  { id: 'swarm_1755890522666', topology: 'mesh', agents: 1 },
  { id: 'swarm_1755885065553', topology: 'star', agents: 5 },
  { id: 'swarm_1755885050389', topology: 'ring', agents: 3 },
  { id: 'swarm_1755884838981', topology: 'hierarchical', agents: 4 }
];

async function forceDestroyPhantoms() {
  console.log(chalk.red.bold('\n🔥 FORCE DESTROY PHANTOM SWARMS\n'));
  
  const supabase = createClient(
    process.env.SUPABASE_URL,
    process.env.SUPABASE_SERVICE_ROLE_KEY || process.env.SUPABASE_ANON_KEY
  );
  
  // Get current user
  const { data: { user } } = await supabase.auth.getUser();
  const userId = user?.id || '54fd58c0-d5d9-403b-abd5-740bd3e99758'; // Use the user ID from the list command
  
  console.log(chalk.yellow(`Using user ID: ${userId}\n`));
  
  for (const phantom of PHANTOM_SWARMS) {
    console.log(chalk.yellow(`Processing ${phantom.id}...`));
    
    try {
      // First, try to insert as active
      const { error: insertError } = await supabase
        .from('user_swarms')
        .insert({
          id: phantom.id,
          user_id: userId,
          topology: phantom.topology,
          max_agents: phantom.agents,
          status: 'active',
          strategy: 'adaptive',
          created_at: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(), // 24 hours ago
          metadata: { phantom: true }
        });
      
      if (!insertError) {
        console.log(chalk.green(`  ✅ Inserted as active`));
        
        // Now destroy it properly
        const { error: updateError } = await supabase
          .from('user_swarms')
          .update({
            status: 'destroyed',
            metadata: { 
              phantom: true, 
              destroyed: new Date().toISOString(),
              reason: 'Phantom cleanup'
            }
          })
          .eq('id', phantom.id);
        
        if (!updateError) {
          console.log(chalk.green(`  ✅ Marked as destroyed`));
        }
      } else {
        // Try to update if exists
        const { error: updateError } = await supabase
          .from('user_swarms')
          .update({
            status: 'destroyed',
            metadata: { 
              phantom: true, 
              destroyed: new Date().toISOString(),
              reason: 'Phantom cleanup'
            }
          })
          .eq('id', phantom.id);
        
        if (!updateError) {
          console.log(chalk.green(`  ✅ Updated to destroyed`));
        } else {
          console.log(chalk.gray(`  ⚠️ ${insertError.message}`));
        }
      }
      
      // Clean up any agents
      await supabase
        .from('user_swarm_agents')
        .delete()
        .eq('swarm_id', phantom.id);
      
    } catch (error) {
      console.error(chalk.red(`  ❌ Error: ${error.message}`));
    }
  }
  
  console.log(chalk.cyan('\n📊 Final Status:'));
  
  // Check active swarms
  const { data: activeSwarms } = await supabase
    .from('user_swarms')
    .select('id, status')
    .eq('user_id', userId)
    .eq('status', 'active');
  
  // Check destroyed swarms
  const { data: destroyedSwarms } = await supabase
    .from('user_swarms')
    .select('id, status')
    .eq('user_id', userId)
    .eq('status', 'destroyed');
  
  console.log(chalk.gray(`  Active swarms: ${activeSwarms?.length || 0}`));
  console.log(chalk.gray(`  Destroyed swarms: ${destroyedSwarms?.length || 0}`));
  
  if (activeSwarms && activeSwarms.length > 0) {
    console.log(chalk.yellow('\n  Still active:'));
    activeSwarms.forEach(s => console.log(chalk.gray(`    - ${s.id}`)));
  }
  
  console.log(chalk.green.bold('\n✅ Phantom cleanup complete!'));
  console.log(chalk.cyan('\nNow run: flow-nexus swarm list'));
  console.log(chalk.gray('It should show 0 active swarms.'));
}

// Run
forceDestroyPhantoms()
  .then(() => process.exit(0))
  .catch(err => {
    console.error(chalk.red('Fatal:'), err);
    process.exit(1);
  });