/**
 * Input Validation Middleware for Payment Operations
 * Comprehensive validation to prevent injection and abuse
 */

// Input validation without external dependencies for security

/**
 * Validation rules for payment operations
 */
const VALIDATION_RULES = {
  // Amount validation
  amount: {
    type: 'number',
    min: 10,
    max: 10000,
    required: true,
    sanitize: (value) => {
      const num = parseFloat(value);
      if (isNaN(num)) throw new Error('Invalid amount');
      // Round to 2 decimal places
      return Math.round(num * 100) / 100;
    },
    validate: (value) => {
      if (value < 10) return 'Minimum amount is $10';
      if (value > 10000) return 'Maximum amount is $10,000';
      if (value !== Math.round(value * 100) / 100) return 'Invalid decimal places';
      return null;
    },
  },
  
  // Email validation
  email: {
    type: 'string',
    required: true,
    maxLength: 254,
    sanitize: (value) => {
      // Remove whitespace and convert to lowercase
      return String(value).trim().toLowerCase();
    },
    validate: (value) => {
      // Basic email validation regex
      const emailRegex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
      if (!emailRegex.test(value)) {
        return 'Invalid email address format';
      }
      // Additional checks for suspicious patterns
      if (value.includes('..') || value.includes('--')) {
        return 'Invalid email address';
      }
      // Check for SQL injection patterns
      const sqlPatterns = /(\b(SELECT|INSERT|UPDATE|DELETE|DROP|UNION|CREATE|ALTER)\b|--|;|\*|'|")/i;
      if (sqlPatterns.test(value)) {
        return 'Invalid characters in email';
      }
      return null;
    },
  },
  
  // Credit balance validation
  target_balance: {
    type: 'number',
    min: 0,
    max: 1000000,
    required: true,
    sanitize: (value) => {
      return Math.max(0, Math.floor(parseFloat(value) || 0));
    },
    validate: (value) => {
      if (value < 0) return 'Balance cannot be negative';
      if (value > 1000000) return 'Balance exceeds maximum';
      return null;
    },
  },
  
  // Boolean validation
  enabled: {
    type: 'boolean',
    required: true,
    sanitize: (value) => {
      if (typeof value === 'boolean') return value;
      if (value === 'true' || value === 1 || value === '1') return true;
      if (value === 'false' || value === 0 || value === '0') return false;
      throw new Error('Invalid boolean value');
    },
    validate: (value) => {
      if (typeof value !== 'boolean') return 'Must be true or false';
      return null;
    },
  },
  
  // Threshold validation
  threshold: {
    type: 'number',
    min: 10,
    max: 1000,
    required: false,
    sanitize: (value) => {
      if (value === undefined || value === null) return undefined;
      return Math.floor(parseFloat(value) || 0);
    },
    validate: (value) => {
      if (value === undefined) return null;
      if (value < 10) return 'Minimum threshold is 10 credits';
      if (value > 1000) return 'Maximum threshold is 1000 credits';
      return null;
    },
  },
  
  // Limit validation for queries
  limit: {
    type: 'number',
    min: 1,
    max: 100,
    default: 10,
    sanitize: (value) => {
      if (value === undefined) return 10;
      const num = parseInt(value, 10);
      if (isNaN(num)) return 10;
      return Math.min(100, Math.max(1, num));
    },
    validate: (value) => {
      if (value < 1 || value > 100) return 'Limit must be between 1 and 100';
      return null;
    },
  },
  
  // Plan validation
  plan: {
    type: 'string',
    required: true,
    enum: ['starter', 'pro', 'enterprise'],
    sanitize: (value) => {
      return String(value).toLowerCase().trim();
    },
    validate: (value) => {
      const validPlans = ['starter', 'pro', 'enterprise'];
      if (!validPlans.includes(value)) {
        return `Invalid plan. Must be one of: ${validPlans.join(', ')}`;
      }
      return null;
    },
  },
  
  // User ID validation
  user_id: {
    type: 'string',
    required: false,
    pattern: /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i,
    sanitize: (value) => {
      if (!value) return undefined;
      return String(value).toLowerCase().trim();
    },
    validate: (value) => {
      if (!value) return null;
      // UUID v4 validation
      const uuidPattern = /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/i;
      if (!uuidPattern.test(value)) {
        return 'Invalid user ID format';
      }
      return null;
    },
  },
};

/**
 * Input validator class
 */
class InputValidator {
  constructor() {
    this.rules = VALIDATION_RULES;
  }
  
  /**
   * Validate input for a payment operation
   */
  validate(operation, params) {
    const errors = [];
    const sanitized = {};
    
    // Get validation rules for this operation
    const operationRules = this.getOperationRules(operation);
    
    // Check for unknown parameters (potential injection attempt)
    const allowedParams = Object.keys(operationRules);
    const unknownParams = Object.keys(params).filter(key => !allowedParams.includes(key));
    
    if (unknownParams.length > 0) {
      errors.push(`Unknown parameters: ${unknownParams.join(', ')}`);
    }
    
    // Validate each parameter
    for (const [param, rule] of Object.entries(operationRules)) {
      const value = params[param];
      
      // Check required
      if (rule.required && (value === undefined || value === null || value === '')) {
        errors.push(`${param} is required`);
        continue;
      }
      
      // Skip optional params if not provided
      if (!rule.required && (value === undefined || value === null)) {
        if (rule.default !== undefined) {
          sanitized[param] = rule.default;
        }
        continue;
      }
      
      try {
        // Sanitize
        let sanitizedValue = value;
        if (rule.sanitize) {
          sanitizedValue = rule.sanitize(value);
        }
        
        // Type check
        if (rule.type && typeof sanitizedValue !== rule.type) {
          errors.push(`${param} must be of type ${rule.type}`);
          continue;
        }
        
        // Pattern check
        if (rule.pattern && !rule.pattern.test(sanitizedValue)) {
          errors.push(`${param} has invalid format`);
          continue;
        }
        
        // Enum check
        if (rule.enum && !rule.enum.includes(sanitizedValue)) {
          errors.push(`${param} must be one of: ${rule.enum.join(', ')}`);
          continue;
        }
        
        // Custom validation
        if (rule.validate) {
          const error = rule.validate(sanitizedValue);
          if (error) {
            errors.push(`${param}: ${error}`);
            continue;
          }
        }
        
        // Additional XSS protection for strings
        if (rule.type === 'string') {
          sanitizedValue = this.sanitizeString(sanitizedValue);
        }
        
        sanitized[param] = sanitizedValue;
      } catch (error) {
        errors.push(`${param}: ${error.message}`);
      }
    }
    
    return {
      valid: errors.length === 0,
      errors: errors,
      sanitized: sanitized,
    };
  }
  
  /**
   * Get validation rules for a specific operation
   */
  getOperationRules(operation) {
    const rules = {
      check_balance: {
        // No required params
      },
      create_payment_link: {
        amount: this.rules.amount,
      },
      configure_auto_refill: {
        enabled: this.rules.enabled,
        threshold: this.rules.threshold,
        amount: { ...this.rules.amount, required: false },
      },
      get_payment_history: {
        limit: this.rules.limit,
      },
      create_subscription: {
        plan: this.rules.plan,
      },
      reduce_credits: {
        email: this.rules.email,
        target_balance: this.rules.target_balance,
      },
    };
    
    return rules[operation] || {};
  }
  
  /**
   * Sanitize string to prevent XSS
   */
  sanitizeString(value) {
    // Remove any HTML tags using regex
    let sanitized = String(value).replace(/<[^>]*>/g, '');
    
    // Remove null bytes
    sanitized = sanitized.replace(/\0/g, '');
    
    // Escape special characters for SQL
    sanitized = sanitized
      .replace(/'/g, "''")
      .replace(/"/g, '""')
      .replace(/\\/g, '\\\\');
    
    // Limit length
    if (sanitized.length > 1000) {
      sanitized = sanitized.substring(0, 1000);
    }
    
    return sanitized;
  }
  
  /**
   * Validate SQL query parameters
   */
  validateSQLParams(params) {
    const sqlInjectionPatterns = [
      /(\b(SELECT|INSERT|UPDATE|DELETE|DROP|UNION|CREATE|ALTER|EXEC|EXECUTE|SCRIPT)\b)/i,
      /(--|#|\/\*|\*\/|xp_|sp_|0x)/i,
      /(\bOR\b.*['"]?\d+['"]?\s*=\s*['"]?\d+)/i,  // Catches 1=1, '1'='1', etc
      /(\bAND\b.*['"]?\d+['"]?\s*=\s*['"]?\d+)/i, // Catches AND conditions
      /(';|";|`)/,
      /(['"]?\s*OR\s+['"]?\d+['"]?\s*=\s*['"]?\d+)/i, // Catches OR 1=1 variations
    ];
    
    for (const [key, value] of Object.entries(params)) {
      if (typeof value === 'string') {
        for (const pattern of sqlInjectionPatterns) {
          if (pattern.test(value)) {
            return {
              valid: false,
              error: `Potential SQL injection detected in ${key}`,
            };
          }
        }
      }
    }
    
    return { valid: true };
  }
  
  /**
   * Validate JSON data
   */
  validateJSON(data) {
    try {
      if (typeof data === 'string') {
        JSON.parse(data);
      }
      return { valid: true };
    } catch (error) {
      return {
        valid: false,
        error: 'Invalid JSON format',
      };
    }
  }
  
  /**
   * Check for path traversal attempts
   */
  checkPathTraversal(value) {
    const patterns = [
      /\.\./,
      /\.\.\\/, 
      /%2e%2e/i,
      /\x00/,
    ];
    
    for (const pattern of patterns) {
      if (pattern.test(value)) {
        return false;
      }
    }
    
    return true;
  }
}

// Export singleton instance
export const inputValidator = new InputValidator();

// Export for testing
export { InputValidator, VALIDATION_RULES };