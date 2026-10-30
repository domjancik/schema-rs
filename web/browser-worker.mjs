import init,{BrowserEngine} from './wasm/schema_web.js';
import {openStorage} from './browser-storage.mjs';
let engine,storage,sessionId,latest,timer,persistenceError='',saving=Promise.resolve(),initialized=false,subscribed=false,manual=false;
const capabilities={execution:'browser-worker',network_output:false,usb_output:false,filesystem:false,scene_editing:true,session_serialization:true};
function capture(){latest=JSON.parse(engine.request('/api/state','null'));latest.snapshot.output_status='Browser preview · no physical outputs';return latest;}
function revision(expected){const current=capture().snapshot.revision;if(expected!==undefined&&expected!==current)throw Error('Revision conflict: refresh state before editing');return current;}
function id(value){if(typeof value!=='string'||! /^[A-Za-z0-9_-]{1,80}$/.test(value))throw Error('Use a project ID of 1–80 letters, digits, - or _.');return value;}
function persist(){
  if(!storage)return Promise.resolve();
  const text=engine.checkpoint();
  saving=saving.catch(()=>{}).then(()=>storage.put('sessions',{id:sessionId,text})).then(()=>{persistenceError='';},error=>{persistenceError=String(error.message||error);throw error;});
  // Every background failure is handled; explicit Save still rejects to its caller.
  saving.catch(()=>{});return saving;
}
async function start(options){
  if(initialized)throw Error('Worker already owns an engine');
  for(const key of ['fps','publishHz'])if(options[key]!==undefined&&(!Number.isFinite(options[key])||options[key]<1||options[key]>120))throw Error('Frame rates must be between 1 and 120 Hz');
  await init();
  if(options.persist!==false){
    storage=await openStorage();sessionId=id(options.sessionId);
    if(navigator.locks){
      let ready;const acquired=new Promise(resolve=>ready=resolve);
      navigator.locks.request('schema-session:'+sessionId,{ifAvailable:true},lock=>{ready(!!lock);return lock?new Promise(()=>{}):undefined;}).catch(()=>ready(false));
      if(!await acquired)throw Error('This browser session is already open in another tab. Open a new browser workspace.');
    }
  }
  engine=new BrowserEngine(options.project?JSON.stringify(options.project):undefined);
  const saved=storage&&await storage.get('sessions',sessionId);
  if(saved&&!options.project)engine.restore(saved.text);
  if(storage)for(const resource of await storage.all('resources'))if(resource.id.startsWith('content:'))engine.request('/api/inputs',JSON.stringify({type:'content',name:resource.id.slice(8),entries:resource.value}));
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
  if(path==='/api/state')return capture();
  if(path==='/api/session/status')return {enabled:!!storage,error:persistenceError||null};
  if(path==='/api/session/save'){await persist();return {saved_revision:capture().snapshot.revision};}
  if(path==='/api/session')return JSON.parse(engine.checkpoint());
  if(path==='/api/projects'){
    const projects=(await storageRequired().all('projects')).map(({project,...entry})=>entry),known=new Set(projects.map(p=>p.id));
    for(const resource of await storage.all('resources')){
      const key=resource.id.split(':')[1];if(known.has(key))continue;known.add(key);
      projects.push({id:key,title:key,fixtures:resource.id.startsWith('layout:')?resource.value.fixtures.length:0,saved_revision:resource.saved_revision||0,modified_unix_ms:resource.modified_unix_ms||0,legacy:true});
    }
    return {projects};
  }
  if(path==='/api/projects/save'){
    const saved_revision=revision(body.expected_revision),key=id(body.id),title=body.title?.trim();if(!title||title.length>200)throw Error('Project title required (maximum 200 characters)');
    const project=structuredClone(capture().project);project.name=title;project.output.enabled=false;
    const entry=await storageRequired().saveProject({id:key,title,project,fixtures:project.fixtures.length,saved_revision,modified_unix_ms:Date.now(),legacy:false},!!body.overwrite);
    await persist();const {project:unused,...metadata}=entry;return {saved_revision,project:metadata};
  }
  if(path==='/api/projects/load'){
    const key=id(body.id),saved=await storageRequired().get('projects',key);let project=saved?.project;
    if(!project){
      const layout=await storage.get('resources','layout:'+key),content=await storage.get('resources','content:'+key);
      if(!layout&&!content)throw Error('Saved project not found');project=structuredClone(capture().project);
      if(layout){Object.assign(project,layout.value);const fixtures=new Set(project.fixtures.map(f=>f.id));project.content=Object.fromEntries(Object.entries(project.content).filter(([fixture])=>fixtures.has(fixture)));}
      if(content)for(const entry of content.value){const layers=project.content[entry.fixture]||=[];while(layers.length<=entry.layer)layers.push(null);layers[entry.layer]=entry.stack;}
    }
    revision(body.expected_revision);const result=JSON.parse(engine.request('/api/command',JSON.stringify({expected_revision:body.expected_revision,command:{type:'replace_project',project}})));await persist();return result;
  }
  if(path==='/api/project/save'){
    const data=capture(),key=id(body.name||'init');
    if(!body.scope||body.scope==='all')return request('/api/projects/save',{id:key,title:data.project.name,overwrite:true,expected_revision:data.snapshot.revision});
    const content=[];
    for(const [fixture,layers]of Object.entries(data.project.content))for(let layer=0;layer<layers.length;layer++)if((!body.fixtures?.length||body.fixtures.includes(fixture))&&(body.layer==null||body.layer===layer))content.push({fixture,layer,stack:layers[layer]});
    const value=body.scope==='content'?content:{fixtures:data.project.fixtures,dmx_models:data.project.dmx_models,dimming:data.project.dimming};
    if(!['layout','content'].includes(body.scope))throw Error('Unknown save scope');
    await storageRequired().put('resources',{id:body.scope+':'+key,value,saved_revision:data.snapshot.revision,modified_unix_ms:Date.now()});if(body.scope==='content')engine.request('/api/inputs',JSON.stringify({type:'content',name:key,entries:content}));return {saved_revision:data.snapshot.revision};
  }
  if(path==='/api/content/load'){
    const saved=await storageRequired().get('resources','content:'+id(body.name));if(!saved)throw Error('Content snapshot not found');
    const project=capture().project,entries=saved.value.filter(entry=>!body.group||project.fixtures.some(f=>f.id===entry.fixture&&f.groups.includes(body.group)));
    return JSON.parse(engine.request('/api/command',JSON.stringify({expected_revision:body.expected_revision,command:{type:'apply_content',entries}})));
  }
  const decoded=path.startsWith('/api/export/')?decodeURIComponent(path):path;
  const result=engine.request(decoded,JSON.stringify(body??null));
  if(['/api/command','/api/import','/api/projects/bundle'].includes(path)){await persist();}
  return path.startsWith('/api/export/')?result:JSON.parse(result);
}
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
