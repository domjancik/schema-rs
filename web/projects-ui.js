"use strict";
window.SchemaProjects=(()=>{
 let options,dialog,select,name,id,details,message,save,load,revision,busy=false,entries=[],selected='';
 const el=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
 const slug=text=>text.normalize('NFKD').replace(/[\u0300-\u036f]/g,'').toLowerCase().replace(/[^a-z0-9_-]+/g,'-').replace(/^-+|-+$/g,'').slice(0,70)||'project';
 function remember(value){try{localStorage.setItem('schema-project-target',value);}catch{}}
 function previous(){try{return localStorage.getItem('schema-project-target')||'';}catch{return'';}}
 function label(text,input){const node=el('label',text);input.setAttribute('aria-label',text);node.append(input);return node;}
 function choose(value){selected=entries.some(e=>e.id===value)?value:'';select.value=selected;id.dataset.custom='';const entry=entries.find(e=>e.id===selected);name.value=entry?.title||options.project().name;id.value=entry?.preset?'carpushit':entry?.id||slug(name.value);id.readOnly=!!entry;details.textContent=entry?.preset?'Built-in reference scene. Loading replaces the current scene, disables output, and can be undone.':entry?(entry.legacy?'Existing layout/content snapshot.':`${entry.fixtures} fixtures · saved ${new Date(entry.modified_unix_ms).toLocaleString()} · revision ${entry.saved_revision}`):'Save a new named project. Your running scene is unchanged by saving.';controls();}
 function controls(){dialog.querySelectorAll('button,input,select').forEach(n=>n.disabled=busy);load.disabled=busy||!selected;save.disabled=busy||!!entries.find(e=>e.id===selected)?.preset;load.textContent=entries.find(e=>e.id===selected)?.preset?'Load preset':'Load selected project';save.textContent=selected?'Save changes':'Save new project';}
 async function refreshList(target){const result=await options.api('/api/projects');entries=[...result.projects,{id:'preset:carpushit',title:'Carpushit',preset:true}];select.replaceChildren(new Option('New saved project…',''));const saved=el('optgroup');saved.label='Saved projects';for(const entry of result.projects)saved.append(new Option(entry.title+' · '+entry.id,entry.id));const presets=el('optgroup');presets.label='Built-in presets';presets.append(new Option('Carpushit · reference scene','preset:carpushit'));select.append(saved,presets);choose(target);}
 async function run(action){if(busy)return;busy=true;controls();message.textContent='Working…';try{await action();}catch(error){message.textContent=error.message;try{await options.refresh();revision=options.revision();}catch{}}finally{busy=false;controls();}}
 function create(){dialog=el('dialog');dialog.id='projects-dialog';dialog.setAttribute('aria-labelledby','projects-title');const head=el('header'),heading=el('h2','Projects');heading.id='projects-title';const close=el('button','×');close.type='button';close.setAttribute('aria-label','Close projects');close.onclick=()=>dialog.close();head.append(heading,close);select=el('select');select.onchange=()=>{choose(select.value);message.textContent='';};name=el('input');name.maxLength=200;name.oninput=()=>{if(!selected&&!id.dataset.custom)id.value=slug(name.value);};id=el('input');id.maxLength=80;id.pattern='[A-Za-z0-9_-]+';id.oninput=()=>id.dataset.custom='true';details=el('p');details.className='project-details';
 save=el('button','Save new project');save.type='button';save.onclick=()=>run(async()=>{if(!name.value.trim()||!/^[A-Za-z0-9_-]{1,80}$/.test(id.value))throw Error('Enter a name and an ID using letters, digits, - or _.');const result=await options.api('/api/projects/save',{id:id.value,title:name.value.trim(),overwrite:!!selected,expected_revision:revision});remember(result.project.id);await refreshList(result.project.id);message.textContent=`Saved ${result.project.title} at revision ${result.saved_revision}.`;});
 load=el('button','Load selected project');load.type='button';load.onclick=()=>run(async()=>{const target=selected,entry=entries.find(e=>e.id===target);if(entry?.preset){const response=await fetch('/scene-assets/carpushit.project.json');if(!response.ok)throw Error('Unable to read the Carpushit preset.');await options.api('/api/projects/import',{json:await response.text(),expected_revision:revision});}else await options.api('/api/projects/load',{id:target,expected_revision:revision});remember(target);await options.loaded();dialog.close();options.status?.('Project loaded · Undo restores the previous scene');});
 const actions=el('div');actions.className='project-actions';actions.append(save,load);for(const[text,action]of [['Transfer manager',()=>SchemaLibrary.open()],['Content library',()=>SchemaContent.open()],['Export portable bundle',()=>SchemaAuthoring.exportBundle()],['Import portable bundle',()=>SchemaAuthoring.importBundle()]]){const b=el('button',text);b.type='button';b.onclick=()=>{dialog.close();action();};actions.append(b);}message=el('p');message.setAttribute('role','status');message.setAttribute('aria-live','polite');const hint=el('p','Loading replaces the scene in all connected views and stops Art-Net output. Undo restores the previous scene and settings.');hint.className='project-hint';dialog.append(head,label('Saved projects',select),details,label('Saved project name',name),label('Project ID',id),actions,message,hint);dialog.addEventListener('cancel',e=>{if(busy)e.preventDefault();});document.body.append(dialog);}
 return{configure(value){options=value;SchemaLibrary.configure(value);},async open(){if(!options.project())return;if(!dialog)create();revision=options.revision();id.dataset.custom='';message.textContent='';if(!dialog.open)dialog.showModal();busy=true;controls();try{await refreshList(previous());}catch(error){message.textContent=error.message;}finally{busy=false;controls();}}};
})();


// Shared by Studio and Schémaboy. Listing is requested only when browsing or
// editing a content reference, never in the animation publication loop.
window.SchemaContent = (() => {
  let options, cached = null, cacheTime = 0, pending;
  const el = (tag,text,cls) => {const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n;};
  const field = (text,input) => {const label=el('label',text);input.setAttribute('aria-label',text);label.append(input);return label;};
  async function list(force=false) {
    if(pending)return pending;
    if(!force && cached && Date.now()-cacheTime<30000)return cached;
    pending=options.api('/api/content').then(result=>{cached=result.contents;cacheTime=Date.now();return cached;}).finally(()=>pending=null);
    return pending;
  }
  async function populate(input,value) {
    input.replaceChildren(new Option(value?`Current: ${value}`:'Choose content…',value||''));
    try {
      const contents=await list();if(!input.isConnected)return;
      input.replaceChildren(new Option('Choose content…',''));
      for(const entry of contents){const option=new Option(entry.name,entry.name);option.disabled=!!entry.error;input.append(option);}
      if(value&&!contents.some(c=>c.name===value))input.append(new Option(`Missing content: ${value}`,value));
      input.value=value||'';
    } catch(error){input.title=error.message;}
  }
  async function open() {
    if(!options.data())return;
    try{await options.refresh();}catch(error){options.status?.(error.message,true);return;}
    const start=options.data(), revision=start.snapshot.revision, ids=[...options.selection()], layer=options.layer();
    const dialog=el('dialog',undefined,'authoring-dialog content-library'), head=el('header'), title=el('h2','Content library'), close=el('button','×');
    dialog.setAttribute('aria-label','Content library');close.type='button';close.setAttribute('aria-label','Close content library');close.onclick=()=>dialog.close();head.append(title,close);
    const search=el('input'), select=el('select'), group=el('select'), scope=el('p','Loading saved content…','content-scope'), load=el('button','Load content'), message=el('p'), saveName=el('input'), suggestions=el('datalist'), overwrite=el('input'), save=el('button','Save selected layer');
    search.type='search';search.placeholder='Find saved content';select.id='content-name';select.size=6;group.id='content-group';
    group.append(new Option('All saved fixture targets',''));
    for(const g of SchemaAuthoringCore.groupChoices(start.project.fixtures,[]))group.append(new Option(`${g.name} · ${g.count} fixtures`,g.name));
    saveName.maxLength=80;saveName.pattern='[A-Za-z0-9_-]+';suggestions.id='content-save-names';saveName.setAttribute('list',suggestions.id);overwrite.type='checkbox';
    message.setAttribute('role','status');message.setAttribute('aria-live','polite');load.type=save.type='button';load.disabled=save.disabled=true;
    let entries=[], selected='', busy=false;
    const targetCount=ids.filter(id=>(id==='__director'?start.project.director:start.project.content[id])?.length>layer).length;
    const controls=()=>{
      dialog.querySelectorAll('button,input,select').forEach(n=>n.disabled=busy);
      const entry=entries.find(e=>e.name===select.value);
      load.disabled=busy||!entry||!!entry.error||!matches(entry).length;
      const exists=entries.some(e=>e.name===saveName.value.trim());overwrite.closest('label').hidden=!exists;
      save.disabled=busy||!targetCount||!saveName.value.trim()||(exists&&!overwrite.checked);
    };
    function matches(entry) {
      return (entry.targets||[]).filter(t=>t.fixture==='__director'?!group.value:start.project.fixtures.some(f=>f.id===t.fixture&&(!group.value||f.groups.includes(group.value))));
    }
    function describe() {
      const entry=entries.find(e=>e.name===select.value);
      if(!entry){scope.textContent=entries.length?'Choose saved content to see its target scope.':'No saved content yet. Save a layer below.';controls();return;}
      if(entry.error){scope.textContent=entry.error;controls();return;}
      const targets=entry.targets||[], matching=matches(entry), fixtures=new Set(matching.map(t=>t.fixture)), missing=new Set(targets.filter(t=>t.fixture!=='__director'&&!start.project.fixtures.some(f=>f.id===t.fixture)).map(t=>t.fixture));
      const layers=[...new Set(matching.map(t=>t.layer+1))].sort((a,b)=>a-b);
      scope.textContent=`${targets.length} saved fixture layers · ${fixtures.size} matching fixtures · layers ${layers.join(', ')||'none'}${missing.size?` · ${missing.size} missing fixtures`:''}. Only saved fixture IDs and layers are restored; other content and layout stay in place.`;
      controls();
    }
    function draw(target=selected) {
      const visible=entries.filter(e=>e.name.toLowerCase().includes(search.value.toLowerCase()));
      select.replaceChildren(...visible.map(e=>{const option=new Option(e.name+(e.error?' · unavailable':''),e.name);option.disabled=!!e.error;return option;}));
      selected=visible.some(e=>e.name===target)?target:visible.find(e=>!e.error)?.name||'';select.value=selected;describe();
    }
    async function refresh(target) {
      entries=await list(true);suggestions.replaceChildren(...entries.map(e=>new Option(e.name,e.name)));draw(target);
    }
    async function run(action) {
      if(busy)return;busy=true;controls();message.textContent='Working…';
      try {await action();}catch(error){message.textContent=error.message;}
      finally {busy=false;controls();}
    }
    select.onchange=()=>{selected=select.value;describe();};search.oninput=()=>draw();group.onchange=describe;
    saveName.oninput=()=>{overwrite.checked=false;controls();};overwrite.onchange=controls;
    save.onclick=()=>run(async()=>{
      const name=saveName.value.trim();if(!/^[A-Za-z0-9_-]{1,80}$/.test(name))throw Error('Use 1–80 letters, digits, - or _ for the snapshot name.');
      if(options.data().snapshot.revision!==revision)throw Error('Revision conflict: close and reopen Content library before saving.');
      await options.api('/api/project/save',{name,scope:'content',fixtures:ids,layer,expected_revision:revision});
      search.value='';overwrite.checked=false;await refresh(name);message.textContent=`Saved ${name} · ${targetCount} fixtures, layer ${layer+1}.`;
    });
    load.onclick=()=>run(async()=>{
      await options.api('/api/content/load',{name:select.value,group:group.value||null,expected_revision:revision});
      await options.refresh();dialog.close();options.status?.('Content loaded · Undo restores previous layers');
    });
    const saveSection=el('details'), summary=el('summary',`Save selected layer · ${targetCount} fixtures, layer ${layer+1}`);saveSection.open=true;
    saveSection.append(summary,field('Snapshot name',saveName),suggestions,field('Overwrite existing snapshot',overwrite),save);
    const transfer=el('button','Transfer manager');transfer.type='button';transfer.onclick=()=>{dialog.close();SchemaLibrary.open();};
    dialog.append(head,field('Find content',search),field('Saved content',select),field('Restrict to group',group),scope,load,transfer,saveSection,message);
    dialog.onclose=()=>dialog.remove();dialog.addEventListener('cancel',e=>{if(busy)e.preventDefault();});document.body.append(dialog);dialog.showModal();
    busy=true;controls();try{await refresh();}catch(error){message.textContent=error.message;scope.textContent='Unable to list saved content.';}finally{busy=false;controls();}search.focus();
  }
  return {configure(value){options=value;},open,list,populate};
})();
