/**
 * Secure Session Management with Encryption
 * Prevents session token theft and implements expiry
 */

import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

class SecureSessionManager {
  constructor() {
    // Use machine-specific key for encryption (more secure than hardcoded)
    this.encryptionKey = this.getOrCreateEncryptionKey();
    this.algorithm = 'aes-256-gcm';
    this.sessionTTL = 48 * 60 * 60 * 1000; // 48 hours in milliseconds
  }

  /**
   * Get or create a machine-specific encryption key
   */
  getOrCreateEncryptionKey() {
    const homeDir = process.env.HOME || process.env.USERPROFILE || '';
    const keyPath = path.join(homeDir, '.flow-nexus', '.key');
    const keyDir = path.dirname(keyPath);
    
    // Create directory if it doesn't exist
    if (!fs.existsSync(keyDir)) {
      fs.mkdirSync(keyDir, { recursive: true, mode: 0o700 }); // Restrictive permissions
    }
    
    // Check if key exists
    if (fs.existsSync(keyPath)) {
      try {
        const key = fs.readFileSync(keyPath, 'utf8');
        return Buffer.from(key, 'hex');
      } catch (error) {
        console.error('Failed to read encryption key:', error.message);
      }
    }
    
    // Generate new key
    const key = crypto.randomBytes(32);
    try {
      fs.writeFileSync(keyPath, key.toString('hex'), { mode: 0o600 }); // Restrictive permissions
      console.log('Generated new encryption key');
    } catch (error) {
      console.error('Failed to save encryption key:', error.message);
    }
    
    return key;
  }

  /**
   * Encrypt session data
   */
  encryptSession(sessionData) {
    try {
      // Add expiry timestamp
      const sessionWithExpiry = {
        ...sessionData,
        encrypted_at: Date.now(),
        expires_at: Date.now() + this.sessionTTL
      };
      
      const text = JSON.stringify(sessionWithExpiry);
      const iv = crypto.randomBytes(16);
      const cipher = crypto.createCipheriv(this.algorithm, this.encryptionKey, iv);
      
      let encrypted = cipher.update(text, 'utf8', 'hex');
      encrypted += cipher.final('hex');
      
      const authTag = cipher.getAuthTag();
      
      // Combine iv, authTag, and encrypted data
      const combined = {
        iv: iv.toString('hex'),
        authTag: authTag.toString('hex'),
        data: encrypted,
        version: '1.0' // For future compatibility
      };
      
      return Buffer.from(JSON.stringify(combined)).toString('base64');
    } catch (error) {
      console.error('Encryption failed:', error.message);
      throw new Error('Failed to encrypt session');
    }
  }

  /**
   * Decrypt session data
   */
  decryptSession(encryptedData) {
    try {
      // Parse the combined data
      const combined = JSON.parse(Buffer.from(encryptedData, 'base64').toString());
      
      if (combined.version !== '1.0') {
        throw new Error('Unsupported encryption version');
      }
      
      const iv = Buffer.from(combined.iv, 'hex');
      const authTag = Buffer.from(combined.authTag, 'hex');
      const encrypted = combined.data;
      
      const decipher = crypto.createDecipheriv(this.algorithm, this.encryptionKey, iv);
      decipher.setAuthTag(authTag);
      
      let decrypted = decipher.update(encrypted, 'hex', 'utf8');
      decrypted += decipher.final('utf8');
      
      const session = JSON.parse(decrypted);
      
      // Check expiry
      if (session.expires_at && Date.now() > session.expires_at) {
        throw new Error('Session expired');
      }
      
      // Remove internal fields before returning
      delete session.encrypted_at;
      delete session.expires_at;
      
      return session;
    } catch (error) {
      if (error.message === 'Session expired') {
        throw error;
      }
      console.error('Decryption failed:', error.message);
      throw new Error('Failed to decrypt session or session corrupted');
    }
  }

  /**
   * Save encrypted session to file
   */
  saveSecureSession(sessionData) {
    try {
      const homeDir = process.env.HOME || process.env.USERPROFILE || '';
      const flowNexusDir = path.join(homeDir, '.flow-nexus');
      
      // Create directory if it doesn't exist
      if (!fs.existsSync(flowNexusDir)) {
        fs.mkdirSync(flowNexusDir, { recursive: true, mode: 0o700 });
      }
      
      const envPath = path.join(flowNexusDir, '.env');
      const backupPath = path.join(flowNexusDir, '.env.backup');
      
      // Backup existing file
      if (fs.existsSync(envPath)) {
        fs.copyFileSync(envPath, backupPath);
      }
      
      // Encrypt session
      const encryptedSession = this.encryptSession(sessionData);
      
      // Read existing content
      let envContent = '';
      if (fs.existsSync(envPath)) {
        envContent = fs.readFileSync(envPath, 'utf8');
        // Remove any existing session lines
        envContent = envContent.replace(/FLOW_NEXUS_SESSION_SECURE=.*/g, '');
        envContent = envContent.replace(/FLOW_NEXUS_SESSION=.*/g, ''); // Remove old unencrypted
        // Clean up extra newlines
        envContent = envContent.replace(/\n\n+/g, '\n');
      }
      
      // Ensure content ends with newline if not empty
      if (envContent && !envContent.endsWith('\n')) {
        envContent += '\n';
      }
      
      // Add encrypted session
      envContent += `FLOW_NEXUS_SESSION_SECURE=${encryptedSession}\n`;
      
      // Write to file with restrictive permissions
      fs.writeFileSync(envPath, envContent, { mode: 0o600 });
      
      // Silenced: console.log('Session securely saved to:', envPath);
      return true;
    } catch (error) {
      console.error('Failed to save secure session:', error.message);
      return false;
    }
  }

  /**
   * Load and decrypt session from file
   */
  loadSecureSession() {
    try {
      // Try multiple .env file locations
      const homeDir = process.env.HOME || process.env.USERPROFILE || '';
      const envPaths = [
        path.join(homeDir, '.flow-nexus', '.env'),  // User home (npm/npx)
        path.join(process.cwd(), '.env'),
        path.join(process.cwd(), '.env.local'),
        path.join(__dirname, '../../.env')
      ];
      
      for (const envPath of envPaths) {
        if (fs.existsSync(envPath)) {
          const envContent = fs.readFileSync(envPath, 'utf8');
          
          // Look for encrypted session first
          const secureMatch = envContent.match(/FLOW_NEXUS_SESSION_SECURE=(.+)/);
          if (secureMatch && secureMatch[1]) {
            try {
              const decryptedSession = this.decryptSession(secureMatch[1]);
              // Silently loaded from envPath
              return decryptedSession;
            } catch (error) {
              if (error.message === 'Session expired') {
                console.log('Session expired, please login again');
                this.clearSession(envPath);
              } else {
                console.error('Failed to decrypt session:', error.message);
              }
              return null;
            }
          }
          
          // Fallback to old unencrypted session (for migration)
          const oldMatch = envContent.match(/FLOW_NEXUS_SESSION=(.+)/);
          if (oldMatch && oldMatch[1] && oldMatch[1].startsWith('{')) {
            try {
              const oldSession = JSON.parse(oldMatch[1]);
              console.log('Migrating old session to encrypted format');
              // Save as encrypted
              this.saveSecureSession(oldSession);
              // Clear old session
              this.clearOldSession(envPath);
              return oldSession;
            } catch (error) {
              console.error('Failed to migrate old session:', error.message);
            }
          }
        }
      }
      
      return null;
    } catch (error) {
      console.error('Failed to load secure session:', error.message);
      return null;
    }
  }

  /**
   * Clear session from file
   */
  clearSession(envPath = null) {
    try {
      const homeDir = process.env.HOME || process.env.USERPROFILE || '';
      const targetPath = envPath || path.join(homeDir, '.flow-nexus', '.env');
      
      if (fs.existsSync(targetPath)) {
        let envContent = fs.readFileSync(targetPath, 'utf8');
        // Remove both encrypted and unencrypted sessions
        envContent = envContent.replace(/FLOW_NEXUS_SESSION_SECURE=.*/g, '');
        envContent = envContent.replace(/FLOW_NEXUS_SESSION=.*/g, '');
        envContent = envContent.replace(/\n\n+/g, '\n');
        fs.writeFileSync(targetPath, envContent);
        // Silenced to avoid spam: console.log('Session cleared from:', targetPath);
      }
    } catch (error) {
      console.error('Failed to clear session:', error.message);
    }
  }

  /**
   * Clear old unencrypted session
   */
  clearOldSession(envPath) {
    try {
      if (fs.existsSync(envPath)) {
        let envContent = fs.readFileSync(envPath, 'utf8');
        // Remove only old unencrypted session
        envContent = envContent.replace(/FLOW_NEXUS_SESSION=.*/g, '');
        envContent = envContent.replace(/\n\n+/g, '\n');
        fs.writeFileSync(envPath, envContent);
      }
    } catch (error) {
      console.error('Failed to clear old session:', error.message);
    }
  }

  /**
   * Check if session needs rotation (for sensitive operations)
   */
  needsRotation(session) {
    if (!session.encrypted_at) return true;
    
    // Rotate after 1 hour for sensitive operations
    const rotationThreshold = 60 * 60 * 1000; // 1 hour
    return (Date.now() - session.encrypted_at) > rotationThreshold;
  }

  /**
   * Generate session fingerprint for additional security
   */
  generateFingerprint() {
    // Combine various system properties for fingerprinting
    const data = {
      platform: process.platform,
      arch: process.arch,
      hostname: process.env.HOSTNAME || 'unknown',
      user: process.env.USER || process.env.USERNAME || 'unknown',
      node: process.version
    };
    
    const hash = crypto.createHash('sha256');
    hash.update(JSON.stringify(data));
    return hash.digest('hex').substring(0, 16);
  }

  /**
   * Validate session fingerprint
   */
  validateFingerprint(session) {
    if (!session.fingerprint) return true; // Skip for old sessions
    return session.fingerprint === this.generateFingerprint();
  }
}

export default new SecureSessionManager();