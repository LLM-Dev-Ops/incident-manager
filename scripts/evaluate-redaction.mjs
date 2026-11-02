import {spawnSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
// Usage: node evaluate-redaction.mjs BASE CANDIDATE TOOLS OUTPUT
// Tools must contain the separately installed @metaharness/flywheel@0.1.12.
const [base,candidate,tools,out]=process.argv.slice(2).map(x=>resolve(x));
mkdirSync(out,{recursive:true});
const run=(cwd,args)=>{const p=spawnSync(args[0],args.slice(1),{cwd,encoding:'utf8',timeout:90000,maxBuffer:10*1024*1024});return {command:args,cwd,status:p.status,error:p.error?.message,stdout:p.stdout,stderr:p.stderr};};
const results={};
for(const [label,root] of [['baseline',base],['candidate',candidate]]){
 const build=run(root,['npm','run','build','--workspaces','--if-present']);
 const test=run(root,['npm','run','test','--workspaces','--if-present']);
 const audit=run(root,['npm','audit','--omit=dev','--json']);
 const trials=Array.from({length:3},()=>run(root,['node',join(candidate,'scripts/probe-redaction.mjs'),root]));
 results[label]={revision:run(root,['git','rev-parse','HEAD']).stdout.trim(),build,test,audit,trials};
 writeFileSync(join(out,label+'.json'),JSON.stringify(results[label],null,2));
}
if(readFileSync(join(base,'package-lock.json'),'utf8')!==readFileSync(join(candidate,'package-lock.json'),'utf8'))throw Error('Dependency lock changed');
const {runFlywheelGenerations,meetsPromotionRule,makeSigner,verifyReplayBundle,gateFingerprint}=await import(pathToFileURL(join(tools,'node_modules/@metaharness/flywheel/dist/index.js')));
const parent=readFileSync(join(base,'packages/ruvector-client/src/decision-event-store.ts'),'utf8');
const report=await runFlywheelGenerations({
 rootPolicy:{revision:results.baseline.revision},
 proposer:async()=>({value:results.candidate.revision,summary:'Copy-on-write own-property-only redaction; frozen four-case behavioral lift, but full Rust/CI gate remains unverified',inverse:{path:'packages/ruvector-client/src/decision-event-store.ts',parentBytes:parent,hash:createHash('sha256').update(parent).digest('hex')}}),
 evaluator:async(policy,suite)=>{
  const result=policy.revision===results.baseline.revision?results.baseline:policy.revision===results.candidate.revision?results.candidate:null;
  if(!result)throw Error('Unmeasured policy');
  const observations=result.trials.map(t=>JSON.parse(t.stdout));
  const wins=observations.flatMap(t=>t[suite.id].map(x=>x.passed));
  return {primary:wins.filter(Boolean).length/wins.length,noopRate:0,costPerWin:0,regressed:true,itemWins:wins};
  // regressed is intentionally true: frozen acceptance requires full Rust/security/CI.
  // No production latency/cost improvement is claimed; zero costs are neutral, unpriced axes.
 },
 promotionRule:meetsPromotionRule,
 holdout:{id:'holdout',items:Array.from({length:12},(_,i)=>i)},
 anchor:{id:'anchors',items:Array.from({length:9},(_,i)=>i)},
 maxGenerations:1,signer:makeSigner(),dataSource:'LIVE',cacheEvaluations:false,
});
const replay=verifyReplayBundle(report.replayBundle,{pinnedGateFingerprint:gateFingerprint(meetsPromotionRule)});
const evidence={testedRevisions:{baseline:results.baseline.revision,candidate:results.candidate.revision},fullAcceptance:'BLOCKED: Rust dependency resolution and required CI/approvals are not all passing',scope:'Real compiled client and store, native loopback HTTP with synthetic fixtures; not deployed RuVector/CloudSQL validation',results:report.replayBundle,replay};
writeFileSync(join(out,'flywheel.json'),JSON.stringify(evidence,null,2));
console.log(JSON.stringify({baseline:results.baseline.revision,candidate:results.candidate.revision,builds:[results.baseline.build.status,results.candidate.build.status],tests:[results.baseline.test.status,results.candidate.test.status],audits:[results.baseline.audit.status,results.candidate.audit.status],trials:Object.fromEntries(Object.entries(results).map(([k,v])=>[k,v.trials.map(t=>{const x=JSON.parse(t.stdout);return {holdout:x.holdout.filter(c=>c.passed).length,anchors:x.anchors.filter(c=>c.passed).length};})])),promotions:report.replayBundle.verified_improvements,replay}));
if(!replay.pass)process.exitCode=1;
