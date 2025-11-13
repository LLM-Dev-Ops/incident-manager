/**
 * Swarm Manager Service
 * Single source of truth for swarm data using Supabase only
 * No local caching - all data comes from database
 */

import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';
import crypto from 'crypto';
import { SUPABASE_URL, SUPABASE_ANON_KEY } from '../config/supabase-config.js';

class SwarmManager {
  constructor(supabaseClient = null) {
    // Use provided client or create new one
    if (supabaseClient) {
      this.supabase = supabaseClient;
      console.log(chalk.gray('[SwarmManager] Using provided Supabase client'));
    } else {
      // Create client with anon key
      this.supabase = createClient(
        SUPABASE_URL,
        SUPABASE_ANON_KEY
      );
      // Try to restore session from environment
      this.initializeSession();
    }
    // NO local cache - always use database
  }
  
  async initializeSession() {
    try {
      // Check if we have a stored session
      const sessionStr = process.env.FLOW_NEXUS_SESSION;
      if (sessionStr) {
        const session = JSON.parse(sessionStr.replace(/\\/g, ''));
        if (session.access_token) {
          // Set the session
          await this.supabase.auth.setSession({
            access_token: session.access_token,
            refresh_token: session.refresh_token
          });
          console.log(chalk.gray('[SwarmManager] Session restored'));
        }
      }
    } catch (err) {
      console.log(chalk.gray('[SwarmManager] No session to restore'));
    }
  }

  /**
   * Get user swarms - ALWAYS from database
   */
  async getUserSwarms(userId) {
    if (!userId) {
      console.log(chalk.yellow('No user ID provided, returning empty swarms'));
      return [];
    }

    try {
      // Ensure session is initialized
      await this.initializeSession();
      
      console.log(chalk.gray(`[SwarmManager] Querying swarms for user: ${userId}`));
      
      const { data, error } = await this.supabase
        .from('user_swarms')
        .select('*')
        .eq('user_id', userId)
        .eq('status', 'active')
        .order('created_at', { ascending: false });
      
      if (error) {
        console.error('[SwarmManager] Failed to get swarms from database:', error);
        return [];
      }
      
      console.log(chalk.gray(`[SwarmManager] Raw query result: ${JSON.stringify(data)}`));
      console.log(chalk.gray(`Found ${data?.length || 0} active swarms in database for user ${userId}`));
      return data || [];
    } catch (err) {
      console.error('[SwarmManager] Error fetching swarms:', err);
      return [];
    }
  }

  /**
   * Get swarm by ID - ALWAYS from database
   */
  async getSwarmById(swarmId) {
    try {
      const { data, error } = await this.supabase
        .from('user_swarms')
        .select('*, user_swarm_agents(*)')
        .eq('id', swarmId)
        .single();
      
      if (error) {
        console.error('Failed to get swarm:', error);
        return null;
      }
      
      return data;
    } catch (err) {
      console.error('Error fetching swarm:', err);
      return null;
    }
  }

  /**
   * Create swarm - directly in database
   */
  async createSwarm(userId, swarmData) {
    try {
      const { data, error } = await this.supabase
        .from('user_swarms')
        .insert({
          id: swarmData.id || crypto.randomUUID(), // Generate proper UUID
          user_id: userId,
          ...swarmData,
          status: 'active',
          created_at: new Date().toISOString()
        })
        .select()
        .single();
      
      if (error) {
        console.error('Failed to create swarm:', error);
        throw error;
      }
      
      return data;
    } catch (err) {
      console.error('Error creating swarm:', err);
      throw err;
    }
  }

  /**
   * Update swarm - directly in database
   */
  async updateSwarm(swarmId, updates) {
    try {
      const { data, error } = await this.supabase
        .from('user_swarms')
        .update(updates)
        .eq('id', swarmId)
        .select()
        .single();
      
      if (error) {
        console.error('Failed to update swarm:', error);
        throw error;
      }
      
      return data;
    } catch (err) {
      console.error('Error updating swarm:', err);
      throw err;
    }
  }

  /**
   * Destroy swarm - mark as destroyed in database
   */
  async destroySwarm(swarmId) {
    try {
      const { data, error } = await this.supabase
        .from('user_swarms')
        .update({
          status: 'destroyed',
          destroyed_at: new Date().toISOString()
        })
        .eq('id', swarmId)
        .select()
        .single();
      
      if (error) {
        console.error('Failed to destroy swarm:', error);
        throw error;
      }
      
      return data;
    } catch (err) {
      console.error('Error destroying swarm:', err);
      throw err;
    }
  }

  /**
   * Get active swarm for user
   */
  async getActiveSwarmId(userId) {
    const swarms = await this.getUserSwarms(userId);
    return swarms.length > 0 ? swarms[0].id : null;
  }

  /**
   * Clean up phantom swarms (that don't exist in DB)
   */
  async cleanupPhantomSwarms(phantomIds) {
    console.log(chalk.yellow('Cleaning up phantom swarms...'));
    
    for (const swarmId of phantomIds) {
      try {
        // Try to insert as destroyed
        await this.supabase
          .from('user_swarms')
          .upsert({
            id: swarmId,
            user_id: 'system',
            topology: 'unknown',
            max_agents: 0,
            status: 'destroyed',
            created_at: new Date().toISOString(),
            metadata: { phantom: true, cleanup: new Date().toISOString() }
          }, {
            onConflict: 'id'
          });
        
        console.log(chalk.green(`  ✅ Phantom ${swarmId} marked as destroyed`));
      } catch (err) {
        console.log(chalk.gray(`  ⚠️ Could not cleanup ${swarmId}`));
      }
    }
  }
}

export default SwarmManager;