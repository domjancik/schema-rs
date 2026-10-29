import {SchemaPlayer,bindCSS} from './schema-player.mjs';
const status=document.getElementById('value');
try{
 const player=await SchemaPlayer.create();window.demoPlayer=player;player.onerror=error=>status.textContent=error.message;
 const state=await player.request('/api/state');
 await player.request('/api/import',{fixture:'fixture-1',layer:0,yaml:'name: Website motion\nstack:\n  Create.Value.LFO: {Period: 3, Type: 0}\n',expected_revision:state.snapshot.revision});
 const elements=[...document.querySelectorAll('.shape')];
 const unbind=elements.map((element,index)=>bindCSS(player,element,{fixture:'fixture-1',map:value=>(value+index/3)%1}));
 player.subscribe(frame=>{const value=frame.fixtures[0]?.result.value;status.textContent=`Engine ${frame.time.toFixed(2)}s · LFO ${value?.toFixed(3)}`;});
 let paused=false;document.getElementById('pause').onclick=async()=>{paused=!paused;const state=await player.request('/api/state');await player.command({type:'pause',paused},state.snapshot.revision);document.getElementById('pause').textContent=paused?'Resume':'Pause';};
 addEventListener('pagehide',()=>{unbind.forEach(stop=>stop());player.dispose();},{once:true});
}catch(error){status.textContent=error.message;}
