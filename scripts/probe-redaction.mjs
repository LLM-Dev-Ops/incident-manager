import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {pathToFileURL} from 'node:url';
import {resolve,join} from 'node:path';
const root=resolve(process.argv[2]||'.');
const {DecisionEventStore}=await import(pathToFileURL(join(root,'packages/ruvector-client/dist/decision-event-store.js')));
const {RuvectorClient}=await import(pathToFileURL(join(root,'packages/ruvector-client/dist/client.js')));
const received=[];
const server=createServer(async(req,res)=>{let raw='';for await(const part of req)raw+=part;received.push({url:req.url,method:req.method,auth:req.headers.authorization,body:JSON.parse(raw)});res.setHeader('content-type','application/json');res.end('{"id":"redaction-fixture"}');});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const store=new DecisionEventStore(new RuvectorClient({baseUrl:`http://127.0.0.1:${server.address().port}`,apiKey:'synthetic-test-only',retries:1,timeoutMs:1000}));
const event=outputs=>({id:'fixture',agent_id:'fixture-agent',agent_version:'1.0.0',agent_classification:'core',decision_type:'incident_open',inputs_hash:'synthetic',outputs,confidence:0.9,constraints_applied:{},execution_ref:'fixture-run',timestamp:'2026-11-02T00:00:00Z',environment:'test',requires_review:false,trace_id:'transient'});
const spec=exclude=>({persist:[],exclude,ttl_seconds:0});
const results={holdout:[],anchors:[]};
async function check(suite,name,fn){try{await fn();results[suite].push({name,passed:true});}catch(error){results[suite].push({name,passed:false,error:error.message});}finally{delete Object.prototype.redactionProbe;}}
async function save(input,policy){const count=received.length,result=await store.store(input,policy);assert.equal(result.success,true);assert.equal(received.length,count+1);const request=received.at(-1);assert.equal(request.url,'/api/v1/decision-events');assert.equal(request.auth,'Bearer synthetic-test-only');return request.body;}
try{
 await check('holdout','nested redaction preserves caller event',async()=>{const input=event({secret:'sensitive-fixture',keep:1}),before=JSON.stringify(input);const sent=await save(input,spec(['outputs.secret']));assert.equal(sent.outputs.secret,undefined);assert.equal(sent.outputs.keep,1);assert.equal(JSON.stringify(input),before);});
 await check('holdout','three-level exclusion preserves sibling fields',async()=>{const input=event({nested:{secret:'sensitive-fixture',keep:2}});const sent=await save(input,spec(['outputs.nested.secret']));assert.deepEqual(sent.outputs,{nested:{keep:2}});assert.equal(input.outputs.nested.secret,'sensitive-fixture');});
 await check('holdout','inherited exclusion path never mutates Object.prototype',async()=>{Object.prototype.redactionProbe='retained';await save(event({keep:1}),spec(['__proto__.redactionProbe']));assert.equal(Object.prototype.redactionProbe,'retained');});
 await check('holdout','array path excludes only requested leaf',async()=>{const input=event({rows:[{secret:'sensitive-fixture',keep:3}]}),before=JSON.stringify(input);const sent=await save(input,spec(['outputs.rows.0.secret']));assert.deepEqual(sent.outputs.rows,[{keep:3}]);assert.equal(JSON.stringify(input),before);});
 await check('anchors','default top-level policy remains append-once',async()=>{const input=event({keep:true}),before=JSON.stringify(input);const sent=await save(input);assert.equal(sent.trace_id,undefined);assert.equal(sent.outputs.keep,true);assert.equal(JSON.stringify(input),before);});
 await check('anchors','unknown exclusion is a no-op',async()=>{const input=event({keep:7});assert.deepEqual(await save(input,spec(['outputs.absent.leaf'])),input);});
 await check('anchors','invalid event is rejected without HTTP',async()=>{const count=received.length,result=await store.store({});assert.equal(result.success,false);assert.equal(result.error.code,'VALIDATION_ERROR');assert.equal(received.length,count);});
}finally{server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
console.log(JSON.stringify({...results,requests:received.length}));
if([...results.holdout,...results.anchors].some(x=>!x.passed))process.exitCode=1;
