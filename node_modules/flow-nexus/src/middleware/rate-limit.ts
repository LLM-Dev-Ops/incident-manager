import Redis from 'ioredis';
import { MCPContext, MCPMiddleware } from './auth';

export interface RateLimit {
  requests: number;
  window: number; // seconds
}

export class RateLimitMiddleware implements MCPMiddleware {
  private redis?: Redis;
  private memoryStore: Map<string, { count: number; resetTime: number }> = new Map();
  private limits: Map<string, RateLimit>;

  constructor(redisUrl?: string) {
    if (redisUrl) {
      this.redis = new Redis(redisUrl);
    }

    // Define rate limits for different tools
    this.limits = new Map([
      // Basic operations - higher limits
      ['daa_agent_list', { requests: 1000, window: 3600 }],
      ['daa_agent_metrics', { requests: 500, window: 3600 }],
      ['daa_assess_quality', { requests: 100, window: 3600 }],
      ['daa_analyze_pricing', { requests: 200, window: 3600 }],
      ['daa_security_scan', { requests: 50, window: 3600 }],
      ['daa_generate_recommendations', { requests: 300, window: 3600 }],
      
      // Administrative operations - lower limits
      ['daa_agent_spawn', { requests: 20, window: 3600 }],
      ['daa_agent_execute', { requests: 200, window: 3600 }],
      ['daa_agent_train', { requests: 10, window: 3600 }],
      ['daa_agent_terminate', { requests: 10, window: 3600 }],
      
      // Default limit for unlisted tools
      ['default', { requests: 100, window: 3600 }]
    ]);
  }

  async process(context: MCPContext, next: () => Promise<any>): Promise<any> {
    if (!context.user || !context.tool) {
      return await next();
    }

    const userId = context.user.id;
    const toolName = context.tool.name;
    const limit = this.limits.get(toolName) || this.limits.get('default')!;
    
    try {
      const allowed = await this.checkRateLimit(userId, toolName, limit);
      
      if (!allowed) {
        throw new Error(`Rate limit exceeded for ${toolName}. Max ${limit.requests} requests per ${limit.window} seconds.`);
      }

      return await next();
    } catch (error) {
      if (error instanceof Error && error.message.includes('Rate limit exceeded')) {
        throw error;
      }
      
      // If rate limiting fails, allow the request but log the error
      console.error('Rate limit check failed:', error);
      return await next();
    }
  }

  private async checkRateLimit(userId: string, toolName: string, limit: RateLimit): Promise<boolean> {
    const key = `rate_limit:${userId}:${toolName}`;
    
    if (this.redis) {
      return await this.checkRedisRateLimit(key, limit);
    } else {
      return this.checkMemoryRateLimit(key, limit);
    }
  }

  private async checkRedisRateLimit(key: string, limit: RateLimit): Promise<boolean> {
    if (!this.redis) {
      throw new Error('Redis not available');
    }

    const pipeline = this.redis.pipeline();
    pipeline.incr(key);
    pipeline.expire(key, limit.window);
    
    const results = await pipeline.exec();
    
    if (!results || results.length < 1 || results[0][1] === null) {
      throw new Error('Redis pipeline failed');
    }

    const count = results[0][1] as number;
    return count <= limit.requests;
  }

  private checkMemoryRateLimit(key: string, limit: RateLimit): boolean {
    const now = Date.now();
    const stored = this.memoryStore.get(key);
    
    if (!stored || now > stored.resetTime) {
      // First request or window expired
      this.memoryStore.set(key, {
        count: 1,
        resetTime: now + (limit.window * 1000)
      });
      return true;
    }
    
    if (stored.count >= limit.requests) {
      return false;
    }
    
    stored.count++;
    return true;
  }

  // Clean up expired entries from memory store
  private cleanupMemoryStore(): void {
    const now = Date.now();
    for (const [key, value] of this.memoryStore.entries()) {
      if (now > value.resetTime) {
        this.memoryStore.delete(key);
      }
    }
  }

  // Get current rate limit status for a user/tool combination
  async getRateLimitStatus(userId: string, toolName: string): Promise<{
    limit: number;
    remaining: number;
    resetTime: number;
  }> {
    const limit = this.limits.get(toolName) || this.limits.get('default')!;
    const key = `rate_limit:${userId}:${toolName}`;
    
    if (this.redis) {
      const count = await this.redis.get(key);
      const ttl = await this.redis.ttl(key);
      
      return {
        limit: limit.requests,
        remaining: Math.max(0, limit.requests - (parseInt(count || '0'))),
        resetTime: ttl > 0 ? Date.now() + (ttl * 1000) : 0
      };
    } else {
      const stored = this.memoryStore.get(key);
      
      if (!stored || Date.now() > stored.resetTime) {
        return {
          limit: limit.requests,
          remaining: limit.requests,
          resetTime: 0
        };
      }
      
      return {
        limit: limit.requests,
        remaining: Math.max(0, limit.requests - stored.count),
        resetTime: stored.resetTime
      };
    }
  }

  // Update rate limits dynamically
  updateRateLimit(toolName: string, limit: RateLimit): void {
    this.limits.set(toolName, limit);
  }

  // Get all configured rate limits
  getRateLimits(): Map<string, RateLimit> {
    return new Map(this.limits);
  }

  // Cleanup resources
  async cleanup(): Promise<void> {
    if (this.redis) {
      await this.redis.quit();
    }
    this.memoryStore.clear();
  }

  // Start periodic cleanup of memory store
  startCleanupInterval(intervalMs: number = 300000): NodeJS.Timeout {
    return setInterval(() => {
      this.cleanupMemoryStore();
    }, intervalMs);
  }
}