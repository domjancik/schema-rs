'use strict';
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.SCHEMA_URL;
if(!base)throw Error('Set SCHEMA_URL to an isolated capture-only engine');
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_EXECUTABLE});try{
 const page=await browser.newPage({baseURL:base,viewport:{width:1500,height:1000}});const errors=[];page.on('pageerror',error=>errors.push(error.message));await page.goto(base);
 const state=()=>page.request.get('/api/state').then(r=>r.json());const before=await state();assert(!before.project.output.enabled);
 await page.getByRole('button',{name:'Import YAML',exact:true}).click();await page.getByLabel('ScenicScript YAML').fill('name: Keyboard input\nstack:\n  Create.Value.Keyboard: {Key Name: Space}\n');await page.getByRole('button',{name:'Validate & apply to this layer',exact:true}).click();await page.locator('#yaml-dialog').waitFor({state:'hidden'});
 await page.locator('#canvas').focus();await page.keyboard.down('Space');await page.waitForFunction(async()=>{const s=await fetch('/api/state').then(r=>r.json());return s.snapshot.fixtures[0].result.value===1});
 await page.keyboard.up('Space');await page.waitForFunction(async()=>{const s=await fetch('/api/state').then(r=>r.json());return s.snapshot.fixtures[0].result.value===0});
 assert(await page.getByRole('button',{name:'Microphone',exact:true}).isVisible());
 await page.getByRole('button',{name:'Director',exact:true}).click();await page.getByRole('button',{name:'+ Create stack',exact:true}).click();await page.getByRole('searchbox',{name:'Search blocks'}).fill('Create.Value.Expression');await page.locator('.picker-results [data-key="Create.Value.Expression"] .picker-add').click();await page.locator('#block-picker-dialog').waitFor({state:'hidden'});
 await page.getByLabel('Expression',{exact:true}).fill('s * 2.0');await page.getByLabel('Expression',{exact:true}).press('Tab');await page.waitForFunction(async()=>{const s=await fetch('/api/state').then(r=>r.json());return s.project.director[0]?.root.properties.Expression==='s * 2.0'&&s.snapshot.director[0].value>0});
 const response=await page.request.get('/api/export/__director/0');assert.equal(response.status(),200);assert.match(await response.text(),/Create.Value.Expression/);
 await page.screenshot({path:'docs/catalog-director.png'});
 for(let i=0;i<3;i++){const current=await state();const result=await page.request.post('/api/command',{data:{expected_revision:current.snapshot.revision,command:{type:'undo'}}});assert.equal(result.status(),200,await result.text());}
 const after=await state();assert.deepEqual(after.project,before.project);assert.deepEqual(errors,[]);
 console.log('Catalog integration: keyboard press/release, hardware control gating, director creation/edit/export, and exact undo restoration passed');
 }finally{await browser.close();}})().catch(error=>{console.error(error);process.exit(1)});
