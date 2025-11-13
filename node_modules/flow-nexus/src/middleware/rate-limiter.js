/**
 * Rate Limiter Middleware for Payment Operations
 * Prevents abuse and ensures API stability
 */

import crypto from 'crypto';

// Rate limit configurations per operation
const RATE_LIMITS = {
  check_balance: {
    requests: 30,      // 30 requests
    window: 60000,     // per minute
    blockDuration: 300000, // 5 minute block if exceeded
  },
  create_payment_link: {
    requests: 5,       // 5 payment links
    window: 3600000,   // per hour
    blockDuration: 3600000, // 1 hour block
  },
  configure_auto_refill: {
    requests: 10,      // 10 config changes
    window: 3600000,   // per hour
    blockDuration: 1800000, // 30 minute block
  },
  get_payment_history: {
    requests: 20,      // 20 requests
    window: 60000,     // per minute
    blockDuration: 300000, // 5 minute block
  },
  create_subscription: {
    requests: 3,       // 3 subscription attempts
    window: 86400000,  // per day
    blockDuration: 3600000, // 1 hour block
  },
  reduce_credits: {
    requests: 10,      // 10 admin operations
    window: 3600000,   // per hour
    blockDuration: 1800000, // 30 minute block
  },
  global: {
    requests: 100,     // 100 total payment operations
    window: 60000,     // per minute
    blockDuration: 600000, // 10 minute block
  },
};

// In-memory store (use Redis in production)
class RateLimitStore {
  constructor() {
    this.requests = new Map();
    this.blocks = new Map();
    
    // Cleanup old entries every minute
    setInterval(() => this.cleanup(), 60000);
  }

  /**
   * Create a unique key for rate limiting
   */
  createKey(userId, operation, ip) {
    const data = `${userId || 'anonymous'}:${operation}:${ip || 'unknown'}`;
    return crypto.createHash('sha256').update(data).digest('hex');
  }

  /**
   * Check if user is blocked
   */
  isBlocked(key) {
    const blockExpiry = this.blocks.get(key);
    if (!blockExpiry) return false;
    
    if (Date.now() > blockExpiry) {
      this.blocks.delete(key);
      return false;
    }
    
    return true;
  }

  /**
   * Block a user for specified duration
   */
  block(key, duration) {
    this.blocks.set(key, Date.now() + duration);
  }

  /**
   * Record a request
   */
  recordRequest(key, window) {
    const now = Date.now();
    const requests = this.requests.get(key) || [];
    
    // Filter out old requests outside the window
    const recentRequests = requests.filter(timestamp => 
      now - timestamp < window
    );
    
    // Add current request
    recentRequests.push(now);
    this.requests.set(key, recentRequests);
    
    return recentRequests.length;
  }

  /**
   * Get request count for a key
   */
  getRequestCount(key, window) {
    const now = Date.now();
    const requests = this.requests.get(key) || [];
    
    return requests.filter(timestamp => 
      now - timestamp < window
    ).length;
  }

  /**
   * Clean up old entries
   */
  cleanup() {
    const now = Date.now();
    
    // Clean expired blocks
    for (const [key, expiry] of this.blocks.entries()) {
      if (now > expiry) {
        this.blocks.delete(key);
      }
    }
    
    // Clean old requests (older than 24 hours)
    const dayAgo = now - 86400000;
    for (const [key, requests] of this.requests.entries()) {
      const recentRequests = requests.filter(timestamp => timestamp > dayAgo);
      
      if (recentRequests.length === 0) {
        this.requests.delete(key);
      } else {
        this.requests.set(key, recentRequests);
      }
    }
  }

  /**
   * Get statistics for monitoring
   */
  getStats() {
    return {
      totalKeys: this.requests.size,
      blockedUsers: this.blocks.size,
      memoryUsage: process.memoryUsage().heapUsed,
    };
  }
}

// Singleton instance
const store = new RateLimitStore();

/**
 * Rate limiter middleware
 */
export function createRateLimiter() {
  return {
    /**
     * Check rate limit for an operation
     * @param {string} operation - Operation name
     * @param {Object} context - Request context with user info
     * @param {string} ip - Client IP address
     * @returns {Object} Rate limit result
     */
    checkLimit: function(operation, context, ip) {
      // Get rate limit config
      const config = RATE_LIMITS[operation] || RATE_LIMITS.global;
      
      // Create keys for user and global limits
      const userId = context?.user?.id;
      const userKey = store.createKey(userId, operation, null);
      const ipKey = store.createKey(null, operation, ip);
      const globalKey = store.createKey(userId, 'global', ip);
      
      // Check if blocked
      if (store.isBlocked(userKey) || store.isBlocked(ipKey) || store.isBlocked(globalKey)) {
        return {
          allowed: false,
          reason: 'Rate limit exceeded - temporarily blocked',
          retryAfter: Math.ceil(config.blockDuration / 1000),
          blocked: true,
        };
      }
      
      // Check request counts
      const userCount = store.getRequestCount(userKey, config.window);
      const ipCount = store.getRequestCount(ipKey, config.window);
      const globalCount = store.getRequestCount(globalKey, RATE_LIMITS.global.window);
      
      // Check limits
      if (userCount >= config.requests) {
        store.block(userKey, config.blockDuration);
        return {
          allowed: false,
          reason: `Too many ${operation} requests - limit is ${config.requests} per ${config.window / 1000} seconds`,
          retryAfter: Math.ceil(config.blockDuration / 1000),
          limit: config.requests,
          remaining: 0,
          resetAt: new Date(Date.now() + config.window).toISOString(),
        };
      }
      
      if (ipCount >= config.requests * 2) { // IP limit is 2x user limit
        store.block(ipKey, config.blockDuration);
        return {
          allowed: false,
          reason: 'Too many requests from this IP address',
          retryAfter: Math.ceil(config.blockDuration / 1000),
          blocked: true,
        };
      }
      
      if (globalCount >= RATE_LIMITS.global.requests) {
        store.block(globalKey, RATE_LIMITS.global.blockDuration);
        return {
          allowed: false,
          reason: 'Too many total requests - please slow down',
          retryAfter: Math.ceil(RATE_LIMITS.global.blockDuration / 1000),
          blocked: true,
        };
      }
      
      // Record the request
      store.recordRequest(userKey, config.window);
      store.recordRequest(ipKey, config.window);
      store.recordRequest(globalKey, RATE_LIMITS.global.window);
      
      // Calculate remaining
      const remaining = config.requests - userCount - 1;
      const resetAt = new Date(Date.now() + config.window);
      
      return {
        allowed: true,
        limit: config.requests,
        remaining: remaining,
        resetAt: resetAt.toISOString(),
        headers: {
          'X-RateLimit-Limit': config.requests.toString(),
          'X-RateLimit-Remaining': remaining.toString(),
          'X-RateLimit-Reset': resetAt.toISOString(),
        },
      };
    },
    
    /**
     * Reset rate limits for a user (admin only)
     */
    reset: function(userId, operation) {
      const userKey = store.createKey(userId, operation || 'global', null);
      store.requests.delete(userKey);
      store.blocks.delete(userKey);
      return { success: true, message: 'Rate limits reset' };
    },
    
    /**
     * Get rate limit stats
     */
    getStats: function() {
      return store.getStats();
    },
    
    /**
     * Configure rate limits (admin only)
     */
    configure: function(operation, config) {
      if (RATE_LIMITS[operation]) {
        Object.assign(RATE_LIMITS[operation], config);
        return { success: true, message: `Rate limits updated for ${operation}` };
      }
      return { success: false, error: 'Unknown operation' };
    },
  };
}

// Export singleton instance
export const rateLimiter = createRateLimiter();

// Export for testing
export { RATE_LIMITS, RateLimitStore };