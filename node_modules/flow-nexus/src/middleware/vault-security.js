/**
 * Vault Security Middleware
 * Secure access to Supabase vault with additional authentication
 */

import crypto from 'crypto';
import jwt from 'jsonwebtoken';

/**
 * Vault access configuration
 */
const VAULT_CONFIG = {
  // Secrets that require additional authentication
  protectedSecrets: [
    'STRIPE_SECRET_KEY',
    'STRIPE_WEBHOOK_SECRET',
    'SUPABASE_SERVICE_ROLE_KEY',
    'ENCRYPTION_KEY',
  ],
  
  // Time-based access window (5 minutes)
  accessWindow: 300000,
  
  // Maximum attempts before lockout
  maxAttempts: 3,
  
  // Lockout duration (30 minutes)
  lockoutDuration: 1800000,
};

/**
 * Vault security manager
 */
class VaultSecurity {
  constructor() {
    this.accessTokens = new Map();
    this.attempts = new Map();
    this.lockouts = new Map();
    
    // Cleanup expired tokens every minute
    setInterval(() => this.cleanup(), 60000);
  }
  
  /**
   * Generate vault access token with additional verification
   */
  async generateVaultToken(userId, operation, context) {
    // Check if user is locked out
    if (this.isLockedOut(userId)) {
      throw new Error('Vault access temporarily locked due to multiple failed attempts');
    }
    
    // Verify user has permission for this operation
    if (!this.hasVaultPermission(context, operation)) {
      this.recordFailedAttempt(userId);
      throw new Error('Insufficient permissions for vault access');
    }
    
    // Generate time-limited token
    const tokenData = {
      userId,
      operation,
      timestamp: Date.now(),
      nonce: crypto.randomBytes(16).toString('hex'),
      fingerprint: this.generateFingerprint(context),
    };
    
    // Sign token with secret
    const secret = process.env.VAULT_SECRET || crypto.randomBytes(32).toString('hex');
    const token = jwt.sign(tokenData, secret, {
      expiresIn: '5m',
      algorithm: 'HS256',
    });
    
    // Store token for validation
    this.accessTokens.set(token, {
      ...tokenData,
      expiresAt: Date.now() + VAULT_CONFIG.accessWindow,
    });
    
    return token;
  }
  
  /**
   * Validate vault access token
   */
  async validateVaultToken(token, secretName, userId) {
    // Check if token exists and is valid
    const tokenData = this.accessTokens.get(token);
    
    if (!tokenData) {
      this.recordFailedAttempt(userId);
      throw new Error('Invalid vault access token');
    }
    
    // Check expiration
    if (Date.now() > tokenData.expiresAt) {
      this.accessTokens.delete(token);
      this.recordFailedAttempt(userId);
      throw new Error('Vault access token expired');
    }
    
    // Check user match
    if (tokenData.userId !== userId) {
      this.recordFailedAttempt(userId);
      throw new Error('Token user mismatch');
    }
    
    // Check if secret is protected
    if (VAULT_CONFIG.protectedSecrets.includes(secretName)) {
      // Verify token signature
      try {
        const secret = process.env.VAULT_SECRET || crypto.randomBytes(32).toString('hex');
        jwt.verify(token, secret, { algorithms: ['HS256'] });
      } catch (error) {
        this.recordFailedAttempt(userId);
        throw new Error('Invalid token signature');
      }
    }
    
    // Token is valid, remove it (single use)
    this.accessTokens.delete(token);
    
    return true;
  }
  
  /**
   * Create secure vault access function
   */
  createSecureVaultAccess(supabase) {
    return async (secretName, userId, vaultToken) => {
      // Validate token first
      await this.validateVaultToken(vaultToken, secretName, userId);
      
      // Additional security checks for protected secrets
      if (VAULT_CONFIG.protectedSecrets.includes(secretName)) {
        // Log access attempt
        await this.logVaultAccess(supabase, userId, secretName, 'attempt');
        
        // Use RPC function with additional auth
        const { data, error } = await supabase.rpc('get_vault_secret_secure', {
          p_secret_name: secretName,
          p_user_id: userId,
          p_vault_token: vaultToken,
        });
        
        if (error) {
          await this.logVaultAccess(supabase, userId, secretName, 'failed');
          throw new Error('Vault access denied');
        }
        
        // Log successful access
        await this.logVaultAccess(supabase, userId, secretName, 'success');
        
        return data;
      } else {
        // Non-protected secrets still require token but less strict
        const { data, error } = await supabase
          .from('vault')
          .select('decrypted_secret')
          .eq('name', secretName)
          .single();
        
        if (error) {
          throw new Error('Secret not found');
        }
        
        return data.decrypted_secret;
      }
    };
  }
  
  /**
   * Check if user has vault permission
   */
  hasVaultPermission(context, operation) {
    // Check authenticated
    if (!context?.user?.id) {
      return false;
    }
    
    // Check session is valid
    if (context.session) {
      const sessionExpiry = new Date(context.session.expires_at);
      if (sessionExpiry < new Date()) {
        return false;
      }
    }
    
    // Admin operations require admin role
    if (operation === 'admin_vault_access') {
      return context.user?.app_metadata?.role === 'admin' ||
             context.user?.email === 'ruv@ruv.net';
    }
    
    // Payment operations require authenticated user
    if (operation === 'payment_vault_access') {
      return context.user?.email_confirmed_at !== null;
    }
    
    return false;
  }
  
  /**
   * Generate device fingerprint for additional security
   */
  generateFingerprint(context) {
    const data = [
      context.user?.id || 'unknown',
      context.user?.email || 'unknown',
      context.ip || 'unknown',
      context.userAgent || 'unknown',
    ].join(':');
    
    return crypto.createHash('sha256').update(data).digest('hex');
  }
  
  /**
   * Record failed attempt
   */
  recordFailedAttempt(userId) {
    const attempts = this.attempts.get(userId) || 0;
    this.attempts.set(userId, attempts + 1);
    
    if (attempts + 1 >= VAULT_CONFIG.maxAttempts) {
      this.lockouts.set(userId, Date.now() + VAULT_CONFIG.lockoutDuration);
      this.attempts.delete(userId);
    }
  }
  
  /**
   * Check if user is locked out
   */
  isLockedOut(userId) {
    const lockoutExpiry = this.lockouts.get(userId);
    
    if (!lockoutExpiry) {
      return false;
    }
    
    if (Date.now() > lockoutExpiry) {
      this.lockouts.delete(userId);
      return false;
    }
    
    return true;
  }
  
  /**
   * Log vault access for audit
   */
  async logVaultAccess(supabase, userId, secretName, status) {
    try {
      await supabase
        .from('vault_access_logs')
        .insert({
          user_id: userId,
          secret_name: secretName,
          access_status: status,
          timestamp: new Date().toISOString(),
        });
    } catch (error) {
      console.error('Failed to log vault access:', error);
    }
  }
  
  /**
   * Cleanup expired data
   */
  cleanup() {
    const now = Date.now();
    
    // Clean expired tokens
    for (const [token, data] of this.accessTokens.entries()) {
      if (now > data.expiresAt) {
        this.accessTokens.delete(token);
      }
    }
    
    // Clean expired lockouts
    for (const [userId, expiry] of this.lockouts.entries()) {
      if (now > expiry) {
        this.lockouts.delete(userId);
      }
    }
    
    // Reset attempts after 1 hour
    const hourAgo = now - 3600000;
    for (const [userId, timestamp] of this.attempts.entries()) {
      if (timestamp < hourAgo) {
        this.attempts.delete(userId);
      }
    }
  }
  
  /**
   * Get security stats
   */
  getStats() {
    return {
      activeTokens: this.accessTokens.size,
      lockedOutUsers: this.lockouts.size,
      failedAttempts: Array.from(this.attempts.values()).reduce((a, b) => a + b, 0),
    };
  }
}

// Export singleton instance
export const vaultSecurity = new VaultSecurity();

// Create secure Supabase RPC function for vault access
export const VAULT_RPC_FUNCTION = `
-- Create secure vault access function
CREATE OR REPLACE FUNCTION get_vault_secret_secure(
  p_secret_name TEXT,
  p_user_id UUID,
  p_vault_token TEXT
)
RETURNS TEXT
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
  v_secret TEXT;
  v_is_admin BOOLEAN;
BEGIN
  -- Check if user is admin
  SELECT 
    COALESCE(
      (raw_app_meta_data->>'role' = 'admin') OR 
      (email = 'ruv@ruv.net'),
      FALSE
    )
  INTO v_is_admin
  FROM auth.users
  WHERE id = p_user_id;
  
  -- Only admins can access protected secrets
  IF p_secret_name IN ('STRIPE_SECRET_KEY', 'STRIPE_WEBHOOK_SECRET', 'SUPABASE_SERVICE_ROLE_KEY') THEN
    IF NOT v_is_admin THEN
      RAISE EXCEPTION 'Unauthorized vault access attempt';
    END IF;
  END IF;
  
  -- Get secret from vault
  SELECT decrypted_secret
  INTO v_secret
  FROM vault.secrets
  WHERE name = p_secret_name;
  
  IF v_secret IS NULL THEN
    RAISE EXCEPTION 'Secret not found';
  END IF;
  
  -- Log access
  INSERT INTO vault_access_logs (
    user_id,
    secret_name,
    access_status,
    vault_token_hash,
    created_at
  ) VALUES (
    p_user_id,
    p_secret_name,
    'success',
    encode(digest(p_vault_token, 'sha256'), 'hex'),
    NOW()
  );
  
  RETURN v_secret;
END;
$$;

-- Create vault access logs table
CREATE TABLE IF NOT EXISTS vault_access_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES auth.users(id),
  secret_name TEXT NOT NULL,
  access_status TEXT NOT NULL,
  vault_token_hash TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Add RLS policies
ALTER TABLE vault_access_logs ENABLE ROW LEVEL SECURITY;

-- Only admins can view vault logs
CREATE POLICY vault_logs_admin_only ON vault_access_logs
  FOR ALL
  USING (
    auth.uid() IN (
      SELECT id FROM auth.users 
      WHERE raw_app_meta_data->>'role' = 'admin'
    )
  );
`;

// Export for testing
export { VaultSecurity, VAULT_CONFIG };