import { createClient } from '@supabase/supabase-js'

export interface Transaction {
  id?: string
  user_id: string
  category: string
  amount: number
  description: string
  reference_id?: string
  metadata?: Record<string, any>
  status?: 'pending' | 'completed' | 'failed' | 'cancelled' | 'refunded'
}

export interface AccountBalance {
  account_code: string
  account_name: string
  account_type: string
  balance: number
  last_updated: string
}

export interface UserBalance {
  user_id: string
  credit_balance: number
  pending_balance: number
  available_balance: number
  last_transaction_date: string
}

export class AccountingService {
  private supabase
  private realtimeChannel: any = null

  constructor(supabaseUrl: string, supabaseKey: string) {
    this.supabase = createClient(supabaseUrl, supabaseKey)
  }

  /**
   * Process a new transaction with automatic ledger entries
   */
  async processTransaction(transaction: Transaction): Promise<{ success: boolean; transaction_id?: string; error?: string }> {
    try {
      const response = await fetch(`${this.supabase.supabaseUrl}/functions/v1/transaction-processor`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.supabase.supabaseKey}`
        },
        body: JSON.stringify(transaction)
      })

      const result = await response.json()

      if (!response.ok) {
        return { success: false, error: result.error }
      }

      return {
        success: true,
        transaction_id: result.transaction_id
      }
    } catch (error) {
      return { success: false, error: error.message }
    }
  }

  /**
   * Get user credit balance
   */
  async getUserBalance(userId: string, includePending: boolean = false): Promise<UserBalance | null> {
    try {
      const response = await fetch(
        `${this.supabase.supabaseUrl}/functions/v1/balance-calculator?operation=user_balance&user_id=${userId}&include_pending=${includePending}`,
        {
          headers: {
            'Authorization': `Bearer ${this.supabase.supabaseKey}`
          }
        }
      )

      const result = await response.json()

      if (!response.ok) {
        throw new Error(result.error)
      }

      return result.user_balance
    } catch (error) {
      console.error('Error fetching user balance:', error)
      return null
    }
  }

  /**
   * Get account balances
   */
  async getAccountBalances(accountCode?: string): Promise<AccountBalance[]> {
    try {
      const url = accountCode 
        ? `${this.supabase.supabaseUrl}/functions/v1/balance-calculator?operation=account_balances&account_code=${accountCode}`
        : `${this.supabase.supabaseUrl}/functions/v1/balance-calculator?operation=account_balances`

      const response = await fetch(url, {
        headers: {
          'Authorization': `Bearer ${this.supabase.supabaseKey}`
        }
      })

      const result = await response.json()

      if (!response.ok) {
        throw new Error(result.error)
      }

      return result.account_balances || []
    } catch (error) {
      console.error('Error fetching account balances:', error)
      return []
    }
  }

  /**
   * Generate reports
   */
  async generateReport(
    reportType: string, 
    options: {
      startDate?: string
      endDate?: string
      userId?: string
      toolCategory?: string
      userTier?: string
      limit?: number
      format?: 'json' | 'csv'
    } = {}
  ): Promise<any> {
    try {
      const params = new URLSearchParams({
        report_type: reportType,
        ...(options.startDate && { start_date: options.startDate }),
        ...(options.endDate && { end_date: options.endDate }),
        ...(options.userId && { user_id: options.userId }),
        ...(options.toolCategory && { tool_category: options.toolCategory }),
        ...(options.userTier && { user_tier: options.userTier }),
        ...(options.limit && { limit: options.limit.toString() }),
        ...(options.format && { format: options.format })
      })

      const response = await fetch(
        `${this.supabase.supabaseUrl}/functions/v1/reporting-analytics?${params}`,
        {
          headers: {
            'Authorization': `Bearer ${this.supabase.supabaseKey}`
          }
        }
      )

      if (options.format === 'csv') {
        return await response.text()
      }

      const result = await response.json()

      if (!response.ok) {
        throw new Error(result.error)
      }

      return result
    } catch (error) {
      console.error('Error generating report:', error)
      throw error
    }
  }

  /**
   * Subscribe to real-time updates
   */
  subscribeToUpdates(
    channel: string,
    event: string,
    callback: (payload: any) => void,
    options: {
      table?: string
      filter?: string
      userId?: string
    } = {}
  ): () => void {
    // Unsubscribe from previous channel if exists
    if (this.realtimeChannel) {
      this.supabase.removeChannel(this.realtimeChannel)
    }

    // Create new subscription
    this.realtimeChannel = this.supabase
      .channel(channel)
      .on('broadcast', { event }, callback)
      .subscribe()

    // Register subscription with coordinator
    fetch(`${this.supabase.supabaseUrl}/functions/v1/realtime-coordinator?action=subscribe`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.supabase.supabaseKey}`
      },
      body: JSON.stringify({
        channel,
        event,
        ...options
      })
    }).catch(error => {
      console.error('Error registering subscription:', error)
    })

    // Return unsubscribe function
    return () => {
      if (this.realtimeChannel) {
        this.supabase.removeChannel(this.realtimeChannel)
        this.realtimeChannel = null
      }
    }
  }

  /**
   * Broadcast message to channel
   */
  async broadcastUpdate(channel: string, event: string, payload: any): Promise<boolean> {
    try {
      const response = await fetch(`${this.supabase.supabaseUrl}/functions/v1/realtime-coordinator?action=broadcast`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.supabase.supabaseKey}`
        },
        body: JSON.stringify({
          channel,
          event,
          payload
        })
      })

      const result = await response.json()
      return result.success || false
    } catch (error) {
      console.error('Error broadcasting update:', error)
      return false
    }
  }

  /**
   * Get transaction history
   */
  async getTransactionHistory(
    userId: string,
    options: {
      limit?: number
      offset?: number
      category?: string
      status?: string
      startDate?: string
      endDate?: string
    } = {}
  ): Promise<Transaction[]> {
    try {
      let query = this.supabase
        .from('transactions')
        .select(`
          *,
          ledger_entries!inner(
            debit_amount,
            credit_amount,
            chart_of_accounts(account_code, account_name)
          )
        `)
        .eq('user_id', userId)
        .order('transaction_date', { ascending: false })

      if (options.limit) {
        query = query.limit(options.limit)
      }

      if (options.offset) {
        query = query.range(options.offset, options.offset + (options.limit || 50) - 1)
      }

      if (options.category) {
        query = query.eq('category', options.category)
      }

      if (options.status) {
        query = query.eq('status', options.status)
      }

      if (options.startDate) {
        query = query.gte('transaction_date', options.startDate)
      }

      if (options.endDate) {
        query = query.lte('transaction_date', options.endDate)
      }

      const { data, error } = await query

      if (error) {
        throw error
      }

      return data.map((transaction: any) => ({
        id: transaction.id,
        user_id: transaction.user_id,
        category: transaction.category,
        amount: transaction.ledger_entries.reduce((sum: number, entry: any) => {
          return sum + (entry.debit_amount || 0)
        }, 0),
        description: transaction.description,
        reference_id: transaction.reference_id,
        metadata: transaction.metadata,
        status: transaction.status
      }))
    } catch (error) {
      console.error('Error fetching transaction history:', error)
      return []
    }
  }

  /**
   * Process tool usage transaction
   */
  async processToolUsage(
    userId: string,
    toolName: string,
    creditsUsed: number,
    metadata: Record<string, any> = {}
  ): Promise<{ success: boolean; transaction_id?: string; error?: string }> {
    return this.processTransaction({
      user_id: userId,
      category: 'tool_usage',
      amount: creditsUsed,
      description: `Tool usage: ${toolName}`,
      reference_id: metadata.execution_id || null,
      metadata: {
        tool_name: toolName,
        tool_category: metadata.tool_category || 'general',
        user_tier: metadata.user_tier || 'free',
        execution_time: metadata.execution_time || null,
        ...metadata
      }
    })
  }

  /**
   * Process challenge reward
   */
  async processChallengeReward(
    userId: string,
    challengeId: string,
    rewardAmount: number,
    challengeName: string,
    metadata: Record<string, any> = {}
  ): Promise<{ success: boolean; transaction_id?: string; error?: string }> {
    return this.processTransaction({
      user_id: userId,
      category: 'challenge_reward',
      amount: rewardAmount,
      description: `Challenge reward: ${challengeName}`,
      reference_id: challengeId,
      metadata: {
        challenge_id: challengeId,
        challenge_name: challengeName,
        difficulty: metadata.difficulty || 'unknown',
        completion_time: metadata.completion_time || null,
        ...metadata
      }
    })
  }

  /**
   * Process credit purchase
   */
  async processCreditPurchase(
    userId: string,
    amount: number,
    paymentMethod: string,
    metadata: Record<string, any> = {}
  ): Promise<{ success: boolean; transaction_id?: string; error?: string }> {
    return this.processTransaction({
      user_id: userId,
      category: 'purchase',
      amount: amount,
      description: `Credit purchase - ${amount} credits`,
      reference_id: metadata.payment_id || null,
      metadata: {
        payment_method: paymentMethod,
        payment_id: metadata.payment_id || null,
        currency: metadata.currency || 'USD',
        exchange_rate: metadata.exchange_rate || 1,
        ...metadata
      }
    })
  }

  /**
   * Refresh materialized views for updated reporting
   */
  async refreshViews(): Promise<boolean> {
    try {
      const response = await fetch(
        `${this.supabase.supabaseUrl}/functions/v1/balance-calculator?operation=refresh_views`,
        {
          headers: {
            'Authorization': `Bearer ${this.supabase.supabaseKey}`
          }
        }
      )

      const result = await response.json()
      return response.ok
    } catch (error) {
      console.error('Error refreshing views:', error)
      return false
    }
  }

  /**
   * Check system balance integrity
   */
  async checkIntegrity(): Promise<{
    is_balanced: boolean
    total_debits: number
    total_credits: number
    difference: number
    unbalanced_transactions: any[]
  }> {
    try {
      const response = await fetch(
        `${this.supabase.supabaseUrl}/functions/v1/reporting-analytics?report_type=reconciliation`,
        {
          headers: {
            'Authorization': `Bearer ${this.supabase.supabaseKey}`
          }
        }
      )

      const result = await response.json()

      if (!response.ok) {
        throw new Error(result.error)
      }

      return {
        is_balanced: result.data.trial_balance.is_balanced,
        total_debits: result.data.trial_balance.total_debits,
        total_credits: result.data.trial_balance.total_credits,
        difference: result.data.trial_balance.difference,
        unbalanced_transactions: result.data.unbalanced_transactions
      }
    } catch (error) {
      console.error('Error checking integrity:', error)
      return {
        is_balanced: false,
        total_debits: 0,
        total_credits: 0,
        difference: 0,
        unbalanced_transactions: []
      }
    }
  }
}