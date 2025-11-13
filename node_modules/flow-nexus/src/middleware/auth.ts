import * as jwt from 'jsonwebtoken';
import { UserProfile, AuthResult } from '../types';

export interface MCPContext {
  user?: UserProfile;
  tool?: { name: string };
  [key: string]: any;
}

export interface MCPMiddleware {
  process(context: MCPContext, next: () => Promise<any>): Promise<any>;
}

export class AuthMiddleware implements MCPMiddleware {
  private jwtSecret: string;

  constructor(jwtSecret: string) {
    this.jwtSecret = jwtSecret;
  }

  async process(context: MCPContext, next: () => Promise<any>): Promise<any> {
    // Extract token from context or headers
    const token = this.extractToken(context);
    
    if (!token) {
      throw new Error('Authentication required');
    }

    try {
      const authResult = await this.authenticateToken(token);
      
      if (!authResult.success || !authResult.user) {
        throw new Error('Invalid authentication token');
      }

      // Add user to context
      context.user = authResult.user;

      // Check permissions for the requested tool
      if (context.tool && !this.hasPermission(authResult.user, context.tool.name)) {
        throw new Error(`Insufficient permissions for ${context.tool.name}`);
      }

      return await next();
    } catch (error) {
      throw new Error(`Authentication failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  private extractToken(context: MCPContext): string | null {
    // Try to extract token from various sources
    if (context.headers?.authorization) {
      const authHeader = context.headers.authorization;
      if (authHeader.startsWith('Bearer ')) {
        return authHeader.substring(7);
      }
    }

    if (context.token) {
      return context.token;
    }

    if (context.auth?.token) {
      return context.auth.token;
    }

    return null;
  }

  private async authenticateToken(token: string): Promise<AuthResult> {
    try {
      const decoded = jwt.verify(token, this.jwtSecret) as any;
      
      // Validate token payload
      if (!decoded.sub || !decoded.aud || decoded.aud !== 'claude-agents') {
        return {
          success: false,
          error: 'Invalid token payload'
        };
      }

      // In a real implementation, you would fetch user data from the database
      // For now, we'll extract user info from the token
      const user: UserProfile = {
        id: decoded.sub,
        username: decoded.username || 'unknown',
        developer_level: decoded.developer_level || 1,
        permissions: this.getPermissions(decoded.developer_level || 1)
      };

      return {
        success: true,
        user
      };
    } catch (error) {
      return {
        success: false,
        error: 'Invalid token'
      };
    }
  }

  private getPermissions(level: number): string[] {
    const basePermissions = [
      'daa_agent_list',
      'daa_agent_metrics',
      'daa_assess_quality',
      'daa_analyze_pricing',
      'daa_security_scan',
      'daa_generate_recommendations'
    ];

    if (level >= 3) {
      basePermissions.push(
        'daa_agent_spawn',
        'daa_agent_execute',
        'daa_agent_train'
      );
    }

    if (level >= 5) {
      basePermissions.push(
        'daa_agent_terminate',
        'admin:metrics',
        'admin:health'
      );
    }

    return basePermissions;
  }

  private hasPermission(user: UserProfile, toolName: string): boolean {
    // Check if user has permission for the specific tool
    return user.permissions.includes(toolName) || user.permissions.includes('admin:all');
  }

  // Static method to create JWT tokens (for testing)
  static createToken(user: UserProfile, secret: string, expiresIn: string = '24h'): string {
    return jwt.sign(
      {
        sub: user.id,
        username: user.username,
        developer_level: user.developer_level,
        aud: 'claude-agents',
        iss: 'flow-cloud-app-store'
      },
      secret,
      { expiresIn: expiresIn }
    );
  }
}