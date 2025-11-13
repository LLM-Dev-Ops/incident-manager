/**
 * Swarm Template Manager
 * Integrates with Supabase app store for swarm templates
 */

import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';

class SwarmTemplateManager {
  constructor(supabaseClient = null) {
    if (supabaseClient) {
      this.supabase = supabaseClient;
    } else {
      this.supabase = createClient(
        process.env.SUPABASE_URL,
        process.env.SUPABASE_ANON_KEY
      );
    }
  }

  /**
   * Get all available swarm templates from app store
   */
  async getStoreTemplates() {
    try {
      const { data, error } = await this.supabase
        .from('app_store_templates')
        .select('*')
        .or('category.eq.swarm,category.eq.ai-agents,category.eq.ai-coordination,template_type.like.%swarm%')
        .eq('is_public', true)
        .order('is_featured', { ascending: false })
        .order('usage_count', { ascending: false });

      if (error) throw error;
      return data || [];
    } catch (err) {
      console.error('Failed to fetch store templates:', err);
      return [];
    }
  }

  /**
   * Get user's saved swarm templates
   */
  async getUserTemplates(userId) {
    try {
      const { data, error } = await this.supabase
        .from('user_agent_templates')
        .select('*')
        .eq('user_id', userId)
        .order('created_at', { ascending: false });

      if (error) throw error;
      return data || [];
    } catch (err) {
      console.error('Failed to fetch user templates:', err);
      return [];
    }
  }

  /**
   * Get featured swarm templates
   */
  async getFeaturedTemplates() {
    try {
      const { data, error } = await this.supabase
        .from('app_store_templates')
        .select('*')
        .eq('is_featured', true)
        .or('category.eq.swarm,category.eq.ai-agents,category.eq.ai-coordination')
        .limit(5);

      if (error) throw error;
      return data || [];
    } catch (err) {
      console.error('Failed to fetch featured templates:', err);
      return [];
    }
  }

  /**
   * Save user template
   */
  async saveUserTemplate(userId, template) {
    try {
      const { data, error } = await this.supabase
        .from('user_agent_templates')
        .insert({
          user_id: userId,
          name: template.name,
          description: template.description,
          category: template.category || 'custom',
          agent_type: template.agent_type || 'swarm',
          sandbox_template: template.sandbox_template || 'node',
          config: template.config,
          metadata: template.metadata
        })
        .select()
        .single();

      if (error) throw error;
      return data;
    } catch (err) {
      console.error('Failed to save template:', err);
      throw err;
    }
  }

  /**
   * Publish template to app store (with rUv pricing)
   */
  async publishTemplate(userId, template) {
    try {
      // Check user's publisher status
      const { data: profile } = await this.supabase
        .from('app_store_profiles')
        .select('*')
        .eq('user_id', userId)
        .single();

      if (!profile?.is_publisher) {
        throw new Error('User must be a publisher to publish templates');
      }

      // Create the app store template
      const { data, error } = await this.supabase
        .from('app_store_templates')
        .insert({
          name: template.name,
          display_name: template.display_name,
          description: template.description,
          category: template.category || 'swarm',
          template_type: 'swarm',
          config: template.config,
          variables: template.variables || {},
          required_variables: template.required_variables || [],
          sandbox_template: template.sandbox_template || 'node',
          author_id: userId,
          tags: template.tags || [],
          version: template.version || '1.0.0',
          is_public: true,
          is_featured: false,
          usage_count: 0
        })
        .select()
        .single();

      if (error) throw error;

      // Set pricing if provided
      if (template.price) {
        await this.supabase
          .from('template_pricing')
          .insert({
            template_name: data.name,
            hourly_rate: template.price,
            description: `rUv credits per use: ${template.price}`
          });
      }

      return data;
    } catch (err) {
      console.error('Failed to publish template:', err);
      throw err;
    }
  }

  /**
   * Purchase/Use a template from the store
   */
  async useTemplate(userId, templateId) {
    try {
      // Get template details
      const { data: template, error: templateError } = await this.supabase
        .from('app_store_templates')
        .select('*')
        .eq('id', templateId)
        .single();

      if (templateError) throw templateError;

      // Check pricing
      const { data: pricing } = await this.supabase
        .from('template_pricing')
        .select('*')
        .eq('template_name', template.name)
        .single();

      if (pricing) {
        // Deduct rUv credits
        const { data: profile } = await this.supabase
          .from('profiles')
          .select('credits_balance')
          .eq('id', userId)
          .single();

        if (profile.credits_balance < pricing.hourly_rate) {
          throw new Error(`Insufficient rUv credits. Need ${pricing.hourly_rate}, have ${profile.credits_balance}`);
        }

        // Deduct credits
        await this.supabase
          .from('profiles')
          .update({ 
            credits_balance: profile.credits_balance - pricing.hourly_rate 
          })
          .eq('id', userId);

        // Record transaction
        await this.supabase
          .from('ruv_transactions')
          .insert({
            user_id: userId,
            amount: -pricing.hourly_rate,
            transaction_type: 'spend_app_purchase',
            description: `Used template: ${template.display_name}`,
            metadata: { template_id: templateId }
          });

        // Pay the author (if not self)
        if (template.author_id && template.author_id !== userId) {
          const authorShare = pricing.hourly_rate * 0.7; // 70% to author
          await this.supabase
            .from('profiles')
            .update({ 
              credits_balance: profile.credits_balance + authorShare 
            })
            .eq('id', template.author_id);

          await this.supabase
            .from('ruv_transactions')
            .insert({
              user_id: template.author_id,
              amount: authorShare,
              transaction_type: 'reward_app_publish',
              description: `Template used: ${template.display_name}`,
              metadata: { template_id: templateId, buyer_id: userId }
            });
        }
      }

      // Update usage count
      await this.supabase
        .from('app_store_templates')
        .update({ 
          usage_count: (template.usage_count || 0) + 1,
          last_used_at: new Date().toISOString()
        })
        .eq('id', templateId);

      // Record deployment
      await this.supabase
        .from('template_deployments')
        .insert({
          template_id: templateId,
          user_id: userId,
          status: 'deployed',
          deployed_at: new Date().toISOString()
        });

      return template;
    } catch (err) {
      console.error('Failed to use template:', err);
      throw err;
    }
  }

  /**
   * Get template marketplace data
   */
  async getMarketplaceData() {
    try {
      // Get top templates by usage
      const { data: topTemplates } = await this.supabase
        .from('app_store_templates')
        .select('*, template_pricing(hourly_rate)')
        .or('category.eq.swarm,category.eq.ai-agents')
        .order('usage_count', { ascending: false })
        .limit(10);

      // Get newest templates
      const { data: newTemplates } = await this.supabase
        .from('app_store_templates')
        .select('*, template_pricing(hourly_rate)')
        .or('category.eq.swarm,category.eq.ai-agents')
        .order('created_at', { ascending: false })
        .limit(10);

      // Get featured templates
      const { data: featuredTemplates } = await this.supabase
        .from('app_store_templates')
        .select('*, template_pricing(hourly_rate)')
        .eq('is_featured', true)
        .or('category.eq.swarm,category.eq.ai-agents')
        .limit(5);

      return {
        top: topTemplates || [],
        new: newTemplates || [],
        featured: featuredTemplates || []
      };
    } catch (err) {
      console.error('Failed to fetch marketplace data:', err);
      return { top: [], new: [], featured: [] };
    }
  }

  /**
   * Format template for interactive display
   */
  formatTemplateForDisplay(template, pricing = null) {
    const priceStr = pricing ? chalk.yellow(`${pricing.hourly_rate} rUv`) : chalk.green('FREE');
    const usageStr = template.usage_count > 0 ? chalk.gray(`(${template.usage_count} uses)`) : '';
    const featuredBadge = template.is_featured ? chalk.cyan('★ ') : '';
    
    return {
      name: `${featuredBadge}${template.display_name} - ${priceStr} ${usageStr}`,
      value: template,
      short: template.display_name
    };
  }

  /**
   * Get interactive template choices for CLI
   */
  async getInteractiveChoices(userId) {
    const choices = [];

    // Get marketplace data
    const marketplace = await this.getMarketplaceData();

    // Featured templates
    if (marketplace.featured.length > 0) {
      choices.push(chalk.cyan('──── ★ Featured Templates ────'));
      marketplace.featured.forEach(t => {
        choices.push(this.formatTemplateForDisplay(t, t.template_pricing));
      });
    }

    // User's saved templates
    const userTemplates = await this.getUserTemplates(userId);
    if (userTemplates.length > 0) {
      choices.push(chalk.green('──── 💾 Your Saved Templates ────'));
      userTemplates.forEach(t => {
        choices.push({
          name: `${t.name} - ${chalk.gray(t.description || 'No description')}`,
          value: { ...t, isUserTemplate: true },
          short: t.name
        });
      });
    }

    // Top templates
    if (marketplace.top.length > 0) {
      choices.push(chalk.yellow('──── 🔥 Popular Templates ────'));
      marketplace.top.slice(0, 5).forEach(t => {
        choices.push(this.formatTemplateForDisplay(t, t.template_pricing));
      });
    }

    // New templates
    if (marketplace.new.length > 0) {
      choices.push(chalk.magenta('──── ✨ New Templates ────'));
      marketplace.new.slice(0, 5).forEach(t => {
        choices.push(this.formatTemplateForDisplay(t, t.template_pricing));
      });
    }

    // Options
    choices.push(chalk.gray('──── Options ────'));
    choices.push({
      name: chalk.cyan('🔧 Custom Configuration'),
      value: 'custom',
      short: 'Custom'
    });
    choices.push({
      name: chalk.green('🔍 Browse Marketplace'),
      value: 'browse',
      short: 'Browse'
    });
    choices.push({
      name: chalk.yellow('💡 Quick Start (Minimal)'),
      value: 'quickstart',
      short: 'Quick Start'
    });

    return choices;
  }
}

export default SwarmTemplateManager;