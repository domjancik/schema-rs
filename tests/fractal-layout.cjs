const assert = require('node:assert/strict'),
  L = require('../web/stack-layout.js');
const leaf = (id) => ({ id, children: [] });
const root = { id: 1, children: [{ id: 2, children: [leaf(4), leaf(5)] }, leaf(3)] };
for (const square of [false, true]) {
  const all = L.geometry(root, new Set(), { fractal: true, square }),
    get = (id) => all.find((r) => r.node.id === id),
    p = get(1),
    a = get(2),
    b = get(3),
    c = get(4),
    d = get(5);
  const near = (a, b) => assert(Math.abs(a - b) < 1e-8, `${a} != ${b}`);
  near(a.width, b.width);
  near(a.x, p.x);
  near(b.x * 190 + b.width, p.x * 190 + p.width);
  near(a.y + a.height, p.y);
  near(b.y + b.height, p.y);
  near(c.y + c.height, a.y);
  near(c.width, d.width);
  near(d.x * 190 + d.width, a.x * 190 + a.width);
  assert(c.scale < a.scale && a.scale < p.scale);
  if (square) for (const r of all) near(r.width, r.height);
  const ports = L.ports(p, all);
  assert.equal(ports.length, 3);
  assert(ports[0].x < ports[1].x && ports[1].x < ports[2].x);
  const studs = L.connections(p, all);
  near(studs[0].x, (a.x - p.x) * 190 + a.width / 2);
  near(studs[0].scale, a.scale);
  assert(!L.outline(p.width, p.height, studs, p.width / 2, p.scale).includes('NaN'));
}
const classic = L.geometry(root);
assert.equal(classic[0].width, 558);
assert.equal(classic[0].height, 112);
const squares = L.geometry(root, new Set(), { square: true });
assert(squares.every((r) => r.width === r.height));
const folded = L.geometry(root, new Set([2]), { fractal: true });
assert.equal(folded.length, 3);
assert.equal(folded.find((r) => r.node.id === 2).width, folded.find((r) => r.node.id === 3).width);
assert.deepEqual(L.geometry(null), []);
const farSeam = [{ id: 1, left: 0, right: 20, bottom: 10, scale: 0.1 }];
assert.equal(L.underside(farSeam, { x: 5, y: 15 }), null);
assert.equal(L.underside(farSeam, { x: 5, y: 11 }), 1);

const W = require('../web/workspace-core.js'),
  workspace = W.create(),
  panel = W.panels(W.active(workspace).root)[0];
assert.doesNotThrow(() => W.context(workspace, panel.id, { fractal: true, square: false }));
assert.throws(() => W.context(workspace, panel.id, { fractal: 'true' }));
assert.throws(() => W.context(workspace, panel.id, { square: 1 }));
console.log(
  'Fractal shared widths, recursive scaling, square aspect, aligned connectors/ports and folded geometry passed'
);

const tall = L.geometry(
  { id: 10, children: [{ id: 11, children: [{ id: 12, children: [leaf(13), leaf(14)] }] }] },
  new Set(),
  { square: true }
);
const fitted = L.fit(tall, { width: 713, height: 340 });
assert(fitted.scale < 0.25);
for (const r of tall) {
  assert(r.x * 190 * fitted.scale + fitted.pan.x >= 0);
  assert((r.x * 190 + r.width) * fitted.scale + fitted.pan.x <= 713);
  assert(r.y * fitted.scale + fitted.pan.y >= 0);
  assert((r.y + r.height) * fitted.scale + fitted.pan.y <= 340);
}
