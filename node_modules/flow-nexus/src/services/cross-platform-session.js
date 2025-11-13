/**
 * Cross-Platform Session Manager
 * Handles persistent session storage that works on Windows, Linux, and Mac
 * Stores sessions in user's home directory for MCP server access
 */

import fs from 'fs';
import path from 'path';
import os from 'os';
import chalk from 'chalk';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import windowsSessionFix from './windows-session-fix.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Use Windows-compatible session manager on Windows with npx
const isWindows = process.platform === 'win32';
const isNpx = process.env.npm_execpath?.includes('npx');

class CrossPlatformSessionManager {
  constructor() {
    // Get user's home directory in a cross-platform way
    this.homeDir = os.homedir();
    
    // Create a .flow-nexus directory in user's home for session storage
    this.sessionDir = path.join(this.homeDir, '.flow-nexus');
    this.sessionFile = path.join(this.sessionDir, 'session.json');
    
    // Also support legacy .env location for backward compatibility
    this.legacyEnvPath = this.findProjectEnv();
    
    // Ensure session directory exists
    this.ensureSessionDir();
  }

  /**
   * Ensure session directory exists
   */
  ensureSessionDir() {
    try {
      if (!fs.existsSync(this.sessionDir)) {
        fs.mkdirSync(this.sessionDir, { recursive: true });
      }
    } catch (error) {
      console.error(chalk.red('Failed to create session directory:'), error.message);
    }
  }

  /**
   * Find project .env file (for backward compatibility)
   */
  findProjectEnv() {
    // Try multiple locations to find .env
    const possiblePaths = [
      path.join(process.cwd(), '.env'),
      path.join(__dirname, '../../../../.env'),
      path.join(__dirname, '../../../.env'),
      path.join(__dirname, '../../.env'),
    ];
    
    for (const envPath of possiblePaths) {
      if (fs.existsSync(envPath)) {
        return envPath;
      }
    }
    
    return path.join(process.cwd(), '.env');
  }

  /**
   * Save session to both JSON file and .env (for compatibility)
   */
  saveSession(session) {
    // Use Windows fix for Windows or npx environments
    if (isWindows || isNpx) {
      return windowsSessionFix.saveSession(session);
    }
    let saved = false;
    
    // Save to JSON file in home directory (primary method)
    try {
      const sessionData = {
        ...session,
        savedAt: new Date().toISOString(),
        platform: process.platform,
        nodeVersion: process.version
      };
      
      fs.writeFileSync(
        this.sessionFile, 
        JSON.stringify(sessionData, null, 2), 
        'utf8'
      );
      
      saved = true;
      // Debug: Session saved to file
    } catch (error) {
      // Debug: Failed to save session to JSON
    }
    
    // Also save to .env for backward compatibility
    if (this.legacyEnvPath) {
      this.saveToEnv(session);
    }
    
    // Update process.env for current session
    process.env.FLOW_NEXUS_SESSION = JSON.stringify(session);
    
    return saved;
  }

  /**
   * Save session to .env file (backward compatibility)
   */
  saveToEnv(session) {
    try {
      let envContent = '';
      if (fs.existsSync(this.legacyEnvPath)) {
        envContent = fs.readFileSync(this.legacyEnvPath, 'utf8');
      }
      
      const sessionStr = JSON.stringify(session).replace(/"/g, '\\"');
      const sessionLine = `FLOW_NEXUS_SESSION="${sessionStr}"`;
      
      const sessionRegex = /^FLOW_NEXUS_SESSION=.*$/m;
      
      if (sessionRegex.test(envContent)) {
        envContent = envContent.replace(sessionRegex, sessionLine);
      } else {
        envContent = envContent.trimEnd() + '\n\n' + sessionLine + '\n';
      }
      
      fs.writeFileSync(this.legacyEnvPath, envContent, 'utf8');
    } catch (error) {
      // Silent fail for .env - JSON is primary
    }
  }

  /**
   * Load session from storage
   */
  loadSession() {
    // Use Windows fix for Windows or npx environments
    if (isWindows || isNpx) {
      return windowsSessionFix.loadSession();
    }
    // Try loading from JSON file first (primary method)
    try {
      if (fs.existsSync(this.sessionFile)) {
        const sessionData = JSON.parse(
          fs.readFileSync(this.sessionFile, 'utf8')
        );
        
        // Validate session is not expired
        if (this.isSessionValid(sessionData)) {
          // Debug: Session loaded from home directory
          
          // Update process.env
          process.env.FLOW_NEXUS_SESSION = JSON.stringify(sessionData);
          
          return sessionData;
        } else {
          // Debug: Session expired, please login again
          this.clearSession();
        }
      }
    } catch (error) {
      // Debug: Failed to load session from JSON
    }
    
    // Fallback to process.env
    if (process.env.FLOW_NEXUS_SESSION) {
      try {
        const session = JSON.parse(
          process.env.FLOW_NEXUS_SESSION.replace(/\\/g, '')
        );
        if (this.isSessionValid(session)) {
          return session;
        }
      } catch (error) {
        // Invalid session in env
      }
    }
    
    // Try legacy .env file
    if (this.legacyEnvPath) {
      return this.loadFromEnv();
    }
    
    return null;
  }

  /**
   * Load session from .env file (backward compatibility)
   */
  loadFromEnv() {
    try {
      if (fs.existsSync(this.legacyEnvPath)) {
        const envContent = fs.readFileSync(this.legacyEnvPath, 'utf8');
        const match = envContent.match(/^FLOW_NEXUS_SESSION="(.+)"$/m);
        
        if (match && match[1]) {
          const sessionStr = match[1].replace(/\\"/g, '"');
          const session = JSON.parse(sessionStr);
          
          if (this.isSessionValid(session)) {
            // Migrate to new location
            this.saveSession(session);
            return session;
          }
        }
      }
    } catch (error) {
      // Silent fail
    }
    
    return null;
  }

  /**
   * Clear session from all storage locations
   */
  clearSession() {
    // Use Windows fix for Windows or npx environments
    if (isWindows || isNpx) {
      return windowsSessionFix.clearSession();
    }
    // Clear JSON file
    try {
      if (fs.existsSync(this.sessionFile)) {
        fs.unlinkSync(this.sessionFile);
      }
    } catch (error) {
      // Silent fail
    }
    
    // Clear from .env
    if (this.legacyEnvPath) {
      this.clearFromEnv();
    }
    
    // Clear from process.env
    delete process.env.FLOW_NEXUS_SESSION;
    
    // Debug: Session cleared
  }

  /**
   * Clear session from .env file
   */
  clearFromEnv() {
    try {
      if (fs.existsSync(this.legacyEnvPath)) {
        let envContent = fs.readFileSync(this.legacyEnvPath, 'utf8');
        const sessionRegex = /^FLOW_NEXUS_SESSION=.*$/m;
        envContent = envContent.replace(sessionRegex, '');
        envContent = envContent.replace(/\n\n+/g, '\n\n');
        fs.writeFileSync(this.legacyEnvPath, envContent, 'utf8');
      }
    } catch (error) {
      // Silent fail
    }
  }

  /**
   * Check if session is valid (not expired)
   */
  isSessionValid(session) {
    if (!session || !session.expires_at) {
      return false;
    }
    
    // Handle both Unix timestamp and ISO date string
    const expiresAt = typeof session.expires_at === 'number' 
      ? new Date(session.expires_at * 1000)
      : new Date(session.expires_at);
      
    const now = new Date();
    
    return expiresAt > now;
  }

  /**
   * Get session info for debugging
   */
  getSessionInfo() {
    // Use Windows fix for Windows or npx environments
    if (isWindows || isNpx) {
      return windowsSessionFix.getSessionInfo();
    }
    const session = this.loadSession();
    
    if (!session) {
      return {
        status: 'No session found',
        locations: {
          jsonFile: this.sessionFile,
          envFile: this.legacyEnvPath,
          processEnv: !!process.env.FLOW_NEXUS_SESSION
        }
      };
    }
    
    return {
      status: 'Session found',
      user: session.user?.email,
      expiresAt: session.expires_at,
      isValid: this.isSessionValid(session),
      locations: {
        jsonFile: fs.existsSync(this.sessionFile),
        envFile: this.legacyEnvPath && fs.existsSync(this.legacyEnvPath),
        processEnv: !!process.env.FLOW_NEXUS_SESSION
      }
    };
  }
}

// Export singleton instance
export default new CrossPlatformSessionManager();