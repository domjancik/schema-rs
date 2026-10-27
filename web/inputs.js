/* Browser hardware acquisition. All analysis and authored state remain engine owned. */
'use strict';
(()=>{
 let active=false;
 const hardwareButtons=['audio-input','midi-input'].map(id=>document.getElementById(id));
 hardwareButtons.forEach(button=>button.hidden=true);
 const token=sessionStorage.getItem('schema-token');
 fetch('/api/catalog',{headers:token?{'x-schema-token':token}:{}}).then(r=>r.json()).then(rows=>{active=rows.some(row=>row.key==='Create.Value.OSC');hardwareButtons.forEach(button=>button.hidden=!active);}).catch(()=>{});
 const inflight=new Set(),pending=new Map();
 async function send(event){if(!active)return;const key=[event.type,event.index??event.slot??''].join('/');if(inflight.has(key)){pending.set(key,event);return;}inflight.add(key);try{const token=sessionStorage.getItem('schema-token');const response=await fetch('/api/inputs',{method:'POST',headers:{'Content-Type':'application/json',...(token?{'x-schema-token':token}:{})},body:JSON.stringify(event)});if(!response.ok&&response.status!==404)console.warn('Input rejected',response.status);}catch(error){console.warn('Input unavailable',error.message);}finally{inflight.delete(key);const next=pending.get(key);if(next){pending.delete(key);send(next);}}}
 window.SchemaInputs={send};
 const keys=new Set();const editing=()=>/INPUT|TEXTAREA|SELECT/.test(document.activeElement?.tagName||'');
 function keyName(e){return e.code==='Space'?'Space':e.key.length===1?e.key.toUpperCase():e.key;}
 window.addEventListener('keydown',e=>{if(editing()||e.repeat)return;keys.add(keyName(e));send({type:'keyboard',keys:[...keys]});});
 window.addEventListener('keyup',e=>{keys.delete(keyName(e));send({type:'keyboard',keys:[...keys]});});
 window.addEventListener('blur',()=>{keys.clear();send({type:'keyboard',keys:[]});});
 let gamepad='';setInterval(()=>{const pad=Array.from(navigator.getGamepads?.()||[]).find(Boolean);const values=pad?[...pad.axes.slice(0,4),pad.buttons[6]?.value||0,pad.buttons[7]?.value||0,...pad.buttons.slice(0,4).map(b=>b.value)]:[];const signature=JSON.stringify(values);if(signature!==gamepad){gamepad=signature;send({type:'xbox',values});}},50);
 let audio=null;const button=document.getElementById('audio-input');
 button.onclick=async()=>{if(audio){audio.stream.getTracks().forEach(t=>t.stop());audio.processor.disconnect();audio.source.disconnect();await audio.context.close();audio=null;button.textContent='Microphone';button.setAttribute('aria-pressed','false');return;}
  try{const stream=await navigator.mediaDevices.getUserMedia({audio:true});const context=new AudioContext();const source=context.createMediaStreamSource(stream);const processor=context.createScriptProcessor(1024,1,1);const silent=context.createGain();silent.gain.value=0;let last=0;processor.onaudioprocess=e=>{if(performance.now()-last<80)return;last=performance.now();send({type:'audio',slot:0,audio:{samples:Array.from(e.inputBuffer.getChannelData(0)),sample_rate:context.sampleRate}});};source.connect(processor);processor.connect(silent);silent.connect(context.destination);audio={stream,context,processor,source};button.textContent='Stop microphone';button.setAttribute('aria-pressed','true');}catch(error){button.title=error.message;console.error(error);}
 };
 const midiButton=document.getElementById('midi-input');
 midiButton.onclick=async()=>{try{if(!navigator.requestMIDIAccess)throw Error('Web MIDI unavailable in this browser');const access=await navigator.requestMIDIAccess({sysex:false});const bind=()=>{for(const input of access.inputs.values())input.onmidimessage=message=>{const[status,data,value]=message.data;const kind=status&0xf0;if(kind===0xb0)send({type:'controller',index:data,value:value/127});if(kind===0x90||kind===0x80)send({type:'pad',index:data%16,pressed:kind===0x90&&value>0});};};bind();access.onstatechange=bind;midiButton.textContent='MIDI connected';}catch(error){midiButton.title=error.message;console.error(error);}};
})();
