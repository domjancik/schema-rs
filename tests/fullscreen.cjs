'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
(async()=>{
for(const file of ['index.html','mobile.html'])assert.match(fs.readFileSync('web/'+file,'utf8'),/id="fullscreen-toggle"/,'Fullscreen control in '+file);
const listeners={},button={hidden:false,disabled:false,textContent:'',attrs:{},setAttribute(k,v){this.attrs[k]=v;}};
let enter=0,exit=0;
const document={fullscreenEnabled:true,fullscreenElement:null,getElementById:id=>id==='fullscreen-toggle'?button:null,addEventListener:(name,fn)=>listeners[name]=fn,documentElement:{async requestFullscreen(){enter++;document.fullscreenElement=this;listeners.fullscreenchange();}},async exitFullscreen(){exit++;this.fullscreenElement=null;listeners.fullscreenchange();}};
vm.runInNewContext(fs.readFileSync('web/engine-client.js','utf8'),{window:{},document});
assert.equal(button.attrs['aria-pressed'],'false');await button.onclick();assert.equal(enter,1);assert.equal(button.attrs['aria-label'],'Exit fullscreen');assert.equal(button.attrs['aria-pressed'],'true');
await button.onclick();assert.equal(exit,1);assert.equal(button.attrs['aria-pressed'],'false');
await button.onclick();document.fullscreenElement=null;listeners.fullscreenchange();assert.equal(button.attrs['aria-pressed'],'false','Escape updates control');
document.documentElement.requestFullscreen=async()=>{throw Error('denied');};await button.onclick();assert.equal(button.disabled,false);assert.match(button.title,/unavailable/i);
document.fullscreenEnabled=false;vm.runInNewContext(fs.readFileSync('web/engine-client.js','utf8'),{window:{},document});assert.equal(button.hidden,true);
console.log('Fullscreen: both views, enter/exit, Escape, rejection, unsupported browser passed.');
})().catch(e=>{console.error(e);process.exitCode=1;});
