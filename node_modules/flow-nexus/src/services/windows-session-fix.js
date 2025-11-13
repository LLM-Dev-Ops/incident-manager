/**
 * Windows Session Fix for NPX Compatibility
 * Ensures session persistence works correctly on Windows with npx
 */

import fs from 'fs';
import path from 'path';
import os from 'os';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

class WindowsSessionManager {
  constructor() {
    // Multiple fallback paths for Windows compatibility
    this.sessionPaths = this.getSessionPaths();
    this.debug = process.env.DEBUG === 'true';
  }

  /**
   * Get all possible session storage paths in priority order
   */
  getSessionPaths() {
    const paths = [];
    
    // 1. User's home directory (primary location)
    const homeDir = os.homedir();
    if (homeDir) {
      paths.push({
        dir: path.join(homeDir, '.flow-nexus'),
        file: path.join(homeDir, '.flow-nexus', 'session.json'),
        type: 'home'
      });
    }
    
    // 2. Windows APPDATA
    if (process.platform === 'win32') {
      const appData = process.env.APPDATA || process.env.LOCALAPPDATA;
      if (appData) {
        paths.push({
          dir: path.join(appData, 'flow-nexus'),
          file: path.join(appData, 'flow-nexus', 'session.json'),
          type: 'appdata'
        });
      }
    }
    
    // 3. Windows USERPROFILE
    if (process.env.USERPROFILE) {
      paths.push({
        dir: path.join(process.env.USERPROFILE, '.flow-nexus'),
        file: path.join(process.env.USERPROFILE, '.flow-nexus', 'session.json'),
        type: 'userprofile'
      });
    }
    
    // 4. Temp directory (last resort for npx)
    const tmpDir = os.tmpdir();
    paths.push({
      dir: path.join(tmpDir, '.flow-nexus-session'),
      file: path.join(tmpDir, '.flow-nexus-session', 'session.json'),
      type: 'temp'
    });
    
    // 5. Current working directory .env (compatibility)
    paths.push({
      dir: process.cwd(),
      file: path.join(process.cwd(), '.env'),
      type: 'env',
      isEnvFile: true
    });
    
    return paths;
  }

  /**
   * Ensure directory exists
   */
  ensureDir(dirPath) {
    try {
      if (!fs.existsSync(dirPath)) {
        fs.mkdirSync(dirPath, { recursive: true });
      }
      return true;
    } catch (error) {
      if (this.debug) {
        console.log(`Failed to create directory ${dirPath}:`, error.message);
      }
      return false;
    }
  }

  /**
   * Save session to all accessible locations for redundancy
   */
  saveSession(session) {
    let savedCount = 0;
    const errors = [];
    
    // Add timestamp to session
    const sessionWithMeta = {
      ...session,
      savedAt: new Date().toISOString(),
      platform: process.platform,
      nodeVersion: process.version
    };
    
    // Try to save to all locations
    for (const location of this.sessionPaths) {
      try {
        if (location.isEnvFile) {
          // Special handling for .env file
          this.saveToEnv(location.file, sessionWithMeta);
          savedCount++;
          if (this.debug) {
            console.log(`✅ Saved session to .env: ${location.file}`);
          }
        } else {
          // Ensure directory exists
          if (this.ensureDir(location.dir)) {
            // Save as JSON
            fs.writeFileSync(
              location.file,
              JSON.stringify(sessionWithMeta, null, 2),
              'utf8'
            );
            savedCount++;
            if (this.debug) {
              console.log(`✅ Saved session to ${location.type}: ${location.file}`);
            }
          }
        }
      } catch (error) {
        errors.push(`${location.type}: ${error.message}`);
        if (this.debug) {
          console.log(`❌ Failed to save to ${location.type}: ${error.message}`);
        }
      }
    }
    
    // Also save to process.env for current session
    process.env.FLOW_NEXUS_SESSION = JSON.stringify(sessionWithMeta);
    
    if (savedCount === 0) {
      // Silent fail for MCP context - errors tracked but not logged
      return false;
    }
    
    if (this.debug) {
      console.log(`Session saved to ${savedCount} location(s)`);
    }
    
    return true;
  }

  /**
   * Save session to .env file
   */
  saveToEnv(envPath, session) {
    try {
      let envContent = '';
      if (fs.existsSync(envPath)) {
        envContent = fs.readFileSync(envPath, 'utf8');
      }
      
      const sessionStr = JSON.stringify(session).replace(/"/g, '\\"');
      const sessionLine = `FLOW_NEXUS_SESSION="${sessionStr}"`;
      
      const sessionRegex = /^FLOW_NEXUS_SESSION=.*$/m;
      
      if (sessionRegex.test(envContent)) {
        envContent = envContent.replace(sessionRegex, sessionLine);
      } else {
        envContent = envContent.trimEnd() + '\n\n' + sessionLine + '\n';
      }
      
      fs.writeFileSync(envPath, envContent, 'utf8');
      return true;
    } catch (error) {
      if (this.debug) {
        console.log('Failed to save to .env:', error.message);
      }
      return false;
    }
  }

  /**
   * Load session from first available location
   */
  loadSession() {
    // First check process.env (fastest)
    if (process.env.FLOW_NEXUS_SESSION) {
      try {
        const session = JSON.parse(
          process.env.FLOW_NEXUS_SESSION.replace(/\\/g, '')
        );
        if (this.isSessionValid(session)) {
          if (this.debug) {
            console.log('✅ Loaded session from process.env');
          }
          return session;
        }
      } catch (error) {
        // Invalid session in env
      }
    }
    
    // Try each location in order
    for (const location of this.sessionPaths) {
      try {
        if (location.isEnvFile) {
          // Load from .env file
          const session = this.loadFromEnv(location.file);
          if (session && this.isSessionValid(session)) {
            if (this.debug) {
              console.log(`✅ Loaded session from .env: ${location.file}`);
            }
            // Update process.env for faster access
            process.env.FLOW_NEXUS_SESSION = JSON.stringify(session);
            return session;
          }
        } else if (fs.existsSync(location.file)) {
          // Load from JSON file
          const sessionData = JSON.parse(
            fs.readFileSync(location.file, 'utf8')
          );
          
          if (this.isSessionValid(sessionData)) {
            if (this.debug) {
              console.log(`✅ Loaded session from ${location.type}: ${location.file}`);
            }
            // Update process.env for faster access
            process.env.FLOW_NEXUS_SESSION = JSON.stringify(sessionData);
            return sessionData;
          }
        }
      } catch (error) {
        if (this.debug) {
          console.log(`Failed to load from ${location.type}: ${error.message}`);
        }
      }
    }
    
    if (this.debug) {
      console.log('❌ No valid session found in any location');
    }
    
    return null;
  }

  /**
   * Load session from .env file
   */
  loadFromEnv(envPath) {
    try {
      if (!fs.existsSync(envPath)) {
        return null;
      }
      
      const envContent = fs.readFileSync(envPath, 'utf8');
      const match = envContent.match(/^FLOW_NEXUS_SESSION="(.+)"$/m);
      
      if (match && match[1]) {
        const sessionStr = match[1].replace(/\\"/g, '"');
        return JSON.parse(sessionStr);
      }
    } catch (error) {
      // Silent fail
    }
    
    return null;
  }

  /**
   * Check if session is valid (not expired)
   */
  isSessionValid(session) {
    if (!session) {
      return false;
    }
    
    // Check for required fields
    if (!session.access_token || !session.user) {
      return false;
    }
    
    // Check expiry
    if (session.expires_at) {
      const expiresAt = typeof session.expires_at === 'number'
        ? new Date(session.expires_at * 1000)
        : new Date(session.expires_at);
      
      const now = new Date();
      return expiresAt > now;
    }
    
    // No expiry info, assume valid
    return true;
  }

  /**
   * Clear session from all locations
   */
  clearSession() {
    let clearedCount = 0;
    
    // Clear from all locations
    for (const location of this.sessionPaths) {
      try {
        if (location.isEnvFile) {
          // Clear from .env
          this.clearFromEnv(location.file);
          clearedCount++;
        } else if (fs.existsSync(location.file)) {
          // Delete JSON file
          fs.unlinkSync(location.file);
          clearedCount++;
          if (this.debug) {
            console.log(`✅ Cleared session from ${location.type}`);
          }
        }
      } catch (error) {
        if (this.debug) {
          console.log(`Failed to clear from ${location.type}: ${error.message}`);
        }
      }
    }
    
    // Clear from process.env
    delete process.env.FLOW_NEXUS_SESSION;
    
    if (this.debug) {
      console.log(`Cleared session from ${clearedCount} location(s)`);
    }
    
    return clearedCount > 0;
  }

  /**
   * Clear session from .env file
   */
  clearFromEnv(envPath) {
    try {
      if (!fs.existsSync(envPath)) {
        return;
      }
      
      let envContent = fs.readFileSync(envPath, 'utf8');
      const sessionRegex = /^FLOW_NEXUS_SESSION=.*$/m;
      envContent = envContent.replace(sessionRegex, '');
      envContent = envContent.replace(/\n\n+/g, '\n\n');
      fs.writeFileSync(envPath, envContent, 'utf8');
    } catch (error) {
      // Silent fail
    }
  }

  /**
   * Get debug info about session locations
   */
  getSessionInfo() {
    const info = {
      platform: process.platform,
      nodeVersion: process.version,
      npx: process.env.npm_execpath?.includes('npx'),
      locations: []
    };
    
    for (const location of this.sessionPaths) {
      const exists = fs.existsSync(location.file);
      let hasValidSession = false;
      
      if (exists) {
        try {
          if (location.isEnvFile) {
            const session = this.loadFromEnv(location.file);
            hasValidSession = this.isSessionValid(session);
          } else {
            const session = JSON.parse(fs.readFileSync(location.file, 'utf8'));
            hasValidSession = this.isSessionValid(session);
          }
        } catch (error) {
          // Invalid session
        }
      }
      
      info.locations.push({
        type: location.type,
        path: location.file,
        exists: exists,
        valid: hasValidSession,
        writable: this.isWritable(location.dir)
      });
    }
    
    // Check process.env
    info.processEnv = !!process.env.FLOW_NEXUS_SESSION;
    
    return info;
  }

  /**
   * Check if directory is writable
   */
  isWritable(dirPath) {
    try {
      // Try to create directory if it doesn't exist
      if (!fs.existsSync(dirPath)) {
        fs.mkdirSync(dirPath, { recursive: true });
      }
      
      // Test write access
      const testFile = path.join(dirPath, '.test-write');
      fs.writeFileSync(testFile, 'test', 'utf8');
      fs.unlinkSync(testFile);
      return true;
    } catch (error) {
      return false;
    }
  }
}

// Export singleton instance
export default new WindowsSessionManager();