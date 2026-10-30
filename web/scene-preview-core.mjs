// Presentation adapter only: output comes from published capture buffers, never graph evaluation.
export const finite = (value, fallback = 0) =>
  Number.isFinite(value) ? value : fallback;
const clamp = (value) => Math.max(0, Math.min(1, finite(value)));
const vector = (value, fallback = [0, 0, 0]) =>
  Array.isArray(value) && value.length >= 3
    ? value.slice(0, 3).map((v, i) => finite(v, fallback[i]))
    : [...fallback];
export function rotateTurns(point, rotation = [0, 0, 0]) {
  const result = vector(point);
  for (let axis = 0; axis < 3; axis++) {
    const [a, b] = [
        [1, 2],
        [2, 0],
        [0, 1],
      ][axis],
      angle = finite(rotation[axis]) * Math.PI * 2,
      x = result[a],
      y = result[b];
    result[a] = x * Math.cos(angle) - y * Math.sin(angle);
    result[b] = x * Math.sin(angle) + y * Math.cos(angle);
  }
  return result;
}
function capturedColor(project, snapshot, fixture, index) {
  if (snapshot.blackout) return [0, 0, 0];
  const slots =
    project.dmx_models?.[fixture.dmx_model]?.slots ||
    {
      dimmer: [{ source: "value" }],
      rgb: [{ source: "red" }, { source: "green" }, { source: "blue" }],
      rgbw: [
        { source: "red" },
        { source: "green" },
        { source: "blue" },
        { source: "white" },
      ],
    }[fixture.model] ||
    [];
  const capture = snapshot.universes?.[fixture.universe];
  if (!capture) return [0, 0, 0];
  const values = {};
  for (let i = 0; i < slots.length; i++) {
    const source = slots[i].source;
    if (
      !slots[i].fine &&
      ["value", "red", "green", "blue", "white"].includes(source) &&
      values[source] === undefined
    )
      values[source] = clamp(
        capture[fixture.address - 1 + index * slots.length + i] / 255,
      );
  }
  if (
    ["red", "green", "blue", "white"].some((key) => values[key] !== undefined)
  ) {
    const w = values.white || 0;
    return [
      (values.red || 0) + w,
      (values.green || 0) + w,
      (values.blue || 0) + w,
    ].map(clamp);
  }
  return Array(3).fill(values.value || 0);
}
export function frameToScene(frame, overrides = {}) {
  const project = frame?.project || {},
    snapshot = frame?.snapshot || {},
    published = new Map(
      (snapshot.fixtures || []).map((f) => [f.fixture.id, f.result || {}]),
    );
  const fixtures = (project.fixtures || []).slice(0, 2048).map((fixture) => {
    const result = published.get(fixture.id) || {},
      style = overrides[fixture.id] || {},
      position = vector(result.position, vector(fixture.position)),
      rotation = vector(result.rotation, vector(fixture.rotation)),
      layoutPosition = vector(fixture.position),
      tint = vector(style.tint, [1, 1, 1]).map(clamp);
    const positions = (
      result.pixel_positions ||
      fixture.pixel_positions || [[0, 0, 0]]
    ).slice(0, 512);
    if (!positions.length) positions.push([0, 0, 0]);
    const pixels = positions.map((offset, index) => {
      const rotated = rotateTurns(offset, rotation),
        color = capturedColor(project, snapshot, fixture, index).map(
          (value, i) => value * tint[i],
        );
      return { position: position.map((v, i) => v + rotated[i]), color };
    });
    const color = [0, 1, 2].map(
      (i) => pixels.reduce((sum, p) => sum + p.color[i], 0) / pixels.length,
    );
    return {
      id: fixture.id,
      name: fixture.name || fixture.id,
      markerColor: /^#[0-9a-f]{6}$/i.test(fixture.marker_color || "") ? fixture.marker_color : null,
      layoutPosition,
      position,
      rotation,
      pixels,
      color,
      level: Math.max(...color),
      kind: style.kind || "point",
      direction: rotateTurns(vector(style.direction, [0, -1, 0]), rotation),
      range: Math.max(0.1, finite(style.range, 8)),
      angle: Math.max(1, Math.min(175, finite(style.angle, 60))),
      width: Math.max(0.02, finite(style.width, 1)),
      height: Math.max(0.02, finite(style.height, 1)),
      groups: [...(fixture.groups || [])],
    };
  });
  const emitters = (snapshot.emitters || [])
    .slice(0, 256)
    .map((emitter, index) => ({
      id: `${emitter.fixture}/${emitter.layer}/${emitter.node}/${index}`,
      fixture: emitter.fixture,
      kind: ["point", "plane", "spot"][emitter.kind] || "point",
      position: vector(emitter.position),
      direction: vector(emitter.direction, [0, -1, 0]),
      color: vector(emitter.color, [1, 1, 1]).map(
        (value) =>
          Math.max(0, value) *
          Math.max(0, finite(emitter.intensity, 1)) *
          clamp(emitter.color?.[3] ?? 1) *
          (snapshot.blackout ? 0 : 1),
      ),
      range: Math.max(0.1, finite(emitter.range, 8)),
      angle: Math.max(1, Math.min(175, finite(emitter.angle, 60))),
      width: Math.max(0.02, finite(emitter.width, 1)),
      height: Math.max(0.02, finite(emitter.height, 1)),
    }));
  return {
    fixtures,
    emitters,
    frame: snapshot.frame ?? 0,
    revision: snapshot.revision ?? 0,
    paused: !!snapshot.paused,
    blackout: !!snapshot.blackout,
    name: project.name || "Live scene",
  };
}
