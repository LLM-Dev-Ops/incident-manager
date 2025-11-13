#!/usr/bin/env node

/**
 * Clear Phantom Swarm Cache
 * Forces cleanup of cached swarm data
 */

import dotenv from 'dotenv';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';
import fs from 'fs/promises';
import path from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load environment
dotenv.config({ path: join(__dirname, '../../../../.env') });

async function clearPhantomCache() {
  console.log(chalk.cyan.bold('\n🧹 CLEARING PHANTOM SWARM CACHE\n'));
  
  // 1. Check local session files
  const homeDir = process.env.HOME || process.env.USERPROFILE;
  const flowNexusDir = path.join(homeDir, '.flow-nexus');
  
  try {
    // Check if .flow-nexus directory exists
    const dirExists = await fs.access(flowNexusDir).then(() => true).catch(() => false);
    
    if (dirExists) {
      console.log(chalk.yellow('Found .flow-nexus directory'));
      
      // List files
      const files = await fs.readdir(flowNexusDir);
      console.log(chalk.gray('  Files:', files.join(', ')));
      
      // Look for any cache files
      for (const file of files) {
        if (file.includes('cache') || file.includes('swarm') || file.includes('session')) {
          const filePath = path.join(flowNexusDir, file);
          console.log(chalk.yellow(`  Removing ${file}...`));
          await fs.unlink(filePath).catch(() => {});
        }
      }
    }
  } catch (error) {
    console.log(chalk.gray('  No local cache directory found'));
  }
  
  // 2. Create Supabase client with service role
  const supabase = createClient(
    process.env.SUPABASE_URL,
    process.env.SUPABASE_SERVICE_ROLE_KEY || process.env.SUPABASE_ANON_KEY
  );
  
  // 3. Force insert phantom swarms as destroyed
  const phantomIds = [
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
  
  console.log(chalk.yellow('\n📝 Registering phantom swarms as destroyed...'));
  
  for (const swarmId of phantomIds) {
    try {
      // Try to upsert the swarm as destroyed
      const { error } = await supabase
        .from('user_swarms')
        .upsert({
          id: swarmId,
          user_id: 'a1b2c3d4-e5f6-7890-abcd-ef1234567890', // System user
          topology: 'mesh',
          max_agents: 0,
          status: 'destroyed',
          destroyed_at: new Date().toISOString(),
          created_at: new Date().toISOString(),
          metadata: { 
            cleanup: 'phantom',
            note: 'Phantom swarm from cached session'
          }
        }, {
          onConflict: 'id'
        });
      
      if (!error) {
        console.log(chalk.green(`  ✅ ${swarmId} marked as destroyed`));
      } else {
        console.log(chalk.gray(`  ⚠️ ${swarmId}: ${error.message}`));
      }
    } catch (err) {
      console.log(chalk.gray(`  ⚠️ ${swarmId}: ${err.message}`));
    }
  }
  
  // 4. Clean up any orphaned records
  console.log(chalk.yellow('\n🧹 Cleaning up orphaned records...'));
  
  // Clean up agents
  for (const swarmId of phantomIds) {
    await supabase
      .from('user_swarm_agents')
      .delete()
      .eq('swarm_id', swarmId);
  }
  
  // Clean up billing
  for (const swarmId of phantomIds) {
    await supabase
      .from('user_swarm_billing')
      .update({ 
        status: 'finalized',
        end_time: new Date().toISOString()
      })
      .eq('swarm_id', swarmId);
  }
  
  // 5. Check current database state
  console.log(chalk.cyan('\n📊 Database Status:'));
  
  const { data: activeSwarms } = await supabase
    .from('user_swarms')
    .select('id, status')
    .eq('status', 'active');
  
  const { data: allSwarms } = await supabase
    .from('user_swarms')
    .select('id, status')
    .in('id', phantomIds);
  
  console.log(chalk.gray(`  Active swarms: ${activeSwarms?.length || 0}`));
  console.log(chalk.gray(`  Phantom swarms in DB: ${allSwarms?.length || 0}`));
  
  if (allSwarms && allSwarms.length > 0) {
    allSwarms.forEach(s => {
      console.log(chalk.gray(`    - ${s.id}: ${s.status}`));
    });
  }
  
  // 6. Clear any local Node.js cache
  console.log(chalk.yellow('\n🔄 Clearing Node.js module cache...'));
  
  // Clear require cache for supabase-client
  const modulePath = path.join(__dirname, '../src/services/supabase-client.js');
  delete require.cache[modulePath];
  
  console.log(chalk.green.bold('\n✅ Cache clearing complete!'));
  console.log(chalk.cyan('\nNext steps:'));
  console.log(chalk.gray('  1. Restart your terminal/codespace'));
  console.log(chalk.gray('  2. Run: flow-nexus auth logout'));
  console.log(chalk.gray('  3. Run: flow-nexus auth login'));
  console.log(chalk.gray('  4. Check: flow-nexus swarm list'));
  console.log(chalk.gray('\nThe phantom swarms should be gone.'));
}

// Run cleanup
clearPhantomCache()
  .then(() => {
    console.log(chalk.green('\n✨ Done'));
    process.exit(0);
  })
  .catch(err => {
    console.error(chalk.red('Fatal:'), err);
    process.exit(1);
  });