import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {test} from 'node:test';
import {fileURLToPath} from 'node:url';

test('native HTTP telemetry failures retain ordered batches without duplicating successful delivery', {timeout:10000},()=>{
  const root=fileURLToPath(new URL('../../../',import.meta.url));
  const run=spawnSync(process.execPath,[`${root}/scripts/probe-telemetry.mjs`,root],{encoding:'utf8',timeout:8000,env:{PATH:process.env.PATH}});
  assert.equal(run.status,0,JSON.stringify({stderr:run.stderr,signal:run.signal}));
  const result=JSON.parse(run.stdout);
  assert.equal(result.holdout.length,6);assert.equal(result.anchors.length,3);
  for(const check of [...result.holdout,...result.anchors])assert.equal(check.passed,true,JSON.stringify(check));
});
