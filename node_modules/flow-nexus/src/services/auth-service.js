#!/usr/bin/env node

import { createClient } from '@supabase/supabase-js';
import crypto from 'crypto';
import jwt from 'jsonwebtoken';

// Service-side authentication manager
export class AuthService {
  constructor() {
    // These will be bundled with the npm package but encrypted
    this.encryptedConfig = null;
    this.userSession = null;
  }

  // Initialize with encrypted service credentials
  async initializeService() {
    // In production, these would be encrypted and embedded in the package
    const encryptedUrl = process.env.ENCRYPTED_SUPABASE_URL || this.getEncryptedUrl();
    const encryptedKey = process.env.ENCRYPTED_SUPABASE_KEY || this.getEncryptedKey();
    
    // Decrypt using a key derived from user registration
    if (this.userSession && this.userSession.apiKey) {
      const decryptedUrl = this.decrypt(encryptedUrl, this.userSession.apiKey);
      const decryptedKey = this.decrypt(encryptedKey, this.userSession.apiKey);
      
      return createClient(decryptedUrl, decryptedKey, {
        auth: {
          persistSession: false,
          autoRefreshToken: true
        },
        global: {
          headers: {
            'x-user-token': this.userSession.token,
            'x-api-key': this.userSession.apiKey
          }
        }
      });
    }
    
    throw new Error('User not authenticated. Please register at https://flow-nexus.com');
  }

  // User registration/login
  async authenticateUser(email, apiKey) {
    // Validate API key against your service backend
    const response = await fetch('https://api.flow-nexus.com/auth/validate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, apiKey })
    }).catch(() => null);

    if (!response || !response.ok) {
      // Fallback to local validation for development
      return this.localAuth(email, apiKey);
    }

    const data = await response.json();
    
    if (data.valid) {
      this.userSession = {
        email,
        apiKey,
        token: data.token,
        expiresAt: data.expiresAt,
        tier: data.tier || 'free',
        limits: data.limits || {
          swarms: 3,
          agents: 10,
          tasks: 100,
          sandboxes: 5
        }
      };
      
      // Store session locally
      await this.saveSession();
      return true;
    }
    
    return false;
  }

  // Local development authentication
  localAuth(email, apiKey) {
    // For local development/testing
    if (apiKey === 'dev-key-12345') {
      this.userSession = {
        email,
        apiKey,
        token: jwt.sign({ email, tier: 'dev' }, 'dev-secret', { expiresIn: '7d' }),
        expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000,
        tier: 'dev',
        limits: {
          swarms: 10,
          agents: 50,
          tasks: 1000,
          sandboxes: 20
        }
      };
      return true;
    }
    return false;
  }

  // Check if user is within limits
  checkLimits(resource, count = 1) {
    if (!this.userSession) return false;
    
    const limits = this.userSession.limits;
    const current = this.getResourceUsage(resource);
    
    return current + count <= limits[resource];
  }

  // Track resource usage
  getResourceUsage(resource) {
    // This would query the database for actual usage
    // For now, return 0
    return 0;
  }

  // Encrypt sensitive data
  encrypt(text, key) {
    const algorithm = 'aes-256-gcm';
    const salt = crypto.randomBytes(32);
    const derivedKey = crypto.pbkdf2Sync(key, salt, 100000, 32, 'sha256');
    const iv = crypto.randomBytes(16);
    const cipher = crypto.createCipheriv(algorithm, derivedKey, iv);
    
    let encrypted = cipher.update(text, 'utf8', 'hex');
    encrypted += cipher.final('hex');
    
    const authTag = cipher.getAuthTag();
    
    return Buffer.concat([salt, iv, authTag, Buffer.from(encrypted, 'hex')]).toString('base64');
  }

  // Decrypt sensitive data
  decrypt(encryptedData, key) {
    const buffer = Buffer.from(encryptedData, 'base64');
    const salt = buffer.slice(0, 32);
    const iv = buffer.slice(32, 48);
    const authTag = buffer.slice(48, 64);
    const encrypted = buffer.slice(64);
    
    const derivedKey = crypto.pbkdf2Sync(key, salt, 100000, 32, 'sha256');
    const decipher = crypto.createDecipheriv('aes-256-gcm', derivedKey, iv);
    decipher.setAuthTag(authTag);
    
    let decrypted = decipher.update(encrypted, null, 'utf8');
    decrypted += decipher.final('utf8');
    
    return decrypted;
  }

  // Get encrypted credentials (these would be bundled with npm package)
  getEncryptedUrl() {
    // This would be replaced during build with actual encrypted URL
    return 'ENCRYPTED_URL_PLACEHOLDER';
  }

  getEncryptedKey() {
    // This would be replaced during build with actual encrypted key
    return 'ENCRYPTED_KEY_PLACEHOLDER';
  }

  // Save session to local file
  async saveSession() {
    if (!this.userSession) return;
    
    const sessionPath = join(process.env.HOME || process.env.USERPROFILE, '.flow-nexus', 'session.json');
    await fs.mkdir(dirname(sessionPath), { recursive: true });
    await fs.writeFile(sessionPath, JSON.stringify(this.userSession), 'utf8');
  }

  // Load session from local file
  async loadSession() {
    try {
      const sessionPath = join(process.env.HOME || process.env.USERPROFILE, '.flow-nexus', 'session.json');
      const data = await fs.readFile(sessionPath, 'utf8');
      this.userSession = JSON.parse(data);
      
      // Check if session is expired
      if (this.userSession.expiresAt < Date.now()) {
        this.userSession = null;
        return false;
      }
      
      return true;
    } catch {
      return false;
    }
  }
}

// Singleton instance
export const authService = new AuthService();