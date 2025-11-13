/**
 * Session Manager
 * Handles persistent session storage in .env file
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

class SessionManager {
  constructor() {
    // Path to root .env file
    this.envPath = path.join(__dirname, '../../../../.env');
  }

  /**
   * Save session to .env file
   */
  saveSessionToEnv(session) {
    try {
      // Read current .env content
      let envContent = '';
      if (fs.existsSync(this.envPath)) {
        envContent = fs.readFileSync(this.envPath, 'utf8');
      }

      // Prepare session string (escape for .env format)
      const sessionStr = JSON.stringify(session).replace(/"/g, '\\"');
      const sessionLine = `FLOW_NEXUS_SESSION="${sessionStr}"`;

      // Check if FLOW_NEXUS_SESSION already exists
      const sessionRegex = /^FLOW_NEXUS_SESSION=.*$/m;
      
      if (sessionRegex.test(envContent)) {
        // Replace existing session
        envContent = envContent.replace(sessionRegex, sessionLine);
        // Silenced: console.log(chalk.gray('Updated existing session in .env'));
      } else {
        // Add new session at the end
        envContent = envContent.trimEnd() + '\n\n' + sessionLine + '\n';
        // Silenced: console.log(chalk.gray('Added new session to .env'));
      }

      // Write back to .env
      fs.writeFileSync(this.envPath, envContent, 'utf8');
      
      // Also update process.env for current session
      process.env.FLOW_NEXUS_SESSION = sessionStr;
      
      // Only show this for actual login/register commands, not every session save
      // Silenced: console.log(chalk.green('✅ Session saved to .env file'));
      return true;
    } catch (error) {
      console.error(chalk.red('Failed to save session to .env:'), error.message);
      return false;
    }
  }

  /**
   * Load session from .env file
   */
  loadSessionFromEnv() {
    try {
      // First check process.env
      if (process.env.FLOW_NEXUS_SESSION) {
        const sessionStr = process.env.FLOW_NEXUS_SESSION;
        const session = JSON.parse(sessionStr.replace(/\\/g, ''));
        // Silenced: console.log(chalk.gray('Session loaded from environment'));
        return session;
      }

      // Try reading from .env file directly
      if (fs.existsSync(this.envPath)) {
        const envContent = fs.readFileSync(this.envPath, 'utf8');
        const match = envContent.match(/^FLOW_NEXUS_SESSION="(.+)"$/m);
        
        if (match && match[1]) {
          const sessionStr = match[1].replace(/\\"/g, '"');
          const session = JSON.parse(sessionStr);
          
          // Update process.env
          process.env.FLOW_NEXUS_SESSION = sessionStr;
          
          // Silenced: console.log(chalk.gray('Session loaded from .env file'));
          return session;
        }
      }

      return null;
    } catch (error) {
      console.error(chalk.gray('No valid session found in .env'));
      return null;
    }
  }

  /**
   * Clear session from .env file
   */
  clearSessionFromEnv() {
    try {
      if (fs.existsSync(this.envPath)) {
        let envContent = fs.readFileSync(this.envPath, 'utf8');
        
        // Remove FLOW_NEXUS_SESSION line
        const sessionRegex = /^FLOW_NEXUS_SESSION=.*$/m;
        envContent = envContent.replace(sessionRegex, '');
        
        // Clean up extra newlines
        envContent = envContent.replace(/\n\n+/g, '\n\n');
        
        fs.writeFileSync(this.envPath, envContent, 'utf8');
        
        // Clear from process.env
        delete process.env.FLOW_NEXUS_SESSION;
        
        // Silenced to avoid spam: console.log(chalk.gray('Session cleared from .env'));
        return true;
      }
      return false;
    } catch (error) {
      console.error(chalk.red('Failed to clear session:'), error.message);
      return false;
    }
  }

  /**
   * Check if session is valid (not expired)
   */
  isSessionValid(session) {
    if (!session || !session.expires_at) {
      return false;
    }

    const expiresAt = new Date(session.expires_at * 1000); // Convert from Unix timestamp
    const now = new Date();
    
    return expiresAt > now;
  }
}

export default SessionManager;