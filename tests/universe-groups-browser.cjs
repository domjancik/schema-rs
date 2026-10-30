const assert = require('node:assert/strict');
const fs = require('node:fs');
const {chromium} = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_EXECUTABLE});
 try {
 const page=await browser.newPage({viewport:{width:1200,height:900}}),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 await page.setContent('<!doctype html><style>:root{--paper:#15251f;--ink:#e3efe7}body{margin:0}</style>');
 await page.addStyleTag({path:'web/output-design.css'});
 await page.addScriptTag({path:'web/output-core.js'});
 await page.addScriptTag({path:'web/output-ui.js'});
 await page.evaluate(()=>{
  const fixture=(id,name,address,model='dimmer',pixels=1)=>({id,name,address,model,universe:0,pixel_positions:Array(pixels).fill([0,0,0])});
  window.p={dimming:{},dmx_models:{uv:{name:'UV',slots:[{name:'Dimmer'},{name:'Program'}]}},fixtures:[{...fixture('uv','UV COB',1),dmx_model:'uv'},fixture('rgb','RGB bar',3,'rgb'),fixture('wrap','Long bar',31,'rgb',2),fixture('last','Last channel',512)]};
  window.commands=[];
  SchemaOutput.configure({project:()=>p,revision:()=>0,command:async c=>{commands.push(c);for(const cmd of c.commands||[])if(cmd.type==='set_fixture')p.fixtures=p.fixtures.map(f=>f.id===cmd.fixture.id?cmd.fixture:f);return true;}});
  SchemaOutput.open('patch');
 });
 const group=id=>page.locator('.universe-fixture-group[data-fixture="'+id+'"]');
 assert.equal(await group('uv').count(),1,'a two-channel fixture must have one visible group');
 assert.equal(await group('uv').locator('.universe-slot').count(),2);
 assert.match(await group('uv').innerText(),/UV COB/);
 assert.equal(await group('wrap').count(),2,'row crossing must preserve fixture identity on both fragments');
 assert.equal(await page.locator('.universe-slot').count(),512);
 assert.match(await page.locator('[data-address="2"]').getAttribute('title'),/Program/);
 const color=await group('uv').first().evaluate(n=>n.style.getPropertyValue('--fixture-hue'));
 assert.equal(await page.locator('.patch-fixture[data-fixture="uv"]').evaluate(n=>n.style.getPropertyValue('--fixture-hue')),color);
 assert.notEqual(await group('rgb').evaluate(n=>n.style.getPropertyValue('--fixture-hue')),color);
 await page.screenshot({path:'target/universe-groups-desktop.png'});
 await page.setViewportSize({width:390,height:844});
 await page.waitForFunction(()=>document.querySelector('.universe-grid').style.getPropertyValue('--patch-columns')==='16');
 assert.equal(await group('wrap').count(),2);
 assert.match(await group('wrap').nth(1).getAttribute('aria-label'),/continued/i);
 assert(await page.locator('.universe-grid').evaluate(n=>n.scrollWidth<=n.clientWidth+1));
 assert(await page.locator('.output-body').evaluate(n=>n.scrollWidth<=n.clientWidth+1),'mobile patch must not overflow horizontally');
 await page.screenshot({path:'target/universe-groups-mobile.png'});
 await page.locator('[data-address="100"]').dispatchEvent('drop',{dataTransfer:await page.evaluateHandle(()=>{const d=new DataTransfer();d.setData('application/schema-fixture','uv');return d;})});
 await page.waitForFunction(()=>commands.length===1);
 assert.equal(await page.locator('.universe-slot[data-address="101"]').getAttribute('data-fixture'),'uv');
 assert.equal(await group('uv').locator('.universe-slot').count(),2);
 await page.getByRole('button',{name:'Close output'}).click();
 await page.setViewportSize({width:1200,height:900});
 await page.evaluate(()=>SchemaOutput.open('patch'));
 assert.equal(await page.locator('.universe-slot').count(),512);
 assert.deepEqual(errors,[]);
 console.log('Universe fixture grouping: adjacent/custom/single/row-wrap/mobile/drop/close-reopen passed.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
