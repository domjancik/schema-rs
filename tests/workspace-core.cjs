'use strict';
const assert = require('node:assert/strict');
const W = require('../web/workspace-core.js');
let state = W.create();
const first = W.panels(state.workspaces[0].root)[0];
state = W.split(state, first.id, 'x');
let rows = W.panels(state.workspaces[0].root);
assert.equal(rows.length, 2);
assert.equal(rows[0].view, rows[1].view);
state = W.context(state, rows[1].id, { fixture: 'fixture-2', layer: 2, scale: 0.4 });
assert.notDeepEqual(
  W.panels(state.workspaces[0].root)[0].context,
  W.panels(state.workspaces[0].root)[1].context
);
state = W.setView(state, rows[1].id, 'scene');
state = W.swap(state, rows[0].id, rows[1].id);
assert.equal(W.panels(state.workspaces[0].root)[0].view, 'scene');
state = W.workspace(state, 'new', 'Lighting');
assert.equal(state.workspaces.length, 2);
const original = state.workspaces[0].root;
state = W.rename(state, 'Lighting setup');
assert.equal(state.workspaces[1].name, 'Lighting setup');
assert.deepEqual(state.workspaces[0].root, original);
state = W.validate(JSON.parse(JSON.stringify(state)));
const active = W.panels(state.workspaces[1].root);
state = W.close(state, active[1].id);
assert.equal(W.panels(state.workspaces[1].root).length, 1);
assert.throws(() => W.close(state, W.panels(state.workspaces[1].root)[0].id));
assert.throws(() => W.setView(state, W.panels(state.workspaces[1].root)[0].id, '<script>'));
const invalid = structuredClone(state);
invalid.workspaces[0].root.ratio = 9;
assert.throws(() => W.validate(invalid));

const broken = structuredClone(state);
W.panels(broken.workspaces[0].root)[0].context.layer = 99;
assert.throws(() => W.validate(broken));

console.log(
  'Workspace split/join/swap, independent view instances, workspace persistence and validation passed.'
);

let dock = W.create();
dock = W.split(dock, W.panels(W.active(dock).root)[0].id, 'x');
dock = W.split(dock, W.panels(W.active(dock).root)[1].id, 'y', 'scene');
const dockPanels = W.panels(W.active(dock).root);
dock = W.move(dock, dockPanels[0].id, dockPanels[2].id, 'bottom');
assert.equal(W.panels(W.active(dock).root).length, 3);
W.validate(dock);
dock = W.close(dock, dockPanels[0].id);
assert.equal(W.panels(W.active(dock).root).length, 2);
