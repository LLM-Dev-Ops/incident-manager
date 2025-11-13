/**
 * MCP Session Bridge
 * Ensures session persistence across stateless MCP tool calls
 * Critical for Windows npx compatibility where each call is isolated
 */

import windowsSessionFix from './windows-session-fix.js';
import supabaseClient from './supabase-client.js';

class MCPSessionBridge {
  constructor() {
    this.sessionRestored = false;
    this.lastRestoredAt = null;
  }

  /**
   * Restore session before ANY tool execution
   * This MUST be called at the start of every tool handler
   */
  async ensureAuthenticated() {
    try {
      // Load persisted session
      const persistedSession = windowsSessionFix.loadSession();
      
      if (!persistedSession) {
        return {
          authenticated: false,
          user: null,
          message: 'No persisted session found'
        };
      }

      // Check if we already have an active session
      const { data: { session: currentSession } } = await supabaseClient.supabase.auth.getSession();
      
      // If no current session or it's different, restore the persisted one
      if (!currentSession || currentSession.access_token !== persistedSession.access_token) {
        try {
          // Set the session in Supabase client
          const { data, error } = await supabaseClient.supabase.auth.setSession({
            access_token: persistedSession.access_token,
            refresh_token: persistedSession.refresh_token
          });

          if (error) {
            // Session might be expired, try to refresh
            if (persistedSession.refresh_token) {
              const { data: refreshedData, error: refreshError } = await supabaseClient.supabase.auth.refreshSession({
                refresh_token: persistedSession.refresh_token
              });

              if (!refreshError && refreshedData.session) {
                // Save the refreshed session
                windowsSessionFix.saveSession(refreshedData.session);
                
                return {
                  authenticated: true,
                  user: refreshedData.user,
                  session: refreshedData.session,
                  message: 'Session refreshed successfully'
                };
              }
            }
            
            return {
              authenticated: false,
              user: null,
              message: 'Session expired or invalid',
              error: error.message
            };
          }

          if (data && data.session) {
            this.sessionRestored = true;
            this.lastRestoredAt = Date.now();
            
            return {
              authenticated: true,
              user: data.user,
              session: data.session,
              message: 'Session restored successfully'
            };
          }
        } catch (e) {
          return {
            authenticated: false,
            user: null,
            message: 'Failed to restore session',
            error: e.message
          };
        }
      }

      // Session already active
      return {
        authenticated: true,
        user: currentSession.user,
        session: currentSession,
        message: 'Session already active'
      };
      
    } catch (error) {
      return {
        authenticated: false,
        user: null,
        message: 'Session restoration error',
        error: error.message
      };
    }
  }

  /**
   * Save session after successful login
   */
  async saveSession(session) {
    if (!session) return false;
    
    // Add user data if not present
    if (!session.user && session.access_token) {
      const { data: { user } } = await supabaseClient.supabase.auth.getUser();
      if (user) {
        session.user = user;
      }
    }
    
    return windowsSessionFix.saveSession(session);
  }

  /**
   * Clear all sessions
   */
  clearSession() {
    this.sessionRestored = false;
    this.lastRestoredAt = null;
    return windowsSessionFix.clearSession();
  }

  /**
   * Check if authenticated (quick check without restoration)
   */
  async isAuthenticated() {
    const { data: { session } } = await supabaseClient.supabase.auth.getSession();
    return !!session;
  }

  /**
   * Get current user (with restoration)
   */
  async getCurrentUser() {
    const authResult = await this.ensureAuthenticated();
    return authResult.authenticated ? authResult.user : null;
  }
}

// Export singleton
export default new MCPSessionBridge();