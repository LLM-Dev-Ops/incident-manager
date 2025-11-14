# @llm-dev-ops/incident-manager-types

TypeScript type definitions for the LLM Incident Manager - Enterprise-grade incident management system for LLM operations.

## Installation

```bash
npm install @llm-dev-ops/incident-manager-types
```

Or with yarn:

```bash
yarn add @llm-dev-ops/incident-manager-types
```

## Usage

```typescript
import {
  Incident,
  Severity,
  IncidentStatus,
  CreateIncidentRequest,
  EscalationPolicy,
  NotificationTemplate,
  RawEvent,
  LLMRequest,
  LLMResponse
} from '@llm-dev-ops/incident-manager-types';

// Create an incident event
const event: RawEvent = {
  event_id: 'evt-123',
  source: 'llm-sentinel',
  source_version: '1.0.0',
  timestamp: new Date().toISOString(),
  event_type: 'anomaly',
  category: 'performance',
  title: 'High Latency Detected',
  description: 'API latency exceeded threshold',
  severity: 'P1',
  resource: {
    type: 'service',
    id: 'api-gateway',
    name: 'API Gateway',
    metadata: {}
  },
  metrics: {
    latency_ms: 5000,
    error_rate: 0.15
  },
  tags: {
    environment: 'production',
    region: 'us-east-1'
  },
  payload: {}
};

// Type-safe incident creation
const request: CreateIncidentRequest = {
  event,
  options: {
    skip_deduplication: false
  }
};

// Work with typed incidents
function handleIncident(incident: Incident): void {
  console.log(`Incident ${incident.id}: ${incident.title}`);
  console.log(`Severity: ${incident.severity}`);
  console.log(`Status: ${incident.status}`);

  if (incident.severity === 'P0' || incident.severity === 'P1') {
    // Handle critical incident
    console.log('Critical incident - triggering escalation');
  }
}
```

## Available Types

### Core Types

- **Incident** - Main incident record
- **RawEvent** - Raw event from source systems
- **IncidentEvent** - Normalized incident event
- **Severity** - P0, P1, P2, P3, P4
- **IncidentStatus** - NEW, ACKNOWLEDGED, IN_PROGRESS, ESCALATED, RESOLVED, CLOSED
- **Category** - performance, security, availability, compliance, cost, other

### LLM Integration Types

- **LLMRequest** - Base LLM request structure
- **LLMResponse** - LLM response structure
- **LLMError** - Error handling
- **SentinelLLMConfig** - Sentinel monitoring configuration
- **ShieldLLMConfig** - Shield security configuration
- **EdgeAgentLLMConfig** - Edge-Agent configuration
- **GovernanceLLMConfig** - Governance configuration

### Policy & Workflow Types

- **EscalationPolicy** - Escalation policy configuration
- **EscalationLevel** - Individual escalation level
- **NotificationTemplate** - Notification templates
- **RoutingRule** - Incident routing rules
- **Playbook** - Automated playbook definitions

### User & Team Types

- **User** - User model
- **Team** - Team model
- **OnCallSchedule** - On-call schedule
- **TeamMetrics** - Team performance metrics

### Analytics Types

- **IncidentAnalytics** - Aggregated incident metrics
- **TeamMetrics** - Team performance data
- **PostMortem** - Post-mortem documents

### Integration Types

- **Integration** - Integration configuration
- **IntegrationType** - Supported integrations
- **RetryPolicy** - Retry configuration

### API Types

- **CreateIncidentRequest** - API request to create incident
- **CreateIncidentResponse** - API response
- **UpdateIncidentRequest** - Update incident request
- **QueryIncidentsRequest** - Query incidents with filters

## Type Categories

All types are organized into logical modules:

```typescript
// Core incident types
import { Incident, RawEvent, IncidentEvent } from '@llm-dev-ops/incident-manager-types';

// Data model types
import {
  EscalationPolicy,
  NotificationTemplate,
  RoutingRule
} from '@llm-dev-ops/incident-manager-types';

// LLM client types
import {
  LLMRequest,
  LLMResponse,
  SentinelLLMConfig,
  ShieldLLMConfig
} from '@llm-dev-ops/incident-manager-types';
```

## Features

✅ **Complete Type Coverage** - 2,400+ lines of TypeScript definitions
✅ **LLM Integration Types** - Types for Sentinel, Shield, Edge-Agent, Governance
✅ **Strict Type Safety** - Compiled with strict mode enabled
✅ **Zero Dependencies** - Pure TypeScript definitions
✅ **Tree-Shakeable** - ES modules for optimal bundle size
✅ **Documentation** - Comprehensive JSDoc comments

## Related Packages

- **[@llm-dev-ops/llm-incident-manager](https://www.npmjs.com/package/@llm-dev-ops/llm-incident-manager)** - Main Rust server with npm CLI
- **[@llm-dev-ops/incident-manager-client](https://www.npmjs.com/package/@llm-dev-ops/incident-manager-client)** - TypeScript/JavaScript client SDK

## Documentation

For complete documentation, see the [LLM Incident Manager repository](https://github.com/globalbusinessadvisors/llm-incident-manager).

## License

MIT OR Apache-2.0

## Version

Current version: 1.0.1 (matches main package version)
