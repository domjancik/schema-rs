(function (global) {
  'use strict';
  const W = global.SchemaWorkspace;
  const labels = {
    stack: 'Stack editor',
    setup: 'Fixture setup',
    scene: 'Scene view',
    inspector: 'Inspector',
    fixtures: 'Fixtures',
    output: 'Output',
    telemetry: 'Telemetry',
    logs: 'System logs',
    dmxtest: 'DMX test',
    gpu: '3D preview'
  };
  const compactLabels = {stack:'Stack',setup:'Setup',scene:'Scene',inspector:'Inspect',fixtures:'Fixtures',output:'Output',telemetry:'Stats',logs:'Logs',dmxtest:'DMX',gpu:'3D'};
  const panelIcons = {
    stack:'M5 4h14v4H5z M5 10h14v4H5z M5 16h14v4H5z',
    setup:'M12 3v18 M3 12h18 M12 3l-3 3 M12 3l3 3 M21 12l-3-3 M21 12l-3 3 M5 17l4-4 M5 17h4 M5 17v-4',
    scene:'M3 17l5-8 5 5 3-4 5 7z M17 4h.01 M3 20h18',
    inspector:'M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14 M15 15l6 6 M7 10h6 M10 7v6',
    fixtures:'M5 3h5v5H5z M14 3h5v5h-5z M5 12h5v5H5z M14 12h5v5h-5z M7.5 20h9',
    output:'M4 5h10v14H4z M7 9h4 M7 13h4 M14 12h7 M18 9l3 3-3 3',
    telemetry:'M3 19h18 M5 16v-5 M10 16V6 M15 16v-8 M20 16V3',
    logs:'M6 3h12v18H6z M9 7h6 M9 11h6 M9 15h4',
    dmxtest:'M5 4v16 M12 4v16 M19 4v16 M3 9h4 M10 15h4 M17 7h4',
    gpu:'M12 3l9 5v9l-9 5-9-5V8z M3 8l9 5 9-5 M12 13v9'
  };
  function panelIcon(view) {
    const icon=document.createElementNS('http://www.w3.org/2000/svg','svg');
    icon.setAttribute('viewBox','0 0 24 24');icon.setAttribute('aria-hidden','true');icon.setAttribute('focusable','false');
    const path=document.createElementNS(icon.namespaceURI,'path');path.setAttribute('d',panelIcons[view]);icon.append(path);return icon;
  }
  const element = (tag, text, cls) => {
    const n = document.createElement(tag);
    if (text !== undefined) n.textContent = text;
    if (cls) n.className = cls;
    return n;
  };
  function mount(options) {
    const main = options.main,
      stash = element('div');
    stash.hidden = true;
    stash.id = 'workspace-legacy';
    document.body.append(stash);
    const legacy = options.legacy;
    Object.values(legacy).forEach((n) => stash.append(n));
    const storage = 'schema-workspaces-v1';
    let state;
    try {
      state = W.upgradeAuthoring(W.validate(JSON.parse(localStorage.getItem(storage))));
    } catch {
      state = defaults();
    }
    let focused = null,
      maximized = null,
      history = [],
      timer,
      drag = null;
    const bodies = new Map();
    const compactMedia=matchMedia('(max-width:900px), (max-width:1100px) and (max-height:500px)');
    let compactSplit=false;try{compactSplit=localStorage.getItem('schema-compact-split')==='1';}catch{}
    function compactFocused(){return compactMedia.matches&&!compactSplit;}
    let compactChoices={};try{compactChoices=JSON.parse(localStorage.getItem('schema-compact-panels-v1')||'{}');if(!compactChoices||typeof compactChoices!=='object')compactChoices={};}catch{}
    function compactId(){const all=leaves();return all.find(p=>p.id===compactChoices[state.active])?.id||all.find(p=>p.view==='stack')?.id||all[0]?.id;}
    function applyCompact(){
      main.classList.toggle('workspace-compact',compactFocused());
      const active=compactId();
      for(const[id,{pane}]of bodies)pane.classList.toggle('compact-active',id===active);
      panelNav.hidden=!compactMedia.matches;
      for(const button of panelNav.children){
        const selected=button.dataset.targetPanel===active&&compactFocused();
        button.setAttribute('aria-pressed',String(selected));
        button.tabIndex=button.dataset.targetPanel===active?0:-1;
      }
      const selected=panelNav.querySelector('[aria-pressed="true"]');
      if(selected&&!panelNav.hidden){
        const item=selected.getBoundingClientRect(),row=panelNav.getBoundingClientRect();
        if(item.left<row.left)panelNav.scrollLeft+=item.left-row.left;
        else if(item.right>row.right)panelNav.scrollLeft+=item.right-row.right;
      }
    }
    function chooseCompact(id){compactChoices[state.active]=id;try{localStorage.setItem('schema-compact-panels-v1',JSON.stringify(compactChoices));}catch{}maximized=null;if(W.find(state,id)?.view==='stack')focus(id);applyMaximize();}
    compactMedia.addEventListener('change',()=>applyMaximize());
    function menuItems(details,summary){
      const items=element('div',undefined,'workspace-menu-items');details.append(summary,items);
      details.addEventListener('toggle',()=>{if(!details.open)return;const r=summary.getBoundingClientRect(),width=Math.min(260,innerWidth-16);items.style.width=width+'px';items.style.left=Math.max(8,Math.min(innerWidth-width-8,r.right-width))+'px';items.style.top=Math.max(8,Math.min(innerHeight-items.offsetHeight-8,r.bottom+6))+'px';});
      return items;
    }

    const bar = element('nav', undefined, 'workspace-tabs');
    bar.setAttribute('aria-label', 'Workspaces');
    main.before(bar);
    const panelNav=element('nav',undefined,'mobile-panel-nav');
    panelNav.setAttribute('aria-label','Studio panels');panelNav.hidden=true;
    const footer=document.querySelector('body>footer');
    if(footer)footer.after(panelNav);else main.after(panelNav);
    panelNav.onkeydown=event=>{
      if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;
      const buttons=[...panelNav.children],current=buttons.indexOf(document.activeElement);
      if(current<0)return;
      event.preventDefault();
      const index=event.key==='Home'?0:event.key==='End'?buttons.length-1:(current+(event.key==='ArrowRight'?1:-1)+buttons.length)%buttons.length;
      buttons[index].click();panelNav.children[index]?.focus();
    };

    main.classList.add('schema-workspace');
    function authoring() { return W.authoringRoot(); }
    function defaults() {
      const s = W.create();
      s.workspaces[0].root = authoring();
      s.authoringLayoutVersion=2;
      return W.validate(s);
    }
    function persist() {
      try {
        localStorage.setItem(storage, JSON.stringify(state));
      } catch (error) {
        options.status('Workspace could not be saved: ' + error.message, true);
      }
    }
    function save(immediate = false) {
      clearTimeout(timer);
      if (immediate) persist();
      else timer = setTimeout(persist, 120);
    }
    window.addEventListener('pagehide', persist);
    function leaves() {
      return W.panels(W.active(state).root);
    }
    function remember() {
      if (focused && W.find(state, focused))
        state = W.context(state, focused, options.capture());
    }
    function load(panel) {
      if (panel?.view === 'stack') options.activate(panel.context, panel.id);
    }
    function change(fn) {
      try {
        remember();
        const next = fn(state);
        history.push(structuredClone(state));
        if (history.length > 30) history.shift();
        state = next;
        maximized = null;
        const current = W.find(state, focused);
        if (!current || current.view !== 'stack')
          focused = leaves().find((p) => p.view === 'stack')?.id || null;
        load(W.find(state, focused));
        save(true);
        draw();
        options.changed?.();
      } catch (error) {
        options.status(error.message, true);
      }
    }
    function focus(id) {
      const panel = W.find(state, id);
      if (!panel || panel.view !== 'stack') return;
      remember();
      focused = id;
      load(panel);
      save();
      attach();
      options.changed?.();
    }
    function tabs() {
      bar.replaceChildren();
      for (const workspace of state.workspaces) {
        const b = element(
          'button',
          workspace.name,
          workspace.id === state.active ? 'active' : ''
        );
        b.setAttribute('aria-pressed', String(workspace.id === state.active));
        b.onclick = () => change((s) => W.workspace(s, workspace.id));
        b.ondblclick = () => rename();
        bar.append(b);
      }
      const add = element('button', '+');
      add.setAttribute('aria-label', 'New workspace');
      add.onclick = () => {
        const name = prompt('Workspace name', 'Workspace');
        if (name) change((s) => W.workspace(s, 'new', name));
      };
      const menu = element('details', undefined, 'workspace-menu'),
        summary = element('summary', 'Workspace ▾');
      const workspaceMenuItems=menuItems(menu,summary),workspaceChoice=element('select');
      workspaceChoice.setAttribute('aria-label','Choose workspace');for(const w of state.workspaces)workspaceChoice.append(new Option(w.name,w.id));workspaceChoice.value=state.active;workspaceChoice.onchange=()=>change(s=>W.workspace(s,workspaceChoice.value));workspaceMenuItems.append(workspaceChoice);const compactToggle=element('button',compactSplit?'Focus one panel':'Show split layout','compact-layout-toggle');compactToggle.onclick=()=>{compactSplit=!compactSplit;try{localStorage.setItem('schema-compact-split',compactSplit?'1':'0');}catch{}applyMaximize();tabs();};workspaceMenuItems.append(compactToggle);

      for (const [title, action] of [
        ['Rename workspace', rename],
        ['Delete workspace', () => change((s) => W.workspace(s, 'delete'))],
        [
          'Reset layout',
          () =>
            change((s) => {
              const n = structuredClone(s);
              W.active(n).root = authoring();
              return W.validate(n);
            })
        ],
        [
          'Undo layout',
          () => {
            if (!history.length) return;
            state = history.pop();
            maximized = null;
            focused = leaves().find((p) => p.view === 'stack')?.id;
            load(W.find(state, focused));
            save(true);
            draw();
            options.changed?.();
          }
        ]
      ]) {
        const b = element('button', title);
        b.onclick = () => {
          menu.open = false;
          action();
        };
        workspaceMenuItems.append(b);
      }
      bar.append(add, menu);


      const hint = element(
        'span',
        'Drag a header to dock · Ctrl Space maximize',
        'workspace-hint'
      );
      bar.append(hint);
      panelNav.replaceChildren();
      const counts={},seen={};
      for(const panel of leaves())counts[panel.view]=(counts[panel.view]||0)+1;
      for(const panel of leaves()){
        const number=(seen[panel.view]||0)+1;seen[panel.view]=number;
        const suffix=counts[panel.view]>1?' '+number:'',name=labels[panel.view]+suffix;
        const button=element('button',undefined,'mobile-panel-button');
        button.type='button';button.dataset.targetPanel=panel.id;button.dataset.view=panel.view;
        button.setAttribute('aria-label',name);button.setAttribute('aria-controls','workspace-pane-'+panel.id);button.title=name;
        button.append(panelIcon(panel.view),element('span',compactLabels[panel.view]+suffix));
        button.onclick=()=>{
          const wasSplit=compactSplit;compactSplit=false;
          if(wasSplit)try{localStorage.setItem('schema-compact-split','0');}catch{}
          chooseCompact(panel.id);
          if(wasSplit)tabs();
        };
        panelNav.append(button);
      }
      applyCompact();
    }
    function rename() {
      const value = prompt('Workspace name', W.active(state).name);
      if (value) change((s) => W.rename(s, value));
    }
    function header(panel) {
      const h = element('header', undefined, 'panel-header'),
        grip = element('button', '⠿', 'panel-grip');
      grip.setAttribute('aria-label', 'Move panel');
      grip.title = 'Drag onto a panel edge to dock; center to exchange';
      grip.onpointerdown = (e) => startDrag(e, panel.id);
      const select = element('select');
      select.setAttribute('aria-label', 'View type');
      for (const view of W.views) {
        const o = element('option', labels[view]);
        o.value = view;
        select.append(o);
      }
      select.value = panel.view;
      select.onchange = () =>
        change((s) => W.setView(s, panel.id, select.value));
      h.append(grip, select);
      options.decorate?.(panel, h, (patch) => setContext(panel.id, patch));
      const controls = element('details', undefined, 'panel-menu'),
        summary = element('summary', '⋯');
      summary.setAttribute('aria-label', 'Panel actions');
      const panelMenuItems=menuItems(controls,summary);
      for (const [title, action] of [
        ['Split side by side', () => change((s) => W.split(s, panel.id, 'x'))],
        ['Split above / below', () => change((s) => W.split(s, panel.id, 'y'))],
        [
          'Maximize / restore',
          () => {
            remember();
            maximized = maximized === panel.id ? null : panel.id;
            applyMaximize();
          }
        ],
        ['Close panel', () => change((s) => W.close(s, panel.id))]
      ]) {
        const b = element('button', title);
        b.onclick = () => {
          controls.open = false;
          action();
        };
        panelMenuItems.append(b);
      }
      h.append(controls);
      h.ondblclick = (e) => {
        if (e.target === h) {
          maximized = maximized === panel.id ? null : panel.id;
          applyMaximize();
        }
      };
      return h;
    }
    // Camera preferences update the model without remounting independent views.
    function setCamera(id, sceneCamera) {
      try {
        state = W.context(state, id, { sceneCamera });
        save();
      } catch (error) {
        options.status(error.message, true);
      }
    }
    function setContext(id, patch) {
      try {
        if (id === focused) remember();
        state = W.context(state, id, patch);
        if (id === focused) load(W.find(state, id));
        save();
        draw();
        options.changed?.();
      } catch (error) {
        options.status(error.message, true);
      }
    }
    function node(tree) {
      if (tree.kind === 'panel') {
        const pane = element(
          'section',
          undefined,
          'workspace-panel' + (tree.view === 'stack' ? ' layer-viewport' : '')
        );
        pane.id = 'workspace-pane-'+tree.id;
        pane.dataset.panel = tree.id;
        pane.dataset.view = tree.view;
        pane.dataset.layer = tree.context.layer ?? 0;
        pane.setAttribute('aria-label', labels[tree.view] + ' panel');
        pane.append(header(tree));
        const body = element('div', undefined, 'panel-body');
        pane.append(body);
        bodies.set(tree.id, { body, pane });
        return pane;
      }
      let currentRatio = tree.ratio;
      const box = element('div', undefined, 'panel-split ' + tree.axis),
        a = element('div', undefined, 'split-child'),
        b = element('div', undefined, 'split-child'),
        divider = element('div', undefined, 'panel-divider');
      a.style.flexBasis = `${tree.ratio * 100}%`;
      b.style.flexBasis = `${(1 - tree.ratio) * 100}%`;
      a.append(node(tree.a));
      b.append(node(tree.b));
      divider.tabIndex = 0;
      divider.role = 'separator';
      divider.setAttribute(
        'aria-orientation',
        tree.axis === 'x' ? 'vertical' : 'horizontal'
      );
      divider.setAttribute('aria-label', 'Resize panels');
      divider.setAttribute('aria-valuenow', Math.round(tree.ratio * 100));
      divider.onkeydown = (e) => {
        if (
          !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)
        )
          return;
        e.preventDefault();
        change((s) =>
          W.resize(
            s,
            tree.id,
            currentRatio +
              (['ArrowLeft', 'ArrowUp'].includes(e.key) ? -0.03 : 0.03)
          )
        );
      };
      divider.onpointerdown = (e) => {
        e.preventDefault();
        divider.setPointerCapture(e.pointerId);
        const initial = state,
          rect = box.getBoundingClientRect();
        let ratio = tree.ratio;
        divider.onpointermove = (event) => {
          ratio = Math.max(
            0.1,
            Math.min(
              0.9,
              tree.axis === 'x'
                ? (event.clientX - rect.left) / rect.width
                : (event.clientY - rect.top) / rect.height
            )
          );
          a.style.flexBasis = `${ratio * 100}%`;
          b.style.flexBasis = `${(1 - ratio) * 100}%`;
        };
        divider.onpointerup = () => {
          divider.onpointermove = null;
          history.push(initial);
          state = W.resize(state, tree.id, ratio);
          currentRatio = ratio;
          divider.setAttribute('aria-valuenow', Math.round(ratio * 100));
          save(true);
        };
        divider.onpointercancel = () => {
          divider.onpointermove = null;
          draw();
        };
      };
      box.append(a, divider, b);
      return box;
    }
    function framePolicy() {
      for (const [id, entry] of bodies) {
        const frame = entry.body.querySelector('iframe');
        if (!frame) continue;
        const send = () =>
          frame.contentWindow?.postMessage(
            {
              type: 'schema-panel-visibility',
              active: maximized ? maximized === id : !compactFocused() || compactId() === id,
              maximized: maximized === id
            },
            location.origin
          );
        if (!frame.dataset.policy) {
          frame.dataset.policy = 'bound';
          frame.addEventListener('load', send);
        }
        send();
      }
    }
    function applyMaximize() {
      main.classList.toggle('panel-maximized', !!maximized);
      for (const [id, entry] of bodies)
        entry.pane.classList.toggle('maximized', id === maximized);
      applyCompact();
      framePolicy();
    }
    window.addEventListener('message', (event) => {
      if (
        event.origin !== location.origin ||
        !['schema-panel-maximize', 'schema-panel-restore'].includes(
          event.data?.type
        )
      )
        return;
      const id = event.data.panel,
        entry = bodies.get(id),
        frame = entry?.body.querySelector('iframe');
      if (!frame || frame.contentWindow !== event.source) return;
      if (event.data.type === 'schema-panel-restore') {
        if (maximized === id) maximized = null;
      } else maximized = maximized === id ? null : id;
      applyMaximize();
    });
    function draw() {
      for(const entry of bodies.values()){entry.body._logsPanel?.destroy();entry.body._logsPanel=null;entry.body._dmxTest?.destroy();entry.body._outputPanel?.destroy();entry.body._sceneCamera?.destroy();}
      Object.values(legacy).forEach((n) => stash.append(n));
      bodies.clear();
      main.replaceChildren(node(W.active(state).root));
      tabs();
      attach();
      applyMaximize();
    }
    function attach() {
      const firstScene = leaves().find((p) => p.view === 'setup')?.id,
        firstInspector = leaves().find(
          (p) => p.view === 'inspector' && !p.context.pinned
        )?.id;
      for (const panel of leaves()) {
        const entry = bodies.get(panel.id);
        if (!entry) continue;
        const { body, pane } = entry;
        pane.classList.toggle('active', panel.id === focused);
        pane.dataset.layer = panel.context.layer ?? 0;
        pane.dataset.fixture = panel.context.fixture || '';
        const selectedLayer = pane.querySelector(
            'select[aria-label="Panel layer"]'
          ),
          selectedFixture = pane.querySelector(
            'select[aria-label="Panel fixture"]'
          );
        if (selectedFixture && options.project) {
          const project = options.project(), fixtures = [...project.fixtures, ...(project.director ? [{id:'__director',name:'Director'}] : [])],
            signature = JSON.stringify(fixtures.map(f => [f.id,f.name]));
          if (selectedFixture.dataset.options !== signature) {
            selectedFixture.replaceChildren(...fixtures.map(f => new Option(f.name,f.id)));
            selectedFixture.dataset.options = signature;
          }
          const preferred = panel.context.fixture || options.capture().fixture;
          selectedFixture.value = fixtures.some(f => f.id===preferred) ? preferred : fixtures[0]?.id || '';
          if (selectedLayer) {
            const layers = selectedFixture.value==='__director' ? project.director : project.content[selectedFixture.value] || [];
            if (selectedLayer.options.length !== layers.length) selectedLayer.replaceChildren(...layers.map((_,i)=>new Option(`Layer ${i+1}`,i)));
          }
        } else if (selectedFixture && panel.context.fixture)
          selectedFixture.value = panel.context.fixture;
        if (selectedLayer && panel.context.layer !== undefined)
          selectedLayer.value = Math.max(0,Math.min(panel.context.layer,selectedLayer.options.length-1));
        const primary =
          panel.view === 'stack' && panel.id === focused
            ? legacy.stack
            : panel.id === firstScene
              ? legacy.scene
              : panel.id === firstInspector
                ? legacy.inspector
                : null;
        if (primary) {
          if (primary.parentNode !== body) {
            body.replaceChildren(primary);
          }
          if (panel.view === 'stack') options.renderActive?.();
        } else {
          if ([...body.children].some((n) => Object.values(legacy).includes(n)))
            Object.values(legacy)
              .filter((n) => n.parentNode === body)
              .forEach((n) => stash.append(n));
          options.render(panel, body, {
            focus: () => focus(panel.id),
            context: (patch) => setContext(panel.id, patch),
            persistCamera: (sceneCamera) => setCamera(panel.id, sceneCamera)
          });
        }
      }
    }
    function startDrag(e, id) {
      if (e.button !== 0) return;
      e.preventDefault();
      e.currentTarget.setPointerCapture(e.pointerId);
      drag = {
        id,
        x: e.clientX,
        y: e.clientY,
        pointer: e.pointerId,
        active: false,
        target: null
      };
    }
    window.addEventListener('pointermove', (e) => {
      if (!drag || e.pointerId !== drag.pointer) return;
      if (
        !drag.active &&
        Math.hypot(e.clientX - drag.x, e.clientY - drag.y) < 7
      )
        return;
      drag.active = true;
      document
        .querySelectorAll('.workspace-panel[data-dock]')
        .forEach((n) => delete n.dataset.dock);
      const pane = document
        .elementFromPoint(e.clientX, e.clientY)
        ?.closest('.workspace-panel');
      if (!pane || pane.dataset.panel === drag.id) {
        drag.target = null;
        return;
      }
      const r = pane.getBoundingClientRect(),
        x = (e.clientX - r.left) / r.width,
        y = (e.clientY - r.top) / r.height,
        edge =
          x < 0.22
            ? 'left'
            : x > 0.78
              ? 'right'
              : y < 0.22
                ? 'top'
                : y > 0.78
                  ? 'bottom'
                  : 'center';
      pane.dataset.dock = edge;
      drag.target = { id: pane.dataset.panel, edge };
    });
    window.addEventListener('pointerup', (e) => {
      if (!drag || e.pointerId !== drag.pointer) return;
      const moving = drag;
      drag = null;
      document
        .querySelectorAll('.workspace-panel[data-dock]')
        .forEach((n) => delete n.dataset.dock);
      if (moving.active && moving.target)
        change((s) =>
          W.move(s, moving.id, moving.target.id, moving.target.edge)
        );
    });
    window.addEventListener('pointercancel', () => {
      drag = null;
      document
        .querySelectorAll('.workspace-panel[data-dock]')
        .forEach((n) => delete n.dataset.dock);
    });
    document.addEventListener(
      'keydown',
      (e) => {
        if (
          e.code === 'Space' &&
          e.ctrlKey &&
          !e.target.closest('input,textarea,select')
        ) {
          e.preventDefault();
          e.stopImmediatePropagation();
          remember();
          maximized = maximized
            ? null
            : e.target.closest('.workspace-panel')?.dataset.panel || focused;
          applyMaximize();
        } else if (e.key === 'Escape' && maximized) {
          e.preventDefault();
          maximized = null;
          applyMaximize();
        }
      },
      true
    );
    focused = leaves().find((p) => p.view === 'stack')?.id || null;
    load(W.find(state, focused));
    draw();
    save(true);
    return {
      refresh: attach,
      tick() {
        for (const [id, entry] of bodies) {
          const panel = W.find(state, id);
          if (panel && (!maximized || maximized===id) && (!compactFocused() || maximized===id || compactId()===id)) options.live?.(panel, entry.body);
        }
      },
      output(){
        if(!leaves().some(p=>p.view==='output'))change(s=>W.ensureOutput(s,focused));
        const output=leaves().find(p=>p.view==='output');if(output){chooseCompact(output.id);bodies.get(output.id)?.pane.querySelector('.panel-header select')?.focus();}
      },
      focus,
      focused: () => focused,
      panel: () => W.find(state, focused),
      context: (patch) => {
        if (focused) {
          state = W.context(state, focused, patch);
          save();
        }
      },
      snapshot: () => structuredClone(state),
      layers(fixture, count) {
        change((s) => {
          const n = structuredClone(s);
          function grid(indices) {
            if (indices.length === 1)
              return W.panel('stack', {
                fixture,
                layer: indices[0],
                scale: 0.65,
                pan: { x: 20, y: 40 }
              });
            const middle = Math.ceil(indices.length / 2);
            return {
              kind: 'split',
              id: crypto.randomUUID(),
              axis: indices.length > 2 ? 'y' : 'x',
              ratio: 0.5,
              a: grid(indices.slice(0, middle)),
              b: grid(indices.slice(middle))
            };
          }
          const existing = n.workspaces.find((w) => w.name === 'Layers');
          const root = {
            kind: 'split',
            id: crypto.randomUUID(),
            axis: 'x',
            ratio: 0.8,
            a: grid(Array.from({ length: Math.min(count, 12) }, (_, i) => i)),
            b: W.panel('inspector')
          };
          if (existing) {
            existing.root = root;
            n.active = existing.id;
          } else {
            const w = { id: crypto.randomUUID(), name: 'Layers', root };
            n.workspaces.push(w);
            n.active = w.id;
          }
          return W.validate(n);
        });
      },
      restore() {
        const w =
          state.workspaces.find((w) => w.name === 'Authoring') ||
          state.workspaces.find((w) => w.name !== 'Layers');
        if (w) change((s) => W.workspace(s, w.id));
      },
      destroy() {
        clearTimeout(timer);
        bar.remove();
        panelNav.remove();
        Object.values(legacy).forEach((n) => main.append(n));
        stash.remove();
      }
    };
  }
  global.SchemaWorkspaceUI = { mount };
})(globalThis);
