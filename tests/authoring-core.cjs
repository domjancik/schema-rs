const assert = require("node:assert/strict");
const A = require("../web/authoring-core.js");
const points = [
  { id: "b", name: "Same B", x: 50, y: 50 },
  { id: "a", name: "Same A", x: 50, y: 50 },
  { id: "c", name: "Other", x: 95, y: 95 },
];
assert.deepEqual(
  A.hits(points, 50, 50, 20).map((p) => p.id),
  ["a", "b"],
);
const labels = A.labels(points, 200, 140, "a");
assert(labels.find((l) => l.id === "a")?.visible);
for (const a of labels.filter((l) => l.visible))
  for (const b of labels.filter((l) => l.visible && l.id > a.id))
    assert(
      a.x + a.width <= b.x ||
        b.x + b.width <= a.x ||
        a.y + 16 <= b.y ||
        b.y + 16 <= a.y,
    );
const p = {
  fixtures: [
    { id: "a", position: [0.1, 2, 0] },
    { id: "b", position: [1.7, 4, 1] },
    { id: "c", position: [8.1, 7, 2] },
  ],
};
assert.deepEqual(
  A.arrange(p, ["a", "b", "c"], { mode: "distribute", axis: 0 }).commands[0]
    .position,
  [4.1, 4, 1],
);
assert(
  A.arrange(p, ["a", "b"], {
    mode: "align",
    axis: 1,
    anchor: "active",
    active: "b",
  }).commands.every((c) => c.position[1] === 4),
);
assert.deepEqual(
  A.arrange(p, ["a"], { mode: "snap", step: 0.5 }).commands[0].position,
  [0, 2, 0],
);
assert.throws(() => A.arrange(p, ["a"], { mode: "snap", step: 0 }));
assert(
  A.search(
    [
      { label: "Release blackout", keywords: "lights output" },
      { label: "Save project", shortcut: "Ctrl+S" },
    ],
    "bl out",
  )[0].label === "Release blackout",
);
console.log(
  "Crowded picking, collision-free labels, arrangement commands and fuzzy action search passed.",
);
