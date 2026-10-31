"use strict";
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path"),
  http = require("node:http"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const root = path.resolve("target/browser-site"),
  types = {
    ".html": "text/html",
    ".js": "text/javascript",
    ".mjs": "text/javascript",
    ".wasm": "application/wasm",
    ".json": "application/json",
    ".css": "text/css",
    ".svg": "image/svg+xml",
    ".woff2": "font/woff2",
    ".png": "image/png",
  };
const server = http.createServer((req, res) => {
  const url = new URL(req.url, "http://localhost");
  if (url.pathname === "/test") {
    res.setHeader("Content-Type", "text/html");
    res.end("<!doctype html><title>Migration test</title>");
    return;
  }
  const file = path.resolve(root, "." + url.pathname);
  if (!file.startsWith(root + path.sep) || !fs.existsSync(file)) {
    res.writeHead(404);
    res.end();
    return;
  }
  const config = JSON.parse(
    fs.readFileSync("scripts/publication/vercel.json", "utf8"),
  );
  for (const header of (
    config.headers.find((h) => h.source === url.pathname) || config.headers[0]
  ).headers)
    res.setHeader(header.key, header.value);
  res.setHeader(
    "Content-Type",
    types[path.extname(file)] || "application/octet-stream",
  );
  fs.createReadStream(file).pipe(res);
});

(async()=>{
 await new Promise(r=>server.listen(0,'127.0.0.1',r));const browser=await chromium.launch({channel:'chrome',headless:true});try{
 const page=await browser.newPage();page.setDefaultTimeout(15000);await page.goto('http://127.0.0.1:'+server.address().port+'/test');
 const proof=await page.evaluate(async()=>{
  const{SchemaPlayer}=await import('/schema-player.mjs');const sessionId='library-'+crypto.randomUUID();const player=await SchemaPlayer.create({persist:true,sessionId,autoTick:false});const call=(p,b)=>player.request(p,b);const check=(v,msg)=>{if(!v)throw Error(msg);};
  const before=await call('/api/session');const item=await call('/api/library/export',{key:'current'});check(typeof item.bundle==='string','Bundle payload must remain lossless text');item.id='browser-copy';item.title='Browser copy';let result=await call('/api/library/import',{item});
  const after=await call('/api/session');check(JSON.stringify(after.project)===JSON.stringify(before.project)&&JSON.stringify(after.undo)===JSON.stringify(before.undo)&&after.snapshot.revision===before.snapshot.revision,'Storing changed the running scene/history');
  let rejected=false;try{await call('/api/library/import',{item});}catch{rejected=true;}check(rejected,'Existing copy replaced without hash');await call('/api/library/import',{item,expected_hash:result.entry.hash});
  const future={...item,id:'future-copy',version:99};rejected=false;try{await call('/api/library/import',{item:future});}catch{rejected=true;}check(rejected,'Unsupported future version accepted');
  await call('/api/project/save',{name:'source-look',scope:'content',fixtures:[before.project.fixtures[0].id],layer:0,expected_revision:before.snapshot.revision});const content=await call('/api/library/export',{key:'content:source-look'});check(typeof content.content==='string','Content payload must remain lossless');content.id='received-look';const doc=JSON.parse(content.content);doc.entries[0].stack={name:'Transferred look',root:{id:1,behavior:'Create.Value.Constant',properties:{Value:.375},children:[],drivers:{}}};content.content=JSON.stringify(doc);await call('/api/library/import',{item:content});check(!(await call('/api/content')).contents.some(c=>c.name==='received-look'),'A staged copy must not enter the active Director registry');await call('/api/library/publish',{key:'portable:content:received-look',expected_revision:(await call('/api/session')).snapshot.revision});check((await call('/api/content')).contents.some(c=>c.name==='received-look'),'Explicitly published preset absent from normal content selectors');
  const start=await call('/api/session');await call('/api/library/load',{key:'portable:content:received-look',expected_revision:start.snapshot.revision});const end=await call('/api/session');check(JSON.stringify(start.project.fixtures)===JSON.stringify(end.project.fixtures),'Content changed layout');check(JSON.stringify(start.project.output)===JSON.stringify(end.project.output),'Content changed output');check(end.undo.length===start.undo.length+1,'Content load not one undo');check(end.project.content[before.project.fixtures[0].id][0].root.properties.Value===.375,'Transferred look not applied');
  rejected=false;try{await call('/api/library/peer',{peer:{origin:'http://lighting-server.local:8787'},operation:'list'});}catch(e){rejected=e.message.includes('hosted');}check(rejected,'Browser unexpectedly proxied a LAN request');
  await call('/api/session/save',{});player.dispose();const resumed=await SchemaPlayer.create({persist:true,sessionId,autoTick:false});const list=await resumed.request('/api/library');check(list.entries.some(e=>e.key==='portable:project:browser-copy')&&list.entries.some(e=>e.key==='portable:content:received-look'),'Stored portable entries lost after restart');resumed.dispose();
  const{openStorage}=await import('/browser-storage.mjs');const storage=await openStorage({project:2,session:2,content:2,layout:2});const backups=await storage.all('migration-backups');check(backups.some(b=>b.store==='library'),'Replaced library copy was not backed up');return{entries:list.entries.length,backups:backups.length};
 });assert(proof.entries>=3);await page.setViewportSize({width:390,height:844});await page.goto('http://127.0.0.1:'+server.address().port+'/studio.html?view=studio');await page.locator('#fixtures button').first().waitFor({state:'attached'});await page.getByRole('button',{name:'Save project',exact:true}).click();await page.getByRole('button',{name:'Transfer manager',exact:true}).click();const dialog=page.getByRole('dialog',{name:'Project and content manager',exact:true});await dialog.getByRole('status').filter({hasText:'Select an item'}).waitFor();assert(await dialog.getByRole('button',{name:'Download file',exact:true}).isEnabled());assert.equal(await dialog.getByRole('button',{name:'Connect',exact:true}).isVisible(),false);assert(await dialog.evaluate(n=>n.scrollWidth<=n.clientWidth+1));await dialog.getByRole('button',{name:'Close manager',exact:true}).click();console.log('Browser portable library: lossless files, non-destructive store, conflict/version rejection, sparse content/undo, preset registration, persistent copies and overwrite backups passed',proof);
 }finally{await browser.close();await new Promise(r=>server.close(r));}
})().catch(e=>{console.error(e);process.exitCode=1;server.close();});
