/* Transient, bounded fold motion. The authored tree and input targets stay separate. */
(function (global) {
  'use strict';
  const LIMIT = 96,
    states = new WeakMap(),
    active = new Set();
  const clamp = (n, a, b) => Math.max(a, Math.min(b, n));
  function anchor(origin, old, next) {
    return {
      x: old.right - (origin.x + (next.x + next.width) * origin.scale),
      y: old.top - (origin.y + next.y * origin.scale)
    };
  }
  function plan(before, after, { owner, closing, knownBeforeIds = before.map((n) => n.id) }) {
    const known = new Set([...knownBeforeIds].map(String));
    const old = new Map(before.map((n) => [String(n.id), n])),
      next = new Map(after.map((n) => [String(n.id), n]));
    const parent = next.get(String(owner)) ||
        old.get(String(owner)) || { x: 0, y: 0, width: 178, height: 112 },
      pivot = { x: parent.x + parent.width / 2, y: parent.y + 12 },
      result = [];
    for (const n of after) {
      if (result.length >= LIMIT) break;
      const was = old.get(String(n.id));
      if (!was && known.has(String(n.id))) continue;
      if (!was || was.ghost) {
        const dx = was ? was.x + was.width / 2 - n.x - n.width / 2 : pivot.x - n.x - n.width / 2,
          dy = was
            ? was.y + was.height * 0.65 - n.y - n.height * 0.65
            : pivot.y - n.y - n.height * 0.65,
          sx = was ? was.width / n.width : 0.16,
          sy = was ? was.height / n.height : 0.2;
        result.push({
          id: String(n.id),
          kind: 'enter',
          duration: 310,
          delay: Math.min(48, result.length * 8),
          frames: [
            {
              transform: `translate(${dx}px,${dy}px) scale(${sx},${sy}) rotate(-2deg)`,
              opacity: was?.opacity || 0
            },
            {
              offset: 0.76,
              transform: 'translate(0px,-3px) scale(1.025,1.015) rotate(.3deg)',
              opacity: 1
            },
            { transform: 'none', opacity: 1 }
          ]
        });
      } else if (
        ['x', 'y', 'width', 'height'].some((k) => Math.abs(was[k] - n[k]) > 0.1) ||
        was.opacity < 0.99
      ) {
        const settle = {
          x: n.x + clamp((n.x - was.x) * 0.025, -5, 5),
          y: n.y + clamp((n.y - was.y) * 0.025, -4, 4),
          width: n.width + clamp((n.width - was.width) * 0.025, -4, 4)
        };
        if (String(n.id) === String(owner)) settle.x = n.x + n.width - settle.width;
        result.push({
          id: String(n.id),
          kind: 'move',
          duration: closing ? 280 : 340,
          delay: 0,
          frames: [
            {
              left: was.x + 'px',
              top: was.y + 'px',
              width: was.width + 'px',
              height: was.height + 'px',
              opacity: was.opacity ?? 1
            },
            {
              offset: 0.78,
              left: settle.x + 'px',
              top: settle.y + 'px',
              width: settle.width + 'px',
              height: n.height + 'px',
              opacity: 1
            },
            {
              left: n.x + 'px',
              top: n.y + 'px',
              width: n.width + 'px',
              height: n.height + 'px',
              opacity: 1
            }
          ]
        });
      }
    }
    for (const n of before) {
      if (result.length >= LIMIT) break;
      if (next.has(String(n.id))) continue;
      const dx = pivot.x - n.x - n.width / 2,
        dy = pivot.y - n.y - n.height * 0.65;
      result.push({
        id: String(n.id),
        kind: 'exit',
        duration: 220,
        delay: Math.min(42, result.filter((p) => p.kind === 'exit').length * 6),
        frames: [
          { transform: 'none', opacity: n.opacity ?? 1 },
          {
            offset: 0.65,
            transform: `translate(${dx * 0.85}px,${dy * 0.85}px) scale(.28,.38) rotate(2deg)`,
            opacity: 0.3
          },
          { transform: `translate(${dx}px,${dy}px) scale(.14,.18) rotate(3deg)`, opacity: 0 }
        ]
      });
    }
    return result;
  }
  const reduce = () =>
    typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
  function origin(world) {
    const r = world.getBoundingClientRect(),
      matrix = new DOMMatrixReadOnly(getComputedStyle(world).transform);
    return { x: r.left, y: r.top, scale: Math.hypot(matrix.a, matrix.b) || 1 };
  }
  function snapshot(element, id, ghost = false) {
    const rect = element.getBoundingClientRect();
    return {
      id: String(id),
      rect: {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        width: rect.width,
        height: rect.height
      },
      opacity: Number(getComputedStyle(element).opacity),
      ghost,
      element
    };
  }
  function capture(world, owner) {
    const nodes = [...world.querySelectorAll('.block[data-node]')],
      selected = nodes.find((n) => n.dataset.node === String(owner)),
      allIds = new Set(nodes.map((n) => n.dataset.node));
    if (reduce()) {
      const items = new Map(selected ? [[String(owner), snapshot(selected, owner)]] : []);
      return { origin: origin(world), items, owner: items.get(String(owner)), allIds };
    }
    const x = selected ? parseFloat(selected.style.left) : 0,
      y = selected ? parseFloat(selected.style.top) : 0,
      right = x + (selected ? parseFloat(selected.style.width) : 0);
    const descendants = selected
      ? nodes
          .filter(
            (n) =>
              n !== selected &&
              parseFloat(n.style.top) < y &&
              parseFloat(n.style.left) >= x - 0.1 &&
              parseFloat(n.style.left) + parseFloat(n.style.width) <= right + 0.1
          )
          .sort((a, b) => parseFloat(b.style.top) - parseFloat(a.style.top))
      : [];
    const important = new Set(descendants),
      ordered = [
        ...(selected ? [selected] : []),
        ...descendants,
        ...nodes.filter((n) => n !== selected && !important.has(n))
      ].slice(0, LIMIT),
      items = new Map(ordered.map((e) => [e.dataset.node, snapshot(e, e.dataset.node)]));
    const state = states.get(world);
    for (const [id, e] of state?.ghosts || []) {
      if (items.size >= LIMIT) break;
      if (!items.has(id)) items.set(id, snapshot(e, id, true));
    }
    for (const item of items.values()) {
      item.clone = item.element.cloneNode(true);
      item.clone.querySelectorAll('.fold-shell-before').forEach((e) => e.remove());
    }
    return { origin: origin(world), items, owner: items.get(String(owner)), allIds };
  }
  function finish(world, state, notify) {
    if (states.get(world) !== state) return;
    states.delete(world);
    active.delete(world);
    delete world.dataset.foldAnimating;
    for (const animation of state.animations) animation.cancel();
    state.layer?.remove();
    for (const e of state.entering) {
      e.inert = false;
      delete e.dataset.foldEntering;
    }
    state.owner?.style.setProperty('z-index', state.zIndex);
    state.oldShell?.remove();
    if (notify) state.done?.();
  }
  function cancel(world) {
    const s = states.get(world);
    if (s) finish(world, s, false);
  }
  function hold(world, scope, revision) {
    const s = states.get(world);
    if (!s) return false;
    if (s.scope !== scope || s.revision !== revision || reduce()) {
      cancel(world);
      return false;
    }
    return true;
  }
  function play(world, before, { owner, closing, scope, revision, done }) {
    cancel(world);
    if (reduce() || typeof world.animate !== 'function' || !before?.items.size) return false;
    const current = origin(world),
      convert = (item) => ({
        id: item.id,
        x: (item.rect.left - current.x) / current.scale,
        y: (item.rect.top - current.y) / current.scale,
        width: item.rect.width / current.scale,
        height: item.rect.height / current.scale,
        opacity: item.opacity,
        ghost: item.ghost
      }),
      old = [...before.items.values()].map(convert),
      nodes = new Map(
        [...world.querySelectorAll('.block[data-node]')].map((e) => [e.dataset.node, e])
      ),
      next = [...nodes.values()].map((e) => ({
        id: e.dataset.node,
        x: parseFloat(e.style.left),
        y: parseFloat(e.style.top),
        width: parseFloat(e.style.width),
        height: parseFloat(e.style.height),
        opacity: 1
      })),
      effects = plan(old, next, { owner, closing, knownBeforeIds: before.allIds });
    const state = {
      scope,
      revision,
      done,
      animations: [],
      ghosts: new Map(),
      entering: [],
      owner: nodes.get(String(owner)),
      zIndex: nodes.get(String(owner))?.style.zIndex || ''
    };
    states.set(world, state);
    active.add(world);
    world.dataset.foldAnimating = String(owner) + (closing ? ':closed' : ':open');
    const animate = (e, frames, options) => {
      const a = e.animate(frames, {
        fill: 'both',
        easing: 'cubic-bezier(.2,.85,.2,1)',
        ...options
      });
      state.animations.push(a);
      return a;
    };
    for (const effect of effects) {
      let element = nodes.get(effect.id);
      if (effect.kind === 'exit') {
        const item = before.items.get(effect.id),
          geometry = old.find((n) => n.id === effect.id);
        element = item.clone;
        element.classList.add('fold-exit-ghost');
        element.classList.remove('selected', 'drag-origin', 'drop-ghost');
        for (const key of ['id', 'data-node', 'data-layer', 'data-fixture', 'tabindex'])
          element.removeAttribute(key);
        element.querySelectorAll('[id]').forEach((e) => e.removeAttribute('id'));
        element.querySelectorAll('button,[tabindex]').forEach((e) => e.remove());
        element.setAttribute('aria-hidden', 'true');
        element.inert = true;
        Object.assign(element.style, {
          left: geometry.x + 'px',
          top: geometry.y + 'px',
          width: geometry.width + 'px',
          height: geometry.height + 'px',
          transformOrigin: '50% 65%',
          pointerEvents: 'none'
        });
        if (!state.layer) {
          state.layer = document.createElement('div');
          state.layer.className = 'fold-motion-layer';
          state.layer.setAttribute('aria-hidden', 'true');
          state.layer.inert = true;
          world.append(state.layer);
        }
        state.layer.append(element);
        state.ghosts.set(effect.id, element);
      } else if (effect.kind === 'enter') {
        element.inert = true;
        element.dataset.foldEntering = '';
        element.style.transformOrigin = '50% 65%';
        state.entering.push(element);
      }
      if (element)
        animate(element, effect.frames, { duration: effect.duration, delay: effect.delay });
    }
    if (state.owner) {
      state.owner.style.zIndex = '4';
      const shell = state.owner.querySelector('.block-shell'),
        oldOwner = before.items.get(String(owner));
      if (shell && oldOwner) {
        const echo = oldOwner.clone.querySelector('.block-shell')?.cloneNode(true);
        if (echo) {
          echo.classList.add('fold-shell-before');
          echo.setAttribute('aria-hidden', 'true');
          shell.after(echo);
          state.oldShell = echo;
          animate(echo, [{ opacity: 1 }, { opacity: 0 }], { duration: 170 });
        }
        animate(
          shell,
          [
            { transform: 'scaleY(1)' },
            { offset: 0.45, transform: closing ? 'scaleY(.97)' : 'scaleY(1.025)' },
            { transform: 'scaleY(1)' }
          ],
          { duration: closing ? 250 : 340 }
        );
      }
      const glyph = state.owner.querySelector('.fold');
      if (glyph)
        animate(
          glyph,
          [
            { transform: 'scale(1) rotate(0deg)' },
            { offset: 0.45, transform: `scale(.82) rotate(${closing ? -14 : 14}deg)` },
            { transform: 'scale(1) rotate(0deg)' }
          ],
          { duration: 260 }
        );
    }
    Promise.allSettled(state.animations.map((a) => a.finished)).then(() =>
      finish(world, state, true)
    );
    return true;
  }
  if (typeof matchMedia === 'function')
    matchMedia('(prefers-reduced-motion: reduce)').addEventListener?.('change', (e) => {
      if (e.matches)
        for (const world of [...active]) {
          const s = states.get(world);
          if (s) finish(world, s, true);
        }
    });
  if (typeof document !== 'undefined')
    document.addEventListener('visibilitychange', () => {
      if (document.hidden)
        for (const world of [...active]) {
          const s = states.get(world);
          if (s) finish(world, s, true);
        }
    });
  const api = { plan, anchor, capture, play, hold, cancel };
  global.SchemaFoldMotion = api;
  if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
