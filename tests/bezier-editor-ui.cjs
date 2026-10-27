"use strict";
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),{chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.SCHEMA_URL;if(!base)throw Error('SCHEMA_URL required');
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_EXECUTABLE});try{
 for(const mobile of [false,true]){
  const page=await browser.newPage({viewport:mobile?{width:390,height:900}:{width:1500,height:1000},hasTouch:mobile});page.setDefaultTimeout(3500);
  const errors=[],edits=[];page.on('pageerror',e=>errors.push(e.message));
  const frame=await page.request.get(base+'/api/state').then(r=>r.json());const fixture=frame.project.fixtures[0];
  const node={id:1,behavior:'Create.Value.Bezier',properties:{Start:0,Control1:0,Control2:1,End:1,Duration:4,Progress:-1,Loop:true,PingPong:false},children:[],drivers:{}};
  frame.project.content[fixture.id][0]={name:'Visual curve',root:node};
  await page.route('**/*',async r=>{const u=new URL(r.request().url()),file=u.pathname==='/mobile'?'mobile.html':u.pathname==='/'?'index.html':u.pathname.slice(1);
   if(u.pathname==='/api/state')return r.fulfill({json:frame});
   if(u.pathname==='/api/command'){const c=r.request().postDataJSON();edits.push(c);assert.equal(c.command.type,'set_property');node.properties[c.command.key]=c.command.value;frame.snapshot.revision++;return r.fulfill({json:{revision:frame.snapshot.revision}});}
   if(!file.includes('..')&&fs.existsSync(path.join('web',file))&&fs.statSync(path.join('web',file)).isFile())return r.fulfill({path:path.resolve('web',file),contentType:file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.html')?'text/html':undefined});
   return r.continue();
  });
  await page.goto(base+(mobile?'/mobile':'/?view=studio'));if(mobile){await page.getByRole('button',{name:'Open selected block inspector'}).click();}
  const graph=page.locator('.value-bezier-editor'),handle=graph.locator('[data-key="Control1"]');await handle.waitFor();
  const initial=await handle.boundingBox(),x=initial.x+initial.width/2,y=initial.y+initial.height/2;
  if(mobile){const cdp=await page.context().newCDPSession(page);await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x,y,id:1}]});await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x,y:y-35,id:1}]});assert.equal(edits.length,0);await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});}
  else {await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x,y-35,{steps:5});assert.equal(edits.length,0);await page.mouse.up();}
  await page.waitForFunction(()=>document.querySelector('.value-bezier-editor')?.dataset.busy==='false');assert.equal(edits.length,1);assert.equal(edits[0].command.key,'Control1');assert(node.properties.Control1>0);assert.equal(node.properties.Start,0);assert.equal(node.properties.End,1);
  await handle.focus();await page.keyboard.press('ArrowUp');await page.waitForFunction(()=>document.querySelector('.value-bezier-editor')?.dataset.busy==='false');assert.equal(edits.length,2);
  const box=await handle.boundingBox();await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2,box.y-20);await page.keyboard.press('Escape');await page.mouse.up();assert.equal(edits.length,2,'Escape cancels without committing');
  if(mobile){await page.getByRole('button',{name:'Close menu',exact:true}).click();await page.getByRole('button',{name:'Open selected block inspector'}).click();await page.locator('#menu').getByRole('button',{name:'Control1',exact:true}).click();await graph.waitFor();await handle.focus();await page.keyboard.press('ArrowUp');await page.waitForFunction(()=>document.querySelector('.value-bezier-editor')?.dataset.busy==='false');assert.equal(edits.length,3);assert.equal(Number(await page.locator('.mobile-property-input').inputValue()),node.properties.Control1);await page.getByRole('button',{name:'Apply property',exact:true}).click();assert.equal(edits.length,4);assert.equal(edits[3].expected_revision,edits[2].expected_revision+1);await page.getByRole('button',{name:'Open selected block inspector'}).click();}
  await page.screenshot({path:'target/bezier-value-'+(mobile?'mobile':'desktop')+'.png'});assert.deepEqual(errors,[]);await page.close();
 }
 console.log('Visual scalar Bezier: desktop mouse, mobile touch, one commit per gesture, keyboard, cancellation and mobile property entry passed.');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
