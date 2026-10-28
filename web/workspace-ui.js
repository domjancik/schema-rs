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
    gpu: '3D preview'
  };
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
      state = W.validate(JSON.parse(localStorage.getItem(storage)));
    } catch {
      state = defaults();
    }
    let focused = null,
      maximized = null,
      history = [],
      timer,
      drag = null;
    const bodies = new Map();
    const bar = element('nav', undefined, 'workspace-tabs');
    bar.setAttribute('aria-label', 'Workspaces');
    main.before(bar);
    main.classList.add('schema-workspace');
    function authoring() {
      const scene = W.panel('setup'),
        stack = W.panel('stack'),
        inspector = W.panel('inspector');
      return {
        kind: 'split',
        id: crypto.randomUUID(),
        axis: 'x',
        ratio: 0.21,
        a: scene,
        b: {
          kind: 'split',
          id: crypto.randomUUID(),
          axis: 'x',
          ratio: 0.72,
          a: stack,
          b: inspector
        }
      };
    }
    function defaults() {
      const s = W.create();
      s.workspaces[0].root = authoring();
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
      menu.append(summary);
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
        menu.append(b);
      }
      bar.append(add, menu);
      const hint = element(
        'span',
        'Drag a header to dock · Ctrl Space maximize',
        'workspace-hint'
      );
      bar.append(hint);
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
      controls.append(summary);
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
        controls.append(b);
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
              active: !maximized || maximized === id,
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
        if (selectedFixture && panel.context.fixture)
          selectedFixture.value = panel.context.fixture;
        if (selectedLayer && panel.context.layer !== undefined)
          selectedLayer.value = panel.context.layer;
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
            context: (patch) => setContext(panel.id, patch)
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
    return {
      refresh: attach,
      tick() {
        for (const [id, entry] of bodies) {
          const panel = W.find(state, id);
          if (panel) options.live?.(panel, entry.body);
        }
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
        Object.values(legacy).forEach((n) => main.append(n));
        stash.remove();
      }
    };
  }
  global.SchemaWorkspaceUI = { mount };
})(globalThis);
