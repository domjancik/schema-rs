const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const root = path.resolve('target/browser-site');
assert(fs.existsSync(path.join(root, 'studio.html')), 'Build the browser host first');
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
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const base='http://127.0.0.1:'+server.address().port;
 const browser=await chromium.launch({channel:'chrome',headless:true});
 try{
 const page=await browser.newPage({viewport:{width:1500,height:950},hasTouch:true}),errors=[];page.setDefaultTimeout(8000);page.on('pageerror',e=>errors.push(e.message));
 await page.goto(base+'/play?view=studio');await page.waitForFunction(()=>SchemaEngine?.kind==='browser'&&document.querySelector('#stage circle[data-fixture]'));
 await page.evaluate(async()=>{const s=await SchemaEngine.request('/api/state');s.project.fixtures.forEach((f,i)=>f.position=i? [5+i*2,0,5]:[-2,1,0]);await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'replace_project',project:s.project}});window.writes=[];const request=SchemaEngine.request.bind(SchemaEngine);SchemaEngine.request=(path,body)=>{if(path==='/api/command')writes.push(body.command);return request(path,body);};});
 await page.waitForFunction(()=>Number(document.querySelector('#stage [data-fixture="fixture-1"]').getAttribute('cx'))===115);
 const before=await page.evaluate(()=>SchemaEngine.request('/api/state')),stage=page.locator('#stage');
 const point=async(x,y)=>stage.evaluate((n,p)=>{const q=new DOMPoint(p.x,p.y).matrixTransform(n.getScreenCTM());return{x:q.x,y:q.y};},{x,y});
 const a=await point(25,25);await page.mouse.move(a.x,a.y);await page.mouse.down();await page.mouse.move(a.x+35,a.y+20,{steps:5});await page.mouse.up();
 await page.waitForFunction(()=>Number(document.querySelector('#stage').dataset.cameraX)!==0);
 const center=await point(140,120),zoom=Number(await stage.getAttribute('data-camera-zoom'));await page.mouse.move(center.x,center.y);await page.mouse.wheel(0,-250);
 await page.waitForFunction(z=>Number(document.querySelector('#stage').dataset.cameraZoom)>z,zoom);
 await page.getByRole('button',{name:'Fit fixtures in view',exact:true}).click();await page.waitForTimeout(100);
 assert.deepEqual((await page.evaluate(()=>SchemaEngine.request('/api/state'))).project,before.project);
 assert.equal(await page.evaluate(()=>writes.length),0,'pan, wheel and fit must not author edits');
 const camera=await stage.evaluate(n=>[n.dataset.cameraX,n.dataset.cameraZ,n.dataset.cameraZoom]);await page.reload();await stage.waitFor();await page.waitForFunction(()=>document.querySelector('#stage [data-fixture="fixture-1"]'));
 assert.deepEqual(await stage.evaluate(n=>[n.dataset.cameraX,n.dataset.cameraZ,n.dataset.cameraZoom]),camera,'camera survives reload');
 await page.evaluate(()=>{window.writes=[];const request=SchemaEngine.request.bind(SchemaEngine);SchemaEngine.request=(path,body)=>{if(path==='/api/command')writes.push(body.command);return request(path,body);};});
 const dragPoint=()=>page.locator('#stage [data-fixture="fixture-1"]').evaluate(n=>{const m=n.getScreenCTM(),p=new DOMPoint(Number(n.getAttribute('cx')),Number(n.getAttribute('cy'))).matrixTransform(m),z=Number(document.querySelector('#stage').dataset.cameraZoom);return{x:p.x,y:p.y,dx:25*z*m.a};});
 let p=await dragPoint();await page.mouse.move(p.x,p.y);await page.mouse.down();await page.mouse.move(p.x+p.dx,p.y,{steps:5});await page.mouse.up();
 await page.waitForFunction(()=>writes.length===1);assert.deepEqual(await page.evaluate(()=>writes[0].position),[0,1,0],'fixture drag uses zoomed world units');
 await page.getByRole('button',{name:'Pan fixture view',exact:true}).click();p=await dragPoint();await page.mouse.move(p.x,p.y);await page.mouse.down();await page.mouse.move(p.x+20,p.y+10,{steps:3});await page.mouse.up();assert.equal(await page.evaluate(()=>writes.length),1,'Pan mode over fixture must not move it');
 await page.getByRole('button',{name:'Pan fixture view',exact:true}).click();
 await page.setViewportSize({width:390,height:844});await page.getByRole('navigation',{name:'Studio panels',exact:true}).getByRole('button',{name:'Fixture setup',exact:true}).tap();
 await page.getByRole('button',{name:'Fit fixtures in view',exact:true}).tap();await page.waitForTimeout(100);p=await dragPoint();
 const cdp=await page.context().newCDPSession(page),touch=(type,pts)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:pts});
 await touch('touchStart',[{x:p.x,y:p.y,id:1}]);await touch('touchStart',[{x:p.x,y:p.y,id:1},{x:p.x+55,y:p.y+20,id:2}]);
 const beforePinch=Number(await stage.getAttribute('data-camera-zoom'));
 await touch('touchMove',[{x:p.x-12,y:p.y,id:1},{x:p.x+75,y:p.y+20,id:2}]);await touch('touchEnd',[]);
 await page.waitForFunction(z=>Number(document.querySelector('#stage').dataset.cameraZoom)>z,beforePinch);assert.equal(await page.evaluate(()=>writes.length),1,'pinch starting on a fixture cancels its drag');
 // Crowded handles must not open a modal before a second finger can join.
 await page.evaluate(async()=>{const s=await SchemaEngine.request('/api/state');s.project.fixtures[1].position=[...s.project.fixtures[0].position];await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'replace_project',project:s.project}});});
 const baseline=await page.evaluate(()=>writes.length);await page.waitForTimeout(150);p=await dragPoint();
 await touch('touchStart',[{x:p.x,y:p.y,id:3}]);
 assert.equal(await page.getByRole('dialog',{name:'Choose fixture',exact:true}).count(),0,'touch picking waits until lift so pinch can start over overlaps');
 await touch('touchStart',[{x:p.x,y:p.y,id:3},{x:p.x+45,y:p.y+15,id:4}]);
 await touch('touchMove',[{x:p.x-10,y:p.y,id:3},{x:p.x+65,y:p.y+15,id:4}]);await touch('touchEnd',[]);
 assert.equal(await page.evaluate(()=>writes.length),baseline);assert.equal(await page.getByRole('dialog',{name:'Choose fixture',exact:true}).count(),0);
 p=await dragPoint();await touch('touchStart',[{x:p.x,y:p.y,id:5}]);await touch('touchEnd',[]);
 const picker=page.getByRole('dialog',{name:'Choose fixture',exact:true});await picker.waitFor();await picker.getByRole('button',{name:new RegExp(before.project.fixtures[0].name)}).click();
 for(const button of await page.locator('.stage-navigation button').all()){const box=await button.boundingBox();assert(box.width>=44&&box.height>=44,'phone navigation controls are touch sized');}
 await page.getByRole('button',{name:'Reset fixture view',exact:true}).tap();await page.waitForFunction(()=>document.querySelector('#stage').dataset.cameraZoom==='1');
 assert.equal(await stage.getAttribute('data-camera-x'),'0');assert.equal(await stage.getAttribute('data-camera-z'),'0');
 await stage.focus();await page.keyboard.press('+');await page.waitForFunction(()=>Number(document.querySelector('#stage').dataset.cameraZoom)>1);
 assert.equal(await page.evaluate(()=>writes.length),baseline);
 assert.equal(await stage.evaluate(n=>{const event=new KeyboardEvent('keydown',{key:'f',ctrlKey:true,bubbles:true,cancelable:true});n.dispatchEvent(event);return event.defaultPrevented;}),false,'browser Find shortcut is untouched');
 assert.deepEqual(errors,[]);
 await page.screenshot({path:'target/fixture-setup-navigation-phone.png'});
 console.log('Fixture setup: pan/wheel/fit/reset, saved camera, zoom-correct one-command fixture drag, Pan mode and touch pinch cancellation passed.');
 }finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(e=>{console.error(e);server.close();process.exitCode=1;});
