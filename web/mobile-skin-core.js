(function (global) {
  'use strict';
  const clamp = (v, lo = 0, hi = 1) => Math.max(lo, Math.min(hi, Number.isFinite(v) ? v : 0));
  function sceneLight(snapshot = {}) {
    const sum = [0, 0, 0];
    let count = 0;
    for (const f of snapshot.fixtures || []) {
      const value = clamp(f.result?.value);
      const colors = f.result?.colors?.length ? f.result.colors : [[value, value, value, 1]];
      // Strided sampling bounds work for long pixel strips without making a frame-sized copy.
      const stride = Math.max(1, Math.ceil(colors.length / 64));
      for (let pixel = 0; pixel < colors.length; pixel += stride) {
        const c = colors[pixel],
          gain = clamp(f.fixture?.dimmer ?? 1) * clamp(c[3] ?? 1);
        for (let i = 0; i < 3; i++) sum[i] += clamp(c[i]) * gain;
        count++;
      }
    }
    if (!count) return { rgb: [114, 222, 194], energy: snapshot.blackout ? 0 : 0.25 };
    const average = sum.map((v) => v / count);
    return {
      rgb: average.map((v) => Math.round(v * 255)),
      energy: snapshot.blackout ? 0 : Math.max(...average)
    };
  }
  function cameraLight(pixels, width, height) {
    let weight = 0,
      x = 0,
      y = 0;
    const sum = [0, 0, 0];
    for (let row = 0; row < height; row++)
      for (let col = 0; col < width; col++) {
        const i = (row * width + col) * 4;
        const luma = (pixels[i] * 0.2126 + pixels[i + 1] * 0.7152 + pixels[i + 2] * 0.0722) / 255;
        for (let channel = 0; channel < 3; channel++) sum[channel] += pixels[i + channel];
        weight += luma;
        x += luma * (((col + 0.5) / width) * 2 - 1);
        y += luma * (((row + 0.5) / height) * 2 - 1);
      }
    return {
      rgb: sum.map((v) => Math.round(v / Math.max(1, width * height))),
      x: weight ? x / weight : 0,
      y: weight ? y / weight : 0,
      energy: weight / Math.max(1, width * height)
    };
  }
  function tilt(event, angle = 0, reference = { beta: 0, gamma: 0 }) {
    const x = clamp((event.gamma - reference.gamma) / 60, -1, 1),
      y = clamp((event.beta - reference.beta) / 60, -1, 1),
      a = (angle * Math.PI) / 180;
    return {
      x: Math.round((x * Math.cos(a) + y * Math.sin(a)) * 1e6) / 1e6,
      y: Math.round((y * Math.cos(a) - x * Math.sin(a)) * 1e6) / 1e6
    };
  }
  // A shallow, uneven ridge: every triangle shares vertices with its neighbours.
  const vertices = [[0,0,0],[.55,0,.012],[1,0,0],[1,.55,.008],
    [1,1,0],[.42,1,.012],[0,1,0],[0,.48,.005],[.43,.42,.12],[.61,.77,.10]];
  const triangles = [[0,1,8],[1,2,8],[2,3,8],[3,9,8],[3,4,9],
    [4,5,9],[5,6,9],[6,7,9],[7,8,9],[7,0,8]];
  const surface = triangles.map((indices, index) => {
    const points = indices.map(i => vertices[i]);
    const [a,b,c] = points;
    const u = [b[0]-a[0], (b[1]-a[1])*1.65, b[2]-a[2]],
      v = [c[0]-a[0], (c[1]-a[1])*1.65, c[2]-a[2]];
    const cross = [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]],
      length = Math.hypot(...cross);
    return { id: 'face-' + index, points, normal: cross.map(n => n / length) };
  });
  function facets(direction) {
    const x = clamp(direction.x, -2, 2), y = clamp(direction.y, -2, 2), z = .75,
      length = Math.hypot(x,y,z), light = [x/length,y/length,z/length];
    return Object.fromEntries(surface.map(face => {
      const diffuse = Math.max(0, face.normal.reduce((sum,n,i) => sum+n*light[i], 0));
      return [face.id, clamp(.5 + (diffuse-light[2])*1.65, .08, .92)];
    }));
  }
  const api = { sceneLight, cameraLight, tilt, facets, surface };
  global.SchemaSkinCore = api;
  if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
