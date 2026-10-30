/* Fixture setup navigation is a browser preference; only fixture drags edit the project. */
(function(global){
  'use strict';
  const LIMIT=1e7,clamp=(v,a,b)=>Math.max(a,Math.min(b,v));
  const defaults=()=>({x:0,z:0,zoom:1});
  function valid(c){
    if(!c||![c.x,c.z,c.zoom].every(Number.isFinite)||Math.abs(c.x)>LIMIT||Math.abs(c.z)>LIMIT||c.zoom<1e-5||c.zoom>1e4)throw Error('Invalid fixture view');
    return {x:c.x,z:c.z,zoom:c.zoom};
  }
  const project=(c,p)=>({x:140+(p[0]-c.x)*12.5*c.zoom,y:120+(p[2]-c.z)*10.5*c.zoom});
  const unproject=(c,p)=>[c.x+(p.x-140)/(12.5*c.zoom),0,c.z+(p.y-120)/(10.5*c.zoom)];
  const pan=(c,dx,dy)=>({...c,x:clamp(c.x-dx/(12.5*c.zoom),-LIMIT,LIMIT),z:clamp(c.z-dy/(10.5*c.zoom),-LIMIT,LIMIT)});
  function zoomAt(c,factor,point){
    if(!Number.isFinite(factor)||factor<=0)throw Error('Invalid zoom');
    const before=unproject(c,point),next={...c,zoom:clamp(c.zoom*factor,1e-5,1e4)},after=unproject(next,point);
    return {...next,x:clamp(next.x+before[0]-after[0],-LIMIT,LIMIT),z:clamp(next.z+before[2]-after[2],-LIMIT,LIMIT)};
  }
  function fit(points){
    points=points.filter(p=>Array.isArray(p)&&p.length===3&&p.every(Number.isFinite));
    if(!points.length)return defaults();
    const xs=points.map(p=>p[0]),zs=points.map(p=>p[2]),loX=Math.min(...xs),hiX=Math.max(...xs),loZ=Math.min(...zs),hiZ=Math.max(...zs);
    return valid({x:clamp((loX+hiX)/2,-LIMIT,LIMIT),z:clamp((loZ+hiZ)/2,-LIMIT,LIMIT),zoom:clamp(Math.min(220/(12.5*Math.max(2,hiX-loX)),180/(10.5*Math.max(2,hiZ-loZ))),1e-5,1e4)});
  }
  function grid(c){
    const raw=2/c.zoom,base=10**Math.floor(Math.log10(raw)),ratio=raw/base,step=(ratio<=1?1:ratio<=2?2:ratio<=5?5:10)*base;
    const lo=unproject(c,{x:0,y:0}),hi=unproject(c,{x:280,y:240}),x=[],y=[];
    for(let n=Math.ceil(lo[0]/step);n<=Math.floor(hi[0]/step)&&x.length<64;n++)x.push({pixel:project(c,[n*step,0,0]).x,axis:n===0});
    for(let n=Math.ceil(lo[2]/step);n<=Math.floor(hi[2]/step)&&y.length<64;n++)y.push({pixel:project(c,[0,0,n*step]).y,axis:n===0});
    return {x,y,step};
  }
  function mount(view,options){
    const key='schema-fixture-setup-view-v1',points=new Map();
    let camera=defaults(),panMode=false,gesture=null,frame=0,saveTimer=0,disposed=false;
    try{const saved=JSON.parse(localStorage.getItem(key));camera=valid(saved);panMode=saved.pan===true;}catch{}
    const toolbar=document.createElement('div');toolbar.className='stage-navigation';toolbar.setAttribute('role','toolbar');toolbar.setAttribute('aria-label','Fixture view navigation');
    const button=(text,label,action)=>{const b=document.createElement('button');b.type='button';b.textContent=text;b.setAttribute('aria-label',label);b.onclick=action;toolbar.append(b);return b;};
    const mode=button('Pan','Pan fixture view',()=>{panMode=!panMode;update();save();});
    button('−','Zoom out fixture view',()=>change(zoomAt(camera,1/1.25,{x:140,y:120})));
    const readout=document.createElement('output');readout.setAttribute('aria-label','Fixture view zoom');toolbar.append(readout);
    button('+','Zoom in fixture view',()=>change(zoomAt(camera,1.25,{x:140,y:120})));
    button('Fit','Fit fixtures in view',()=>change(fit(options.positions())));
    button('Reset','Reset fixture view',()=>change(defaults()));
    toolbar.title='Drag empty space or use Pan to move the view. Scroll or pinch to zoom. Drag a fixture to move it.';
    view.before(toolbar);view.tabIndex=0;view.setAttribute('aria-label','Editable top-down stage. Drag background to pan, scroll or pinch to zoom.');
    function save(){clearTimeout(saveTimer);saveTimer=setTimeout(persist,120);}
    function persist(){try{localStorage.setItem(key,JSON.stringify({...camera,pan:panMode}));}catch{}}
    function update(){
      mode.setAttribute('aria-pressed',String(panMode));view.classList.toggle('stage-pan-mode',panMode);
      readout.textContent=camera.zoom>=.01?Math.round(camera.zoom*100)+'%':(camera.zoom*100).toPrecision(2)+'%';
      view.dataset.cameraZoom=camera.zoom;view.dataset.cameraX=camera.x;view.dataset.cameraZ=camera.z;
      const axis=view.parentElement.querySelector('.stage-axis span');
      if(axis){const lo=unproject(camera,{x:0,y:0}),hi=unproject(camera,{x:280,y:240});axis.textContent=`X ${lo[0].toFixed(1)}…${hi[0].toFixed(1)}`;axis.title=`Z ${lo[2].toFixed(1)}…${hi[2].toFixed(1)} metres`;}
    }
    function paint(){frame=0;if(!disposed){update();options.changed();}}
    function change(next){camera=valid(next);save();if(!frame)frame=requestAnimationFrame(paint);}
    function point(event){const matrix=view.getScreenCTM();if(!matrix)return null;const p=new DOMPoint(event.clientX,event.clientY).matrixTransform(matrix.inverse());return {x:p.x,y:p.y};}
    function geometry(){const all=[...points.values()];return {center:{x:all.reduce((s,p)=>s+p.x,0)/all.length,y:all.reduce((s,p)=>s+p.y,0)/all.length},distance:all.length>1?Math.max(1,Math.hypot(all[0].x-all[1].x,all[0].y-all[1].y)):0};}
    function capture(event){try{view.setPointerCapture(event.pointerId);}catch{}}
    function stop(event){event.preventDefault();event.stopImmediatePropagation();}
    function cancel(){points.clear();gesture=null;view.classList.remove('stage-navigating');options.cancelFixture();}
    function down(event){
      if(event.pointerType!=='touch'&&![0,1,2].includes(event.button))return;
      const p=point(event);if(!p)return;points.set(event.pointerId,p);
      const fixture=event.target.closest('.fixture-layout[data-fixture]');
      if(points.size>1||panMode||event.button!==0||!fixture){
        options.cancelFixture();gesture=geometry();capture(event);view.classList.add('stage-navigating');stop(event);
      }
    }
    function move(event){
      if(!points.has(event.pointerId))return;
      const p=point(event);if(!p){cancel();return;}points.set(event.pointerId,p);
      if(!gesture)return;
      const next=geometry();
      let c=camera;
      if(gesture.distance&&next.distance)c=zoomAt(c,next.distance/gesture.distance,gesture.center);
      c=pan(c,next.center.x-gesture.center.x,next.center.y-gesture.center.y);
      change(c);gesture=next;stop(event);
    }
    function end(event){
      if(!points.has(event.pointerId))return;
      const navigating=!!gesture;points.delete(event.pointerId);
      if(event.type==='pointercancel'||event.type==='lostpointercapture')options.cancelFixture();
      gesture=navigating&&points.size?geometry():null;
      if(!gesture)view.classList.remove('stage-navigating');
      if(navigating)stop(event);
    }
    function wheel(event){options.cancelFixture();const p=point(event);if(!p)return;const delta=event.deltaY*(event.deltaMode===1?16:event.deltaMode===2?240:1);change(zoomAt(camera,Math.exp(-clamp(delta,-1000,1000)*.0015),p));event.preventDefault();}
    function keyboard(event){
      if(event.target!==view||event.ctrlKey||event.metaKey||event.altKey)return;
      if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(event.key)){change(pan(camera,event.key==='ArrowLeft'?24:event.key==='ArrowRight'?-24:0,event.key==='ArrowUp'?24:event.key==='ArrowDown'?-24:0));event.preventDefault();event.stopPropagation();}
      else if(['+','=','-','Home','f','F'].includes(event.key)){change(event.key==='Home'?defaults():['f','F'].includes(event.key)?fit(options.positions()):zoomAt(camera,event.key==='-'?1/1.25:1.25,{x:140,y:120}));event.preventDefault();event.stopPropagation();}
    }
    const context=event=>event.preventDefault(),hidden=()=>{if(document.hidden)cancel();};
    view.addEventListener('pointerdown',down,true);window.addEventListener('pointermove',move,true);window.addEventListener('pointerup',end,true);window.addEventListener('pointercancel',end,true);view.addEventListener('lostpointercapture',end,true);view.addEventListener('wheel',wheel,{passive:false});view.addEventListener('keydown',keyboard);view.addEventListener('contextmenu',context);window.addEventListener('blur',cancel);window.addEventListener('pagehide',persist);document.addEventListener('visibilitychange',hidden);
    update();
    return {project:p=>project(camera,p),unproject:p=>unproject(camera,p),grid:()=>grid(camera),destroy(){disposed=true;clearTimeout(saveTimer);cancelAnimationFrame(frame);persist();toolbar.remove();view.removeEventListener('pointerdown',down,true);window.removeEventListener('pointermove',move,true);window.removeEventListener('pointerup',end,true);window.removeEventListener('pointercancel',end,true);view.removeEventListener('lostpointercapture',end,true);view.removeEventListener('wheel',wheel);view.removeEventListener('keydown',keyboard);view.removeEventListener('contextmenu',context);window.removeEventListener('blur',cancel);window.removeEventListener('pagehide',persist);document.removeEventListener('visibilitychange',hidden);}};
  }
  const api={defaults,valid,project,unproject,pan,zoomAt,fit,grid,mount};global.SchemaStageNavigation=api;if(typeof module!=='undefined')module.exports=api;
})(globalThis);
