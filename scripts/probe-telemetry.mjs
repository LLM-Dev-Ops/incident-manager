import {createServer} from 'node:http';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';

// Native HTTP against compiled production code; no injected fetch or fake clocks.
const root=resolve(process.argv[2]);
const {TelemetryEmitter}=await import(pathToFileURL(`${root}/packages/ruvector-client/dist/telemetry.js`));
const holdout=[],anchors=[];
const originalError=console.error;
console.error=()=>{};
for(const status of [400,401,422,429,500,503,200,204]) {
  const batches=[];
  const server=createServer(async(req,res)=>{
    let body='';for await(const part of req)body+=part;
    batches.push(JSON.parse(body).events);
    res.writeHead(batches.length===1?status:204);res.end();
  });
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  const emitter=new TelemetryEmitter({endpoint:`http://127.0.0.1:${server.address().port}`,serviceName:'native-contract',environment:'qa',batchSize:100,flushIntervalMs:60000});
  const event={event_type:'metric',agent_id:'fixture-agent',execution_ref:'fixture-execution',timestamp:'2026-11-07T13:00:00.000Z',payload:{id:status}};
  try {
    emitter.emit(event);await emitter.flush();
    if(status>=400)emitter.emit({...event,payload:{id:'new'}});
    await emitter.flush();await emitter.flush();
    const passed=status>=400?batches.length===2&&JSON.stringify(batches[1])===JSON.stringify([event,{...event,payload:{id:'new'}}]):batches.length===1&&JSON.stringify(batches[0])===JSON.stringify([event]);
    (status>=400?holdout:anchors).push({id:`HTTP-${status}`,passed,batches});
  }finally{await emitter.shutdown();server.closeAllConnections();await new Promise(r=>server.close(r));}
}
const disabled=new TelemetryEmitter({endpoint:'http://127.0.0.1:1',serviceName:'disabled',environment:'qa',enabled:false});
disabled.emitInvocation('fixture','fixture');await disabled.flush();await disabled.shutdown();
anchors.push({id:'disabled-noop',passed:disabled.buffer.length===0});
console.error=originalError;
console.log(JSON.stringify({holdout,anchors}));
