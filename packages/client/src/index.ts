/**
 * @llm-dev-ops/incident-manager-client
 *
 * TypeScript/JavaScript client SDK for LLM Incident Manager
 * WebSocket streaming and GraphQL API client
 *
 * @version 1.0.1
 * @license MIT OR Apache-2.0
 */

import { createClient, Client } from 'graphql-ws';
import type {
  Incident,
  Severity,
  IncidentStatus,
  RawEvent
} from '@llm-dev-ops/incident-manager-types';

/**
 * Client configuration options
 */
export interface ClientOptions {
  /** WebSocket URL (e.g., 'ws://localhost:8080/graphql/ws') */
  wsUrl: string;

  /** JWT authentication token */
  authToken: string;

  /** Maximum number of retry attempts (default: 5) */
  retryAttempts?: number;

  /** Custom retry wait function */
  retryWait?: (retries: number) => Promise<void>;

  /** WebSocket implementation (required for Node.js) */
  webSocketImpl?: any;

  /** Connected callback */
  onConnected?: () => void;

  /** Disconnected callback */
  onDisconnected?: () => void;

  /** Error callback */
  onError?: (error: any) => void;
}

/**
 * Incident update event
 */
export interface IncidentUpdate {
  updateType: string;
  incidentId: string;
  timestamp: string;
  changes?: Record<string, any>;
}

/**
 * State change event
 */
export interface StateChange {
  incidentId: string;
  oldState: IncidentStatus;
  newState: IncidentStatus;
  changedBy: string;
  timestamp: string;
}

/**
 * Alert event
 */
export interface Alert extends RawEvent {
  acknowledged: boolean;
}

/**
 * LLM Incident Manager Client
 *
 * Provides real-time WebSocket streaming and GraphQL API access
 * to the LLM Incident Manager system.
 */
export class IncidentManagerClient {
  private client: Client;
  private subscriptions: Map<string, () => void> = new Map();

  constructor(options: ClientOptions) {
    const {
      wsUrl,
      authToken,
      retryAttempts = 5,
      retryWait,
      webSocketImpl,
      onConnected,
      onDisconnected,
      onError
    } = options;

    this.client = createClient({
      url: wsUrl,
      webSocketImpl,
      connectionParams: {
        Authorization: `Bearer ${authToken}`
      },
      retryAttempts,
      retryWait: retryWait || ((retries) => {
        const delay = Math.min(1000 * 2 ** retries, 30000);
        return new Promise(resolve => setTimeout(resolve, delay));
      }),
      on: {
        connected: onConnected,
        closed: onDisconnected,
        error: onError
      }
    });
  }

  /**
   * Subscribe to critical incidents (P0 and P1)
   */
  public subscribeToCriticalIncidents(handler: (incident: Incident) => void): void {
    const query = `
      subscription {
        criticalIncidents {
          id
          external_id
          fingerprint
          severity
          status
          category
          title
          description
          impact
          source
          source_event_id
          assigned_to
          assigned_team
          escalation_level
          created_at
          updated_at
          acknowledged_at
          resolved_at
          closed_at
          sla {
            acknowledgment_deadline
            resolution_deadline
            acknowledgment_breached
            resolution_breached
          }
          related_incidents
          parent_incident
          duplicate_of
          metrics {
            mttd
            mtta
            mttr
          }
          resource {
            type
            id
            name
            metadata
          }
          environment
          tags
          labels
          resolution {
            root_cause
            resolution_notes
            resolved_by
            playbook_used
            actions_taken {
              id
              type
              description
              executed_by
              executed_at
              result
              metadata
            }
          }
          metadata
        }
      }
    `;

    const unsubscribe = this.client.subscribe<{ criticalIncidents: Incident }>(
      { query },
      {
        next: (data) => {
          if (data.data?.criticalIncidents) {
            handler(data.data.criticalIncidents);
          }
        },
        error: (error) => {
          console.error('Critical incidents subscription error:', error);
        },
        complete: () => {
          console.log('Critical incidents subscription completed');
        }
      }
    );

    this.subscriptions.set('critical-incidents', unsubscribe);
  }

  /**
   * Subscribe to incident updates with custom severity filters
   */
  public subscribeToIncidentUpdates(
    severities: Severity[],
    handler: (update: IncidentUpdate) => void
  ): void {
    const query = `
      subscription IncidentUpdates($severities: [Severity!]) {
        incidentUpdates(severities: $severities, activeOnly: true) {
          updateType
          incidentId
          timestamp
          changes
        }
      }
    `;

    const unsubscribe = this.client.subscribe<{ incidentUpdates: IncidentUpdate }>(
      {
        query,
        variables: { severities }
      },
      {
        next: (data) => {
          if (data.data?.incidentUpdates) {
            handler(data.data.incidentUpdates);
          }
        },
        error: (error) => {
          console.error('Incident updates subscription error:', error);
        },
        complete: () => {
          console.log('Incident updates subscription completed');
        }
      }
    );

    this.subscriptions.set('incident-updates', unsubscribe);
  }

  /**
   * Subscribe to new incidents
   */
  public subscribeToNewIncidents(handler: (incident: Incident) => void): void {
    const query = `
      subscription {
        newIncidents {
          id
          title
          description
          severity
          status
          category
          created_at
          environment
          resource {
            type
            name
          }
          tags
        }
      }
    `;

    const unsubscribe = this.client.subscribe<{ newIncidents: Incident }>(
      { query },
      {
        next: (data) => {
          if (data.data?.newIncidents) {
            handler(data.data.newIncidents);
          }
        },
        error: (error) => {
          console.error('New incidents subscription error:', error);
        },
        complete: () => {
          console.log('New incidents subscription completed');
        }
      }
    );

    this.subscriptions.set('new-incidents', unsubscribe);
  }

  /**
   * Subscribe to incident state changes
   */
  public subscribeToStateChanges(handler: (stateChange: StateChange) => void): void {
    const query = `
      subscription {
        incidentStateChanges {
          incidentId
          oldState
          newState
          changedBy
          timestamp
        }
      }
    `;

    const unsubscribe = this.client.subscribe<{ incidentStateChanges: StateChange }>(
      { query },
      {
        next: (data) => {
          if (data.data?.incidentStateChanges) {
            handler(data.data.incidentStateChanges);
          }
        },
        error: (error) => {
          console.error('State changes subscription error:', error);
        },
        complete: () => {
          console.log('State changes subscription completed');
        }
      }
    );

    this.subscriptions.set('state-changes', unsubscribe);
  }

  /**
   * Subscribe to all incoming alerts
   */
  public subscribeToAlerts(handler: (alert: Alert) => void): void {
    const query = `
      subscription {
        alerts {
          event_id
          source
          source_version
          timestamp
          received_at
          event_type
          category
          title
          description
          severity
          resource {
            type
            id
            name
            metadata
          }
          metrics
          tags
          labels
          correlation_id
          parent_event_id
          payload
          acknowledged
        }
      }
    `;

    const unsubscribe = this.client.subscribe<{ alerts: Alert }>(
      { query },
      {
        next: (data) => {
          if (data.data?.alerts) {
            handler(data.data.alerts);
          }
        },
        error: (error) => {
          console.error('Alerts subscription error:', error);
        },
        complete: () => {
          console.log('Alerts subscription completed');
        }
      }
    );

    this.subscriptions.set('alerts', unsubscribe);
  }

  /**
   * Execute a custom GraphQL subscription
   */
  public subscribe(
    query: string,
    onData: (data: any) => void,
    onError?: (error: any) => void
  ): () => void {
    return this.client.subscribe(
      { query },
      {
        next: (data) => onData(data.data),
        error: onError || ((error) => console.error('Subscription error:', error)),
        complete: () => console.log('Subscription completed')
      }
    );
  }

  /**
   * Unsubscribe from a specific subscription
   */
  public unsubscribe(subscriptionId: string): void {
    const unsubscribe = this.subscriptions.get(subscriptionId);
    if (unsubscribe) {
      unsubscribe();
      this.subscriptions.delete(subscriptionId);
    }
  }

  /**
   * Unsubscribe from all active subscriptions
   */
  public unsubscribeAll(): void {
    this.subscriptions.forEach((unsubscribe) => unsubscribe());
    this.subscriptions.clear();
  }

  /**
   * Close the WebSocket connection and cleanup all subscriptions
   */
  public close(): void {
    this.unsubscribeAll();
    this.client.dispose();
  }
}

// Re-export types for convenience
export type {
  Incident,
  Severity,
  IncidentStatus,
  RawEvent
} from '@llm-dev-ops/incident-manager-types';
