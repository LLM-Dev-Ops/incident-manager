/**
 * Swarm Cleanup Service
 * Properly destroys swarms, terminates all sandboxes, and stops billing
 */

import { createClient } from '@supabase/supabase-js';
import { Sandbox } from 'e2b';
import chalk from 'chalk';

class SwarmCleanupService {
  constructor() {
    this.supabase = createClient(
      process.env.SUPABASE_URL,
      process.env.SUPABASE_SERVICE_ROLE_KEY || process.env.SUPABASE_ANON_KEY
    );
    this.e2bApiKey = process.env.E2B_API_KEY;
  }

  /**
   * Completely destroy a swarm and all its resources
   */
  async destroySwarm(swarmId, userId = null) {
    console.log(chalk.yellow(`\n🔥 Destroying swarm ${swarmId}...`));
    
    const results = {
      swarmId,
      sandboxesTerminated: 0,
      agentsRemoved: 0,
      billingFinalized: false,
      databaseUpdated: false,
      errors: []
    };

    try {
      // 1. Get swarm details
      const { data: swarm, error: swarmError } = await this.supabase
        .from('user_swarms')
        .select('*, user_swarm_agents(*)')
        .eq('id', swarmId)
        .single();

      if (swarmError || !swarm) {
        throw new Error(`Swarm not found: ${swarmId}`);
      }

      // 2. Terminate all E2B sandboxes
      if (swarm.user_swarm_agents && swarm.user_swarm_agents.length > 0) {
        console.log(chalk.cyan(`  Terminating ${swarm.user_swarm_agents.length} agent sandboxes...`));
        
        for (const agent of swarm.user_swarm_agents) {
          try {
            if (agent.sandbox_id) {
              // Try to terminate via E2B API
              if (this.e2bApiKey) {
                try {
                  const sandbox = await Sandbox.connect(agent.sandbox_id, { apiKey: this.e2bApiKey });
                  await sandbox.close();
                  console.log(chalk.green(`    ✅ Terminated sandbox ${agent.sandbox_id}`));
                } catch (e2bError) {
                  // Sandbox might already be terminated
                  console.log(chalk.gray(`    ⚠️  Sandbox ${agent.sandbox_id} already terminated or not found`));
                }
              }
              
              // Update sandbox evaluation record
              await this.supabase
                .from('sandbox_evaluations')
                .update({
                  sandbox_status: 'terminated',
                  paused_at: new Date().toISOString(),
                  pause_reason: 'Swarm destroyed',
                  auto_paused: true
                })
                .eq('sandbox_id', agent.sandbox_id);
              
              // Update sandbox billing
              await this.supabase
                .from('sandbox_billing')
                .update({
                  status: 'terminated',
                  end_time: new Date().toISOString(),
                  charged_to_user: false // Don't charge after manual destruction
                })
                .eq('sandbox_id', agent.sandbox_id);
              
              results.sandboxesTerminated++;
            }
          } catch (sandboxError) {
            results.errors.push(`Failed to terminate sandbox ${agent.sandbox_id}: ${sandboxError.message}`);
          }
        }
      }

      // 3. Calculate final billing
      try {
        const runtime = swarm.runtime_minutes || 0;
        const hourlyRate = swarm.hourly_rate || 3;
        const totalCost = (runtime / 60) * hourlyRate;
        
        // Update swarm billing record
        await this.supabase
          .from('user_swarm_billing')
          .update({
            status: 'finalized',
            end_time: new Date().toISOString(),
            total_runtime_minutes: runtime,
            total_cost: totalCost,
            finalized_at: new Date().toISOString()
          })
          .eq('swarm_id', swarmId);
        
        results.billingFinalized = true;
        results.finalCost = totalCost;
        
        console.log(chalk.cyan(`  💰 Final billing: ${totalCost.toFixed(2)} rUv for ${runtime} minutes`));
      } catch (billingError) {
        results.errors.push(`Billing finalization failed: ${billingError.message}`);
      }

      // 4. Delete agent records
      const { error: agentDeleteError } = await this.supabase
        .from('user_swarm_agents')
        .delete()
        .eq('swarm_id', swarmId);

      if (!agentDeleteError) {
        results.agentsRemoved = swarm.user_swarm_agents?.length || 0;
        console.log(chalk.green(`  ✅ Removed ${results.agentsRemoved} agent records`));
      } else {
        results.errors.push(`Failed to delete agents: ${agentDeleteError.message}`);
      }

      // 5. Update swarm status to destroyed
      const { error: updateError } = await this.supabase
        .from('user_swarms')
        .update({
          status: 'destroyed',
          destroyed_at: new Date().toISOString(),
          metadata: {
            ...(swarm.metadata || {}),
            destruction_reason: 'Manual destroy',
            final_agent_count: swarm.user_swarm_agents?.length || 0,
            final_runtime_minutes: swarm.runtime_minutes || 0
          }
        })
        .eq('id', swarmId);

      if (!updateError) {
        results.databaseUpdated = true;
        console.log(chalk.green(`  ✅ Swarm status updated to 'destroyed'`));
      } else {
        results.errors.push(`Failed to update swarm status: ${updateError.message}`);
      }

      // 6. Log the destruction
      await this.supabase
        .from('system_logs')
        .insert({
          log_type: 'swarm_destroyed',
          message: `Swarm ${swarmId} destroyed`,
          metadata: {
            swarm_id: swarmId,
            user_id: userId,
            sandboxes_terminated: results.sandboxesTerminated,
            agents_removed: results.agentsRemoved,
            final_cost: results.finalCost,
            errors: results.errors
          }
        });

      // Success summary
      if (results.errors.length === 0) {
        console.log(chalk.green.bold(`\n✅ Swarm ${swarmId} completely destroyed!`));
      } else {
        console.log(chalk.yellow(`\n⚠️  Swarm ${swarmId} destroyed with ${results.errors.length} warnings`));
        results.errors.forEach(err => console.log(chalk.gray(`  - ${err}`)));
      }

      return results;

    } catch (error) {
      console.error(chalk.red(`\n❌ Failed to destroy swarm: ${error.message}`));
      results.errors.push(error.message);
      return results;
    }
  }

  /**
   * Destroy all swarms for a user
   */
  async destroyAllUserSwarms(userId) {
    console.log(chalk.yellow(`\n🔥 Destroying all swarms for user ${userId}...`));
    
    // Get all active swarms
    const { data: swarms, error } = await this.supabase
      .from('user_swarms')
      .select('id')
      .eq('user_id', userId)
      .eq('status', 'active');

    if (error) {
      console.error(chalk.red('Failed to fetch swarms:'), error);
      return [];
    }

    if (!swarms || swarms.length === 0) {
      console.log(chalk.green('✅ No active swarms to destroy'));
      return [];
    }

    const results = [];
    for (const swarm of swarms) {
      const result = await this.destroySwarm(swarm.id, userId);
      results.push(result);
    }

    return results;
  }

  /**
   * Clean up orphaned sandboxes (sandboxes without swarms)
   */
  async cleanupOrphanedSandboxes() {
    console.log(chalk.cyan('\n🧹 Cleaning up orphaned sandboxes...'));
    
    // Find sandboxes that don't have associated swarms
    const { data: orphaned, error } = await this.supabase
      .from('sandbox_evaluations')
      .select('sandbox_id')
      .eq('sandbox_status', 'running')
      .is('submission_id', null);

    if (error) {
      console.error(chalk.red('Failed to find orphaned sandboxes:'), error);
      return;
    }

    if (!orphaned || orphaned.length === 0) {
      console.log(chalk.green('✅ No orphaned sandboxes found'));
      return;
    }

    for (const sandbox of orphaned) {
      try {
        // Terminate in E2B
        if (this.e2bApiKey) {
          try {
            const sb = await Sandbox.connect(sandbox.sandbox_id, { apiKey: this.e2bApiKey });
            await sb.close();
          } catch (e) {
            // Already terminated
          }
        }

        // Update database
        await this.supabase
          .from('sandbox_evaluations')
          .update({
            sandbox_status: 'terminated',
            pause_reason: 'Orphaned sandbox cleanup'
          })
          .eq('sandbox_id', sandbox.sandbox_id);

        console.log(chalk.green(`  ✅ Cleaned up orphaned sandbox ${sandbox.sandbox_id}`));
      } catch (err) {
        console.error(chalk.red(`  ❌ Failed to cleanup ${sandbox.sandbox_id}:`, err.message));
      }
    }
  }

  /**
   * Force destroy all active swarms (emergency cleanup)
   */
  async forceDestroyAllActiveSwarms() {
    console.log(chalk.red.bold('\n🚨 EMERGENCY: Force destroying ALL active swarms...\n'));
    
    const { data: swarms, error } = await this.supabase
      .from('user_swarms')
      .select('id, user_id')
      .eq('status', 'active');

    if (error) {
      console.error(chalk.red('Failed to fetch swarms:'), error);
      return;
    }

    if (!swarms || swarms.length === 0) {
      console.log(chalk.green('✅ No active swarms found'));
      return;
    }

    console.log(chalk.yellow(`Found ${swarms.length} active swarms to destroy`));
    
    const results = [];
    for (const swarm of swarms) {
      console.log(chalk.gray(`\nProcessing swarm ${swarm.id}...`));
      const result = await this.destroySwarm(swarm.id, swarm.user_id);
      results.push(result);
    }

    // Summary
    const totalSandboxes = results.reduce((sum, r) => sum + r.sandboxesTerminated, 0);
    const totalAgents = results.reduce((sum, r) => sum + r.agentsRemoved, 0);
    const totalCost = results.reduce((sum, r) => sum + (r.finalCost || 0), 0);
    
    console.log(chalk.cyan.bold('\n📊 Cleanup Summary:'));
    console.log(chalk.gray(`  Swarms destroyed: ${results.length}`));
    console.log(chalk.gray(`  Sandboxes terminated: ${totalSandboxes}`));
    console.log(chalk.gray(`  Agents removed: ${totalAgents}`));
    console.log(chalk.gray(`  Total cost saved: ${totalCost.toFixed(2)} rUv`));
    
    return results;
  }
}

export default SwarmCleanupService;