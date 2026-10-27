import assert from "node:assert/strict";
import { frameToScene, rotateTurns } from "../web/scene-preview-core.mjs";
const fixture = {
  id: "uv",
  name: "UV",
  model: "dimmer",
  dmx_model: "uv",
  universe: 0,
  address: 1,
  position: [1, 2, 3],
  rotation: [0, 0, 0],
  pixel_positions: [[0, 0, 0]],
  dimmer: 0.5,
  groups: [],
};
const frame = {
  project: {
    fixtures: [fixture],
    dmx_models: {
      uv: { slots: [{ source: "value" }, { source: "constant", constant: 0 }] },
    },
  },
  snapshot: {
    blackout: false,
    fixtures: [{ fixture, result: { value: 1, position: [4, 5, 6] } }],
    universes: { 0: [64, 0] },
    emitters: [],
  },
};
const before = JSON.stringify(frame),
  scene = frameToScene(frame, {
    uv: { tint: [0.4, 0.1, 1], kind: "spot", direction: [0, 0, -1] },
  });
assert.deepEqual(scene.fixtures[0].layoutPosition, [1, 2, 3]);
assert.deepEqual(scene.fixtures[0].position, [4, 5, 6]);
assert.equal(scene.fixtures[0].level, 64 / 255);
assert.equal(scene.fixtures[0].kind, "spot");
assert.deepEqual(scene.fixtures[0].color, [
  (0.4 * 64) / 255,
  (0.1 * 64) / 255,
  64 / 255,
]);
assert.equal(JSON.stringify(frame), before, "Publication stays immutable");
const dark = frameToScene({
  ...frame,
  snapshot: { ...frame.snapshot, blackout: true },
});
assert.equal(dark.fixtures[0].level, 0);
const rgb = {
  ...fixture,
  dmx_model: null,
  model: "rgbw",
  address: 2,
  pixel_positions: [
    [0, 0, 0],
    [1, 0, 0],
  ],
  rotation: [0, 0.25, 0],
  pixel_reverse: true,
};
const result = frameToScene({
  project: { fixtures: [rgb] },
  snapshot: {
    fixtures: [],
    universes: { 0: [0, 128, 0, 0, 64, 0, 128, 0, 64] },
  },
});
assert.equal(result.fixtures[0].pixels.length, 2);
assert.deepEqual(result.fixtures[0].pixels[0].color, [
  192 / 255,
  64 / 255,
  64 / 255,
]);
assert(Math.abs(rotateTurns([1, 0, 0], [0, 0.25, 0])[2] + 1) < 1e-8);
const emit = frameToScene({
  ...frame,
  snapshot: {
    ...frame.snapshot,
    emitters: [
      {
        fixture: "uv",
        node: 3,
        layer: 0,
        kind: 2,
        position: [0, 1, 0],
        direction: [0, -1, 0],
        color: [1, 0.5, 0, 1],
        intensity: 0.7,
        range: 8,
        angle: 40,
        width: 2,
        height: 1,
      },
    ],
  },
});
assert.equal(emit.emitters[0].kind, "spot");
assert.equal(emit.emitters[0].range, 8);
assert.equal(
  frameToScene({ project: { fixtures: [fixture] }, snapshot: {} }).fixtures[0]
    .level,
  0,
  "Missing output never invents live brightness",
);
console.log(
  "Preview adapter: captured DMX, RGBW/pixels, rotation, blackout, emitters and immutability passed.",
);

const turned = frameToScene(
  {
    ...frame,
    snapshot: {
      ...frame.snapshot,
      fixtures: [{ fixture, result: { rotation: [0, 0.25, 0] } }],
    },
  },
  { uv: { kind: "spot", direction: [0, 0, -1] } },
);
assert(
  Math.abs(turned.fixtures[0].direction[0] + 1) < 1e-8,
  "Published rotations steer fixture spot directions",
);
