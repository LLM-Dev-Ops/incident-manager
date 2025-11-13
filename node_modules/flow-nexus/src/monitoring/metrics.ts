import { EventEmitter } from 'events';

export interface SystemMetrics {
  cpu: {
    usage: number;
    loadAverage: number[];
  };
  memory: {
    used: number;
    total: number;
    percentage: number;
    heap: NodeJS.MemoryUsage;
  };
  agents: {
    total: number;
    active: number;
    idle: number;
    error: number;
    byType: Record<string, number>;
  };
  tasks: {
    completed: number;
    failed: number;
    averageExecutionTime: number;
    throughput: number; // tasks per minute
  };
  system: {
    uptime: number;
    nodeVersion: string;
    platform: string;
    pid: number;
  };
}

export interface AgentEvent {
  type: 'spawned' | 'terminated' | 'error';
  agentId: string;
  agentType: string;
  timestamp: Date;
  metadata?: any;
}

export interface TaskEvent {
  agentId: string;
  functionName: string;
  executionTime: number;
  success: boolean;
  timestamp: Date;
  error?: string;
}

export interface ServiceEvent {
  service: string;
  event: 'connected' | 'disconnected' | 'error';
  timestamp: Date;
  metadata?: any;
}

export class MonitoringService extends EventEmitter {
  private agentEvents: AgentEvent[] = [];
  private taskEvents: TaskEvent[] = [];
  private serviceEvents: ServiceEvent[] = [];
  private metricsInterval?: NodeJS.Timeout;
  private isInitialized = false;
  private healthStatus = true;

  // Counters
  private counters = {
    agentsSpawned: 0,
    agentsTerminated: 0,
    agentErrors: 0,
    tasksCompleted: 0,
    tasksFailed: 0,
    totalExecutionTime: 0
  };

  // Gauges
  private gauges = {
    activeAgents: 0,
    idleAgents: 0,
    errorAgents: 0,
    currentThroughput: 0
  };

  async initialize(): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    try {
      // Start metrics collection
      this.startMetricsCollection();
      
      // Set up event cleanup
      this.startEventCleanup();
      
      this.isInitialized = true;
      this.healthStatus = true;
      
      console.log('Monitoring service initialized');
      this.emit('initialized');
    } catch (error) {
      console.error('Failed to initialize monitoring service:', error);
      this.healthStatus = false;
      throw error;
    }
  }

  recordAgentEvent(type: AgentEvent['type'], data: any): void {
    const event: AgentEvent = {
      type,
      agentId: data.agentId,
      agentType: data.type || 'unknown',
      timestamp: new Date(),
      metadata: data
    };

    this.agentEvents.push(event);
    
    // Update counters
    switch (type) {
      case 'spawned':
        this.counters.agentsSpawned++;
        this.gauges.activeAgents++;
        break;
      case 'terminated':
        this.counters.agentsTerminated++;
        this.gauges.activeAgents = Math.max(0, this.gauges.activeAgents - 1);
        break;
      case 'error':
        this.counters.agentErrors++;
        this.gauges.errorAgents++;
        break;
    }

    this.emit('agentEvent', event);
  }

  recordTaskCompletion(data: any): void {
    const event: TaskEvent = {
      agentId: data.agentId,
      functionName: data.functionName,
      executionTime: data.executionTime,
      success: true,
      timestamp: new Date()
    };

    this.taskEvents.push(event);
    
    // Update counters
    this.counters.tasksCompleted++;
    this.counters.totalExecutionTime += data.executionTime;

    this.emit('taskCompleted', event);
  }

  recordTaskError(data: any): void {
    const event: TaskEvent = {
      agentId: data.agentId,
      functionName: data.functionName,
      executionTime: data.executionTime || 0,
      success: false,
      timestamp: new Date(),
      error: data.error
    };

    this.taskEvents.push(event);
    
    // Update counters
    this.counters.tasksFailed++;

    this.emit('taskError', event);
  }

  recordServiceEvent(service: string, event: ServiceEvent['event'], metadata?: any): void {
    const serviceEvent: ServiceEvent = {
      service,
      event,
      timestamp: new Date(),
      metadata
    };

    this.serviceEvents.push(serviceEvent);
    this.emit('serviceEvent', serviceEvent);
  }

  recordServiceError(service: string, error: any): void {
    this.recordServiceEvent(service, 'error', { error: error.toString() });
  }

  async getMetrics(): Promise<SystemMetrics> {
    const memoryUsage = process.memoryUsage();
    const cpuUsage = process.cpuUsage();
    
    // Calculate CPU percentage (simplified)
    const cpuPercent = this.calculateCpuUsage(cpuUsage);
    
    // Calculate throughput (tasks per minute)
    const now = Date.now();
    const oneMinuteAgo = now - 60000;
    const recentTasks = this.taskEvents.filter(e => e.timestamp.getTime() > oneMinuteAgo);
    const throughput = recentTasks.length;

    // Calculate average execution time
    const totalExecutionTime = this.taskEvents.reduce((sum, e) => sum + e.executionTime, 0);
    const averageExecutionTime = this.taskEvents.length > 0 ? 
      totalExecutionTime / this.taskEvents.length : 0;

    // Count agents by type
    const recentAgentEvents = this.agentEvents.slice(-100); // Last 100 events
    const agentsByType: Record<string, number> = {};
    
    for (const event of recentAgentEvents) {
      if (event.type === 'spawned') {
        agentsByType[event.agentType] = (agentsByType[event.agentType] || 0) + 1;
      }
    }

    return {
      cpu: {
        usage: cpuPercent,
        loadAverage: process.platform !== 'win32' ? require('os').loadavg() : [0, 0, 0]
      },
      memory: {
        used: memoryUsage.heapUsed,
        total: memoryUsage.heapTotal,
        percentage: (memoryUsage.heapUsed / memoryUsage.heapTotal) * 100,
        heap: memoryUsage
      },
      agents: {
        total: this.gauges.activeAgents + this.gauges.idleAgents,
        active: this.gauges.activeAgents,
        idle: this.gauges.idleAgents,
        error: this.gauges.errorAgents,
        byType: agentsByType
      },
      tasks: {
        completed: this.counters.tasksCompleted,
        failed: this.counters.tasksFailed,
        averageExecutionTime,
        throughput
      },
      system: {
        uptime: process.uptime(),
        nodeVersion: process.version,
        platform: process.platform,
        pid: process.pid
      }
    };
  }

  getAgentEvents(limit: number = 100): AgentEvent[] {
    return this.agentEvents.slice(-limit);
  }

  getTaskEvents(limit: number = 100): TaskEvent[] {
    return this.taskEvents.slice(-limit);
  }

  getServiceEvents(limit: number = 100): ServiceEvent[] {
    return this.serviceEvents.slice(-limit);
  }

  // Get performance statistics for a specific time window
  getPerformanceStats(windowMs: number = 3600000): {
    tasksCompleted: number;
    tasksFailed: number;
    averageExecutionTime: number;
    successRate: number;
    throughput: number;
  } {
    const now = Date.now();
    const windowStart = now - windowMs;
    
    const windowTasks = this.taskEvents.filter(e => e.timestamp.getTime() > windowStart);
    const completedTasks = windowTasks.filter(e => e.success);
    const failedTasks = windowTasks.filter(e => !e.success);
    
    const totalExecutionTime = completedTasks.reduce((sum, e) => sum + e.executionTime, 0);
    const averageExecutionTime = completedTasks.length > 0 ? 
      totalExecutionTime / completedTasks.length : 0;
    
    const successRate = windowTasks.length > 0 ? 
      (completedTasks.length / windowTasks.length) * 100 : 0;
    
    const throughput = (windowTasks.length / windowMs) * 60000; // tasks per minute

    return {
      tasksCompleted: completedTasks.length,
      tasksFailed: failedTasks.length,
      averageExecutionTime,
      successRate,
      throughput
    };
  }

  // Get metrics for a specific agent
  getAgentMetrics(agentId: string): {
    tasksCompleted: number;
    tasksFailed: number;
    averageExecutionTime: number;
    lastActivity: Date | null;
    errorRate: number;
  } {
    const agentTasks = this.taskEvents.filter(e => e.agentId === agentId);
    const completedTasks = agentTasks.filter(e => e.success);
    const failedTasks = agentTasks.filter(e => !e.success);
    
    const totalExecutionTime = completedTasks.reduce((sum, e) => sum + e.executionTime, 0);
    const averageExecutionTime = completedTasks.length > 0 ? 
      totalExecutionTime / completedTasks.length : 0;
    
    const lastActivity = agentTasks.length > 0 ? 
      agentTasks[agentTasks.length - 1].timestamp : null;
    
    const errorRate = agentTasks.length > 0 ? 
      (failedTasks.length / agentTasks.length) * 100 : 0;

    return {
      tasksCompleted: completedTasks.length,
      tasksFailed: failedTasks.length,
      averageExecutionTime,
      lastActivity,
      errorRate
    };
  }

  // Health check
  isHealthy(): boolean {
    return this.healthStatus && this.isInitialized;
  }

  // Set health status
  setHealthStatus(healthy: boolean): void {
    this.healthStatus = healthy;
    this.emit('healthChanged', healthy);
  }

  private calculateCpuUsage(cpuUsage: NodeJS.CpuUsage): number {
    // Simplified CPU usage calculation
    // In a real implementation, you would track changes over time
    const totalUsage = cpuUsage.user + cpuUsage.system;
    return Math.min(100, (totalUsage / 1000000) * 100); // Convert to percentage
  }

  private startMetricsCollection(): void {
    // Collect metrics every 30 seconds
    this.metricsInterval = setInterval(async () => {
      try {
        const metrics = await this.getMetrics();
        this.emit('metricsCollected', metrics);
        
        // Update current throughput gauge
        this.gauges.currentThroughput = metrics.tasks.throughput;
        
        // Check for anomalies
        this.checkForAnomalies(metrics);
      } catch (error) {
        console.error('Error collecting metrics:', error);
        this.setHealthStatus(false);
      }
    }, 30000);
  }

  private checkForAnomalies(metrics: SystemMetrics): void {
    // Check for high memory usage
    if (metrics.memory.percentage > 90) {
      this.emit('anomaly', {
        type: 'high_memory_usage',
        value: metrics.memory.percentage,
        threshold: 90
      });
    }

    // Check for high error rate
    const recentStats = this.getPerformanceStats(300000); // Last 5 minutes
    if (recentStats.successRate < 80 && recentStats.tasksCompleted > 10) {
      this.emit('anomaly', {
        type: 'high_error_rate',
        value: 100 - recentStats.successRate,
        threshold: 20
      });
    }

    // Check for low throughput
    if (recentStats.throughput < 1 && this.gauges.activeAgents > 0) {
      this.emit('anomaly', {
        type: 'low_throughput',
        value: recentStats.throughput,
        threshold: 1
      });
    }
  }

  private startEventCleanup(): void {
    // Clean up old events every hour
    setInterval(() => {
      const maxAge = 24 * 60 * 60 * 1000; // 24 hours
      const cutoff = Date.now() - maxAge;
      
      this.agentEvents = this.agentEvents.filter(e => e.timestamp.getTime() > cutoff);
      this.taskEvents = this.taskEvents.filter(e => e.timestamp.getTime() > cutoff);
      this.serviceEvents = this.serviceEvents.filter(e => e.timestamp.getTime() > cutoff);
      
      console.log('Cleaned up old monitoring events');
    }, 3600000); // Every hour
  }

  async cleanup(): Promise<void> {
    if (this.metricsInterval) {
      clearInterval(this.metricsInterval);
    }

    this.agentEvents = [];
    this.taskEvents = [];
    this.serviceEvents = [];
    
    this.isInitialized = false;
    this.healthStatus = false;
    
    console.log('Monitoring service cleanup completed');
    this.emit('cleanup');
  }

  // Export metrics in Prometheus format (simplified)
  getPrometheusMetrics(): string {
    const metrics: string[] = [];
    
    metrics.push(`# HELP daa_agents_total Total number of DAA agents`);
    metrics.push(`# TYPE daa_agents_total gauge`);
    metrics.push(`daa_agents_total{status="active"} ${this.gauges.activeAgents}`);
    metrics.push(`daa_agents_total{status="idle"} ${this.gauges.idleAgents}`);
    metrics.push(`daa_agents_total{status="error"} ${this.gauges.errorAgents}`);
    
    metrics.push(`# HELP daa_tasks_total Total number of completed tasks`);
    metrics.push(`# TYPE daa_tasks_total counter`);
    metrics.push(`daa_tasks_total{status="completed"} ${this.counters.tasksCompleted}`);
    metrics.push(`daa_tasks_total{status="failed"} ${this.counters.tasksFailed}`);
    
    metrics.push(`# HELP daa_task_duration_avg Average task execution time in milliseconds`);
    metrics.push(`# TYPE daa_task_duration_avg gauge`);
    const avgDuration = this.counters.tasksCompleted > 0 ? 
      this.counters.totalExecutionTime / this.counters.tasksCompleted : 0;
    metrics.push(`daa_task_duration_avg ${avgDuration}`);
    
    metrics.push(`# HELP daa_throughput_current Current throughput in tasks per minute`);
    metrics.push(`# TYPE daa_throughput_current gauge`);
    metrics.push(`daa_throughput_current ${this.gauges.currentThroughput}`);
    
    return metrics.join('\n') + '\n';
  }
}