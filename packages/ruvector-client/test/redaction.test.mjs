import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {test} from 'node:test';
import {fileURLToPath} from 'node:url';

test('native DecisionEvent redaction preserves inputs and own-path boundaries', {timeout:7000}, () => {
  const root=fileURLToPath(new URL('../../../',import.meta.url));
  const run=spawnSync(process.execPath,[`${root}/scripts/probe-redaction.mjs`,root],{
    encoding:'utf8',timeout:5000,maxBuffer:1024*1024,env:{PATH:process.env.PATH}
  });
  assert.equal(run.status,0,JSON.stringify({stdout:run.stdout,stderr:run.stderr,signal:run.signal}));
  const result=JSON.parse(run.stdout);
  assert.equal(result.holdout.length,4);
  assert.equal(result.anchors.length,3);
  for(const check of [...result.holdout,...result.anchors])assert.equal(check.passed,true,JSON.stringify(check));
});
