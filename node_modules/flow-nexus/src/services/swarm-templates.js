/**
 * Swarm Templates Service
 * Provides pre-configured swarm templates and configurations
 */

import chalk from 'chalk';

class SwarmTemplates {
  constructor() {
    this.templates = {
      // Quick Start Templates
      quickstart: {
        minimal: {
          name: '🚀 Minimal Swarm',
          description: 'Lightweight swarm for simple tasks',
          topology: 'star',
          maxAgents: 2,
          strategy: 'balanced',
          agentTypes: ['coordinator', 'worker'],
          templates: ['node', 'python'],
          cost: 7, // 3 base + 2*2 agents
          icon: '⚡',
          recommended: true
        },
        standard: {
          name: '📦 Standard Swarm',
          description: 'Balanced swarm for most applications',
          topology: 'mesh',
          maxAgents: 5,
          strategy: 'adaptive',
          agentTypes: ['coordinator', 'worker', 'analyzer', 'optimizer', 'monitor'],
          templates: ['node', 'python', 'react', 'nextjs', 'vanilla'],
          cost: 13, // 3 base + 5*2 agents
          icon: '🎯',
          recommended: true
        },
        advanced: {
          name: '🔥 Advanced Swarm',
          description: 'High-performance swarm for complex tasks',
          topology: 'hierarchical',
          maxAgents: 8,
          strategy: 'specialized',
          agentTypes: ['coordinator', 'worker', 'worker', 'analyzer', 'optimizer', 'monitor', 'documenter', 'tester'],
          templates: ['node', 'python', 'react', 'nextjs', 'vanilla', 'node', 'python', 'node'],
          cost: 19, // 3 base + 8*2 agents
          icon: '🚀'
        }
      },
      
      // Specialized Templates
      specialized: {
        webdev: {
          name: '🌐 Web Development Swarm',
          description: 'Optimized for web application development',
          topology: 'mesh',
          maxAgents: 6,
          strategy: 'specialized',
          agentTypes: ['frontend-dev', 'backend-dev', 'api-designer', 'ui-designer', 'tester', 'deployer'],
          templates: ['react', 'node', 'node', 'vanilla', 'python', 'node'],
          cost: 15,
          icon: '🌐',
          tags: ['web', 'fullstack', 'frontend', 'backend']
        },
        ml: {
          name: '🧠 Machine Learning Swarm',
          description: 'Specialized for ML/AI workflows',
          topology: 'hierarchical',
          maxAgents: 7,
          strategy: 'specialized',
          agentTypes: ['ml-engineer', 'data-processor', 'trainer', 'evaluator', 'optimizer', 'deployer', 'monitor'],
          templates: ['python', 'python', 'python', 'python', 'python', 'node', 'python'],
          cost: 17,
          icon: '🧠',
          tags: ['ml', 'ai', 'data-science', 'deep-learning']
        },
        api: {
          name: '🔌 API Development Swarm',
          description: 'Build and test REST/GraphQL APIs',
          topology: 'star',
          maxAgents: 5,
          strategy: 'specialized',
          agentTypes: ['api-designer', 'backend-dev', 'tester', 'documenter', 'security'],
          templates: ['node', 'node', 'python', 'node', 'python'],
          cost: 13,
          icon: '🔌',
          tags: ['api', 'rest', 'graphql', 'backend']
        },
        research: {
          name: '🔬 Research & Analysis Swarm',
          description: 'Data gathering and analysis tasks',
          topology: 'mesh',
          maxAgents: 4,
          strategy: 'adaptive',
          agentTypes: ['researcher', 'analyst', 'documenter', 'validator'],
          templates: ['python', 'python', 'node', 'python'],
          cost: 11,
          icon: '🔬',
          tags: ['research', 'analysis', 'data']
        },
        testing: {
          name: '🧪 Testing & QA Swarm',
          description: 'Comprehensive testing and quality assurance',
          topology: 'ring',
          maxAgents: 5,
          strategy: 'balanced',
          agentTypes: ['test-designer', 'unit-tester', 'integration-tester', 'e2e-tester', 'reporter'],
          templates: ['node', 'python', 'node', 'python', 'node'],
          cost: 13,
          icon: '🧪',
          tags: ['testing', 'qa', 'quality']
        }
      },
      
      // Enterprise Templates
      enterprise: {
        microservices: {
          name: '🏢 Microservices Orchestrator',
          description: 'Manage microservices architecture',
          topology: 'hierarchical',
          maxAgents: 10,
          strategy: 'specialized',
          agentTypes: ['orchestrator', 'service', 'service', 'service', 'gateway', 'monitor', 'logger', 'balancer', 'cache', 'queue'],
          templates: ['node', 'node', 'python', 'node', 'node', 'python', 'node', 'node', 'node', 'python'],
          cost: 23,
          icon: '🏢',
          tags: ['enterprise', 'microservices', 'distributed']
        },
        devops: {
          name: '⚙️ DevOps Pipeline',
          description: 'CI/CD and infrastructure automation',
          topology: 'mesh',
          maxAgents: 8,
          strategy: 'adaptive',
          agentTypes: ['ci-runner', 'cd-deployer', 'infra-manager', 'monitor', 'security-scanner', 'backup', 'alerter', 'reporter'],
          templates: ['node', 'node', 'python', 'python', 'python', 'node', 'node', 'node'],
          cost: 19,
          icon: '⚙️',
          tags: ['devops', 'cicd', 'automation', 'infrastructure']
        }
      },
      
      // Custom Templates (User-defined)
      custom: {}
    };
  }

  /**
   * Get all available templates
   */
  getAllTemplates() {
    const all = [];
    
    Object.entries(this.templates).forEach(([category, templates]) => {
      Object.entries(templates).forEach(([key, template]) => {
        all.push({
          ...template,
          category,
          key,
          id: `${category}-${key}`
        });
      });
    });
    
    return all;
  }

  /**
   * Get templates by category
   */
  getTemplatesByCategory(category) {
    return this.templates[category] || {};
  }

  /**
   * Get a specific template
   */
  getTemplate(category, key) {
    return this.templates[category]?.[key];
  }

  /**
   * Get recommended templates
   */
  getRecommendedTemplates() {
    return this.getAllTemplates().filter(t => t.recommended);
  }

  /**
   * Search templates by tags
   */
  searchByTags(tags) {
    const searchTags = Array.isArray(tags) ? tags : [tags];
    return this.getAllTemplates().filter(template => {
      if (!template.tags) return false;
      return searchTags.some(tag => template.tags.includes(tag.toLowerCase()));
    });
  }

  /**
   * Add custom template
   */
  addCustomTemplate(key, template) {
    this.templates.custom[key] = {
      ...template,
      custom: true,
      createdAt: new Date().toISOString()
    };
    return this.templates.custom[key];
  }

  /**
   * Format template for display
   */
  formatTemplateChoice(template) {
    const costColor = template.cost <= 10 ? chalk.green : 
                     template.cost <= 15 ? chalk.yellow : 
                     chalk.red;
    
    return {
      name: `${template.icon || '📦'} ${template.name} - ${costColor(template.cost + ' rUv')}`,
      value: template,
      short: template.name
    };
  }

  /**
   * Get interactive choices for inquirer
   */
  getInteractiveChoices() {
    const choices = [];
    
    // Recommended templates first
    const recommended = this.getRecommendedTemplates();
    if (recommended.length > 0) {
      choices.push(new chalk.gray('──── Recommended ────'));
      recommended.forEach(t => choices.push(this.formatTemplateChoice(t)));
    }
    
    // Quick Start
    choices.push(new chalk.gray('──── Quick Start ────'));
    Object.values(this.templates.quickstart).forEach(t => {
      if (!t.recommended) {
        choices.push(this.formatTemplateChoice(t));
      }
    });
    
    // Specialized
    choices.push(new chalk.gray('──── Specialized ────'));
    Object.values(this.templates.specialized).forEach(t => {
      choices.push(this.formatTemplateChoice(t));
    });
    
    // Enterprise
    choices.push(new chalk.gray('──── Enterprise ────'));
    Object.values(this.templates.enterprise).forEach(t => {
      choices.push(this.formatTemplateChoice(t));
    });
    
    // Custom templates if any
    if (Object.keys(this.templates.custom).length > 0) {
      choices.push(new chalk.gray('──── Custom ────'));
      Object.values(this.templates.custom).forEach(t => {
        choices.push(this.formatTemplateChoice(t));
      });
    }
    
    // Custom configuration option
    choices.push(new chalk.gray('────────────────'));
    choices.push({
      name: chalk.cyan('⚙️  Custom Configuration'),
      value: 'custom',
      short: 'Custom'
    });
    
    return choices;
  }

  /**
   * Generate swarm config from template
   */
  generateConfig(template, overrides = {}) {
    return {
      topology: template.topology,
      maxAgents: template.maxAgents,
      strategy: template.strategy,
      agents: template.agentTypes.map((type, i) => ({
        type,
        template: template.templates[i],
        capabilities: this.getAgentCapabilities(type)
      })),
      metadata: {
        template: template.name,
        templateId: template.id,
        icon: template.icon,
        tags: template.tags || []
      },
      ...overrides
    };
  }

  /**
   * Get agent capabilities based on type
   */
  getAgentCapabilities(agentType) {
    const capabilities = {
      'coordinator': ['orchestration', 'delegation', 'monitoring'],
      'worker': ['execution', 'processing', 'reporting'],
      'analyzer': ['analysis', 'metrics', 'insights'],
      'optimizer': ['optimization', 'performance', 'efficiency'],
      'monitor': ['monitoring', 'alerting', 'logging'],
      'documenter': ['documentation', 'reporting', 'summarization'],
      'tester': ['testing', 'validation', 'quality'],
      'frontend-dev': ['ui', 'react', 'css', 'javascript'],
      'backend-dev': ['api', 'database', 'server', 'node'],
      'ml-engineer': ['machine-learning', 'tensorflow', 'pytorch'],
      'researcher': ['research', 'data-gathering', 'analysis'],
      'security': ['security', 'vulnerability', 'audit']
    };
    
    return capabilities[agentType] || ['general'];
  }
}

export default SwarmTemplates;