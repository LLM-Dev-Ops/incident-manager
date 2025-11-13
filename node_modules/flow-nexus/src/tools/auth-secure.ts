/**
 * Secure MCP Authentication Tool
 * Handles user login/registration with obfuscated Supabase credentials
 */

import { createClient, SupabaseClient } from '@supabase/supabase-js';
import * as crypto from 'crypto';
import { z } from 'zod';
import { secureConfig } from '../config/secure-config';

// Validation schemas
const RegisterSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
  username: z.string().min(3).optional()
});

const LoginSchema = z.object({
  email: z.string().email(),
  password: z.string()
});

export class SecureMCPAuthTool {
  private supabase: SupabaseClient | null = null;

  constructor() {
    this.initializeSupabase();
  }

  /**
   * Initialize Supabase client with obfuscated credentials
   */
  private initializeSupabase(): boolean {
    const config = secureConfig.getSupabaseConfig();
    
    if (!config) {
      console.error('❌ Supabase configuration not available. Please authenticate first.');
      return false;
    }

    this.supabase = createClient(config.url, config.anonKey);
    return true;
  }

  /**
   * Check if user needs to authenticate
   */
  public checkAuth(): { authenticated: boolean; message?: string } {
    if (!secureConfig.isAuthenticated()) {
      return {
        authenticated: false,
        message: secureConfig.getAuthNotice()
      };
    }

    const userConfig = secureConfig.getUserConfig();
    return {
      authenticated: true,
      message: `Authenticated as: ${userConfig.email} (${userConfig.credits} credits)`
    };
  }

  /**
   * Initialize MCP for first-time users
   */
  public async init(): Promise<{ success: boolean; message: string }> {
    console.log(`
╔════════════════════════════════════════════════════════════╗
║                 🚀 FLOW NEXUS MCP INITIALIZATION           ║
╚════════════════════════════════════════════════════════════╝

Welcome to Flow Nexus MCP Server!

This will set up your authentication credentials.
You can either:
  1. Create a new account (recommended)
  2. Login with existing credentials
  3. Use a template for manual configuration

`);

    // Create template file
    secureConfig.createUserEnvTemplate();

    return {
      success: true,
      message: `
Initialization complete! Next steps:

Option 1 - Register (Recommended):
  $ mcp-flow register --email your@email.com --password YourPassword123

Option 2 - Login:
  $ mcp-flow login --email your@email.com --password YourPassword123

Option 3 - Manual Setup:
  1. Edit .env.user.template with your credentials
  2. Rename to .env.user
  3. Restart MCP server

Your credentials will be saved in .env.user (not tracked by git).
`
    };
  }

  /**
   * Register a new user
   */
  public async register(params: {
    email: string;
    password: string;
    username?: string;
  }): Promise<{ success: boolean; message?: string; error?: string }> {
    try {
      // Initialize Supabase with hardcoded credentials for registration
      const config = {
        url: Buffer.from('aHR0cHM6Ly9lb2p1Y2ducHNrb3Z0YWRmd2Zpci5zdXBhYmFzZS5jbw==', 'base64').toString(),
        anonKey: Buffer.from('ZXlKaGJHY2lPaUpJVXpJMU5pSXNJblI1Y0NJNklrcFhWQ0o5LmV5SnBjM01pT2lKemRYQmhZbUZ6WlNJc0luSmxaaUk2SW1WdmFuVmpaMjV3YzJ0dmRuUmhaR1ozWm1seUlpd2ljbTlzWlNJNkltRnViMjRpTENKcFlYUWlPakUzTXpRMk5EQTNPVGDZLEW1WjNCQ0l4TURFM01UWSzT1JPfQ4=', 'base64').toString() + 'ubi1leHBpOjl5SjI1NTAyMTY3OTh9Lm4zNTRfMU01TWZlTFB0aWFmUTRuTjRRaVlTdEs4TjhjQ3BOdzdlTFc5M1k='
      };
      
      const tempSupabase = createClient(config.url, config.anonKey);

      // Validate input
      const validated = RegisterSchema.parse(params);

      // Create user in Supabase Auth
      const { data: authData, error: authError } = await tempSupabase.auth.signUp({
        email: validated.email,
        password: validated.password,
        options: {
          data: {
            username: validated.username
          }
        }
      });

      if (authError) {
        return { 
          success: false, 
          error: authError.message 
        };
      }

      if (!authData.user || !authData.session) {
        return { 
          success: false, 
          error: 'Registration failed - no user created' 
        };
      }

      // Generate API key
      const apiKey = this.generateApiKey();

      // Save credentials to .env.user
      await secureConfig.saveUserConfig({
        userId: authData.user.id,
        email: validated.email,
        accessToken: authData.session.access_token,
        refreshToken: authData.session.refresh_token,
        apiKey: apiKey,
        credits: 1000
      });

      return {
        success: true,
        message: `
✅ Registration successful!

Your account has been created with:
  • Email: ${validated.email}
  • User ID: ${authData.user.id}
  • API Key: ${apiKey.substring(0, 20)}...
  • Credits: 1000

Credentials saved to: .env.user
You can now use all MCP tools and features.
`
      };

    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Registration failed'
      };
    }
  }

  /**
   * Login existing user
   */
  public async login(params: {
    email: string;
    password: string;
  }): Promise<{ success: boolean; message?: string; error?: string }> {
    try {
      // Initialize Supabase with hardcoded credentials for login
      const config = {
        url: Buffer.from('aHR0cHM6Ly9lb2p1Y2ducHNrb3Z0YWRmd2Zpci5zdXBhYmFzZS5jbw==', 'base64').toString(),
        anonKey: Buffer.from('ZXlKaGJHY2lPaUpJVXpJMU5pSXNJblI1Y0NJNklrcFhWQ0o5LmV5SnBjM01pT2lKemRYQmhZbUZ6WlNJc0luSmxaaUk2SW1WdmFuVmpaMjV3YzJ0dmRuUmhaR1ozWm1seUlpd2ljbTlzWlNJNkltRnViMjRpTENKcFlYUWlPakUzTXpRMk5EQTNPVGDZLEW1WjNCQ0l4TURFM01UWSzT1JPfQ4=', 'base64').toString() + 'ubi1leHBpOjl5SjI1NTAyMTY3OTh9Lm4zNTRfMU01TWZlTFB0aWFmUTRuTjRRaVlTdEs4TjhjQ3BOdzdlTFc5M1k='
      };
      
      const tempSupabase = createClient(config.url, config.anonKey);

      // Validate input
      const validated = LoginSchema.parse(params);

      // Sign in with Supabase Auth
      const { data: authData, error: authError } = await tempSupabase.auth.signInWithPassword({
        email: validated.email,
        password: validated.password
      });

      if (authError) {
        return { 
          success: false, 
          error: authError.message 
        };
      }

      if (!authData.user || !authData.session) {
        return { 
          success: false, 
          error: 'Login failed - invalid credentials' 
        };
      }

      // Generate API key if needed
      const apiKey = this.generateApiKey();

      // Save credentials to .env.user
      await secureConfig.saveUserConfig({
        userId: authData.user.id,
        email: validated.email,
        accessToken: authData.session.access_token,
        refreshToken: authData.session.refresh_token,
        apiKey: apiKey,
        credits: 1000
      });

      return {
        success: true,
        message: `
✅ Login successful!

Welcome back, ${validated.email}!
Your session has been restored.

Credentials saved to: .env.user
You can now use all MCP tools and features.
`
      };

    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Login failed'
      };
    }
  }

  /**
   * Logout current user
   */
  public async logout(): Promise<{ success: boolean; message: string }> {
    secureConfig.clearUserConfig();
    
    return {
      success: true,
      message: `
✅ Logged out successfully!

Your credentials have been removed from:
  • .env.user (deleted)
  • System config (cleared)

To use MCP again, please login or register.
`
    };
  }

  /**
   * Get current session info
   */
  public getSession(): { authenticated: boolean; user?: any; message?: string } {
    if (!secureConfig.isAuthenticated()) {
      return {
        authenticated: false,
        message: 'Not authenticated. Please run: mcp-flow init'
      };
    }

    const userConfig = secureConfig.getUserConfig();
    return {
      authenticated: true,
      user: {
        id: userConfig.userId,
        email: userConfig.email,
        credits: userConfig.credits,
        apiKey: userConfig.apiKey?.substring(0, 20) + '...'
      },
      message: 'Active session'
    };
  }

  /**
   * Check credit balance
   */
  public checkCredits(): { success: boolean; credits?: number; message?: string } {
    if (!secureConfig.isAuthenticated()) {
      return {
        success: false,
        message: 'Not authenticated. Please login first.'
      };
    }

    const userConfig = secureConfig.getUserConfig();
    return {
      success: true,
      credits: userConfig.credits || 0,
      message: `Current balance: ${userConfig.credits || 0} credits`
    };
  }

  /**
   * Generate API key for user
   */
  private generateApiKey(): string {
    const prefix = 'fln';
    const timestamp = Date.now().toString(36);
    const random = crypto.randomBytes(16).toString('hex');
    return `${prefix}_${timestamp}_${random}`;
  }
}

// Export tool functions for MCP
export const secureAuthTools = {
  name: 'auth',
  description: 'Secure authentication tools for Flow Nexus MCP',
  tools: {
    init: {
      description: 'Initialize MCP authentication (first-time setup)',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new SecureMCPAuthTool();
        return await auth.init();
      }
    },

    checkAuth: {
      description: 'Check authentication status',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new SecureMCPAuthTool();
        return auth.checkAuth();
      }
    },
    
    register: {
      description: 'Register a new user account',
      parameters: {
        type: 'object',
        properties: {
          email: { type: 'string', description: 'User email address' },
          password: { type: 'string', description: 'Password (min 8 characters)' },
          username: { type: 'string', description: 'Optional username' }
        },
        required: ['email', 'password']
      },
      handler: async (params: any) => {
        const auth = new SecureMCPAuthTool();
        return await auth.register(params);
      }
    },
    
    login: {
      description: 'Login to existing account',
      parameters: {
        type: 'object',
        properties: {
          email: { type: 'string', description: 'User email address' },
          password: { type: 'string', description: 'Password' }
        },
        required: ['email', 'password']
      },
      handler: async (params: any) => {
        const auth = new SecureMCPAuthTool();
        return await auth.login(params);
      }
    },
    
    logout: {
      description: 'Logout current user',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new SecureMCPAuthTool();
        return await auth.logout();
      }
    },
    
    session: {
      description: 'Get current session information',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new SecureMCPAuthTool();
        return auth.getSession();
      }
    },
    
    credits: {
      description: 'Check current credit balance',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new SecureMCPAuthTool();
        return auth.checkCredits();
      }
    }
  }
};