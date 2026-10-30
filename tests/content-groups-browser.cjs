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
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 const base=process.env.SCHEMA_URL||'http://127.0.0.1:'+server.address().port;
 const browser=await chromium.launch({channel:'chrome',headless:true});
 try{
 const page=await browser.newPage({viewport:{width:1440,height:950}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(10000);
 await page.goto(base+(process.env.SCHEMA_URL?'/?view=studio':'/play?view=studio'));await page.getByLabel('Fixture marker color',{exact:true}).waitFor();
 const state=()=>page.evaluate(()=>SchemaEngine.request('/api/state'));
 const initial=await state(),a=initial.project.fixtures[0],b=initial.project.fixtures[4];assert(!initial.project.output.enabled);
 await page.locator('#fixtures button').filter({hasText:b.name}).click({modifiers:['Shift']});
 const group=page.locator('#fixture-form').getByRole('checkbox',{name:'Group front',exact:true});assert(await group.evaluate(n=>n.indeterminate));const rev=(await state()).snapshot.revision;
 await group.check();await page.waitForFunction(({id})=>SchemaEngine.request('/api/state').then(s=>s.project.fixtures.find(f=>f.id===id).groups.includes('front')),{id:b.id});let after=await state();assert.equal(after.snapshot.revision,rev+1);assert(after.project.fixtures[4].groups.includes('back'));
 await page.locator('#undo').click();await page.waitForFunction(({id})=>SchemaEngine.request('/api/state').then(s=>!s.project.fixtures.find(f=>f.id===id).groups.includes('front')),{id:b.id});
 await page.getByLabel('New group name',{exact:true}).fill('Tour crew');await page.getByRole('button',{name:'Add group',exact:true}).click();await page.waitForFunction(()=>SchemaEngine.request('/api/state').then(s=>s.project.fixtures.filter(f=>f.groups.includes('Tour crew')).length===2));
 await page.getByLabel('Filter groups',{exact:true}).fill('Tour');assert.equal(await page.locator('#fixture-form .group-option:visible').count(),1);await page.getByLabel('Filter groups',{exact:true}).fill('');
 await page.locator('#apply').click();await page.getByRole('button',{name:'Browse saved content',exact:true}).click();const library=page.getByRole('dialog',{name:'Content library',exact:true});await library.waitFor();
 await library.getByLabel('Snapshot name',{exact:true}).fill('selector-test');await library.getByRole('button',{name:'Save selected layer',exact:true}).click();await page.waitForFunction(()=>document.querySelector('#content-name')?.value==='selector-test');const saved=await state();assert.equal(saved.snapshot.revision,after.snapshot.revision+2,'Group creation is one edit; saving adds no revision');
 await page.screenshot({path:'target/content-library-desktop.png'});await library.getByLabel('Restrict to group',{exact:true}).selectOption('Tour crew');assert((await library.locator('.content-scope').textContent()).includes('2 matching fixtures'));
 await library.getByRole('button',{name:'Close content library',exact:true}).click();
 await page.evaluate(async id=>{const s=await SchemaEngine.request('/api/state');await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'replace_layer',fixture:id,layer:0,stack:null}});},a.id);
 await page.locator('#apply').click();await page.getByRole('button',{name:'Browse saved content',exact:true}).click();await library.getByLabel('Saved content',{exact:true}).selectOption('selector-test');await library.getByLabel('Restrict to group',{exact:true}).selectOption('front');await library.getByRole('button',{name:'Load content',exact:true}).click();await library.waitFor({state:'hidden'});after=await state();assert.deepEqual(after.project.content[a.id][0],saved.project.content[a.id][0]);assert.deepEqual(after.project.fixtures,saved.project.fixtures);
 // A stale content dialog must report a conflict and stay open, never silently rebase.
 await page.locator('#apply').click();await page.getByRole('button',{name:'Browse saved content',exact:true}).click();await library.getByLabel('Saved content',{exact:true}).selectOption('selector-test');await page.evaluate(async id=>{const s=await SchemaEngine.request('/api/state');await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'set_dimmer',fixture:id,value:.42}});},a.id);await library.getByRole('button',{name:'Load content',exact:true}).click();await library.getByRole('status').filter({hasText:/Revision conflict/i}).waitFor();assert(await library.isVisible());await library.getByRole('button',{name:'Close content library',exact:true}).click();
 await page.locator('#fixtures button').filter({hasText:a.name}).click();
 await page.evaluate(async id=>{const s=await SchemaEngine.request('/api/state');await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'replace_layer',fixture:id,layer:0,stack:{name:'Recall',enabled:false,root:{id:9001,behavior:'Assign.Content',properties:{Content:'selector-test',Group:'front'},children:[],drivers:{}}}}});},a.id);
 await page.getByLabel('Content',{exact:true}).waitFor();await page.getByLabel('Content',{exact:true}).selectOption('selector-test');assert.equal(await page.getByLabel('Content',{exact:true}).evaluate(n=>n.tagName),'SELECT');
 await page.setViewportSize({width:390,height:844});await page.goto(base+'/mobile');await page.locator('#fixture-row button').first().waitFor();await page.locator('.app-menu>summary').click();await page.locator('#fixture-menu-setup').click();await page.getByRole('checkbox',{name:'Group Tour crew',exact:true}).waitFor();assert(await page.getByRole('checkbox',{name:'Group Tour crew',exact:true}).isChecked());assert(await page.getByRole('checkbox',{name:'Group front',exact:true}).isChecked());const checkSize=await page.getByRole('checkbox',{name:'Group Tour crew',exact:true}).boundingBox();assert(checkSize.width<=24&&checkSize.height<=24,'Checkboxes must not stretch to full mobile field width');await page.screenshot({path:'target/content-groups-mobile.png'});await page.getByRole('checkbox',{name:'Group Tour crew',exact:true}).uncheck();await page.waitForFunction(()=>SchemaEngine.request('/api/state').then(s=>!s.project.fixtures[0].groups.includes('Tour crew')));
 await page.getByLabel('X position',{exact:true}).fill('1.5');await page.getByRole('button',{name:'Apply position',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('#menu').open);assert.equal((await state()).project.fixtures[0].position[0],1.5);
 await page.getByRole('button',{name:'Place selected fixture',exact:true}).click();const placement=page.getByRole('dialog',{name:'Fixture position',exact:true});await placement.getByRole('checkbox',{name:'Group front',exact:true}).waitFor();assert(await placement.getByRole('checkbox',{name:'Group front',exact:true}).isChecked());await placement.getByLabel('New group name',{exact:true}).fill('Scene group');await placement.getByRole('button',{name:'Add group',exact:true}).click();await placement.getByRole('checkbox',{name:'Group Scene group',exact:true}).waitFor();await placement.getByLabel('X position (m)',{exact:true}).fill('1.75');await placement.getByRole('button',{name:'Apply position',exact:true}).click();await placement.waitFor({state:'hidden'});assert.equal((await state()).project.fixtures[0].position[0],1.75);
 await page.locator('.app-menu>summary').click();await page.locator('#content-library').click();await library.getByLabel('Saved content',{exact:true}).selectOption('selector-test');assert.equal(await library.getByLabel('Restrict to group',{exact:true}).evaluate(n=>n.tagName),'SELECT');await library.getByRole('button',{name:'Close content library',exact:true}).click();
 await page.locator('#save').click();const projects=page.getByRole('dialog',{name:'Projects',exact:true});await projects.getByLabel('Saved projects',{exact:true}).selectOption('preset:carpushit');assert.equal(await projects.getByLabel('Saved project name',{exact:true}).inputValue(),'Carpushit');assert(await projects.getByRole('button',{name:'Save changes',exact:true}).isDisabled());await projects.getByRole('button',{name:'Load preset',exact:true}).click();await projects.waitFor({state:'hidden'});after=await state();assert(after.project.fixtures.length>20);assert(!after.project.output.enabled);
 assert.deepEqual(errors,[]);console.log('Selectors: mixed groups, additive membership, undo, content save/list/scope/load/conflict, mobile setup revision and preset loading passed.');
 }finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1});
