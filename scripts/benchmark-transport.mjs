import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { contract, probeTransport } from './probe-transport.mjs';

const [baseline, candidate, output] = process.argv.slice(2).map(x => resolve(x));
const measurements = [];
for (let trial = 0; trial < 5; trial++) {
  // Alternate ordering; retain every sample, including watchdog timeouts.
  for (const [revision, root] of trial % 2 ? [['candidate', candidate], ['baseline', baseline]] : [['baseline', baseline], ['candidate', candidate]]) {
    measurements.push({ trial, revision, results: probeTransport(root, contract.holdout) });
  }
}
const summary = ['baseline', 'candidate'].map(revision => {
  const runs = measurements.filter(x => x.revision === revision);
  const byCase = contract.holdout.map(name => {
    const results = runs.map(x => x.results.find(y => y.name === name));
    const times = results.map(x => x.elapsed_ms).sort((a, b) => a - b);
    return { name, passes: results.filter(x => x.pass).length, trials: 5, workerMedianMs: times[2],
      watchdogTimeouts: results.filter(x => x.error === 'ETIMEDOUT').length };
  });
  return { revision, passes: runs.flatMap(x => x.results).filter(x => x.pass).length, total: 20, byCase };
});
assert.equal(summary[1].passes, 20);
const evidence = { contract: contract.id, measurements, summary,
  interpretation: 'Worker elapsed includes Node startup/HTTP fixture teardown. Deadline response elapsed is separately retained in each report. The baseline network-cleanup samples are right-censored by the 2000ms watchdog, not successful completion latencies. Synthetic loopback reliability/latency only; not production throughput or cost.' };
writeFileSync(output, JSON.stringify(evidence, null, 2) + '\n');
console.log(JSON.stringify(summary));
