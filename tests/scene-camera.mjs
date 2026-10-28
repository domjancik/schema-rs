import assert from "node:assert/strict";
import {
  navigateCamera,
  cameraPreset,
  validateCamera,
} from "../web/scene-camera.mjs";
const pose = { position: [4, 3, 6], target: [0, 1, 0], up: [0, 1, 0] };
const orbit = navigateCamera(pose, { orbit: [0.2, -0.1] });
assert.notDeepEqual(orbit.position, pose.position);
assert.deepEqual(orbit.target, pose.target);
const distance = (p) =>
  Math.hypot(...p.position.map((v, i) => v - p.target[i]));
assert.ok(Math.abs(distance(orbit) - distance(pose)) < 1e-10);
const pan = navigateCamera(pose, { pan: [1, 0.5] });
assert.notDeepEqual(pan.target, pose.target);
assert.ok(Math.abs(distance(pan) - distance(pose)) < 1e-10);
assert.deepEqual(pose.position, [4, 3, 6]);
assert.ok(distance(navigateCamera(pose, { zoom: 0.5 })) < distance(pose));
assert.ok(distance(navigateCamera(pose, { zoom: 1e10 })) <= 150.00001);
const top = cameraPreset(pose, "top");
assert.deepEqual(top.target, pose.target);
assert.ok(top.position[1] > top.target[1]);
assert.deepEqual(top.up, [0, 1, 0]);
assert.throws(() => validateCamera({ ...pose, position: [NaN, 0, 0] }));
assert.throws(() => cameraPreset(pose, "unsafe"));
console.log(
  "Camera orbit/pan/zoom, clipping bounds, presets and immutable navigation passed.",
);
