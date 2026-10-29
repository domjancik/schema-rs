import assert from 'node:assert/strict';
import N from '../web/schematic-camera.js';
import { cameraPreset, navigateCamera, validateCamera } from '../web/scene-camera.mjs';
const c = N.fit(
    [
      [0, 0, 0],
      [2, 3, 4]
    ],
    400,
    300
  ),
  iso = N.preset(c, 'iso'),
  origin = N.project([0, 0, 0], iso, 400, 300);
const lengths = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1]
].map((p) => {
  const q = N.project(p, iso, 400, 300);
  return Math.hypot(q.x - origin.x, q.y - origin.y);
});
assert(
  Math.max(...lengths) - Math.min(...lengths) < 1e-9,
  'Isometric axes must have identical foreshortening'
);
const rotated = N.rotate(iso, 1);
assert.equal(rotated.pitch, iso.pitch);
assert.deepEqual(rotated.target, iso.target);
assert.equal(rotated.zoom, iso.zoom);
assert(Math.abs(rotated.yaw - iso.yaw - Math.PI / 4) < 1e-9);
const pose = { position: [4, 3, 6], target: [0, 1, 0], up: [0, 1, 0] },
  gpu = cameraPreset(pose, 'iso'),
  offset = gpu.position.map((v, i) => Math.abs(v - gpu.target[i]));
assert(Math.max(...offset) - Math.min(...offset) < 1e-9);
assert.equal(gpu.projection, 'orthographic');
assert(gpu.span > 0);
const zoom = navigateCamera(gpu, { zoom: 0.5 });
assert.equal(zoom.projection, 'orthographic');
assert.equal(zoom.span, gpu.span * 0.5);
assert.deepEqual(zoom.target, gpu.target);
assert.throws(() => validateCamera({ ...gpu, projection: 'invalid' }));
assert.throws(() => validateCamera({ ...gpu, span: NaN }));
console.log(
  'Exact isometric axis scales,45° rotation, GPU orthographic presets and span-preserving navigation passed'
);
