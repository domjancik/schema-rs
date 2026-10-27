'use strict';
window.SchemaEngine={async request(path,body){const token=sessionStorage.getItem('schema-token');const response=await fetch(path,{method:body===undefined?'GET':'POST',headers:{'content-type':'application/json',...(token?{'x-schema-token':token}:{})},...(body===undefined?{}:{body:JSON.stringify(body)})});if(!response.ok){let text=await response.text();try{text=JSON.parse(text).error||text;}catch{}throw Error(text);}return response.headers.get('content-type')?.includes('json')?response.json():response.text();}};
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
