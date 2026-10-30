/* Stack locations are editor context only; breadcrumbs never submit engine commands. */
(function(g){
 'use strict';
 const layers=(p,id)=>id==='__director'?(p.director||[]):(p.content[id]||[]);
 function findPath(root,id){if(!root)return null;if(root.id===id)return[root];for(const child of root.children||[]){const found=findPath(child,id);if(found)return[root,...found];}return null;}
 function describe(project,requested={},catalog=[]){
  const fixture=requested.fixture==='__director'||project.fixtures.some(f=>f.id===requested.fixture)?requested.fixture:project.fixtures[0]?.id||'__director';
  const available=layers(project,fixture),layer=Math.max(0,Math.min(Number.isInteger(requested.layer)?requested.layer:0,Math.max(0,available.length-1))),stack=available[layer];
  const name=node=>catalog.find(b=>b.key===node?.behavior)?.name||node?.behavior?.split('.').at(-1)||'Empty layer';
  const fixtureName=fixture==='__director'?'Director':project.fixtures.find(f=>f.id===fixture)?.name||fixture;
  const trail=[],target=(node,path=trail)=>({fixture,layer,node:node?.id||0,driverTrail:path.map(step=>({...step}))});
  let displayed=stack?.root;const rootContext=target(displayed,[]),crumbs=[{id:'root',kind:'root',label:'Root',title:name(displayed),context:rootContext}];
  const addBlocks=path=>{for(const node of(path||[]).slice(1))crumbs.push({id:`block:${trail.length}:${node.id}`,kind:'block',label:name(node),title:`${name(node)} · #${node.id}`,context:target(node)});};
  for(const step of requested.driverTrail||requested.context||[]){
   const ownerPath=findPath(displayed,step.owner),owner=ownerPath?.at(-1),driver=owner?.drivers?.[step.key];if(!driver)break;
   addBlocks(ownerPath);trail.push({owner:owner.id,key:step.key});displayed=driver;
   crumbs.push({id:`driver:${trail.length}:${step.key}`,kind:'driver',key:step.key,label:step.key+' driver',title:`${name(owner)} · ${step.key} driver`,context:target(driver)});
  }
  const selectedPath=findPath(displayed,requested.node)|| (displayed?[displayed]:[]);addBlocks(selectedPath);
  crumbs.at(-1).current=true;
  return{fixtureName,context:target(selectedPath.at(-1)||displayed),rootContext,crumbs,layerName:stack?.name||'Empty layer',layers:available};
 }
 function render(host,options){
  const data=describe(options.project,options.context,options.catalog),el=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
  host.classList.add('stack-breadcrumbs');host.setAttribute('aria-label','Stack breadcrumbs');host.setAttribute('role','navigation');host.replaceChildren();
  host.onpointerdown=()=>options.beforeNavigate?.();host.onkeydown=e=>e.stopPropagation();
  function go(context,focus){options.beforeNavigate?.();options.navigate(context);queueMicrotask(()=>{if(host.isConnected)[...host.querySelectorAll('[data-crumb-focus]')].find(n=>n.dataset.crumbFocus===focus)?.focus({preventScroll:true});});}
  const prefix=el('div');prefix.className='stack-crumb-prefix';
  const fixture=el('select');fixture.className='crumb-fixture';fixture.setAttribute('aria-label','Breadcrumb fixture');fixture.dataset.crumbFocus='fixture';
  for(const f of options.project.fixtures)fixture.add(new Option(f.name,f.id));if(options.project.director)fixture.add(new Option('Director','__director'));
  fixture.value=data.context.fixture;fixture.title=data.fixtureName;fixture.onchange=()=>go(describe(options.project,{fixture:fixture.value,layer:0},options.catalog).rootContext,'fixture');
  const layer=el('select');layer.className='crumb-layer';layer.setAttribute('aria-label','Breadcrumb layer');layer.dataset.crumbFocus='layer';
  if(!data.layers.length)layer.add(new Option('Layer 1 · Empty layer','0'));
  data.layers.forEach((stack,i)=>layer.add(new Option(`Layer ${i+1} · ${stack?.name||'Empty layer'}${stack?.enabled===false?' · Off':''}`,i)));
  layer.value=data.context.layer;layer.title=`Layer ${data.context.layer+1} · ${data.layerName}`;layer.onchange=()=>go(describe(options.project,{fixture:data.context.fixture,layer:Number(layer.value)},options.catalog).rootContext,'layer');
  const separator=()=>{const n=el('span','›');n.className='stack-crumb-separator';n.setAttribute('aria-hidden','true');return n;};
  prefix.append(fixture,separator(),layer);host.append(prefix,separator());
  const root=el('button','Root');root.type='button';root.className='stack-crumb-root';root.title='Layer root · '+data.crumbs[0].title;root.setAttribute('aria-label','Go to layer root');root.dataset.crumbFocus='root';if(data.crumbs[0].current)root.setAttribute('aria-current','location');root.onclick=()=>go(data.rootContext,'root');host.append(root);
  const trail=el('ol');trail.className='stack-crumb-trail';
  for(const crumb of data.crumbs.slice(1)){
   const item=el('li'),button=el('button',crumb.label);button.type='button';button.title=crumb.title;button.dataset.crumbFocus=crumb.id;button.dataset.stackNode=crumb.context.node;
   button.setAttribute('aria-label',crumb.kind==='driver'?'Open '+crumb.label:'Select '+crumb.label+' block');if(crumb.current)button.setAttribute('aria-current','location');
   if(crumb.kind==='driver')button.dataset.driverKey=crumb.key;
   button.onclick=()=>go(crumb.context,crumb.id);item.append(separator(),button);trail.append(item);
  }
  host.append(trail);return data;
 }
 const api={describe,render};g.SchemaStackNavigation=api;if(typeof module!=='undefined')module.exports=api;
})(globalThis);
