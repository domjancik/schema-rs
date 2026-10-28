(function (global) {
  'use strict';
  const make = (tag, text, cls) => {
    const e = document.createElement(tag);
    if (text !== undefined) e.textContent = text;
    if (cls) e.className = cls;
    return e;
  };
  function adapters(a) {
    const getLayers = (p, id) => (id === '__director' ? p.director : p.content[id] || []);
    function render(panel, body, api) {
      body._sceneCamera?.destroy();
      body._sceneCamera = null;
      const data = a.data();
      if (!data) return;
      if (panel.view === 'gpu') {
        if (!body.querySelector('iframe')) {
          const frame = make('iframe', undefined, 'panel-preview');
          frame.src = '/preview?panel=' + encodeURIComponent(panel.id);
          frame.title = 'Independent 3D scene preview';
          frame.allow = 'fullscreen';
          body.replaceChildren(frame);
        }
        return;
      }
      if (panel.view === 'stack') {
        const c = { ...panel.context },
          fixture = c.fixture || a.context().fixture,
          layer = Math.max(0, Math.min(c.layer || 0, getLayers(data.project, fixture).length - 1)),
          stack = getLayers(data.project, fixture)[layer];
        body.replaceChildren();
        const surface = make('div', undefined, 'layer-canvas'),
          world = make('div', undefined, 'layer-world'),
          empty = make('div', undefined, 'layer-empty'),
          start = make('button', '+ Create stack');
        start.onclick = () => {
          api.focus();
          a.create();
        };
        empty.append(make('p', 'An open layer.'), start);
        surface.append(world, empty);
        body.append(surface);
        let displayed = stack?.root;
        for (const step of c.driverTrail || [])
          displayed = global.SchemaBlocks.find(displayed, step.owner)?.drivers[
            step.key
          ];
        a.drawStack(displayed || null, {
          world,
          empty,
          surface,
          layer,
          view: { pan: c.pan, scale: c.scale, folded: new Set(c.folded || []), fractal: !!c.fractal, square: !!c.square },
          fixture,
          activate: api.focus
        });
        const label = make(
          'span',
          `${fixture} / Layer ${layer + 1}${stack?.enabled === false ? ' · OFF' : ''}`,
          'panel-layer-label'
        );
        body.append(label);
        body.closest('.workspace-panel').dataset.fixture = fixture;
        body.closest('.workspace-panel').dataset.layer = layer;
        body.closest('.workspace-panel').classList.toggle('muted', stack?.enabled === false);
        surface.onwheel = (e) => {
          e.preventDefault();
          const scale = Math.max(
            1e-6,
            Math.min(16, (c.scale || 0.65) * (e.deltaY < 0 ? 1.08 : 1 / 1.08))
          );
          api.context({ scale });
        };
        surface.onpointerdown = (e) => {
          if (e.button !== 0 || e.target.closest('.block,button')) return;
          e.preventDefault();
          surface.setPointerCapture(e.pointerId);
          const start = { x: e.clientX, y: e.clientY },
            pan = c.pan || { x: 20, y: 40 };
          let next = pan;
          surface.onpointermove = (event) => {
            next = { x: pan.x + event.clientX - start.x, y: pan.y + event.clientY - start.y };
            world.style.transform = `translate(${next.x}px,${next.y}px) scale(${c.scale || 0.65})`;
          };
          surface.onpointerup = () => {
            surface.onpointermove = null;
            api.context({ pan: next });
          };
        };
        return;
      }
      if (panel.view === 'scene' || panel.view === 'setup') {
        scene(panel, body, api, data);
        return;
      }
      body._panelTick = null;
      body.replaceChildren();
      const box = make('div', undefined, 'panel-simple');
      body.append(box);
      if (panel.view === 'fixtures') {
        for (const f of data.project.fixtures) {
          const b = make('button', f.name);
          b.setAttribute('aria-label', 'Select ' + f.name);
          b.onclick = () => a.select(f.id);
          box.append(b);
        }
        const add = make('button', '+ Add fixture');
        add.onclick = () => document.getElementById('add-fixture').click();
        box.append(add);
        return;
      }
      if (panel.view === 'output') {
        const config = data.project.output;
        box.append(make('h3', config.transport === 'open_dmx' ? 'Output / Open DMX USB' : 'Output / Art-Net'));
        const connection = make('button', 'Configure connection…');
        connection.onclick = () => global.SchemaOutput.open('connection');
        box.append(connection);
        if (config.transport === 'open_dmx') box.append(make('p', `${config.device} · Universe ${config.universe || 0}`));
        const target = make('input');
        target.value = config.target;
        target.hidden = config.transport === 'open_dmx';
        target.setAttribute('aria-label', 'Output destination');
        const enabled = make('input');
        enabled.type = 'checkbox';
        enabled.checked = config.enabled;
        const label = make('label', 'Live DMX output');
        label.prepend(enabled);
        const apply = make('button', 'Apply output settings');
        apply.onclick = () =>
          a.command({
            type: 'set_output',
            config: { ...config, target: target.value, enabled: enabled.checked }
          });
        const design = make('button', 'DMX models / Universe patch');
        design.onclick = () => global.SchemaOutput.open();
        box.append(target, label, apply, design, make('p', data.snapshot.output_status));
        const outputStatus = box.lastChild;
        body._panelTick = () => {
          outputStatus.textContent = a.data().snapshot.output_status;
        };
        return;
      }
      if (panel.view === 'telemetry') {
        const text = make('pre');
        box.append(make('h3', 'Engine telemetry'), text);
        body._panelTick = () => {
          const s = a.data().snapshot;
          text.textContent = `Revision ${s.revision}\nFrame ${s.frame}\nTime ${s.time.toFixed(2)} s\nEvaluation ${s.evaluation_us} µs\nFixtures ${s.fixtures.length}\n${s.output_status}\n${s.diagnostics.join('\n')}`;
        };
        body._panelTick();
        return;
      }
      if (panel.view === 'inspector') {
        const context = panel.context.pinned ? panel.context : a.context(),
          fixture = context.fixture || a.context().fixture,
          layer = context.layer || 0,
          stack = getLayers(data.project, fixture)[layer],
          node = global.SchemaBlocks.find(stack?.root, context.node || stack?.root?.id);
        const pinned = make('input');
        pinned.type = 'checkbox';
        pinned.checked = !!panel.context.pinned;
        const pin = make('label', 'Pin this inspection');
        pin.prepend(pinned);
        pinned.onchange = () => api.context({ ...a.context(), pinned: pinned.checked });
        box.append(pin);
        if (!node) {
          box.append(make('p', 'Select a block.'));
          return;
        }
        const info = a.catalog().find((b) => b.key === node.behavior);
        box.append(make('h3', info?.name || node.behavior), make('p', info?.description || ''));
        const revision = data.snapshot.revision;
        for (const [key, value] of Object.entries({ ...info?.defaults, ...node.properties })) {
          const label = make('label', key);
          let input;
          if (info?.options?.[key]) {
            input = make('select');
            info.options[key].forEach((title, index) => {
              const o = make('option', title);
              o.value = index;
              input.append(o);
            });
            input.value = value;
          } else if (typeof value === 'boolean') {
            input = make('input');
            input.type = 'checkbox';
            input.checked = value;
          } else {
            input = make('input');
            input.value = Array.isArray(value) ? JSON.stringify(value) : value;
            input.type = typeof value === 'number' ? 'number' : 'text';
            if (input.type === 'number') input.step = 'any';
          }
          input.onchange = () => {
            try {
              const next =
                typeof value === 'boolean'
                  ? input.checked
                  : typeof value === 'number' || info?.options?.[key]
                    ? Number(input.value)
                    : Array.isArray(value)
                      ? JSON.parse(input.value)
                      : input.value;
              a.command(
                { type: 'set_property', fixture, layer, node: node.id, key, value: next },
                revision
              );
            } catch (error) {
              a.status(error.message, true);
            }
          };
          label.append(input);
          box.append(label);
        }
        return;
      }
    }
    function decorate(panel, header, update) {
      if (panel.view === 'stack') {
        const data = a.data(),
          context = panel.context,
          fixture = make('select', undefined, 'panel-context');
        fixture.setAttribute('aria-label', 'Panel fixture');
        for (const f of [...data.project.fixtures, { id: '__director', name: 'Director' }]) {
          const option = make('option', f.name);
          option.value = f.id;
          fixture.append(option);
        }
        fixture.value = context.fixture || a.context().fixture;
        const layers = getLayers(data.project, fixture.value),
          layer = make('select');
        layer.setAttribute('aria-label', 'Panel layer');
        layers.forEach((_, i) => {
          const option = make('option', `Layer ${i + 1}`);
          option.value = i;
          layer.append(option);
        });
        layer.value = Math.min(context.layer || 0, layers.length - 1);
        fixture.onchange = () =>
          update({ fixture: fixture.value, layer: 0, node: 0, driverTrail: [] });
        layer.onchange = () => update({ layer: Number(layer.value), node: 0, driverTrail: [] });
        const toggle = make('input');
        toggle.type = 'checkbox';
        toggle.setAttribute('aria-label', 'Enable panel layer');
        const program = layers[Number(layer.value)];
        toggle.checked = program?.enabled !== false;
        toggle.disabled = !program;
        toggle.style.width = '14px';
        toggle.onchange = () =>
          a.command({
            type: 'set_layer_enabled',
            fixture: fixture.value,
            layer: Number(layer.value),
            enabled: toggle.checked
          });
        const layout = make('details', undefined, 'stack-layout-menu');
        layout.append(make('summary', 'Layout'));
        const choices = make('div', undefined, 'stack-layout-choices');
        for (const [key, title, options] of [
          ['fractal', 'Sibling layout', ['Expanded', 'Fractal']],
          ['square', 'Block shape', ['Rectangle', 'Square']]
        ]) {
          const label = make('label', title), select = make('select');
          select.setAttribute('aria-label', title);
          options.forEach((title, i) => { const o = make('option', title); o.value = String(!!i); select.append(o); });
          select.value = String(!!context[key]);
          select.onchange = () => update({ [key]: select.value === 'true' });
          label.append(select); choices.append(label);
        }
        layout.append(choices);
        header.append(fixture, layer, toggle, layout);
      } else if (panel.view === 'inspector') {
        const pin = make('input');
        pin.type = 'checkbox';
        pin.checked = !!panel.context.pinned;
        pin.setAttribute('aria-label', 'Pin inspector');
        pin.title = 'Keep inspecting this block when another view changes selection';
        pin.onchange = () => update({ ...a.context(), pinned: pin.checked });
        header.append(pin);
      }
    }
    function scene(panel, body, api, data) {
      if (panel.context.axes === '3d') {
        body.replaceChildren();
        const view = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        view.classList.add('panel-scene');
        view.style.marginTop = '42px';
        view.style.height = 'calc(100% - 42px)';
        const controls = make('div', undefined, 'panel-scene-controls');
        for (const axes of ['3d', 'xz', 'xy', 'yz']) {
          const button = make('button', axes.toUpperCase());
          button.setAttribute('aria-pressed', String(axes === '3d'));
          button.onclick = () => api.context({ axes });
          controls.append(button);
        }
        body.append(view, controls);
        const navigation = SchemaSchematicCamera.mount(view, {
          camera: panel.context.sceneCamera,
          data: () => a.data(),
          selected: () => a.context().fixture,
          select: (id) => {
            const focused = view.contains(document.activeElement);
            a.select(id);
            if (focused)
              document
                .querySelector('[data-panel="' + CSS.escape(panel.id) + '"] .schematic-scene')
                ?.focus({ preventScroll: true });
          },
          changed: (sceneCamera) => {
            const focused = view.contains(document.activeElement);
            if(api.persistCamera)api.persistCamera(sceneCamera);else api.context({ sceneCamera });
            if (focused)
              document
                .querySelector('[data-panel="' + CSS.escape(panel.id) + '"] .schematic-scene')
                ?.focus({ preventScroll: true });
          }
        });
        navigation.toolbar(controls);
        body._sceneCamera = navigation;
        body._panelTick = () => {
          const top = controls.getBoundingClientRect().height + 16;
          view.style.marginTop = top + 'px';
          view.style.height = `calc(100% - ${top}px)`;
          navigation.update();
        };
        body._panelTick();
        return;
      }
      const c = panel.context,
        axes = c.axes || 'xz',
        indices = axes === 'xy' ? [0, 1] : axes === 'yz' ? [2, 1] : [0, 2],
        width = Math.max(160, body.clientWidth),
        height = Math.max(160, body.clientHeight),
        positions = data.project.fixtures.map((f) => f.position),
        limits = indices.map((axis) => [
          Math.min(0, ...positions.map((p) => p[axis])),
          Math.max(0, ...positions.map((p) => p[axis]))
        ]),
        fitZoom = Math.min(
          8,
          (width - 70) / (Math.max(1, limits[0][1] - limits[0][0]) * 20),
          (height - 70) / (Math.max(1, limits[1][1] - limits[1][0]) * 20)
        ),
        scale = c.sceneZoom || fitZoom,
        fitPan = {
          x: -(limits[0][0] + limits[0][1]) * 10 * scale,
          y: (limits[1][0] + limits[1][1]) * 10 * scale
        },
        pan = c.scenePan || fitPan,
        ns = 'http://www.w3.org/2000/svg',
        svg = (tag, attrs) => {
          const n = document.createElementNS(ns, tag);
          for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
          return n;
        };
      body.replaceChildren();
      const view = svg('svg', {
          viewBox: `0 0 ${width} ${height}`,
          class: 'panel-scene',
          'aria-label': axes.toUpperCase() + ' scene view'
        }),
        controls = make('div', undefined, 'panel-scene-controls');
      for (const axis of ['3d', 'xz', 'xy', 'yz']) {
        const b = make('button', axis.toUpperCase());
        b.onclick = () => api.context({ axes: axis });
        controls.append(b);
      }
      const fit = make('button', 'Fit');
      fit.onclick = () => api.context({ scenePan: fitPan, sceneZoom: fitZoom });
      controls.append(fit);
      body.append(view, controls);
      const point = (p) => [
        width / 2 + pan.x + p[indices[0]] * 20 * scale,
        height / 2 + pan.y - p[indices[1]] * 20 * scale
      ];
      for (let i = -30; i <= 30; i++) {
        const x = width / 2 + pan.x + i * 20 * scale,
          y = height / 2 + pan.y + i * 20 * scale;
        view.append(
          svg('line', {
            x1: x,
            x2: x,
            y1: 0,
            y2: height,
            stroke: 'var(--line)',
            'stroke-width': i === 0 ? 1.5 : 0.5
          }),
          svg('line', {
            x1: 0,
            x2: width,
            y1: y,
            y2: y,
            stroke: 'var(--line)',
            'stroke-width': i === 0 ? 1.5 : 0.5
          })
        );
      }
      for (const f of data.project.fixtures) {
        const live = data.snapshot.fixtures.find((s) => s.fixture.id === f.id),
          p = point(f.position),
          q = point(live?.result.position || f.position),
          color = live?.result.colors?.[0] || [0.45, 0.87, 0.76, 1],
          fill = `rgb(${color
            .slice(0, 3)
            .map((v) => Math.round(Math.max(0, Math.min(1, v)) * 255))
            .join(',')})`;
        const liveDot = svg('circle', {
          cx: q[0],
          cy: q[1],
          r: 8,
          fill,
          opacity: 0.5,
          'pointer-events': 'none'
        });
        liveDot.dataset.fixture = f.id;
        view.append(liveDot);
        const dot = svg('circle', {
          cx: p[0],
          cy: p[1],
          r: 5,
          fill: 'var(--paper)',
          stroke: fill,
          'stroke-width': 2,
          tabindex: 0,
          role: 'button',
          'aria-label': 'Select ' + f.name
        });
        dot.onclick = () => a.select(f.id);
        dot.onkeydown = (e) => {
          if (e.key === 'Enter') a.select(f.id);
        };
        const label = svg('text', { x: p[0] + 9, y: p[1] - 7, 'pointer-events': 'none' });
        label.textContent = f.name;
        view.append(dot, label);
      }
      body._panelTick = () => {
        for (const dot of view.querySelectorAll('circle[data-fixture]')) {
          const live = a.data().snapshot.fixtures.find((f) => f.fixture.id === dot.dataset.fixture);
          if (!live) continue;
          const position = point(live.result.position || live.fixture.position),
            color = live.result.colors?.[0] || [0, 0, 0, 1];
          dot.setAttribute('cx', position[0]);
          dot.setAttribute('cy', position[1]);
          dot.setAttribute(
            'fill',
            `rgb(${color
              .slice(0, 3)
              .map((v) => Math.round(Math.max(0, Math.min(1, v)) * 255))
              .join(',')})`
          );
        }
      };
      view.onwheel = (e) => {
        e.preventDefault();
        api.context({
          sceneZoom: Math.max(0.15, Math.min(8, scale * (e.deltaY < 0 ? 1.1 : 1 / 1.1)))
        });
      };
      view.onpointerdown = (e) => {
        if (e.button !== 0 || e.target.closest('[role=button]')) return;
        view.setPointerCapture(e.pointerId);
        const start = { x: e.clientX, y: e.clientY };
        let next = pan;
        view.onpointermove = (event) => {
          const rect = view.getBoundingClientRect();
          next = {
            x: pan.x + ((event.clientX - start.x) * width) / rect.width,
            y: pan.y + ((event.clientY - start.y) * height) / rect.height
          };
        };
        view.onpointerup = () => {
          view.onpointermove = null;
          api.context({ scenePan: next });
        };
      };
    }
    return { render, decorate, live: (_panel, body) => body._panelTick?.() };
  }
  global.SchemaStudioPanels = { adapters };
})(globalThis);
