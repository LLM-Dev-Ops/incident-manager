import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { createNoOpTelemetryEmitter } from '@agentics/ruvector-client';

import { createAgentConfig, validateConfig } from '../config.js';
import { verifyRegistration } from '../registration.js';

describe('workspace contract', () => {
  it('exposes a usable no-op telemetry emitter from the client package', async () => {
    const telemetry = createNoOpTelemetryEmitter();

    telemetry.emitMetrics(
      'incident-escalation-agent',
      '550e8400-e29b-41d4-a716-446655440000',
      { evaluation_count: 1 }
    );

    await telemetry.shutdown();
  });

  it('keeps registration and explicit configuration overrides valid', () => {
    assert.deepEqual(verifyRegistration(), { valid: true, errors: [] });

    const config = createAgentConfig({
      agentId: 'incident-escalation:test',
      environment: 'qa',
      ruvector: {
        baseUrl: 'https://ruvector.internal',
        apiKey: 'test-only',
        timeoutMs: 1_000
      },
      telemetry: {
        endpoint: 'https://observatory.internal',
        enabled: false
      },
      escalation: {
        maxEscalationLevel: 5,
        minEscalationInterval: 300,
        autoEscalationConfidenceThreshold: 0.8,
        requireApprovalForP0: true
      },
      debug: false
    });

    assert.deepEqual(validateConfig(config), { valid: true, errors: [] });
    assert.equal(config.environment, 'qa');
    assert.equal(config.ruvector.baseUrl, 'https://ruvector.internal');
  });
});
