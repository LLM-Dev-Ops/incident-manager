import { Tool } from '../types/mcp.js'
import { AccountingService } from '../services/accounting-service.js'

export class AccountingTools {
  private accountingService: AccountingService

  constructor(supabaseUrl: string, supabaseKey: string) {
    this.accountingService = new AccountingService(supabaseUrl, supabaseKey)
  }

  getTools(): Tool[] {
    return [
      {
        name: 'process_transaction',
        description: 'Process a new financial transaction with automatic double-entry bookkeeping',
        inputSchema: {
          type: 'object',
          properties: {
            user_id: {
              type: 'string',
              description: 'User ID for the transaction'
            },
            category: {
              type: 'string',
              enum: ['tool_usage', 'challenge_reward', 'subscription', 'purchase', 'refund', 'bonus', 'penalty', 'transfer', 'adjustment'],
              description: 'Transaction category'
            },
            amount: {
              type: 'number',
              description: 'Transaction amount in credits'
            },
            description: {
              type: 'string',
              description: 'Transaction description'
            },
            reference_id: {
              type: 'string',
              description: 'External reference ID (optional)'
            },
            metadata: {
              type: 'object',
              description: 'Additional transaction metadata (optional)'
            }
          },
          required: ['user_id', 'category', 'amount', 'description']
        }
      },
      {
        name: 'get_user_balance',
        description: 'Get user credit balance with optional pending transactions',
        inputSchema: {
          type: 'object',
          properties: {
            user_id: {
              type: 'string',
              description: 'User ID to get balance for'
            },
            include_pending: {
              type: 'boolean',
              description: 'Include pending transactions in balance calculation',
              default: false
            }
          },
          required: ['user_id']
        }
      },
      {
        name: 'get_account_balances',
        description: 'Get all account balances or specific account balance',
        inputSchema: {
          type: 'object',
          properties: {
            account_code: {
              type: 'string',
              description: 'Specific account code to get balance for (optional)'
            }
          }
        }
      },
      {
        name: 'generate_report',
        description: 'Generate various financial and usage reports',
        inputSchema: {
          type: 'object',
          properties: {
            report_type: {
              type: 'string',
              enum: [
                'daily_summary',
                'tool_usage',
                'user_activity',
                'revenue_analysis',
                'credit_flow',
                'reconciliation',
                'top_users',
                'performance_metrics'
              ],
              description: 'Type of report to generate'
            },
            start_date: {
              type: 'string',
              description: 'Start date for report (YYYY-MM-DD format)'
            },
            end_date: {
              type: 'string',
              description: 'End date for report (YYYY-MM-DD format)'
            },
            user_id: {
              type: 'string',
              description: 'Filter by specific user ID'
            },
            tool_category: {
              type: 'string',
              description: 'Filter by tool category'
            },
            user_tier: {
              type: 'string',
              description: 'Filter by user tier'
            },
            limit: {
              type: 'number',
              description: 'Limit number of results',
              default: 100
            },
            format: {
              type: 'string',
              enum: ['json', 'csv'],
              description: 'Report output format',
              default: 'json'
            }
          },
          required: ['report_type']
        }
      },
      {
        name: 'get_transaction_history',
        description: 'Get transaction history for a user with filtering options',
        inputSchema: {
          type: 'object',
          properties: {
            user_id: {
              type: 'string',
              description: 'User ID to get transaction history for'
            },
            limit: {
              type: 'number',
              description: 'Number of transactions to return',
              default: 50
            },
            offset: {
              type: 'number',
              description: 'Number of transactions to skip',
              default: 0
            },
            category: {
              type: 'string',
              description: 'Filter by transaction category'
            },
            status: {
              type: 'string',
              enum: ['pending', 'completed', 'failed', 'cancelled', 'refunded'],
              description: 'Filter by transaction status'
            },
            start_date: {
              type: 'string',
              description: 'Start date filter (YYYY-MM-DD format)'
            },
            end_date: {
              type: 'string',
              description: 'End date filter (YYYY-MM-DD format)'
            }
          },
          required: ['user_id']
        }
      },
      {
        name: 'process_tool_usage',
        description: 'Process a tool usage transaction with metadata',
        inputSchema: {
          type: 'object',
          properties: {
            user_id: {
              type: 'string',
              description: 'User ID using the tool'
            },
            tool_name: {
              type: 'string',
              description: 'Name of the tool being used'
            },
            credits_used: {
              type: 'number',
              description: 'Number of credits consumed'
            },
            metadata: {
              type: 'object',
              properties: {
                tool_category: {
                  type: 'string',
                  description: 'Category of the tool'
                },
                user_tier: {
                  type: 'string',
                  description: 'User tier (free, pro, enterprise)'
                },
                execution_time: {
                  type: 'number',
                  description: 'Tool execution time in milliseconds'
                },
                execution_id: {
                  type: 'string',
                  description: 'Unique execution identifier'
                }
              }
            }
          },
          required: ['user_id', 'tool_name', 'credits_used']
        }
      },
      {
        name: 'process_challenge_reward',
        description: 'Process a challenge completion reward',
        inputSchema: {
          type: 'object',
          properties: {
            user_id: {
              type: 'string',
              description: 'User ID completing the challenge'
            },
            challenge_id: {
              type: 'string',
              description: 'Challenge identifier'
            },
            reward_amount: {
              type: 'number',
              description: 'Reward amount in credits'
            },
            challenge_name: {
              type: 'string',
              description: 'Name of the challenge'
            },
            metadata: {
              type: 'object',
              properties: {
                difficulty: {
                  type: 'string',
                  description: 'Challenge difficulty level'
                },
                completion_time: {
                  type: 'number',
                  description: 'Time taken to complete in milliseconds'
                }
              }
            }
          },
          required: ['user_id', 'challenge_id', 'reward_amount', 'challenge_name']
        }
      },
      {
        name: 'process_credit_purchase',
        description: 'Process a credit purchase transaction',
        inputSchema: {
          type: 'object',
          properties: {
            user_id: {
              type: 'string',
              description: 'User ID making the purchase'
            },
            amount: {
              type: 'number',
              description: 'Number of credits purchased'
            },
            payment_method: {
              type: 'string',
              description: 'Payment method used'
            },
            metadata: {
              type: 'object',
              properties: {
                payment_id: {
                  type: 'string',
                  description: 'Payment processor transaction ID'
                },
                currency: {
                  type: 'string',
                  description: 'Currency used for payment'
                },
                exchange_rate: {
                  type: 'number',
                  description: 'Exchange rate if not USD'
                }
              }
            }
          },
          required: ['user_id', 'amount', 'payment_method']
        }
      },
      {
        name: 'subscribe_to_updates',
        description: 'Subscribe to real-time accounting updates',
        inputSchema: {
          type: 'object',
          properties: {
            channel: {
              type: 'string',
              enum: ['accounting_updates', 'transaction_feed', 'balance_changes', 'user_activity', 'system_alerts'],
              description: 'Channel to subscribe to'
            },
            event: {
              type: 'string',
              enum: ['transaction_created', 'transaction_completed', 'balance_updated', 'daily_summary_ready'],
              description: 'Event type to listen for'
            },
            user_id: {
              type: 'string',
              description: 'Filter updates for specific user (optional)'
            }
          },
          required: ['channel', 'event']
        }
      },
      {
        name: 'broadcast_update',
        description: 'Broadcast a message to a real-time channel',
        inputSchema: {
          type: 'object',
          properties: {
            channel: {
              type: 'string',
              description: 'Channel to broadcast to'
            },
            event: {
              type: 'string',
              description: 'Event type'
            },
            payload: {
              type: 'object',
              description: 'Message payload to broadcast'
            }
          },
          required: ['channel', 'event', 'payload']
        }
      },
      {
        name: 'refresh_views',
        description: 'Refresh materialized views for updated reporting data',
        inputSchema: {
          type: 'object',
          properties: {}
        }
      },
      {
        name: 'check_integrity',
        description: 'Check accounting system integrity and balance validation',
        inputSchema: {
          type: 'object',
          properties: {}
        }
      }
    ]
  }

  async handleToolCall(name: string, args: any): Promise<any> {
    try {
      switch (name) {
        case 'process_transaction':
          return await this.accountingService.processTransaction(args)

        case 'get_user_balance':
          return await this.accountingService.getUserBalance(args.user_id, args.include_pending)

        case 'get_account_balances':
          return await this.accountingService.getAccountBalances(args.account_code)

        case 'generate_report':
          return await this.accountingService.generateReport(args.report_type, {
            startDate: args.start_date,
            endDate: args.end_date,
            userId: args.user_id,
            toolCategory: args.tool_category,
            userTier: args.user_tier,
            limit: args.limit,
            format: args.format
          })

        case 'get_transaction_history':
          return await this.accountingService.getTransactionHistory(args.user_id, {
            limit: args.limit,
            offset: args.offset,
            category: args.category,
            status: args.status,
            startDate: args.start_date,
            endDate: args.end_date
          })

        case 'process_tool_usage':
          return await this.accountingService.processToolUsage(
            args.user_id,
            args.tool_name,
            args.credits_used,
            args.metadata || {}
          )

        case 'process_challenge_reward':
          return await this.accountingService.processChallengeReward(
            args.user_id,
            args.challenge_id,
            args.reward_amount,
            args.challenge_name,
            args.metadata || {}
          )

        case 'process_credit_purchase':
          return await this.accountingService.processCreditPurchase(
            args.user_id,
            args.amount,
            args.payment_method,
            args.metadata || {}
          )

        case 'subscribe_to_updates':
          // Return subscription instructions since this needs to be handled client-side
          return {
            subscription_setup: 'Use the AccountingService.subscribeToUpdates() method in your client application',
            channel: args.channel,
            event: args.event,
            user_filter: args.user_id,
            websocket_url: `${process.env.SUPABASE_URL?.replace('https://', 'wss://')}/realtime/v1/websocket`,
            example_code: `
// JavaScript example
const unsubscribe = accountingService.subscribeToUpdates('${args.channel}', '${args.event}', (payload) => {
  console.log('Received update:', payload)
}, { userId: '${args.user_id || 'USER_ID'}' })

// Call unsubscribe() when done
            `
          }

        case 'broadcast_update':
          return await this.accountingService.broadcastUpdate(args.channel, args.event, args.payload)

        case 'refresh_views':
          const refreshed = await this.accountingService.refreshViews()
          return { success: refreshed, message: refreshed ? 'Views refreshed successfully' : 'Failed to refresh views' }

        case 'check_integrity':
          return await this.accountingService.checkIntegrity()

        default:
          throw new Error(`Unknown tool: ${name}`)
      }
    } catch (error) {
      console.error(`Error in accounting tool ${name}:`, error)
      return {
        error: error.message,
        tool: name,
        timestamp: new Date().toISOString()
      }
    }
  }
}

export default AccountingTools