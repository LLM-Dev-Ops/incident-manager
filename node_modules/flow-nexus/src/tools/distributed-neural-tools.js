/**
 * MCP Tools for Distributed Neural Network Deployment
 * Integrates with existing E2B sandbox capabilities and DAA system
 */

import { distributedNeuralSandbox } from '../services/distributed-neural-sandbox.js';
import { db } from '../services/supabase.js';
import { e2b } from '../services/e2b-service.js';

export const distributedNeuralTools = [
  {
    name: 'neural_cluster_init',
    description: 'Initialize a distributed neural network cluster using E2B sandboxes',
    inputSchema: {
      type: 'object',
      properties: {
        name: { 
          type: 'string', 
          description: 'Cluster name' 
        },
        topology: { 
          type: 'string',
          enum: ['mesh', 'ring', 'star', 'hierarchical'],
          default: 'mesh',
          description: 'Network topology for node connections'
        },
        architecture: {
          type: 'string',
          enum: ['transformer', 'cnn', 'rnn', 'gnn', 'hybrid'],
          default: 'transformer',
          description: 'Neural network architecture'
        },
        wasmOptimization: {
          type: 'boolean',
          default: true,
          description: 'Enable WASM acceleration'
        },
        daaEnabled: {
          type: 'boolean',
          default: true,
          description: 'Enable DAA autonomous coordination'
        },
        consensus: {
          type: 'string',
          enum: ['proof-of-learning', 'byzantine', 'raft', 'gossip'],
          default: 'proof-of-learning',
          description: 'DAA consensus mechanism'
        }
      },
      required: ['name']
    },
    handler: async (args) => {
      try {
        const cluster = await distributedNeuralSandbox.initializeNeuralCluster({
          name: args.name,
          topology: args.topology,
          architecture: args.architecture,
          wasmOptimization: args.wasmOptimization,
          daaEnabled: args.daaEnabled,
          consensus: args.consensus,
          metadata: {
            created_via: 'mcp_tool',
            timestamp: new Date().toISOString()
          }
        });

        return {
          success: true,
          cluster_id: cluster.id,
          status: cluster.status,
          topology: cluster.topology,
          architecture: cluster.neuralArchitecture,
          wasm_enabled: cluster.wasmOptimization,
          daa_enabled: cluster.daaEnabled
        };
      } catch (error) {
        return {
          success: false,
          error: error.message
        };
      }
    }
  },

  {
    name: 'neural_node_deploy',
    description: 'Deploy a neural network node in an E2B sandbox',
    inputSchema: {
      type: 'object',
      properties: {
        cluster_id: {
          type: 'string',
          description: 'Cluster ID to deploy node into'
        },
        node_type: {
          type: 'string',
          enum: ['worker', 'parameter_server', 'aggregator', 'validator'],
          default: 'worker',
          description: 'Node type/role in the distributed system'
        },
        role: {
          type: 'string',
          enum: ['worker', 'parameter_server', 'aggregator', 'validator'],
          default: 'worker',
          description: 'Node role in the distributed system'
        },
        template: {
          type: 'string',
          default: 'nodejs',
          description: 'E2B sandbox template (uses existing templates)'
        },
        model: {
          type: 'string',
          enum: ['base', 'large', 'xl', 'custom'],
          default: 'base',
          description: 'Neural model size'
        },
        capabilities: {
          type: 'array',
          items: { type: 'string' },
          description: 'Node capabilities',
          default: ['training', 'inference']
        },
        autonomy: {
          type: 'number',
          minimum: 0,
          maximum: 1,
          default: 0.8,
          description: 'DAA autonomy level (0-1)'
        },
        layers: {
          type: 'array',
          description: 'Custom neural network layers configuration'
        }
      },
      required: ['cluster_id']
    },
    handler: async (args) => {
      try {
        // Check if cluster exists (async now)
        const clusterStatus = await distributedNeuralSandbox.getClusterStatus(args.cluster_id);
        if (!clusterStatus) {
          return {
            success: false,
            error: `Cluster ${args.cluster_id} not found`
          };
        }

        // Deploy node using existing sandbox capabilities
        const nodeConfig = {
          role: args.role || args.node_type || 'worker',
          template: args.template || 'nodejs',
          model: args.model || 'base',
          capabilities: args.capabilities || ['training', 'inference'],
          autonomy: args.autonomy || 0.8,
          topology: clusterStatus.topology,
          layers: args.layers,
          env_vars: {
            // No JWT tokens - using session-based auth
            USE_SESSION_AUTH: 'true',
            CLUSTER_ID: args.cluster_id
          }
        };

        const node = await distributedNeuralSandbox.deployNeuralNode(
          args.cluster_id,
          nodeConfig
        );

        // Use existing sandbox_create flow
        if (node.status === 'deployed' && node.sandboxId) {
          // Store sandbox info in database using existing methods
          await db.client
            .from('sandboxes')
            .update({
              metadata: {
                ...node,
                neural_cluster: args.cluster_id,
                daa_enabled: clusterStatus.daaEnabled,
                wasm_enabled: clusterStatus.wasmStatus === 'loaded'
              }
            })
            .eq('id', node.sandboxId);
        }

        return {
          success: node.status === 'deployed',
          node_id: node.id,
          sandbox_id: node.sandboxId,
          role: node.role,
          status: node.status,
          capabilities: node.capabilities,
          cluster_id: args.cluster_id
        };
      } catch (error) {
        return {
          success: false,
          error: error.message
        };
      }
    }
  },

  {
    name: 'neural_cluster_connect',
    description: 'Connect nodes in the neural cluster based on topology',
    inputSchema: {
      type: 'object',
      properties: {
        cluster_id: {
          type: 'string',
          description: 'Cluster ID'
        },
        topology: {
          type: 'string',
          enum: ['mesh', 'ring', 'star', 'hierarchical'],
          description: 'Override topology (uses cluster default if not specified)'
        }
      },
      required: ['cluster_id']
    },
    handler: async (args) => {
      try {
        const cluster = await distributedNeuralSandbox.getClusterStatus(args.cluster_id);
        if (!cluster) {
          return {
            success: false,
            error: `Cluster ${args.cluster_id} not found`
          };
        }

        const topology = args.topology || cluster.topology;
        const connections = await distributedNeuralSandbox.connectNodes(
          args.cluster_id,
          topology
        );

        return {
          success: true,
          cluster_id: args.cluster_id,
          topology,
          connections: connections.length,
          nodes_connected: cluster.nodeCount
        };
      } catch (error) {
        return {
          success: false,
          error: error.message
        };
      }
    }
  },

  {
    name: 'neural_train_distributed',
    description: 'Start distributed neural network training across sandbox cluster',
    inputSchema: {
      type: 'object',
      properties: {
        cluster_id: {
          type: 'string',
          description: 'Cluster ID'
        },
        dataset: {
          type: 'string',
          description: 'Training dataset identifier or inline data'
        },
        epochs: {
          type: 'integer',
          minimum: 1,
          maximum: 1000,
          default: 10,
          description: 'Number of training epochs'
        },
        batch_size: {
          type: 'integer',
          minimum: 1,
          maximum: 512,
          default: 32,
          description: 'Batch size for training'
        },
        learning_rate: {
          type: 'number',
          minimum: 0.00001,
          maximum: 1,
          default: 0.001,
          description: 'Learning rate'
        },
        optimizer: {
          type: 'string',
          enum: ['adam', 'sgd', 'rmsprop', 'adagrad'],
          default: 'adam',
          description: 'Optimization algorithm'
        },
        federated: {
          type: 'boolean',
          default: false,
          description: 'Enable federated learning mode'
        }
      },
      required: ['cluster_id', 'dataset']
    },
    handler: async (args) => {
      try {
        const cluster = await distributedNeuralSandbox.getClusterStatus(args.cluster_id);
        if (!cluster) {
          return {
            success: false,
            error: `Cluster ${args.cluster_id} not found`
          };
        }

        if (cluster.nodeCount === 0) {
          return {
            success: false,
            error: 'No nodes deployed in cluster'
          };
        }

        // Parse dataset (could be inline JSON or dataset ID)
        let data, labels;
        try {
          const dataset = JSON.parse(args.dataset);
          data = dataset.data || dataset.X || dataset.features;
          labels = dataset.labels || dataset.y || dataset.targets;
        } catch {
          // If not JSON, treat as dataset identifier
          // In production, would fetch from data store
          data = args.dataset;
          labels = null;
        }

        const trainingConfig = {
          data,
          labels,
          epochs: args.epochs || 10,
          batchSize: args.batch_size || 32,
          learningRate: args.learning_rate || 0.001,
          optimizer: args.optimizer || 'adam',
          federated: args.federated || false
        };

        const session = await distributedNeuralSandbox.startDistributedTraining(
          args.cluster_id,
          trainingConfig
        );

        // If using existing sandboxes, trigger training via sandbox_execute
        for (const node of cluster.nodes) {
          if (node.sandboxId && node.status === 'deployed') {
            // Execute training code in sandbox using existing capabilities
            await e2b.executeInSandbox(node.sandboxId, {
              type: 'train',
              config: trainingConfig,
              nodeRole: node.role
            });
          }
        }

        return {
          success: true,
          session_id: session.id,
          cluster_id: args.cluster_id,
          status: session.status,
          nodes_training: cluster.nodeCount,
          config: {
            epochs: trainingConfig.epochs,
            batch_size: trainingConfig.batchSize,
            learning_rate: trainingConfig.learningRate,
            optimizer: trainingConfig.optimizer,
            federated: trainingConfig.federated
          }
        };
      } catch (error) {
        return {
          success: false,
          error: error.message
        };
      }
    }
  },

  {
    name: 'neural_cluster_status',
    description: 'Get status of distributed neural cluster and training sessions',
    inputSchema: {
      type: 'object',
      properties: {
        cluster_id: {
          type: 'string',
          description: 'Cluster ID'
        }
      },
      required: ['cluster_id']
    },
    handler: async (args) => {
      try {
        const status = await distributedNeuralSandbox.getClusterStatus(args.cluster_id);
        
        if (!status) {
          return {
            success: false,
            error: `Cluster ${args.cluster_id} not found`
          };
        }

        // Get sandbox status for each node using existing capabilities
        const nodeStatuses = await Promise.all(
          status.nodes.map(async (node) => {
            if (node.sandboxId) {
              // Use existing sandbox status check
              const { data: sandbox } = await db.client
                .from('sandboxes')
                .select('*')
                .eq('id', node.sandboxId)
                .single();
              
              return {
                node_id: node.id,
                sandbox_id: node.sandboxId,
                role: node.role,
                status: sandbox?.status || node.status,
                connections: node.connections.length,
                metrics: node.metrics
              };
            }
            return {
              node_id: node.id,
              role: node.role,
              status: node.status,
              connections: node.connections.length
            };
          })
        );

        return {
          success: true,
          cluster_id: status.id,
          status: status.status,
          topology: status.topology,
          architecture: status.neuralArchitecture,
          created_at: status.createdAt,
          uptime: status.uptime || `${Math.floor((Date.now() - new Date(status.createdAt).getTime()) / 1000)}s`,
          nodes: nodeStatuses,
          performance: {
            throughput: status.throughput || 'N/A',
            latency: status.latency || 'N/A'
          },
          features: {
            daa_enabled: status.daaStatus === 'active',
            wasm_enabled: status.wasmStatus === 'loaded',
            nodes_deployed: status.nodeCount || nodeStatuses.length,
            connections_active: status.activeConnections || 0,
            training_sessions: status.trainingSessions || 0
          }
        };
      } catch (error) {
        return {
          success: false,
          error: error.message
        };
      }
    }
  },

  {
    name: 'neural_predict_distributed',
    description: 'Run inference across distributed neural network',
    inputSchema: {
      type: 'object',
      properties: {
        cluster_id: {
          type: 'string',
          description: 'Cluster ID'
        },
        input_data: {
          type: 'string',
          description: 'Input data for prediction (JSON)'
        },
        aggregation: {
          type: 'string',
          enum: ['mean', 'majority', 'weighted', 'ensemble'],
          default: 'mean',
          description: 'How to aggregate predictions from multiple nodes'
        }
      },
      required: ['cluster_id', 'input_data']
    },
    handler: async (args) => {
      try {
        const cluster = await distributedNeuralSandbox.getClusterStatus(args.cluster_id);
        if (!cluster) {
          return {
            success: false,
            error: `Cluster ${args.cluster_id} not found`
          };
        }

        // Parse input data
        let inputData;
        try {
          inputData = JSON.parse(args.input_data);
        } catch {
          return {
            success: false,
            error: 'Invalid JSON input data'
          };
        }

        // Run predictions on all nodes
        const predictions = [];
        for (const node of cluster.nodes) {
          if (node.sandboxId && node.capabilities.includes('inference')) {
            // Execute prediction in sandbox using existing capabilities
            const result = await e2b.executeInSandbox(node.sandboxId, {
              type: 'predict',
              data: inputData
            });
            
            if (result) {
              predictions.push({
                node_id: node.id,
                prediction: result.prediction,
                confidence: result.confidence
              });
            }
          }
        }

        // Aggregate predictions based on strategy
        let finalPrediction;
        switch (args.aggregation) {
          case 'mean':
            // Average all predictions
            finalPrediction = predictions.reduce((acc, p) => 
              acc + p.prediction, 0) / predictions.length;
            break;
          case 'majority':
            // Most common prediction
            const counts = {};
            predictions.forEach(p => {
              counts[p.prediction] = (counts[p.prediction] || 0) + 1;
            });
            finalPrediction = Object.keys(counts).reduce((a, b) => 
              counts[a] > counts[b] ? a : b);
            break;
          case 'weighted':
            // Weight by confidence
            const weightedSum = predictions.reduce((acc, p) => 
              acc + p.prediction * p.confidence, 0);
            const totalWeight = predictions.reduce((acc, p) => 
              acc + p.confidence, 0);
            finalPrediction = weightedSum / totalWeight;
            break;
          case 'ensemble':
            // Return all predictions for ensemble
            finalPrediction = predictions;
            break;
        }

        return {
          success: true,
          cluster_id: args.cluster_id,
          prediction: finalPrediction,
          aggregation_method: args.aggregation,
          nodes_used: predictions.length,
          individual_predictions: predictions
        };
      } catch (error) {
        return {
          success: false,
          error: error.message
        };
      }
    }
  },

  {
    name: 'neural_cluster_terminate',
    description: 'Terminate distributed neural cluster and cleanup sandboxes',
    inputSchema: {
      type: 'object',
      properties: {
        cluster_id: {
          type: 'string',
          description: 'Cluster ID to terminate'
        }
      },
      required: ['cluster_id']
    },
    handler: async (args) => {
      try {
        const cluster = await distributedNeuralSandbox.getClusterStatus(args.cluster_id);
        if (!cluster) {
          return {
            success: false,
            error: `Cluster ${args.cluster_id} not found`
          };
        }

        // Terminate all sandboxes using existing capabilities
        if (cluster.nodes && cluster.nodes.length > 0) {
          for (const node of cluster.nodes) {
            if (node.sandboxId) {
              // Update sandbox status in database
              await db.client
                .from('sandboxes')
                .update({
                  status: 'terminated',
                  stopped_at: new Date().toISOString()
                })
                .eq('id', node.sandboxId);
            }
          }
        }

        // Terminate cluster
        const terminated = await distributedNeuralSandbox.terminateCluster(args.cluster_id);

        return {
          success: terminated,
          cluster_id: args.cluster_id,
          nodes_terminated: cluster.nodeCount,
          message: `Cluster ${cluster.name} terminated`
        };
      } catch (error) {
        return {
          success: false,
          error: error.message
        };
      }
    }
  }
];

// Export for MCP server integration
export default distributedNeuralTools;