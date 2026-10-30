/* Studio property controls. Drafts are local; native commands own authored values. */
(function(root){
  'use strict';
  const controls=new Set();let focusRequest=null,serial=0;
  const timeKeys=/^(Period|Duration|Time|Interval|Attack|Decay|Filter Time|Delay|Transition)$/;
  const integerKeys=/^(Seed|Index|Slot|Count|Steps|Samples|Iterations|Pixel count|Setup channel|Universe|Address|Pad|Layer|Frequency Band)$/;
  function spec(behavior={},key=''){
    const integer=integerKeys.test(key)||/^(Int|UInt|EnumIndex)/.test(behavior.parameter_types?.[key]||'');
    const result={step:integer?1:0.01,integer};
    if(key==='Period')return {...result,suggested:[0.05,30],step:0.05,unit:'s'};
    if(timeKeys.test(key))return {...result,suggested:[0,10],unit:'s'};
    if(/^(Value|Opacity|Probability|Randomness|Saturation|Brightness|Phase|Mix|Damping|Fixture dimmer)$/.test(key))result.suggested=[0,1];
    else if(/^(Minimum|Maximum|Source Min|Source Max|Offset|Balance)$/.test(key))result.suggested=[-1,1];
    else if(/^(Gain|Scale)$/.test(key))result.suggested=[0,4];
    else if(key==='Speed')result.suggested=[-2,2];
    else if(/^(X|Y|Z)$/.test(key)){result.suggested=[-10,10];result.step=0.1;result.unit='m';}
    else if(/^(Radius|Distance|Padding)$/.test(key)){result.suggested=[0,10];result.step=0.05;result.unit='m';}
    else if(/^(Count|Steps|Samples|Iterations)$/.test(key))result.suggested=[1,32];
    if(key==='Fixture dimmer')Object.assign(result,{min:0,max:1});
    if(key==='Address')Object.assign(result,{min:1,max:512});
    if(key==='Universe')Object.assign(result,{min:0,max:32767});
    if(key==='Pixel count')Object.assign(result,{min:1,max:512,suggested:[1,32]});
    if(key==='Setup channel')Object.assign(result,{min:0,max:512});
    return result;
  }
  function nudge(value,direction,settings={},modifiers={}){
    if(!Number.isFinite(value))throw Error('Enter a finite number.');
    const step=(settings.step||0.01)*(modifiers.shiftKey&&!settings.integer?0.1:modifiers.altKey?10:1);
    const digits=Math.min(12,Math.max(0,Math.ceil(-Math.log10(step))+2));
    const next=Number((value+direction*step).toFixed(digits));
    if(!Number.isFinite(next))return value;
    return Math.max(settings.min??-Infinity,Math.min(settings.max??Infinity,next));
  }
  function bounds(settings,value){
    if(!settings.suggested)return null;
    return {min:Math.min(settings.suggested[0],value),max:Math.max(settings.suggested[1],value)};
  }
  const inlineEnum=labels=>Array.isArray(labels)&&labels.length>=2&&labels.length<=6;
  const el=(tag,cls)=>{const n=document.createElement(tag);if(cls)n.className=cls;return n;};
  const wheelPixels=e=>e.deltaY*(e.deltaMode===1?40:e.deltaMode===2?800:1);
  function restore(id,input){if(focusRequest===id){focusRequest=null;queueMicrotask(()=>{if(input.isConnected)input.focus({preventScroll:true});});}}
  function number(input,options){
    const settings=options.settings||spec(options.behavior,options.key),id=options.id||'number-'+(++serial);
    const host=el('div','numeric-editor'),baseline=Number(input.value),range=settings.suggested?el('input','numeric-slider'):null;
    input.dataset.numericManaged='true';input.step=settings.integer?'1':'any';input.inputMode=settings.integer?'numeric':'decimal';
    if(settings.min!==undefined)input.min=settings.min;if(settings.max!==undefined)input.max=settings.max;
    input.title='Focus to use the wheel or arrow keys · Shift: fine · Alt: coarse · Enter: apply · Escape: cancel';
    host.dataset.editor=id;host.append(input);
    const hint=el('small','numeric-hint');
    if(range){range.type='range';range.step=settings.step;range.setAttribute('aria-label',options.key+' slider');range.title='Drag to set the value; release to apply';host.append(range);hint.textContent=`Suggested ${settings.suggested.join('–')}${settings.unit?' '+settings.unit:''}`;host.append(hint);}
    let editing=false,dirty=false,pointer=false,revision,timer,wheel=0,disposed=false;
    const control={host,get active(){return editing;},flush,cancel,dispose(){disposed=true;cancel();controls.delete(control);}};controls.add(control);
    function begin(){if(!editing){editing=true;revision=options.revision();wheel=0;}}
    function sync(){
      const value=Number(input.value);if(range&&input.value.trim()&&Number.isFinite(value)){const b=bounds(settings,value);range.min=b.min;range.max=b.max;range.value=value;}
      host.classList.toggle('numeric-draft',dirty);
    }
    function draft(value){begin();input.value=String(value);input.setCustomValidity('');dirty=value!==baseline;sync();}
    function cancel(){clearTimeout(timer);editing=false;dirty=false;pointer=false;wheel=0;input.value=String(baseline);input.setCustomValidity('');sync();}
    async function flush(){
      clearTimeout(timer);if(disposed)return false;if(!editing)return true;
      const value=Number(input.value),valid=input.value.trim()&&Number.isFinite(value)&&input.checkValidity();
      if(!valid){input.setCustomValidity('Enter a finite number within the allowed limits.');return false;}
      const changed=dirty&&value!==baseline;editing=false;dirty=false;pointer=false;sync();if(!changed)return true;
      const focused=host.contains(document.activeElement);if(focused)focusRequest=id;
      try{return await options.commit(value,revision);}catch(error){input.setCustomValidity(error.message);return false;}
      finally{if(focusRequest===id){focusRequest=null;if(input.isConnected&&focused)input.focus({preventScroll:true});}}
    }
    const schedule=()=>{clearTimeout(timer);timer=setTimeout(flush,300);};
    input.oninput=()=>{begin();dirty=true;input.setCustomValidity('');sync();};
    input.onchange=()=>{if(!editing){begin();dirty=Number(input.value)!==baseline;}flush();};
    input.onblur=e=>{if(!host.contains(e.relatedTarget)&&!pointer){if(!input.value.trim()||!Number.isFinite(Number(input.value))||!input.checkValidity())cancel();else flush();}};
    input.onkeydown=e=>{
      if(e.key==='Escape'){e.preventDefault();e.stopPropagation();cancel();return;}
      if(e.key==='Enter'){e.preventDefault();e.stopPropagation();flush();return;}
      if((e.key==='ArrowUp'||e.key==='ArrowDown')&&!e.ctrlKey&&!e.metaKey){e.preventDefault();e.stopPropagation();draft(nudge(Number(input.value),e.key==='ArrowUp'?1:-1,settings,e));schedule();}
    };
    function onWheel(e){
      if(e.ctrlKey||e.metaKey||!e.deltaY||!host.contains(document.activeElement))return;
      const value=Number(input.value);if(!input.value.trim()||!Number.isFinite(value))return;
      e.preventDefault();e.stopPropagation();begin();wheel-=wheelPixels(e);const steps=Math.trunc(wheel/100);if(steps){wheel-=steps*100;draft(nudge(value,Math.max(-20,Math.min(20,steps)),settings,e));}schedule();
    }
    input.addEventListener('wheel',onWheel,{passive:false});
    if(range){
      range.onpointerdown=()=>{begin();pointer=true;};range.oninput=()=>draft(Number(range.value));
      range.onpointerup=()=>{pointer=false;flush();};range.onpointercancel=cancel;
      range.addEventListener('lostpointercapture',()=>{if(pointer)cancel();});
      range.onchange=()=>{if(!pointer)flush();};range.onblur=()=>{if(!pointer)flush();};
      range.onkeydown=e=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();cancel();}};
      range.addEventListener('wheel',onWheel,{passive:false});
    }
    sync();restore(id,input);return host;
  }
  function enumChoices(options){
    const host=el('div','enum-choices'),index=options.labels.includes(options.value)?options.labels.indexOf(options.value):Number(options.value);
    host.dataset.editor=options.id;host.setAttribute('role','radiogroup');host.setAttribute('aria-label',options.key);let busy=false;
    const buttons=options.labels.map((label,i)=>{
      const b=el('button');b.type='button';b.textContent=label;b.setAttribute('role','radio');b.setAttribute('aria-label',label);b.setAttribute('aria-checked',String(index===i));b.tabIndex=index===i||(!Number.isInteger(index)&&i===0)?0:-1;
      b.onpointerdown=e=>{if([...controls].some(c=>c.active))e.preventDefault();};
      b.onclick=async e=>{
        e.stopPropagation();if(busy||i===index)return;busy=true;buttons.forEach(n=>n.disabled=true);
        try{
          const drafts=[...controls].filter(c=>c.active);
          for(const control of drafts)if(await control.flush()===false)return;
          focusRequest=options.id;
          await options.commit(i,drafts.length?options.revision():undefined);
        }finally{busy=false;buttons.forEach(n=>n.disabled=false);if(focusRequest===options.id){focusRequest=null;if(b.isConnected)b.focus({preventScroll:true});}}
      };
      b.onkeydown=e=>{let next;if(['ArrowRight','ArrowDown'].includes(e.key))next=(i+1)%options.labels.length;else if(['ArrowLeft','ArrowUp'].includes(e.key))next=(i+options.labels.length-1)%options.labels.length;else if(e.key==='Home')next=0;else if(e.key==='End')next=options.labels.length-1;else return;e.preventDefault();e.stopPropagation();buttons[next].click();};host.append(b);return b;
    });
    if(!buttons.some((b,i)=>i===index)){buttons[0].tabIndex=0;const missing=el('small');missing.textContent='Current: '+String(options.value);host.append(missing);}
    restore(options.id,buttons[index]||buttons[0]);return host;
  }
  function cancel(host){
    const focused=document.activeElement,editor=host?.contains(focused)?focused.closest('[data-editor]'):null;
    if(editor&&!focusRequest){const id=editor.dataset.editor;focusRequest=id;queueMicrotask(()=>{if(focusRequest===id)focusRequest=null;});}
    for(const c of [...controls])if(!host||host.contains(c.host))c.dispose();
  }
  const fallback=new Map();
  function install(){
    document.addEventListener('wheel',e=>{
      const input=e.target.closest?.('input[type=number]');
      if(!input||input.disabled||input.readOnly||input.dataset.numericManaged||document.activeElement!==input||e.ctrlKey||e.metaKey||!e.deltaY)return;
      if(!input.value.trim()||!Number.isFinite(Number(input.value)))return;
      e.preventDefault();e.stopPropagation();
      const state=fallback.get(input)||{wheel:0,changed:false};clearTimeout(state.timer);state.wheel-=wheelPixels(e);
      const steps=Math.trunc(state.wheel/100);if(steps){
        state.wheel-=steps*100;const step=Number(input.step)>0?Number(input.step):0.01,current=Number(input.value);
        const next=nudge(current,steps,{step,integer:step>=1,min:input.min===''?undefined:Number(input.min),max:input.max===''?undefined:Number(input.max)},e);
        if(next!==current){input.value=String(next);state.changed=true;input.dispatchEvent(new Event('input',{bubbles:true}));}
      }
      state.timer=setTimeout(()=>{fallback.delete(input);if(state.changed&&input.isConnected)input.dispatchEvent(new Event('change',{bubbles:true}));},300);fallback.set(input,state);
    },{passive:false});
    root.addEventListener('blur',()=>{for(const c of controls)c.cancel();for(const state of fallback.values())clearTimeout(state.timer);fallback.clear();});
  }
  const api={spec,nudge,bounds,inlineEnum,number,enumChoices,cancel,active:()=>[...controls].some(c=>c.active)};
  root.SchemaPropertyEditors=api;if(typeof module!=='undefined')module.exports=api;
  if(typeof document!=='undefined')install();
})(globalThis);
