import { createClient, SupabaseClient, RealtimeChannel } from '@supabase/supabase-js';
import { EventEmitter } from 'events';

export interface SupabaseConfig {
  url: string;
  serviceKey: string;
  anonKey?: string;
}

export interface RealtimeSubscription {
  table: string;
  event: 'INSERT' | 'UPDATE' | 'DELETE' | '*';
  filter?: string;
  callback: (payload: any) => Promise<void>;
}

export class SupabaseService extends EventEmitter {
  private client: SupabaseClient;
  private channels: Map<string, RealtimeChannel> = new Map();
  private subscriptions: Map<string, RealtimeSubscription> = new Map();

  constructor(config: SupabaseConfig) {
    super();
    
    this.client = createClient(config.url, config.serviceKey, {
      auth: {
        autoRefreshToken: false,
        persistSession: false
      },
      realtime: {
        params: {
          eventsPerSecond: 10
        }
      }
    });
  }

  async connect(): Promise<void> {
    try {
      // Test the connection
      const { data, error } = await this.client
        .from('applications')
        .select('count', { count: 'exact', head: true });

      if (error) {
        throw new Error(`Failed to connect to Supabase: ${error.message}`);
      }

      console.log('Connected to Supabase successfully');
      this.emit('connected');
    } catch (error) {
      console.error('Supabase connection failed:', error);
      this.emit('error', error);
      throw error;
    }
  }

  async subscribe(subscription: RealtimeSubscription): Promise<string> {
    const subscriptionId = `${subscription.table}_${subscription.event}_${Date.now()}`;
    
    const channel = this.client
      .channel(`realtime:${subscriptionId}`)
      .on(
        'postgres_changes',
        {
          event: subscription.event,
          schema: 'public',
          table: subscription.table,
          filter: subscription.filter
        },
        async (payload: any) => {
          try {
            await subscription.callback(payload);
          } catch (error) {
            console.error(`Error in subscription callback for ${subscriptionId}:`, error);
            this.emit('subscriptionError', { subscriptionId, error });
          }
        }
      )
      .subscribe((status) => {
        console.log(`Subscription ${subscriptionId} status:`, status);
        if (status === 'SUBSCRIBED') {
          this.emit('subscribed', subscriptionId);
        } else if (status === 'CHANNEL_ERROR') {
          this.emit('subscriptionError', { subscriptionId, error: 'Channel error' });
        }
      });

    this.channels.set(subscriptionId, channel);
    this.subscriptions.set(subscriptionId, subscription);

    return subscriptionId;
  }

  async unsubscribe(subscriptionId: string): Promise<void> {
    const channel = this.channels.get(subscriptionId);
    if (channel) {
      await this.client.removeChannel(channel);
      this.channels.delete(subscriptionId);
      this.subscriptions.delete(subscriptionId);
      console.log(`Unsubscribed from ${subscriptionId}`);
    }
  }

  async unsubscribeAll(): Promise<void> {
    const unsubscribePromises = Array.from(this.channels.keys()).map(id => this.unsubscribe(id));
    await Promise.all(unsubscribePromises);
    console.log('Unsubscribed from all channels');
  }

  // Application operations
  async getApplication(id: string): Promise<any> {
    const { data, error } = await this.client
      .from('applications')
      .select(`
        *,
        developer:app_store_profiles(username, display_name),
        category:app_categories(name, slug)
      `)
      .eq('id', id)
      .single();

    if (error) {
      throw new Error(`Failed to get application: ${error.message}`);
    }

    return data;
  }

  async updateApplication(id: string, updates: any): Promise<any> {
    const { data, error } = await this.client
      .from('applications')
      .update(updates)
      .eq('id', id)
      .select()
      .single();

    if (error) {
      throw new Error(`Failed to update application: ${error.message}`);
    }

    return data;
  }

  async searchApplications(filters: {
    category?: string;
    featured?: boolean;
    search?: string;
    limit?: number;
    offset?: number;
  }): Promise<any> {
    let query = this.client
      .from('applications')
      .select(`
        *,
        developer:app_store_profiles(username, display_name),
        category:app_categories(name, slug),
        tags:application_tags(tag:app_tags(name, slug))
      `)
      .eq('status', 'approved');

    if (filters.category) {
      query = query.eq('app_categories.slug', filters.category);
    }

    if (filters.featured) {
      query = query.eq('featured', true);
    }

    if (filters.search) {
      query = query.or(`name.ilike.%${filters.search}%,description.ilike.%${filters.search}%`);
    }

    const { data, error } = await query
      .order('created_at', { ascending: false })
      .range(filters.offset || 0, (filters.offset || 0) + (filters.limit || 20) - 1);

    if (error) {
      throw new Error(`Failed to search applications: ${error.message}`);
    }

    return data;
  }

  // User operations
  async getUserProfile(userId: string): Promise<any> {
    const { data, error } = await this.client
      .from('app_store_profiles')
      .select('*')
      .eq('id', userId)
      .single();

    if (error) {
      throw new Error(`Failed to get user profile: ${error.message}`);
    }

    return data;
  }

  async getUserInstalledApps(userId: string): Promise<any[]> {
    const { data, error } = await this.client
      .from('app_installations')
      .select(`
        id,
        installed_at,
        application:applications(id, name, category, icon_url)
      `)
      .eq('user_id', userId)
      .is('uninstalled_at', null);

    if (error) {
      throw new Error(`Failed to get user installed apps: ${error.message}`);
    }

    return data || [];
  }

  // rUv transaction operations
  async createRuvTransaction(transaction: {
    user_id: string;
    amount: number;
    type: 'earn' | 'spend' | 'transfer';
    description: string;
    metadata?: any;
  }): Promise<any> {
    const { data, error } = await this.client
      .from('ruv_transactions')
      .insert(transaction)
      .select()
      .single();

    if (error) {
      throw new Error(`Failed to create rUv transaction: ${error.message}`);
    }

    return data;
  }

  async getUserRuvBalance(userId: string): Promise<number> {
    const { data, error } = await this.client
      .from('ruv_transactions')
      .select('amount')
      .eq('user_id', userId);

    if (error) {
      throw new Error(`Failed to get rUv balance: ${error.message}`);
    }

    const balance = (data || []).reduce((sum, transaction) => sum + transaction.amount, 0);
    return Math.max(0, balance); // Ensure non-negative balance
  }

  // Challenge operations
  async getChallenge(id: string): Promise<any> {
    const { data, error } = await this.client
      .from('challenges')
      .select(`
        *,
        creator:app_store_profiles(username, display_name),
        category:challenge_categories(name, slug)
      `)
      .eq('id', id)
      .single();

    if (error) {
      throw new Error(`Failed to get challenge: ${error.message}`);
    }

    return data;
  }

  async submitChallengeSolution(submission: {
    challenge_id: string;
    user_id: string;
    code: string;
    language: string;
  }): Promise<any> {
    const { data, error } = await this.client
      .from('challenge_submissions')
      .insert(submission)
      .select()
      .single();

    if (error) {
      throw new Error(`Failed to submit challenge solution: ${error.message}`);
    }

    return data;
  }

  // Analytics and metrics
  async getApplicationAnalytics(applicationId: string, timeframe: string = '30d'): Promise<any> {
    // This would query analytics tables
    // For now, return mock data
    return {
      downloads: Math.floor(Math.random() * 10000),
      rating: (Math.random() * 2 + 3).toFixed(1),
      revenue: Math.floor(Math.random() * 5000),
      activeUsers: Math.floor(Math.random() * 1000)
    };
  }

  async getMarketData(): Promise<any> {
    // This would query market analysis tables
    // For now, return mock data
    return {
      totalApplications: Math.floor(Math.random() * 1000 + 500),
      totalUsers: Math.floor(Math.random() * 10000 + 5000),
      totalRuvInCirculation: Math.floor(Math.random() * 1000000 + 500000),
      averageApplicationPrice: (Math.random() * 50 + 10).toFixed(2)
    };
  }

  // Storage operations
  async uploadFile(bucket: string, path: string, file: Buffer, contentType: string): Promise<string> {
    const { data, error } = await this.client.storage
      .from(bucket)
      .upload(path, file, {
        contentType,
        upsert: true
      });

    if (error) {
      throw new Error(`Failed to upload file: ${error.message}`);
    }

    const { data: publicUrl } = this.client.storage
      .from(bucket)
      .getPublicUrl(path);

    return publicUrl.publicUrl;
  }

  async deleteFile(bucket: string, path: string): Promise<void> {
    const { error } = await this.client.storage
      .from(bucket)
      .remove([path]);

    if (error) {
      throw new Error(`Failed to delete file: ${error.message}`);
    }
  }

  // Health check
  async healthCheck(): Promise<{ status: string; timestamp: string }> {
    try {
      const { error } = await this.client
        .from('applications')
        .select('id')
        .limit(1);

      if (error) {
        throw error;
      }

      return {
        status: 'healthy',
        timestamp: new Date().toISOString()
      };
    } catch (error) {
      return {
        status: 'unhealthy',
        timestamp: new Date().toISOString()
      };
    }
  }

  // Get the underlying Supabase client for advanced operations
  getClient(): SupabaseClient {
    return this.client;
  }

  // Cleanup
  async cleanup(): Promise<void> {
    await this.unsubscribeAll();
    console.log('Supabase service cleanup completed');
  }
}