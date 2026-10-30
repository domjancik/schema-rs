"use strict";
window.SchemaLogs = (() => {
  const views = new Set();
  let entries = [], cursor = 0, generation = null, timer = null, pending = false, error = '', lost = false;
  const el = (tag, text, cls) => {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    if (cls) node.className = cls;
    return node;
  };
  const uptime = ms => {
    const seconds = Math.floor(ms / 1000);
    return `${Math.floor(seconds / 3600)}:${String(Math.floor(seconds / 60) % 60).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
  };
  async function poll() {
    if (pending || document.hidden || ![...views].some(view => view.visible())) return;
    pending = true;
    try {
      let page = await SchemaEngine.request('/api/logs', {after: cursor, limit: 200});
      if (generation !== null && generation !== page.generation) {
        cursor = 0; entries = []; lost = false;
        page = await SchemaEngine.request('/api/logs', {after: 0, limit: 200});
      }
      generation = page.generation;
      lost ||= !!page.truncated;
      if (page.reset) entries = [];
      const changed = page.entries.length || error;
      entries = [...entries, ...page.entries].slice(-512);
      cursor = page.next;
      error = '';
      if (changed) for (const view of views) view.update();
    } catch (failure) {
      error = `Logs unavailable: ${failure.message}`;
      for (const view of views) view.update();
    } finally { pending = false; }
  }
  function mount(host) {
    const root = el('section', undefined, 'system-logs'), toolbar = el('div', undefined, 'system-logs-tools');
    const search = el('input'), severity = el('select'), status = el('p', 'Loading logs…', 'system-logs-status'), list = el('div', undefined, 'system-log-list');
    search.type = 'search'; search.placeholder = 'Search logs'; search.setAttribute('aria-label', 'Search logs');
    severity.setAttribute('aria-label', 'Log severity');
    for (const [value, label] of [['all','All levels'],['warning','Warnings'],['error','Errors'],['info','Info']]) severity.append(new Option(label,value));
    let paused = false, frozen = [], cleared = 0, shown = [], viewGeneration = generation;
    const button = (text, action) => { const node = el('button', text); node.type = 'button'; node.onclick = action; return node; };
    const pause = button('Pause logs', () => { paused = !paused; frozen = entries.slice(); pause.textContent = paused ? 'Resume logs' : 'Pause logs'; pause.setAttribute('aria-pressed',String(paused)); update(); if (!paused) poll(); });
    pause.setAttribute('aria-pressed','false');
    const follow = el('input'); follow.type = 'checkbox'; follow.checked = true;
    const followLabel = el('label','Follow'); followLabel.prepend(follow);
    const formatted = () => shown.map(entry => `${uptime(entry.elapsed_ms)} ${entry.level.toUpperCase()} [${entry.source}] ${entry.message}`).join('\n');
    const copy = button('Copy', async () => {
      try { await navigator.clipboard.writeText(formatted()); status.textContent = 'Visible logs copied'; }
      catch { status.textContent = 'Clipboard unavailable here; use Download or select the log text.'; }
    });
    const download = button('Download', () => {
      const url = URL.createObjectURL(new Blob([formatted() + '\n'], {type:'text/plain;charset=utf-8'})), link = el('a');
      link.href = url; link.download = 'schema-logs.txt'; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
    });
    const clear = button('Clear view', () => { cleared = cursor; update(); });
    toolbar.append(search, severity, pause, followLabel, copy, download, clear);
    status.setAttribute('role','status');
    list.setAttribute('aria-label','System log entries'); list.tabIndex = 0;
    root.append(toolbar, status, list, el('p','Engine runtime events · times since engine start · last 512 events · resets on restart','system-logs-note'));
    host.append(root);
    search.oninput = severity.onchange = () => update();
    function update() {
      if (!root.isConnected) return;
      if (viewGeneration !== generation && !paused) { cleared = 0; viewGeneration = generation; }
      const query = search.value.trim().toLowerCase();
      shown = (paused ? frozen : entries).filter(entry => entry.id > cleared && (severity.value === 'all' || entry.level === severity.value) && `${entry.source} ${entry.message}`.toLowerCase().includes(query));
      const scroll = list.scrollTop;
      list.replaceChildren();
      for (const entry of shown) {
        const row = el('div',undefined,`system-log-row log-${entry.level}`);
        row.append(el('time',uptime(entry.elapsed_ms)),el('strong',entry.level.toUpperCase()),el('span',entry.source,'system-log-source'),el('span',entry.message,'system-log-message'));
        list.append(row);
      }
      status.textContent = error || `${paused?'Paused · ':''}${shown.length} events${lost?' · Older events expired from the buffer':''}`;
      if (!shown.length) list.append(el('p','No matching events.'));
      if (follow.checked && !paused) list.scrollTop = list.scrollHeight; else list.scrollTop = scroll;
    }
    const view = {update,visible:()=>!paused && root.isConnected && root.getClientRects().length > 0 && getComputedStyle(root).visibility !== 'hidden'};
    views.add(view);
    if (entries.length) update();
    if (!timer) timer = setInterval(poll,1000);
    poll();
    return {destroy(){ views.delete(view); root.remove(); if (!views.size) {clearInterval(timer);timer=null;} }};
  }
  function open() {
    let dialog = document.getElementById('system-logs-dialog');
    if (dialog) { if (!dialog.open) dialog.showModal(); return; }
    dialog = el('dialog'); dialog.id = 'system-logs-dialog'; dialog.setAttribute('aria-label','System logs');
    const header = el('div',undefined,'system-logs-heading'), close = el('button','Close'); close.type = 'button'; close.onclick = () => dialog.close();
    header.append(el('h2','System logs'),close);dialog.append(header);document.body.append(dialog);dialog.showModal();
    const view = mount(dialog);
    dialog.addEventListener('close',()=>{view.destroy();dialog.remove();},{once:true});
  }
  function render(host) {
    if (!host._logsPanel) { host.replaceChildren();host._logsPanel = mount(host); }
    host._panelTick = () => {};
  }
  const actions = document.querySelector('.header-actions,.app-menu-panel');
  if (actions) { const button = el('button','System logs'); button.type='button';button.id='system-logs-open';button.onclick=open;actions.append(button); }
  return {mount,render,open};
})();
