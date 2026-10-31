/* Lightweight orthographic 3D navigation; camera state never enters the engine. */
(function (global) {
  'use strict';
  const clamp = (n, a, b) => Math.max(a, Math.min(b, n)),
    dot = (a, b) => a.reduce((v, x, i) => v + x * b[i], 0);
  const defaults = {
    target: [0, 0, 0],
    yaw: Math.PI / 4,
    pitch: Math.asin(1 / Math.sqrt(3)),
    zoom: 20,
    mode: 'orbit'
  };
  function valid(c) {
    if (
      !c ||
      !Array.isArray(c.target) ||
      c.target.length !== 3 ||
      !c.target.every((n) => Number.isFinite(n) && Math.abs(n) <= 1e7) ||
      !Number.isFinite(c.yaw) ||
      Math.abs(c.yaw) > Math.PI * 2 ||
      !Number.isFinite(c.pitch) ||
      Math.abs(c.pitch) > Math.PI / 2 ||
      !Number.isFinite(c.zoom) ||
      c.zoom < 1e-5 ||
      c.zoom > 1e5 ||
      !['orbit', 'pan'].includes(c.mode)
    )
      throw Error('Invalid scene camera');
    return { ...c, target: [...c.target] };
  }
  function basis(c) {
    return {
      right: [Math.cos(c.yaw), 0, -Math.sin(c.yaw)],
      up: [
        -Math.sin(c.yaw) * Math.sin(c.pitch),
        Math.cos(c.pitch),
        -Math.cos(c.yaw) * Math.sin(c.pitch)
      ],
      eye: [
        Math.sin(c.yaw) * Math.cos(c.pitch),
        Math.sin(c.pitch),
        Math.cos(c.yaw) * Math.cos(c.pitch)
      ]
    };
  }
  function project(p, c, w, h) {
    const b = basis(c),
      v = p.map((n, i) => n - c.target[i]);
    return {
      x: w / 2 + dot(v, b.right) * c.zoom,
      y: h / 2 - dot(v, b.up) * c.zoom,
      depth: dot(v, b.eye)
    };
  }
  function navigate(c, { orbit, pan, zoom } = {}) {
    const n = valid(c);
    if (orbit) {
      n.yaw =
        ((((n.yaw - orbit[0] * 0.008 + Math.PI) % (Math.PI * 2)) + Math.PI * 2) % (Math.PI * 2)) -
        Math.PI;
      n.pitch = clamp(n.pitch + orbit[1] * 0.008, -1.565, 1.565);
    }
    if (pan) {
      const b = basis(n);
      n.target = n.target.map((v, i) =>
        clamp(v + (-b.right[i] * pan[0] + b.up[i] * pan[1]) / n.zoom, -1e7, 1e7)
      );
    }
    if (zoom !== undefined) {
      if (!Number.isFinite(zoom) || zoom <= 0) throw Error('Invalid camera zoom');
      n.zoom = clamp(n.zoom * zoom, 1e-5, 1e5);
    }
    return valid(n);
  }
  function preset(c, name) {
    const values = {
      iso: [Math.PI / 4, Math.asin(1 / Math.sqrt(3))],
      front: [0, 0],
      right: [Math.PI / 2, 0],
      top: [0, Math.PI / 2 - 0.001]
    };
    if (!values[name]) throw Error('Unknown camera viewpoint');
    return valid({ ...c, yaw: values[name][0], pitch: values[name][1] });
  }
  function rotate(c, steps) {
    if (!Number.isFinite(steps)) throw Error('Invalid rotation');
    const n = valid(c);
    n.yaw =
      ((((n.yaw + (steps * Math.PI) / 4 + Math.PI) % (2 * Math.PI)) + 2 * Math.PI) %
        (2 * Math.PI)) -
      Math.PI;
    return n;
  }
  function fit(points, w, h, c = defaults) {
    const p = points.filter((p) => Array.isArray(p) && p.length === 3 && p.every(Number.isFinite)),
      n = valid(c);
    if (!p.length) return { ...defaults, target: [0, 0, 0] };
    n.target = [0, 1, 2].map(
      (i) => (Math.min(...p.map((v) => v[i])) + Math.max(...p.map((v) => v[i]))) / 2
    );
    const projected = p.map((v) => project(v, { ...n, zoom: 1 }, 0, 0)),
      rx = Math.max(0.5, ...projected.map((v) => Math.abs(v.x))),
      ry = Math.max(0.5, ...projected.map((v) => Math.abs(v.y)));
    const padding = Math.min(24, w / 6, h / 6);
    n.zoom = clamp(
      Math.min(Math.max(1, w - padding * 2) / (rx * 2), Math.max(1, h - padding * 2) / (ry * 2)),
      1e-5,
      1e5
    );
    return n;
  }
  function mount(view, options) {
    const ns = 'http://www.w3.org/2000/svg',
      svg = (tag, attrs = {}) => {
        const e = document.createElementNS(ns, tag);
        for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
        return e;
      };
    let camera;
    try {
      camera = valid(options.camera);
    } catch {
      camera = null;
    }
    let placement;
    let labelKey="",labelPositions=[];
    let automaticFit = !camera;
    let width = 360,
      height = 200,
      initial = null,
      moved = false,
      multi = false,
      disposed = false,
      selectionUntil = 0;
    const pointers = new Map(),
      buttons = [];
    view.classList.add('schematic-scene');
    view.setAttribute('tabindex', '0');
    view.setAttribute('aria-label', '3D scene camera');
    view.style.touchAction = 'none';
    const state = () => options.data(),
      points = () =>
        state().snapshot.fixtures.flatMap((f) => [
          f.fixture.position,
          ...(f.result.position ? [f.result.position] : [])
        ]);
    function commit() {
      if (camera) options.changed?.(valid(camera));
    }
    function change(action) {
      automaticFit = false;
      camera = navigate(camera, action);
      draw();
      commit();
    }
    function viewpoint(name) {
      camera = preset(camera, name);
      if (name === 'iso') camera.mode = 'orbit';
      draw();
      commit();
    }
    function rotateView(steps) {
      automaticFit = false;
      camera = rotate(camera, steps);
      draw();
      commit();
    }
    function fitAll() {
      automaticFit = true;
      camera = fit(points(), width, height, camera || defaults);
      draw();
      commit();
    }
    function frame() {
      const f = state().project.fixtures.find((f) => f.id === options.selected());
      if (f) {
        automaticFit = false;
        camera = { ...camera, target: [...f.position] };
        draw();
        commit();
      }
    }
    function draw() {
      if (disposed) return;
      const focusedFixture = view.contains(document.activeElement)
        ? document.activeElement.closest('[data-scene-fixture]')?.dataset.sceneFixture
        : null;
      const rect = view.getBoundingClientRect();
      const resized = width !== Math.max(1, rect.width) || height !== Math.max(1, rect.height);
      width = Math.max(1, rect.width);
      height = Math.max(1, rect.height);
      if (!camera || (automaticFit && resized))
        camera = fit(points(), width, height, camera || defaults);
      view.setAttribute('viewBox', `0 0 ${width} ${height}`);
      view.dataset.camera = JSON.stringify(camera);
      view.replaceChildren();
      const span = Math.max(width, height) / camera.zoom,
        step = 10 ** Math.floor(Math.log10(Math.max(0.01, span / 6))),
        cx = Math.round(camera.target[0] / step) * step,
        cz = Math.round(camera.target[2] / step) * step;
      for (let i = -8; i <= 8; i++)
        for (const axis of [0, 2]) {
          const a =
              axis === 0 ? [cx + i * step, 0, cz - 8 * step] : [cx - 8 * step, 0, cz + i * step],
            b = axis === 0 ? [cx + i * step, 0, cz + 8 * step] : [cx + 8 * step, 0, cz + i * step],
            p = project(a, camera, width, height),
            q = project(b, camera, width, height);
          view.append(
            svg('path', {
              d: `M${p.x} ${p.y}L${q.x} ${q.y}`,
              class: 'schematic-grid',
              'pointer-events': 'none'
            })
          );
        }
      state()
        .snapshot.fixtures.map((f) => ({
          f,
          p: project(f.fixture.position, camera, width, height)
        }))
        .sort((a, b) => a.p.depth - b.p.depth)
        .forEach(({ f, p }) => {
          const color = SchemaAuthoringCore.outputColor(f.result);
          if (f.result.position) {
            const q = project(f.result.position, camera, width, height);
            if (Math.hypot(q.x - p.x, q.y - p.y) > 0.5) {
              view.append(
                svg('path', {
                  d: `M${p.x} ${p.y}L${q.x} ${q.y}`,
                  class: 'schematic-stem',
                  'pointer-events': 'none'
                }),
                svg('circle', {
                  cx: q.x,
                  cy: q.y,
                  r: 5,
                  fill: color,
                  'pointer-events': 'none',
                  'data-live-fixture': f.fixture.id
                })
              );
            }
          }
          const g = svg('g', {
            role: 'button',
            tabindex: 0,
            'aria-label': `Select ${f.fixture.name} in scene`,
            'aria-pressed': String(f.fixture.id === options.selected()),
            'data-scene-fixture': f.fixture.id,
            'data-fixture': f.fixture.id,
            class: 'stage-fixture'
          });
          if (f.fixture.id === options.selected()) g.append(svg('circle', {cx:p.x,cy:p.y,r:12,class:'fixture-selection','pointer-events':'none','aria-hidden':'true'}));
          g.append(
            svg('circle', { cx: p.x, cy: p.y, r: 18, fill: 'transparent', class: 'schematic-hit' }),
            svg('circle', {
              cx: p.x,
              cy: p.y,
              r: f.fixture.id === options.selected() ? 8 : 4,
              fill: color,
              style: `--fixture-marker:${SchemaAuthoringCore.markerColor(f.fixture) || "var(--state-selected,#72dec2)"}`,
              stroke: 'var(--accent,#72dec2)',
              'stroke-width': 2,
              'pointer-events': 'none',
              class: 'fixture-layout-dot'
            })
          );
          const label = svg('text', {
            x: p.x + 9,
            y: p.y - 9,
            'pointer-events': 'none',
            class: 'schematic-label'
          });
          label.textContent = options.compact
            ? String(state().project.fixtures.findIndex((a) => a.id === f.fixture.id) + 1).padStart(
                2,
                '0'
              )
            : f.fixture.name;
          const fullLabel=label.textContent;if([...fullLabel].length>24)label.textContent=[...fullLabel].slice(0,23).join('')+'…';
          g.append(label);
          g.onkeydown = (e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              e.stopPropagation();
              options.select(f.fixture.id);
            }
          };
          g.onclick = (e) => {
            if (e.detail === 0 && performance.now() > selectionUntil) options.select(f.fixture.id);
          };
          view.append(g);
          if(label.getComputedTextLength()>180)label.textContent=[...fullLabel].slice(0,11).join('')+'…';
        });
      if(global.SchemaAuthoringCore){
        const points=[...view.querySelectorAll('[data-scene-fixture]')].map(g=>{const dot=g.querySelector('.fixture-layout-dot');return {id:g.dataset.sceneFixture,name:g.querySelector('.schematic-label').textContent,labelWidth:g.querySelector('.schematic-label').getComputedTextLength(),x:Number(dot.getAttribute('cx')),y:Number(dot.getAttribute('cy'))};});
        const key=JSON.stringify([width,height,options.selected(),points]);if(key!==labelKey){labelKey=key;labelPositions=global.SchemaAuthoringCore.labels(points,width,height,options.selected());}
        for(const label of labelPositions){const text=view.querySelector(`[data-scene-fixture="${CSS.escape(label.id)}"] .schematic-label`);text.setAttribute('x',label.x);text.setAttribute('y',label.y+12);text.style.display=label.visible?'':'none';}
      }
      if (focusedFixture) {
        const item = [...view.querySelectorAll('[data-scene-fixture]')].find(
          (e) => e.dataset.sceneFixture === focusedFixture
        );
        item?.focus({ preventScroll: true });
      }
      for (const b of buttons)
        b.setAttribute('aria-pressed', String(b.dataset.mode === camera.mode));
      placement?.update();
      options.painted?.(camera);
    }
    view.oncontextmenu = (e) => e.preventDefault();
    view.onpointerdown = (e) => {
      if (placement?.active || e.button > 2) return;
      view.focus({ preventScroll: true });
      if (!pointers.size) {
        initial = valid(camera);
        moved = false;
        multi = false;
      }
      pointers.set(e.pointerId, {
        x: e.clientX,
        y: e.clientY,
        startX: e.clientX,
        startY: e.clientY,
        fixture: e.target.closest('[data-scene-fixture]')?.dataset.sceneFixture,
        pan: e.button !== 0 || e.shiftKey
      });
      if (pointers.size > 1) {
        multi = true;
        moved = true;
      }
      view.setPointerCapture(e.pointerId);
      e.preventDefault();
    };
    view.onpointermove = (e) => {
      const p = pointers.get(e.pointerId);
      if (!p) return;
      const old = [...pointers.values()].map((v) => ({ ...v }));
      const dx = e.clientX - p.x,
        dy = e.clientY - p.y;
      p.x = e.clientX;
      p.y = e.clientY;
      if (!moved && Math.hypot(p.x - p.startX, p.y - p.startY) < 5) return;
      moved = true;
      automaticFit = false;
      if (pointers.size > 1) {
        const next = [...pointers.values()],
          mid = (a) => [(a[0].x + a[1].x) / 2, (a[0].y + a[1].y) / 2],
          a = mid(old),
          b = mid(next),
          distance = (a) => Math.hypot(a[0].x - a[1].x, a[0].y - a[1].y);
        camera = navigate(camera, {
          pan: [b[0] - a[0], b[1] - a[1]],
          zoom: distance(old) > 1 ? Math.max(0.1, distance(next) / distance(old)) : 1
        });
      } else
        camera = navigate(
          camera,
          p.pan || camera.mode === 'pan' ? { pan: [dx, dy] } : { orbit: [dx, dy] }
        );
      draw();
      e.preventDefault();
    };
    function end(e, cancel = false) {
      const p = pointers.get(e.pointerId);
      if (!p) return;
      pointers.delete(e.pointerId);
      if (view.hasPointerCapture(e.pointerId)) view.releasePointerCapture(e.pointerId);
      if (cancel) {
        pointers.clear();
        camera = initial;
        draw();
        selectionUntil = performance.now() + 400;
        return;
      }
      if (!pointers.size) {
        selectionUntil = performance.now() + 400;
        if (moved) commit();
        else if (!multi) {
          const point=new DOMPoint(e.clientX,e.clientY).matrixTransform(view.getScreenCTM().inverse());
          const points=[...view.querySelectorAll('[data-scene-fixture]')].map(g=>{const dot=g.querySelector('.fixture-layout-dot'),fixture=state().project.fixtures.find(f=>f.id===g.dataset.sceneFixture);return {...fixture,x:Number(dot.getAttribute('cx')),y:Number(dot.getAttribute('cy'))};});
          const near=global.SchemaAuthoringCore?.hits(points,point.x,point.y,e.pointerType==='touch'?26:18)||[];
          if(near.length>1&&global.SchemaAuthoring)global.SchemaAuthoring.choose(near,options.select,e);else if(near[0]||p.fixture)options.select(near[0]?.id||p.fixture);
        }
      }
    }
    view.onpointerup = (e) => end(e);
    view.onpointercancel = (e) => end(e, true);
    view.onlostpointercapture = (e) => end(e, true);
    view.onwheel = (e) => {
      e.preventDefault();
      if(placement?.active)return;
      const pixels = e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? height : 1);
      change({ zoom: Math.exp(clamp(-pixels, -300, 300) * 0.002) });
    };
    view.onkeydown = (e) => {
      if (['Delete', 'Backspace'].includes(e.key)) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }
      if (e.target !== view) return;
      if(placement?.active)return;
      const key = e.key.toLowerCase(),
        dx = { arrowleft: -18, arrowright: 18 }[key] || 0,
        dy = { arrowup: -18, arrowdown: 18 }[key] || 0;
      let handled = true;
      if (dx || dy) change(e.shiftKey ? { pan: [dx, dy] } : { orbit: [dx, dy] });
      else if (key === '+' || key === '=') change({ zoom: 1.2 });
      else if (key === '-') change({ zoom: 1 / 1.2 });
      else if (key === 'f') fitAll();
      else if (key === 'home') {
        camera = preset(camera, 'iso');
        draw();
        commit();
      } else if (['1', '3', '7'].includes(key)) {
        camera = preset(camera, { 1: 'front', 3: 'right', 7: 'top' }[key]);
        draw();
        commit();
      } else handled = false;
      if (handled) {
        e.preventDefault();
        e.stopPropagation();
      }
    };
    function toolbar(host) {
      const details = document.createElement('details');
      details.className = 'schematic-camera';
      const summary = document.createElement('summary');
      summary.textContent = 'Camera';
      details.append(summary);
      const hasPopover = 'showPopover' in HTMLElement.prototype;
      const popup = document.createElement(hasPopover ? 'div' : 'dialog');
      popup.className = 'schematic-camera-popup';
      if (hasPopover) popup.setAttribute('popover', 'manual');
      details.append(popup);
      details.addEventListener('toggle', () => {
        if (details.open) {
          if (hasPopover) {
            if (!popup.matches(':popover-open')) popup.showPopover();
          } else if (!popup.open) popup.showModal();
        } else if (hasPopover) {
          if (popup.matches(':popover-open')) popup.hidePopover();
        } else if (popup.open) popup.close();
      });
      popup.addEventListener('cancel', () => {
        details.open = false;
      });
      details.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && details.open) {
          e.preventDefault();
          e.stopPropagation();
          details.open = false;
          summary.focus();
        }
      });

      const add = (name, label, action) => {
        const b = document.createElement('button');
        b.type = 'button';
        b.textContent = label;
        b.setAttribute('aria-label', name);
        b.onclick = action;
        b.onkeydown = (e) => {
          if (e.key === ' ' || e.key === 'Enter') {
            e.preventDefault();
            e.stopPropagation();
            if (!e.repeat) action();
          }
        };
        popup.append(b);
        return b;
      };
      for (const mode of ['orbit', 'pan']) {
        const b = add(
          mode === 'orbit' ? 'Orbit camera' : 'Pan camera',
          mode === 'orbit' ? 'Orbit' : 'Pan',
          () => {
            camera = { ...camera, mode };
            draw();
            commit();
          }
        );
        b.dataset.mode = mode;
        buttons.push(b);
      }
      add('Fit scene camera', 'Fit', fitAll);
      add('Frame selected fixture', 'Frame', frame);
      for (const name of ['iso', 'front', 'right', 'top'])
        add(name + ' camera viewpoint', name[0].toUpperCase() + name.slice(1), () => {
          camera = preset(camera, name);
          draw();
          commit();
        });
      for (const [label, delta] of [
        ['←', [-18, 0]],
        ['↑', [0, -18]],
        ['↓', [0, 18]],
        ['→', [18, 0]]
      ])
        add('Move camera ' + label, label, () =>
          change(camera.mode === 'pan' ? { pan: delta } : { orbit: delta })
        );
      add('Zoom camera in', '+', () => change({ zoom: 1.2 }));
      add('Zoom camera out', '−', () => change({ zoom: 1 / 1.2 }));
      const hint = document.createElement('p');
      hint.textContent =
        'Drag to orbit · Shift / right drag to pan · two fingers pan + pinch · wheel zoom';
      popup.append(hint);
      add('Close camera controls', 'Close', () => {
        details.open = false;
        summary.focus();
      });
      if (options.directIso !== false)
        host.append(add('Isometric view', 'Iso', () => viewpoint('iso')));
      host.append(
        add('Rotate view left 45°', '↶', () => rotateView(-1)),
        add('Rotate view right 45°', '↷', () => rotateView(1)),
        details
      );
      placement?.toolbar(host);
      draw();
      return details;
    }
    if(global.SchemaPlacement)placement=global.SchemaPlacement.mount({view:()=>view,host:view.parentElement,key:'schematic:'+(view.closest('[data-panel]')?.dataset.panel||view.id),data:options.data,selected:options.selected,resize:draw,fit:fitAll,frame:()=>{const f=state().project.fixtures.find(f=>f.id===options.selected());if(f){automaticFit=false;camera={...camera,target:[...f.position],zoom:Math.max(40,camera.zoom)};draw();commit();}},project:p=>({...project(p,camera,width,height),w:1}),active:on=>{if(on){pointers.clear();automaticFit=false;commit();}}});
    draw();
    return {
      update: draw,
      toolbar,
      fit: fitAll,
      frame,
      navigate: change,
      viewpoint,
      rotate: rotateView,
      get: () => valid(camera),
      destroy() {
        disposed = true;
        placement?.destroy();
        for (const key of [
          'onpointerdown',
          'onpointermove',
          'onpointerup',
          'onpointercancel',
          'onlostpointercapture',
          'onwheel',
          'onkeydown',
          'oncontextmenu'
        ])
          view[key] = null;
      }
    };
  }
  const api = { rotate, valid, project, navigate, preset, fit, mount };
  global.SchemaSchematicCamera = api;
  if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
