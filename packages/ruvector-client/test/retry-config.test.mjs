import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

test('non-finite retry configuration preserves one-attempt fallback', { timeout: 4000 }, () => {
  const clientUrl = new URL('../dist/client.js', import.meta.url).href;
  const run = spawnSync(process.execPath, ['--input-type=module', '-e', `
    import { createServer } from 'node:http';
    import { RuvectorClient } from ${JSON.stringify(clientUrl)};
    let calls = 0;
    const server = createServer((req, res) => { calls++; res.writeHead(503); res.end('synthetic'); });
    await new Promise(r => server.listen(0, '127.0.0.1', r));
    try {
      const client = new RuvectorClient({ baseUrl: 'http://127.0.0.1:' + server.address().port,
        apiKey: 'synthetic-test-only', retries: NaN, timeoutMs: 30000 });
      const result = await client.health();
      console.log(JSON.stringify({ calls, code: result.error?.code }));
    } finally { server.closeAllConnections(); await new Promise(r => server.close(r)); }
  `], { encoding: 'utf8', timeout: 1800, env: { PATH: process.env.PATH } });
  assert.equal(run.status, 0, JSON.stringify({ signal: run.signal, error: run.error?.code, stderr: run.stderr }));
  assert.deepEqual(JSON.parse(run.stdout), { calls: 1, code: 'REQUEST_FAILED' });
});
