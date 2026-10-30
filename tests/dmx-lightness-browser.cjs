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
 const page=await browser.newPage({viewport:{width:1440,height:950}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(7000);await page.goto(base+(process.env.SCHEMA_URL?'/?view=studio':'/play?view=studio'));await page.locator('#fixtures button').first().waitFor();
 await page.getByRole('button',{name:'Output design',exact:true}).click();await page.getByRole('button',{name:'Models',exact:true}).click();const panel=page.locator('#output-design-dialog');const source=panel.getByLabel('Slot source',{exact:true}).first();await source.selectOption('lightness');
 await panel.getByLabel('Model ID',{exact:true}).fill('lightness-test');await panel.getByLabel('Model name',{exact:true}).fill('Lightness dimmer');await panel.getByRole('button',{name:'Save DMX model',exact:true}).click();
 await page.waitForFunction(()=>SchemaEngine.request('/api/state').then(s=>s.project.dmx_models['lightness-test']?.slots[0].source==='lightness'));
 await panel.getByLabel('Existing model',{exact:true}).selectOption('lightness-test');assert.equal(await panel.getByLabel('Slot source',{exact:true}).first().inputValue(),'lightness');
 const result=await page.evaluate(async()=>{const s=await SchemaEngine.request('/api/state'),f=s.project.fixtures[0];assertOutputDisabled(s);function assertOutputDisabled(s){if(s.project.output.enabled)throw Error('Use a capture-only test workspace');}const stack={name:'Red with independent value',root:{id:9998,behavior:'Create.Color.Color',properties:{Color:'#ff0000'},children:[],drivers:{}}};await SchemaEngine.request('/api/command',{expected_revision:s.snapshot.revision,command:{type:'batch',commands:[{type:'set_fixture',fixture:{...f,dmx_model:'lightness-test',dimmer:1,pixel_positions:[[0,0,0]]}},{type:'replace_layer',fixture:f.id,layer:0,stack}]}});return f;});
 await page.waitForFunction(({id,address,universe})=>SchemaEngine.request('/api/state').then(s=>s.snapshot.universes[universe]?.[address-1]===128),result);
 await page.setViewportSize({width:390,height:844});assert.equal(await source.inputValue(),'lightness');await page.screenshot({path:'target/dmx-lightness-source.png'});assert.deepEqual(errors,[]);console.log('Lightness DMX source: selectable, saved/reopened, encoded by real engine, responsive model editor passed');
 }finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1});
