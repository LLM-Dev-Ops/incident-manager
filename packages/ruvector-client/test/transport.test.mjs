import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { contract, probeTransport } from '../../../scripts/probe-transport.mjs';

const root = fileURLToPath(new URL('../../../', import.meta.url));
for (const name of [...contract.holdout, ...contract.anchor]) {
  test(`native RuVector transport: ${name}`, { timeout: 5000 }, () => {
    const [result] = probeTransport(root, [name]);
    assert.equal(result.pass, true, JSON.stringify(result));
  });
}
