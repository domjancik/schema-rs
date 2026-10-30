'use strict';
window.SchemaEngine={async request(path,body){const token=sessionStorage.getItem('schema-token');const response=await fetch(path,{method:body===undefined?'GET':'POST',headers:{'content-type':'application/json',...(token?{'x-schema-token':token}:{})},...(body===undefined?{}:{body:JSON.stringify(body)})});if(!response.ok){let text=await response.text();try{text=JSON.parse(text).error||text;}catch{}throw Error(text);}return response.headers.get('content-type')?.includes('json')?response.json():response.text();}};
// Browser and native transports share the same command/publication protocol.
if(document.documentElement.dataset.engine==='browser'||new URLSearchParams(location.search).get('engine')==='browser'){
 const remote=window.SchemaEngine;
 let parentEngine;try{if(parent!==window&&parent.location.origin===location.origin&&parent.SchemaEngine?.kind==='browser')parentEngine=parent.SchemaEngine;}catch{}
 let sessionId=sessionStorage.getItem('schema-browser-session');
 if(!sessionId){sessionId=crypto.randomUUID();sessionStorage.setItem('schema-browser-session',sessionId);}
 const ready=parentEngine?parentEngine.ready:import('/schema-player.mjs').then(({SchemaPlayer})=>SchemaPlayer.create({persist:true,sessionId}));
 ready.catch(error=>{document.getElementById('status')?.replaceChildren(document.createTextNode(error.message));});
 window.SchemaEngine={kind:'browser',ready,async request(path,body){return path.startsWith('/api/')?(await ready).request(path,body):remote.request(path,body);}};
 // The browser app has no hardware transport; computed DMX remains inspectable.
 for(const id of ['output-enabled','output-apply','connect']){const node=document.getElementById(id);if(node){node.disabled=true;node.title='Browser engine: preview and editing only';}}
 document.addEventListener('click',event=>{
  const link=event.target.closest('a[href]');if(!link)return;const url=new URL(link.href,location.href);
  if(url.origin===location.origin&&url.pathname==='/'&&url.searchParams.get('view')==='studio'){event.preventDefault();location.href='/play?view=studio';}
 });
 const bar=document.querySelector('.header-actions');
 if(bar){
  const fresh=document.createElement('button');fresh.textContent='New browser session';fresh.onclick=()=>{sessionStorage.setItem('schema-browser-session',crypto.randomUUID());location.reload();};bar.append(fresh);
  const download=document.createElement('button');download.textContent='Export project';download.onclick=async()=>{const {project}=await window.SchemaEngine.request('/api/state');const url=URL.createObjectURL(new Blob([JSON.stringify(project,null,2)],{type:'application/json'}));const link=document.createElement('a');link.href=url;link.download='schema-project.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);};
  const upload=document.createElement('button');upload.textContent='Import project';upload.onclick=()=>{const input=document.createElement('input');input.type='file';input.accept='.json,application/json';input.onchange=async()=>{try{const file=input.files[0];if(!file)return;if(file.size>16*1024*1024)throw Error('Project exceeds 16 MiB');const project=JSON.parse(await file.text());if(project.output)project.output.enabled=false;const state=await window.SchemaEngine.request('/api/state');await window.SchemaEngine.request('/api/command',{expected_revision:state.snapshot.revision,command:{type:'replace_project',project}});}catch(error){alert(error.message);}};input.click();};bar.append(upload,download);
 }
}
window.SchemaEditorContext={load(){try{return JSON.parse(sessionStorage.getItem('schema-editor-context')||'{}');}catch{return{};}},save(value){try{sessionStorage.setItem('schema-editor-context',JSON.stringify(value));}catch{}}};

// Fullscreen is a local display preference; it never changes engine state.
(()=>{
 const button=document.getElementById('fullscreen-toggle');
 if(!button)return;
 button.hidden=true;
 const root=document.documentElement;
 if(!document.fullscreenEnabled||typeof root.requestFullscreen!=='function')return;
 button.hidden=false;
 function sync(){
  const active=Boolean(document.fullscreenElement);
  const label=active?'Exit fullscreen':'Enter fullscreen';
  button.setAttribute('aria-pressed',String(active));
  button.setAttribute('aria-label',label);
  button.title=label;
  button.textContent=active?'⤡':'⛶';
 }
 document.addEventListener('fullscreenchange',sync);
 button.onclick=async()=>{
  button.disabled=true;
  try{
   if(document.fullscreenElement)await document.exitFullscreen();
   else await root.requestFullscreen();
   sync();
  }catch{
   sync();
   button.title='Fullscreen unavailable in this browser or window';
  }finally{button.disabled=false;}
 };
 sync();
})();

// Reload/open actions wait for field commits and preserve the browser session.
window.SchemaAppNavigation=(()=>{
 const writes=new Set(),request=SchemaEngine.request.bind(SchemaEngine);
 SchemaEngine.request=(path,body)=>{
  const result=Promise.resolve().then(()=>request(path,body));
  if(body!==undefined&&/^\/api\/(command|import|projects\/(save|load)|session\/save)$/.test(path)){
   writes.add(result);result.then(()=>writes.delete(result),()=>writes.delete(result));
  }
  return result;
 };
 async function flush(){
  document.activeElement?.blur();
  await new Promise(resolve=>setTimeout(resolve,0));
  const deadline=Date.now()+10000;
  while(writes.size){
   if(Date.now()>deadline)throw Error('An edit is still being saved. Try again in a moment.');
   let timer;
   try{await Promise.race([Promise.all([...writes]),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('An edit is still being saved. Try again in a moment.')),Math.max(1,deadline-Date.now()));})]);}finally{clearTimeout(timer);}
  }
 }
 return{flush,async prepare(){
  await flush();
  for(const frame of document.querySelectorAll('iframe'))try{if(frame.contentWindow?.location.origin===location.origin)await frame.contentWindow.SchemaAppNavigation?.flush();}catch(error){if(error.name!=='SecurityError')throw error;}
  if(SchemaEngine.kind==='browser')await SchemaEngine.request('/api/session/save',{});
 }};
})();
