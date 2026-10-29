const add = (a, b) => a.map((v, i) => v + b[i]),
  sub = (a, b) => a.map((v, i) => v - b[i]),
  scale = (a, n) => a.map((v) => v * n),
  length = (a) => Math.hypot(...a),
  unit = (a) => {
    const n = length(a);
    return n > 1e-8 ? scale(a, 1 / n) : [0, 0, 1];
  },
  cross = (a, b) => [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ],
  clamp = (v, a, b) => Math.max(a, Math.min(b, v));
export function validateCamera(pose) {
  if (
    !pose ||
    !["position", "target", "up"].every(
      (key) =>
        Array.isArray(pose[key]) &&
        pose[key].length === 3 &&
        pose[key].every((v) => Number.isFinite(v) && Math.abs(v) < 100000),
    ) ||
    (pose.projection !== undefined &&
      !["perspective", "orthographic"].includes(pose.projection)) ||
    (pose.span !== undefined &&
      (!Number.isFinite(pose.span) || pose.span < 1e-5 || pose.span > 1e7)) ||
    length(pose.up) < 0.1 ||
    length(sub(pose.position, pose.target)) < 0.001
  )
    throw Error("Invalid camera state");
  return pose;
}
export function navigateCamera(pose, { orbit, pan, zoom } = {}) {
  validateCamera(pose);
  let target = [...pose.target],
    offset = sub(pose.position, pose.target),
    distance = clamp(length(offset), 0.2, 150),
    up = [...pose.up];
  if (orbit) {
    if (!orbit.every(Number.isFinite)) throw Error("Invalid orbit");
    const yaw = Math.atan2(offset[0], offset[2]) + orbit[0],
      pitch = clamp(
        Math.asin(clamp(offset[1] / length(offset), -1, 1)) + orbit[1],
        -Math.PI / 2 + 0.005,
        Math.PI / 2 - 0.005,
      );
    offset = [
      Math.sin(yaw) * Math.cos(pitch) * distance,
      Math.sin(pitch) * distance,
      Math.cos(yaw) * Math.cos(pitch) * distance,
    ];
    up = [0, 1, 0];
  }
  if (pan) {
    if (!pan.every(Number.isFinite)) throw Error("Invalid pan");
    const panDistance =
      pose.projection === "orthographic" && pose.span
        ? pose.span / (2 * Math.tan((24 * Math.PI) / 180))
        : distance;
    const forward = unit(scale(offset, -1)),
      right = unit(cross(forward, up)),
      vertical = unit(cross(right, forward));
    target = add(
      target,
      add(
        scale(right, pan[0] * panDistance * 0.08),
        scale(vertical, pan[1] * panDistance * 0.08),
      ),
    );
  }
  if (zoom !== undefined) {
    if (!Number.isFinite(zoom) || zoom <= 0) throw Error("Invalid zoom");
    distance = clamp(distance * zoom, 0.2, 150);
    offset = scale(unit(offset), distance);
  }
  return validateCamera({
    position: add(target, offset),
    target,
    up,
    ...(pose.projection ? { projection: pose.projection } : {}),
    ...(pose.span !== undefined
      ? { span: clamp(pose.span * (zoom ?? 1), 1e-5, 1e7) }
      : {}),
  });
}
export function cameraPreset(pose, name) {
  validateCamera(pose);
  const directions = {
      front: [0, 0, -1],
      back: [0, 0, 1],
      right: [1, 0, 0],
      left: [-1, 0, 0],
      top: [0, 1, 0.000001],
      iso: [1, 1, 1],
    },
    direction = directions[name];
  if (!direction) throw Error("Unknown camera preset");
  const distance = clamp(length(sub(pose.position, pose.target)), 0.2, 150);
  return {
    position: add(pose.target, scale(unit(direction), distance)),
    target: [...pose.target],
    up: [0, 1, 0],
    ...(name === "iso"
      ? {
          projection: "orthographic",
          span: pose.span ?? 2 * Math.tan((24 * Math.PI) / 180) * distance,
        }
      : pose.projection
        ? {
            projection: pose.projection,
            ...(pose.span ? { span: pose.span } : {}),
          }
        : {}),
  };
}
