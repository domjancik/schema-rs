const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8787';
(async () => {
 const browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_EXECUTABLE ? {executablePath:process.env.CHROMIUM_EXECUTABLE} : {}) });
 try {
  const page = await browser.newPage({viewport:{width:1500,height:1000}});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(base);await page.waitForSelector('#fixtures button');
  assert.equal(await page.locator('#fixtures button').count(),8);
  const state=async()=>fetch(base+'/api/state').then(r=>r.json());
  const wait=async(predicate)=>{for(let i=0;i<60;i++){if(predicate(await state()))return;await new Promise(r=>setTimeout(r,50));}throw Error('State assertion timed out');};
  const original=(await state()).project.fixtures[0].position[0];
  await page.getByLabel('X',{exact:true}).fill('2.5');await page.getByLabel('X',{exact:true}).press('Tab');
  await wait(s=>s.project.fixtures[0].position[0]===2.5);
  await page.getByRole('button',{name:'↶ Undo',exact:true}).click();await wait(s=>s.project.fixtures[0].position[0]===original);
  await page.getByRole('button',{name:'↷ Redo',exact:true}).click();await wait(s=>s.project.fixtures[0].position[0]===2.5);
  const before=await state();await page.evaluate(()=>{const end=Date.now()+350;while(Date.now()<end){};});const after=await state();assert(after.snapshot.frame>before.snapshot.frame+5);
  await page.getByRole('button',{name:'Import YAML',exact:true}).click();
  await page.getByLabel('ScenicScript YAML').fill('name: Spatial test\nstack:\n  Convert.Value.ToColor:\n    in:\n      - Spatial.Axis: {Axis: 0, Minimum: -5, Maximum: 5}\n');
  await page.getByRole('button',{name:'Validate & apply to this layer'}).click();await wait(s=>s.project.content['fixture-1'][0].name==='Spatial test');
  await page.getByRole('button',{name:'Layer 2',exact:true}).click();await page.getByRole('button',{name:'+ Create stack',exact:true}).click();
  await page.locator('#catalog [data-key="Create.Color.Color"]').click();await page.getByRole('button',{name:'Add block',exact:true}).click();await wait(s=>s.project.content['fixture-1'][1]?.root.behavior==='Create.Color.Color');
  await page.getByRole('button',{name:'Layer 1',exact:true}).click();await page.getByRole('button',{name:'Save project ↗',exact:true}).click();
  await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Saved layout'));
  assert.equal(errors.length,0,errors.join('\n'));
  await page.screenshot({path:process.env.SCHEMA_SCREENSHOT||'docs/studio.png',fullPage:true});
  const report={web_errors:errors,fixture_count:8,scene_move:true,undo_redo:true,independent_engine_during_blocked_ui:true,yaml_import:true,independent_layer_edit:true,project_save:true};
  require('node:fs').writeFileSync('docs/browser-verification.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1)});
