/**
 * MCP Authentication Bridge
 * Bridges authentication between Supabase and MCP tools
 * Ensures session context is available to all MCP tools
 */

import crossPlatformSession from './cross-platform-session.js';
import supabaseClient from './supabase-client.js';

class MCPAuthBridge {
  constructor() {
    this.currentSession = null;
    this.authContext = null;
    this.initialized = false;
  }

  /**
   * Initialize the auth bridge
   */
  async initialize() {
    if (this.initialized) {
      return this.authContext;
    }

    try {
      // Load session from cross-platform storage
      const storedSession = crossPlatformSession.loadSession();
      
      if (storedSession) {
        // Validate and refresh if needed
        const validated = await this.validateSession(storedSession);
        
        if (validated) {
          this.currentSession = validated;
          this.authContext = this.createAuthContext(validated);
          this.initialized = true;
          
          console.log('✅ MCP Auth Bridge initialized with existing session');
          return this.authContext;
        }
      }
      
      // No valid session found
      this.authContext = {
        authenticated: false,
        user: null,
        userId: null,
        email: null,
        credits: 0,
        message: 'Not authenticated. Please use auth_init to login.'
      };
      
      return this.authContext;
    } catch (error) {
      console.error('Auth bridge initialization error:', error);
      
      return {
        authenticated: false,
        error: error.message
      };
    }
  }

  /**
   * Validate session with Supabase
   */
  async validateSession(session) {
    try {
      // Set session in Supabase client
      const { data, error } = await supabaseClient.supabase.auth.setSession({
        access_token: session.access_token,
        refresh_token: session.refresh_token
      });
      
      if (error) {
        console.log('Session validation failed:', error.message);
        return null;
      }
      
      // Get fresh user data
      const { data: { user }, error: userError } = await supabaseClient.supabase.auth.getUser();
      
      if (userError || !user) {
        console.log('Failed to get user:', userError?.message);
        return null;
      }
      
      // Session is valid, return updated session
      return {
        ...data.session,
        user: user
      };
    } catch (error) {
      console.error('Session validation error:', error);
      return null;
    }
  }

  /**
   * Create auth context for MCP tools
   */
  createAuthContext(session) {
    if (!session || !session.user) {
      return {
        authenticated: false,
        user: null
      };
    }
    
    return {
      authenticated: true,
      user: session.user,
      userId: session.user.id,
      email: session.user.email,
      session: {
        access_token: session.access_token,
        refresh_token: session.refresh_token,
        expires_at: session.expires_at
      },
      credits: session.user.user_metadata?.credits || 0,
      tier: session.user.user_metadata?.tier || 'free'
    };
  }

  /**
   * Check if user is authenticated
   */
  async isAuthenticated() {
    if (!this.initialized) {
      await this.initialize();
    }
    
    return this.authContext?.authenticated === true;
  }

  /**
   * Get current auth context
   */
  async getAuthContext() {
    if (!this.initialized) {
      await this.initialize();
    }
    
    return this.authContext;
  }

  /**
   * Login and update auth context
   */
  async login(email, password) {
    try {
      // Use supabaseClient to login
      const result = await supabaseClient.login(email, password);
      
      if (result && result.session) {
        // Save to cross-platform storage
        crossPlatformSession.saveSession(result.session);
        
        // Update auth context
        this.currentSession = result.session;
        this.authContext = this.createAuthContext(result.session);
        this.initialized = true;
        
        return {
          success: true,
          session: result.session,
          authContext: this.authContext
        };
      }
      
      return {
        success: false,
        error: 'Login failed'
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Register new user
   */
  async register(email, password) {
    try {
      const result = await supabaseClient.register(email, password);
      
      if (result && result.session) {
        // Save to cross-platform storage
        crossPlatformSession.saveSession(result.session);
        
        // Update auth context
        this.currentSession = result.session;
        this.authContext = this.createAuthContext(result.session);
        this.initialized = true;
        
        return {
          success: true,
          session: result.session,
          authContext: this.authContext
        };
      }
      
      return {
        success: false,
        error: 'Registration failed'
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Logout and clear auth context
   */
  async logout() {
    try {
      await supabaseClient.logout();
      crossPlatformSession.clearSession();
      
      this.currentSession = null;
      this.authContext = {
        authenticated: false,
        user: null
      };
      
      return {
        success: true,
        message: 'Logged out successfully'
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Refresh session if needed
   */
  async refreshSession() {
    try {
      const { data, error } = await supabaseClient.supabase.auth.refreshSession();
      
      if (error) {
        throw error;
      }
      
      if (data.session) {
        // Save refreshed session
        crossPlatformSession.saveSession(data.session);
        
        // Update auth context
        this.currentSession = data.session;
        this.authContext = this.createAuthContext(data.session);
        
        return {
          success: true,
          session: data.session
        };
      }
      
      return {
        success: false,
        error: 'Failed to refresh session'
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Check if a tool requires authentication
   */
  requiresAuth(toolName) {
    const authRequiredTools = [
      'seraphina_chat',
      'seraphina_seek_audience',
      'swarm_init',
      'swarm_spawn',
      'neural_train',
      'neural_predict',
      'app_publish',
      'app_update',
      'challenge_create',
      'challenge_attempt',
      'ruv_transfer',
      'ruv_balance'
    ];
    
    return authRequiredTools.includes(toolName);
  }

  /**
   * Wrap tool execution with auth check
   */
  async executeWithAuth(toolName, toolFunction, args) {
    // Check if tool requires auth
    if (!this.requiresAuth(toolName)) {
      // No auth required, execute directly
      return await toolFunction(args);
    }
    
    // Initialize auth if needed
    if (!this.initialized) {
      await this.initialize();
    }
    
    // Check authentication
    if (!this.authContext?.authenticated) {
      return {
        error: 'Authentication required',
        message: 'Please login using auth_init tool first',
        requiresAuth: true
      };
    }
    
    // Check for expired session and refresh
    if (this.currentSession?.expires_at) {
      const expiresAt = new Date(this.currentSession.expires_at * 1000);
      const now = new Date();
      
      if (expiresAt <= now) {
        const refreshResult = await this.refreshSession();
        
        if (!refreshResult.success) {
          return {
            error: 'Session expired',
            message: 'Please login again using auth_init tool',
            requiresAuth: true
          };
        }
      }
    }
    
    // Execute tool with auth context
    try {
      const result = await toolFunction({
        ...args,
        authContext: this.authContext,
        userId: this.authContext.userId,
        userEmail: this.authContext.email
      });
      
      return result;
    } catch (error) {
      // Check if error is auth-related
      if (error.message?.includes('auth') || error.message?.includes('unauthorized')) {
        return {
          error: 'Authentication failed',
          message: 'Please login again using auth_init tool',
          requiresAuth: true
        };
      }
      
      throw error;
    }
  }
}

// Export singleton instance
export default new MCPAuthBridge();