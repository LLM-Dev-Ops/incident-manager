import { userIsolation, UserContext } from '../middleware/user-isolation';

// List of tools that require authentication
const AUTHENTICATED_TOOLS = [
  // Swarm management
  'swarm_init',
  'swarm_status',
  'swarm_monitor',
  'agent_spawn',
  'agent_list',
  'agent_metrics',
  'task_orchestrate',
  'task_status',
  'task_results',
  
  // DAA tools
  'daa_agent_create',
  'daa_workflow_create',
  'daa_workflow_execute',
  'daa_knowledge_share',
  'daa_learning_status',
  
  // Sandbox tools
  'sandbox_create',
  'sandbox_execute',
  'sandbox_list',
  'sandbox_stop',
  'sandbox_configure',
  'sandbox_delete',
  'sandbox_status',
  'sandbox_upload',
  'sandbox_logs',
  
  // App store tools
  'app_store_publish_app',
  'app_store_complete_challenge',
  'app_store_earn_ruv',
  'challenge_submit',
  
  // User-specific tools
  'ruv_balance',
  'ruv_history',
  'achievements_list',
  'user_stats',
  'user_profile',
  'user_update_profile',
  'app_installed',
  
  // Storage tools
  'storage_upload',
  'storage_delete',
  'storage_list',
  'storage_get_url',
  
  // Workflow tools
  'workflow_create',
  'workflow_execute',
  'workflow_export',
  
  // Real-time tools
  'realtime_subscribe',
  'realtime_unsubscribe',
  'realtime_list',
  'execution_stream_subscribe',
  'execution_stream_status',
  'execution_files_list',
  'execution_file_get'
];

// List of public tools that don't require authentication
const PUBLIC_TOOLS = [
  // Auth tools
  'auth_init',
  'auth_status',
  'user_register',
  'user_login',
  'user_logout',
  'user_verify_email',
  'user_reset_password',
  'user_update_password',
  
  // Read-only tools
  'template_list',
  'template_get',
  'app_store_list_templates',
  'challenges_list',
  'challenge_get',
  'leaderboard_get',
  'app_get',
  'app_search',
  'app_analytics',
  'system_health',
  'market_data',
  
  // Feature detection
  'features_detect',
  'benchmark_run',
  'neural_status',
  'neural_patterns',
  'memory_usage'
];

export interface AuthenticatedToolContext {
  user: UserContext;
  originalParams: any;
  toolName: string;
}

export class AuthenticatedToolWrapper {
  private userContext: UserContext | null = null;
  
  constructor() {
    this.initialize();
  }
  
  private async initialize() {
    // Try to get user context from environment
    const user = userIsolation.getAuthenticatedUser();
    if (user) {
      this.userContext = await userIsolation.getUserContext(user.apiKey);
      if (this.userContext) {
        console.log(`✅ User authenticated: ${user.email}`);
      }
    }
  }
  
  // Check if a tool requires authentication
  requiresAuth(toolName: string): boolean {
    return AUTHENTICATED_TOOLS.includes(toolName);
  }
  
  // Check if a tool is public
  isPublicTool(toolName: string): boolean {
    return PUBLIC_TOOLS.includes(toolName);
  }
  
  // Wrap tool execution with authentication
  async executeAuthenticatedTool(
    toolName: string,
    params: any,
    executor: (context: AuthenticatedToolContext) => Promise<any>
  ): Promise<any> {
    // Check if tool requires authentication
    if (!this.requiresAuth(toolName)) {
      // Public tool - execute without authentication
      return executor({
        user: null as any,
        originalParams: params,
        toolName
      });
    }
    
    // Get user context
    const apiKey = params.apiKey || process.env.FLOW_NEXUS_API_KEY;
    const userContext = await userIsolation.getUserContext(apiKey);
    
    if (!userContext) {
      return {
        error: 'Authentication required',
        message: `Please login to Flow Nexus using: mcp__flow-nexus__user_login with your email and password`,
        tool: toolName,
        helpUrl: 'https://flow-nexus.com/docs/authentication'
      };
    }
    
    // Apply user scope to parameters
    const scopedParams = this.applyScopeToParams(params, userContext);
    
    // Check resource limits if applicable
    const limitCheck = await this.checkToolLimits(toolName, scopedParams, userContext);
    if (!limitCheck.allowed) {
      return {
        error: 'Resource limit exceeded',
        message: `You have reached your ${limitCheck.resource} limit (${limitCheck.current}/${limitCheck.limit})`,
        tier: userContext.tier,
        upgradeUrl: 'https://flow-nexus.com/upgrade'
      };
    }
    
    // Execute the tool with user context
    try {
      const result = await executor({
        user: userContext,
        originalParams: scopedParams,
        toolName
      });
      
      // Log tool usage for analytics
      await this.logToolUsage(userContext, toolName, params, result);
      
      return result;
      
    } catch (error: any) {
      console.error(`Tool execution failed for ${toolName}:`, error);
      return {
        error: 'Tool execution failed',
        message: error.message,
        tool: toolName
      };
    }
  }
  
  // Apply user scope to tool parameters
  private applyScopeToParams(params: any, userContext: UserContext): any {
    const scoped = { ...params };
    
    // Always add user_id to parameters
    scoped.user_id = userContext.userId;
    
    // Remove any attempts to access other users' data
    delete scoped.other_user_id;
    delete scoped.target_user_id;
    
    // For queries, add user filter
    if (scoped.filter) {
      scoped.filter = `(${scoped.filter}) AND user_id = '${userContext.userId}'`;
    } else if (scoped.where) {
      scoped.where = { ...scoped.where, user_id: userContext.userId };
    }
    
    return scoped;
  }
  
  // Check tool-specific resource limits
  private async checkToolLimits(
    toolName: string,
    params: any,
    userContext: UserContext
  ): Promise<{ allowed: boolean; resource?: string; current?: number; limit?: number }> {
    // Check limits based on tool type
    switch (toolName) {
      case 'swarm_init':
        return await userIsolation.checkResourceLimit(userContext, 'swarms', 1);
        
      case 'agent_spawn':
        return await userIsolation.checkResourceLimit(userContext, 'agents', 1);
        
      case 'task_orchestrate':
        return await userIsolation.checkResourceLimit(userContext, 'tasks', 1);
        
      case 'sandbox_create':
        return await userIsolation.checkResourceLimit(userContext, 'sandboxes', 1);
        
      default:
        return { allowed: true };
    }
  }
  
  // Log tool usage for analytics and rate limiting
  private async logToolUsage(
    userContext: UserContext,
    toolName: string,
    params: any,
    result: any
  ): Promise<void> {
    try {
      await userContext.supabase
        .from('tool_usage_logs')
        .insert({
          user_id: userContext.userId,
          tool_name: toolName,
          parameters: params,
          result_status: result.error ? 'error' : 'success',
          error_message: result.error,
          executed_at: new Date().toISOString()
        });
    } catch (error) {
      // Silent fail - don't interrupt tool execution
      console.error('Failed to log tool usage:', error);
    }
  }
  
  // Get user-specific data only
  async getUserData(table: string, userContext: UserContext): Promise<any[]> {
    const { data, error } = await userContext.supabase
      .from(table)
      .select('*')
      .eq('user_id', userContext.userId);
    
    if (error) {
      throw error;
    }
    
    return data || [];
  }
  
  // Create user-owned resource
  async createUserResource(
    table: string,
    data: any,
    userContext: UserContext
  ): Promise<any> {
    const resourceData = {
      ...data,
      user_id: userContext.userId,
      created_at: new Date().toISOString()
    };
    
    const { data: created, error } = await userContext.supabase
      .from(table)
      .insert(resourceData)
      .select()
      .single();
    
    if (error) {
      throw error;
    }
    
    return created;
  }
  
  // Update user-owned resource
  async updateUserResource(
    table: string,
    id: string,
    updates: any,
    userContext: UserContext
  ): Promise<any> {
    // First verify ownership
    const owns = await userIsolation.validateOwnership(
      userContext.supabase,
      table,
      id,
      userContext.userId
    );
    
    if (!owns) {
      throw new Error('Resource not found or access denied');
    }
    
    const { data, error } = await userContext.supabase
      .from(table)
      .update({
        ...updates,
        updated_at: new Date().toISOString()
      })
      .eq('id', id)
      .eq('user_id', userContext.userId)
      .select()
      .single();
    
    if (error) {
      throw error;
    }
    
    return data;
  }
  
  // Delete user-owned resource
  async deleteUserResource(
    table: string,
    id: string,
    userContext: UserContext
  ): Promise<boolean> {
    // First verify ownership
    const owns = await userIsolation.validateOwnership(
      userContext.supabase,
      table,
      id,
      userContext.userId
    );
    
    if (!owns) {
      throw new Error('Resource not found or access denied');
    }
    
    const { error } = await userContext.supabase
      .from(table)
      .delete()
      .eq('id', id)
      .eq('user_id', userContext.userId);
    
    if (error) {
      throw error;
    }
    
    return true;
  }
}

// Export singleton instance
export const authenticatedTools = new AuthenticatedToolWrapper();