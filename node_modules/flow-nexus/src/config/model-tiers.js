/**
 * Model Tier Configuration for Seraphina Chat
 * Pricing based on actual API costs with 100% markup on ALL models
 */

export const MODEL_TIERS = {
  // Claude 3 Haiku - Fast and economical
  basic: {
    id: 'basic',
    name: 'Basic (Haiku)',
    model: 'claude-3-haiku-20240307',
    description: 'Fast responses for simple queries',
    maxTokens: 1024,
    temperature: 0.7,
    // Actual cost: ~$0.001 per request
    // Input: $0.25/1M tokens, Output: $1.25/1M tokens
    costPerRequest: 0.001,
    ruvCredits: 2,  // 100% markup (0.001 * 100 = 0.1 cents, rounded up to 2)
    features: [
      'Quick responses',
      'Basic tool execution',
      'Limited context (3 messages)',
      'Great for simple queries'
    ]
  },

  // Claude 3.5 Sonnet - Balanced performance
  standard: {
    id: 'standard',
    name: 'Standard (Sonnet)',
    model: 'claude-3-5-sonnet-20241022',
    description: 'Balanced performance and capabilities',
    maxTokens: 2048,
    temperature: 0.7,
    // Actual cost: ~$0.02 per request
    // Input: $3/1M tokens, Output: $15/1M tokens
    costPerRequest: 0.02,
    ruvCredits: 4,  // 100% markup (0.02 * 100 = 2 cents * 2 = 4 cents)
    features: [
      'Intelligent responses',
      'Full tool execution',
      'Standard context (5 messages)',
      'Recommended for most users'
    ]
  },

  // Claude 3 Opus - Maximum capability
  premium: {
    id: 'premium',
    name: 'Premium (Opus)',
    model: 'claude-3-opus-20240229',
    description: 'Maximum intelligence and capabilities',
    maxTokens: 4096,
    temperature: 0.8,
    // Actual cost: ~$0.10 per request
    // Input: $15/1M tokens, Output: $75/1M tokens
    costPerRequest: 0.10,
    ruvCredits: 20,  // 100% markup (0.10 * 100 = 10 cents * 2 = 20 cents)
    features: [
      'Most intelligent responses',
      'Advanced reasoning',
      'Extended context (10 messages)',
      'Complex multi-step operations',
      'Best for challenging tasks'
    ]
  },

  // Claude 3.5 Sonnet with extended context
  advanced: {
    id: 'advanced',
    name: 'Advanced (Sonnet Extended)',
    model: 'claude-3-5-sonnet-20241022',
    description: 'Extended context with Sonnet performance',
    maxTokens: 4096,
    temperature: 0.8,
    // Higher token usage = higher cost
    costPerRequest: 0.04,
    ruvCredits: 8,  // 100% markup (0.04 * 100 = 4 cents * 2 = 8 cents)
    features: [
      'Extended responses',
      'Full tool execution',
      'Extended context (10 messages)',
      'Long-form content generation'
    ]
  }
};

// Calculate actual token costs for transparency
export const calculateTokenCost = (model, inputTokens, outputTokens) => {
  const pricing = {
    'claude-3-haiku-20240307': {
      input: 0.25 / 1_000_000,   // $0.25 per million
      output: 1.25 / 1_000_000   // $1.25 per million
    },
    'claude-3-5-sonnet-20241022': {
      input: 3 / 1_000_000,       // $3 per million
      output: 15 / 1_000_000      // $15 per million
    },
    'claude-3-opus-20240229': {
      input: 15 / 1_000_000,      // $15 per million
      output: 75 / 1_000_000      // $75 per million
    }
  };

  const modelPricing = pricing[model];
  if (!modelPricing) return 0;

  return (inputTokens * modelPricing.input) + (outputTokens * modelPricing.output);
};

// Get tier by ID or model name
export const getTier = (tierIdOrModel) => {
  // Check if it's a tier ID
  const tier = MODEL_TIERS[tierIdOrModel];
  if (tier) return tier;

  // Check if it's a model name
  return Object.values(MODEL_TIERS).find(t => t.model === tierIdOrModel);
};

// Get default tier from environment or fallback
export const getDefaultTier = () => {
  const defaultModel = process.env.SERAPHINA_DEFAULT_MODEL || 'standard';
  return MODEL_TIERS[defaultModel] || MODEL_TIERS.standard;
};

// Validate if user has enough credits for tier
export const canAffordTier = (balance, tierId) => {
  const tier = MODEL_TIERS[tierId];
  return tier && balance >= tier.ruvCredits;
};

// Get affordable tiers for user
export const getAffordableTiers = (balance) => {
  return Object.values(MODEL_TIERS)
    .filter(tier => balance >= tier.ruvCredits)
    .sort((a, b) => b.ruvCredits - a.ruvCredits);
};

export default MODEL_TIERS;