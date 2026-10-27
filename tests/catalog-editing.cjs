const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.SCHEMA_URL;
if(!base)throw Error('Set SCHEMA_URL to an isolated capture-only test engine; this test edits it.');
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_EXECUTABLE});try{
 const page=await browser.newPage({baseURL:base});await page.goto(base);await page.getByRole('button',{name:'Add input',exact:true}).click();
 const before=await page.request.get('/api/state').then(r=>r.json());assert.equal(before.project.output.enabled,false);
 const fixture=before.project.fixtures[0].id,root=before.project.content[fixture][0].root;
 await page.getByRole('searchbox',{name:'Search blocks'}).fill('clmap');await page.getByRole('searchbox',{name:'Search blocks'}).press('Enter');await page.locator('#block-picker-dialog').waitFor({state:'hidden'});
 let after=await page.request.get('/api/state').then(r=>r.json());assert.equal(after.project.content[fixture][0].root.children.length,root.children.length+1);assert.equal(after.project.content[fixture][0].root.children.at(-1).behavior,'Modify.Value.Clamp');
 await page.getByRole('button',{name:'Add input',exact:true}).click();await page.getByRole('button',{name:'Recent',exact:true}).click();assert.equal(await page.locator('.picker-results .picker-row').count(),1);assert.equal(await page.locator('.picker-results .picker-row').getAttribute('data-key'),'Modify.Value.Clamp');await page.getByRole('button',{name:'Close block picker'}).click();
 await page.locator('#undo').click();await page.waitForFunction(revision=>document.querySelector('#frame').textContent.includes(`REV ${revision}`),after.snapshot.revision+1);after=await page.request.get('/api/state').then(r=>r.json());assert.deepEqual(after.project.content[fixture][0].root,root);console.log('Catalog editing: keyboard insert, recent history and engine undo passed');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exit(1)});
