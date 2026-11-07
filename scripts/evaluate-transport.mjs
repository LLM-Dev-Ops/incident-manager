import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { contract, probeTransport } from './probe-transport.mjs';

// node scripts/evaluate-transport.mjs BASELINE CANDIDATE PINNED_TOOLS OUTPUT
const [baseline, candidate, tools, output] = process.argv.slice(2).map(x => resolve(x));
const hash = b => createHash('sha256').update(b).digest('hex');
const fw = await import(pathToFileURL(join(tools, 'node_modules/@metaharness/flywheel/dist/index.js')));
assert.equal(JSON.parse(readFileSync(join(tools, 'node_modules/@metaharness/flywheel/package.json'))).version, '0.1.12');
const source = 'packages/ruvector-client/src/client.ts';
const bytes = root => readFileSync(join(root, source));
const parentHash = hash(bytes(baseline)), candidateHash = hash(bytes(candidate));
assert.notEqual(parentHash, candidateHash);
const measurements = [];
let retainedNames;

function regression(root) {
  const vitest = spawnSync(process.execPath, [join(root, 'node_modules/vitest/vitest.mjs'), 'run', '--reporter=json'], {
    cwd: join(root, 'packages/agentics-contracts'), encoding: 'utf8', timeout: 30000,
    env: { PATH: process.env.PATH },
  });
  assert.equal(vitest.status, 0, vitest.stderr); assert.equal(vitest.signal, null);
  const report = JSON.parse(vitest.stdout.slice(vitest.stdout.indexOf('{')));
  const assertions = report.testResults.flatMap(x => x.assertionResults);
  assert.equal(assertions.length, 104); assert(assertions.every(x => x.status === 'passed'));
  const names = assertions.map(x => x.fullName).sort();
  const node = spawnSync(process.execPath, ['--test', '--test-reporter=tap', 'dist/__tests__/workspace-contract.test.js'], {
    cwd: join(root, 'packages/agents/escalation-agent'), encoding: 'utf8', timeout: 30000,
    env: { PATH: process.env.PATH },
  });
  assert.equal(node.status, 0); assert.equal(node.signal, null);
  assert.match(node.stdout, /# tests 2\b/); assert.match(node.stdout, /# fail 0\b/); assert.match(node.stdout, /# skipped 0\b/);
  names.push(...node.stdout.split('\n').filter(x => x.includes('# Subtest:')).map(x => x.trim()).sort());
  if (root === baseline) retainedNames = names;
  assert.deepEqual(names, retainedNames);
  return { vitestExit: vitest.status, nodeExit: node.status, assertions: 106, names, nodeOutput: node.stdout };
}

async function evaluator(policy, suite) {
  const root = policy.transport === parentHash ? baseline : candidate;
  assert.equal(hash(bytes(root)), policy.transport);
  const report = probeTransport(root, suite.id === 'holdout' ? contract.holdout : contract.anchor);
  const retained = suite.id === 'anchor' ? regression(root) : undefined;
  const wins = report.map(x => x.pass);
  if (retained) wins.push(true);
  const primary = wins.filter(Boolean).length / wins.length;
  const score = { primary, noopRate: 1 - primary, costPerWin: 0, regressed: suite.id === 'anchor' && wins.some(x => !x), itemWins: wins };
  measurements.push({ source: policy.transport, suite: suite.id, report, retained, score });
  assert.equal(hash(bytes(root)), policy.transport);
  return score;
}

const fingerprint = fw.gateFingerprint(fw.meetsPromotionRule);
const result = await fw.runFlywheelGenerations({
  rootPolicy: { transport: parentHash },
  proposer: async () => ({ value: candidateHash, summary: 'Keep per-attempt deadlines through response bodies and clear timers before retry backoff',
    inverse: { path: source, parentBytes: bytes(baseline).toString('utf8'), parentSha256: parentHash } }),
  evaluator, holdout: { id: 'holdout', items: contract.holdout },
  anchor: { id: 'anchor', items: [...contract.anchor, 'all 106 original workspace tests retained'] },
  maxGenerations: 1, signer: fw.makeSigner(), promotionRule: fw.meetsPromotionRule, dataSource: 'LIVE_LOOPBACK_HTTP_CHILD_PROCESSES_AND_RETAINED_WORKSPACE_TESTS',
});
const replay = fw.verifyReplayBundle(result.replayBundle, { pinnedGateFingerprint: fingerprint });
assert.equal(replay.pass, true); assert.equal(result.promotions.length, 1);
const evidence = { contract, contractSha256: hash(readFileSync(new URL('../docs/metaharness/2026-10-19/contract.json', import.meta.url))),
  probeSha256: hash(readFileSync(new URL('./probe-transport.mjs', import.meta.url))), parentHash, candidateHash,
  measurements, promotions: result.promotions.length, fingerprint, replay, replayBundle: result.replayBundle,
  boundary: 'Component transport promotion only, not repository merge approval. Real native fetch over ephemeral loopback HTTP and real child process exit/watchdog measurements; no fetch mocks. Cost/energy unmeasured: costPerWin=0 is neutral input. Inverse bytes enable source rollback, not deletion of append-only service data. No deployment or production service writes.' };
writeFileSync(output, JSON.stringify(evidence, null, 2) + '\n');
console.log(JSON.stringify({ promotions: evidence.promotions, replay,
  measurements: measurements.map(x => ({ suite: x.suite, primary: x.score.primary, passes: x.report.filter(y => y.pass).length, retained: x.retained?.assertions })) }));
