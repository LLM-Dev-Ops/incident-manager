import { createClient, SupabaseClient } from '@supabase/supabase-js';
import jwt from 'jsonwebtoken';
import crypto from 'crypto';

export interface UserContext {
  userId: string;
  email: string;
  apiKey: string;
  tier: 'free' | 'pro' | 'enterprise';
  limits: {
    swarms: number;
    agents: number;
    tasks: number;
    sandboxes: number;
    storage: string;
  };
  supabase: SupabaseClient;
}

export class UserIsolationMiddleware {
  private userContexts: Map<string, UserContext> = new Map();
  private supabaseUrl: string;
  private supabaseServiceKey: string;
  private jwtSecret: string;

  constructor() {
    // Load from environment
    this.supabaseUrl = process.env.SUPABASE_URL!;
    this.supabaseServiceKey = process.env.SUPABASE_SERVICE_KEY!;
    this.jwtSecret = process.env.JWT_SECRET!;
    
    // Validate user configuration on startup
    this.validateUserConfig();
  }

  private validateUserConfig(): void {
    const userEmail = process.env.FLOW_NEXUS_USER_EMAIL;
    const userApiKey = process.env.FLOW_NEXUS_API_KEY;
    const userId = process.env.FLOW_NEXUS_USER_ID;
    
    if (!userEmail || !userApiKey || !userId) {
      console.error(`
❌ User authentication not configured!

Please run the initialization script first:
  npm run init-user

This will:
  1. Create your Flow Nexus account
  2. Generate a secure API key
  3. Configure your .env file
  4. Set up user data isolation

Without authentication, most MCP tools will be disabled.
`);
      
      // Allow server to start but mark as unauthenticated
      process.env.FLOW_NEXUS_AUTHENTICATED = 'false';
    } else {
      process.env.FLOW_NEXUS_AUTHENTICATED = 'true';
      console.log(`✅ Authenticated as: ${userEmail}`);
    }
  }

  async getUserContext(apiKey?: string): Promise<UserContext | null> {
    // Use provided API key or fall back to environment
    const key = apiKey || process.env.FLOW_NEXUS_API_KEY;
    const userId = process.env.FLOW_NEXUS_USER_ID;
    
    if (!key || !userId) {
      return null;
    }
    
    // Check cache
    if (this.userContexts.has(key)) {
      return this.userContexts.get(key)!;
    }
    
    try {
      // Create user-specific Supabase client
      const supabase = createClient(this.supabaseUrl, this.supabaseServiceKey, {
        auth: {
          persistSession: false,
          autoRefreshToken: false
        },
        global: {
          headers: {
            'x-user-id': userId,
            'x-api-key': key
          }
        }
      });
      
      // Fetch user profile
      const { data: profile, error } = await supabase
        .from('user_profiles')
        .select('*')
        .eq('id', userId)
        .eq('api_key', key)
        .single();
      
      if (error || !profile) {
        console.error('Failed to fetch user profile:', error);
        return null;
      }
      
      // Create user context
      const context: UserContext = {
        userId: profile.id,
        email: profile.email,
        apiKey: key,
        tier: profile.tier || 'free',
        limits: profile.settings?.limits || {
          swarms: 3,
          agents: 10,
          tasks: 100,
          sandboxes: 5,
          storage: '1GB'
        },
        supabase
      };
      
      // Cache the context
      this.userContexts.set(key, context);
      
      return context;
      
    } catch (error) {
      console.error('Error creating user context:', error);
      return null;
    }
  }

  // Apply RLS policies to ensure data isolation
  async applyUserScope(supabase: SupabaseClient, userId: string): Promise<void> {
    // Set the user context for RLS
    await supabase.rpc('set_user_context', { user_id: userId });
  }

  // Check if a resource belongs to the user
  async validateOwnership(
    supabase: SupabaseClient,
    table: string,
    resourceId: string,
    userId: string
  ): Promise<boolean> {
    const { data, error } = await supabase
      .from(table)
      .select('user_id')
      .eq('id', resourceId)
      .single();
    
    if (error || !data) {
      return false;
    }
    
    return data.user_id === userId;
  }

  // Check resource limits
  async checkResourceLimit(
    context: UserContext,
    resource: keyof UserContext['limits'],
    requestedCount: number = 1
  ): Promise<{ allowed: boolean; current: number; limit: number }> {
    // Get current usage
    let currentUsage = 0;
    
    switch (resource) {
      case 'swarms':
        const { count: swarmCount } = await context.supabase
          .from('swarms')
          .select('*', { count: 'exact', head: true })
          .eq('user_id', context.userId);
        currentUsage = swarmCount || 0;
        break;
        
      case 'agents':
        const { count: agentCount } = await context.supabase
          .from('agents')
          .select('*', { count: 'exact', head: true })
          .eq('user_id', context.userId);
        currentUsage = agentCount || 0;
        break;
        
      case 'tasks':
        const { count: taskCount } = await context.supabase
          .from('tasks')
          .select('*', { count: 'exact', head: true })
          .eq('user_id', context.userId)
          .gte('created_at', new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString());
        currentUsage = taskCount || 0;
        break;
        
      case 'sandboxes':
        const { count: sandboxCount } = await context.supabase
          .from('sandboxes')
          .select('*', { count: 'exact', head: true })
          .eq('user_id', context.userId)
          .eq('status', 'active');
        currentUsage = sandboxCount || 0;
        break;
    }
    
    const limit = context.limits[resource] as number;
    const allowed = currentUsage + requestedCount <= limit;
    
    return {
      allowed,
      current: currentUsage,
      limit
    };
  }

  // Generate user-specific JWT token
  generateUserToken(userId: string, email: string): string {
    return jwt.sign(
      {
        userId,
        email,
        iat: Date.now(),
        exp: Date.now() + 7 * 24 * 60 * 60 * 1000 // 7 days
      },
      this.jwtSecret
    );
  }

  // Verify user token
  verifyUserToken(token: string): { userId: string; email: string } | null {
    try {
      const decoded = jwt.verify(token, this.jwtSecret) as any;
      return {
        userId: decoded.userId,
        email: decoded.email
      };
    } catch {
      return null;
    }
  }

  // Clear user context cache
  clearUserContext(apiKey: string): void {
    this.userContexts.delete(apiKey);
  }

  // Get authenticated user from environment
  getAuthenticatedUser(): { userId: string; email: string; apiKey: string } | null {
    const userId = process.env.FLOW_NEXUS_USER_ID;
    const email = process.env.FLOW_NEXUS_USER_EMAIL;
    const apiKey = process.env.FLOW_NEXUS_API_KEY;
    
    if (!userId || !email || !apiKey) {
      return null;
    }
    
    return { userId, email, apiKey };
  }
}

// Export singleton instance
export const userIsolation = new UserIsolationMiddleware();