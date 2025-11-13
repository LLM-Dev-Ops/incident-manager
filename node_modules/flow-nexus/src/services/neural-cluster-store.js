/**
 * Neural Cluster State Store
 * Persists distributed neural cluster data across tool calls
 */

import { db } from './supabase.js';

class NeuralClusterStore {
  constructor() {
    // In-memory cache for quick access
    this.clusters = new Map();
    this.nodes = new Map();
    this.sessions = new Map();
    
    // Use database for persistence if available
    this.useDatabase = true;
  }

  /**
   * Initialize or get existing cluster
   */
  async initCluster(clusterId, clusterData) {
    // Store in memory
    this.clusters.set(clusterId, clusterData);
    
    // Try to persist to database
    if (this.useDatabase) {
      try {
        const { error } = await db.client
          .from('neural_clusters')
          .upsert({
            id: clusterId,
            name: clusterData.name,
            topology: clusterData.topology,
            architecture: clusterData.neuralArchitecture,
            wasm_enabled: clusterData.wasmOptimization,
            daa_enabled: clusterData.daaEnabled,
            status: clusterData.status,
            metadata: clusterData.metadata,
            created_at: clusterData.createdAt
          });
        
        if (error && error.message.includes('does not exist')) {
          // Table doesn't exist, fallback to memory only
          console.log('Neural clusters table not found, using memory storage only');
          this.useDatabase = false;
        }
      } catch (err) {
        console.log('Database storage unavailable, using memory storage');
        this.useDatabase = false;
      }
    }
    
    return clusterData;
  }

  /**
   * Get cluster by ID
   */
  async getCluster(clusterId) {
    // Check memory first
    if (this.clusters.has(clusterId)) {
      return this.clusters.get(clusterId);
    }
    
    // Try database if available
    if (this.useDatabase) {
      try {
        const { data, error } = await db.client
          .from('neural_clusters')
          .select('*')
          .eq('id', clusterId)
          .single();
        
        if (data) {
          // Reconstruct cluster object
          const cluster = {
            id: data.id,
            name: data.name,
            topology: data.topology,
            neuralArchitecture: data.architecture,
            wasmOptimization: data.wasm_enabled,
            daaEnabled: data.daa_enabled,
            status: data.status,
            metadata: data.metadata,
            createdAt: data.created_at,
            nodes: []
          };
          
          // Cache it
          this.clusters.set(clusterId, cluster);
          
          // Load associated nodes
          await this.loadClusterNodes(clusterId);
          
          return cluster;
        }
      } catch (err) {
        console.log('Could not load cluster from database');
      }
    }
    
    return null;
  }

  /**
   * Add node to cluster
   */
  async addNode(clusterId, node) {
    const cluster = await this.getCluster(clusterId);
    if (!cluster) {
      throw new Error(`Cluster ${clusterId} not found`);
    }
    
    // Add to cluster's node array
    if (!cluster.nodes) {
      cluster.nodes = [];
    }
    cluster.nodes.push(node);
    
    // Store node separately for quick access
    const nodeKey = `${clusterId}:${node.id}`;
    this.nodes.set(nodeKey, node);
    
    // Update cluster in memory
    this.clusters.set(clusterId, cluster);
    
    // Persist to database if available
    if (this.useDatabase) {
      try {
        await db.client
          .from('neural_nodes')
          .upsert({
            id: node.id,
            cluster_id: clusterId,
            role: node.role,
            sandbox_id: node.sandboxId,
            status: node.status,
            capabilities: node.capabilities,
            connections: node.connections,
            model: node.neuralModel,
            metadata: {
              wasmModules: node.wasmModules,
              daaAgent: node.daaAgent,
              metrics: node.metrics
            }
          });
      } catch (err) {
        console.log('Could not persist node to database');
      }
    }
    
    return node;
  }

  /**
   * Load nodes for a cluster
   */
  async loadClusterNodes(clusterId) {
    if (this.useDatabase) {
      try {
        const { data: nodes } = await db.client
          .from('neural_nodes')
          .select('*')
          .eq('cluster_id', clusterId);
        
        if (nodes) {
          const cluster = this.clusters.get(clusterId);
          if (cluster) {
            cluster.nodes = nodes.map(n => ({
              id: n.id,
              clusterId: n.cluster_id,
              role: n.role,
              sandboxId: n.sandbox_id,
              status: n.status,
              capabilities: n.capabilities,
              connections: n.connections || [],
              neuralModel: n.model,
              wasmModules: n.metadata?.wasmModules || [],
              daaAgent: n.metadata?.daaAgent,
              metrics: n.metadata?.metrics || {}
            }));
            
            // Cache nodes
            nodes.forEach(n => {
              const nodeKey = `${clusterId}:${n.id}`;
              this.nodes.set(nodeKey, n);
            });
          }
        }
      } catch (err) {
        console.log('Could not load nodes from database');
      }
    }
  }

  /**
   * Update cluster connections
   */
  async updateConnections(clusterId, connections) {
    const cluster = await this.getCluster(clusterId);
    if (!cluster) {
      throw new Error(`Cluster ${clusterId} not found`);
    }
    
    cluster.connections = connections;
    cluster.status = 'connected';
    
    // Apply connections to nodes
    if (cluster.nodes) {
      connections.forEach(conn => {
        const fromNode = cluster.nodes.find(n => n.id === conn.from);
        const toNode = cluster.nodes.find(n => n.id === conn.to);
        
        if (fromNode && toNode) {
          if (!fromNode.connections) fromNode.connections = [];
          if (!toNode.connections) toNode.connections = [];
          
          fromNode.connections.push(conn.to);
          if (conn.type === 'bidirectional') {
            toNode.connections.push(conn.from);
          }
        }
      });
    }
    
    this.clusters.set(clusterId, cluster);
    
    // Update in database if available
    if (this.useDatabase) {
      try {
        await db.client
          .from('neural_clusters')
          .update({
            status: 'connected',
            metadata: {
              ...cluster.metadata,
              connections: connections
            }
          })
          .eq('id', clusterId);
      } catch (err) {
        console.log('Could not update connections in database');
      }
    }
    
    return connections;
  }

  /**
   * Add training session
   */
  async addSession(sessionId, sessionData) {
    this.sessions.set(sessionId, sessionData);
    
    if (this.useDatabase) {
      try {
        await db.client
          .from('neural_sessions')
          .upsert({
            id: sessionId,
            cluster_id: sessionData.clusterId,
            status: sessionData.status,
            config: sessionData.config,
            metrics: sessionData.metrics,
            started_at: new Date(sessionData.startTime).toISOString()
          });
      } catch (err) {
        console.log('Could not persist session to database');
      }
    }
    
    return sessionData;
  }

  /**
   * Get all clusters
   */
  async getAllClusters() {
    const clusters = [];
    
    // Get from memory
    for (const cluster of this.clusters.values()) {
      clusters.push(cluster);
    }
    
    // Also check database if available
    if (this.useDatabase && clusters.length === 0) {
      try {
        const { data } = await db.client
          .from('neural_clusters')
          .select('*')
          .order('created_at', { ascending: false });
        
        if (data) {
          for (const row of data) {
            const cluster = {
              id: row.id,
              name: row.name,
              topology: row.topology,
              neuralArchitecture: row.architecture,
              wasmOptimization: row.wasm_enabled,
              daaEnabled: row.daa_enabled,
              status: row.status,
              metadata: row.metadata,
              createdAt: row.created_at,
              nodes: []
            };
            
            // Cache it
            this.clusters.set(cluster.id, cluster);
            clusters.push(cluster);
          }
        }
      } catch (err) {
        console.log('Could not load clusters from database');
      }
    }
    
    return clusters;
  }

  /**
   * Delete cluster
   */
  async deleteCluster(clusterId) {
    // Remove from memory
    this.clusters.delete(clusterId);
    
    // Remove associated nodes from memory
    for (const [key, node] of this.nodes.entries()) {
      if (key.startsWith(`${clusterId}:`)) {
        this.nodes.delete(key);
      }
    }
    
    // Remove associated sessions
    for (const [sessionId, session] of this.sessions.entries()) {
      if (session.clusterId === clusterId) {
        this.sessions.delete(sessionId);
      }
    }
    
    // Remove from database if available
    if (this.useDatabase) {
      try {
        // Delete nodes first (foreign key constraint)
        await db.client
          .from('neural_nodes')
          .delete()
          .eq('cluster_id', clusterId);
        
        // Delete sessions
        await db.client
          .from('neural_sessions')
          .delete()
          .eq('cluster_id', clusterId);
        
        // Delete cluster
        await db.client
          .from('neural_clusters')
          .delete()
          .eq('id', clusterId);
      } catch (err) {
        console.log('Could not delete cluster from database');
      }
    }
    
    return true;
  }
}

// Export singleton instance
export const neuralClusterStore = new NeuralClusterStore();