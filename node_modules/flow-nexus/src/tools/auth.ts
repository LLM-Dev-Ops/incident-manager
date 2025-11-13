/**
 * MCP Authentication Tool
 * Handles user login/registration and .env file management
 */

import { createClient, SupabaseClient } from '@supabase/supabase-js';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as crypto from 'crypto';
import * as dotenv from 'dotenv';
import { z } from 'zod';

// Validation schemas
const RegisterSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
  username: z.string().min(3).optional(),
  organizationName: z.string().optional()
});

const LoginSchema = z.object({
  email: z.string().email(),
  password: z.string()
});

export interface AuthCredentials {
  userId: string;
  email: string;
  accessToken: string;
  refreshToken: string;
  supabaseUrl: string;
  supabaseAnonKey: string;
  apiKey?: string;
  organizationId?: string;
  credits?: number;
}

export class MCPAuthTool {
  private supabase: SupabaseClient;
  private envPath: string;
  private credentials: AuthCredentials | null = null;

  constructor(
    private supabaseUrl: string = process.env.SUPABASE_URL || '',
    private supabaseAnonKey: string = process.env.SUPABASE_ANON_KEY || ''
  ) {
    this.supabase = createClient(this.supabaseUrl, this.supabaseAnonKey);
    this.envPath = path.resolve(process.cwd(), '.env');
  }

  /**
   * Register a new user
   */
  async register(params: {
    email: string;
    password: string;
    username?: string;
    organizationName?: string;
  }): Promise<{ success: boolean; credentials?: AuthCredentials; error?: string }> {
    try {
      // Validate input
      const validated = RegisterSchema.parse(params);

      // Create user in Supabase Auth
      const { data: authData, error: authError } = await this.supabase.auth.signUp({
        email: validated.email,
        password: validated.password,
        options: {
          data: {
            username: validated.username,
            organization_name: validated.organizationName
          }
        }
      });

      if (authError) {
        return { success: false, error: authError.message };
      }

      if (!authData.user || !authData.session) {
        return { success: false, error: 'Registration failed - no user created' };
      }

      // Create user profile in database
      const { data: profile, error: profileError } = await this.supabase
        .from('user_profiles')
        .insert({
          id: authData.user.id,
          email: validated.email,
          username: validated.username,
          credits: 1000, // Initial credits
          api_key: this.generateApiKey(),
          created_at: new Date().toISOString()
        })
        .select()
        .single();

      if (profileError) {
        console.error('Profile creation error:', profileError);
        // Continue anyway - auth is successful
      }

      // Create organization if provided
      let organizationId: string | undefined;
      if (validated.organizationName) {
        const { data: org, error: orgError } = await this.supabase
          .from('organizations')
          .insert({
            name: validated.organizationName,
            owner_id: authData.user.id,
            created_at: new Date().toISOString()
          })
          .select()
          .single();

        if (!orgError && org) {
          organizationId = org.id;
        }
      }

      // Prepare credentials
      this.credentials = {
        userId: authData.user.id,
        email: validated.email,
        accessToken: authData.session.access_token,
        refreshToken: authData.session.refresh_token,
        supabaseUrl: this.supabaseUrl,
        supabaseAnonKey: this.supabaseAnonKey,
        apiKey: profile?.api_key,
        organizationId,
        credits: profile?.credits || 1000
      };

      // Save to .env file
      await this.saveCredentialsToEnv(this.credentials);

      return {
        success: true,
        credentials: this.credentials
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
  async login(params: {
    email: string;
    password: string;
  }): Promise<{ success: boolean; credentials?: AuthCredentials; error?: string }> {
    try {
      // Validate input
      const validated = LoginSchema.parse(params);

      // Sign in with Supabase Auth
      const { data: authData, error: authError } = await this.supabase.auth.signInWithPassword({
        email: validated.email,
        password: validated.password
      });

      if (authError) {
        return { success: false, error: authError.message };
      }

      if (!authData.user || !authData.session) {
        return { success: false, error: 'Login failed - invalid credentials' };
      }

      // Get user profile
      const { data: profile, error: profileError } = await this.supabase
        .from('user_profiles')
        .select('*')
        .eq('id', authData.user.id)
        .single();

      if (profileError) {
        console.error('Profile fetch error:', profileError);
      }

      // Get user's organization
      const { data: orgMembership } = await this.supabase
        .from('organization_members')
        .select('organization_id')
        .eq('user_id', authData.user.id)
        .single();

      // Prepare credentials
      this.credentials = {
        userId: authData.user.id,
        email: validated.email,
        accessToken: authData.session.access_token,
        refreshToken: authData.session.refresh_token,
        supabaseUrl: this.supabaseUrl,
        supabaseAnonKey: this.supabaseAnonKey,
        apiKey: profile?.api_key,
        organizationId: orgMembership?.organization_id,
        credits: profile?.credits
      };

      // Save to .env file
      await this.saveCredentialsToEnv(this.credentials);

      return {
        success: true,
        credentials: this.credentials
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
  async logout(): Promise<{ success: boolean; error?: string }> {
    try {
      // Sign out from Supabase
      const { error } = await this.supabase.auth.signOut();
      
      if (error) {
        return { success: false, error: error.message };
      }

      // Clear credentials from memory
      this.credentials = null;

      // Remove auth-related vars from .env
      await this.removeAuthFromEnv();

      return { success: true };

    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Logout failed'
      };
    }
  }

  /**
   * Get current session
   */
  async getSession(): Promise<{ 
    success: boolean; 
    session?: any; 
    credentials?: AuthCredentials;
    error?: string 
  }> {
    try {
      // Try to get session from Supabase
      const { data: { session }, error } = await this.supabase.auth.getSession();

      if (error) {
        return { success: false, error: error.message };
      }

      if (!session) {
        // Try to load from .env
        const envCredentials = await this.loadCredentialsFromEnv();
        if (envCredentials) {
          this.credentials = envCredentials;
          return {
            success: true,
            credentials: envCredentials
          };
        }
        return { success: false, error: 'No active session' };
      }

      // Get user profile
      const { data: profile } = await this.supabase
        .from('user_profiles')
        .select('*')
        .eq('id', session.user.id)
        .single();

      this.credentials = {
        userId: session.user.id,
        email: session.user.email!,
        accessToken: session.access_token,
        refreshToken: session.refresh_token,
        supabaseUrl: this.supabaseUrl,
        supabaseAnonKey: this.supabaseAnonKey,
        apiKey: profile?.api_key,
        credits: profile?.credits
      };

      return {
        success: true,
        session,
        credentials: this.credentials
      };

    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to get session'
      };
    }
  }

  /**
   * Refresh current session
   */
  async refreshSession(): Promise<{ success: boolean; credentials?: AuthCredentials; error?: string }> {
    try {
      const { data: { session }, error } = await this.supabase.auth.refreshSession();

      if (error) {
        return { success: false, error: error.message };
      }

      if (!session) {
        return { success: false, error: 'Failed to refresh session' };
      }

      // Update credentials
      if (this.credentials) {
        this.credentials.accessToken = session.access_token;
        this.credentials.refreshToken = session.refresh_token;
        
        // Save updated credentials
        await this.saveCredentialsToEnv(this.credentials);
      }

      return {
        success: true,
        credentials: this.credentials!
      };

    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to refresh session'
      };
    }
  }

  /**
   * Generate API key for user
   */
  private generateApiKey(): string {
    const prefix = 'fln'; // Flow Nexus
    const timestamp = Date.now().toString(36);
    const random = crypto.randomBytes(16).toString('hex');
    return `${prefix}_${timestamp}_${random}`;
  }

  /**
   * Save credentials to .env file
   */
  private async saveCredentialsToEnv(credentials: AuthCredentials): Promise<void> {
    try {
      // Read existing .env file
      let envContent = '';
      try {
        envContent = await fs.readFile(this.envPath, 'utf-8');
      } catch (error) {
        // File doesn't exist, will create new one
      }

      // Parse existing env
      const env = dotenv.parse(envContent);

      // Update with new credentials
      env['FLOW_NEXUS_USER_ID'] = credentials.userId;
      env['FLOW_NEXUS_EMAIL'] = credentials.email;
      env['FLOW_NEXUS_ACCESS_TOKEN'] = credentials.accessToken;
      env['FLOW_NEXUS_REFRESH_TOKEN'] = credentials.refreshToken;
      env['FLOW_NEXUS_API_KEY'] = credentials.apiKey || '';
      env['FLOW_NEXUS_CREDITS'] = String(credentials.credits || 0);
      
      if (credentials.organizationId) {
        env['FLOW_NEXUS_ORG_ID'] = credentials.organizationId;
      }

      // Keep Supabase credentials
      if (!env['SUPABASE_URL']) {
        env['SUPABASE_URL'] = credentials.supabaseUrl;
      }
      if (!env['SUPABASE_ANON_KEY']) {
        env['SUPABASE_ANON_KEY'] = credentials.supabaseAnonKey;
      }

      // Add metadata
      env['FLOW_NEXUS_AUTH_CREATED'] = new Date().toISOString();
      env['FLOW_NEXUS_AUTH_VERSION'] = '1.0.0';

      // Build new .env content
      const newEnvContent = Object.entries(env)
        .map(([key, value]) => `${key}=${value}`)
        .join('\n');

      // Add header comment
      const header = `# =====================================================
# FLOW NEXUS AUTHENTICATION
# Generated: ${new Date().toISOString()}
# User: ${credentials.email}
# =====================================================

`;

      // Write to file
      await fs.writeFile(this.envPath, header + newEnvContent);

      console.log('✅ Credentials saved to .env file');

    } catch (error) {
      console.error('Failed to save credentials to .env:', error);
      throw error;
    }
  }

  /**
   * Load credentials from .env file
   */
  private async loadCredentialsFromEnv(): Promise<AuthCredentials | null> {
    try {
      const envContent = await fs.readFile(this.envPath, 'utf-8');
      const env = dotenv.parse(envContent);

      if (!env['FLOW_NEXUS_USER_ID'] || !env['FLOW_NEXUS_ACCESS_TOKEN']) {
        return null;
      }

      return {
        userId: env['FLOW_NEXUS_USER_ID'],
        email: env['FLOW_NEXUS_EMAIL'],
        accessToken: env['FLOW_NEXUS_ACCESS_TOKEN'],
        refreshToken: env['FLOW_NEXUS_REFRESH_TOKEN'],
        supabaseUrl: env['SUPABASE_URL'] || this.supabaseUrl,
        supabaseAnonKey: env['SUPABASE_ANON_KEY'] || this.supabaseAnonKey,
        apiKey: env['FLOW_NEXUS_API_KEY'],
        organizationId: env['FLOW_NEXUS_ORG_ID'],
        credits: parseFloat(env['FLOW_NEXUS_CREDITS'] || '0')
      };

    } catch (error) {
      return null;
    }
  }

  /**
   * Remove auth credentials from .env
   */
  private async removeAuthFromEnv(): Promise<void> {
    try {
      const envContent = await fs.readFile(this.envPath, 'utf-8');
      const env = dotenv.parse(envContent);

      // Remove Flow Nexus auth keys
      const keysToRemove = [
        'FLOW_NEXUS_USER_ID',
        'FLOW_NEXUS_EMAIL',
        'FLOW_NEXUS_ACCESS_TOKEN',
        'FLOW_NEXUS_REFRESH_TOKEN',
        'FLOW_NEXUS_API_KEY',
        'FLOW_NEXUS_CREDITS',
        'FLOW_NEXUS_ORG_ID',
        'FLOW_NEXUS_AUTH_CREATED',
        'FLOW_NEXUS_AUTH_VERSION'
      ];

      keysToRemove.forEach(key => delete env[key]);

      // Rebuild .env content
      const newEnvContent = Object.entries(env)
        .map(([key, value]) => `${key}=${value}`)
        .join('\n');

      await fs.writeFile(this.envPath, newEnvContent);

      console.log('✅ Auth credentials removed from .env');

    } catch (error) {
      console.error('Failed to remove auth from .env:', error);
    }
  }

  /**
   * Check credit balance
   */
  async checkCredits(): Promise<{ success: boolean; credits?: number; error?: string }> {
    try {
      if (!this.credentials) {
        const session = await this.getSession();
        if (!session.success) {
          return { success: false, error: 'Not authenticated' };
        }
      }

      const { data: profile, error } = await this.supabase
        .from('user_profiles')
        .select('credits')
        .eq('id', this.credentials!.userId)
        .single();

      if (error) {
        return { success: false, error: error.message };
      }

      return {
        success: true,
        credits: profile?.credits || 0
      };

    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to check credits'
      };
    }
  }
}

// Export tool functions for MCP
export const authTools = {
  name: 'auth',
  description: 'Authentication tools for Flow Nexus MCP',
  tools: {
    register: {
      description: 'Register a new user account',
      parameters: {
        type: 'object',
        properties: {
          email: { type: 'string', description: 'User email address' },
          password: { type: 'string', description: 'Password (min 8 characters)' },
          username: { type: 'string', description: 'Optional username' },
          organizationName: { type: 'string', description: 'Optional organization name' }
        },
        required: ['email', 'password']
      },
      handler: async (params: any) => {
        const auth = new MCPAuthTool();
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
        const auth = new MCPAuthTool();
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
        const auth = new MCPAuthTool();
        return await auth.logout();
      }
    },
    
    getSession: {
      description: 'Get current session and credentials',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new MCPAuthTool();
        return await auth.getSession();
      }
    },
    
    refreshSession: {
      description: 'Refresh authentication tokens',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new MCPAuthTool();
        return await auth.refreshSession();
      }
    },
    
    checkCredits: {
      description: 'Check current credit balance',
      parameters: {
        type: 'object',
        properties: {}
      },
      handler: async () => {
        const auth = new MCPAuthTool();
        return await auth.checkCredits();
      }
    }
  }
};