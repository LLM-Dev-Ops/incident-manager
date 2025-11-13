/**
 * Sandbox Lifecycle Manager
 * Manages E2B sandbox billing, lifecycle, and state preservation
 * Prevents runaway costs and ensures 24-hour limits
 */

import { createClient } from '@supabase/supabase-js';
import { SUPABASE_URL, SUPABASE_ANON_KEY } from '../config/supabase-config.js';
import { Sandbox } from 'e2b';
import chalk from 'chalk';

class SandboxLifecycleManager {
  constructor() {
    this.supabase = createClient(
      SUPABASE_URL,
      SUPABASE_ANON_KEY // Only use anon key for security
    );
    
    this.MAX_RUNTIME_HOURS = 24;
    this.WARNING_THRESHOLD_HOURS = 23;
    this.HOURLY_RATE_RUV = 3; // 3 rUv per hour
    this.CHECK_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes
    
    this.activeSandboxes = new Map();
    this.checkInterval = null;
  }

  /**
   * Start monitoring sandboxes
   */
  startMonitoring() {
    console.log(chalk.cyan('🔍 Starting sandbox lifecycle monitoring...'));
    
    // Initial check
    this.checkAllSandboxes();
    
    // Schedule regular checks
    this.checkInterval = setInterval(() => {
      this.checkAllSandboxes();
    }, this.CHECK_INTERVAL_MS);
  }

  /**
   * Stop monitoring
   */
  stopMonitoring() {
    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
      console.log(chalk.yellow('⏹️ Stopped sandbox monitoring'));
    }
  }

  /**
   * Create a new sandbox with billing tracking
   */
  async createSandbox(submissionId, userId, challengeId, options = {}) {
    const sandboxId = `sandbox_${submissionId}_${Date.now()}`;
    
    try {
      // Create E2B sandbox
      const sandbox = await Sandbox.create({
        template: options.template || 'node',
        apiKey: process.env.E2B_API_KEY,
        ...options
      });
      
      // Initialize billing record
      const { error: billingError } = await this.supabase
        .from('sandbox_billing')
        .insert({
          sandbox_id: sandboxId,
          submission_id: submissionId,
          user_id: userId,
          status: 'active',
          hourly_rate_ruv: this.HOURLY_RATE_RUV,
          max_runtime_hours: this.MAX_RUNTIME_HOURS
        });
      
      if (billingError) {
        console.error(chalk.red('Failed to create billing record:'), billingError);
      }
      
      // Initialize evaluation record
      const { error: evalError } = await this.supabase
        .from('sandbox_evaluations')
        .insert({
          sandbox_id: sandboxId,
          submission_id: submissionId,
          challenge_id: challengeId,
          sandbox_status: 'running',
          started_at: new Date().toISOString()
        });
      
      if (evalError) {
        console.error(chalk.red('Failed to create evaluation record:'), evalError);
      }
      
      // Track in memory
      this.activeSandboxes.set(sandboxId, {
        sandbox,
        submissionId,
        userId,
        startTime: Date.now(),
        isPaused: false
      });
      
      console.log(chalk.green(`✅ Created sandbox ${sandboxId} with billing protection`));
      
      return { sandboxId, sandbox };
      
    } catch (error) {
      console.error(chalk.red('Failed to create sandbox:'), error);
      throw error;
    }
  }

  /**
   * Pause a sandbox to stop billing
   */
  async pauseSandbox(sandboxId, reason = 'Manual pause') {
    const sandboxData = this.activeSandboxes.get(sandboxId);
    
    if (!sandboxData || sandboxData.isPaused) {
      return false;
    }
    
    try {
      // Calculate runtime and cost
      const runtimeMinutes = Math.floor((Date.now() - sandboxData.startTime) / 60000);
      const totalCost = (runtimeMinutes / 60) * this.HOURLY_RATE_RUV;
      
      // Pause in E2B (if supported)
      if (sandboxData.sandbox && typeof sandboxData.sandbox.pause === 'function') {
        await sandboxData.sandbox.pause();
      }
      
      // Update database
      await this.supabase
        .from('sandbox_evaluations')
        .update({
          sandbox_status: 'paused',
          paused_at: new Date().toISOString(),
          runtime_minutes: runtimeMinutes,
          total_cost_ruv: totalCost,
          auto_paused: true,
          pause_reason: reason
        })
        .eq('sandbox_id', sandboxId);
      
      await this.supabase
        .from('sandbox_billing')
        .update({
          status: 'paused',
          end_time: new Date().toISOString(),
          runtime_minutes: runtimeMinutes,
          total_cost_ruv: totalCost,
          updated_at: new Date().toISOString()
        })
        .eq('sandbox_id', sandboxId);
      
      // Update in-memory state
      sandboxData.isPaused = true;
      
      // Log the pause
      await this.logEvent('sandbox_paused', `Sandbox ${sandboxId} paused: ${reason}`, {
        sandboxId,
        runtimeMinutes,
        totalCost,
        reason
      });
      
      console.log(chalk.yellow(`⏸️ Paused sandbox ${sandboxId}: ${reason}`));
      console.log(chalk.gray(`  Runtime: ${runtimeMinutes} min, Cost: ${totalCost.toFixed(2)} rUv`));
      
      return true;
      
    } catch (error) {
      console.error(chalk.red('Failed to pause sandbox:'), error);
      return false;
    }
  }

  /**
   * Preserve sandbox state for judge review
   */
  async preserveStateForReview(sandboxId, submissionId) {
    try {
      const sandboxData = this.activeSandboxes.get(sandboxId);
      
      if (!sandboxData) {
        console.warn(chalk.yellow(`Sandbox ${sandboxId} not found in active list`));
        return null;
      }
      
      // Get files from sandbox (if E2B supports it)
      let files = [];
      if (sandboxData.sandbox && typeof sandboxData.sandbox.filesystem === 'object') {
        try {
          // This would need actual E2B API calls
          files = await this.getFilesFromSandbox(sandboxData.sandbox);
        } catch (e) {
          console.warn('Could not retrieve files from sandbox');
        }
      }
      
      // Build state snapshot
      const stateSnapshot = {
        sandboxId,
        submissionId,
        preservedAt: new Date().toISOString(),
        files,
        runtimeMinutes: Math.floor((Date.now() - sandboxData.startTime) / 60000),
        cost: ((Date.now() - sandboxData.startTime) / 60000 / 60) * this.HOURLY_RATE_RUV
      };
      
      // Store state
      await this.supabase
        .from('sandbox_evaluations')
        .update({
          state_snapshot: stateSnapshot
        })
        .eq('sandbox_id', sandboxId);
      
      // Mark as non-billable for judge review
      await this.supabase
        .from('sandbox_billing')
        .update({
          is_judge_review: true,
          charged_to_user: false
        })
        .eq('sandbox_id', sandboxId);
      
      console.log(chalk.green(`💾 Preserved state for sandbox ${sandboxId}`));
      
      return stateSnapshot;
      
    } catch (error) {
      console.error(chalk.red('Failed to preserve state:'), error);
      return null;
    }
  }

  /**
   * Check all active sandboxes for limits
   */
  async checkAllSandboxes() {
    console.log(chalk.cyan('🔍 Checking sandbox limits...'));
    
    // Get active sandboxes from database
    const { data: activeSandboxes, error } = await this.supabase
      .from('sandbox_billing')
      .select(`
        *,
        sandbox_evaluations!inner(
          submission_id,
          sandbox_status
        )
      `)
      .eq('status', 'active');
    
    if (error) {
      console.error(chalk.red('Failed to fetch active sandboxes:'), error);
      return;
    }
    
    if (!activeSandboxes || activeSandboxes.length === 0) {
      return;
    }
    
    console.log(chalk.gray(`Found ${activeSandboxes.length} active sandboxes`));
    
    for (const billing of activeSandboxes) {
      const runtimeHours = (Date.now() - new Date(billing.start_time).getTime()) / (1000 * 60 * 60);
      
      // Check 24-hour limit
      if (runtimeHours >= this.MAX_RUNTIME_HOURS) {
        await this.pauseSandbox(billing.sandbox_id, `Exceeded ${this.MAX_RUNTIME_HOURS} hour limit`);
        await this.terminateSandbox(billing.sandbox_id);
        continue;
      }
      
      // Check if judging is complete
      const { data: judgeDecision } = await this.supabase
        .from('judge_decisions')
        .select('id')
        .eq('submission_id', billing.sandbox_evaluations.submission_id)
        .single();
      
      if (judgeDecision) {
        await this.pauseSandbox(billing.sandbox_id, 'Judging complete');
        await this.preserveStateForReview(
          billing.sandbox_id,
          billing.sandbox_evaluations.submission_id
        );
        continue;
      }
      
      // Send warning at 23 hours
      if (runtimeHours >= this.WARNING_THRESHOLD_HOURS && !billing.warning_sent_at) {
        await this.sendWarning(billing);
      }
    }
  }

  /**
   * Send warning about approaching limit
   */
  async sendWarning(billing) {
    await this.supabase
      .from('sandbox_billing')
      .update({
        warning_sent_at: new Date().toISOString()
      })
      .eq('sandbox_id', billing.sandbox_id);
    
    await this.logEvent('sandbox_warning', `Sandbox ${billing.sandbox_id} approaching 24hr limit`, {
      sandboxId: billing.sandbox_id,
      runtimeHours: (Date.now() - new Date(billing.start_time).getTime()) / (1000 * 60 * 60)
    });
    
    console.log(chalk.yellow(`⚠️ Warning sent for sandbox ${billing.sandbox_id}`));
  }

  /**
   * Terminate a sandbox completely
   */
  async terminateSandbox(sandboxId) {
    const sandboxData = this.activeSandboxes.get(sandboxId);
    
    if (sandboxData && sandboxData.sandbox) {
      try {
        await sandboxData.sandbox.close();
      } catch (e) {
        console.error(chalk.red('Failed to close E2B sandbox:'), e);
      }
    }
    
    // Update status
    await this.supabase
      .from('sandbox_billing')
      .update({
        status: 'terminated',
        updated_at: new Date().toISOString()
      })
      .eq('sandbox_id', sandboxId);
    
    await this.supabase
      .from('sandbox_evaluations')
      .update({
        sandbox_status: 'terminated'
      })
      .eq('sandbox_id', sandboxId);
    
    // Remove from active list
    this.activeSandboxes.delete(sandboxId);
    
    console.log(chalk.red(`🛑 Terminated sandbox ${sandboxId}`));
  }

  /**
   * Get files from sandbox (placeholder - needs E2B API)
   */
  async getFilesFromSandbox(sandbox) {
    // This would need actual E2B API implementation
    return [];
  }

  /**
   * Log event to system_logs
   */
  async logEvent(type, message, metadata) {
    await this.supabase
      .from('system_logs')
      .insert({
        log_type: type,
        message,
        metadata
      });
  }

  /**
   * Calculate total costs for a user
   */
  async calculateUserCosts(userId, dateFrom = null, dateTo = null) {
    const query = this.supabase
      .from('sandbox_billing')
      .select('*')
      .eq('user_id', userId);
    
    if (dateFrom) query.gte('start_time', dateFrom);
    if (dateTo) query.lte('start_time', dateTo);
    
    const { data, error } = await query;
    
    if (error) {
      console.error(chalk.red('Failed to calculate costs:'), error);
      return null;
    }
    
    const summary = {
      totalSandboxes: data.length,
      totalRuntimeMinutes: 0,
      totalCostRuv: 0,
      judgeReviewCostRuv: 0,
      userChargedCostRuv: 0
    };
    
    for (const billing of data) {
      summary.totalRuntimeMinutes += billing.runtime_minutes || 0;
      summary.totalCostRuv += parseFloat(billing.total_cost_ruv) || 0;
      
      if (billing.is_judge_review) {
        summary.judgeReviewCostRuv += parseFloat(billing.total_cost_ruv) || 0;
      }
      
      if (billing.charged_to_user) {
        summary.userChargedCostRuv += parseFloat(billing.total_cost_ruv) || 0;
      }
    }
    
    return summary;
  }

  /**
   * Clean up old sandboxes
   */
  async cleanupOldSandboxes() {
    console.log(chalk.cyan('🧹 Cleaning up old sandboxes...'));
    
    // Terminate sandboxes older than 48 hours
    const { data, error } = await this.supabase
      .from('sandbox_evaluations')
      .update({
        sandbox_status: 'terminated',
        pause_reason: 'Cleanup - 48hr limit'
      })
      .in('sandbox_status', ['running', 'paused'])
      .lt('started_at', new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString())
      .select();
    
    if (data && data.length > 0) {
      console.log(chalk.green(`✅ Cleaned up ${data.length} old sandboxes`));
    }
  }
}

export default SandboxLifecycleManager;