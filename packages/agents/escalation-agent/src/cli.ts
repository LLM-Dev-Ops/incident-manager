#!/usr/bin/env node
/**
 * Incident Escalation Agent CLI
 *
 * CLI-invokable endpoint for the Escalation Agent.
 *
 * Commands:
 *   escalate evaluate <incident_id>  - Evaluate an incident for escalation
 *   escalate inspect <incident_id>   - Inspect escalation state
 *   escalate list                    - List active escalations
 */

import { parseArgs } from 'util';
import { createAgentConfig, validateConfig } from './config.js';
import { EscalationDecisionEngine } from './engine.js';
import { handleRequest } from './handler.js';
import {
  RuvectorClient,
  TelemetryEmitter,
  createNoOpTelemetryEmitter
} from '@agentics/ruvector-client';
import type {
  EscalationAgentInput,
  Severity,
  Environment
} from '@agentics/contracts';

// ============================================================================
// CLI TYPES
// ============================================================================

interface CLIOptions {
  dryRun: boolean;
  verbose: boolean;
  json: boolean;
  signalSource?: string;
  policyId?: string;
  includeHistory?: boolean;
  severity?: Severity;
  status?: string;
  limit?: number;
}

// ============================================================================
// MAIN CLI
// ============================================================================

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
    printUsage();
    process.exit(0);
  }

  const command = args[0];
  const subArgs = args.slice(1);

  try {
    switch (command) {
      case 'evaluate':
        await evaluateCommand(subArgs);
        break;
      case 'inspect':
        await inspectCommand(subArgs);
        break;
      case 'list':
        await listCommand(subArgs);
        break;
      case 'version':
        console.log('1.0.0');
        break;
      default:
        console.error(`Unknown command: ${command}`);
        printUsage();
        process.exit(1);
    }
  } catch (error) {
    const err = error as Error;
    console.error(`Error: ${err.message}`);
    if (process.env.DEBUG === 'true') {
      console.error(err.stack);
    }
    process.exit(1);
  }
}

// ============================================================================
// COMMANDS
// ============================================================================

async function evaluateCommand(args: string[]): Promise<void> {
  const { values, positionals } = parseArgs({
    args,
    options: {
      'dry-run': { type: 'boolean', default: false },
      'verbose': { type: 'boolean', short: 'v', default: false },
      'json': { type: 'boolean', default: false },
      'signal-source': { type: 'string', default: 'manual' },
      'policy-id': { type: 'string' }
    },
    allowPositionals: true
  });

  if (positionals.length === 0) {
    console.error('Error: incident_id is required');
    console.error('Usage: escalate evaluate <incident_id> [options]');
    process.exit(1);
  }

  const incidentId = positionals[0];
  const options: CLIOptions = {
    dryRun: values['dry-run'] as boolean,
    verbose: values['verbose'] as boolean,
    json: values['json'] as boolean,
    signalSource: values['signal-source'] as string,
    policyId: values['policy-id'] as string | undefined
  };

  const config = createAgentConfig();
  const configValidation = validateConfig(config);
  if (!configValidation.valid) {
    console.error('Configuration error:', configValidation.errors.join(', '));
    process.exit(1);
  }

  // Initialize ruvector client to fetch incident state
  const ruvectorClient = new RuvectorClient({
    baseUrl: config.ruvector.baseUrl,
    apiKey: config.ruvector.apiKey,
    timeoutMs: config.ruvector.timeoutMs
  });

  // Fetch incident state
  const incidentResult = await ruvectorClient.getIncidentState(incidentId);
  if (!incidentResult.success) {
    console.error(`Failed to fetch incident ${incidentId}: ${incidentResult.error.message}`);
    process.exit(1);
  }

  const incident = incidentResult.data;

  // Build input
  const input: EscalationAgentInput = {
    incident_id: incidentId,
    fingerprint: `manual-eval-${incidentId}`,
    current_severity: incident.severity as Severity,
    current_status: incident.status as any,
    current_escalation_level: incident.escalation_level,
    category: 'other',
    signal_source: options.signalSource as any ?? 'manual',
    signal_timestamp: new Date().toISOString(),
    signal_payload: {
      type: 'manual'
    },
    environment: config.environment as Environment,
    title: `Incident ${incidentId}`,
    description: 'Manual evaluation requested via CLI',
    affected_resource: {
      type: 'service',
      id: 'unknown',
      name: 'Unknown'
    },
    tags: {},
    incident_created_at: incident.updated_at, // Approximate
    incident_updated_at: incident.updated_at,
    escalation_history: [],
    sla: {
      acknowledgment_breached: false,
      resolution_breached: false
    },
    execution_id: crypto.randomUUID(),
    policy_id: options.policyId
  };

  if (options.verbose) {
    console.log('Input:', JSON.stringify(input, null, 2));
  }

  // Execute
  if (options.dryRun) {
    // Dry run - just evaluate, don't persist
    const engine = new EscalationDecisionEngine(config);
    const result = engine.evaluate(input);

    if (options.json) {
      console.log(JSON.stringify({
        dry_run: true,
        output: result.output,
        confidence: result.confidence,
        processing_time_ms: result.processingTimeMs
      }, null, 2));
    } else {
      printEvaluationResult(result.output, result.confidence, true);
    }
  } else {
    // Full execution with persistence
    const telemetry = config.telemetry.enabled ?
      new TelemetryEmitter({
        endpoint: config.telemetry.endpoint,
        apiKey: config.telemetry.apiKey,
        serviceName: 'escalation-agent-cli',
        environment: config.environment,
        enabled: true
      }) :
      createNoOpTelemetryEmitter();

    try {
      const result = await handleRequest(input, config, input.execution_id, telemetry);

      if (options.json) {
        console.log(JSON.stringify({
          dry_run: false,
          decision_event: result.decision_event,
          persisted: result.persisted,
          warnings: result.warnings
        }, null, 2));
      } else {
        printEvaluationResult(
          result.decision_event.outputs,
          result.decision_event.confidence,
          false
        );
        if (result.warnings.length > 0) {
          console.log('\nWarnings:');
          result.warnings.forEach(w => console.log(`  - ${w}`));
        }
        console.log(`\nPersisted: ${result.persisted ? 'Yes' : 'No'}`);
      }
    } finally {
      await telemetry.shutdown();
    }
  }
}

async function inspectCommand(args: string[]): Promise<void> {
  const { values, positionals } = parseArgs({
    args,
    options: {
      'include-history': { type: 'boolean', default: false },
      'json': { type: 'boolean', default: false }
    },
    allowPositionals: true
  });

  if (positionals.length === 0) {
    console.error('Error: incident_id is required');
    console.error('Usage: escalate inspect <incident_id> [options]');
    process.exit(1);
  }

  const incidentId = positionals[0];
  const options: CLIOptions = {
    dryRun: false,
    verbose: false,
    json: values['json'] as boolean,
    includeHistory: values['include-history'] as boolean
  };

  const config = createAgentConfig();
  const ruvectorClient = new RuvectorClient({
    baseUrl: config.ruvector.baseUrl,
    apiKey: config.ruvector.apiKey,
    timeoutMs: config.ruvector.timeoutMs
  });

  // Fetch escalation state
  const stateResult = await ruvectorClient.getEscalationState(incidentId);
  if (!stateResult.success) {
    console.error(`Failed to fetch escalation state: ${stateResult.error.message}`);
    process.exit(1);
  }

  const state = stateResult.data;

  if (!state) {
    console.log(`No escalation state found for incident ${incidentId}`);
    process.exit(0);
  }

  if (options.json) {
    console.log(JSON.stringify(state, null, 2));
  } else {
    console.log(`Escalation State for Incident: ${incidentId}`);
    console.log('─'.repeat(50));
    console.log(`Status:          ${state.status}`);
    console.log(`Current Level:   ${state.current_level}`);
    console.log(`Policy ID:       ${state.policy_id}`);
    console.log(`Started At:      ${state.started_at}`);
    console.log(`Level Reached:   ${state.level_reached_at}`);
    console.log(`Acknowledged:    ${state.acknowledged ? 'Yes' : 'No'}`);
    if (state.acknowledged_by) {
      console.log(`Acknowledged By: ${state.acknowledged_by}`);
    }
    if (state.next_escalation_at) {
      console.log(`Next Escalation: ${state.next_escalation_at}`);
    }
    console.log(`Repeat Count:    ${state.repeat_count}`);

    if (options.includeHistory && state.notification_history.length > 0) {
      console.log('\nNotification History:');
      for (const notif of state.notification_history) {
        console.log(`  - [${notif.sent_at}] Level ${notif.level} → ${notif.target} via ${notif.channel} (${notif.success ? 'success' : 'failed'})`);
      }
    }
  }
}

async function listCommand(args: string[]): Promise<void> {
  const { values } = parseArgs({
    args,
    options: {
      'severity': { type: 'string' },
      'status': { type: 'string' },
      'limit': { type: 'string', default: '20' },
      'json': { type: 'boolean', default: false }
    }
  });

  const options: CLIOptions = {
    dryRun: false,
    verbose: false,
    json: values['json'] as boolean,
    severity: values['severity'] as Severity | undefined,
    status: values['status'] as string | undefined,
    limit: parseInt(values['limit'] as string, 10)
  };

  const config = createAgentConfig();
  const ruvectorClient = new RuvectorClient({
    baseUrl: config.ruvector.baseUrl,
    apiKey: config.ruvector.apiKey,
    timeoutMs: config.ruvector.timeoutMs
  });

  // Note: This would need a query endpoint in ruvector-service
  // For now, we'll show a placeholder message
  console.log('Listing active escalations...');
  console.log('(This feature requires ruvector-service escalation query endpoint)');

  if (options.json) {
    console.log(JSON.stringify({ escalations: [], total: 0 }, null, 2));
  } else {
    console.log('\nNo active escalations found or query endpoint not available.');
  }
}

// ============================================================================
// HELPERS
// ============================================================================

function printUsage(): void {
  console.log(`
Incident Escalation Agent CLI

Usage:
  escalate <command> [options]

Commands:
  evaluate <incident_id>    Evaluate an incident for escalation
  inspect <incident_id>     Inspect escalation state
  list                      List active escalations
  version                   Show version

Evaluate Options:
  --dry-run                 Run evaluation without persisting
  -v, --verbose             Verbose output
  --json                    Output as JSON
  --signal-source <source>  Signal source (default: manual)
  --policy-id <id>          Specific policy to apply

Inspect Options:
  --include-history         Include notification history
  --json                    Output as JSON

List Options:
  --severity <level>        Filter by severity (P0-P4)
  --status <status>         Filter by status
  --limit <n>               Limit results (default: 20)
  --json                    Output as JSON

Environment Variables:
  RUVECTOR_BASE_URL         ruvector-service base URL
  RUVECTOR_API_KEY          API key for authentication
  ENVIRONMENT               Environment (production/staging/development/qa)
  DEBUG                     Enable debug output
`);
}

function printEvaluationResult(
  output: any,
  confidence: number,
  dryRun: boolean
): void {
  const decisionColors: Record<string, string> = {
    'escalate': '\x1b[31m',    // Red
    'deescalate': '\x1b[32m',  // Green
    'maintain': '\x1b[33m',    // Yellow
    'defer': '\x1b[36m'        // Cyan
  };
  const reset = '\x1b[0m';

  console.log('\nEscalation Evaluation Result');
  console.log('─'.repeat(50));
  console.log(`Mode:       ${dryRun ? 'DRY RUN' : 'LIVE'}`);
  console.log(`Decision:   ${decisionColors[output.decision] ?? ''}${output.decision.toUpperCase()}${reset}`);
  console.log(`Confidence: ${(confidence * 100).toFixed(1)}%`);
  console.log(`Reason:     ${output.reason}`);

  if (output.new_severity) {
    console.log(`\nSeverity Change:`);
    console.log(`  New Severity: ${output.new_severity}`);
    console.log(`  New Level:    ${output.new_escalation_level}`);
    console.log(`  Delta:        ${output.severity_delta}`);
  }

  if (output.orchestrator_actions && output.orchestrator_actions.length > 0) {
    console.log(`\nOrchestrator Actions (${output.orchestrator_actions.length}):`);
    for (const action of output.orchestrator_actions) {
      console.log(`  - [${action.priority}] ${action.action_type}${action.async ? ' (async)' : ''}`);
    }
  }

  if (output.next_evaluation_at) {
    console.log(`\nNext Evaluation: ${output.next_evaluation_at}`);
  }

  if (output.defer_until) {
    console.log(`Deferred Until:  ${output.defer_until}`);
  }

  console.log('\nPolicy Applied:');
  console.log(`  ID:       ${output.applied_policy.policy_id}`);
  console.log(`  Name:     ${output.applied_policy.policy_name}`);
  console.log(`  Version:  ${output.applied_policy.policy_version}`);
  console.log(`  Max Level: ${output.applied_policy.max_level}`);
}

// ============================================================================
// ENTRY POINT
// ============================================================================

main().catch(console.error);
