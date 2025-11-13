#!/usr/bin/env node

import crypto from 'crypto';
import jwt from 'jsonwebtoken';
import { RateLimiterMemory } from 'rate-limiter-flexible';

// Security middleware for MCP server
export class SecurityMiddleware {
  constructor() {
    // Rate limiters for different operations
    this.rateLimiters = {
      global: new RateLimiterMemory({
        points: 100, // Number of requests
        duration: 60, // Per 60 seconds
      }),
      auth: new RateLimiterMemory({
        points: 5,
        duration: 300, // 5 auth attempts per 5 minutes
      }),
      swarm: new RateLimiterMemory({
        points: 10,
        duration: 60, // 10 swarm operations per minute
      }),
      sandbox: new RateLimiterMemory({
        points: 5,
        duration: 60, // 5 sandbox operations per minute
      }),
      neural: new RateLimiterMemory({
        points: 3,
        duration: 60, // 3 neural training sessions per minute
      })
    };

    // API key validation
    this.validApiKeys = new Map();
    
    // Session tracking
    this.activeSessions = new Map();
    
    // Audit log
    this.auditLog = [];
    
    // Security configuration
    this.config = {
      jwtSecret: process.env.JWT_SECRET || crypto.randomBytes(32).toString('hex'),
      apiKeyPrefix: 'fnx_', // Flow Nexus prefix
      sessionTimeout: 24 * 60 * 60 * 1000, // 24 hours
      maxSessionsPerUser: 5,
      enableAuditLog: true,
      enableRateLimiting: true,
      enableInputValidation: true
    };
  }

  // Validate API key
  async validateApiKey(apiKey) {
    if (!apiKey || !apiKey.startsWith(this.config.apiKeyPrefix)) {
      return { valid: false, error: 'Invalid API key format' };
    }

    // Check if key exists and is valid
    const keyData = this.validApiKeys.get(apiKey);
    if (!keyData) {
      return { valid: false, error: 'API key not found' };
    }

    if (keyData.expiresAt && keyData.expiresAt < Date.now()) {
      return { valid: false, error: 'API key expired' };
    }

    if (keyData.revoked) {
      return { valid: false, error: 'API key revoked' };
    }

    return { 
      valid: true, 
      userId: keyData.userId,
      tier: keyData.tier,
      limits: keyData.limits
    };
  }

  // Generate new API key
  generateApiKey(userId, tier = 'free') {
    const key = `${this.config.apiKeyPrefix}${crypto.randomBytes(32).toString('hex')}`;
    const keyData = {
      userId,
      tier,
      createdAt: Date.now(),
      expiresAt: tier === 'free' ? Date.now() + 30 * 24 * 60 * 60 * 1000 : null, // 30 days for free tier
      revoked: false,
      limits: this.getTierLimits(tier)
    };

    this.validApiKeys.set(key, keyData);
    return key;
  }

  // Get tier limits
  getTierLimits(tier) {
    const limits = {
      free: {
        swarms: 3,
        agents: 10,
        tasks: 100,
        sandboxes: 5,
        requestsPerMinute: 20
      },
      pro: {
        swarms: 10,
        agents: 50,
        tasks: 1000,
        sandboxes: 20,
        requestsPerMinute: 100
      },
      enterprise: {
        swarms: -1, // Unlimited
        agents: -1,
        tasks: -1,
        sandboxes: -1,
        requestsPerMinute: 1000
      }
    };

    return limits[tier] || limits.free;
  }

  // Rate limiting
  async checkRateLimit(userId, operation = 'global') {
    if (!this.config.enableRateLimiting) {
      return { allowed: true };
    }

    const limiter = this.rateLimiters[operation] || this.rateLimiters.global;
    
    try {
      await limiter.consume(userId);
      return { allowed: true };
    } catch (rateLimiterRes) {
      return { 
        allowed: false, 
        retryAfter: Math.round(rateLimiterRes.msBeforeNext / 1000) || 60
      };
    }
  }

  // Input validation
  validateInput(input, schema) {
    if (!this.config.enableInputValidation) {
      return { valid: true };
    }

    // Basic validation rules
    const validators = {
      string: (value, rules = {}) => {
        if (typeof value !== 'string') return false;
        if (rules.minLength && value.length < rules.minLength) return false;
        if (rules.maxLength && value.length > rules.maxLength) return false;
        if (rules.pattern && !rules.pattern.test(value)) return false;
        return true;
      },
      number: (value, rules = {}) => {
        if (typeof value !== 'number') return false;
        if (rules.min !== undefined && value < rules.min) return false;
        if (rules.max !== undefined && value > rules.max) return false;
        return true;
      },
      array: (value, rules = {}) => {
        if (!Array.isArray(value)) return false;
        if (rules.minItems && value.length < rules.minItems) return false;
        if (rules.maxItems && value.length > rules.maxItems) return false;
        return true;
      },
      object: (value) => {
        return typeof value === 'object' && value !== null && !Array.isArray(value);
      }
    };

    // Validate against schema
    for (const [field, rules] of Object.entries(schema)) {
      const value = input[field];
      
      if (rules.required && value === undefined) {
        return { valid: false, error: `Missing required field: ${field}` };
      }

      if (value !== undefined) {
        const validator = validators[rules.type];
        if (!validator || !validator(value, rules)) {
          return { valid: false, error: `Invalid value for field: ${field}` };
        }
      }
    }

    return { valid: true };
  }

  // Sanitize input
  sanitizeInput(input) {
    if (typeof input === 'string') {
      // Remove potential SQL injection patterns
      input = input.replace(/['";\\]/g, '');
      // Remove potential XSS patterns
      input = input.replace(/<script[^>]*>.*?<\/script>/gi, '');
      input = input.replace(/<[^>]+>/g, '');
      // Trim whitespace
      input = input.trim();
    } else if (typeof input === 'object' && input !== null) {
      // Recursively sanitize object properties
      for (const key in input) {
        input[key] = this.sanitizeInput(input[key]);
      }
    }
    
    return input;
  }

  // Session management
  createSession(userId, apiKey) {
    const sessionId = crypto.randomBytes(32).toString('hex');
    const session = {
      id: sessionId,
      userId,
      apiKey,
      createdAt: Date.now(),
      lastActivity: Date.now(),
      requestCount: 0
    };

    // Check max sessions per user
    const userSessions = Array.from(this.activeSessions.values())
      .filter(s => s.userId === userId);
    
    if (userSessions.length >= this.config.maxSessionsPerUser) {
      // Remove oldest session
      const oldest = userSessions.sort((a, b) => a.createdAt - b.createdAt)[0];
      this.activeSessions.delete(oldest.id);
    }

    this.activeSessions.set(sessionId, session);
    return sessionId;
  }

  // Validate session
  validateSession(sessionId) {
    const session = this.activeSessions.get(sessionId);
    
    if (!session) {
      return { valid: false, error: 'Invalid session' };
    }

    const now = Date.now();
    if (now - session.lastActivity > this.config.sessionTimeout) {
      this.activeSessions.delete(sessionId);
      return { valid: false, error: 'Session expired' };
    }

    // Update activity
    session.lastActivity = now;
    session.requestCount++;

    return { valid: true, session };
  }

  // Audit logging
  logOperation(operation, userId, details = {}) {
    if (!this.config.enableAuditLog) {
      return;
    }

    const entry = {
      timestamp: new Date().toISOString(),
      operation,
      userId,
      details,
      ip: details.ip || 'unknown',
      userAgent: details.userAgent || 'unknown'
    };

    this.auditLog.push(entry);

    // Keep only last 10000 entries in memory
    if (this.auditLog.length > 10000) {
      this.auditLog = this.auditLog.slice(-5000);
    }
  }

  // Get audit log
  getAuditLog(userId = null, limit = 100) {
    let logs = this.auditLog;
    
    if (userId) {
      logs = logs.filter(entry => entry.userId === userId);
    }

    return logs.slice(-limit);
  }

  // Encrypt sensitive data
  encrypt(text) {
    const algorithm = 'aes-256-gcm';
    const key = Buffer.from(this.config.jwtSecret.slice(0, 32));
    const iv = crypto.randomBytes(16);
    const cipher = crypto.createCipheriv(algorithm, key, iv);
    
    let encrypted = cipher.update(text, 'utf8', 'hex');
    encrypted += cipher.final('hex');
    
    const authTag = cipher.getAuthTag();
    
    return {
      encrypted,
      iv: iv.toString('hex'),
      authTag: authTag.toString('hex')
    };
  }

  // Decrypt sensitive data
  decrypt(encryptedData) {
    const algorithm = 'aes-256-gcm';
    const key = Buffer.from(this.config.jwtSecret.slice(0, 32));
    const decipher = crypto.createDecipheriv(
      algorithm, 
      key, 
      Buffer.from(encryptedData.iv, 'hex')
    );
    
    decipher.setAuthTag(Buffer.from(encryptedData.authTag, 'hex'));
    
    let decrypted = decipher.update(encryptedData.encrypted, 'hex', 'utf8');
    decrypted += decipher.final('utf8');
    
    return decrypted;
  }

  // Generate JWT token
  generateToken(payload) {
    return jwt.sign(payload, this.config.jwtSecret, {
      expiresIn: '24h',
      issuer: 'flow-nexus',
      audience: 'mcp-server'
    });
  }

  // Verify JWT token
  verifyToken(token) {
    try {
      return jwt.verify(token, this.config.jwtSecret, {
        issuer: 'flow-nexus',
        audience: 'mcp-server'
      });
    } catch (error) {
      return null;
    }
  }

  // Initialize with default API keys for testing
  initializeTestKeys() {
    // Development key
    this.validApiKeys.set('fnx_dev_12345', {
      userId: 'dev-user',
      tier: 'pro',
      createdAt: Date.now(),
      expiresAt: null,
      revoked: false,
      limits: this.getTierLimits('pro')
    });

    // Test key for CI/CD
    this.validApiKeys.set('fnx_test_67890', {
      userId: 'test-user',
      tier: 'free',
      createdAt: Date.now(),
      expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000, // 7 days
      revoked: false,
      limits: this.getTierLimits('free')
    });
  }

  // Middleware for MCP server
  async validateRequest(tool, args, context = {}) {
    const response = {
      allowed: true,
      errors: [],
      warnings: []
    };

    try {
      // 1. API Key validation
      if (context.apiKey) {
        const keyValidation = await this.validateApiKey(context.apiKey);
        if (!keyValidation.valid) {
          response.allowed = false;
          response.errors.push(keyValidation.error);
          return response;
        }
        context.userId = keyValidation.userId;
        context.tier = keyValidation.tier;
        context.limits = keyValidation.limits;
      }

      // 2. Session validation
      if (context.sessionId) {
        const sessionValidation = this.validateSession(context.sessionId);
        if (!sessionValidation.valid) {
          response.warnings.push(sessionValidation.error);
        }
      }

      // 3. Rate limiting
      const rateLimitType = this.getOperationType(tool);
      const rateLimit = await this.checkRateLimit(
        context.userId || 'anonymous',
        rateLimitType
      );
      
      if (!rateLimit.allowed) {
        response.allowed = false;
        response.errors.push(`Rate limit exceeded. Retry after ${rateLimit.retryAfter} seconds`);
        return response;
      }

      // 4. Input validation and sanitization
      if (args) {
        const schema = this.getToolSchema(tool);
        if (schema) {
          const validation = this.validateInput(args, schema);
          if (!validation.valid) {
            response.allowed = false;
            response.errors.push(validation.error);
            return response;
          }
        }

        // Sanitize input
        args = this.sanitizeInput(args);
      }

      // 5. Resource limits check
      if (context.limits) {
        const resourceCheck = this.checkResourceLimits(tool, context.limits);
        if (!resourceCheck.allowed) {
          response.allowed = false;
          response.errors.push(resourceCheck.error);
          return response;
        }
      }

      // 6. Audit logging
      this.logOperation(tool, context.userId || 'anonymous', {
        args: args ? Object.keys(args) : [],
        tier: context.tier,
        ip: context.ip,
        userAgent: context.userAgent
      });

      return response;

    } catch (error) {
      response.allowed = false;
      response.errors.push(`Security check failed: ${error.message}`);
      return response;
    }
  }

  // Get operation type for rate limiting
  getOperationType(tool) {
    const operationMap = {
      'swarm_init': 'swarm',
      'agent_spawn': 'swarm',
      'task_orchestrate': 'swarm',
      'sandbox_create': 'sandbox',
      'sandbox_execute': 'sandbox',
      'neural_train': 'neural',
      'auth_init': 'auth',
      'auth_status': 'auth'
    };

    return operationMap[tool] || 'global';
  }

  // Get tool validation schema
  getToolSchema(tool) {
    const schemas = {
      'swarm_init': {
        topology: { type: 'string', required: true },
        maxAgents: { type: 'number', min: 1, max: 100 },
        strategy: { type: 'string' }
      },
      'agent_spawn': {
        type: { type: 'string', required: true },
        capabilities: { type: 'array', maxItems: 10 },
        name: { type: 'string', maxLength: 100 }
      },
      'task_orchestrate': {
        task: { type: 'string', required: true, maxLength: 1000 },
        priority: { type: 'string' },
        strategy: { type: 'string' }
      },
      'sandbox_execute': {
        sandbox_id: { type: 'string', required: true },
        code: { type: 'string', required: true, maxLength: 10000 },
        language: { type: 'string' }
      }
    };

    return schemas[tool];
  }

  // Check resource limits
  checkResourceLimits(tool, limits) {
    // Map tools to resource types
    const resourceMap = {
      'swarm_init': 'swarms',
      'agent_spawn': 'agents',
      'task_orchestrate': 'tasks',
      'sandbox_create': 'sandboxes'
    };

    const resource = resourceMap[tool];
    if (!resource || limits[resource] === -1) {
      return { allowed: true };
    }

    // In production, this would check actual usage from database
    // For now, return allowed
    return { allowed: true };
  }
}

// Export singleton instance
export const security = new SecurityMiddleware();

// Initialize test keys in development
if (process.env.NODE_ENV !== 'production') {
  security.initializeTestKeys();
}