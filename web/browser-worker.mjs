import init,{BrowserEngine} from './wasm/schema_web.js';
import {openStorage} from './browser-storage.mjs';
let engine,storage,sessionId,latest,timer,persistenceError='',saving=Promise.resolve(),initialized=false,subscribed=false,manual=false,persistedSession=null,persistedNeedsBackup=false;
const capabilities={execution:'browser-worker',network_output:false,usb_output:false,filesystem:false,scene_editing:true,session_serialization:true};
function capture(){latest=JSON.parse(engine.request('/api/state/compact','null'));latest.snapshot.output_status='Browser preview · no physical outputs';return latest;}
function revision(expected){const current=capture().snapshot.revision;if(expected!==undefined&&expected!==current)throw Error('Revision conflict: refresh state before editing');return current;}
function id(value){if(typeof value!=='string'||! /^[A-Za-z0-9_-]{1,80}$/.test(value))throw Error('Use a project ID of 1–80 letters, digits, - or _.');return value;}
function persist(){
  if(!storage)return Promise.resolve();
  const text=engine.checkpoint();
  saving=saving.catch(()=>{}).then(async()=>{persistedSession=await storage.saveSession({id:sessionId,text},persistedSession?.text??null,persistedNeedsBackup);persistedNeedsBackup=false;}).then(()=>{persistenceError='';},error=>{persistenceError=String(error.message||error);throw error;});
  // Every background failure is handled; explicit Save still rejects to its caller.
  saving.catch(()=>{});return saving;
}
async function start(options){
  if(initialized)throw Error('Worker already owns an engine');
  for(const key of ['fps','publishHz'])if(options[key]!==undefined&&(!Number.isFinite(options[key])||options[key]<1||options[key]>120))throw Error('Frame rates must be between 1 and 120 Hz');
  await init();
  engine=new BrowserEngine(options.project?JSON.stringify(options.project):undefined);
  if(options.persist!==false){
    storage=await openStorage(JSON.parse(engine.request('/api/storage/versions','null')));sessionId=id(options.sessionId);
    if(navigator.locks){
      let ready;const acquired=new Promise(resolve=>ready=resolve);
      navigator.locks.request('schema-session:'+sessionId,{ifAvailable:true},lock=>{ready(!!lock);return lock?new Promise(()=>{}):undefined;}).catch(()=>ready(false));
      if(!await acquired)throw Error('This browser session is already open in another tab. Open a new browser workspace.');
    }
  }
  const saved=storage&&await storage.get('sessions',sessionId);
  persistedSession=saved||null;
  if(saved&&options.project)throw Error('This browser session already contains saved data. Use a new session ID for a new project.');
  if(saved){const report=JSON.parse(engine.restore(saved.text));persistedNeedsBackup=report.steps.length>0;}
  if(storage)for(const resource of await storage.all('resources'))if(resource.id.startsWith('content:'))engine.request('/api/resources/content',JSON.stringify({name:resource.id.slice(8),text:resourceText(resource)}));
  initialized=true;capture();manual=options.autoTick===false;if(manual)return capabilities;let previous=performance.now(),lastPosted=0,lastSaved=previous;
  timer=setInterval(()=>{
    try{
      const now=performance.now(),dt=(now-previous)/1000;previous=now;
      const clock=new Date();engine.request('/api/inputs',JSON.stringify({type:'clock',seconds:clock.getUTCHours()*3600+clock.getUTCMinutes()*60+clock.getUTCSeconds()}));
      latest=JSON.parse(engine.advance(dt));latest.snapshot.output_status='Browser preview · no physical outputs';
      if(subscribed&&now-lastPosted>=1000/(options.publishHz||30)){lastPosted=now;postMessage({type:'frame',frame:latest.snapshot});}
      if(storage&&now-lastSaved>2000){lastSaved=now;persist().catch(error=>postMessage({type:'storage-error',error:String(error.message||error)}));}
    }catch(error){clearInterval(timer);postMessage({type:'fatal',error:String(error.message||error)});}
  },1000/Math.max(1,Math.min(120,options.fps||60)));
  return capabilities;
}
async function request(path,body){
  if(path==='/api/capabilities')return capabilities;
  if(path==='/api/state')return JSON.parse(engine.request('/api/state','null'));
  if(path==='/api/state/compact')return capture();
  if(path==='/api/session/status')return {enabled:!!storage,error:persistenceError||null};
  if(path==='/api/session/save'){await persist();return {saved_revision:capture().snapshot.revision};}
  if(path==='/api/session')return JSON.parse(engine.checkpoint());
  if(path==='/api/session/raw')return engine.checkpoint();
  if(path==='/api/project/raw')return engine.request(path,'null');
  if(path==='/api/projects/bundle/raw')return engine.request('/api/projects/bundle','null');
  if(path==='/api/projects'){
    const projects=(await storageRequired().all('projects')).map(({project,project_text,...entry})=>entry),known=new Set(projects.map(p=>p.id));
    for(const resource of await storage.all('resources')){
      const key=resource.id.split(':')[1];if(known.has(key))continue;known.add(key);
      projects.push({id:key,title:key,fixtures:resource.id.startsWith('layout:')?JSON.parse(resourceText(resource)).fixtures.length:0,saved_revision:resource.saved_revision||0,modified_unix_ms:resource.modified_unix_ms||0,legacy:true});
    }
    return {projects};
  }
  if(path==='/api/projects/save'){
    const saved_revision=revision(body.expected_revision),key=id(body.id),title=body.title?.trim();if(!title||title.length>200)throw Error('Project title required (maximum 200 characters)');
    const project_text=engine.request('/api/storage/snapshot',JSON.stringify({format:'project',title}));
    const existing=await storageRequired().get('projects',key);if(existing)migrate('project',existing.project_text??existing.project);
    const entry=await storageRequired().saveProject({id:key,title,project_text,fixtures:capture().project.fixtures.length,saved_revision,modified_unix_ms:Date.now(),legacy:false},!!body.overwrite,existing??null);
    await persist();const {project:unused,project_text:unusedText,...metadata}=entry;return {saved_revision,project:metadata};
  }
  if(path==='/api/projects/load'){
    const key=id(body.id),saved=await storageRequired().get('projects',key);revision(body.expected_revision);let result;
    if(saved)result=engine.request('/api/projects/import',JSON.stringify({json:saved.project_text??JSON.stringify(saved.project),expected_revision:body.expected_revision}));
    else{
      const layout=await storage.get('resources','layout:'+key),content=await storage.get('resources','content:'+key);
      if(!layout&&!content)throw Error('Saved project not found');
      result=engine.request('/api/projects/compose',JSON.stringify({layout:layout?layoutText(layout,key):null,content:content?resourceText(content):null,expected_revision:body.expected_revision}));
    }
    await persist();return JSON.parse(result);
  }
  if(path==='/api/project/save'){
    const data=capture(),key=id(body.name||'init');
    if(!body.scope||body.scope==='all')return request('/api/projects/save',{id:key,title:data.project.name,overwrite:true,expected_revision:data.snapshot.revision});
    if(!['layout','content'].includes(body.scope))throw Error('Unknown save scope');
    const value_text=engine.request('/api/storage/snapshot',JSON.stringify({format:body.scope,fixtures:body.fixtures||[],layer:body.layer}));
    const resourceId=body.scope+':'+key,existing=await storageRequired().get('resources',resourceId);if(existing)migrate(body.scope,body.scope==='layout'?layoutText(existing,key):resourceText(existing));
    await storageRequired().saveResource({id:resourceId,value_text,saved_revision:data.snapshot.revision,modified_unix_ms:Date.now()},existing??null);
    if(body.scope==='content')engine.request('/api/resources/content',JSON.stringify({name:key,text:value_text}));return {saved_revision:data.snapshot.revision};
  }
  if(path==='/api/content/load'){
    const saved=await storageRequired().get('resources','content:'+id(body.name));if(!saved)throw Error('Content snapshot not found');
    const result=engine.request(path,JSON.stringify({text:resourceText(saved),group:body.group,expected_revision:body.expected_revision}));await persist();return JSON.parse(result);
  }
  const decoded=path.startsWith('/api/export/')?decodeURIComponent(path):path;
  const result=engine.request(decoded,JSON.stringify(body??null));
  if(['/api/command','/api/import','/api/projects/bundle','/api/projects/import'].includes(path)){await persist();}
  return path.startsWith('/api/export/')?result:JSON.parse(result);
}
function migrate(format,document){return JSON.parse(engine.request('/api/storage/migrate',JSON.stringify({format,...(typeof document==='string'?{text:document}:{document})})));}
function resourceText(record){return record.value_text??JSON.stringify(record.value);}
function layoutText(record,name){return record.value_text??JSON.stringify({...record.value,version:record.value.version??1,name:record.value.name??name});}
function layoutDocument(value,name){return migrate('layout',{...value,version:value.version??1,name:value.name??name}).document;}
function storageRequired(){if(!storage)throw Error('This embedded engine has no persistence adapter. Use an editor workspace or export its session.');return storage;}
let queue=Promise.resolve();
onmessage=({data})=>{
  queue=queue.catch(()=>{}).then(async()=>{
    try{
      let result;
      if(data.method==='init')result=await start(data.options||{});
      else if(!initialized)throw Error('Engine not initialized');
      else if(data.method==='advance'){if(!manual)throw Error('Explicit advance requires autoTick:false');result=JSON.parse(engine.advance(data.dt));latest=result;if(subscribed)postMessage({type:'frame',frame:result.snapshot});}
      else if(data.method==='subscribe'){subscribed=!!data.enabled;result=true;}
      else if(data.method==='request')result=await request(data.path,data.body);
      else if(data.method==='restore'){engine.restore(data.text);await persist();result=capture();}
      else throw Error('Unknown engine operation');
      postMessage({id:data.id,result});
    }catch(error){postMessage({id:data.id,error:String(error.message||error)});}
  });
};
