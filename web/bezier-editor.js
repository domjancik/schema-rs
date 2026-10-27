"use strict";
// Scalar cubic: horizontal control locations are fixed thirds, matching the engine.
(function(root){
 const keys=['Start','Control1','Control2','End'];
 const value=(p,t)=>{const u=1-t;return u*u*u*p[0]+3*u*u*t*p[1]+3*u*t*t*p[2]+t*t*t*p[3];};
 function range(p){const lo=Math.min(...p),hi=Math.max(...p);if(hi===lo)return[lo-1,hi+1];const pad=(hi-lo)*.15;return[lo-pad,hi+pad];}
 function mount(container,{properties,drivers={},onCommit}){
  const doc=container.ownerDocument,win=doc.defaultView,make=(tag,attrs={})=>{const n=doc.createElementNS('http://www.w3.org/2000/svg',tag);for(const[k,v]of Object.entries(attrs))n.setAttribute(k,v);return n;};
  const box=doc.createElement('section');box.className='value-bezier-editor';box.dataset.busy='false';
  const heading=doc.createElement('h3');heading.textContent='Bézier value';
  const caption=doc.createElement('p');caption.textContent='Time → value · drag vertically or use ↑ / ↓. Shift makes larger steps. One undo per edit.';
  const graph=make('svg',{viewBox:'0 0 292 208',role:'group','aria-label':'Visual Bézier value curve'});
  let points=keys.map((key,i)=>Number(properties[key]??[0,0,1,1][i])),bounds=range(points),gesture=null,busy=false;
  const x=i=>32+i/3*244,y=v=>16+(bounds[1]-v)/(bounds[1]-bounds[0])*148;
  const grid=make('g',{class:'value-bezier-grid'}),gridLabels=[];graph.append(grid);
  for(let i=0;i<=4;i++){const yy=16+i*37;grid.append(make('line',{x1:32,y1:yy,x2:276,y2:yy}));const text=make('text',{x:27,y:yy+3,'text-anchor':'end'});text.textContent=Number((bounds[1]-i/4*(bounds[1]-bounds[0])).toPrecision(3));grid.append(text);gridLabels.push(text);}
  for(const [xx,text]of [[32,'0%'],[154,'Progress'],[276,'100%']]){const label=make('text',{x:xx,y:192,'text-anchor':'middle'});label.textContent=text;grid.append(label);}
  const guide=make('path',{class:'value-bezier-guide'}),curve=make('path',{class:'value-bezier-path'});graph.append(guide,curve);
  const handles=keys.map((key,i)=>{const h=make('g',{'data-key':key,tabindex:drivers[key]?-1:0,role:'slider','aria-label':key+' curve handle','aria-orientation':'vertical','aria-disabled':String(!!drivers[key]),class:'value-bezier-handle'+(drivers[key]?' driven':'')});h.append(make('circle',{r:18,class:'value-bezier-hit'}),make('circle',{r:i===0||i===3?6:5,class:'value-bezier-dot'}));const title=make('title');title.textContent=key+(drivers[key]?' · controlled by a driver':' · drag value vertically');h.append(title);graph.append(h);return h;});
  const readout=doc.createElement('output');readout.className='value-bezier-readout';readout.setAttribute('aria-live','polite');
  function draw(){gridLabels.forEach((label,i)=>label.textContent=Number((bounds[1]-i/4*(bounds[1]-bounds[0])).toPrecision(3)));const p=points.map((v,i)=>[x(i),y(v)]);curve.setAttribute('d',`M${p[0]} C${p[1]} ${p[2]} ${p[3]}`);guide.setAttribute('d',`M${p[0]} L${p[1]} M${p[2]} L${p[3]}`);handles.forEach((h,i)=>{h.setAttribute('transform',`translate(${p[i]})`);h.setAttribute('aria-valuenow',points[i]);h.setAttribute('aria-valuemin',bounds[0]);h.setAttribute('aria-valuemax',bounds[1]);});readout.textContent=keys.map((k,i)=>k+' '+Number(points[i].toFixed(3))).join(' · ');}
  function cancel(){if(!gesture)return;const g=gesture;gesture=null;points[g.index]=g.before;draw();if(graph.hasPointerCapture(g.pointer))graph.releasePointerCapture(g.pointer);}
  async function commit(index,before){const next=Math.round(points[index]*1000)/1000;points[index]=next;if(Math.abs(next-before)<.0005){draw();return;}busy=true;box.dataset.busy='true';try{if(!await onCommit(keys[index],next))points[index]=before;}catch(error){points[index]=before;readout.textContent=error.message;}finally{busy=false;bounds=range(points);box.dataset.busy='false';draw();}}
  handles.forEach((handle,index)=>{
   handle.onpointerdown=e=>{if(e.button!==0||busy||gesture||drivers[keys[index]])return;e.preventDefault();e.stopPropagation();handle.focus({preventScroll:true});gesture={index,pointer:e.pointerId,before:points[index]};graph.setPointerCapture(e.pointerId);};
   handle.onkeydown=e=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();cancel();return;}if(!['ArrowUp','ArrowDown','Home','End'].includes(e.key)||busy||gesture||drivers[keys[index]])return;e.preventDefault();e.stopPropagation();const before=points[index],step=e.shiftKey?.1:.01;points[index]=e.key==='Home'?bounds[0]:e.key==='End'?bounds[1]:before+(e.key==='ArrowUp'?step:-step);commit(index,before);};
  });
  graph.onpointermove=e=>{if(gesture?.pointer!==e.pointerId)return;e.preventDefault();const matrix=graph.getScreenCTM();if(!matrix)return;const p=new win.DOMPoint(e.clientX,e.clientY).matrixTransform(matrix.inverse());points[gesture.index]=bounds[1]-(Math.max(16,Math.min(164,p.y))-16)/148*(bounds[1]-bounds[0]);draw();};
  graph.onpointerup=e=>{if(gesture?.pointer!==e.pointerId)return;const g=gesture;gesture=null;if(graph.hasPointerCapture(e.pointerId))graph.releasePointerCapture(e.pointerId);commit(g.index,g.before);};
  graph.onpointercancel=graph.onlostpointercapture=e=>{if(gesture?.pointer===e.pointerId)cancel();};
  graph.onkeydown=e=>{if(e.key==='Escape'&&gesture){e.preventDefault();e.stopPropagation();cancel();}};
  // Capture Escape even when the pointer has moved outside the SVG.
  doc.addEventListener('keydown',e=>{if(e.key==='Escape'&&gesture){e.preventDefault();e.stopPropagation();cancel();}},{capture:true,signal:(()=>{const controller=new win.AbortController();const observer=new win.MutationObserver(()=>{if(!box.isConnected){cancel();controller.abort();observer.disconnect();}});observer.observe(doc.documentElement,{childList:true,subtree:true});return controller.signal;})()});
  box.append(heading,graph,caption,readout);container.append(box);draw();return{cancel,get active(){return !!gesture;}};
 }
 const api={keys,value,range,mount};if(typeof module!=='undefined')module.exports=api;else root.SchemaBezier=api;
})(typeof window==='undefined'?globalThis:window);
