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
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 const base='http://127.0.0.1:'+server.address().port;
 const browser=await chromium.launch({channel:'chrome',headless:true});
 try{
 const page=await browser.newPage({viewport:{width:390,height:844},hasTouch:true}),errors=[];
 page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(5000);
 await page.goto(base+'/play?view=studio');await page.locator('.workspace-panel:visible').first().waitFor();
 const before=await page.evaluate(()=>SchemaEngine.request('/api/state'));
 const nav=page.getByRole('navigation',{name:'Studio panels',exact:true});
 assert.equal(await nav.count(),1,'mobile panel selection must have a bottom icon row');
 assert(await nav.isVisible());
 assert.equal(await nav.locator('button').count(),4);
 assert.equal(await page.locator('[data-panel]').count(),await page.locator('.workspace-panel').count(),'panel identity remains unique to actual panes');
 assert.equal(await nav.locator('svg').count(),4);
 assert.equal(await page.getByLabel('Studio panel',{exact:true}).count(),0,'top dropdown is replaced');
 await nav.getByRole('button',{name:'Output',exact:true}).tap();
 await page.locator('.workspace-panel[data-view="output"]:visible').waitFor();
 assert.equal(await nav.getByRole('button',{name:'Output',exact:true}).getAttribute('aria-pressed'),'true');
 assert.equal(await page.locator('.workspace-panel:visible').count(),1);
 for(const [width,height]of [[320,568],[390,844],[768,1024],[844,390]]){
  await page.setViewportSize({width,height});
  const row=await nav.boundingBox(),pane=await page.locator('.workspace-panel:visible').boundingBox(),footer=await page.locator('body>footer').boundingBox();
  assert(row.y+row.height<=height+1&&row.y>=height-90,'row stays at viewport bottom');
  assert(pane.y+pane.height<=row.y+1&&footer.y+footer.height<=row.y+1,'row reserves space below editor and status');
  assert(pane.width>=width-20&&pane.height>=height*.45,'editor keeps useful area');
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'no horizontal page overflow');
  for(const b of await nav.locator('button').all()){const r=await b.boundingBox();assert(r.width>=44&&r.height>=44);}
 }
 await page.setViewportSize({width:390,height:844});await page.screenshot({path:'target/mobile-panel-nav.png'});
 await page.reload();await page.locator('.workspace-panel[data-view="output"]:visible').waitFor();
 const customWorkspace=await page.evaluate(()=>{const W=SchemaWorkspace,s=W.create(),panels=[W.panel('stack'),W.panel('stack'),...W.views.filter(v=>v!=='stack'&&v!=='gpu').map(v=>W.panel(v))];s.authoringLayoutVersion=2;s.workspaces[0].root=panels.reduce((a,b,i)=>a?{kind:'split',id:'test-split-'+i,axis:'x',ratio:.5,a,b}:b,null);return JSON.stringify(W.validate(s));});
 await page.addInitScript(value=>localStorage.setItem('schema-workspaces-v1',value),customWorkspace);
 await page.reload();await nav.getByRole('button',{name:'Stack editor 2',exact:true}).waitFor();
 assert(await nav.evaluate(n=>n.scrollWidth>n.clientWidth),'many panels scroll within the row');
 await nav.getByRole('button',{name:'System logs',exact:true}).tap();
 await page.locator('.workspace-panel[data-view="logs"]:visible .system-log-row').first().waitFor();
 await nav.getByRole('button',{name:'Stack editor 2',exact:true}).tap();
 assert.equal(await nav.getByRole('button',{name:'Stack editor 2',exact:true}).getAttribute('aria-pressed'),'true');
 await page.keyboard.press('End');assert.equal(await nav.locator('button[aria-pressed="true"]').count(),1);
 await page.keyboard.press('Home');assert.equal(await nav.getByRole('button',{name:'Stack editor 1',exact:true}).getAttribute('aria-pressed'),'true');
 await page.setViewportSize({width:1500,height:950});assert(!await nav.isVisible(),'desktop retains split panels');
 assert((await page.locator('.workspace-panel:visible').count())>1);
 const after=await page.evaluate(()=>SchemaEngine.request('/api/state'));assert.deepEqual(after.project,before.project);assert.equal(after.snapshot.revision,before.snapshot.revision);
 assert.deepEqual(errors,[]);
 console.log('Mobile panel icons: touch switching, viewport/safe space, overflow, duplicate instances, persistence, keyboard and unchanged scene passed.');
 }finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(e=>{console.error(e);server.close();process.exitCode=1;});
