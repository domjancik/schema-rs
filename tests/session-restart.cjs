const assert=require('node:assert/strict');
const fs=require('node:fs/promises'),os=require('node:os'),path=require('node:path');
const {spawn}=require('node:child_process');
(async()=>{
 const root=await fs.mkdtemp(path.join(os.tmpdir(),'schema-session-restart-'));
 const binary=path.resolve(process.env.SCHEMA_BINARY||'target/debug/schema-rs.exe');
 const base='http://127.0.0.1:8790';let child;
 const env={...process.env};delete env.SCHEMA_TOKEN;
 async function start(){child=spawn(binary,['serve','--project',root,'--bind','127.0.0.1:8790'],{env,windowsHide:true,stdio:['ignore','pipe','pipe']});let log='';child.stdout.on('data',d=>log+=d);child.stderr.on('data',d=>log+=d);for(let i=0;i<100;i++){if(child.exitCode!==null)throw Error(log);try{const response=await fetch(base+'/api/session/status');if(response.ok){assert((await response.json()).enabled);return;}}catch{}await new Promise(r=>setTimeout(r,50));}throw Error('Engine failed to start: '+log);}
 async function stop(){if(!child||child.exitCode!==null)return;const process=child;await new Promise((resolve,reject)=>{const timeout=setTimeout(()=>reject(Error('Engine did not stop')),5000);process.once('exit',()=>{clearTimeout(timeout);resolve();});process.kill();});child=null;}
 const state=()=>fetch(base+'/api/state').then(r=>r.json());
 const command=async command=>{const current=await state();const response=await fetch(base+'/api/command',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({expected_revision:current.snapshot.revision,command})});assert.equal(response.status,200,await response.text());};
 try{
  await start();const original=await state();assert.equal(original.project.output.enabled,false);
  await command({type:'move_fixture',fixture:'fixture-1',position:[2,3,4]});await command({type:'set_dimmer',fixture:'fixture-1',value:0.3});await command({type:'undo'});
  let response=await fetch(base+'/api/session/save',{method:'POST'});assert.equal(response.status,200);const saved=await response.json();assert.equal(saved.saved_revision,3);
  const checkpoint=JSON.parse(await fs.readFile(path.join(root,'.session/session.json'),'utf8'));assert.equal(checkpoint.undo.length,1);assert.equal(checkpoint.redo.length,1);
  await stop();await start();const restored=await state();assert.equal(restored.snapshot.revision,3);assert(restored.snapshot.can_undo&&restored.snapshot.can_redo);assert(restored.snapshot.time>=checkpoint.snapshot.time);assert.deepEqual(restored.project,checkpoint.project);
  await command({type:'redo'});assert.equal((await state()).project.fixtures[0].dimmer,0.3);await command({type:'undo'});await command({type:'undo'});assert.deepEqual((await state()).project,original.project);
  response=await fetch(base+'/api/project/save',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({name:'init'})});assert.equal(response.status,200);const exported=await fetch(base+'/api/session').then(r=>r.json());assert.equal(exported.redo.length,2);
  console.log('Process restart: automatic session recovery, exact authored project, undo/redo ordering, clock continuation and Save project history flush passed');
 }finally{await stop();if(!root.startsWith(path.join(os.tmpdir(),'schema-session-restart-')))throw Error('Unexpected test directory');await fs.rm(root,{recursive:true,force:true});}
})().catch(e=>{console.error(e);process.exit(1)});
