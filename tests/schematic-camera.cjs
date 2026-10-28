const assert = require('node:assert/strict'),
  N = require('../web/schematic-camera.js');
const points = [
    [-150, 2, -30],
    [50, 4, 70],
    [0, 80, -80]
  ],
  camera = N.fit(points, 360, 180);
for (const p of points) {
  const q = N.project(p, camera, 360, 180);
  assert(q.x >= 20 && q.x <= 340 && q.y >= 20 && q.y <= 160);
}
const center = N.project(camera.target, camera, 360, 180);
assert.deepEqual([center.x, center.y], [180, 90]);
const orbit = N.navigate(camera, { orbit: [30, 15] });
assert.notDeepEqual(N.project(points[0], orbit, 360, 180), N.project(points[0], camera, 360, 180));
const pan = N.navigate(camera, { pan: [40, 20] });
const before = N.project(points[0], camera, 360, 180),
  after = N.project(points[0], pan, 360, 180);
assert(Math.abs(after.x - before.x - 40) < 1e-8 && Math.abs(after.y - before.y - 20) < 1e-8);
assert.equal(N.navigate(camera, { zoom: 2 }).zoom, camera.zoom * 2);
assert.throws(() => N.valid({ ...camera, zoom: NaN }));
assert.throws(() => N.valid({ ...camera, target: [Infinity, 0, 0] }));
assert.equal(N.preset(camera, 'front').pitch, 0);
assert(N.preset(camera, 'top').pitch > 1.5);
const W = require('../web/workspace-core.js'),
  s = W.create(),
  id = W.panels(W.active(s).root)[0].id;
assert.throws(() => W.context(s, id, { sceneCamera: { zoom: NaN } }));
console.log('schematic camera fit, navigation, bounds and workspace validation passed');

const tiny = N.fit(
  [
    [-3, 0, -1],
    [3, 0, 2]
  ],
  300,
  32
);
assert(
  Math.abs(N.project([-3, 0, -1], tiny, 300, 32).y - N.project([3, 0, 2], tiny, 300, 32).y) > 12,
  'Small viewports should use adaptive padding'
);
