'use strict';
// Only a disconnected navigation page is cached. Engine data and commands stay online.
const CACHE='schema-mobile-offline-v1';
self.addEventListener('install',event=>event.waitUntil(caches.open(CACHE).then(cache=>cache.add('/offline.html'))));
self.addEventListener('activate',event=>event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(key=>key.startsWith('schema-mobile-offline-')&&key!==CACHE).map(key=>caches.delete(key))))));
self.addEventListener('fetch',event=>{
 const request=event.request,url=new URL(request.url);
 if(request.method!=='GET'||request.mode!=='navigate'||url.origin!==self.location.origin||!['/mobile','/mobile.html'].includes(url.pathname))return;
 event.respondWith(fetch(request).catch(()=>caches.match('/offline.html')));
});
