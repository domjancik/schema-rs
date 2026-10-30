import {createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
export async function verify(origin,expected){
 const url=new URL(origin);if(!['http:','https:'].includes(url.protocol)||url.username||url.password||url.search||url.hash||url.pathname!=='/')throw Error('Invalid publication origin');
 const get=async path=>{const response=await fetch(new URL(path,url),{headers:{'Cache-Control':'no-cache'},signal:AbortSignal.timeout(60000)});if(!response.ok)throw Error('Public request failed: HTTP '+response.status+' '+path);return response;};
 for(const [path,key]of [['/browser-build.json','source_commit'],['/downloads/latest.json','commit']]){const manifest=await(await get(path)).json();if(manifest[key]!==expected.commit)throw Error('Published source differs: '+path);}
 for(const entry of Object.values(expected.platforms)){
  if(!/^[A-Za-z0-9._-]+$/.test(entry.filename))throw Error('Invalid download filename');
  const response=await get('/downloads/'+entry.filename),digest=createHash('sha256');let size=0;
  for await(const block of response.body){digest.update(block);size+=block.length;}
  if(size!==entry.bytes||digest.digest('hex')!==entry.sha256)throw Error('Public download verification failed: '+entry.filename);
 }
 return {commit:expected.commit,platforms:Object.keys(expected.platforms),verified:true};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
 try{console.log(JSON.stringify(await verify(process.argv[2],JSON.parse(await readFile(process.argv[3],'utf8')))));}catch(error){console.error(error.message);process.exitCode=1;}
}
