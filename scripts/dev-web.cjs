'use strict';
// Live frontend development against an existing engine. Never starts another evaluator.
const http=require('node:http');
const fs=require('node:fs');
const path=require('node:path');
const blocks=require('../web/block-edit.js');
const {isIP}=require('node:net');
const {networkInterfaces}=require('node:os');
const {timingSafeEqual}=require('node:crypto');
const port=Number(process.env.SCHEMA_WEB_PORT||8788);
const bind=process.env.SCHEMA_WEB_BIND||'127.0.0.1';
const accessKey=process.env.SCHEMA_WEB_TOKEN||'';
const engineKey=process.env.SCHEMA_ENGINE_TOKEN;
if(!Number.isInteger(port)||port<1||port>65535||!isIP(bind))throw Error('Use a valid SCHEMA_WEB_PORT and literal SCHEMA_WEB_BIND address');
if(accessKey&&!/^(?:[0-9]{4}|[A-Za-z0-9_-]{16,256})$/.test(accessKey))throw Error('SCHEMA_WEB_TOKEN must contain four digits or 16–256 letters, digits, underscores or hyphens');
if(!['127.0.0.1','::1'].includes(bind)&&!accessKey)throw Error('Network frontend requires SCHEMA_WEB_TOKEN');
const hostAddress=address=>address.includes(':')?'['+address+']':address;
const hosts=new Set(['127.0.0.1','localhost',bind].map(address=>hostAddress(address)+':'+port));
if(bind==='0.0.0.0'||bind==='::')for(const entry of Object.values(networkInterfaces()).flat())if(entry&&!entry.internal&&!entry.address.includes('%'))hosts.add(hostAddress(entry.address)+':'+port);
const keyBytes=Buffer.from(accessKey);
function authorized(value){if(typeof value!=='string')return false;const supplied=Buffer.from(value);return supplied.length===keyBytes.length&&timingSafeEqual(supplied,keyBytes);}
const failedAttempts=new Map();
function authenticate(req,res){
 if(!accessKey)return true;
 const value=req.headers['x-schema-token'],peer=req.socket.remoteAddress,now=Date.now();
 for(const [address,entry]of failedAttempts)if(entry.expires<=now)failedAttempts.delete(address);
 const entry=failedAttempts.get(peer);
 if(value&&(entry?.attempts>=5||!entry&&failedAttempts.size>=1024)){res.writeHead(429,{'Content-Type':'application/json','Retry-After':String(Math.max(1,Math.ceil(((entry?.expires||now+60000)-now)/1000)))});res.end(JSON.stringify({error:'Too many incorrect access codes. Try again in one minute.'}));return false;}
 if(authorized(value)){failedAttempts.delete(peer);return true;}
 if(value){const next=entry||{attempts:0,expires:now+60000};next.attempts++;failedAttempts.set(peer,next);}
 res.writeHead(401,{'Content-Type':'application/json'});res.end(JSON.stringify({error:'Access code required. Open Connection and enter the studio access code.'}));return false;
}

const backend=new URL(process.env.SCHEMA_ENGINE_URL||'http://127.0.0.1:8787');
if(backend.protocol!=='http:'||!['127.0.0.1','localhost','[::1]'].includes(backend.hostname))throw Error('Development engine must be loopback HTTP');
const assets={'/app':['index.html','text/html; charset=utf-8'],'/':['index.html','text/html; charset=utf-8'],'/app.js':['app.js','text/javascript; charset=utf-8'],'/style.css':['style.css','text/css; charset=utf-8']};
for(const [route,file,mime] of [['/mobile','mobile.html','text/html; charset=utf-8'],['/mobile.html','mobile.html','text/html; charset=utf-8'],['/mobile.css','mobile.css','text/css; charset=utf-8'],['/mobile-skin.css','mobile-skin.css','text/css; charset=utf-8'],...['mobile.js','mobile-skin-core.js','mobile-skin.js','mobile-core.js','view-variant.js','engine-client.js'].map(file=>['/'+file,file,'text/javascript; charset=utf-8'])])assets[route]=[file,mime];
for(const file of ['fixture-placement.js','fixture-placement.css','dmx-test.js','scene-stacks.js','fold-motion.js','schematic-camera.js','schematic-camera.css','workspace-core.js','workspace-ui.js','studio-panels.js','workspace.css','projects-ui.js','bezier-editor.js','bezier-editor.css','block-edit.js','block-ui.js','block-edit.css','plugins-ui.js','plugins.css','block-picker.js','block-picker.css','stack-layout.js','stack-editor.css','output-core.js','output-ui.js','output-design.css'])assets['/'+file]=[file,file.endsWith('.css')?'text/css; charset=utf-8':'text/javascript; charset=utf-8'];
assets['/icons/plugin-component.svg']=['icons/plugin-component.svg','image/svg+xml'];
for(const [file,mime] of [['app.webmanifest','application/manifest+json'],['app-icon-192.png','image/png'],['app-icon-512.png','image/png'],['offline.html','text/html'],['install-ui.js','text/javascript'],['mobile-worker.js','text/javascript']])assets['/'+file]=[file,mime];
assets['/welcome']=['welcome.html','text/html; charset=utf-8'];
for(const file of ['landing.css','landing.js'])assets['/'+file]=[file,file.endsWith('.css')?'text/css; charset=utf-8':'text/javascript; charset=utf-8'];
assets['/preview']=['preview.html','text/html'];
assets['/interface.css']=['interface.css','text/css'];
assets['/scene-preview.css']=['scene-preview.css','text/css'];
assets['/scene-camera.mjs']=['scene-camera.mjs','text/javascript'];
assets['/scene-preview-core.mjs']=['scene-preview-core.mjs','text/javascript'];
assets['/scene-preview-renderer.mjs']=['scene-preview-renderer.mjs','text/javascript'];
assets['/scene-preview-ui.mjs']=['scene-preview-ui.mjs','text/javascript'];
assets['/scene-preview-launcher.js']=['scene-preview-launcher.js','text/javascript'];
assets['/scene-preview-page.js']=['scene-preview-page.js','text/javascript'];
assets['/vendor/scene-three.mjs']=['vendor/scene-three.mjs','text/javascript'];
assets['/vendor/scene-three.mjs.LEGAL.txt']=['vendor/scene-three.mjs.LEGAL.txt','text/plain'];
assets['/vendor/THREE-LICENSE.txt']=['vendor/THREE-LICENSE.txt','text/plain'];
assets['/vendor/VERSIONS.json']=['vendor/VERSIONS.json','application/json'];
assets['/scene-assets/carpushit.glb']=['scene-assets/carpushit.glb','model/gltf-binary'];
assets['/scene-assets/carpushit.project.json']=['scene-assets/carpushit.project.json','application/json'];
assets['/scene-assets/carpushit-reference.json']=['scene-assets/carpushit-reference.json','application/json'];
assets['/scene-assets/carpushit-lights.json']=['scene-assets/carpushit-lights.json','application/json'];
assets['/scene-assets/carpushit-provenance.json']=['scene-assets/carpushit-provenance.json','application/json'];
assets['/favicon.ico']=['favicon.ico','image/x-icon'];
assets['/inputs.js']=['inputs.js','text/javascript; charset=utf-8'];
assets['/catalog-browser.js']=['catalog-browser.js','text/javascript; charset=utf-8'];
assets['/block-metadata.json']=['block-metadata.json','application/json'];
for(const row of Object.values(JSON.parse(fs.readFileSync(path.join(__dirname,'../web/block-metadata.json'),'utf8'))))assets['/'+row.icon]=[row.icon,'image/svg+xml'];
http.createServer((req,res)=>{
 const host=req.headers.host;
 if(!hosts.has(host?.toLowerCase())||req.headers.origin&&req.headers.origin!==`http://${host}`){res.writeHead(403);res.end();return;}
 res.setHeader('Cache-Control','no-store');res.setHeader('X-Content-Type-Options','nosniff');
 const downloadPath=req.url.split('?')[0];
 if(['/downloads/latest.json','/downloads/Schema-s-Windows-x64.zip','/downloads/SHA256SUMS.txt'].includes(downloadPath)){
  const filename=downloadPath.slice('/downloads/'.length),file=path.join(__dirname,'../target/distribution/current',filename);
  if(!fs.existsSync(file)){res.writeHead(filename==='latest.json'?200:404,{'Content-Type':'application/json'});res.end(JSON.stringify({available:false}));return;}
  res.setHeader('Content-Type',filename.endsWith('.zip')?'application/zip':filename.endsWith('.json')?'application/json':'text/plain');
  if(filename.endsWith('.zip'))res.setHeader('Content-Disposition','attachment; filename="Schema-s-Windows-x64.zip"');
  const stream=fs.createReadStream(file);stream.on('error',()=>res.destroy());stream.pipe(res);return;
 }
 if(req.url.startsWith('/api/')){
  if(!authenticate(req,res))return;
  const headers={...req.headers,host:backend.host};if(headers.origin)headers.origin=backend.origin;
  if(engineKey)headers['x-schema-token']=engineKey;else if(accessKey)delete headers['x-schema-token'];
  const forward=(payload)=>{
   const outgoing={...headers};
   if(payload){delete outgoing['transfer-encoding'];outgoing['content-length']=payload.length;}
   const upstream=http.request({hostname:backend.hostname,port:backend.port||80,path:req.url,method:req.method,headers:outgoing},reply=>{res.writeHead(reply.statusCode,reply.headers);reply.pipe(res);});
   upstream.on('error',()=>{if(!res.headersSent)res.writeHead(502);res.end('Engine unavailable');});req.on('aborted',()=>upstream.destroy());if(payload)upstream.end(payload);else req.pipe(upstream);
  };
  if(req.method==='POST'&&req.url==='/api/command'&&req.headers['content-type']?.includes('application/json')){
   (async()=>{
    const chunks=[];let bytes=0;for await(const chunk of req){bytes+=chunk.length;if(bytes>512*1024){res.writeHead(413);res.end('Command exceeds limit');return;}chunks.push(chunk);}
    const original=Buffer.concat(chunks);let body;try{body=JSON.parse(original);}catch{forward(original);return;}
    const c=body?.command,fields=['type','fixture','layer','node','target','branch','below'];
    const valid=c?.type==='move_node'&&Object.keys(c).every(k=>fields.includes(k))&&typeof c.fixture==='string'&&Number.isInteger(c.layer)&&Number.isSafeInteger(c.node)&&Number.isSafeInteger(c.target)&&typeof c.branch==='boolean'&&typeof c.below==='boolean';
    if(!valid){forward(original);return;}
    // Older open tabs still issue move_node. Translate it through the same atomic
    // planner used by current drag/drop, while the existing engine keeps history.
    const auth={...(headers['x-schema-token']?{'x-schema-token':headers['x-schema-token']}:{})};
    const current=await fetch(new URL('/api/state',backend),{headers:auth});
    if(!current.ok){res.writeHead(current.status,{'Content-Type':current.headers.get('content-type')||'text/plain'});res.end(await current.text());return;}
    const frame=await current.json();
    if(body.expected_revision!=null&&body.expected_revision!==frame.snapshot.revision){res.writeHead(409,{'Content-Type':'application/json'});res.end(JSON.stringify({error:'Revision conflict: refresh state before editing'}));return;}
    const plan=blocks.move(frame.project,{fixture:c.fixture,layer:c.layer,node:c.node},{fixture:c.fixture,layer:c.layer,target:c.target,placement:c.below?'below':'input'},c.branch?'branch':'block');
    forward(Buffer.from(JSON.stringify({...body,expected_revision:body.expected_revision??frame.snapshot.revision,command:plan.command})));
   })().catch(error=>{if(!res.headersSent)res.writeHead(400,{'Content-Type':'application/json'});res.end(JSON.stringify({error:error.message}));});return;
  }
  forward();return;
 }
 const asset=assets[req.url.split('?')[0]];if(req.method!=='GET'||!asset){res.writeHead(404);res.end();return;}
 const framePolicy=downloadPath==='/preview'?"'self'":"'none'";
 res.setHeader('Content-Security-Policy',`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' data: blob:; img-src 'self' blob: data:; frame-ancestors ${framePolicy}; base-uri 'none'`);
 fs.readFile(path.join(__dirname,'../web',asset[0]),(error,data)=>{if(error){res.writeHead(500);res.end('Asset unavailable');return;}res.setHeader('Content-Type',asset[1]);res.end(data);});
}).listen(port,bind,()=>console.log(`Live web UI: http://${hostAddress(bind)}:${port} → engine ${backend.origin}${accessKey?' (access key required)':''}`));
