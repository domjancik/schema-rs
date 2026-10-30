(function (root) {
  "use strict";
  function hits(points, x, y, r = 24) {
    return points
      .map((p) => ({ ...p, distance: Math.hypot(p.x - x, p.y - y) }))
      .filter((p) => p.distance <= r)
      .sort((a, b) => a.distance - b.distance || a.id.localeCompare(b.id));
  }
  function labels(points, width, height, selected) {
    const placed = [],
      result = [];
    const overlap = (a, b) =>
      a.x < b.x + b.width + 5 &&
      a.x + a.width + 5 > b.x &&
      a.y < b.y + 20 &&
      a.y + 20 > b.y;
    for (const p of [...points].sort(
      (a, b) =>
        (b.id === selected) - (a.id === selected) || a.id.localeCompare(b.id),
    )) {
      const w = Math.min(
        180,
        Math.max(24, p.labelWidth ?? p.name.length * 6.5),
      );
      let label = {
        id: p.id,
        text: p.name,
        width: w,
        visible: false,
        x: 0,
        y: 0,
      };
      for (const [dx, dy] of [
        [12, -24],
        [12, 8],
        [-w - 12, -24],
        [-w - 12, 8],
        [12, -44],
        [12, 28],
        [-w / 2, -64],
        [-w / 2, 48],
      ]) {
        const box = {
          x: Math.max(3, Math.min(width - w - 3, p.x + dx)),
          y: Math.max(3, Math.min(height - 19, p.y + dy)),
          width: w,
        };
        if (
          !placed.some((q) => overlap(box, q)) &&
          !points.some(
            (q) =>
              q.x > box.x - 5 &&
              q.x < box.x + w + 5 &&
              q.y > box.y - 4 &&
              q.y < box.y + 20,
          )
        ) {
          label = { ...label, ...box, visible: true };
          placed.push(box);
          break;
        }
      }
      result.push(label);
    }
    return result;
  }
  function arrange(project, ids, options) {
    const selected = new Set(ids),
      fixtures = project.fixtures.filter((f) => selected.has(f.id)),
      axis = Number(options.axis ?? 0);
    if (![0, 1, 2].includes(axis) || !fixtures.length)
      throw Error("Select fixtures and a valid axis");
    const coordinates = new Map(fixtures.map((f) => [f.id, [...f.position]]));
    if (options.mode === "snap") {
      const step = Number(options.step);
      if (!Number.isFinite(step) || step <= 0 || step > 10000)
        throw Error("Snap spacing must be positive");
      for (const p of coordinates.values())
        for (let i = 0; i < 3; i++) p[i] = Math.round(p[i] / step) * step;
    } else if (options.mode === "align") {
      if (fixtures.length < 2)
        throw Error("Select at least two fixtures to align");
      const values = fixtures.map((f) => f.position[axis]),
        min = Math.min(...values),
        max = Math.max(...values);
      const value =
        options.anchor === "min"
          ? min
          : options.anchor === "max"
            ? max
            : options.anchor === "center"
              ? (min + max) / 2
              : (fixtures.find((f) => f.id === options.active) || fixtures[0])
                  .position[axis];
      for (const p of coordinates.values()) p[axis] = value;
    } else if (options.mode === "distribute") {
      if (fixtures.length < 3)
        throw Error("Select at least three fixtures to distribute");
      const sorted = [...fixtures].sort(
          (a, b) =>
            a.position[axis] - b.position[axis] || a.id.localeCompare(b.id),
        ),
        min = sorted[0].position[axis],
        max = sorted.at(-1).position[axis];
      sorted.forEach(
        (f, i) =>
          (coordinates.get(f.id)[axis] =
            min + ((max - min) * i) / (sorted.length - 1)),
      );
    } else throw Error("Unknown arrangement");
    const commands = fixtures
      .map((f) => ({
        type: "move_fixture",
        fixture: f.id,
        position: coordinates.get(f.id).map((v) => Math.round(v * 1e6) / 1e6),
      }))
      .filter((c) =>
        c.position.some(
          (v, i) =>
            v !== project.fixtures.find((f) => f.id === c.fixture).position[i],
        ),
      );
    return { type: "batch", commands };
  }
  function score(text, query) {
    text = text.toLowerCase();
    let at = -1,
      total = 0;
    for (const char of query.toLowerCase().replace(/\s/g, "")) {
      const next = text.indexOf(char, at + 1);
      if (next < 0) return Infinity;
      total += next - at - 1;
      at = next;
    }
    return total;
  }
  function search(actions, query) {
    return actions
      .map((a, i) => ({
        a,
        i,
        score: score(
          a.label + " " + (a.keywords || "") + " " + (a.shortcut || ""),
          query,
        ),
      }))
      .filter((x) => Number.isFinite(x.score))
      .sort((a, b) => a.score - b.score || a.i - b.i)
      .map((x) => x.a);
  }
  const markerColor = fixture => /^#[0-9a-f]{6}$/i.test(fixture?.marker_color || "") ? fixture.marker_color : null;
  const api = { hits, labels, arrange, search, markerColor };
  root.SchemaAuthoringCore = api;
  if (typeof module !== "undefined") module.exports = api;
})(globalThis);
