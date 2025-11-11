# LLM-Incident-Manager Architecture Patterns

## Quick Reference for Implementation

This document provides code examples and architectural patterns for implementing the LLM-Incident-Manager system.

---

## 1. Data Models

### Incident Schema

```typescript
interface Incident {
  // Core identification
  id: string;                          // Unique incident ID (e.g., "inc_abc123")
  fingerprint: string;                 // Deduplication fingerprint (SHA-256)
  dedup_key?: string;                  // Optional explicit dedup key

  // Classification
  title: string;                       // Human-readable title
  description: string;                 // Detailed description
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info';
  category: string;                    // e.g., "model_performance", "infrastructure"
  service_id: string;                  // Affected service
  environment: 'production' | 'staging' | 'development';

  // Status tracking
  status: 'triggered' | 'acknowledged' | 'investigating' | 'identified' | 'resolved' | 'closed';
  triggered_at: Date;
  acknowledged_at?: Date;
  resolved_at?: Date;
  acknowledged_by?: string;            // User email or ID
  resolved_by?: string;

  // LLM-specific metadata
  labels: {
    model_id?: string;                 // e.g., "gpt-4", "claude-3-opus"
    model_version?: string;            // e.g., "v1.2.3"
    metric_type?: string;              // e.g., "latency", "accuracy", "cost"
    tenant_id?: string;                // Customer/tenant identifier
    region?: string;                   // Deployment region
    component?: string;                // e.g., "inference", "embedding"
    [key: string]: string | undefined;
  };

  // Context
  metadata: {
    current_value?: number;
    threshold?: number;
    unit?: string;
    dashboard_url?: string;
    runbook_url?: string;
    deployment_id?: string;
    recent_changes?: Array<{
      type: string;
      description: string;
      timestamp: Date;
    }>;
    [key: string]: any;
  };

  // Correlation
  parent_incident_id?: string;         // If this is a child incident
  related_incident_ids?: string[];     // Related/correlated incidents
  suppressed?: boolean;                // If suppressed due to correlation

  // Notifications
  notifications: Array<{
    channel: string;                   // "slack", "email", "sms", "webhook"
    target: string;                    // Channel-specific target (email, webhook URL)
    sent_at: Date;
    delivered: boolean;
    error?: string;
  }>;

  // Escalation
  escalation_level: number;            // Current escalation level (0, 1, 2...)
  escalation_policy_id?: string;
  assigned_to?: string;

  // Audit
  timeline: Array<{
    timestamp: Date;
    event: string;                     // "triggered", "acknowledged", "escalated", etc.
    user?: string;
    details?: any;
  }>;

  // Aggregation
  occurrence_count: number;            // Times this incident was triggered
  first_seen_at: Date;
  last_seen_at: Date;
}
```

### Escalation Policy Schema

```typescript
interface EscalationPolicy {
  id: string;
  name: string;
  description?: string;

  // Matching rules
  services: string[];                  // Service IDs this policy applies to
  severity?: ('critical' | 'high' | 'medium' | 'low')[];
  environment?: ('production' | 'staging' | 'development')[];

  // Escalation levels
  levels: Array<{
    level: number;
    delay_minutes: number;             // Delay before escalating to this level
    targets: Array<{
      type: 'user' | 'schedule' | 'slack_channel' | 'team';
      id: string;                      // User ID, schedule ID, channel name, etc.
      email?: string;                  // For user type
    }>;
  }>;

  // Repeat configuration
  repeat?: {
    enabled: boolean;
    count: number;                     // Number of times to repeat escalation
    interval_minutes?: number;         // Override delay for repeats
  };

  // Time restrictions
  business_hours_only?: boolean;
  timezone?: string;
  business_hours?: {
    days: number[];                    // 0=Sunday, 6=Saturday
    start: string;                     // "09:00"
    end: string;                       // "17:00"
  };

  created_at: Date;
  updated_at: Date;
}
```

### Notification Rule Schema

```typescript
interface NotificationRule {
  id: string;
  name: string;
  description?: string;
  priority: number;                    // Higher priority rules evaluated first

  // Matching conditions (all must match)
  match: {
    severity?: ('critical' | 'high' | 'medium' | 'low')[];
    category?: string[];
    service_id?: string[];
    environment?: ('production' | 'staging' | 'development')[];
    labels?: Record<string, string[]>; // Label key-value matches
  };

  // Notification channels
  channels: Array<{
    type: 'slack' | 'email' | 'sms' | 'webhook' | 'pagerduty';
    config: {
      // Slack config
      channel?: string;
      mention?: string;                // @oncall, @here, @channel
      thread_updates?: boolean;        // Post updates in thread

      // Email config
      recipients?: string[];
      cc?: string[];
      subject_template?: string;

      // Webhook config
      url?: string;
      method?: 'POST' | 'PUT';
      headers?: Record<string, string>;
      secret?: string;                 // For signature verification

      // SMS/PagerDuty config
      service_key?: string;
      phone_numbers?: string[];
    };
  }>;

  // Delivery options
  digest?: {
    enabled: boolean;
    interval_minutes: number;          // Batch notifications every N minutes
  };

  rate_limit?: {
    max_per_hour?: number;
    max_per_day?: number;
  };

  enabled: boolean;
  created_at: Date;
  updated_at: Date;
}
```

---

## 2. Core Algorithms

### Fingerprint Generation

```typescript
import crypto from 'crypto';

function generateFingerprint(incident: Partial<Incident>): string {
  // Create stable fingerprint from key attributes
  const components = [
    incident.service_id || '',
    incident.category || '',
    incident.labels?.metric_type || '',
    incident.labels?.model_id || '',
    incident.environment || '',
    // Round threshold to prevent minor variations from creating different fingerprints
    incident.metadata?.threshold ? Math.round(incident.metadata.threshold) : '',
  ];

  const fingerprintString = components
    .filter(c => c !== '')
    .join('|');

  return crypto
    .createHash('sha256')
    .update(fingerprintString)
    .digest('hex');
}

// Example usage
const incident = {
  service_id: 'llm-inference-prod',
  category: 'model_performance',
  labels: { metric_type: 'latency', model_id: 'gpt-4' },
  environment: 'production',
  metadata: { threshold: 5.0 }
};

const fingerprint = generateFingerprint(incident);
// Output: "a3b2c1d4e5f6..." (deterministic hash)
```

### Deduplication Logic

```typescript
interface DeduplicationResult {
  action: 'create' | 'merge' | 'suppress';
  existing_incident_id?: string;
  reason: string;
}

async function deduplicateIncident(
  newIncident: Partial<Incident>,
  lookbackMinutes: number = 15
): Promise<DeduplicationResult> {
  const fingerprint = generateFingerprint(newIncident);
  const lookbackTime = new Date(Date.now() - lookbackMinutes * 60 * 1000);

  // Find existing incidents with same fingerprint
  const existingIncidents = await db.incidents.findMany({
    where: {
      fingerprint,
      status: { in: ['triggered', 'acknowledged', 'investigating', 'identified'] },
      first_seen_at: { gte: lookbackTime }
    },
    orderBy: { first_seen_at: 'desc' }
  });

  if (existingIncidents.length === 0) {
    return { action: 'create', reason: 'No matching incident found' };
  }

  const existing = existingIncidents[0];

  // Check if incident is resolved but retriggering
  if (existing.status === 'resolved' &&
      new Date(existing.resolved_at!).getTime() < Date.now() - 5 * 60 * 1000) {
    return {
      action: 'create',
      reason: 'Previous incident resolved >5 min ago, creating new'
    };
  }

  // Merge into existing incident
  await db.incidents.update({
    where: { id: existing.id },
    data: {
      occurrence_count: { increment: 1 },
      last_seen_at: new Date(),
      timeline: {
        push: {
          timestamp: new Date(),
          event: 'occurrence',
          details: { count: existing.occurrence_count + 1 }
        }
      }
    }
  });

  return {
    action: 'merge',
    existing_incident_id: existing.id,
    reason: `Merged into existing incident (occurrence #${existing.occurrence_count + 1})`
  };
}
```

### Severity Scoring

```typescript
interface SeverityFactors {
  user_impact_percentage: number;      // 0-100
  performance_degradation: number;     // Multiplier (e.g., 2.5 = 250% of normal)
  error_rate: number;                  // 0-100
  cost_increase: number;               // Multiplier
  sla_risk_minutes: number;            // Minutes until SLA breach
}

function calculateSeverity(factors: SeverityFactors): Incident['severity'] {
  // Weighted scoring
  const weights = {
    user_impact: 0.35,
    performance: 0.25,
    errors: 0.20,
    cost: 0.10,
    sla: 0.10
  };

  // Normalize factors to 0-100 scale
  const normalized = {
    user_impact: factors.user_impact_percentage,
    performance: Math.min(factors.performance_degradation * 20, 100),
    errors: factors.error_rate,
    cost: Math.min(factors.cost_increase * 20, 100),
    sla: factors.sla_risk_minutes <= 60 ? 100 : Math.max(0, 100 - factors.sla_risk_minutes / 10)
  };

  // Calculate weighted score
  const score =
    normalized.user_impact * weights.user_impact +
    normalized.performance * weights.performance +
    normalized.errors * weights.errors +
    normalized.cost * weights.cost +
    normalized.sla * weights.sla;

  // Map score to severity
  if (score >= 80) return 'critical';
  if (score >= 60) return 'high';
  if (score >= 40) return 'medium';
  if (score >= 20) return 'low';
  return 'info';
}

// Example usage
const severity = calculateSeverity({
  user_impact_percentage: 25,          // 25% of users affected
  performance_degradation: 3.5,        // 350% of normal latency
  error_rate: 15,                      // 15% error rate
  cost_increase: 1.2,                  // 20% cost increase
  sla_risk_minutes: 30                 // SLA breach in 30 minutes
});
// Output: "high"
```

### Correlation Detection

```typescript
interface CorrelationResult {
  correlated: boolean;
  parent_incident_id?: string;
  correlation_type?: 'duplicate' | 'cascade' | 'related';
  confidence: number;                  // 0-1
  reason: string;
}

async function detectCorrelation(
  newIncident: Partial<Incident>,
  lookbackMinutes: number = 30
): Promise<CorrelationResult> {
  const lookbackTime = new Date(Date.now() - lookbackMinutes * 60 * 1000);

  // Get recent incidents
  const recentIncidents = await db.incidents.findMany({
    where: {
      status: { in: ['triggered', 'acknowledged', 'investigating'] },
      first_seen_at: { gte: lookbackTime }
    },
    orderBy: { triggered_at: 'desc' }
  });

  for (const existing of recentIncidents) {
    // Check for exact duplicates (same fingerprint)
    if (existing.fingerprint === generateFingerprint(newIncident)) {
      return {
        correlated: true,
        parent_incident_id: existing.id,
        correlation_type: 'duplicate',
        confidence: 1.0,
        reason: 'Exact fingerprint match'
      };
    }

    // Check for cascading failures (dependency graph)
    const isCascade = await checkCascadingFailure(newIncident, existing);
    if (isCascade) {
      return {
        correlated: true,
        parent_incident_id: existing.id,
        correlation_type: 'cascade',
        confidence: 0.9,
        reason: 'Detected cascading failure from upstream service'
      };
    }

    // Check for related incidents (similar symptoms)
    const similarity = calculateSimilarity(newIncident, existing);
    if (similarity > 0.75) {
      return {
        correlated: true,
        parent_incident_id: existing.id,
        correlation_type: 'related',
        confidence: similarity,
        reason: `High similarity score: ${similarity.toFixed(2)}`
      };
    }
  }

  return {
    correlated: false,
    confidence: 0,
    reason: 'No correlation found'
  };
}

async function checkCascadingFailure(
  newIncident: Partial<Incident>,
  existingIncident: Incident
): Promise<boolean> {
  // Check service dependency graph
  const dependencyGraph = await getServiceDependencies();

  if (!newIncident.service_id || !existingIncident.service_id) {
    return false;
  }

  // Check if new incident's service depends on existing incident's service
  const dependencies = dependencyGraph[newIncident.service_id] || [];
  return dependencies.includes(existingIncident.service_id);
}

function calculateSimilarity(
  incident1: Partial<Incident>,
  incident2: Incident
): number {
  let matchPoints = 0;
  let totalPoints = 0;

  // Same service (high weight)
  totalPoints += 3;
  if (incident1.service_id === incident2.service_id) matchPoints += 3;

  // Same category (medium weight)
  totalPoints += 2;
  if (incident1.category === incident2.category) matchPoints += 2;

  // Same model (medium weight)
  totalPoints += 2;
  if (incident1.labels?.model_id === incident2.labels?.model_id) matchPoints += 2;

  // Same metric type (low weight)
  totalPoints += 1;
  if (incident1.labels?.metric_type === incident2.labels?.metric_type) matchPoints += 1;

  // Same environment (low weight)
  totalPoints += 1;
  if (incident1.environment === incident2.environment) matchPoints += 1;

  return matchPoints / totalPoints;
}
```

---

## 3. Notification Patterns

### Multi-Channel Notification Dispatcher

```typescript
interface NotificationChannel {
  send(incident: Incident, target: string, config: any): Promise<void>;
}

class NotificationDispatcher {
  private channels: Map<string, NotificationChannel>;
  private circuitBreakers: Map<string, CircuitBreaker>;

  async notify(incident: Incident, rules: NotificationRule[]): Promise<void> {
    const notifications: Promise<void>[] = [];

    for (const rule of rules) {
      if (!this.matchesRule(incident, rule)) continue;

      for (const channelConfig of rule.channels) {
        const channel = this.channels.get(channelConfig.type);
        const breaker = this.circuitBreakers.get(channelConfig.type);

        if (!channel || !breaker) continue;

        // Execute with circuit breaker protection
        const notification = breaker.execute(async () => {
          await this.sendWithRetry(channel, incident, channelConfig);
        }).catch(async (error) => {
          console.error(`Failed to send ${channelConfig.type} notification:`, error);

          // Try fallback channel
          await this.sendFallback(incident, channelConfig.type, error);
        });

        notifications.push(notification);
      }
    }

    await Promise.allSettled(notifications);
  }

  private async sendWithRetry(
    channel: NotificationChannel,
    incident: Incident,
    config: any,
    maxRetries: number = 3
  ): Promise<void> {
    let lastError: Error | undefined;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
      try {
        await channel.send(incident, config.config.channel || config.config.url, config.config);

        // Record successful delivery
        await this.recordDelivery(incident.id, config.type, true);
        return;
      } catch (error) {
        lastError = error as Error;

        // Exponential backoff: 1s, 2s, 4s
        const delayMs = Math.pow(2, attempt) * 1000;
        await new Promise(resolve => setTimeout(resolve, delayMs));
      }
    }

    // All retries failed
    await this.recordDelivery(incident.id, config.type, false, lastError?.message);
    throw lastError;
  }

  private async sendFallback(
    incident: Incident,
    failedChannel: string,
    error: Error
  ): Promise<void> {
    // Define fallback chain
    const fallbackChain: Record<string, string> = {
      'slack': 'email',
      'email': 'webhook',
      'sms': 'email'
    };

    const fallback = fallbackChain[failedChannel];
    if (!fallback) return;

    const fallbackChannel = this.channels.get(fallback);
    if (!fallbackChannel) return;

    console.log(`Attempting fallback: ${failedChannel} → ${fallback}`);

    try {
      await fallbackChannel.send(incident, 'fallback', {
        note: `Primary channel (${failedChannel}) failed: ${error.message}`
      });
    } catch (fallbackError) {
      console.error(`Fallback also failed:`, fallbackError);
    }
  }

  private matchesRule(incident: Incident, rule: NotificationRule): boolean {
    // Check severity
    if (rule.match.severity && !rule.match.severity.includes(incident.severity)) {
      return false;
    }

    // Check category
    if (rule.match.category && !rule.match.category.includes(incident.category)) {
      return false;
    }

    // Check service
    if (rule.match.service_id && !rule.match.service_id.includes(incident.service_id)) {
      return false;
    }

    // Check environment
    if (rule.match.environment && !rule.match.environment.includes(incident.environment)) {
      return false;
    }

    // Check labels
    if (rule.match.labels) {
      for (const [key, values] of Object.entries(rule.match.labels)) {
        const incidentValue = incident.labels[key];
        if (!incidentValue || !values.includes(incidentValue)) {
          return false;
        }
      }
    }

    return true;
  }

  private async recordDelivery(
    incidentId: string,
    channel: string,
    success: boolean,
    error?: string
  ): Promise<void> {
    await db.incidents.update({
      where: { id: incidentId },
      data: {
        notifications: {
          push: {
            channel,
            sent_at: new Date(),
            delivered: success,
            error
          }
        }
      }
    });
  }
}
```

### Circuit Breaker Implementation

```typescript
enum CircuitState {
  CLOSED = 'CLOSED',    // Normal operation
  OPEN = 'OPEN',        // Failing, reject requests
  HALF_OPEN = 'HALF_OPEN' // Testing if recovered
}

class CircuitBreaker {
  private state: CircuitState = CircuitState.CLOSED;
  private failureCount: number = 0;
  private successCount: number = 0;
  private lastFailureTime: number = 0;

  constructor(
    private threshold: number = 5,           // Failures before opening
    private timeout: number = 60000,         // Time before trying again (ms)
    private halfOpenSuccessThreshold: number = 2  // Successes to close
  ) {}

  async execute<T>(fn: () => Promise<T>): Promise<T> {
    if (this.state === CircuitState.OPEN) {
      if (Date.now() - this.lastFailureTime > this.timeout) {
        this.state = CircuitState.HALF_OPEN;
        this.successCount = 0;
      } else {
        throw new Error('Circuit breaker is OPEN');
      }
    }

    try {
      const result = await fn();
      this.onSuccess();
      return result;
    } catch (error) {
      this.onFailure();
      throw error;
    }
  }

  private onSuccess(): void {
    this.failureCount = 0;

    if (this.state === CircuitState.HALF_OPEN) {
      this.successCount++;
      if (this.successCount >= this.halfOpenSuccessThreshold) {
        this.state = CircuitState.CLOSED;
      }
    }
  }

  private onFailure(): void {
    this.failureCount++;
    this.lastFailureTime = Date.now();

    if (this.failureCount >= this.threshold) {
      this.state = CircuitState.OPEN;
    }
  }

  getState(): CircuitState {
    return this.state;
  }
}
```

### Slack Channel Implementation

```typescript
import { WebClient } from '@slack/web-api';

class SlackNotificationChannel implements NotificationChannel {
  private client: WebClient;

  constructor(token: string) {
    this.client = new WebClient(token);
  }

  async send(incident: Incident, channel: string, config: any): Promise<void> {
    const color = this.getSeverityColor(incident.severity);
    const mention = config.mention || '';

    const message = {
      channel,
      text: `${mention} ${this.getSeverityEmoji(incident.severity)} *${incident.severity.toUpperCase()}* - ${incident.title}`,
      attachments: [{
        color,
        fields: [
          {
            title: 'Service',
            value: incident.service_id,
            short: true
          },
          {
            title: 'Environment',
            value: incident.environment,
            short: true
          },
          {
            title: 'Status',
            value: incident.status,
            short: true
          },
          {
            title: 'Triggered',
            value: this.formatTimestamp(incident.triggered_at),
            short: true
          },
          ...(incident.labels.model_id ? [{
            title: 'Model',
            value: incident.labels.model_id,
            short: true
          }] : []),
          ...(incident.metadata.current_value && incident.metadata.threshold ? [{
            title: 'Current / Threshold',
            value: `${incident.metadata.current_value} / ${incident.metadata.threshold} ${incident.metadata.unit || ''}`,
            short: true
          }] : [])
        ],
        text: incident.description,
        footer: `Incident ID: ${incident.id}`,
        actions: [
          {
            type: 'button',
            text: 'Acknowledge',
            url: `https://incidents.company.com/${incident.id}/acknowledge`,
            style: 'primary'
          },
          {
            type: 'button',
            text: 'View Dashboard',
            url: incident.metadata.dashboard_url || '#'
          },
          {
            type: 'button',
            text: 'Runbook',
            url: incident.metadata.runbook_url || '#'
          }
        ]
      }]
    };

    const response = await this.client.chat.postMessage(message);

    // Store thread_ts for updates
    if (response.ts && config.thread_updates) {
      await this.storeThreadId(incident.id, response.ts as string);
    }
  }

  async sendUpdate(incident: Incident, threadTs: string, update: string): Promise<void> {
    await this.client.chat.postMessage({
      channel: incident.notifications[0]?.target || '',
      thread_ts: threadTs,
      text: update
    });
  }

  private getSeverityColor(severity: string): string {
    const colors = {
      critical: '#FF0000',
      high: '#FF6600',
      medium: '#FFB800',
      low: '#0099FF',
      info: '#999999'
    };
    return colors[severity as keyof typeof colors] || '#999999';
  }

  private getSeverityEmoji(severity: string): string {
    const emojis = {
      critical: '🔴',
      high: '🟠',
      medium: '🟡',
      low: '🔵',
      info: '⚪'
    };
    return emojis[severity as keyof typeof emojis] || '⚪';
  }

  private formatTimestamp(date: Date): string {
    return `<!date^${Math.floor(date.getTime() / 1000)}^{date_short_pretty} at {time}|${date.toISOString()}>`;
  }

  private async storeThreadId(incidentId: string, threadTs: string): Promise<void> {
    await db.incidentThreads.create({
      data: { incident_id: incidentId, thread_ts: threadTs, channel: 'slack' }
    });
  }
}
```

---

## 4. Escalation Engine

```typescript
class EscalationEngine {
  private timers: Map<string, NodeJS.Timeout> = new Map();

  async startEscalation(incident: Incident, policy: EscalationPolicy): Promise<void> {
    // Clear any existing escalation timers
    this.clearEscalation(incident.id);

    // Execute level 0 immediately
    await this.executeLevel(incident, policy, 0);

    // Schedule future escalation levels
    for (let i = 1; i < policy.levels.length; i++) {
      const level = policy.levels[i];
      const delay = level.delay_minutes * 60 * 1000;

      const timer = setTimeout(async () => {
        // Check if incident still needs escalation
        const current = await db.incidents.findUnique({ where: { id: incident.id } });

        if (!current || current.status === 'resolved' || current.status === 'closed') {
          return; // Incident resolved, don't escalate
        }

        if (current.acknowledged_at && !this.shouldEscalateAfterAck(policy)) {
          return; // Incident acknowledged and policy doesn't escalate after ack
        }

        await this.executeLevel(current, policy, i);
      }, delay);

      this.timers.set(`${incident.id}-${i}`, timer);
    }

    // Schedule repeats if configured
    if (policy.repeat?.enabled) {
      await this.scheduleRepeats(incident, policy);
    }
  }

  async executeLevel(incident: Incident, policy: EscalationPolicy, levelIndex: number): Promise<void> {
    const level = policy.levels[levelIndex];

    console.log(`Escalating incident ${incident.id} to level ${levelIndex}`);

    // Update incident escalation level
    await db.incidents.update({
      where: { id: incident.id },
      data: {
        escalation_level: levelIndex,
        timeline: {
          push: {
            timestamp: new Date(),
            event: 'escalated',
            details: { level: levelIndex, targets: level.targets }
          }
        }
      }
    });

    // Notify all targets at this level
    const notifications: Promise<void>[] = [];

    for (const target of level.targets) {
      switch (target.type) {
        case 'user':
          notifications.push(this.notifyUser(incident, target.email || target.id));
          break;

        case 'schedule':
          const oncall = await this.getOncallUser(target.id);
          if (oncall) {
            notifications.push(this.notifyUser(incident, oncall.email));
          }
          break;

        case 'slack_channel':
          notifications.push(this.notifySlackChannel(incident, target.id));
          break;

        case 'team':
          const team = await this.getTeamMembers(target.id);
          for (const member of team) {
            notifications.push(this.notifyUser(incident, member.email));
          }
          break;
      }
    }

    await Promise.allSettled(notifications);
  }

  private async scheduleRepeats(incident: Incident, policy: EscalationPolicy): Promise<void> {
    if (!policy.repeat) return;

    const maxLevel = policy.levels.length - 1;
    const intervalMs = (policy.repeat.interval_minutes || policy.levels[maxLevel].delay_minutes) * 60 * 1000;

    for (let repeat = 1; repeat <= policy.repeat.count; repeat++) {
      const delay = (maxLevel + repeat) * intervalMs;

      const timer = setTimeout(async () => {
        const current = await db.incidents.findUnique({ where: { id: incident.id } });

        if (!current || current.status === 'resolved' || current.status === 'closed') {
          return;
        }

        console.log(`Repeat escalation ${repeat}/${policy.repeat!.count} for incident ${incident.id}`);

        // Restart escalation from level 0
        await this.executeLevel(current, policy, 0);
      }, delay);

      this.timers.set(`${incident.id}-repeat-${repeat}`, timer);
    }
  }

  clearEscalation(incidentId: string): void {
    // Clear all timers for this incident
    for (const [key, timer] of this.timers.entries()) {
      if (key.startsWith(incidentId)) {
        clearTimeout(timer);
        this.timers.delete(key);
      }
    }
  }

  private shouldEscalateAfterAck(policy: EscalationPolicy): boolean {
    // Some policies stop escalating after acknowledgment
    // This would be configurable per policy
    return false; // Default: don't escalate after ack
  }

  private async notifyUser(incident: Incident, email: string): Promise<void> {
    // Send email, SMS, or page based on incident severity
    if (incident.severity === 'critical') {
      await sendSMS(email, `CRITICAL: ${incident.title}`);
      await sendEmail(email, 'critical-incident', incident);
    } else {
      await sendEmail(email, 'incident-notification', incident);
    }
  }

  private async notifySlackChannel(incident: Incident, channel: string): Promise<void> {
    const slackChannel = this.channels.get('slack') as SlackNotificationChannel;
    await slackChannel.send(incident, channel, {});
  }

  private async getOncallUser(scheduleId: string): Promise<{ email: string } | null> {
    // Query on-call schedule system (PagerDuty, Opsgenie, etc.)
    // Return currently on-call user
    return { email: 'oncall@company.com' }; // Placeholder
  }

  private async getTeamMembers(teamId: string): Promise<Array<{ email: string }>> {
    // Query team membership from database or directory service
    return []; // Placeholder
  }
}
```

---

## 5. Auto-Remediation Framework

```typescript
interface RemediationAction {
  trigger: {
    alert_name?: string;
    category?: string;
    severity?: string;
  };
  conditions?: Array<{
    type: string;
    [key: string]: any;
  }>;
  actions: Array<{
    type: string;
    [key: string]: any;
  }>;
  max_auto_attempts?: number;
}

class AutoRemediationEngine {
  private remediations: RemediationAction[];
  private attemptCounts: Map<string, number> = new Map();

  async tryRemediate(incident: Incident): Promise<boolean> {
    const matchingRemediations = this.remediations.filter(r =>
      this.matchesTrigger(incident, r.trigger)
    );

    for (const remediation of matchingRemediations) {
      // Check if we've exceeded max attempts
      const attemptKey = `${incident.fingerprint}-${remediation.actions[0].type}`;
      const attempts = this.attemptCounts.get(attemptKey) || 0;

      if (remediation.max_auto_attempts && attempts >= remediation.max_auto_attempts) {
        console.log(`Max auto-remediation attempts reached for ${attemptKey}`);
        continue;
      }

      // Check conditions
      if (remediation.conditions) {
        const conditionsMet = await this.checkConditions(incident, remediation.conditions);
        if (!conditionsMet) continue;
      }

      // Execute remediation actions
      try {
        await this.executeActions(incident, remediation.actions);

        // Increment attempt count
        this.attemptCounts.set(attemptKey, attempts + 1);

        // Add note to incident
        await db.incidents.update({
          where: { id: incident.id },
          data: {
            timeline: {
              push: {
                timestamp: new Date(),
                event: 'auto_remediation',
                user: 'system',
                details: {
                  actions: remediation.actions.map(a => a.type),
                  attempt: attempts + 1
                }
              }
            }
          }
        });

        return true;
      } catch (error) {
        console.error('Auto-remediation failed:', error);

        // Add failure note to incident
        await db.incidents.update({
          where: { id: incident.id },
          data: {
            timeline: {
              push: {
                timestamp: new Date(),
                event: 'auto_remediation_failed',
                user: 'system',
                details: { error: (error as Error).message }
              }
            }
          }
        });
      }
    }

    return false;
  }

  private matchesTrigger(incident: Incident, trigger: RemediationAction['trigger']): boolean {
    if (trigger.category && incident.category !== trigger.category) return false;
    if (trigger.severity && incident.severity !== trigger.severity) return false;
    if (trigger.alert_name && incident.title !== trigger.alert_name) return false;
    return true;
  }

  private async checkConditions(incident: Incident, conditions: any[]): Promise<boolean> {
    for (const condition of conditions) {
      switch (condition.type) {
        case 'recent_deployment':
          const hasRecentDeploy = await this.checkRecentDeployment(
            incident.service_id,
            condition.within_minutes
          );
          if (!hasRecentDeploy) return false;
          break;

        case 'metric_threshold':
          const metricsurpasses = await this.checkMetricThreshold(
            condition.metric,
            condition.value
          );
          if (!metricsurpasses) return false;
          break;

        // Add more condition types as needed
      }
    }

    return true;
  }

  private async executeActions(incident: Incident, actions: any[]): Promise<void> {
    for (const action of actions) {
      switch (action.type) {
        case 'rollback':
          await this.rollbackDeployment(action.target || incident.service_id);
          break;

        case 'scale_up':
          await this.scaleService(
            action.target || incident.service_id,
            action.replicas
          );
          break;

        case 'restart':
          await this.restartService(action.target || incident.service_id);
          break;

        case 'increase_rate_limit':
          await this.updateRateLimit(
            incident.labels.api_key || '',
            action.multiplier,
            action.duration_minutes
          );
          break;

        case 'switch_model_variant':
          await this.switchModelVariant(
            incident.labels.model_id || '',
            action.variant
          );
          break;

        case 'notify':
          await this.sendNotification(incident, action.message);
          break;

        // Add more action types as needed
      }
    }
  }

  private async checkRecentDeployment(serviceId: string, withinMinutes: number): Promise<boolean> {
    // Query deployment history
    // Return true if deployment within time window
    return false; // Placeholder
  }

  private async checkMetricThreshold(metric: string, threshold: number): Promise<boolean> {
    // Query monitoring system
    // Return true if metric exceeds threshold
    return false; // Placeholder
  }

  private async rollbackDeployment(serviceId: string): Promise<void> {
    console.log(`Rolling back deployment for ${serviceId}`);
    // Execute rollback via deployment system (kubectl, AWS, etc.)
  }

  private async scaleService(serviceId: string, replicaDelta: string): Promise<void> {
    console.log(`Scaling ${serviceId} by ${replicaDelta}`);
    // Execute scaling via orchestrator (Kubernetes, ECS, etc.)
  }

  private async restartService(serviceId: string): Promise<void> {
    console.log(`Restarting ${serviceId}`);
    // Execute restart
  }

  private async updateRateLimit(apiKey: string, multiplier: number, durationMin: number): Promise<void> {
    console.log(`Increasing rate limit for ${apiKey} by ${multiplier}x for ${durationMin} minutes`);
    // Update rate limit in API gateway
  }

  private async switchModelVariant(modelId: string, variant: string): Promise<void> {
    console.log(`Switching ${modelId} to variant: ${variant}`);
    // Update model routing configuration
  }

  private async sendNotification(incident: Incident, message: string): Promise<void> {
    await db.incidents.update({
      where: { id: incident.id },
      data: {
        timeline: {
          push: {
            timestamp: new Date(),
            event: 'note',
            user: 'system',
            details: { note: message }
          }
        }
      }
    });
  }
}
```

---

## 6. API Endpoint Examples

```typescript
import express from 'express';

const app = express();
app.use(express.json());

// Create incident
app.post('/v1/incidents', async (req, res) => {
  try {
    const incidentData: Partial<Incident> = req.body;

    // Generate fingerprint
    const fingerprint = generateFingerprint(incidentData);

    // Check for deduplication
    const dedupResult = await deduplicateIncident(incidentData);

    if (dedupResult.action === 'merge') {
      return res.status(200).json({
        status: 'merged',
        incident_id: dedupResult.existing_incident_id,
        message: dedupResult.reason
      });
    }

    // Calculate severity if not provided
    if (!incidentData.severity && incidentData.metadata) {
      incidentData.severity = calculateSeverity({
        user_impact_percentage: incidentData.metadata.user_impact || 0,
        performance_degradation: incidentData.metadata.degradation || 1,
        error_rate: incidentData.metadata.error_rate || 0,
        cost_increase: incidentData.metadata.cost_increase || 1,
        sla_risk_minutes: incidentData.metadata.sla_risk_minutes || 999
      });
    }

    // Create incident
    const incident = await db.incidents.create({
      data: {
        ...incidentData,
        id: `inc_${generateId()}`,
        fingerprint,
        status: 'triggered',
        triggered_at: new Date(),
        first_seen_at: new Date(),
        last_seen_at: new Date(),
        occurrence_count: 1,
        escalation_level: 0,
        timeline: [{
          timestamp: new Date(),
          event: 'triggered',
          user: 'system'
        }]
      }
    });

    // Try auto-remediation
    const autoRemediation = new AutoRemediationEngine();
    const remediated = await autoRemediation.tryRemediate(incident);

    if (!remediated) {
      // Find escalation policy
      const policy = await findEscalationPolicy(incident);

      if (policy) {
        // Start escalation
        const escalation = new EscalationEngine();
        await escalation.startEscalation(incident, policy);
      }

      // Send notifications
      const rules = await db.notificationRules.findMany({ where: { enabled: true } });
      const dispatcher = new NotificationDispatcher();
      await dispatcher.notify(incident, rules);
    }

    res.status(201).json({
      status: 'created',
      incident
    });
  } catch (error) {
    console.error('Error creating incident:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// Acknowledge incident
app.patch('/v1/incidents/:id/acknowledge', async (req, res) => {
  try {
    const { id } = req.params;
    const { user, note } = req.body;

    const incident = await db.incidents.update({
      where: { id },
      data: {
        status: 'acknowledged',
        acknowledged_at: new Date(),
        acknowledged_by: user,
        timeline: {
          push: {
            timestamp: new Date(),
            event: 'acknowledged',
            user,
            details: { note }
          }
        }
      }
    });

    // Clear escalation timers
    const escalation = new EscalationEngine();
    escalation.clearEscalation(id);

    res.json({ status: 'acknowledged', incident });
  } catch (error) {
    console.error('Error acknowledging incident:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// Resolve incident
app.patch('/v1/incidents/:id/resolve', async (req, res) => {
  try {
    const { id } = req.params;
    const { user, resolution } = req.body;

    const incident = await db.incidents.update({
      where: { id },
      data: {
        status: 'resolved',
        resolved_at: new Date(),
        resolved_by: user,
        timeline: {
          push: {
            timestamp: new Date(),
            event: 'resolved',
            user,
            details: { resolution }
          }
        }
      }
    });

    // Clear escalation timers
    const escalation = new EscalationEngine();
    escalation.clearEscalation(id);

    // Send resolution notification
    const rules = await db.notificationRules.findMany({ where: { enabled: true } });
    const dispatcher = new NotificationDispatcher();
    // Filter for resolution notifications
    const resolutionRules = rules.filter(r =>
      r.channels.some(c => (c.config as any).send_on_resolve)
    );
    await dispatcher.notify(incident, resolutionRules);

    res.json({ status: 'resolved', incident });
  } catch (error) {
    console.error('Error resolving incident:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// List incidents
app.get('/v1/incidents', async (req, res) => {
  try {
    const {
      status,
      severity,
      service_id,
      environment,
      limit = 50,
      offset = 0
    } = req.query;

    const where: any = {};
    if (status) where.status = { in: (status as string).split(',') };
    if (severity) where.severity = { in: (severity as string).split(',') };
    if (service_id) where.service_id = service_id;
    if (environment) where.environment = environment;

    const incidents = await db.incidents.findMany({
      where,
      orderBy: { triggered_at: 'desc' },
      take: Number(limit),
      skip: Number(offset)
    });

    const total = await db.incidents.count({ where });

    res.json({
      incidents,
      pagination: {
        total,
        limit: Number(limit),
        offset: Number(offset)
      }
    });
  } catch (error) {
    console.error('Error listing incidents:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// Get incident timeline
app.get('/v1/incidents/:id/timeline', async (req, res) => {
  try {
    const { id } = req.params;

    const incident = await db.incidents.findUnique({
      where: { id },
      select: { timeline: true }
    });

    if (!incident) {
      return res.status(404).json({ error: 'Incident not found' });
    }

    res.json({ timeline: incident.timeline });
  } catch (error) {
    console.error('Error fetching timeline:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// Webhook receiver (Prometheus AlertManager format)
app.post('/v1/webhooks/alertmanager', async (req, res) => {
  try {
    const { alerts } = req.body;

    for (const alert of alerts) {
      const incident: Partial<Incident> = {
        title: alert.labels.alertname,
        description: alert.annotations.description || alert.annotations.summary,
        severity: mapAlertManagerSeverity(alert.labels.severity),
        category: alert.labels.category || 'infrastructure',
        service_id: alert.labels.service || alert.labels.job,
        environment: alert.labels.environment || 'production',
        labels: alert.labels,
        metadata: {
          ...alert.annotations,
          dashboard_url: alert.annotations.dashboard,
          runbook_url: alert.annotations.runbook,
          alert_manager_url: alert.generatorURL
        }
      };

      // Create incident (will handle deduplication automatically)
      await fetch('http://localhost:3000/v1/incidents', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(incident)
      });
    }

    res.status(200).json({ status: 'received' });
  } catch (error) {
    console.error('Error processing AlertManager webhook:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

function mapAlertManagerSeverity(severity: string): Incident['severity'] {
  const mapping: Record<string, Incident['severity']> = {
    'critical': 'critical',
    'error': 'high',
    'warning': 'medium',
    'info': 'low'
  };
  return mapping[severity] || 'medium';
}

app.listen(3000, () => {
  console.log('Incident Manager API listening on port 3000');
});
```

---

## Summary

This architecture reference provides:

1. **Complete data models** for incidents, policies, and rules
2. **Core algorithms** for fingerprinting, deduplication, severity scoring, and correlation
3. **Notification patterns** with circuit breakers and multi-channel delivery
4. **Escalation engine** with time-based and repeat logic
5. **Auto-remediation framework** with configurable actions
6. **REST API examples** for incident management

These patterns can be directly implemented in the LLM-Incident-Manager system, with customization for specific LLM workload requirements.
