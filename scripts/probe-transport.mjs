import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const contract = JSON.parse(readFileSync(new URL('../docs/metaharness/2026-10-19/contract.json', import.meta.url)));
const self = fileURLToPath(import.meta.url);

export function probeTransport(root, names = [...contract.holdout, ...contract.anchor]) {
  return names.map(name => {
    const started = performance.now();
    const run = spawnSync(process.execPath, [self, '--worker', resolve(root), name], {
      encoding: 'utf8', timeout: contract.worker_timeout_ms,
      env: { PATH: process.env.PATH }, maxBuffer: 1024 * 1024,
    });
    const line = run.stdout?.trim().split('\n').findLast(x => x.startsWith('{'));
    const report = line ? JSON.parse(line) : null;
    return { name, pass: run.status === 0 && run.signal === null && report?.pass === true,
      exit: run.status, signal: run.signal, error: run.error?.code,
      elapsed_ms: performance.now() - started, report, stderr: run.stderr };
  });
}

async function worker(root, name) {
  assert([...contract.holdout, ...contract.anchor].includes(name));
  const { RuvectorClient } = await import(pathToFileURL(join(root, 'packages/ruvector-client/dist/client.js')));
  const requests = [], timers = [];
  const slow = ['slow-success', 'slow-error', 'partial-success'].includes(name);
  const server = createServer(async (req, res) => {
    let body = '';
    for await (const chunk of req) body += chunk;
    requests.push({ method: req.method, url: req.url, authorization: req.headers.authorization, body });
    if (name === 'network-cleanup') { req.socket.destroy(); return; }
    if (slow) {
      res.writeHead(name === 'slow-error' ? 503 : 200, { 'content-type': 'application/json' });
      res.flushHeaders();
      if (name === 'partial-success') res.write('{"status":');
      timers.push(setTimeout(() => res.end(name === 'partial-success' ? '"healthy"}' : '{"status":"healthy"}'), contract.slow_body_ms));
      return;
    }
    res.setHeader('content-type', 'application/json');
    if (name === 'client-error') { res.writeHead(400); res.end('invalid request'); }
    else if (name === 'rate-limit') { res.writeHead(429); res.end('rate limited'); }
    else if (name === 'server-retry' && requests.length === 1) { res.writeHead(503); res.end('retry'); }
    else if (name === 'invalid-json') { res.end('{invalid'); }
    else if (name === 'append-once') { res.end('{"id":"event-fixture"}'); }
    else if (name === 'patch-state') { res.end('{"severity":"P1"}'); }
    else { res.end('{"status":"healthy"}'); }
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const client = new RuvectorClient({ baseUrl: `http://127.0.0.1:${server.address().port}`,
    apiKey: 'synthetic-test-only', timeoutMs: slow ? contract.timeout_ms : 30000,
    retries: ['server-retry', 'rate-limit', 'client-error'].includes(name) ? 2 : 1 });
  let result, error;
  const started = performance.now();
  try {
    if (name === 'append-once') result = await client.storeDecisionEvent({ id: 'event-fixture', payload: 'synthetic' });
    else if (name === 'patch-state') result = await client.updateIncidentState('incident-fixture', { severity: 'P1' });
    else result = await client.health();
  } catch (err) { error = String(err); }
  finally {
    for (const timer of timers) clearTimeout(timer);
    server.closeAllConnections();
    await new Promise(r => server.close(r));
  }
  const elapsed_ms = performance.now() - started;
  let pass = !error;
  if (slow) pass &&= !result.success && result.error.code === 'TIMEOUT' && result.error.retryable === true && elapsed_ms < contract.maximum_timeout_completion_ms && requests.length === 1;
  else if (name === 'network-cleanup' || name === 'invalid-json') pass &&= !result.success && result.error.code === 'NETWORK_ERROR' && requests.length === 1;
  else if (name === 'client-error' || name === 'rate-limit') pass &&= !result.success && result.error.code === 'REQUEST_FAILED' && result.error.details.statusCode === (name === 'client-error' ? 400 : 429) && result.error.retryable === (name === 'rate-limit') && requests.length === (name === 'client-error' ? 1 : 2);
  else {
    pass &&= result.success === true;
    if (name === 'append-once') pass &&= result.data.id === 'event-fixture' && requests.length === 1 && requests[0].method === 'POST' && requests[0].url === '/api/v1/decision-events' && JSON.parse(requests[0].body).id === 'event-fixture';
    else if (name === 'patch-state') pass &&= result.data.severity === 'P1' && requests[0].method === 'PATCH' && requests[0].url === '/api/v1/incidents/incident-fixture/state' && requests[0].body === '{"severity":"P1"}';
    else pass &&= result.data.status === 'healthy' && requests.length === (name === 'server-retry' ? 2 : 1);
  }
  pass &&= requests.every(r => r.authorization === 'Bearer synthetic-test-only');
  console.log(JSON.stringify({ name, pass, elapsed_ms, result, error, requests }));
  // Do not call process.exit: a leaked client timer must keep this child alive
  // and fail the parent's fixed watchdog, even after the request returns.
}

if (process.argv[1] === self) {
  if (process.argv[2] === '--worker') await worker(resolve(process.argv[3]), process.argv[4]);
  else console.log(JSON.stringify(probeTransport(process.argv[2] || process.cwd()), null, 2));
}
