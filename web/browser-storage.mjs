// Origin-local persistence adapter. The Rust core only encodes/decodes bytes.
export function openStorage() {
  return new Promise((resolve,reject)=>{
    const request=indexedDB.open('schema-browser',1);
    request.onupgradeneeded=()=>{for(const name of ['sessions','projects','resources'])request.result.createObjectStore(name,{keyPath:'id'});};
    request.onsuccess=()=>resolve(new BrowserStorage(request.result));
    request.onerror=()=>reject(request.error);
    request.onblocked=()=>reject(Error('Close older browser editors to upgrade project storage.'));
  });
}
class BrowserStorage {
  constructor(db){this.db=db;}
  access(store,mode,action){return new Promise((resolve,reject)=>{const tx=this.db.transaction(store,mode);let result;const request=action(tx.objectStore(store));request.onsuccess=()=>result=request.result;request.onerror=()=>reject(request.error);tx.oncomplete=()=>resolve(result);tx.onabort=()=>reject(tx.error||Error('Storage transaction aborted'));tx.onerror=()=>reject(tx.error);});}
  get(store,id){return this.access(store,'readonly',s=>s.get(id));}
  all(store){return this.access(store,'readonly',s=>s.getAll());}
  put(store,value){return this.access(store,'readwrite',s=>s.put(value));}
  saveProject(record,overwrite){return new Promise((resolve,reject)=>{
    const tx=this.db.transaction('projects','readwrite'),store=tx.objectStore('projects'),request=store.get(record.id);let failure;
    request.onsuccess=()=>{if(request.result&&!overwrite){failure=Error('Project exists; select it to overwrite.');tx.abort();}else store.put(record);};
    tx.oncomplete=()=>resolve(record);tx.onabort=()=>reject(failure||tx.error);tx.onerror=()=>reject(tx.error);
  });}
}
