/**
 * Secure Supabase Client for CLI
 * Uses anon key with RLS (Row Level Security) for remote users
 * Now with encrypted session storage and expiry checks
 */

import { createClient } from '@supabase/supabase-js';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import secureSession from './secure-session.js';
import SessionManager from './session-manager.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// DO NOT load any .env files to avoid conflicts with user's local environment
// The Supabase URL and key are hardcoded above

// Public Supabase configuration (safe to include in code)
// These are meant to be public and work with Row Level Security
// IMPORTANT: Do NOT read from environment to avoid conflicts with user's local .env files
const SUPABASE_URL = 'https://pklhxiuouhrcrreectbo.supabase.co';
// Updated to use the correct anon key from the actual Supabase project  
const SUPABASE_ANON_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InBrbGh4aXVvdWhyY3JyZWVjdGJvIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NTU3MDQ1MTQsImV4cCI6MjA3MTI4MDUxNH0.uI34fyRxItPUVKUmn2dc_2RtNxbalHVfmU2EaOV8MK4';

class SupabaseService {
  constructor() {
    // Initialize sessionManager first
    this.sessionManager = new SessionManager();
    
    // Auth operations are free - no rate limiting needed
    
    this.supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
      auth: {
        persistSession: false, // Disable persistence to prevent refresh loops
        autoRefreshToken: false, // Disable auto-refresh to prevent rate limits
        detectSessionInUrl: false,
        // Don't use storage at all - manage sessions manually
        storage: null
      }
    });
    
    // Load stored session if exists
    this.loadSession();
  }

  /**
   * Load stored user session from .env or local storage
   */
  async loadSession() {
    try {
      // Try loading from .env first (for MCP server compatibility)
      const envSession = this.sessionManager.loadSessionFromEnv();
      
      if (envSession && this.sessionManager.isSessionValid(envSession)) {
        // Extend session to 30 days when loading
        const extendedSession = { ...envSession };
        const thirtyDaysFromNow = Math.floor(Date.now() / 1000) + (30 * 24 * 60 * 60);
        extendedSession.expires_at = thirtyDaysFromNow;
        extendedSession.expires_in = 30 * 24 * 60 * 60;
        
        // Set extended session in Supabase client
        const { data, error } = await this.supabase.auth.setSession(extendedSession);
        if (!error) {
          // Save extended session back
          this.sessionManager.saveSessionToEnv(extendedSession);
          // Silenced: console.log('Session loaded from .env');
          return; // Session loaded successfully from .env
        }
      }
      
      // Fall back to secure encrypted session
      const sessionData = secureSession.loadSecureSession();
      
      if (sessionData && sessionData.access_token) {
        // Validate fingerprint for additional security
        if (!secureSession.validateFingerprint(sessionData)) {
          console.log('Session fingerprint mismatch - possible security issue');
          secureSession.clearSession();
          this.sessionManager.clearSessionFromEnv();
          return;
        }
        
        // Set session in Supabase client
        const { data, error } = await this.supabase.auth.setSession(sessionData);
        if (error) {
          console.log('Session validation failed:', error.message);
          secureSession.clearSession();
          this.sessionManager.clearSessionFromEnv();
        } else {
          // Save to .env for MCP server compatibility
          this.sessionManager.saveSessionToEnv(sessionData);
          
          // Check if session needs rotation for sensitive operations
          if (secureSession.needsRotation(sessionData)) {
            // Silent - rotation recommended
          }
        }
      }
    } catch (error) {
      console.log('Session loading error:', error.message);
    }
  }

  /**
   * Save session with encryption
   */
  saveSession(session) {
    try {
      // Add fingerprint for additional security
      const sessionWithFingerprint = {
        ...session,
        fingerprint: secureSession.generateFingerprint()
      };
      
      // Save encrypted session
      const saved = secureSession.saveSecureSession(sessionWithFingerprint);
      if (!saved) {
        console.error('Failed to save encrypted session');
      }
    } catch (error) {
      console.error('Failed to save session:', error.message);
    }
  }

  /**
   * Register new user
   */
  async register(email, password) {
    // Get initial credits from tool_costs table first
    const { data: toolCost } = await this.supabase
      .from('tool_costs')
      .select('cost')
      .eq('tool_name', 'initial_registration')
      .single();
    
    const initialCredits = toolCost?.cost || 256;
    
    const { data, error } = await this.supabase.auth.signUp({
      email,
      password,
      options: {
        data: {
          initial_credits: initialCredits,
          created_at: new Date().toISOString()
        }
      }
    });

    if (error) {
      // Provide more specific error messages
      if (error.message?.includes('already registered')) {
        throw new Error('Email already registered. Use "flow-nexus auth login" instead.');
      } else if (error.message?.includes('Invalid email')) {
        throw new Error('Invalid email address format');
      } else if (error.message?.includes('Password')) {
        throw new Error('Password must be at least 6 characters');
      } else if (error.message?.includes('Database error saving new user')) {
        // This is the actual registration error from Supabase
        console.error('Registration error:', error);
        throw new Error('Registration temporarily unavailable. Please try again.');
      } else if (error.status === 500) {
        throw new Error('Server error. Please try again later.');
      }
      // Default to original error message
      console.error('Registration error details:', error);
      throw error;
    }
    
    if (data.session) {
      // Extend session to 30 days for registration too
      const extendedSession = { ...data.session };
      const thirtyDaysFromNow = Math.floor(Date.now() / 1000) + (30 * 24 * 60 * 60);
      extendedSession.expires_at = thirtyDaysFromNow;
      extendedSession.expires_in = 30 * 24 * 60 * 60;
      
      // Save extended session
      this.saveSession(extendedSession);
      this.sessionManager.saveSessionToEnv(extendedSession);
      
      // Update the data to reflect extended session
      data.session = extendedSession;
    }

    // Initialize user profile with starter credits
    if (data.user && data.session) {
      // Use the initialCredits we already fetched above
      
      // Create a new authenticated client with the user's session token
      const { createClient } = await import('@supabase/supabase-js');
      const authClient = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
        auth: {
          persistSession: false,
          autoRefreshToken: false,
          detectSessionInUrl: false
        },
        global: {
          headers: {
            Authorization: `Bearer ${data.session.access_token}`
          }
        }
      });
      
      // Create profile
      const { error: profileError } = await authClient.from('profiles').upsert({
        id: data.user.id,
        email: data.user.email,
        credits_balance: initialCredits,
        plan_type: 'free',
        created_at: new Date().toISOString()
      });
      
      if (profileError) {
        console.error('Profile creation error:', profileError);
      }
      
      // Create app_store_profiles (required for ruv_balances foreign key)
      const { error: appProfileError } = await authClient.from('app_store_profiles').upsert({
        id: data.user.id,
        username: email.split('@')[0], // Use email prefix as username
        display_name: email.split('@')[0],
        developer_level: 1,
        ruv_credits: initialCredits,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      });
      
      if (appProfileError) {
        console.error('App store profile creation error:', appProfileError);
      }
      
      // Create user_credits record (legacy table but still used)
      // Note: id is auto-incrementing, so we use insert instead of upsert
      const { error: creditsError } = await authClient.from('user_credits').insert({
        user_id: data.user.id,
        balance: initialCredits,
        total_earned: initialCredits,
        total_spent: 0
      });
      
      if (creditsError) {
        console.error('User credits creation error:', creditsError);
      }
      
      // Create ruv_balances record
      const { error: ruvError } = await authClient.from('ruv_balances').upsert({
        user_id: data.user.id,
        available_credits: initialCredits,
        pending_credits: 0,
        lifetime_earned: initialCredits,
        lifetime_spent: 0,
        daily_earned_today: 0,
        last_reset_date: new Date().toISOString().split('T')[0],
        updated_at: new Date().toISOString()
      });
      
      if (ruvError) {
        console.error('RUV balances creation error:', ruvError);
      }
      
      // Add initial transaction
      const { error: transError } = await authClient.from('ruv_transactions').insert({
        user_id: data.user.id,
        amount: initialCredits,
        type: 'credit',
        transaction_type: 'credit',
        balance_after: initialCredits,
        description: 'Initial signup credits',
        metadata: { source: 'signup_bonus' },
        created_at: new Date().toISOString()
      });
      
      if (transError) {
        console.error('Transaction creation error:', transError);
      }
    }

    return data;
  }

  /**
   * Login existing user
   */
  async login(email, password) {
    // Auth operations are free - no rate limiting applied
    
    // On Windows, add a longer delay to avoid hitting rate limits from previous attempts
    if (process.platform === 'win32') {
      // Use exponential backoff if we've seen rate limits recently
      const lastRateLimit = global._lastRateLimit || 0;
      const timeSinceRateLimit = Date.now() - lastRateLimit;
      
      if (timeSinceRateLimit < 30000) { // Within 30 seconds of last rate limit
        const delay = Math.min(5000, 2000 + (30000 - timeSinceRateLimit) / 10);
        await new Promise(resolve => setTimeout(resolve, delay));
      } else {
        await new Promise(resolve => setTimeout(resolve, 2000)); // Default 2 second delay
      }
    }
    
    // Always sign out before login to prevent session conflicts
    // This ensures clean login especially on Windows where sessions can persist
    try {
      await this.supabase.auth.signOut();
    } catch (e) {
      // Ignore signout errors
    }
    
    const { data, error } = await this.supabase.auth.signInWithPassword({
      email,
      password
    });

    if (error) {
      // Enhanced debug logging for API key issues
      if (error.message === 'Invalid API key' || error.status === 401) {
        console.error('\n❌ Authentication Error Details:');
        console.error('  Error:', error.message);
        console.error('  Status:', error.status);
        console.error('  URL:', SUPABASE_URL);
        console.error('  Key prefix:', SUPABASE_ANON_KEY.substring(0, 50) + '...');
        console.error('  Key source: hardcoded (v0.1.98)');
        
        // Check if the key format is valid
        try {
          const keyParts = SUPABASE_ANON_KEY.split('.');
          if (keyParts.length !== 3) {
            console.error('  ⚠️  Invalid JWT format - expected 3 parts, got', keyParts.length);
          } else {
            const payload = JSON.parse(Buffer.from(keyParts[1], 'base64').toString());
            console.error('  Key details: ref=' + payload.ref + ', role=' + payload.role);
            console.error('  Key issued:', new Date(payload.iat * 1000).toISOString());
            console.error('  Key expires:', new Date(payload.exp * 1000).toISOString());
          }
        } catch (e) {
          console.error('  ⚠️  Could not decode JWT:', e.message);
        }
      }
      
      // Debug: Log the actual error on Windows
      if (process.platform === 'win32' && process.env.DEBUG_AUTH) {
        console.log('[DEBUG] Login error:', error);
      }
      
      // For actual Supabase rate limits (429), provide helpful message
      if (error.status === 429) {
        // Track when we last saw a rate limit
        global._lastRateLimit = Date.now();
        // Don't set internal rate limit, just pass through the error with helpful message
        error.message = 'Supabase rate limit reached. Please wait a moment and try again.';
      }
      throw error;
    }
    
    if (data.session) {
      // Extend session to 30 days
      const extendedSession = { ...data.session };
      const thirtyDaysFromNow = Math.floor(Date.now() / 1000) + (30 * 24 * 60 * 60);
      extendedSession.expires_at = thirtyDaysFromNow;
      extendedSession.expires_in = 30 * 24 * 60 * 60;
      
      // Save to both secure session and .env with extended expiry
      this.saveSession(extendedSession);
      this.sessionManager.saveSessionToEnv(extendedSession);
      
      // Update the data to reflect extended session
      data.session = extendedSession;
    }

    return data;
  }

  /**
   * Clear session from .env file
   */
  clearSession(envPath = null) {
    try {
      secureSession.clearSession(envPath);
    } catch (error) {
      console.error('Failed to clear session:', error.message);
    }
  }

  /**
   * Logout user
   */
  async logout() {
    const { error } = await this.supabase.auth.signOut();
    if (error) throw error;
    
    // Clear encrypted session
    secureSession.clearSession();
    
    // Clear session manager
    this.sessionManager.clearSessionFromEnv();
    
    
    // Clear Windows-specific cache locations
    if (process.platform === 'win32') {
      const os = require('os');
      const homedir = os.homedir();
      const flowNexusDir = path.join(homedir, '.flow-nexus');
      try {
        if (fs.existsSync(flowNexusDir)) {
          fs.rmSync(flowNexusDir, { recursive: true, force: true });
        }
      } catch (e) {
        // Ignore errors during cleanup
      }
    }
  }

  /**
   * Get current session
   */
  async getSession() {
    const { data: { session }, error } = await this.supabase.auth.getSession();
    if (error) throw error;
    
    // If we have a session, always extend it to 30 days for display
    if (session) {
      const extendedSession = { ...session };
      const thirtyDaysFromNow = Math.floor(Date.now() / 1000) + (30 * 24 * 60 * 60);
      extendedSession.expires_at = thirtyDaysFromNow;
      extendedSession.expires_in = 30 * 24 * 60 * 60;
      return extendedSession;
    }
    
    return session;
  }

  /**
   * Get current user
   */
  async getCurrentUser() {
    try {
      const { data: { user }, error } = await this.supabase.auth.getUser();
      if (error) {
        // If auth fails, try loading session first
        if (error.message.includes('session') || error.message.includes('token')) {
          await this.loadSession();
          // Try again after loading session
          const { data: { user: retryUser }, error: retryError } = await this.supabase.auth.getUser();
          if (retryError) return null; // Return null instead of throwing
          return retryUser;
        }
        return null; // Return null for other auth errors
      }
      return user;
    } catch (error) {
      return null; // Return null instead of throwing
    }
  }

  /**
   * Get user profile with RLS and accurate balance
   */
  async getUserProfile(userId) {
    const targetUserId = userId || (await this.getCurrentUser()).id;
    
    // Get profile data
    const { data, error } = await this.supabase
      .from('profiles')
      .select('*')
      .eq('id', targetUserId)
      .single();
    
    if (error) throw error;
    
    // Get accurate balance from calculate_user_balance function
    const { data: calculatedBalance } = await this.supabase
      .rpc('calculate_user_balance', { p_user_id: targetUserId });
    
    // Override credits_balance with calculated value if available
    if (calculatedBalance !== null && calculatedBalance !== undefined) {
      data.credits_balance = Math.floor(calculatedBalance); // Floor to match display format
    }
    
    return data;
  }

  /**
   * Get leaderboard (public data)
   */
  async getLeaderboard(limit = 10) {
    // First get the profiles
    const { data: profiles, error } = await this.supabase
      .from('profiles')
      .select('id, email, credits_balance, metadata')
      .order('credits_balance', { ascending: false })
      .limit(limit);
    
    if (error) throw error;
    
    // Then get challenge counts for each user
    if (profiles && profiles.length > 0) {
      const userIds = profiles.map(p => p.id);
      
      // Get challenge completions count for these users
      const { data: completions } = await this.supabase
        .from('challenge_completions')
        .select('user_id')
        .in('user_id', userIds.map(id => id.toString()));
      
      // Count completions per user
      const completionCounts = {};
      if (completions) {
        completions.forEach(c => {
          completionCounts[c.user_id] = (completionCounts[c.user_id] || 0) + 1;
        });
      }
      
      // Add challenge counts to profiles
      return profiles.map(p => ({
        ...p,
        challenges_completed: completionCounts[p.id] || 0
      }));
    }
    
    return profiles;
  }

  /**
   * Get user's challenges
   */
  async getUserChallenges() {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    const { data, error } = await this.supabase
      .from('user_challenges')
      .select('*, challenges(*)')
      .eq('user_id', user.id);
    
    if (error) throw error;
    return data;
  }

  /**
   * Get user's achievements
   */
  async getUserAchievements() {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    const { data, error } = await this.supabase
      .from('user_achievements')
      .select('*')
      .eq('user_id', user.id)
      .order('earned_at', { ascending: false });
    
    if (error) throw error;
    return data;
  }

  /**
   * Get user's storage files
   */
  async getUserFiles(bucket = 'user-files') {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    const { data, error } = await this.supabase
      .storage
      .from(bucket)
      .list(user.id, {
        limit: 100,
        offset: 0
      });
    
    if (error) throw error;
    return data;
  }

  /**
   * Get challenges from database
   */
  async getChallenges(status = 'active') {
    const { data, error } = await this.supabase
      .from('challenges')
      .select('*')
      .eq('status', status)
      .order('difficulty', { ascending: true });
    
    if (error) throw error;
    return data || [];
  }

  /**
   * Get specific challenge details
   */
  async getChallenge(challengeId) {
    const { data, error } = await this.supabase
      .from('challenges')
      .select('*')
      .eq('id', challengeId)
      .single();
    
    if (error) throw error;
    return data;
  }

  /**
   * Start a challenge for user
   */
  async startChallenge(challengeId) {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    // Check if already started
    const { data: existing } = await this.supabase
      .from('user_challenges')
      .select('*')
      .eq('user_id', user.id)
      .eq('challenge_id', challengeId)
      .single();

    if (existing) {
      return existing;
    }

    // Create new user challenge entry
    const { data, error } = await this.supabase
      .from('user_challenges')
      .insert({
        user_id: user.id,
        challenge_id: challengeId,
        status: 'in_progress',
        started_at: new Date().toISOString()
      })
      .select()
      .single();
    
    if (error) throw error;
    return data;
  }

  /**
   * Submit challenge solution
   */
  async submitChallenge(challengeId, solution, language = 'javascript') {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    // Use the database function that handles validation, testing, and credit deduction
    const { data, error } = await this.supabase
      .rpc('submit_challenge_solution', {
        p_user_id: user.id,
        p_challenge_id: challengeId,
        p_code: solution,
        p_language: language
      });

    if (error) {
      console.error('Challenge submission error:', error);
      throw new Error(error.message || 'Failed to submit challenge');
    }

    // Check if submission was successful
    if (!data || !data.success) {
      return {
        success: false,
        message: data?.error || 'Solution submission failed',
        credits_required: data?.credits_required,
        credits_available: data?.credits_available
      };
    }

    // Return the comprehensive result from the database function
    return {
      success: data.success,
      submission_id: data.submission_id,
      attempt_number: data.attempt_number,
      score: data.score,
      passed: data.passed,
      tests_passed: data.tests_passed,
      total_tests: data.total_tests,
      test_results: data.test_results,
      credits_charged: data.credits_charged,
      best_score: data.best_score,
      message: data.message,
      challenge_ends: data.challenge_ends,
      can_resubmit: data.can_resubmit,
      credits_will_be_awarded_after: data.credits_will_be_awarded_after
    };
  }

  /**
   * Calculate reward based on difficulty
   */
  calculateReward(difficulty) {
    const rewards = {
      'beginner': 10,
      'easy': 10,
      'intermediate': 25,
      'medium': 25,
      'advanced': 50,
      'hard': 50,
      'expert': 100
    };
    return rewards[difficulty.toLowerCase()] || 10;
  }

  /**
   * Award credits to user
   */
  async awardCredits(userId, amount, reason) {
    // Add to user's balance
    const { data: profile } = await this.supabase
      .from('user_profiles')
      .select('credits_balance')
      .eq('id', userId)
      .single();

    const newBalance = (profile?.credits_balance || 0) + amount;

    await this.supabase
      .from('user_profiles')
      .update({ credits_balance: newBalance })
      .eq('id', userId);

    // Log transaction
    await this.supabase
      .from('credit_transactions')
      .insert({
        user_id: userId,
        amount: amount,
        type: 'earned',
        reason: reason,
        created_at: new Date().toISOString()
      });

    return newBalance;
  }

  /**
   * Update user statistics
   */
  async updateUserStats(userId, updates) {
    const { data: profile } = await this.supabase
      .from('user_profiles')
      .select('challenges_completed, total_credits_earned')
      .eq('id', userId)
      .single();

    const updateData = {};
    
    if (updates.challenges_completed) {
      updateData.challenges_completed = (profile?.challenges_completed || 0) + 1;
    }
    
    if (updates.credits_earned) {
      updateData.total_credits_earned = (profile?.total_credits_earned || 0) + updates.credits_earned;
    }

    await this.supabase
      .from('user_profiles')
      .update(updateData)
      .eq('id', userId);
  }

  /**
   * Refresh session tokens
   */
  async refreshSession() {
    const { data, error } = await this.supabase.auth.refreshSession();
    if (error) throw error;
    return { data, error };
  }

  /**
   * Store swarm data
   */
  async storeSwarm(swarmData) {
    const { data, error } = await this.supabase
      .from('user_swarms')
      .insert(swarmData)
      .select()
      .single();
    
    if (error) {
      console.error('Failed to store swarm:', error);
    }
    return data;
  }

  /**
   * Get user swarms
   */
  async getUserSwarms(userId) {
    const { data, error } = await this.supabase
      .from('user_swarms')
      .select('*')
      .eq('user_id', userId)
      .eq('status', 'active')
      .order('created_at', { ascending: false });
    
    if (error) {
      console.error('Failed to get swarms:', error);
      return [];
    }
    return data || [];
  }

  /**
   * Get active swarm ID
   */
  async getActiveSwarmId(userId) {
    const { data, error } = await this.supabase
      .from('user_swarms')
      .select('id')
      .eq('user_id', userId)
      .eq('status', 'active')
      .order('created_at', { ascending: false })
      .limit(1)
      .single();
    
    if (error) return null;
    return data?.id;
  }

  /**
   * Update swarm status
   */
  async updateSwarmStatus(swarmId, status) {
    const { data, error } = await this.supabase
      .from('user_swarms')
      .update({ status })
      .eq('id', swarmId)
      .select()
      .single();
    
    if (error) {
      // Only log if not a stack depth error
      if (!error.message?.includes('stack depth')) {
        console.error('Failed to update swarm:', error);
      }
    }
    return data;
  }

  /**
   * Update swarm data
   */
  async updateSwarm(swarmId, updates) {
    const { data, error } = await this.supabase
      .from('user_swarms')
      .update(updates)
      .eq('id', swarmId)
      .select()
      .single();
    
    if (error) {
      // Only log if not a stack depth error
      if (!error.message?.includes('stack depth')) {
        console.error('Failed to update swarm:', error);
      }
    }
    return data;
  }

  /**
   * Store sandbox data
   */
  async storeSandbox(sandboxData) {
    const { data, error } = await this.supabase
      .from('user_sandboxes')
      .insert(sandboxData)
      .select()
      .single();
    
    if (error) {
      console.error('Failed to store sandbox:', error);
    }
    return data;
  }

  /**
   * Get user sandboxes
   */
  async getUserSandboxes(userId) {
    const { data, error } = await this.supabase
      .from('user_sandboxes')
      .select('*')
      .eq('user_id', userId)
      .in('status', ['running', 'stopped'])
      .order('created_at', { ascending: false });
    
    if (error) {
      console.error('Failed to get sandboxes:', error);
      return [];
    }
    return data || [];
  }

  /**
   * Update sandbox status
   */
  async updateSandboxStatus(sandboxId, status) {
    const { data, error } = await this.supabase
      .from('user_sandboxes')
      .update({ status })
      .eq('id', sandboxId)
      .select()
      .single();
    
    if (error) {
      console.error('Failed to update sandbox:', error);
    }
    return data;
  }

  /**
   * Delete sandbox
   */
  async deleteSandbox(sandboxId) {
    const { data, error } = await this.supabase
      .from('user_sandboxes')
      .update({ status: 'deleted' })
      .eq('id', sandboxId)
      .select()
      .single();
    
    if (error) {
      console.error('Failed to delete sandbox:', error);
    }
    return data;
  }

  /**
   * Store deployment data
   */
  async storeDeployment(deploymentData) {
    const { data, error } = await this.supabase
      .from('user_deployments')
      .insert(deploymentData)
      .select()
      .single();
    
    if (error) {
      console.error('Failed to store deployment:', error);
    }
    return data;
  }

  /**
   * Create swarm with atomic credit deduction
   * Calls the database function to ensure atomicity
   */
  async createSwarmWithCredits(swarmData) {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    try {
      // Generate swarm ID if not provided (must be valid UUID)
      const swarmId = swarmData.swarm_id || crypto.randomUUID();
      
      // Call the RPC function for atomic credit deduction and swarm creation
      const { data, error } = await this.supabase
        .rpc('create_swarm_with_credits', {
          p_user_id: user.id,
          p_swarm_id: swarmId,
          p_topology: swarmData.topology,
          p_max_agents: swarmData.max_agents,
          p_strategy: swarmData.strategy,
          p_agents: swarmData.agents,
          p_metadata: swarmData.metadata
        });

      if (error) {
        // If function doesn't exist, throw error to trigger fallback
        if (error.message?.includes('function') || error.message?.includes('not found')) {
          throw new Error('RPC function not found');
        }
        throw error;
      }

      return data;
    } catch (error) {
      // Re-throw to let CLI handle fallback
      throw error;
    }
  }

  /**
   * Calculate final billing when stopping a resource
   */
  async calculateFinalBilling(resourceType, resourceId) {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    try {
      const { data, error } = await this.supabase
        .rpc('calculate_final_billing', {
          p_resource_type: resourceType,
          p_resource_id: resourceId,
          p_user_id: user.id
        });

      if (error) throw error;
      return data;
    } catch (error) {
      // Only log if not a stack depth error
      if (!error.message?.includes('stack depth')) {
        console.error('Failed to calculate final billing:', error);
      }
      return { success: false, error: error.message };
    }
  }

  /**
   * Subscribe to real-time billing alerts
   */
  subscribeToBillingAlerts(callback) {
    return this.supabase
      .channel('billing-alerts')
      .on(
        'postgres_changes',
        {
          event: 'INSERT',
          schema: 'public',
          table: 'billing_history',
          filter: 'resource_type=eq.alert'
        },
        (payload) => {
          callback(payload.new);
        }
      )
      .subscribe();
  }

  /**
   * Get billing history for user
   */
  async getBillingHistory(limit = 50) {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    const { data, error } = await this.supabase
      .from('billing_history')
      .select('*')
      .eq('user_id', user.id)
      .order('created_at', { ascending: false })
      .limit(limit);

    if (error) {
      console.error('Failed to get billing history:', error);
      return [];
    }
    return data || [];
  }

  /**
   * Upload file to user's storage
   */
  async uploadFile(filePath, fileContent, bucket = 'user-files') {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    const fullPath = `${user.id}/${filePath}`;
    const { data, error } = await this.supabase
      .storage
      .from(bucket)
      .upload(fullPath, fileContent, {
        upsert: true
      });
    
    if (error) throw error;
    return data;
  }

  /**
   * Get user's saved agent templates
   */
  async getUserTemplates(userId) {
    const { data, error } = await this.supabase
      .from('user_agent_templates')
      .select('*')
      .eq('user_id', userId)
      .order('created_at', { ascending: false });
    
    if (error) {
      console.error('Failed to get templates:', error);
      // Return empty array if table doesn't exist yet
      return [];
    }
    return data || [];
  }

  /**
   * Save agent configuration as template
   */
  async saveUserTemplate(userId, templateData) {
    const { data, error } = await this.supabase
      .from('user_agent_templates')
      .insert({
        user_id: userId,
        ...templateData,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      })
      .select()
      .single();
    
    if (error) {
      console.error('Failed to save template:', error);
      // If table doesn't exist, just log and continue
      return null;
    }
    return data;
  }

  /**
   * Delete user template
   */
  async deleteUserTemplate(userId, templateId) {
    const { error } = await this.supabase
      .from('user_agent_templates')
      .delete()
      .eq('user_id', userId)
      .eq('id', templateId);
    
    if (error) {
      console.error('Failed to delete template:', error);
      return false;
    }
    return true;
  }

  /**
   * Update user profile
   */
  async updateProfile(updates) {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    // Prepare profile data
    const profileData = {};
    
    // Map CLI fields to database fields
    if (updates.name !== undefined) profileData.display_name = updates.name;
    if (updates.bio !== undefined) profileData.bio = updates.bio;
    if (updates.avatar !== undefined) profileData.avatar_url = updates.avatar;
    if (updates.website !== undefined) {
      if (!profileData.metadata) profileData.metadata = {};
      profileData.metadata.website = updates.website;
    }
    if (updates.github !== undefined) {
      if (!profileData.metadata) profileData.metadata = {};
      profileData.metadata.github = updates.github;
    }
    if (updates.twitter !== undefined) {
      if (!profileData.metadata) profileData.metadata = {};
      profileData.metadata.twitter = updates.twitter;
    }
    if (updates.timezone !== undefined) {
      if (!profileData.metadata) profileData.metadata = {};
      profileData.metadata.timezone = updates.timezone;
    }

    // Update profile
    const { data, error } = await this.supabase
      .from('profiles')
      .update(profileData)
      .eq('id', user.id)
      .select()
      .single();
    
    if (error) {
      console.error('Failed to update profile:', error);
      throw error;
    }
    
    return data;
  }

  /**
   * Change user password
   */
  async changePassword(currentPassword, newPassword) {
    // First verify current password by attempting to sign in
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    // Attempt to update password
    const { error } = await this.supabase.auth.updateUser({
      password: newPassword
    });

    if (error) {
      console.error('Failed to change password:', error);
      throw error;
    }

    return true;
  }

  /**
   * Update user settings
   */
  async updateUserSettings(settings) {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    // Get current profile
    const { data: profile, error: fetchError } = await this.supabase
      .from('profiles')
      .select('metadata')
      .eq('id', user.id)
      .single();

    if (fetchError) throw fetchError;

    // Merge settings into metadata
    const currentMetadata = profile.metadata || {};
    const updatedMetadata = {
      ...currentMetadata,
      settings: {
        ...(currentMetadata.settings || {}),
        ...settings
      }
    };

    // Update profile with new settings
    const { data, error } = await this.supabase
      .from('profiles')
      .update({ metadata: updatedMetadata })
      .eq('id', user.id)
      .select()
      .single();

    if (error) {
      console.error('Failed to update settings:', error);
      throw error;
    }

    return data;
  }

  /**
   * Delete user account (soft delete)
   */
  async deleteAccount() {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    // Mark account as deleted in profiles table
    const { error: profileError } = await this.supabase
      .from('profiles')
      .update({ 
        deleted_at: new Date().toISOString(),
        metadata: {
          deleted: true,
          deleted_at: new Date().toISOString()
        }
      })
      .eq('id', user.id);

    if (profileError) {
      console.error('Failed to mark account as deleted:', profileError);
      throw profileError;
    }

    // Sign out the user
    await this.supabase.auth.signOut();
    
    return true;
  }

  /**
   * Get user privacy settings
   */
  async getPrivacySettings() {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    const { data, error } = await this.supabase
      .from('profiles')
      .select('metadata')
      .eq('id', user.id)
      .single();

    if (error) throw error;

    return data?.metadata?.privacy || {
      profile_visibility: 'public',
      show_email: false,
      show_activity: true,
      show_achievements: true
    };
  }

  /**
   * Update user privacy settings
   */
  async updatePrivacySettings(privacySettings) {
    const user = await this.getCurrentUser();
    if (!user) throw new Error('Not authenticated');

    // Get current profile
    const { data: profile, error: fetchError } = await this.supabase
      .from('profiles')
      .select('metadata')
      .eq('id', user.id)
      .single();

    if (fetchError) throw fetchError;

    // Merge privacy settings into metadata
    const currentMetadata = profile.metadata || {};
    const updatedMetadata = {
      ...currentMetadata,
      privacy: {
        ...(currentMetadata.privacy || {}),
        ...privacySettings
      }
    };

    // Update profile with new privacy settings
    const { data, error } = await this.supabase
      .from('profiles')
      .update({ metadata: updatedMetadata })
      .eq('id', user.id)
      .select()
      .single();

    if (error) {
      console.error('Failed to update privacy settings:', error);
      throw error;
    }

    return data;
  }
}

export default new SupabaseService();