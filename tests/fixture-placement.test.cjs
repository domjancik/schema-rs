const test = require("node:test"),
  assert = require("node:assert/strict");
const P = require("../web/fixture-placement.js");
const ortho = (p) => ({
  x: 200 + p[0] * 40 + p[2] * 20,
  y: 200 - p[1] * 40 + p[2] * 10,
  w: 1,
});
test("axis move follows world coordinates and leaves the other layout coordinates intact", () => {
  const position = [1, 2, 3],
    before = [...position];
  const moved = P.move(position, [0], { x: 80, y: 0 }, ortho, 0);
  assert.deepEqual(moved, [3, 2, 3]);
  assert.deepEqual(position, before);
  assert.deepEqual(
    P.move(position, [1], { x: 0, y: -60 }, ortho, 0),
    [1, 3.5, 3],
  );
});
test("plane move keeps height and snaps only edited axes", () => {
  const position = [0.23, 1.123, 0.37];
  assert.deepEqual(
    P.move(position, [0, 2], { x: 60, y: 10 }, ortho, 0.5),
    [1, 1.123, 1.5],
  );
  const headOn = (p) => ({ x: p[0], y: p[1], w: 1 });
  assert.equal(P.move(position, [2], { x: 50, y: 0 }, headOn, 0), null);
  assert.equal(P.move(position, [0, 2], { x: 50, y: 0 }, headOn, 0), null);
});
test("perspective motion uses homogeneous depth rather than a constant pixel scale", () => {
  const perspective = (p) => ({
    x: 300 + (p[0] * 200) / (6 - p[2]),
    y: 200 - (p[1] * 200) / (6 - p[2]),
    w: 6 - p[2],
  });
  const p = [2, 1, 0],
    target = perspective([2, 1, 2]),
    start = perspective(p);
  assert.deepEqual(
    P.move(
      p,
      [2],
      { x: target.x - start.x, y: target.y - start.y },
      perspective,
      0,
    ),
    [2, 1, 2],
  );
  const q = [3, 1, 2],
    end = perspective(q);
  assert.deepEqual(
    P.move(
      p,
      [0, 2],
      { x: end.x - start.x, y: end.y - start.y },
      perspective,
      0,
    ),
    q,
  );
});
test("invalid and behind-camera placements cannot emit coordinates", () => {
  assert.throws(
    () => P.move([NaN, 0, 0], [0], { x: 1, y: 1 }, ortho, 0),
    /position/i,
  );
  assert.equal(P.move([0, 0, 0], [0], { x: Infinity, y: 0 }, ortho, 0), null);
  assert.throws(
    () => P.move([0, 0, 0], [0], { x: 1, y: 0 }, ortho, -1),
    /snap/i,
  );
  assert.equal(
    P.move([0, 0, 0], [0], { x: 1, y: 0 }, () => ({ x: 0, y: 0, w: -1 }), 0),
    null,
  );
});
