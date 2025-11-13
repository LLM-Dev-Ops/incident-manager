/**
 * MCP Server Darknet Tools
 * Provides darknet capabilities for agent communication
 */

import { Tool } from '@modelcontextprotocol/sdk/types.js';
import { 
  DarknetAddressGenerator, 
  DarknetAddressManager,
  NetworkType,
  Capability,
  type DarknetAddress 
} from '../../src/lib/darknet/address.js';
import { createClient } from '@supabase/supabase-js';
import bcrypt from 'bcrypt';
import { randomBytes } from 'crypto';

// Initialize Supabase client
const supabase = createClient(
  process.env.SUPABASE_URL || '',
  process.env.SUPABASE_SERVICE_ROLE_KEY || ''
);

// Darknet address manager
const addressManager = new DarknetAddressManager();

/**
 * Initialize darknet node
 */
export const darknetInit: Tool = {
  name: 'darknet_init',
  description: 'Initialize a darknet node with secure address generation',
  inputSchema: {
    type: 'object',
    properties: {
      network: {
        type: 'string',
        enum: ['mainnet', 'testnet', 'darknet', 'hybrid'],
        default: 'darknet',
        description: 'Network type for the node'
      },
      capabilities: {
        type: 'array',
        items: {
          type: 'string',
          enum: [
            'agent_spawn',
            'task_orchestrate', 
            'memory_share',
            'swarm_coordinate',
            'consensus_participate',
            'darknet_relay',
            'quantum_resistant'
          ]
        },
        default: ['agent_spawn', 'task_orchestrate'],
        description: 'Node capabilities'
      }
    }
  },
  async execute(params: any) {
    try {
      const network = params.network as NetworkType || NetworkType.DARKNET;
      const capabilities = params.capabilities as Capability[] || [
        Capability.AGENT_SPAWN,
        Capability.TASK_ORCHESTRATE
      ];
      
      // Generate darknet address
      const address = await DarknetAddressGenerator.generate(network, capabilities);
      
      // Store in manager
      addressManager.store('default', address);
      
      // Store in database
      const { error } = await supabase
        .from('darknet_nodes')
        .insert({
          public_key: address.publicKey,
          onion_address: address.onionAddress,
          multiaddr: address.multiaddr,
          network_id: address.networkId,
          capabilities: address.capabilities,
          checksum: address.checksum,
          status: 'active'
        });
      
      if (error) {
        console.error('Failed to store darknet node:', error);
      }
      
      return {
        success: true,
        address: {
          onionAddress: address.onionAddress,
          multiaddr: address.multiaddr,
          publicKey: address.publicKey,
          networkId: address.networkId,
          capabilities: address.capabilities,
          checksum: address.checksum
        },
        message: `Darknet node initialized on ${network} network`
      };
    } catch (error) {
      return {
        success: false,
        error: `Failed to initialize darknet node: ${error}`
      };
    }
  }
};

/**
 * Generate darknet address
 */
export const darknetAddress: Tool = {
  name: 'darknet_address',
  description: 'Generate a new darknet address',
  inputSchema: {
    type: 'object',
    properties: {
      alias: {
        type: 'string',
        description: 'Alias for the address'
      },
      network: {
        type: 'string',
        enum: ['mainnet', 'testnet', 'darknet', 'hybrid'],
        default: 'darknet'
      }
    },
    required: ['alias']
  },
  async execute(params: any) {
    try {
      const address = await DarknetAddressGenerator.generate(
        params.network as NetworkType || NetworkType.DARKNET
      );
      
      // Store with alias
      addressManager.store(params.alias, address);
      
      return {
        success: true,
        alias: params.alias,
        address: {
          onionAddress: address.onionAddress,
          publicKey: address.publicKey,
          checksum: address.checksum
        }
      };
    } catch (error) {
      return {
        success: false,
        error: `Failed to generate address: ${error}`
      };
    }
  }
};

/**
 * Register darknet user
 */
export const darknetRegister: Tool = {
  name: 'darknet_register',
  description: 'Register a user with darknet identity',
  inputSchema: {
    type: 'object',
    properties: {
      username: {
        type: 'string',
        description: 'Darknet username (e.g., ruv.dark)'
      },
      email: {
        type: 'string',
        description: 'Email address (optional for darknet users)'
      },
      initialCredits: {
        type: 'number',
        default: 0,
        description: 'Initial rUv credits allocation'
      },
      isGenesis: {
        type: 'boolean',
        default: false,
        description: 'Is this a genesis user?'
      }
    },
    required: ['username']
  },
  async execute(params: any) {
    try {
      // Generate darknet address for user
      const address = await DarknetAddressGenerator.generate(
        NetworkType.DARKNET,
        [
          Capability.AGENT_SPAWN,
          Capability.TASK_ORCHESTRATE,
          Capability.MEMORY_SHARE,
          Capability.SWARM_COORDINATE,
          Capability.CONSENSUS_PARTICIPATE,
          Capability.DARKNET_RELAY,
          Capability.QUANTUM_RESISTANT
        ]
      );
      
      // Generate secure password
      const password = randomBytes(32).toString('hex');
      const hashedPassword = await bcrypt.hash(password, 12);
      
      // Create user in Supabase Auth
      const { data: authData, error: authError } = await supabase.auth.admin.createUser({
        email: params.email || `${params.username}@darknet.local`,
        password: password,
        email_confirm: true,
        user_metadata: {
          username: params.username,
          darknet_address: address.onionAddress,
          is_genesis: params.isGenesis || false
        }
      });
      
      if (authError) {
        throw new Error(`Auth creation failed: ${authError.message}`);
      }
      
      // Create app store profile
      const { error: profileError } = await supabase
        .from('app_store_profiles')
        .insert({
          id: authData.user.id,
          username: params.username,
          display_name: params.username,
          ruv_credits: params.initialCredits || 0,
          darknet_address: address.onionAddress,
          darknet_public_key: address.publicKey,
          is_genesis_user: params.isGenesis || false
        });
      
      if (profileError) {
        throw new Error(`Profile creation failed: ${profileError.message}`);
      }
      
      // Store darknet identity
      const { error: darknetError } = await supabase
        .from('darknet_identities')
        .insert({
          user_id: authData.user.id,
          username: params.username,
          public_key: address.publicKey,
          onion_address: address.onionAddress,
          multiaddr: address.multiaddr,
          network_id: address.networkId,
          capabilities: address.capabilities,
          checksum: address.checksum,
          password_hash: hashedPassword
        });
      
      if (darknetError) {
        throw new Error(`Darknet identity creation failed: ${darknetError.message}`);
      }
      
      // If genesis user, allocate initial credits
      if (params.isGenesis && params.initialCredits > 0) {
        const { error: txError } = await supabase
          .from('ruv_transactions')
          .insert({
            user_id: authData.user.id,
            transaction_type: 'genesis_allocation',
            amount: params.initialCredits,
            balance_after: params.initialCredits,
            description: `Genesis allocation for ${params.username}`,
            metadata: {
              genesis: true,
              darknet_address: address.onionAddress
            }
          });
        
        if (txError) {
          console.error('Failed to record genesis allocation:', txError);
        }
      }
      
      return {
        success: true,
        user: {
          id: authData.user.id,
          username: params.username,
          darknetAddress: address.onionAddress,
          publicKey: address.publicKey,
          multiaddr: address.multiaddr,
          password: password, // Return only on initial creation
          initialCredits: params.initialCredits,
          isGenesis: params.isGenesis
        },
        message: `Darknet user ${params.username} registered successfully`
      };
    } catch (error) {
      return {
        success: false,
        error: `Registration failed: ${error}`
      };
    }
  }
};

/**
 * Connect to darknet peer
 */
export const darknetConnect: Tool = {
  name: 'darknet_connect',
  description: 'Connect to a darknet peer',
  inputSchema: {
    type: 'object',
    properties: {
      peerAddress: {
        type: 'string',
        description: 'Peer onion address or multiaddr'
      },
      protocol: {
        type: 'string',
        enum: ['tcp', 'quic', 'websocket'],
        default: 'tcp'
      }
    },
    required: ['peerAddress']
  },
  async execute(params: any) {
    try {
      // Store peer connection
      const { error } = await supabase
        .from('darknet_peers')
        .insert({
          peer_address: params.peerAddress,
          protocol: params.protocol || 'tcp',
          status: 'connected',
          last_seen: new Date().toISOString()
        });
      
      if (error) {
        console.error('Failed to store peer connection:', error);
      }
      
      return {
        success: true,
        peer: params.peerAddress,
        protocol: params.protocol,
        status: 'connected'
      };
    } catch (error) {
      return {
        success: false,
        error: `Connection failed: ${error}`
      };
    }
  }
};

/**
 * Spawn agent with darknet identity
 */
export const darknetSpawn: Tool = {
  name: 'darknet_spawn',
  description: 'Spawn an agent with darknet identity',
  inputSchema: {
    type: 'object',
    properties: {
      agentType: {
        type: 'string',
        description: 'Type of agent to spawn'
      },
      darknetEnabled: {
        type: 'boolean',
        default: true,
        description: 'Enable darknet communication'
      },
      privateMode: {
        type: 'boolean',
        default: true,
        description: 'Run in private mode'
      }
    },
    required: ['agentType']
  },
  async execute(params: any) {
    try {
      // Generate agent darknet address
      const address = await DarknetAddressGenerator.generate(
        NetworkType.DARKNET,
        [Capability.AGENT_SPAWN, Capability.TASK_ORCHESTRATE]
      );
      
      // Store agent identity
      const { data, error } = await supabase
        .from('darknet_agents')
        .insert({
          agent_type: params.agentType,
          darknet_address: address.onionAddress,
          public_key: address.publicKey,
          multiaddr: address.multiaddr,
          private_mode: params.privateMode,
          status: 'active'
        })
        .select()
        .single();
      
      if (error) {
        throw new Error(`Failed to spawn agent: ${error.message}`);
      }
      
      return {
        success: true,
        agent: {
          id: data.id,
          type: params.agentType,
          darknetAddress: address.onionAddress,
          publicKey: address.publicKey,
          privateMode: params.privateMode
        },
        message: `Agent ${params.agentType} spawned with darknet identity`
      };
    } catch (error) {
      return {
        success: false,
        error: `Agent spawn failed: ${error}`
      };
    }
  }
};

/**
 * List darknet addresses
 */
export const darknetList: Tool = {
  name: 'darknet_list',
  description: 'List stored darknet addresses',
  inputSchema: {
    type: 'object',
    properties: {}
  },
  async execute() {
    try {
      const addresses = addressManager.list();
      
      return {
        success: true,
        addresses: addresses.map(({ alias, address }) => ({
          alias,
          onionAddress: address.onionAddress,
          networkId: address.networkId,
          capabilities: address.capabilities
        }))
      };
    } catch (error) {
      return {
        success: false,
        error: `Failed to list addresses: ${error}`
      };
    }
  }
};

// Export all darknet tools
export const darknetTools = [
  darknetInit,
  darknetAddress,
  darknetRegister,
  darknetConnect,
  darknetSpawn,
  darknetList
];