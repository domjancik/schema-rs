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
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(9000);await page.goto(base+(process.env.SCHEMA_URL?'/?view=studio':'/play?view=studio'));await page.locator('#fixtures button').first().waitFor();
 const target=await page.evaluate(async()=>{const s=await SchemaEngine.request('/api/state');if(s.project.output.enabled)throw Error('Use a capture-only workspace');const p=s.project,f=p.fixtures[0].id,leaf=id=>({id,behavior:'Create.Value.Constant',properties:{Value:.5},children:[],drivers:{}});p.content[f]=[{name:'Nested example',root:{id:1,behavior:'Modify.Value.MapTo',properties:{Minimum:.2,Maximum:1},children:[leaf(2)],drivers:{Minimum:{id:3,behavior:'Create.Value.LFO',properties:{Period:2,Type:2},children:[],drivers:{Period:leaf(5)}}}}},{name:'Other program',root:leaf(10)},null];p.director=[{name:'Director program',root:leaf(20)}];await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'replace_project',project:p}});return f;});
 const state=()=>page.evaluate(()=>SchemaEngine.request('/api/session'));const until=async predicate=>{for(let i=0;i<200;i++){if(await predicate())return;await page.waitForTimeout(20);}throw Error('Navigation state not rendered');};
 const nav=page.locator('#stack-breadcrumbs');await nav.getByRole('button',{name:'Go to layer root',exact:true}).waitFor();await until(async()=>await nav.getByLabel('Breadcrumb layer',{exact:true}).inputValue()==='0');const before=await state();
 await page.locator('#inspector').getByTitle('Edit driver',{exact:true}).click();await nav.getByRole('button',{name:'Open Minimum driver',exact:true}).waitFor();await page.locator('#inspector').getByTitle('Edit driver',{exact:true}).click();await nav.getByRole('button',{name:'Open Period driver',exact:true}).waitFor();assert.equal(await nav.locator('[data-driver-key]').count(),2);
 await nav.getByRole('button',{name:'Open Minimum driver',exact:true}).click();await until(async()=>await nav.locator('[data-driver-key]').count()===1);assert((await page.locator('#inspector .key').textContent()).includes('Create.Value.LFO'));
 await nav.getByRole('button',{name:'Go to layer root',exact:true}).click();await until(async()=>await nav.locator('[data-driver-key]').count()===0);assert((await page.locator('#inspector .key').textContent()).includes('Modify.Value.MapTo'));

 // Breadcrumbs reveal a selected block that was hidden by a folded ancestor.
 await page.evaluate(()=>{ui.folded.add(1);chooseNode(2);});await nav.getByRole('button',{name:'Select Constant value block',exact:true}).click();await page.locator('#world .block[data-node="2"]').waitFor();assert.equal(await page.evaluate(()=>ui.folded.has(1)),false);
 await nav.getByLabel('Breadcrumb layer',{exact:true}).selectOption('1');await until(async()=>await page.locator('#stack-name').textContent()==='Other program');await nav.getByRole('button',{name:'Go to layer root',exact:true}).press('Enter');
 await nav.getByLabel('Breadcrumb fixture',{exact:true}).selectOption('__director');await until(async()=>await page.locator('#stack-name').textContent()==='Director program');await nav.getByLabel('Breadcrumb fixture',{exact:true}).selectOption(target);await nav.getByLabel('Breadcrumb layer',{exact:true}).selectOption('2');await page.locator('#empty').waitFor();assert.equal(await nav.getByRole('button',{name:'Go to layer root',exact:true}).getAttribute('aria-current'),'location');
 await page.setViewportSize({width:390,height:844});await page.getByRole('navigation',{name:'Studio panels',exact:true}).getByRole('button',{name:'Stack editor',exact:true}).click();await nav.getByRole('button',{name:'Go to layer root',exact:true}).waitFor();const bounds=await nav.getByRole('button',{name:'Go to layer root',exact:true}).boundingBox();assert(bounds.x>=0&&bounds.x+bounds.width<=390,'Root must remain reachable on narrow screens');await page.screenshot({path:'target/stack-breadcrumbs-mobile.png'});await page.setViewportSize({width:1440,height:1000});
 await page.locator('#split-layers').click();const secondary=page.locator('.workspace-panel[data-view=stack][data-layer="1"]');await secondary.getByRole('navigation',{name:'Stack breadcrumbs',exact:true}).waitFor();const otherBefore=await page.evaluate(()=>SchemaWorkspace.panels(SchemaWorkspace.active(workspaceUI.snapshot()).root).find(p=>p.view==='stack'&&p.context.layer===0).context);await secondary.getByRole('button',{name:'Go to layer root',exact:true}).click();await secondary.locator('#canvas').waitFor();assert.equal(await page.locator('#stack-name').textContent(),'Other program');assert.equal(await page.locator('.workspace-panel[data-view=stack]').count(),3);const otherAfter=await page.evaluate(()=>SchemaWorkspace.panels(SchemaWorkspace.active(workspaceUI.snapshot()).root).find(p=>p.view==='stack'&&p.context.layer===0).context);assert.deepEqual(otherAfter,otherBefore,'Navigating one pane must preserve other pane contexts');
 const after=await state();for(const key of ['project','undo','redo'])assert.deepEqual(after[key],before[key],'Breadcrumb navigation must not mutate '+key);assert.equal(after.snapshot.revision,before.snapshot.revision);assert.deepEqual(errors,[]);await page.screenshot({path:'target/stack-breadcrumbs-studio.png'});console.log('Stack breadcrumbs: nested driver ancestors/root, layer/fixture/Director/empty navigation, keyboard, mobile and split panes with unchanged project/history passed');
 }finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1});
