import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
const [base,candidate,tools,output]=process.argv.slice(2).map(x=>resolve(x));
const fw=await import(pathToFileURL(`${tools}/node_modules/@metaharness/flywheel/dist/index.js`));
assert.equal(JSON.parse(readFileSync(`${tools}/node_modules/@metaharness/flywheel/package.json`)).version,'0.1.12');
const path='packages/ruvector-client/src/telemetry.ts',hash=b=>createHash('sha256').update(b).digest('hex');
const bytes=r=>readFileSync(`${r}/${path}`),parentHash=hash(bytes(base)),candidateHash=hash(bytes(candidate));
const trials=[];
function measure(root){
  const run=spawnSync(process.execPath,[new URL('./probe-telemetry.mjs',import.meta.url).pathname,root],{encoding:'utf8',timeout:8000,env:{PATH:process.env.PATH}});
  assert.equal(run.status,0,run.stderr);return JSON.parse(run.stdout);
}
for(let i=0;i<5;i++)for(const root of i%2?[candidate,base]:[base,candidate])trials.push({trial:i,root,report:measure(root)});
const measurements=[];
const fingerprint=fw.gateFingerprint(fw.meetsPromotionRule);
const result=await fw.runFlywheelGenerations({
  rootPolicy:{revision:parentHash},
  proposer:async()=>({value:candidateHash,summary:'Retain native HTTP failure batches',inverse:{path,parentBytes:bytes(base).toString('utf8'),parentSha256:parentHash}}),
  evaluator:async(policy,suite)=>{
    const root=policy.revision===parentHash?base:candidate;
    assert.equal(hash(bytes(root)),policy.revision);
    const report=measure(root),items=suite.id==='holdout'?report.holdout:report.anchors;
    const wins=items.map(x=>x.passed);
    // Fail closed: repository Rust/security/orchestration gates did not pass.
    const score={primary:wins.filter(Boolean).length/wins.length,itemWins:wins,noopRate:0,costPerWin:0,regressed:true};
    measurements.push({revision:policy.revision,suite:suite.id,report,score});return score;
  },
  holdout:{id:'holdout',items:[400,401,422,429,500,503]},anchor:{id:'anchor',items:[200,204,'disabled']},
  promotionRule:fw.meetsPromotionRule,maxGenerations:1,signer:fw.makeSigner(),dataSource:'LIVE_NATIVE_HTTP_COMPILED_TYPESCRIPT'
});
const replay=fw.verifyReplayBundle(result.replayBundle,{pinnedGateFingerprint:fingerprint});
assert.equal(replay.pass,true);assert.equal(result.promotions.length,0);
const evidence={parentHash,candidateHash,contract:JSON.parse(readFileSync(new URL('../docs/metaharness/2026-11-07/contract.json',import.meta.url))),contractSha256:hash(readFileSync(new URL('../docs/metaharness/2026-11-07/contract.json',import.meta.url))),probeSha256:hash(readFileSync(new URL('./probe-telemetry.mjs',import.meta.url))),trials,measurements,promotions:result.promotions.length,fingerprint,replay,replayBundle:result.replayBundle,inverse:{path,parentBytes:bytes(base).toString('utf8'),parentSha256:parentHash},boundary:'No full-gate promotion. regressed=true is a fail-closed repository veto, not a newly observed component regression. Zero cost inputs are neutral, not measured savings. Root-only replay is not candidate promotion proof. Rollback restores source bytes, never deletes service memory.'};
writeFileSync(output,JSON.stringify(evidence,null,2)+'\n');
console.log(JSON.stringify({trials:trials.map(x=>({trial:x.trial,root:x.root,holdout:x.report.holdout.filter(y=>y.passed).length,anchors:x.report.anchors.filter(y=>y.passed).length})),promotions:evidence.promotions,replay}));
