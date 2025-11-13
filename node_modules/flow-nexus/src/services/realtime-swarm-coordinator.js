/**
 * Real-time Swarm Coordinator for Flow Nexus
 * Enables real-time communication between swarms and Flow Nexus platform
 * Uses WebSockets and Server-Sent Events for bi-directional communication
 */

import { createClient } from '@supabase/supabase-js';
import WebSocket from 'ws';
import EventEmitter from 'events';

const SUPABASE_URL = process.env.VITE_SUPABASE_URL;
const SUPABASE_ANON_KEY = process.env.VITE_SUPABASE_ANON_KEY;

class RealtimeSwarmCoordinator extends EventEmitter {
  constructor() {
    super();
    this.supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY);
    this.swarms = new Map(); // Active swarm connections
    this.channels = new Map(); // Supabase realtime channels
    this.wsServer = null;
    this.heartbeatInterval = null;
  }

  /**
   * Initialize real-time coordinator
   */
  async initialize(port = 8080) {
    console.log('🚀 Initializing Real-time Swarm Coordinator...');
    
    // Setup WebSocket server for direct swarm communication
    this.setupWebSocketServer(port);
    
    // Setup Supabase realtime subscriptions
    await this.setupRealtimeSubscriptions();
    
    // Start heartbeat monitoring
    this.startHeartbeat();
    
    console.log('✅ Real-time Swarm Coordinator initialized');
  }

  /**
   * Setup WebSocket server for swarm connections
   */
  setupWebSocketServer(port) {
    this.wsServer = new WebSocket.Server({ port });
    
    this.wsServer.on('connection', (ws, req) => {
      const swarmId = this.extractSwarmId(req.url);
      console.log(`🔗 New swarm connection: ${swarmId}`);
      
      // Register swarm connection
      this.registerSwarm(swarmId, ws);
      
      // Handle swarm messages
      ws.on('message', (message) => {
        this.handleSwarmMessage(swarmId, message);
      });
      
      // Handle disconnection
      ws.on('close', () => {
        console.log(`🔌 Swarm disconnected: ${swarmId}`);
        this.unregisterSwarm(swarmId);
      });
      
      // Send welcome message
      this.sendToSwarm(swarmId, {
        type: 'connection_established',
        swarmId,
        timestamp: new Date().toISOString(),
        capabilities: this.getCapabilities()
      });
    });
    
    console.log(`📡 WebSocket server listening on port ${port}`);
  }

  /**
   * Setup Supabase realtime subscriptions
   */
  async setupRealtimeSubscriptions() {
    // Subscribe to swarm events
    const swarmChannel = this.supabase
      .channel('swarm-events')
      .on('postgres_changes', {
        event: '*',
        schema: 'public',
        table: 'swarm_events'
      }, (payload) => {
        this.handleDatabaseEvent('swarm_events', payload);
      })
      .on('postgres_changes', {
        event: '*',
        schema: 'public',
        table: 'swarm_tasks'
      }, (payload) => {
        this.handleDatabaseEvent('swarm_tasks', payload);
      })
      .on('postgres_changes', {
        event: '*',
        schema: 'public',
        table: 'agent_status'
      }, (payload) => {
        this.handleDatabaseEvent('agent_status', payload);
      })
      .subscribe();
    
    this.channels.set('swarm-events', swarmChannel);
    
    // Subscribe to challenge events for judge coordination
    const challengeChannel = this.supabase
      .channel('challenge-events')
      .on('postgres_changes', {
        event: 'UPDATE',
        schema: 'public',
        table: 'challenges',
        filter: 'status=eq.completed'
      }, (payload) => {
        this.handleChallengeCompletion(payload);
      })
      .subscribe();
    
    this.channels.set('challenge-events', challengeChannel);
    
    console.log('📻 Realtime subscriptions established');
  }

  /**
   * Register a new swarm connection
   */
  registerSwarm(swarmId, ws) {
    this.swarms.set(swarmId, {
      id: swarmId,
      ws,
      status: 'active',
      connectedAt: new Date(),
      lastHeartbeat: new Date(),
      agents: [],
      tasks: []
    });
    
    // Notify other swarms of new connection
    this.broadcastToSwarms({
      type: 'swarm_joined',
      swarmId,
      timestamp: new Date().toISOString()
    }, swarmId);
    
    // Update database
    this.updateSwarmStatus(swarmId, 'active');
  }

  /**
   * Unregister a swarm connection
   */
  unregisterSwarm(swarmId) {
    const swarm = this.swarms.get(swarmId);
    if (swarm) {
      this.swarms.delete(swarmId);
      
      // Notify other swarms
      this.broadcastToSwarms({
        type: 'swarm_left',
        swarmId,
        timestamp: new Date().toISOString()
      });
      
      // Update database
      this.updateSwarmStatus(swarmId, 'disconnected');
    }
  }

  /**
   * Handle incoming message from swarm
   */
  async handleSwarmMessage(swarmId, message) {
    try {
      const data = JSON.parse(message);
      console.log(`📨 Message from swarm ${swarmId}:`, data.type);
      
      switch (data.type) {
        case 'heartbeat':
          this.handleHeartbeat(swarmId);
          break;
          
        case 'agent_spawned':
          await this.handleAgentSpawned(swarmId, data);
          break;
          
        case 'task_orchestrated':
          await this.handleTaskOrchestrated(swarmId, data);
          break;
          
        case 'task_completed':
          await this.handleTaskCompleted(swarmId, data);
          break;
          
        case 'performance_metrics':
          await this.handlePerformanceMetrics(swarmId, data);
          break;
          
        case 'coordination_request':
          await this.handleCoordinationRequest(swarmId, data);
          break;
          
        case 'memory_sync':
          await this.handleMemorySync(swarmId, data);
          break;
          
        case 'broadcast':
          this.handleBroadcast(swarmId, data);
          break;
          
        default:
          console.log(`Unknown message type: ${data.type}`);
      }
      
      // Store message in database for audit
      await this.storeSwarmMessage(swarmId, data);
      
    } catch (error) {
      console.error(`Error handling swarm message:`, error);
    }
  }

  /**
   * Handle database events and relay to swarms
   */
  handleDatabaseEvent(table, payload) {
    console.log(`📊 Database event on ${table}:`, payload.eventType);
    
    // Determine which swarms should receive this event
    const targetSwarms = this.determineTargetSwarms(table, payload);
    
    // Send event to relevant swarms
    targetSwarms.forEach(swarmId => {
      this.sendToSwarm(swarmId, {
        type: 'database_event',
        table,
        event: payload.eventType,
        data: payload.new || payload.old,
        timestamp: new Date().toISOString()
      });
    });
  }

  /**
   * Handle challenge completion and trigger judge swarm
   */
  async handleChallengeCompletion(payload) {
    const challengeId = payload.new.id;
    console.log(`🏁 Challenge completed: ${challengeId}`);
    
    // Find or create judge swarm
    const judgeSwarmId = `judge-${challengeId}`;
    
    // Send judge request to appropriate swarm
    this.sendToSwarm(judgeSwarmId, {
      type: 'judge_request',
      challengeId,
      entries: await this.fetchChallengeEntries(challengeId),
      criteria: payload.new.criteria,
      timestamp: new Date().toISOString()
    });
    
    // Broadcast to monitoring swarms
    this.broadcastToSwarms({
      type: 'challenge_judging_started',
      challengeId,
      timestamp: new Date().toISOString()
    });
  }

  /**
   * Handle heartbeat from swarm
   */
  handleHeartbeat(swarmId) {
    const swarm = this.swarms.get(swarmId);
    if (swarm) {
      swarm.lastHeartbeat = new Date();
      
      // Send heartbeat response
      this.sendToSwarm(swarmId, {
        type: 'heartbeat_ack',
        timestamp: new Date().toISOString()
      });
    }
  }

  /**
   * Handle agent spawned event
   */
  async handleAgentSpawned(swarmId, data) {
    const swarm = this.swarms.get(swarmId);
    if (swarm) {
      swarm.agents.push({
        id: data.agentId,
        type: data.agentType,
        capabilities: data.capabilities,
        status: 'active'
      });
      
      // Store in database
      await this.storeAgentInfo(swarmId, data);
      
      // Notify other swarms if collaborative
      if (data.collaborative) {
        this.broadcastToSwarms({
          type: 'agent_available',
          swarmId,
          agentId: data.agentId,
          agentType: data.agentType,
          capabilities: data.capabilities
        }, swarmId);
      }
    }
  }

  /**
   * Handle task orchestration
   */
  async handleTaskOrchestrated(swarmId, data) {
    const swarm = this.swarms.get(swarmId);
    if (swarm) {
      swarm.tasks.push({
        id: data.taskId,
        description: data.description,
        priority: data.priority,
        status: 'in_progress'
      });
      
      // Store in database
      await this.storeTaskInfo(swarmId, data);
      
      // Check if other swarms can help
      if (data.requestAssistance) {
        await this.requestSwarmAssistance(swarmId, data);
      }
    }
  }

  /**
   * Handle task completion
   */
  async handleTaskCompleted(swarmId, data) {
    const swarm = this.swarms.get(swarmId);
    if (swarm) {
      const task = swarm.tasks.find(t => t.id === data.taskId);
      if (task) {
        task.status = 'completed';
        task.result = data.result;
      }
      
      // Update database
      await this.updateTaskStatus(data.taskId, 'completed', data.result);
      
      // Notify interested parties
      this.broadcastToSwarms({
        type: 'task_completed',
        swarmId,
        taskId: data.taskId,
        result: data.result,
        timestamp: new Date().toISOString()
      });
    }
  }

  /**
   * Handle performance metrics
   */
  async handlePerformanceMetrics(swarmId, data) {
    // Store metrics in database
    await this.storePerformanceMetrics(swarmId, data.metrics);
    
    // Check for performance issues
    const issues = this.analyzePerformance(data.metrics);
    if (issues.length > 0) {
      // Send optimization suggestions
      this.sendToSwarm(swarmId, {
        type: 'optimization_suggestions',
        issues,
        suggestions: this.generateOptimizations(issues),
        timestamp: new Date().toISOString()
      });
    }
  }

  /**
   * Handle coordination requests between swarms
   */
  async handleCoordinationRequest(swarmId, data) {
    console.log(`🤝 Coordination request from ${swarmId}`);
    
    // Find suitable partner swarms
    const partners = this.findPartnerSwarms(data.requirements);
    
    if (partners.length > 0) {
      // Setup coordination channel
      const coordinationId = this.setupCoordination(swarmId, partners, data);
      
      // Notify all parties
      [swarmId, ...partners].forEach(id => {
        this.sendToSwarm(id, {
          type: 'coordination_established',
          coordinationId,
          participants: [swarmId, ...partners],
          task: data.task,
          timestamp: new Date().toISOString()
        });
      });
    } else {
      // No suitable partners found
      this.sendToSwarm(swarmId, {
        type: 'coordination_failed',
        reason: 'No suitable partner swarms available',
        timestamp: new Date().toISOString()
      });
    }
  }

  /**
   * Handle memory synchronization
   */
  async handleMemorySync(swarmId, data) {
    // Store memory snapshot
    await this.storeMemorySnapshot(swarmId, data.memory);
    
    // Check if other swarms need this memory
    if (data.shareWithSwarms) {
      data.shareWithSwarms.forEach(targetId => {
        this.sendToSwarm(targetId, {
          type: 'memory_shared',
          fromSwarm: swarmId,
          memory: data.memory,
          timestamp: new Date().toISOString()
        });
      });
    }
  }

  /**
   * Handle broadcast messages
   */
  handleBroadcast(swarmId, data) {
    this.broadcastToSwarms({
      type: 'broadcast_message',
      fromSwarm: swarmId,
      message: data.message,
      priority: data.priority || 'normal',
      timestamp: new Date().toISOString()
    }, swarmId);
  }

  /**
   * Send message to specific swarm
   */
  sendToSwarm(swarmId, message) {
    const swarm = this.swarms.get(swarmId);
    if (swarm && swarm.ws.readyState === WebSocket.OPEN) {
      swarm.ws.send(JSON.stringify(message));
    }
  }

  /**
   * Broadcast message to all swarms except sender
   */
  broadcastToSwarms(message, excludeSwarmId = null) {
    this.swarms.forEach((swarm, swarmId) => {
      if (swarmId !== excludeSwarmId && swarm.ws.readyState === WebSocket.OPEN) {
        swarm.ws.send(JSON.stringify(message));
      }
    });
  }

  /**
   * Start heartbeat monitoring
   */
  startHeartbeat() {
    this.heartbeatInterval = setInterval(() => {
      const now = new Date();
      const timeout = 60000; // 60 seconds
      
      this.swarms.forEach((swarm, swarmId) => {
        const lastHeartbeat = swarm.lastHeartbeat;
        if (now - lastHeartbeat > timeout) {
          console.log(`💔 Swarm ${swarmId} timed out`);
          this.unregisterSwarm(swarmId);
        }
      });
    }, 30000); // Check every 30 seconds
  }

  /**
   * Helper methods for database operations
   */
  async updateSwarmStatus(swarmId, status) {
    try {
      await this.supabase
        .from('swarm_status')
        .upsert({
          swarm_id: swarmId,
          status,
          updated_at: new Date().toISOString()
        });
    } catch (error) {
      console.error('Error updating swarm status:', error);
    }
  }

  async storeSwarmMessage(swarmId, message) {
    try {
      await this.supabase
        .from('swarm_messages')
        .insert({
          swarm_id: swarmId,
          message_type: message.type,
          payload: message,
          created_at: new Date().toISOString()
        });
    } catch (error) {
      console.error('Error storing swarm message:', error);
    }
  }

  async storeAgentInfo(swarmId, agentData) {
    try {
      await this.supabase
        .from('swarm_agents')
        .insert({
          swarm_id: swarmId,
          agent_id: agentData.agentId,
          agent_type: agentData.agentType,
          capabilities: agentData.capabilities,
          created_at: new Date().toISOString()
        });
    } catch (error) {
      console.error('Error storing agent info:', error);
    }
  }

  async storeTaskInfo(swarmId, taskData) {
    try {
      await this.supabase
        .from('swarm_tasks')
        .insert({
          swarm_id: swarmId,
          task_id: taskData.taskId,
          description: taskData.description,
          priority: taskData.priority,
          status: 'in_progress',
          created_at: new Date().toISOString()
        });
    } catch (error) {
      console.error('Error storing task info:', error);
    }
  }

  async updateTaskStatus(taskId, status, result) {
    try {
      await this.supabase
        .from('swarm_tasks')
        .update({
          status,
          result,
          completed_at: status === 'completed' ? new Date().toISOString() : null
        })
        .eq('task_id', taskId);
    } catch (error) {
      console.error('Error updating task status:', error);
    }
  }

  async storePerformanceMetrics(swarmId, metrics) {
    try {
      await this.supabase
        .from('swarm_metrics')
        .insert({
          swarm_id: swarmId,
          metrics,
          created_at: new Date().toISOString()
        });
    } catch (error) {
      console.error('Error storing performance metrics:', error);
    }
  }

  async storeMemorySnapshot(swarmId, memory) {
    try {
      await this.supabase
        .from('swarm_memory')
        .upsert({
          swarm_id: swarmId,
          memory_snapshot: memory,
          updated_at: new Date().toISOString()
        });
    } catch (error) {
      console.error('Error storing memory snapshot:', error);
    }
  }

  async fetchChallengeEntries(challengeId) {
    try {
      const { data, error } = await this.supabase
        .from('challenge_entries')
        .select('*')
        .eq('challenge_id', challengeId);
      
      if (error) throw error;
      return data;
    } catch (error) {
      console.error('Error fetching challenge entries:', error);
      return [];
    }
  }

  /**
   * Helper methods for swarm coordination
   */
  extractSwarmId(url) {
    const match = url.match(/swarm\/([^\/]+)/);
    return match ? match[1] : `swarm-${Date.now()}`;
  }

  getCapabilities() {
    return {
      realtime: true,
      database_sync: true,
      memory_sharing: true,
      coordination: true,
      performance_monitoring: true,
      challenge_judging: true
    };
  }

  determineTargetSwarms(table, payload) {
    // Logic to determine which swarms should receive database events
    const allSwarmIds = Array.from(this.swarms.keys());
    
    // Filter based on table and event type
    switch (table) {
      case 'swarm_tasks':
        // All swarms get task updates
        return allSwarmIds;
      case 'agent_status':
        // Only swarms with matching agents
        return allSwarmIds.filter(id => {
          const swarm = this.swarms.get(id);
          return swarm && swarm.agents.some(a => a.id === payload.new?.agent_id);
        });
      default:
        return allSwarmIds;
    }
  }

  findPartnerSwarms(requirements) {
    const partners = [];
    
    this.swarms.forEach((swarm, swarmId) => {
      // Check if swarm meets requirements
      const meetsRequirements = requirements.every(req => {
        if (req.type === 'agent_type') {
          return swarm.agents.some(a => a.type === req.value);
        }
        if (req.type === 'min_agents') {
          return swarm.agents.length >= req.value;
        }
        if (req.type === 'capability') {
          return swarm.agents.some(a => a.capabilities.includes(req.value));
        }
        return false;
      });
      
      if (meetsRequirements) {
        partners.push(swarmId);
      }
    });
    
    return partners;
  }

  setupCoordination(initiatorId, partnerIds, data) {
    const coordinationId = `coord-${Date.now()}`;
    
    // Create coordination channel
    const channel = this.supabase
      .channel(`coordination-${coordinationId}`)
      .on('broadcast', { event: 'message' }, (payload) => {
        // Relay coordination messages
        [initiatorId, ...partnerIds].forEach(swarmId => {
          this.sendToSwarm(swarmId, {
            type: 'coordination_message',
            coordinationId,
            message: payload,
            timestamp: new Date().toISOString()
          });
        });
      })
      .subscribe();
    
    this.channels.set(coordinationId, channel);
    
    return coordinationId;
  }

  analyzePerformance(metrics) {
    const issues = [];
    
    if (metrics.cpu > 90) {
      issues.push({ type: 'high_cpu', value: metrics.cpu });
    }
    if (metrics.memory > 85) {
      issues.push({ type: 'high_memory', value: metrics.memory });
    }
    if (metrics.taskQueueSize > 100) {
      issues.push({ type: 'task_backlog', value: metrics.taskQueueSize });
    }
    if (metrics.errorRate > 0.05) {
      issues.push({ type: 'high_error_rate', value: metrics.errorRate });
    }
    
    return issues;
  }

  generateOptimizations(issues) {
    const suggestions = [];
    
    issues.forEach(issue => {
      switch (issue.type) {
        case 'high_cpu':
          suggestions.push({
            type: 'scale_agents',
            action: 'spawn_more_agents',
            reason: 'High CPU utilization detected'
          });
          break;
        case 'high_memory':
          suggestions.push({
            type: 'memory_cleanup',
            action: 'clear_unused_memory',
            reason: 'High memory usage detected'
          });
          break;
        case 'task_backlog':
          suggestions.push({
            type: 'task_distribution',
            action: 'request_swarm_assistance',
            reason: 'Large task queue detected'
          });
          break;
        case 'high_error_rate':
          suggestions.push({
            type: 'error_analysis',
            action: 'review_error_logs',
            reason: 'High error rate detected'
          });
          break;
      }
    });
    
    return suggestions;
  }

  /**
   * Cleanup on shutdown
   */
  async shutdown() {
    console.log('🛑 Shutting down Real-time Swarm Coordinator...');
    
    // Clear heartbeat interval
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
    }
    
    // Close all swarm connections
    this.swarms.forEach((swarm, swarmId) => {
      this.sendToSwarm(swarmId, {
        type: 'coordinator_shutdown',
        timestamp: new Date().toISOString()
      });
      swarm.ws.close();
    });
    
    // Unsubscribe from all channels
    this.channels.forEach(channel => {
      this.supabase.removeChannel(channel);
    });
    
    // Close WebSocket server
    if (this.wsServer) {
      this.wsServer.close();
    }
    
    console.log('✅ Coordinator shutdown complete');
  }
}

// Export singleton instance
export const realtimeCoordinator = new RealtimeSwarmCoordinator();

// Auto-initialize if running as standalone
if (import.meta.url === `file://${process.argv[1]}`) {
  realtimeCoordinator.initialize()
    .then(() => console.log('🚀 Real-time Swarm Coordinator running'))
    .catch(error => console.error('Failed to initialize coordinator:', error));
  
  // Handle shutdown gracefully
  process.on('SIGINT', async () => {
    await realtimeCoordinator.shutdown();
    process.exit(0);
  });
}