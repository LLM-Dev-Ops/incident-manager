/**
 * Neural Training Service
 * Integrates e2b sandboxes, ruv-fann WASM, and DIY configuration
 * Uses existing credit system and database pricing
 */

import { E2BService } from './e2b-service.js';
import DatabasePricingService from './database-pricing.js';
import CreditManager from './credit-manager.js';
import { createHash } from 'crypto';
import fs from 'fs/promises';
import path from 'path';

export class NeuralTrainingService {
  constructor(supabaseClient) {
    this.supabase = supabaseClient;
    this.e2bService = new E2BService();
    this.pricingService = new DatabasePricingService(supabaseClient);
    this.creditManager = new CreditManager(supabaseClient);
    this.activeSandboxes = new Map();
    this.trainingJobs = new Map();
  }

  /**
   * Create a DIY neural training configuration
   */
  async createDIYConfig(userId, config) {
    const configId = `diy_${createHash('sha256')
      .update(`${userId}_${Date.now()}`)
      .digest('hex')
      .slice(0, 12)}`;

    const diyConfig = {
      id: configId,
      userId,
      name: config.name || 'DIY Neural Network',
      architecture: {
        type: config.architecture?.type || 'feedforward',
        layers: config.architecture?.layers || [
          { type: 'input', neurons: 10 },
          { type: 'dense', neurons: 20, activation: 'relu' },
          { type: 'output', neurons: 2, activation: 'softmax' }
        ],
        connections: config.architecture?.connections || 'sequential'
      },
      training: {
        algorithm: config.training?.algorithm || 'backprop',
        learningRate: config.training?.learningRate || 0.001,
        epochs: config.training?.epochs || 100,
        batchSize: config.training?.batchSize || 32,
        earlyStopping: config.training?.earlyStopping || {
          patience: 10,
          minDelta: 0.001
        }
      },
      divergent: {
        enabled: config.divergent?.enabled || false,
        type: config.divergent?.type || 'creative',
        factor: config.divergent?.factor || 0.3,
        patterns: config.divergent?.patterns || []
      },
      resources: {
        tier: config.resources?.tier || 'nano', // nano, micro, mini, standard, extended, unlimited
        maxTime: config.resources?.maxTime || 60,
        maxMemory: config.resources?.maxMemory || '128MB'
      },
      createdAt: new Date().toISOString()
    };

    // Store configuration in database
    const { data, error } = await this.supabase
      .from('neural_configs')
      .insert({
        id: configId,
        user_id: userId,
        config_data: diyConfig,
        status: 'draft'
      })
      .select()
      .single();

    if (error) throw new Error(`Failed to save config: ${error.message}`);
    return data;
  }

  /**
   * Deploy neural training to e2b sandbox
   */
  async deployToSandbox(userId, configId) {
    // Get configuration
    const { data: config, error: configError } = await this.supabase
      .from('neural_configs')
      .select('*')
      .eq('id', configId)
      .eq('user_id', userId)
      .single();

    if (configError || !config) {
      throw new Error('Configuration not found');
    }

    const configData = config.config_data;
    
    // Determine tool name based on tier
    const toolName = `diy_neural_${configData.resources.tier}`;
    
    // Check if user can afford
    const canAfford = await this.pricingService.canAffordTool(userId, toolName);
    if (!canAfford.canAfford) {
      return {
        success: false,
        error: `Insufficient credits. Need ${canAfford.cost} credits, have ${canAfford.balance}`
      };
    }

    // Create sandbox
    const sandboxName = `neural_${configId.slice(0, 8)}`;
    const sandbox = await this.e2bService.createSandbox('neural-runtime', sandboxName);
    
    // Generate training code
    const trainingCode = this.generateTrainingCode(configData);
    
    // Deploy code to sandbox
    const deployment = {
      sandboxId: sandbox.id,
      configId,
      userId,
      code: trainingCode,
      status: 'deploying',
      startTime: Date.now()
    };
    
    this.activeSandboxes.set(sandbox.id, deployment);
    
    // Execute training with credit deduction
    const result = await this.executeTraining(userId, sandbox.id, toolName, configData);
    
    return result;
  }

  /**
   * Execute neural training with credit deduction
   */
  async executeTraining(userId, sandboxId, toolName, config) {
    const trainingId = `train_${Date.now()}`;
    
    try {
      // Start training job
      const job = {
        id: trainingId,
        sandboxId,
        userId,
        config,
        status: 'running',
        startTime: Date.now(),
        metrics: {
          epoch: 0,
          loss: null,
          accuracy: null
        }
      };
      
      this.trainingJobs.set(trainingId, job);

      // Deduct credits using existing system
      const creditResult = await this.creditManager.deductCreditsForTool(
        userId,
        toolName,
        {
          config_id: config.id,
          sandbox_id: sandboxId,
          training_id: trainingId
        },
        null,
        {
          architecture: config.architecture.type,
          layers: config.architecture.layers.length,
          epochs: config.training.epochs,
          divergent: config.divergent.enabled
        }
      );

      if (!creditResult.success) {
        job.status = 'failed';
        job.error = creditResult.error;
        return creditResult;
      }

      // Simulate training execution (in production, this would run actual WASM)
      const trainingResult = await this.simulateTraining(job);
      
      // Update job status
      job.status = 'completed';
      job.endTime = Date.now();
      job.duration = job.endTime - job.startTime;
      job.result = trainingResult;
      
      // Store training result
      const { error: storeError } = await this.supabase
        .from('neural_training_results')
        .insert({
          id: trainingId,
          user_id: userId,
          config_id: config.id,
          sandbox_id: sandboxId,
          metrics: trainingResult.metrics,
          model_url: trainingResult.modelUrl,
          credits_used: creditResult.cost,
          duration_ms: job.duration,
          created_at: new Date().toISOString()
        });

      if (storeError) {
        console.error('Failed to store training result:', storeError);
      }

      return {
        success: true,
        trainingId,
        sandboxId,
        metrics: trainingResult.metrics,
        modelUrl: trainingResult.modelUrl,
        creditsUsed: creditResult.cost,
        balance: creditResult.balance_after,
        duration: job.duration
      };

    } catch (error) {
      console.error('Training execution failed:', error);
      return {
        success: false,
        error: error.message
      };
    } finally {
      // Clean up sandbox
      await this.e2bService.stopSandbox(sandboxId);
      this.activeSandboxes.delete(sandboxId);
    }
  }

  /**
   * Generate training code for deployment
   */
  generateTrainingCode(config) {
    return `
// Auto-generated Neural Training Code
// Configuration: ${config.name}

import { NeuralNetwork } from '@ruv-fann/wasm';

// Initialize network
const network = new NeuralNetwork({
  architecture: ${JSON.stringify(config.architecture, null, 2)},
  training: ${JSON.stringify(config.training, null, 2)},
  divergent: ${JSON.stringify(config.divergent, null, 2)}
});

// Training function
async function train(data) {
  console.log('Starting training with config:', network.config);
  
  const history = [];
  for (let epoch = 0; epoch < ${config.training.epochs}; epoch++) {
    const metrics = await network.trainEpoch(data);
    history.push(metrics);
    
    // Report progress
    console.log(\`Epoch \${epoch + 1}/\${${config.training.epochs}}: Loss=\${metrics.loss.toFixed(4)}, Accuracy=\${metrics.accuracy.toFixed(4)}\`);
    
    // Early stopping check
    if (${config.training.earlyStopping.patience} > 0) {
      if (checkEarlyStopping(history, ${config.training.earlyStopping.patience}, ${config.training.earlyStopping.minDelta})) {
        console.log('Early stopping triggered');
        break;
      }
    }
  }
  
  return {
    model: network.serialize(),
    history,
    finalMetrics: history[history.length - 1]
  };
}

// Helper function for early stopping
function checkEarlyStopping(history, patience, minDelta) {
  if (history.length < patience + 1) return false;
  
  const recent = history.slice(-patience);
  const best = Math.min(...recent.map(h => h.loss));
  const current = recent[recent.length - 1].loss;
  
  return (current - best) < minDelta;
}

// Export for execution
export { train, network };
`;
  }

  /**
   * Simulate training (placeholder for actual WASM execution)
   */
  async simulateTraining(job) {
    // Simulate training progress
    const epochs = job.config.training.epochs;
    const metrics = {
      loss: [],
      accuracy: [],
      val_loss: [],
      val_accuracy: []
    };

    for (let i = 0; i < Math.min(epochs, 10); i++) {
      // Simulate decreasing loss and increasing accuracy
      metrics.loss.push(1.0 - (i * 0.08) + Math.random() * 0.1);
      metrics.accuracy.push(0.5 + (i * 0.04) + Math.random() * 0.05);
      metrics.val_loss.push(1.1 - (i * 0.07) + Math.random() * 0.15);
      metrics.val_accuracy.push(0.45 + (i * 0.035) + Math.random() * 0.08);
      
      // Update job metrics
      job.metrics.epoch = i + 1;
      job.metrics.loss = metrics.loss[i];
      job.metrics.accuracy = metrics.accuracy[i];
      
      // Small delay to simulate processing
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    // Generate mock model URL
    const modelId = `model_${job.id}`;
    const modelUrl = `/models/${modelId}.wasm`;

    return {
      metrics: {
        final_loss: metrics.loss[metrics.loss.length - 1],
        final_accuracy: metrics.accuracy[metrics.accuracy.length - 1],
        final_val_loss: metrics.val_loss[metrics.val_loss.length - 1],
        final_val_accuracy: metrics.val_accuracy[metrics.val_accuracy.length - 1],
        epochs_completed: metrics.loss.length,
        history: metrics
      },
      modelUrl,
      modelId
    };
  }

  /**
   * Get training status
   */
  getTrainingStatus(trainingId) {
    const job = this.trainingJobs.get(trainingId);
    if (!job) return null;
    
    return {
      id: job.id,
      status: job.status,
      progress: job.metrics.epoch / job.config.training.epochs,
      metrics: job.metrics,
      duration: job.endTime ? job.endTime - job.startTime : Date.now() - job.startTime
    };
  }

  /**
   * List user's training jobs
   */
  async listUserTrainings(userId, limit = 10) {
    const { data, error } = await this.supabase
      .from('neural_training_results')
      .select('*')
      .eq('user_id', userId)
      .order('created_at', { ascending: false })
      .limit(limit);

    if (error) {
      console.error('Failed to list trainings:', error);
      return [];
    }

    return data;
  }

  /**
   * Create neural template for app store
   */
  async createTemplate(userId, templateData) {
    const templateId = `tpl_${createHash('sha256')
      .update(`${userId}_${templateData.name}_${Date.now()}`)
      .digest('hex')
      .slice(0, 12)}`;

    const template = {
      id: templateId,
      name: templateData.name,
      description: templateData.description,
      category: templateData.category || 'neural',
      tier: templateData.tier || 'free',
      author_id: userId,
      config_data: templateData.config,
      performance_metrics: templateData.metrics || {},
      price_credits: templateData.price || 0,
      is_public: templateData.public !== false,
      created_at: new Date().toISOString()
    };

    const { data, error } = await this.supabase
      .from('neural_templates')
      .insert(template)
      .select()
      .single();

    if (error) throw new Error(`Failed to create template: ${error.message}`);
    return data;
  }

  /**
   * Deploy template from app store
   */
  async deployTemplate(userId, templateId) {
    // Get template
    const { data: template, error } = await this.supabase
      .from('neural_templates')
      .select('*')
      .eq('id', templateId)
      .single();

    if (error || !template) {
      throw new Error('Template not found');
    }

    // Check if template is free or user can afford
    if (template.price_credits > 0) {
      const canAfford = await this.pricingService.canAffordTool(userId, 'template_purchase');
      if (!canAfford.canAfford) {
        return {
          success: false,
          error: `Insufficient credits for template. Need ${template.price_credits} credits`
        };
      }
    }

    // Create configuration from template
    const config = await this.createDIYConfig(userId, {
      name: `${template.name} (from template)`,
      ...template.config_data
    });

    // Deploy to sandbox
    return await this.deployToSandbox(userId, config.id);
  }
}

export default NeuralTrainingService;