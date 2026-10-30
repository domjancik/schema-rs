(function (global) {
  'use strict';
  const views = ['stack', 'setup', 'scene', 'inspector', 'fixtures', 'output', 'telemetry', 'gpu', 'dmxtest'];
  let serial = 0;
  const id = () => `panel-${Date.now().toString(36)}-${(++serial).toString(36)}`;
  const copy = (value) => structuredClone(value),
    panel = (view = 'stack', context = {}) => ({
      kind: 'panel',
      id: id(),
      view,
      context: copy(context)
    });
  function panels(node) {
    return node.kind === 'panel' ? [node] : [...panels(node.a), ...panels(node.b)];
  }
  function active(state) {
    return state.workspaces.find((w) => w.id === state.active);
  }
  function validContext(c) {
    if(c.sceneStacks!==undefined&&!['off','selected','all'].includes(c.sceneStacks))throw Error('Invalid scene stack scope');
    if(c.sceneStackScale!==undefined&&(!Number.isFinite(c.sceneStackScale)||c.sceneStackScale<.15||c.sceneStackScale>1))throw Error('Invalid scene stack size');
    for (const key of ['fractal', 'square'])
      if (c[key] !== undefined && typeof c[key] !== 'boolean') throw Error('Invalid stack layout');
    if (c.layer !== undefined && (!Number.isInteger(c.layer) || c.layer < 0 || c.layer >= 32))
      throw Error('Invalid panel layer');
    if (c.fixture !== undefined && (typeof c.fixture !== 'string' || c.fixture.length > 100))
      throw Error('Invalid panel fixture');
    for (const key of ['scale', 'sceneZoom'])
      if (c[key] !== undefined && (!Number.isFinite(c[key]) || c[key] < (key === 'scale' ? 1e-6 : 0.05) || c[key] > 16))
        throw Error('Invalid view scale');
    for (const key of ['pan', 'scenePan'])
      if (
        c[key] !== undefined &&
        (!c[key] ||
          !['x', 'y'].every(
            (axis) => Number.isFinite(c[key][axis]) && Math.abs(c[key][axis]) <= 10000000
          ))
      )
        throw Error('Invalid view navigation');
    if (c.sceneCamera !== undefined) {
      const n = c.sceneCamera;
      if (
        !n ||
        !Array.isArray(n.target) ||
        n.target.length !== 3 ||
        !n.target.every((v) => Number.isFinite(v) && Math.abs(v) <= 1e7) ||
        !Number.isFinite(n.zoom) ||
        n.zoom < 1e-5 ||
        n.zoom > 1e5 ||
        !Number.isFinite(n.yaw) ||
        Math.abs(n.yaw) > Math.PI * 2 ||
        !Number.isFinite(n.pitch) ||
        Math.abs(n.pitch) > Math.PI / 2 ||
        !['orbit', 'pan'].includes(n.mode)
      )
        throw Error('Invalid scene camera');
    }
    if (c.node !== undefined && (!Number.isSafeInteger(c.node) || c.node < 0))
      throw Error('Invalid node selection');
    if (
      c.folded !== undefined &&
      (!Array.isArray(c.folded) ||
        c.folded.length > 2048 ||
        c.folded.some((id) => !Number.isSafeInteger(id) || id < 0))
    )
      throw Error('Invalid folded blocks');
    if (c.axes !== undefined && !['xz', 'xy', 'yz', '3d'].includes(c.axes))
      throw Error('Invalid scene axes');
    if (
      c.driverTrail !== undefined &&
      (!Array.isArray(c.driverTrail) ||
        c.driverTrail.length > 64 ||
        c.driverTrail.some(
          (d) =>
            !d || !Number.isSafeInteger(d.owner) || typeof d.key !== 'string' || d.key.length > 200
        ))
    )
      throw Error('Invalid driver context');
  }
  function validate(state) {
    if (
      !state ||
      state.version !== 1 ||
      !Array.isArray(state.workspaces) ||
      !state.workspaces.length ||
      state.workspaces.length > 12
    )
      throw Error('Invalid workspace document');
    const ids = new Set();
    let total = 0;
    function check(node, depth = 0) {
      if (
        !node ||
        depth > 16 ||
        typeof node.id !== 'string' ||
        node.id.length > 100 ||
        ids.has(node.id)
      )
        throw Error('Invalid panel identity');
      ids.add(node.id);
      if (node.kind === 'panel') {
        if (
          !views.includes(node.view) ||
          !node.context ||
          typeof node.context !== 'object' ||
          Array.isArray(node.context)
        )
          throw Error('Invalid view instance');
        validContext(node.context);
        if (++total > 192) throw Error('Too many panels');
      } else if (node.kind === 'split') {
        if (
          !['x', 'y'].includes(node.axis) ||
          !Number.isFinite(node.ratio) ||
          node.ratio < 0.1 ||
          node.ratio > 0.9
        )
          throw Error('Invalid split');
        check(node.a, depth + 1);
        check(node.b, depth + 1);
      } else throw Error('Unknown panel node');
    }
    const names = new Set();
    for (const w of state.workspaces) {
      if (
        typeof w.id !== 'string' ||
        names.has(w.id) ||
        typeof w.name !== 'string' ||
        !w.name.trim() ||
        w.name.length > 60
      )
        throw Error('Invalid workspace');
      names.add(w.id);
      check(w.root);
      if (panels(w.root).length > 16) throw Error('Panel limit reached (16)');
    }
    if (!active(state)) throw Error('Missing active workspace');
    if (JSON.stringify(state).length > 262144) throw Error('Workspace exceeds size limit');
    return state;
  }
  function create() {
    const workspace = { id: id(), name: 'Authoring', root: panel() };
    return { version: 1, active: workspace.id, workspaces: [workspace] };
  }
  function change(state, fn) {
    const next = copy(state);
    fn(next, active(next));
    return validate(next);
  }
  function replace(node, target, fn) {
    if (node.id === target) return fn(node);
    if (node.kind === 'split') {
      node.a = replace(node.a, target, fn);
      node.b = replace(node.b, target, fn);
    }
    return node;
  }
  function find(state, target) {
    return panels(active(state).root).find((p) => p.id === target);
  }
  function split(state, target, axis, view) {
    return change(state, (_, w) => {
      let found = false;
      w.root = replace(w.root, target, (old) => {
        if (old.kind !== 'panel') throw Error('Choose a panel');
        found = true;
        const second = panel(view || old.view, old.context);
        return { kind: 'split', id: id(), axis, ratio: 0.5, a: old, b: second };
      });
      if (!found) throw Error('Panel no longer exists');
    });
  }
  function remove(node, target) {
    if (node.id === target) return null;
    if (node.kind === 'split') {
      node.a = remove(node.a, target);
      node.b = remove(node.b, target);
      return node.a && node.b ? node : node.a || node.b;
    }
    return node;
  }
  function close(state, target) {
    return change(state, (_, w) => {
      if (!find(state, target)) throw Error('Panel no longer exists');
      const next = remove(w.root, target);
      if (!next) throw Error('Keep at least one panel');
      w.root = next;
    });
  }
  function context(state, target, patch) {
    return change(state, (_, w) => {
      let found = false;
      w.root = replace(w.root, target, (node) => {
        if (node.kind !== 'panel') throw Error('Choose a panel');
        found = true;
        node.context = { ...node.context, ...copy(patch) };
        return node;
      });
      if (!found) throw Error('Panel no longer exists');
    });
  }
  function setView(state, target, view) {
    if (!views.includes(view)) throw Error('Unknown view');
    return change(state, (_, w) => {
      w.root = replace(w.root, target, (node) => ({ ...node, view, context: {} }));
    });
  }
  function swap(state, source, target) {
    return change(state, (_, w) => {
      const a = find(state, source),
        b = find(state, target);
      if (!a || !b) throw Error('Panel no longer exists');
      w.root = replace(w.root, source, () => ({ ...copy(b), id: source }));
      w.root = replace(w.root, target, () => ({ ...copy(a), id: target }));
    });
  }
  function resize(state, target, ratio) {
    return change(state, (_, w) => {
      w.root = replace(w.root, target, (node) => {
        if (node.kind !== 'split') throw Error('Choose a divider');
        node.ratio = Math.max(0.1, Math.min(0.9, ratio));
        return node;
      });
    });
  }
  function move(state, source, target, edge) {
    if (source === target) return state;
    if (edge === 'center') return swap(state, source, target);
    return change(state, (_, w) => {
      const item = find(state, source);
      if (!item || !find(state, target)) throw Error('Panel no longer exists');
      w.root = remove(w.root, source);
      w.root = replace(w.root, target, (node) => {
        const before = ['left', 'top'].includes(edge);
        return {
          kind: 'split',
          id: id(),
          axis: ['left', 'right'].includes(edge) ? 'x' : 'y',
          ratio: 0.5,
          a: before ? item : node,
          b: before ? node : item
        };
      });
    });
  }
  function maximize(state, target) {
    return change(state, (_, w) => {
      const item = find(state, target);
      if (!item) throw Error('Panel no longer exists');
      w.root = copy(item);
    });
  }
  function workspace(state, action, name) {
    return change(state, (next) => {
      if (action === 'new') {
        const clone = copy(active(next));
        clone.id = id();
        clone.name = (name || 'Workspace').trim();
        function identities(node) {
          node.id = id();
          if (node.kind === 'split') {
            identities(node.a);
            identities(node.b);
          }
        }
        identities(clone.root);
        next.workspaces.push(clone);
        next.active = clone.id;
      } else if (action === 'delete') {
        if (next.workspaces.length === 1) throw Error('Keep at least one workspace');
        next.workspaces = next.workspaces.filter((w) => w.id !== next.active);
        next.active = next.workspaces[0].id;
      } else {
        if (!next.workspaces.some((w) => w.id === action))
          throw Error('Workspace no longer exists');
        next.active = action;
      }
    });
  }
  function rename(state, name) {
    return change(state, (_, w) => {
      w.name = name.trim();
    });
  }
  function outputSplit(stack) {
    return { kind: 'split', id: id(), axis: 'y', ratio: 0.7, a: stack, b: panel('output') };
  }
  function authoringRoot() {
    return {kind:'split',id:id(),axis:'x',ratio:.21,a:panel('setup'),b:{kind:'split',id:id(),axis:'x',ratio:.72,a:outputSplit(panel('stack')),b:panel('inspector')}};
  }
  function upgradeAuthoring(state) {
    const next=copy(state);
    if(next.authoringLayoutVersion===2)return validate(next);
    next.authoringLayoutVersion=2;
    for(const workspace of next.workspaces){
      const r=workspace.root;
      if(workspace.name==='Authoring'&&r.kind==='split'&&r.axis==='x'&&r.ratio===.21&&r.a.view==='setup'&&r.b.kind==='split'&&r.b.axis==='x'&&r.b.ratio===.72&&r.b.a.view==='stack'&&r.b.b.view==='inspector') r.b.a=outputSplit(r.b.a);
    }
    return validate(next);
  }
  function ensureOutput(state,target) {
    if(panels(active(state).root).some(p=>p.view==='output'))return copy(state);
    return change(state,(_,workspace)=>{
      const all=panels(workspace.root),anchor=all.find(p=>p.id===target)||all.find(p=>p.view==='stack')||all[0];
      workspace.root=replace(workspace.root,anchor.id,outputSplit);
    });
  }
  const api = {
    views,
    authoringRoot,
    upgradeAuthoring,
    ensureOutput,
    create,
    validate,
    panels,
    active,
    find,
    split,
    close,
    context,
    setView,
    swap,
    resize,
    move,
    maximize,
    workspace,
    rename,
    panel
  };
  if (typeof module !== 'undefined') module.exports = api;
  else global.SchemaWorkspace = api;
})(globalThis);
