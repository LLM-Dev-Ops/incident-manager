/**
 * Secure Configuration Manager
 * Handles obfuscated Supabase credentials and user authentication
 */

import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Obfuscated production credentials (base64 encoded and encrypted)
const ENCRYPTED_CONFIG = {
  // These are encrypted with a derivation of the app name
  url: 'U2FsdGVkX1+8xKqZ9vM5kX5H3PxXKL8vKGz7yY5RhYw5lRPGXKL8vKGz7yY5RhYw5lRPGXKL8v',
  anonKey: 'U2FsdGVkX1+9yKqZ9vM5kX5H3PxXKL8vKGz7yY5RhYw5lRPGXKL8vKGz7yY5RhYw5lRPG',
  salt: 'flow-nexus-mcp-2024'
};

export interface UserConfig {
  userId?: string;
  email?: string;
  apiKey?: string;
  accessToken?: string;
  refreshToken?: string;
  credits?: number;
  isAuthenticated: boolean;
}

export class SecureConfigManager {
  private static instance: SecureConfigManager;
  private userConfig: UserConfig = { isAuthenticated: false };
  private configPath: string;
  private userEnvPath: string;
  private systemConfigPath: string;
  
  // Hardcoded obfuscated credentials
  private readonly OBFUSCATED_SUPABASE = {
    url: Buffer.from('aHR0cHM6Ly9lb2p1Y2ducHNrb3Z0YWRmd2Zpci5zdXBhYmFzZS5jbw==', 'base64').toString(),
    anonKey: Buffer.from('ZXlKaGJHY2lPaUpJVXpJMU5pSXNJblI1Y0NJNklrcFhWQ0o5LmV5SnBjM01pT2lKemRYQmhZbUZ6WlNJc0luSmxaaUk2SW1WdmFuVmpaMjV3YzJ0dmRuUmhaR1ozWm1seUlpd2ljbTlzWlNJNkltRnViMjRpTENKcFlYUWlPakUzTXpRMk5EQTNPVGDZLEW1VjNSQ0l4TURFM01UWSzT1RPfQ4=', 'base64').toString() + 'ubi1leHBpOjl5SjI1NTAyMTY3OTh9Lm4zNTRfMU01TWZlTFB0aWFmUTRuTjRRaVlTdEs4TjhjQ3BOdzdlTFc5M1k='
  };

  private constructor() {
    this.configPath = path.join(os.homedir(), '.flow-nexus');
    this.userEnvPath = path.join(process.cwd(), '.env.user');
    this.systemConfigPath = path.join(this.configPath, 'config.json');
    this.loadUserConfig();
  }

  public static getInstance(): SecureConfigManager {
    if (!SecureConfigManager.instance) {
      SecureConfigManager.instance = new SecureConfigManager();
    }
    return SecureConfigManager.instance;
  }

  /**
   * Get Supabase configuration (obfuscated from user)
   */
  public getSupabaseConfig(): { url: string; anonKey: string } | null {
    // Only return if user is authenticated
    if (!this.userConfig.isAuthenticated) {
      return null;
    }
    
    return {
      url: this.OBFUSCATED_SUPABASE.url,
      anonKey: this.OBFUSCATED_SUPABASE.anonKey
    };
  }

  /**
   * Check if user is authenticated
   */
  public isAuthenticated(): boolean {
    return this.userConfig.isAuthenticated;
  }

  /**
   * Get authentication notice for unauthenticated users
   */
  public getAuthNotice(): string {
    return `
╔════════════════════════════════════════════════════════════╗
║                    🔐 AUTHENTICATION REQUIRED              ║
╠════════════════════════════════════════════════════════════╣
║                                                            ║
║  Welcome to Flow Nexus MCP Server!                        ║
║                                                            ║
║  You need to authenticate before using this service.      ║
║  Please run one of the following commands:                ║
║                                                            ║
║  For new users:                                           ║
║    $ mcp-flow init                                        ║
║    $ mcp-flow register --email your@email.com            ║
║                                                            ║
║  For existing users:                                      ║
║    $ mcp-flow login --email your@email.com               ║
║                                                            ║
║  This will create a .env.user file with your credentials  ║
║  that will be used for all future sessions.              ║
║                                                            ║
║  Benefits of registration:                                ║
║    • 1000 free credits to start                          ║
║    • Persistent sessions                                  ║
║    • Usage tracking and history                          ║
║    • Access to all MCP tools                             ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝
`;
  }

  /**
   * Load user configuration from .env.user file
   */
  private loadUserConfig(): void {
    try {
      // First try .env.user in current directory
      if (fs.existsSync(this.userEnvPath)) {
        const content = fs.readFileSync(this.userEnvPath, 'utf-8');
        this.parseUserEnv(content);
      } 
      // Then try system config directory
      else if (fs.existsSync(this.systemConfigPath)) {
        const content = fs.readFileSync(this.systemConfigPath, 'utf-8');
        const config = JSON.parse(content);
        this.userConfig = {
          ...config,
          isAuthenticated: true
        };
      }
      // Check for Flow Nexus credentials in main .env (legacy)
      else {
        const mainEnvPath = path.join(process.cwd(), '.env');
        if (fs.existsSync(mainEnvPath)) {
          const content = fs.readFileSync(mainEnvPath, 'utf-8');
          if (content.includes('FLOW_NEXUS_USER_ID')) {
            this.parseUserEnv(content);
          }
        }
      }
    } catch (error) {
      console.error('Failed to load user config:', error);
      this.userConfig = { isAuthenticated: false };
    }
  }

  /**
   * Parse user environment variables
   */
  private parseUserEnv(content: string): void {
    const lines = content.split('\n');
    const config: any = {};
    
    for (const line of lines) {
      if (line.startsWith('FLOW_NEXUS_')) {
        const [key, ...valueParts] = line.split('=');
        const value = valueParts.join('=');
        const configKey = key.replace('FLOW_NEXUS_', '').toLowerCase();
        
        switch (configKey) {
          case 'user_id':
            config.userId = value;
            break;
          case 'email':
            config.email = value;
            break;
          case 'api_key':
            config.apiKey = value;
            break;
          case 'access_token':
            config.accessToken = value;
            break;
          case 'refresh_token':
            config.refreshToken = value;
            break;
          case 'credits':
            config.credits = parseFloat(value);
            break;
        }
      }
    }

    if (config.userId && config.apiKey) {
      this.userConfig = {
        ...config,
        isAuthenticated: true
      };
    }
  }

  /**
   * Save user configuration to .env.user
   */
  public async saveUserConfig(config: Partial<UserConfig>): Promise<void> {
    this.userConfig = {
      ...this.userConfig,
      ...config,
      isAuthenticated: true
    };

    // Create .env.user file content
    const envContent = `# =====================================================
# FLOW NEXUS USER CONFIGURATION
# Generated: ${new Date().toISOString()}
# =====================================================
#
# This file contains your personal authentication credentials.
# Do NOT commit this file to version control.
# Do NOT share these credentials with anyone.
#
# To regenerate credentials, run: mcp-flow login
#
# =====================================================

# User Credentials
FLOW_NEXUS_USER_ID=${this.userConfig.userId || ''}
FLOW_NEXUS_EMAIL=${this.userConfig.email || ''}
FLOW_NEXUS_API_KEY=${this.userConfig.apiKey || ''}
FLOW_NEXUS_ACCESS_TOKEN=${this.userConfig.accessToken || ''}
FLOW_NEXUS_REFRESH_TOKEN=${this.userConfig.refreshToken || ''}
FLOW_NEXUS_CREDITS=${this.userConfig.credits || 0}

# Session Information
FLOW_NEXUS_AUTH_VERSION=2.0.0
FLOW_NEXUS_AUTH_CREATED=${new Date().toISOString()}
`;

    // Save to .env.user
    fs.writeFileSync(this.userEnvPath, envContent);

    // Also save to system config directory
    if (!fs.existsSync(this.configPath)) {
      fs.mkdirSync(this.configPath, { recursive: true });
    }
    
    fs.writeFileSync(
      this.systemConfigPath,
      JSON.stringify(this.userConfig, null, 2)
    );

    console.log(`\n✅ Credentials saved to ${this.userEnvPath}`);
  }

  /**
   * Clear user configuration (logout)
   */
  public clearUserConfig(): void {
    this.userConfig = { isAuthenticated: false };
    
    // Remove .env.user file
    if (fs.existsSync(this.userEnvPath)) {
      fs.unlinkSync(this.userEnvPath);
    }
    
    // Remove system config
    if (fs.existsSync(this.systemConfigPath)) {
      fs.unlinkSync(this.systemConfigPath);
    }
  }

  /**
   * Get current user configuration
   */
  public getUserConfig(): UserConfig {
    return this.userConfig;
  }

  /**
   * Create a template .env.user file for manual configuration
   */
  public createUserEnvTemplate(): void {
    const template = `# =====================================================
# FLOW NEXUS USER CONFIGURATION TEMPLATE
# =====================================================
#
# Instructions:
# 1. Run 'mcp-flow init' to automatically generate this file
# 2. Or manually fill in your credentials below
# 3. Rename this file to .env.user
#
# To get credentials:
#   - Register: mcp-flow register --email your@email.com
#   - Login: mcp-flow login --email your@email.com
#
# =====================================================

# Your User Credentials (required)
FLOW_NEXUS_USER_ID=your-user-id-here
FLOW_NEXUS_EMAIL=your-email@example.com
FLOW_NEXUS_API_KEY=your-api-key-here

# Session Tokens (optional - will be auto-generated)
FLOW_NEXUS_ACCESS_TOKEN=
FLOW_NEXUS_REFRESH_TOKEN=

# Account Information (optional)
FLOW_NEXUS_CREDITS=1000

# DO NOT MODIFY BELOW
FLOW_NEXUS_AUTH_VERSION=2.0.0
FLOW_NEXUS_AUTH_CREATED=${new Date().toISOString()}
`;

    const templatePath = path.join(process.cwd(), '.env.user.template');
    fs.writeFileSync(templatePath, template);
    console.log(`\n📝 Template created: ${templatePath}`);
    console.log('   Edit this file with your credentials and rename to .env.user');
  }
}

// Export singleton instance
export const secureConfig = SecureConfigManager.getInstance();