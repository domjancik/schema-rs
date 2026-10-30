// Embeddable consumer API: typed scene values, no DOM or physical output in the engine.
export class SchemaPlayer {
  constructor(options={}){
    this.kind='browser';this.pending=new Map();this.listeners=new Set();this.serial=0;this.closed=false;
    this.worker=new Worker(new URL('./browser-worker.mjs',import.meta.url),{type:'module'});
    this.worker.onmessage=({data})=>{
      if(data.type==='frame'){this.lastFrame=data.frame;for(const fn of this.listeners){try{fn(data.frame);}catch(error){this.onerror?.(error);}}return;}
      if(data.type==='fatal'||data.type==='storage-error'){this.onerror?.(Error(data.error));if(data.type==='fatal')this.fail(Error(data.error));return;}
      const pending=this.pending.get(data.id);if(!pending)return;this.pending.delete(data.id);clearTimeout(pending.timer);data.error?pending.reject(Error(data.error)):pending.resolve(data.result);
    };
    this.worker.onerror=event=>{const error=Error(event.message||'Browser engine failed to load');this.fail(error);this.onerror?.(error);};
    this.ready=this.call('init',{options}).then(capabilities=>{this.capabilities=capabilities;return this;});
  }
  static async create(options={}){const engine=new SchemaPlayer({persist:false,...options});try{await engine.ready;return engine;}catch(error){engine.dispose();throw error;}}
  call(method,body={}){return new Promise((resolve,reject)=>{
    if(this.closed){reject(Error('Engine disposed'));return;}const id=++this.serial;
    const timer=setTimeout(()=>{this.pending.delete(id);reject(Error('Browser engine request timed out'));},30000);
    this.pending.set(id,{resolve,reject,timer});try{this.worker.postMessage({id,method,...body});}catch(error){this.pending.delete(id);clearTimeout(timer);reject(error);}
  });}
  async request(path,body){await this.ready;return this.call('request',{path,body});}
  async command(command,expectedRevision){return this.request('/api/command',{command,expected_revision:expectedRevision});}
  async input(event){return this.request('/api/inputs',event);}
  async advance(dt){await this.ready;return this.call('advance',{dt});}
  async checkpoint(){return this.request('/api/session');}
  async checkpointText(){return this.request('/api/session/raw');}
  async restore(session){await this.ready;return this.call('restore',{text:typeof session==='string'?session:JSON.stringify(session)});}
  subscribe(listener){this.listeners.add(listener);this.ready.then(()=>this.call('subscribe',{enabled:true})).catch(error=>this.onerror?.(error));return()=>{this.listeners.delete(listener);if(!this.listeners.size&&!this.closed)this.call('subscribe',{enabled:false}).catch(()=>{});};}
  fail(error){this.closed=true;this.worker.terminate();this.listeners.clear();for(const request of this.pending.values()){clearTimeout(request.timer);request.reject(error);}this.pending.clear();}
  dispose(){if(this.closed)return;this.closed=true;this.worker.terminate();this.fail(Error('Engine disposed'));this.listeners.clear();}
}
// Explicit, bounded sink: a scalar bank value maps to one CSS custom property.
// Complex renderers can subscribe directly instead of passing through a DOM adapter.
export function bindCSS(player,element,{fixture,property='--schema-value',map=value=>value}={}){
  if(!/^--[a-zA-Z0-9_-]+$/.test(property))throw Error('Choose a CSS custom property');
  return player.subscribe(frame=>{const value=frame.fixtures.find(row=>row.fixture.id===fixture)?.result.value;if(!Number.isFinite(value))return;const output=map(value);if(Number.isFinite(output))element.style.setProperty(property,String(output));});
}
