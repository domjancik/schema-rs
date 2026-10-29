const assert = require('node:assert/strict');
const M = require('../web/mobile-skin-core.js');
assert.deepEqual(M.sceneLight({ fixtures: [] }).rgb, [114, 222, 194]);
const red = M.sceneLight({
  fixtures: [
    {
      result: {
        colors: [
          [1, 0, 0, 1],
          [0, 0, 1, 1]
        ]
      }
    },
    { result: { colors: [[1, 0, 0, 1]] } }
  ]
});
assert(red.rgb[0] > red.rgb[2] && red.rgb[1] === 0);
assert.equal(
  M.sceneLight({ blackout: true, fixtures: [{ result: { colors: [[1, 1, 1, 1]] } }] }).energy,
  0
);
const pixels = new Uint8ClampedArray(4 * 4 * 4);
for (let y = 0; y < 4; y++)
  for (let x = 0; x < 4; x++) {
    const i = (y * 4 + x) * 4;
    pixels[i] = x < 2 ? 255 : 0;
    pixels[i + 3] = 255;
  }
const camera = M.cameraLight(pixels, 4, 4);
assert.deepEqual(camera.rgb, [128, 0, 0]);
assert(camera.x < 0);
assert(M.surface.length >= 8, 'Broad triangular facets cover the shell');
const area = M.surface.reduce((total, face) => {
  const [a,b,c] = face.points;
  assert.equal(face.points.length, 3);
  assert(Math.abs(Math.hypot(...face.normal)-1) < 1e-6);
  assert(face.normal[2] > 0.8, 'Shallow relief, no steep bevels');
  return total + Math.abs((b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]))/2;
}, 0);
assert(Math.abs(area-1) < 1e-6, 'The surface covers the full housing');
const positive = M.surface.reduce((a,b) => a.normal[0] > b.normal[0] ? a : b);
const negative = M.surface.reduce((a,b) => a.normal[0] < b.normal[0] ? a : b);
assert(M.facets({x:1,y:0})[positive.id] > M.facets({x:-1,y:0})[positive.id]);
assert(M.facets({x:1,y:0})[negative.id] < M.facets({x:-1,y:0})[negative.id]);
assert.deepEqual(M.tilt({ beta: 80, gamma: 15 }, 0, { beta: 80, gamma: 15 }), {x:0,y:0});
assert.equal(M.tilt({ beta: 95, gamma: 15 }, 0, { beta: 80, gamma: 15 }).y, .25);
assert.deepEqual(M.tilt({ beta: NaN, gamma: null }, 0), { x: 0, y: 0 });
assert.deepEqual(M.tilt({ beta: 0, gamma: 30 }, 90), { x: 0, y: -0.5 });
for (const v of Object.values(M.facets({ x: 1e10, y: NaN })))
  assert(Number.isFinite(v) && v >= 0 && v <= 1);

assert.deepEqual(
  M.sceneLight({ fixtures: [{ fixture: { dimmer: 0.5 }, result: { colors: [[1, 0, 0, 0.5]] } }] })
    .rgb,
  [64, 0, 0]
);
assert.deepEqual(
  M.sceneLight({ fixtures: [{ fixture: { dimmer: 1 }, result: { value: 0.5 } }] }).rgb,
  [128, 128, 128]
);

console.log('mobile skin lighting math passed');
