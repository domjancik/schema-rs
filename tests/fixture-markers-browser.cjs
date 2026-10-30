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
 const browser=await chromium.launch({channel:'chrome',headless:true});
 try{
 const page=await browser.newPage({viewport:{width:1440,height:950}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(12000);
 await page.goto(base+(process.env.SCHEMA_URL?'/?view=studio':'/play?view=studio'));await page.getByLabel('Fixture marker color',{exact:true}).waitFor();
 await page.evaluate(async()=>{await SchemaEngine.request('/api/command',{command:{type:'pause',paused:true}});const state=await SchemaEngine.request('/api/state');await SchemaEngine.request('/api/command',{expected_revision:state.snapshot.revision,command:{type:'batch',commands:state.project.fixtures.map(f=>({type:'set_fixture',fixture:{...f,marker_color:null}}))}});});await page.waitForFunction(()=>!document.querySelector('#fixtures .fixture-marked'));
 const state=()=>page.evaluate(()=>SchemaEngine.request('/api/state'));const before=await state(),first=before.project.fixtures[0],second=before.project.fixtures[1];
 const color=async value=>page.getByLabel('Fixture marker color',{exact:true}).evaluate((input,value)=>{input.value=value;input.dispatchEvent(new Event('change',{bubbles:true}));},value);
 await color('#a78bfa');await page.locator('#fixtures .fixture-marked').waitFor();let after=await state();assert.equal(after.project.fixtures[0].marker_color,'#a78bfa');assert.deepEqual(after.snapshot.universes,before.snapshot.universes);assert.deepEqual(after.snapshot.fixtures[0].result,before.snapshot.fixtures[0].result);
 assert.equal(await page.locator('#stage [data-fixture="'+first.id+'"]').getAttribute('fill'),'#a78bfa');
 await page.locator('#undo').click();await page.waitForFunction(()=>!document.querySelector('#fixtures .fixture-marked'));await page.locator('#redo').click();await page.locator('#fixtures .fixture-marked').waitFor();
 await page.locator('#fixtures button').filter({hasText:second.name}).click({modifiers:['Shift']});assert(await page.getByRole('button',{name:'Clear fixture marker',exact:true}).isEnabled(),'An unmarked active fixture must not disable clearing marked peers');
 await page.getByRole('button',{name:'Clear fixture marker',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('#fixtures .fixture-marked'));
 await page.locator('#fixtures button').filter({hasText:second.name}).click({modifiers:['Shift']});const revision=(await state()).snapshot.revision;await color('#f0e442');await page.waitForFunction(()=>document.querySelectorAll('#fixtures .fixture-marked').length===2);after=await state();assert.equal(after.snapshot.revision,revision+1);assert.equal(after.project.fixtures[0].marker_color,'#f0e442');assert.equal(after.project.fixtures[1].marker_color,'#f0e442');
 await page.getByRole('button',{name:'Find fixture',exact:true}).click();await page.locator('dialog[open] .fixture-marked').first().waitFor();assert.equal(await page.locator('dialog[open] .fixture-marked').count(),2);await page.locator('dialog[open] .fixture-marked').first().click();
 await page.reload();await page.locator('#fixtures .fixture-marked').first().waitFor();assert.equal((await state()).project.fixtures[0].marker_color,'#f0e442');
 await page.setViewportSize({width:390,height:844});await page.goto(base+'/mobile');await page.locator('#scene-title.fixture-marked').waitFor();await page.locator('.app-menu>summary').click();await page.locator('#fixture-menu-setup').click();await page.getByLabel('Fixture marker color',{exact:true}).waitFor();await color('#56b4e9');await page.waitForFunction(()=>document.querySelector('#scene-title').style.getPropertyValue('--fixture-marker')==='#56b4e9');
 await page.getByLabel('X position',{exact:true}).fill('1.25');await page.getByRole('button',{name:'Apply position',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('#menu').open);after=await state();assert.equal(after.project.fixtures[0].position[0],1.25);assert.equal(after.project.fixtures[0].marker_color,'#56b4e9');await page.locator('.view-switch [data-view="3d"]').click();await page.waitForFunction(()=>document.querySelector('#scene [data-scene-fixture="fixture-1"] .fixture-layout-dot')?.getAttribute('fill')==='#56b4e9');
 await page.locator('.app-menu>summary').click();await page.locator('#fixture-menu-setup').click();await page.getByRole('button',{name:'Clear fixture marker',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('#scene-title').classList.contains('fixture-marked'));
 assert.deepEqual(errors,[]);console.log('Fixture markers: Studio edit/clear, shared batch undo, unchanged DMX/light colors, saved reload, finder swatches and mobile editing/position revision passed');
 }finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(e=>{console.error(e);process.exitCode=1;server.close();});
