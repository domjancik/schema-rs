const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const root = path.resolve('target/browser-site');
if(!process.env.SCHEMA_URL)assert(fs.existsSync(path.join(root, 'studio.html')), 'Build the browser host first');
const types = {'.html':'text/html','.js':'text/javascript','.mjs':'text/javascript','.wasm':'application/wasm','.json':'application/json','.css':'text/css','.svg':'image/svg+xml'};
const server = http.createServer((req,res) => {
  const url = new URL(req.url,'http://localhost');
  let name = ['/play','/app'].includes(url.pathname) ? '/studio.html' : url.pathname === '/mobile' ? '/mobile.html' : url.pathname === '/preview' ? '/preview.html' : url.pathname;
  const file = path.resolve(root,'.'+name);
  const config=JSON.parse(fs.readFileSync(path.resolve('scripts/publication/vercel.json'),'utf8'));
  const policy=config.headers.find(group=>group.source===url.pathname)||config.headers[0];
  for(const header of policy.headers)res.setHeader(header.key,header.value);
  if (!file.startsWith(root+path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {res.writeHead(404);res.end();return;}
  res.setHeader('Content-Type',types[path.extname(file)]||'application/octet-stream');
  fs.createReadStream(file).pipe(res);
});
(async()=>{
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const base=process.env.SCHEMA_URL||'http://127.0.0.1:'+server.address().port;
 const browser=await chromium.launch({channel:'chrome',headless:true});try{
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(8000);
 await page.goto(base+(process.env.SCHEMA_URL?'/?view=studio':'/play?view=studio'));await page.locator('#fixtures button').first().waitFor();
 await page.evaluate(async()=>{const s=await SchemaEngine.request('/api/state');if(s.project.output.enabled)throw Error('Use capture-only test workspace');await SchemaEngine.request('/api/import',{fixture:s.project.fixtures[0].id,layer:0,expected_revision:s.snapshot.revision,yaml:'name: Editor test\nstack:\n  Create.Value.LFO: {Period: 4, Type: 0}\n'});});
 const inspector=page.locator('#inspector'),period=inspector.getByRole('spinbutton',{name:'Period',exact:true}),slider=inspector.getByRole('slider',{name:'Period slider',exact:true});await slider.waitFor();
 const until=async predicate=>{for(let i=0;i<200;i++){if(await predicate())return;await page.waitForTimeout(20);}throw Error('Expected engine state was not published');};
 const state=()=>page.evaluate(()=>SchemaEngine.request('/api/state'));const value=async()=>{const s=await state();return s.project.content[s.project.fixtures[0].id][0].root.properties.Period;};
 assert.equal(await inspector.getByRole('radiogroup',{name:'Type',exact:true}).getByRole('radio').count(),3);let before=await state();await inspector.getByRole('radio',{name:'Sine',exact:true}).click();await until(async()=>{const s=await state();return s.project.content[s.project.fixtures[0].id][0].root.properties.Type===2;});assert.equal((await state()).snapshot.revision,before.snapshot.revision+1);
 await period.focus();await period.hover();before=await state();await page.mouse.wheel(0,-100);await page.mouse.wheel(0,-100);await until(async()=>await value()===4.1);assert.equal((await state()).snapshot.revision,before.snapshot.revision+1,'One wheel burst is one history edit');assert.equal(await period.evaluate(n=>document.activeElement===n),true,'Keep focus after wheel commit');
 before=await state();await period.fill('120');await period.press('Enter');await until(async()=>await value()===120);assert.equal(Number(await slider.getAttribute('max')),120,'Manual entry outside suggested range stays usable');
 before=await state();await slider.evaluate(n=>{n.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:9,button:0}));n.value='10';n.dispatchEvent(new Event('input',{bubbles:true}));n.value='12';n.dispatchEvent(new Event('input',{bubbles:true}));});assert.equal(await value(),120,'Slider draft is not an engine edit');await slider.evaluate(n=>n.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId:9,button:0})));await until(async()=>await value()===12);assert.equal((await state()).snapshot.revision,before.snapshot.revision+1);
 before=await state();await period.fill('55');await period.press('Escape');assert.equal(await period.inputValue(),'12');assert.equal((await state()).snapshot.revision,before.snapshot.revision);

 // Wheel scrolling over an unfocused number is ordinary scrolling, and a canceled drag is not authored.
 await page.locator('#stack-name').click();const prevented=await period.evaluate(n=>{const e=new WheelEvent('wheel',{deltaY:-100,bubbles:true,cancelable:true});n.dispatchEvent(e);return e.defaultPrevented;});assert.equal(prevented,false);
 before=await state();await slider.evaluate(n=>{n.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:10,button:0}));n.value='9';n.dispatchEvent(new Event('input',{bubbles:true}));n.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:10}));});assert.equal(await value(),12);assert.equal((await state()).snapshot.revision,before.snapshot.revision);
 await period.fill('');await period.press('Enter');assert.equal(await period.evaluate(n=>n.checkValidity()),false);await page.locator('#stack-name').click();assert.equal(await period.inputValue(),'12');assert.equal(await page.evaluate(()=>SchemaPropertyEditors.active()),false);
 await period.focus();await period.press('Shift+ArrowUp');await until(async()=>await value()===12.005);await page.locator('#undo').click();await until(async()=>await value()===12);
 // Native group/radio keyboard navigation remains a single engine edit.
 await inspector.getByRole('radio',{name:'Sine',exact:true}).focus();await inspector.getByRole('radio',{name:'Sine',exact:true}).press('ArrowLeft');await until(async()=>{const s=await state();return s.project.content[s.project.fixtures[0].id][0].root.properties.Type===1;});
 // Selecting an enum after typing a number must settle that edit instead of losing the click.
 await period.fill('7');await inspector.getByRole('radio',{name:'Sawtooth',exact:true}).click();await until(async()=>{const s=await state(),p=s.project.content[s.project.fixtures[0].id][0].root.properties;return p.Period===7&&p.Type===0;});
 await page.locator('#undo').click();await until(async()=>{const s=await state();return s.project.content[s.project.fixtures[0].id][0].root.properties.Type===1;});await page.locator('#undo').click();await until(async()=>await value()===12);

 // Real pointer dragging, including release beyond the slider, commits once.
 before=await state();const rect=await slider.boundingBox();await page.mouse.move(rect.x+rect.width*.4,rect.y+rect.height/2);await page.mouse.down();await page.mouse.move(rect.x+rect.width+30,rect.y+rect.height/2,{steps:5});assert.equal((await state()).snapshot.revision,before.snapshot.revision);await page.mouse.up();await until(async()=>await value()!==12);assert.equal((await state()).snapshot.revision,before.snapshot.revision+1);await page.locator('#undo').click();await until(async()=>await value()===12);
 // Generic numeric dialogs get focused wheel input, without noop change events.
 const fallback=await page.evaluate(async()=>{const input=document.createElement('input');input.type='number';input.value='2';input.max='5';document.body.append(input);input.focus();let changes=0;input.onchange=()=>changes++;input.dispatchEvent(new WheelEvent('wheel',{deltaY:-5,bubbles:true,cancelable:true}));await new Promise(r=>setTimeout(r,350));const untouched=changes;input.dispatchEvent(new WheelEvent('wheel',{deltaY:-100,bubbles:true,cancelable:true}));await new Promise(r=>setTimeout(r,350));const result={untouched,changes,value:Number(input.value)};input.remove();return result;});assert.deepEqual(fallback,{untouched:0,changes:1,value:3});
 // Another author's edit during a draft must reject the old revision, not overwrite it.
 await period.fill('8');await page.evaluate(async()=>{const s=await SchemaEngine.request('/api/state');await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'set_dimmer',fixture:s.project.fixtures[0].id,value:.73}});});await period.press('Enter');await page.waitForFunction(()=>document.querySelector('#status').dataset.error==='true');assert.equal(await value(),12);


 // Fine authored dim levels survive exact entry and precision nudging.
 await period.fill('0.00625');await period.press('Enter');await until(async()=>await value()===0.00625);assert.equal(await period.inputValue(),'0.00625');await period.press('Shift+ArrowUp');await until(async()=>await value()===0.01125);
 // Fixture setup shares the editor and the captured revision semantics.
 const coordinate=page.locator('#fixture-form').getByRole('spinbutton',{name:'X',exact:true});await coordinate.scrollIntoViewIfNeeded();await coordinate.focus();await coordinate.hover();before=await state();const x=before.project.fixtures[0].position[0];await page.mouse.wheel(0,-100);await until(async()=>{const s=await state();return Math.abs(s.project.fixtures[0].position[0]-(x+0.1))<1e-9;});assert.equal((await state()).snapshot.revision,before.snapshot.revision+1);
 before=await state();await page.locator('#fixture-form').getByRole('slider',{name:'Fixture dimmer slider',exact:true}).evaluate(n=>{n.value='0.5';n.dispatchEvent(new Event('input',{bubbles:true}));n.dispatchEvent(new Event('change',{bubbles:true}));});await until(async()=>(await state()).project.fixtures[0].dimmer===0.5);assert.equal((await state()).snapshot.revision,before.snapshot.revision+1);
 await page.screenshot({path:'target/property-editors-studio.png'});assert.deepEqual(errors,[]);console.log('Studio editors: visible enums, focused wheel single undo/focus retention, exact extended ranges, deferred slider, Escape and stale-revision rejection passed');
 }finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1});
