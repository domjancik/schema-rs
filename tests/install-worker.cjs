const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
(async()=>{const source=fs.readFileSync('web/mobile-worker.js','utf8'),events={};let response;
vm.runInNewContext(source,{self:{addEventListener:(name,fn)=>events[name]=fn,location:{origin:'https://schema.test'}},URL,fetch:()=>Promise.reject(Error('offline')),caches:{match:async()=>({offline:true})}});
events.fetch({request:{url:'https://schema.test/api/state',method:'GET',mode:'cors'},respondWith:()=>{throw Error('API must never be intercepted');}});
events.fetch({request:{url:'https://schema.test/api/command',method:'POST',mode:'cors'},respondWith:()=>{throw Error('Commands must never be intercepted');}});
events.fetch({request:{url:'https://schema.test/mobile',method:'GET',mode:'navigate'},respondWith:value=>response=value});assert.deepEqual(await response,{offline:true});
console.log('Mobile service worker leaves live state and commands uncached; navigation gets disconnected fallback.');
})().catch(e=>{console.error(e);process.exitCode=1});
