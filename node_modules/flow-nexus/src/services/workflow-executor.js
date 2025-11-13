/**
 * Workflow Execution Engine
 * Bridges workflow definitions with actual sandbox execution
 */

import supabaseClient from './supabase-client.js';

export class WorkflowExecutor {
  constructor() {
    this.activeExecutions = new Map();
  }

  /**
   * Execute a workflow with its steps
   */
  async executeWorkflow(workflowId, inputData = {}, userId = null) {
    try {
      // Get workflow definition
      const { data: workflow, error: workflowError } = await supabaseClient.supabase
        .from('workflows')
        .select('*')
        .eq('id', workflowId)
        .single();

      if (workflowError || !workflow) {
        throw new Error(`Workflow not found: ${workflowId}`);
      }

      // Create execution record
      const executionId = `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
      
      const { data: execution, error: execError } = await supabaseClient.supabase
        .from('workflow_executions')
        .insert({
          id: executionId,
          workflow_id: workflowId,
          status: 'running',
          started_at: new Date().toISOString(),
          input_data: inputData,
          user_id: userId
        })
        .select()
        .single();

      if (execError) {
        throw new Error(`Failed to create execution: ${execError.message}`);
      }

      // Store active execution
      this.activeExecutions.set(executionId, {
        workflow,
        execution,
        currentStep: 0,
        context: { ...inputData },
        sandboxId: null
      });

      // Execute steps
      const result = await this.executeSteps(executionId);

      // Update execution status
      await supabaseClient.supabase
        .from('workflow_executions')
        .update({
          status: result.success ? 'completed' : 'failed',
          completed_at: new Date().toISOString(),
          output_data: result.output,
          error: result.error
        })
        .eq('id', executionId);

      // Clean up
      this.activeExecutions.delete(executionId);

      return {
        success: result.success,
        execution_id: executionId,
        workflow_id: workflowId,
        output: result.output,
        error: result.error,
        steps_executed: result.stepsExecuted
      };

    } catch (error) {
      console.error('Workflow execution error:', error);
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Execute workflow steps sequentially
   */
  async executeSteps(executionId) {
    const execution = this.activeExecutions.get(executionId);
    if (!execution) {
      return { success: false, error: 'Execution not found' };
    }

    const { workflow } = execution;
    const steps = workflow.definition?.steps || [];
    const results = [];
    let lastOutput = execution.context;

    try {
      for (let i = 0; i < steps.length; i++) {
        const step = steps[i];
        execution.currentStep = i;

        // Record step start
        await this.recordStepEvent(executionId, step.id, 'started', { 
          step_index: i,
          step_name: step.name 
        });

        // Execute step based on action type
        const stepResult = await this.executeStep(step, lastOutput, execution);
        
        results.push({
          step_id: step.id,
          name: step.name,
          success: stepResult.success,
          output: stepResult.output,
          error: stepResult.error
        });

        if (!stepResult.success) {
          // Record failure and stop execution
          await this.recordStepEvent(executionId, step.id, 'failed', { 
            error: stepResult.error 
          });
          
          return {
            success: false,
            error: `Step ${step.name} failed: ${stepResult.error}`,
            stepsExecuted: results,
            output: lastOutput
          };
        }

        // Record success
        await this.recordStepEvent(executionId, step.id, 'completed', { 
          output: stepResult.output 
        });

        // Pass output to next step
        lastOutput = { ...lastOutput, ...stepResult.output };
      }

      return {
        success: true,
        stepsExecuted: results,
        output: lastOutput
      };

    } catch (error) {
      return {
        success: false,
        error: error.message,
        stepsExecuted: results,
        output: lastOutput
      };
    }
  }

  /**
   * Execute a single workflow step
   */
  async executeStep(step, context, execution) {
    try {
      const { action, parameters = {} } = step;

      // Resolve parameters with context
      const resolvedParams = this.resolveParameters(parameters, context);

      switch (action) {
        case 'sandbox_execute':
          return await this.executeSandboxCode(resolvedParams, execution);
        
        case 'api_call':
          return await this.executeApiCall(resolvedParams);
        
        case 'database_query':
          return await this.executeDatabaseQuery(resolvedParams);
        
        case 'condition':
          return await this.evaluateCondition(resolvedParams, context);
        
        case 'transform':
          return await this.transformData(resolvedParams, context);
        
        case 'parallel':
          return await this.executeParallelSteps(resolvedParams, context, execution);
        
        case 'wait':
          await new Promise(resolve => setTimeout(resolve, resolvedParams.duration || 1000));
          return { success: true, output: {} };
        
        default:
          // For unknown actions, just pass through
          return { 
            success: true, 
            output: { 
              action, 
              parameters: resolvedParams,
              message: `Simulated ${action}` 
            } 
          };
      }
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Execute code in a sandbox
   */
  async executeSandboxCode(params, execution) {
    try {
      // Create or reuse sandbox
      if (!execution.sandboxId) {
        // Call Edge Function to create sandbox
        const { data: createResult, error: createError } = await supabaseClient.supabase.functions
          .invoke('mcp-tools-e2b', {
            body: {
              action: 'sandbox_create',
              params: {
                template: params.template || 'base',
                name: `workflow-${execution.execution.id}`,
                metadata: {
                  workflow_id: execution.workflow.id,
                  execution_id: execution.execution.id
                }
              }
            }
          });

        if (createError || !createResult?.sandbox_id) {
          throw new Error(`Failed to create sandbox: ${createError?.message || 'Unknown error'}`);
        }

        execution.sandboxId = createResult.sandbox_id;
      }

      // Execute code in sandbox
      const { data: execResult, error: execError } = await supabaseClient.supabase.functions
        .invoke('mcp-tools-e2b', {
          body: {
            action: 'sandbox_execute',
            params: {
              sandbox_id: execution.sandboxId,
              code: params.code,
              language: params.language || 'javascript'
            }
          }
        });

      if (execError) {
        throw new Error(`Sandbox execution failed: ${execError.message}`);
      }

      return {
        success: true,
        output: {
          sandbox_id: execution.sandboxId,
          execution_result: execResult.output,
          exit_code: execResult.exit_code
        }
      };

    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Execute an API call
   */
  async executeApiCall(params) {
    try {
      const { url, method = 'GET', headers = {}, body } = params;
      
      const response = await fetch(url, {
        method,
        headers: {
          'Content-Type': 'application/json',
          ...headers
        },
        body: body ? JSON.stringify(body) : undefined
      });

      const data = await response.json();
      
      return {
        success: response.ok,
        output: {
          status: response.status,
          data
        }
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Execute a database query
   */
  async executeDatabaseQuery(params) {
    try {
      const { table, operation = 'select', filters = {}, data } = params;
      
      let query = supabaseClient.supabase.from(table);
      
      switch (operation) {
        case 'select':
          query = query.select(params.columns || '*');
          break;
        case 'insert':
          query = query.insert(data);
          break;
        case 'update':
          query = query.update(data);
          break;
        case 'delete':
          query = query.delete();
          break;
      }

      // Apply filters
      Object.entries(filters).forEach(([key, value]) => {
        query = query.eq(key, value);
      });

      const { data: result, error } = await query;

      if (error) {
        throw error;
      }

      return {
        success: true,
        output: { result }
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Evaluate a condition
   */
  async evaluateCondition(params, context) {
    try {
      const { condition, then: thenAction, else: elseAction } = params;
      
      // Simple condition evaluation (can be expanded)
      const result = this.evaluateExpression(condition, context);
      
      const actionToExecute = result ? thenAction : elseAction;
      
      if (actionToExecute) {
        return await this.executeStep(actionToExecute, context, {});
      }

      return {
        success: true,
        output: { condition_result: result }
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Transform data
   */
  async transformData(params, context) {
    try {
      const { mapping } = params;
      const output = {};

      for (const [key, value] of Object.entries(mapping)) {
        output[key] = this.resolveValue(value, context);
      }

      return {
        success: true,
        output
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Execute steps in parallel
   */
  async executeParallelSteps(params, context, execution) {
    try {
      const { steps = [] } = params;
      
      const promises = steps.map(step => 
        this.executeStep(step, context, execution)
      );

      const results = await Promise.all(promises);
      
      const allSuccess = results.every(r => r.success);
      const outputs = results.map((r, i) => ({
        step: steps[i].name || `Step ${i + 1}`,
        ...r
      }));

      return {
        success: allSuccess,
        output: { parallel_results: outputs }
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  }

  /**
   * Record workflow step events
   */
  async recordStepEvent(executionId, stepId, event, metadata = {}) {
    try {
      await supabaseClient.supabase
        .from('workflow_audit_log')
        .insert({
          workflow_execution_id: executionId,
          event_type: `step_${event}`,
          event_data: {
            step_id: stepId,
            ...metadata
          },
          created_at: new Date().toISOString()
        });
    } catch (error) {
      console.error('Failed to record step event:', error);
    }
  }

  /**
   * Resolve parameters with context values
   */
  resolveParameters(params, context) {
    const resolved = {};
    
    for (const [key, value] of Object.entries(params)) {
      resolved[key] = this.resolveValue(value, context);
    }
    
    return resolved;
  }

  /**
   * Resolve a single value (supports template variables)
   */
  resolveValue(value, context) {
    if (typeof value === 'string' && value.startsWith('{{') && value.endsWith('}}')) {
      const path = value.slice(2, -2).trim();
      return this.getValueByPath(context, path);
    }
    
    if (typeof value === 'object' && value !== null) {
      if (Array.isArray(value)) {
        return value.map(v => this.resolveValue(v, context));
      }
      
      const resolved = {};
      for (const [k, v] of Object.entries(value)) {
        resolved[k] = this.resolveValue(v, context);
      }
      return resolved;
    }
    
    return value;
  }

  /**
   * Get value from object by path
   */
  getValueByPath(obj, path) {
    return path.split('.').reduce((current, key) => current?.[key], obj);
  }

  /**
   * Evaluate a simple expression
   */
  evaluateExpression(expression, context) {
    // Simple equality check (can be expanded)
    if (typeof expression === 'object' && expression.operator) {
      const { operator, left, right } = expression;
      const leftValue = this.resolveValue(left, context);
      const rightValue = this.resolveValue(right, context);
      
      switch (operator) {
        case '==': return leftValue == rightValue;
        case '!=': return leftValue != rightValue;
        case '>': return leftValue > rightValue;
        case '<': return leftValue < rightValue;
        case '>=': return leftValue >= rightValue;
        case '<=': return leftValue <= rightValue;
        default: return false;
      }
    }
    
    return Boolean(this.resolveValue(expression, context));
  }

  /**
   * Clean up sandbox when workflow completes
   */
  async cleanupSandbox(sandboxId) {
    if (!sandboxId) return;
    
    try {
      await supabaseClient.supabase.functions
        .invoke('mcp-tools-e2b', {
          body: {
            action: 'sandbox_stop',
            params: { sandbox_id: sandboxId }
          }
        });
    } catch (error) {
      console.error('Failed to stop sandbox:', error);
    }
  }
}

// Export singleton instance
export const workflowExecutor = new WorkflowExecutor();