/**
 * TypeScript/JavaScript WebSocket Client Example
 *
 * This example demonstrates how to connect to the LLM Incident Manager
 * WebSocket API and subscribe to real-time incident updates.
 *
 * Requirements:
 *   npm install graphql-ws
 */

import { createClient, Client } from 'graphql-ws';
import WebSocket from 'ws'; // Only needed in Node.js

interface Incident {
  id: string;
  title: string;
  description: string;
  severity: 'P0' | 'P1' | 'P2' | 'P3' | 'P4';
  state: string;
  createdAt: string;
  affectedResources: string[];
}

interface IncidentUpdate {
  updateType: string;
  incidentId: string;
  timestamp: string;
}

class IncidentMonitor {
  private client: Client;
  private subscriptions: Map<string, () => void> = new Map();

  constructor(wsUrl: string, authToken: string) {
    this.client = createClient({
      url: wsUrl,
      webSocketImpl: WebSocket, // Omit this line in browser environment
      connectionParams: {
        Authorization: `Bearer ${authToken}`
      },
      retryAttempts: 5,
      retryWait: (retries) => {
        // Exponential backoff: 1s, 2s, 4s, 8s, 16s
        const delay = Math.min(1000 * 2 ** retries, 30000);
        console.log(`Reconnecting in ${delay}ms (attempt ${retries + 1})`);
        return new Promise(resolve => setTimeout(resolve, delay));
      },
      on: {
        connected: () => console.log('✅ Connected to incident stream'),
        closed: () => console.log('❌ Disconnected from incident stream'),
        error: (error) => console.error('Connection error:', error)
      }
    });
  }

  /**
   * Subscribe to critical incidents (P0 and P1)
   */
  subscribeToCriticalIncidents(handler: (incident: Incident) => void): void {
    const query = `
      subscription {
        criticalIncidents {
          id
          title
          description
          severity
          state
          createdAt
          affectedResources
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
   * Subscribe to incident updates with custom filters
   */
  subscribeToIncidentUpdates(
    severities: string[],
    handler: (update: IncidentUpdate) => void
  ): void {
    const query = `
      subscription IncidentUpdates($severities: [Severity!]) {
        incidentUpdates(severities: $severities, activeOnly: true) {
          updateType
          incidentId
          timestamp
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
  subscribeToNewIncidents(
    severities: string[],
    handler: (incident: Incident) => void
  ): void {
    const query = `
      subscription NewIncidents($severities: [Severity!]) {
        newIncidents(severities: $severities) {
          id
          title
          description
          severity
          state
          createdAt
          affectedResources
        }
      }
    `;

    const unsubscribe = this.client.subscribe<{ newIncidents: Incident }>(
      {
        query,
        variables: { severities }
      },
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
   * Unsubscribe from a specific subscription
   */
  unsubscribe(subscriptionName: string): void {
    const unsubscribe = this.subscriptions.get(subscriptionName);
    if (unsubscribe) {
      unsubscribe();
      this.subscriptions.delete(subscriptionName);
      console.log(`Unsubscribed from ${subscriptionName}`);
    }
  }

  /**
   * Disconnect and cleanup all subscriptions
   */
  disconnect(): void {
    this.subscriptions.forEach((unsubscribe) => unsubscribe());
    this.subscriptions.clear();
    this.client.dispose();
    console.log('Disconnected and cleaned up all subscriptions');
  }
}

// Example usage
async function main() {
  const wsUrl = process.env.WS_URL || 'ws://localhost:8080/graphql/ws';
  const authToken = process.env.AUTH_TOKEN || 'YOUR_JWT_TOKEN';

  const monitor = new IncidentMonitor(wsUrl, authToken);

  // Subscribe to critical incidents
  monitor.subscribeToCriticalIncidents((incident) => {
    console.log('🚨 CRITICAL INCIDENT:', {
      id: incident.id,
      title: incident.title,
      severity: incident.severity,
      affected: incident.affectedResources
    });

    // Send to PagerDuty, Slack, etc.
    sendPagerDutyAlert(incident);
  });

  // Subscribe to all P0 and P1 updates
  monitor.subscribeToIncidentUpdates(['P0', 'P1'], (update) => {
    console.log('📢 Incident Update:', {
      type: update.updateType,
      incidentId: update.incidentId,
      timestamp: update.timestamp
    });

    // Update dashboard, send notifications, etc.
    updateDashboard(update);
  });

  // Subscribe to new incidents
  monitor.subscribeToNewIncidents(['P0', 'P1', 'P2'], (incident) => {
    console.log('🆕 New Incident:', {
      id: incident.id,
      title: incident.title,
      severity: incident.severity
    });
  });

  // Graceful shutdown
  process.on('SIGINT', () => {
    console.log('\nShutting down gracefully...');
    monitor.disconnect();
    process.exit(0);
  });

  console.log('Monitoring incidents... Press Ctrl+C to stop.');
}

// Mock notification functions (implement these based on your needs)
function sendPagerDutyAlert(incident: Incident): void {
  // Implementation for PagerDuty integration
  console.log('Would send to PagerDuty:', incident.id);
}

function updateDashboard(update: IncidentUpdate): void {
  // Implementation for dashboard update
  console.log('Would update dashboard:', update.incidentId);
}

// Run the monitor
if (require.main === module) {
  main().catch(console.error);
}

export { IncidentMonitor, Incident, IncidentUpdate };
