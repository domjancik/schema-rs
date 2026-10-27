import { SceneView } from "./scene-preview-renderer.mjs";

import { frameToScene } from "./scene-preview-core.mjs";

let current;

const assets = "/scene-assets/";

const fetchJSON = async (url) => {
  const response = await fetch(url);
  if (!response.ok) throw Error("Cannot load " + url);
  return response.json();
};

async function preset() {
  const [project, reference, lights] = await Promise.all(
    [
      "carpushit.project.json",
      "carpushit-reference.json",
      "carpushit-lights.json",
    ].map((name) => fetchJSON(assets + name)),
  );
  const universes = {};
  for (const f of reference.fixtures) {
    const universe =
      universes[f.universe] || (universes[f.universe] = Array(512).fill(0));
    universe[f.address_1_based - 1] = f.captured_byte;
  }
  const overrides = {};
  for (const source of lights.sources) {
    const hex = source.color_srgb.slice(1);
    overrides[source.fixture_id] = {
      kind: source.kind === "line" ? "plane" : source.kind,
      tint: [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255),
      direction: source.direction || [0, -1, 0],
      range: source.range_m,
      angle: source.outer_angle_degrees,
      width: source.line_points_m
        ? Math.max(
            0.04,
            Math.max(...source.line_points_m.map((p) => p[0])) -
              Math.min(...source.line_points_m.map((p) => p[0])),
          )
        : source.width_m || 1,
      height: source.line_points_m
        ? Math.max(
            0.04,
            Math.max(...source.line_points_m.map((p) => p[2])) -
              Math.min(...source.line_points_m.map((p) => p[2])),
          )
        : source.height_m || 1,
    };
  }
  return {
    project,
    overrides,
    frame: {
      project,
      snapshot: {
        fixtures: [],
        universes,
        emitters: [],
        frame: 0,
        blackout: false,
      },
    },
  };
}

export async function open(adapter = {}) {
  if (current?.failed) {
    clearInterval(current.timer);
    current.view?.dispose();
    current.dialog.remove();
    current = null;
  }
  if (current) {
    current.dialog.showModal();
    if (current.view) current.view.start();
    else await current.initialize();
    return;
  }

  const dialog = document.createElement("dialog");
  dialog.id = "scene-preview-dialog";
  dialog.setAttribute("aria-label", "GPU scene preview");
  dialog.innerHTML = `<div class="preview-heading"><div><span class="preview-kicker">SCHEMA / SCENE VIEW</span><h2>Light in the room.</h2></div><button id="preview-close" aria-label="Close scene preview">×</button></div><div class="preview-controls"><label>Scene<select id="preview-source"><option value="live">Live project</option><option value="carpushit">Carpushit · reference preset</option></select></label><label>Graphics<select id="preview-backend"><option value="auto">Auto · WebGPU / WebGL2</option><option value="webgl">WebGL2</option></select></label><label>Quality<select id="preview-quality"><option value="low">Low · 12 lights</option><option value="balanced" selected>Balanced · 40 lights</option><option value="high">High · 64 lights</option></select></label><button id="preview-fit">Fit scene</button><label class="preview-file">Import scenery<input id="preview-model" type="file" accept=".glb,model/gltf-binary"></label><button id="preview-reset-model">Clear scenery</button><button id="preview-load-preset" hidden>Use Carpushit in project…</button></div><div class="preview-settings"><label><input id="preview-gi" type="checkbox" checked>SSGI</label><label><input id="preview-volumes" type="checkbox" checked>Volumetrics</label><label><input id="preview-layout" type="checkbox" checked>Layout handles</label><label><input id="preview-emitters" type="checkbox" checked>Light emitters</label><label>Haze<input id="preview-haze" type="range" min="0" max="4" step=".05" value=".18"></label><label>Exposure<input id="preview-exposure" type="range" min=".2" max="4" step=".1" value="1.4"></label><label>Light gain<input id="preview-gain" type="range" min="1" max="100" step="1" value="12"></label><label>Room fill<input id="preview-ambient" type="range" min="0" max="1" step=".01" value=".13"></label></div><div id="preview-review" hidden><p>Replace the current shared scene with Carpushit’s lighting layout and full look? Output will be disabled. This is one undoable project change. The reference view alone changes nothing.</p><button id="preview-confirm-load">Replace current scene</button><button id="preview-cancel-load">Keep current scene</button></div><div class="preview-viewport"><canvas id="preview-canvas" aria-label="Interactive lighting preview"></canvas><p id="preview-loading" role="status">Starting graphics…</p><span id="preview-selection"></span></div><div class="preview-foot"><span id="preview-stats" role="status"></span><span id="preview-mode">Live captured output · orbit / pinch / pan</span></div><p class="preview-note">Visual approximation · screen-space GI only sees visible surfaces. Haze, fixture power and UV tint are preview settings; fluorescence and physical optical transport are not modeled.</p>`;

  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = "/scene-preview.css";
  document.head.append(link);
  document.body.append(dialog);
  dialog.showModal();

  if (matchMedia("(max-width:700px), (pointer:coarse)").matches)
    dialog.querySelector("#preview-quality").value = "low";
  const params = new URLSearchParams(location.search);
  for (const key of ["backend", "quality", "source"]) {
    const input = dialog.querySelector("#preview-" + key),
      value = params.get(key === "source" ? "scene" : key);
    if (value && [...input.options].some((option) => option.value === value))
      input.value = value;
  }

  const $ = (id) => dialog.querySelector("#" + id),
    state = (current = {
      dialog,
      view: null,
      frame: null,
      geometry: null,
      source: "live",
      preset: null,
      adapter,
      closed: false,
    });
  let loading = false,
    frameKey = null,
    fitPending = true,
    serial = 0,
    sourceSerial = 0;

  const error = (message) => {
    $("preview-loading").textContent = message;
    $("preview-loading").hidden = false;
  };

  const options = () => ({
    backend: $("preview-backend").value,
    quality: $("preview-quality").value,
    onSelect: (id) => {
      $("preview-selection").textContent = id;
      if (state.source === "live") adapter.onSelect?.(id);
    },
    onError: (err) => {
      state.failed = true;
      error("Rendering stopped: " + err.message);
    },
  });

  const settings = () =>
    Object.fromEntries(
      [
        "gi",
        "volumes",
        "layout",
        "emitters",
        "haze",
        "exposure",
        "gain",
        "ambient",
      ].map((key) => {
        const input = $("preview-" + key);
        return [
          key,
          input.type === "checkbox" ? input.checked : Number(input.value),
        ];
      }),
    );

  function update(frame) {
    if (!state.view || !frame) return;
    const useOverrides =
      state.source === "carpushit" ||
      (state.preset &&
        frame.project.fixtures.filter((f) =>
          state.preset.project.fixtures.some((p) => p.id === f.id),
        ).length >= 30);
    state.view.update(
      frameToScene(frame, useOverrides ? state.preset.overrides : {}),
    );
    if (fitPending) {
      state.view.fit();
      fitPending = false;
    }
  }

  async function initialize() {
    const request = ++serial;
    loading = true;
    state.view?.dispose();
    state.view = null;
    const old = $("preview-canvas"),
      canvas = document.createElement("canvas");
    canvas.id = "preview-canvas";
    canvas.setAttribute("aria-label", "Interactive lighting preview");
    old.replaceWith(canvas);
    $("preview-loading").hidden = false;
    try {
      const view = await SceneView.create(canvas, options());
      if (request !== serial || !dialog.open) {
        view.dispose();
        return;
      }
      state.view = view;
      for (const key of ["gi", "volumes"]) {
        $("preview-" + key).disabled = !view.advanced;
        if (!view.advanced) $("preview-" + key).checked = false;
      }
      view.configure(settings());
      if (state.geometry) await view.loadGeometry(state.geometry.slice(0));
      update(state.frame);
      if (dialog.open) view.start();
      $("preview-loading").hidden = true;
    } catch (err) {
      if (request === serial) {
        state.failed = true;
        error("Graphics unavailable: " + err.message);
      }
    } finally {
      if (request === serial) loading = false;
    }
  }

  async function setSource() {
    const epoch = ++sourceSerial,
      source = $("preview-source").value;
    state.source = source;
    $("preview-review").hidden = true;
    $("preview-load-preset").hidden = source !== "carpushit";
    $("preview-mode").textContent =
      source === "carpushit"
        ? "Carpushit reference · static full look · engine unchanged"
        : "Live captured output · orbit / pinch / pan";
    try {
      let frame,
        geometry = null;
      state.preset ||= await preset();
      if (source === "carpushit") {
        frame = state.preset.frame;
        const response = await fetch(assets + "carpushit.glb");
        if (!response.ok) throw Error("Carpushit geometry unavailable");
        geometry = await response.arrayBuffer();
      } else {
        frame =
          adapter.getFrame?.() ||
          (await window.SchemaEngine.request("/api/state"));
        if (
          frame.project.fixtures.filter((f) =>
            state.preset.project.fixtures.some((p) => p.id === f.id),
          ).length >= 30
        )
          geometry = await (
            await fetch(assets + "carpushit.glb")
          ).arrayBuffer();
      }
      if (epoch !== sourceSerial) return;
      state.frame = frame;
      state.geometry = geometry;
      if (state.view) {
        if (geometry) await state.view.loadGeometry(geometry.slice(0));
        else state.view.clearGeometry();
      }
      if (epoch !== sourceSerial) return;
      fitPending = true;
      frameKey = null;
      update(frame);
      if (state.view) $("preview-loading").hidden = true;
    } catch (err) {
      if (epoch === sourceSerial) error(err.message);
    }
  }

  $("preview-close").onclick = () => dialog.close();
  dialog.addEventListener("close", () => {
    serial++;
    loading = false;
    state.view?.stop();
  });
  dialog.addEventListener("cancel", () => state.view?.stop());
  $("preview-fit").onclick = () => state.view?.fit();
  $("preview-backend").onchange = initialize;
  $("preview-quality").onchange = initialize;

  for (const key of [
    "gi",
    "volumes",
    "layout",
    "emitters",
    "haze",
    "exposure",
    "gain",
    "ambient",
  ])
    $("preview-" + key).addEventListener("input", () =>
      state.view?.configure(settings()),
    );

  $("preview-source").onchange = setSource;
  $("preview-reset-model").onclick = () => {
    state.geometry = null;
    state.view?.clearGeometry();
    state.view?.fit();
    if (state.view) $("preview-loading").hidden = true;
  };
  $("preview-model").onchange = async (event) => {
    const file = event.target.files[0];
    if (!file) return;
    try {
      if (file.size > 32 * 1024 * 1024) throw Error("Use a GLB under 32 MB");
      const buffer = await file.arrayBuffer();
      await state.view?.loadGeometry(buffer.slice(0));
      state.geometry = buffer;
      $("preview-loading").hidden = true;
    } catch (err) {
      error(err.message);
    }
  };

  $("preview-load-preset").onclick = () => {
    $("preview-review").hidden = false;
  };
  $("preview-cancel-load").onclick = () => {
    $("preview-review").hidden = true;
  };
  $("preview-confirm-load").onclick = async () => {
    const button = $("preview-confirm-load");
    button.disabled = true;
    try {
      const data = state.preset || (await preset());
      const ok = adapter.onLoadProject
        ? await adapter.onLoadProject(structuredClone(data.project))
        : await window.SchemaEngine.request("/api/state").then((live) =>
            window.SchemaEngine.request("/api/command", {
              expected_revision: live.snapshot.revision,
              command: { type: "replace_project", project: data.project },
            }),
          );
      if (ok === false) throw Error("Project change rejected");
      $("preview-source").value = "live";
      await setSource();
    } catch (err) {
      error(err.message);
    } finally {
      button.disabled = false;
    }
  };

  state.initialize = initialize;
  await setSource();
  if (dialog.open) await initialize();

  state.timer = setInterval(async () => {
    if (!dialog.open || document.hidden || loading) return;
    if (state.source === "live") {
      try {
        const frame =
          adapter.getFrame?.() ||
          (await window.SchemaEngine.request("/api/state"));
        if (state.source !== "live") return;
        const key = frame.snapshot.frame + ":" + frame.snapshot.revision;
        if (key !== frameKey) {
          state.frame = frame;
          frameKey = key;
          update(frame);
        }
      } catch (err) {
        $("preview-mode").textContent = "Engine disconnected · last frame";
      }
    }
    if (state.view) {
      const stats = state.view.stats;
      $("preview-stats").textContent =
        `${stats.backend} · ${stats.fps} fps · ${stats.fixtures} fixtures · ${stats.lights}/${stats.sourceCount} lights${state.view.advanced ? "" : " · basic lighting"}`;
      $("preview-stats").dataset.backend = stats.backend;
      $("preview-stats").dataset.frames = stats.frames;
    }
  }, 125);

  addEventListener(
    "pagehide",
    () => {
      clearInterval(state.timer);
      state.view?.dispose();
    },
    { once: true },
  );
}
