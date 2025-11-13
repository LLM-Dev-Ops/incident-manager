#!/usr/bin/env node

import { db } from './supabase.js';
import { security } from '../middleware/security.js';
import crypto from 'crypto';
import bcrypt from 'bcryptjs';
import fs from 'fs/promises';
import { join, dirname } from 'path';

// User registration and management service
export class RegistrationService {
  constructor() {
    this.pendingRegistrations = new Map();
    this.emailVerificationTokens = new Map();
  }

  // Register new user
  async registerUser(email, password, metadata = {}) {
    try {
      // Validate email format
      if (!this.validateEmail(email)) {
        return { 
          success: false, 
          error: 'Invalid email format' 
        };
      }

      // Check password strength
      const passwordCheck = this.validatePassword(password);
      if (!passwordCheck.valid) {
        return { 
          success: false, 
          error: passwordCheck.error 
        };
      }

      // Hash password
      const passwordHash = await bcrypt.hash(password, 12);

      // Create user in Supabase Auth
      const { user, error: authError } = await db.signUp(email, password, {
        ...metadata,
        registered_via: 'mcp_server',
        tier: 'free'
      });

      if (authError) {
        return { 
          success: false, 
          error: authError.message 
        };
      }

      // Generate API key for the user
      const apiKey = security.generateApiKey(user.id, 'free');

      // Create user profile
      await db.client
        .from('user_profiles')
        .insert([{
          id: user.id,
          email: email,
          username: metadata.username || email.split('@')[0],
          full_name: metadata.full_name || '',
          avatar_url: metadata.avatar_url || null,
          bio: metadata.bio || '',
          tier: 'free',
          api_key: apiKey,
          ruv_balance: 100, // Starting balance
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString()
        }]);

      // Generate email verification token
      const verificationToken = crypto.randomBytes(32).toString('hex');
      this.emailVerificationTokens.set(verificationToken, {
        userId: user.id,
        email: email,
        expiresAt: Date.now() + 24 * 60 * 60 * 1000 // 24 hours
      });

      // Send verification email (in production)
      if (process.env.NODE_ENV === 'production') {
        await this.sendVerificationEmail(email, verificationToken);
      }

      // Log registration
      security.logOperation('user_registration', user.id, {
        email: email,
        tier: 'free',
        metadata: Object.keys(metadata)
      });

      return {
        success: true,
        userId: user.id,
        apiKey: apiKey,
        verificationToken: process.env.NODE_ENV !== 'production' ? verificationToken : undefined,
        message: 'Registration successful. Please check your email to verify your account.'
      };

    } catch (error) {
      console.error('Registration error:', error);
      return {
        success: false,
        error: 'Registration failed. Please try again.'
      };
    }
  }

  // Verify email
  async verifyEmail(token) {
    const tokenData = this.emailVerificationTokens.get(token);
    
    if (!tokenData) {
      return { success: false, error: 'Invalid verification token' };
    }

    if (tokenData.expiresAt < Date.now()) {
      this.emailVerificationTokens.delete(token);
      return { success: false, error: 'Verification token expired' };
    }

    try {
      // Update user profile
      await db.client
        .from('user_profiles')
        .update({ 
          email_verified: true,
          verified_at: new Date().toISOString()
        })
        .eq('id', tokenData.userId);

      // Award bonus credits for verification
      await db.awardCredits(tokenData.userId, 50, 'Email verification bonus');

      this.emailVerificationTokens.delete(token);

      return {
        success: true,
        message: 'Email verified successfully. You received 50 bonus rUv credits!'
      };

    } catch (error) {
      return {
        success: false,
        error: 'Verification failed. Please try again.'
      };
    }
  }

  // Login user
  async loginUser(email, password) {
    try {
      const { user, session, error } = await db.signIn(email, password);

      if (error) {
        security.logOperation('failed_login', email, {
          error: error.message
        });
        return { success: false, error: 'Invalid credentials' };
      }

      // Get user profile
      const profile = await db.getUserProfile(user.id);

      // Create session
      const sessionId = security.createSession(user.id, profile.api_key);

      // Generate JWT token
      const token = security.generateToken({
        userId: user.id,
        email: user.email,
        tier: profile.tier,
        sessionId: sessionId
      });

      security.logOperation('user_login', user.id, {
        email: email,
        tier: profile.tier
      });

      return {
        success: true,
        userId: user.id,
        sessionId: sessionId,
        token: token,
        apiKey: profile.api_key,
        profile: {
          email: profile.email,
          username: profile.username,
          tier: profile.tier,
          ruvBalance: profile.ruv_balance
        }
      };

    } catch (error) {
      console.error('Login error:', error);
      return {
        success: false,
        error: 'Login failed. Please try again.'
      };
    }
  }

  // Reset password
  async resetPassword(email) {
    try {
      // Generate reset token
      const resetToken = crypto.randomBytes(32).toString('hex');
      const hashedToken = crypto.createHash('sha256').update(resetToken).digest('hex');

      // Store reset token
      await db.client
        .from('password_reset_tokens')
        .insert([{
          email: email,
          token: hashedToken,
          expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString() // 1 hour
        }]);

      // Send reset email (in production)
      if (process.env.NODE_ENV === 'production') {
        await this.sendPasswordResetEmail(email, resetToken);
      }

      return {
        success: true,
        message: 'Password reset email sent',
        resetToken: process.env.NODE_ENV !== 'production' ? resetToken : undefined
      };

    } catch (error) {
      return {
        success: false,
        error: 'Failed to initiate password reset'
      };
    }
  }

  // Update password with reset token
  async updatePassword(token, newPassword) {
    try {
      const hashedToken = crypto.createHash('sha256').update(token).digest('hex');

      // Verify token
      const { data: tokenData, error } = await db.client
        .from('password_reset_tokens')
        .select('*')
        .eq('token', hashedToken)
        .single();

      if (error || !tokenData) {
        return { success: false, error: 'Invalid reset token' };
      }

      if (new Date(tokenData.expires_at) < new Date()) {
        return { success: false, error: 'Reset token expired' };
      }

      // Validate new password
      const passwordCheck = this.validatePassword(newPassword);
      if (!passwordCheck.valid) {
        return { success: false, error: passwordCheck.error };
      }

      // Update password in Supabase Auth
      const { error: updateError } = await db.client.auth.updateUser({
        password: newPassword
      });

      if (updateError) {
        return { success: false, error: 'Failed to update password' };
      }

      // Delete used token
      await db.client
        .from('password_reset_tokens')
        .delete()
        .eq('token', hashedToken);

      return {
        success: true,
        message: 'Password updated successfully'
      };

    } catch (error) {
      return {
        success: false,
        error: 'Failed to update password'
      };
    }
  }

  // Upgrade user tier
  async upgradeTier(userId, newTier, paymentInfo = {}) {
    try {
      // Validate tier
      const validTiers = ['free', 'pro', 'enterprise'];
      if (!validTiers.includes(newTier)) {
        return { success: false, error: 'Invalid tier' };
      }

      // Process payment (in production)
      if (process.env.NODE_ENV === 'production' && newTier !== 'free') {
        const paymentResult = await this.processPayment(userId, newTier, paymentInfo);
        if (!paymentResult.success) {
          return { success: false, error: 'Payment failed' };
        }
      }

      // Update user profile
      await db.updateUserProfile(userId, {
        tier: newTier,
        upgraded_at: new Date().toISOString()
      });

      // Generate new API key with updated tier
      const oldKey = await this.getUserApiKey(userId);
      if (oldKey) {
        // Revoke old key
        const keyData = security.validApiKeys.get(oldKey);
        if (keyData) {
          keyData.revoked = true;
        }
      }

      // Generate new key
      const newApiKey = security.generateApiKey(userId, newTier);

      // Update API key in profile
      await db.client
        .from('user_profiles')
        .update({ api_key: newApiKey })
        .eq('id', userId);

      // Award bonus credits for upgrade
      const bonusCredits = {
        pro: 500,
        enterprise: 2000
      };

      if (bonusCredits[newTier]) {
        await db.awardCredits(userId, bonusCredits[newTier], `Upgrade to ${newTier} tier bonus`);
      }

      security.logOperation('tier_upgrade', userId, {
        oldTier: 'free',
        newTier: newTier
      });

      return {
        success: true,
        message: `Successfully upgraded to ${newTier} tier`,
        newApiKey: newApiKey,
        bonusCredits: bonusCredits[newTier] || 0
      };

    } catch (error) {
      console.error('Tier upgrade error:', error);
      return {
        success: false,
        error: 'Failed to upgrade tier'
      };
    }
  }

  // Get user API key
  async getUserApiKey(userId) {
    try {
      const profile = await db.getUserProfile(userId);
      return profile?.api_key;
    } catch {
      return null;
    }
  }

  // Validate email format
  validateEmail(email) {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
  }

  // Validate password strength
  validatePassword(password) {
    if (!password || password.length < 8) {
      return { valid: false, error: 'Password must be at least 8 characters long' };
    }

    if (!/[A-Z]/.test(password)) {
      return { valid: false, error: 'Password must contain at least one uppercase letter' };
    }

    if (!/[a-z]/.test(password)) {
      return { valid: false, error: 'Password must contain at least one lowercase letter' };
    }

    if (!/[0-9]/.test(password)) {
      return { valid: false, error: 'Password must contain at least one number' };
    }

    return { valid: true };
  }

  // Send verification email (placeholder)
  async sendVerificationEmail(email, token) {
    // In production, integrate with email service (SendGrid, AWS SES, etc.)
    console.log(`Verification email would be sent to ${email} with token ${token}`);
    return true;
  }

  // Send password reset email (placeholder)
  async sendPasswordResetEmail(email, token) {
    // In production, integrate with email service
    console.log(`Password reset email would be sent to ${email} with token ${token}`);
    return true;
  }

  // Process payment (placeholder)
  async processPayment(userId, tier, paymentInfo) {
    // In production, integrate with payment provider (Stripe, PayPal, etc.)
    console.log(`Processing payment for user ${userId} upgrading to ${tier}`);
    return { success: true, transactionId: crypto.randomBytes(16).toString('hex') };
  }

  // Get user statistics
  async getUserStats(userId) {
    try {
      const [swarms, agents, tasks, sandboxes] = await Promise.all([
        db.client.from('swarms').select('id').eq('owner_id', userId),
        db.client.from('agents').select('id').eq('owner_id', userId),
        db.client.from('tasks').select('id').eq('owner_id', userId),
        db.client.from('sandboxes').select('id').eq('owner_id', userId)
      ]);

      return {
        swarms: swarms.data?.length || 0,
        agents: agents.data?.length || 0,
        tasks: tasks.data?.length || 0,
        sandboxes: sandboxes.data?.length || 0
      };

    } catch (error) {
      console.error('Failed to get user stats:', error);
      return {
        swarms: 0,
        agents: 0,
        tasks: 0,
        sandboxes: 0
      };
    }
  }
}

// Export singleton instance
export const registration = new RegistrationService();