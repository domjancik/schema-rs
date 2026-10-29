const assert = require('node:assert/strict'),
  S = require('../web/scene-stacks.js');
const project = {
  fixtures: [
    { id: 'a', position: [1, 2, 3] },
    { id: 'b', position: [4, 5, 6] }
  ],
  content: { a: [{ root: { id: 1, children: [] } }], b: [null] }
};
assert.deepEqual(S.fixtures(project, 'off', new Set(['a'])), []);
assert.deepEqual(
  S.fixtures(project, 'selected', new Set(['b'])).map((f) => f.id),
  ['b']
);
assert.equal(S.fixtures(project, 'all', new Set()).length, 2);
const rows = [{ x: 0, y: 0, width: 178, height: 112, scale: 1 }];
const p = S.pose(rows, { x: 200, y: 300 }, 0.5);
assert.equal(p.x + (178 * 0.5) / 2, 200);
assert.equal(p.y + 112 * 0.5, 282);
assert.equal(S.count({ children: [{ children: [] }] }), 2);
console.log('Scene-stack scope, anchored layout position and node budget count passed');

const W=require('../web/workspace-core.js'),w=W.create(),id=W.panels(W.active(w).root)[0].id;assert.doesNotThrow(()=>W.context(w,id,{sceneStacks:'all',sceneStackScale:.35}));assert.throws(()=>W.context(w,id,{sceneStacks:'wrong'}));assert.throws(()=>W.context(w,id,{sceneStackScale:5}));
