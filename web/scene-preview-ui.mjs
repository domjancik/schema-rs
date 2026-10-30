import { validateCamera } from "./scene-camera.mjs";
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
    current.placement?.destroy();
    current.dialog.remove();
    current = null;
  }
  if (current) {
    if (current.page) current.dialog.show();
    else current.dialog.showModal();
    if (current.view) current.view.start();
    else await current.initialize();
    return;
  }

  const dialog = document.createElement("dialog");
  dialog.id = "scene-preview-dialog";
  dialog.setAttribute("aria-label", "Scene view");
  dialog.innerHTML = `<div class="preview-heading"><div><span class="preview-kicker">SCHÉMA</span><h2>Scene view</h2></div><button id="preview-close" aria-label="Close scene preview">×</button></div><div class="preview-controls"><label>Scene<select id="preview-source"><option value="live">Live project</option><option value="carpushit">Carpushit</option></select></label><label>Graphics<select id="preview-backend"><option value="auto">Auto · WebGPU / WebGL2</option><option value="webgl">WebGL2</option></select></label><label>Quality<select id="preview-quality"><option value="low">Low · 12 lights</option><option value="balanced" selected>Balanced · 40 lights</option><option value="high">High · 64 lights</option></select></label><button id="preview-fit">Fit scene</button><button id="preview-iso" aria-label="Isometric view">Iso</button><button id="preview-rotate-left" aria-label="Rotate view left 45°">↶</button><button id="preview-rotate-right" aria-label="Rotate view right 45°">↷</button><label class="preview-file">Import scenery<input id="preview-model" type="file" accept=".glb,model/gltf-binary"></label><button id="preview-reset-model">Clear scenery</button><button id="preview-load-preset" hidden>Use Carpushit in project…</button></div><div class="preview-settings"><label><input id="preview-gi" type="checkbox" checked title="Approximate indirect lighting from visible surfaces">Indirect light</label><label><input id="preview-volumes" type="checkbox" checked>Light beams</label><label><input id="preview-layout" type="checkbox" checked>Layout handles</label><label><input id="preview-emitters" type="checkbox" checked>Light emitters</label><label>Haze<input id="preview-haze" type="range" min="0" max="4" step=".05" value=".18"></label><label>Exposure<input id="preview-exposure" type="range" min=".2" max="4" step=".1" value="1.4"></label><label>Light gain<input id="preview-gain" type="range" min="1" max="100" step="1" value="12"></label><label>Room fill<input id="preview-ambient" type="range" min="0" max="1" step=".01" value=".13"></label></div><div id="preview-review" hidden><p>Replace the current scene with Carpushit? Output will be disabled. You can undo this change.</p><button id="preview-confirm-load">Replace current scene</button><button id="preview-cancel-load">Keep current scene</button></div><div class="preview-viewport"><canvas id="preview-canvas" aria-label="Interactive lighting preview"></canvas><p id="preview-loading" role="status">Starting graphics…</p><span id="preview-selection"></span></div><div class="preview-foot"><span id="preview-stats" role="status"></span><span id="preview-mode">Live · orbit / pinch / pan</span></div>`;

  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = "/scene-preview.css";
  document.head.append(link);
  document.body.append(dialog);
  if (adapter.page) {
    dialog.classList.add("preview-page");
    if (adapter.embedded) {
      dialog.classList.add("preview-embedded");
      dialog.querySelector(".preview-heading").hidden = true;
    } else {
      const back = dialog.querySelector("#preview-close");
      back.textContent = "← Studio";
      back.setAttribute("aria-label", "Return to Studio");
    }
    dialog.show();
  } else dialog.showModal();

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
      page: !!adapter.page,
      view: null,
      frame: null,
      geometry: null,
      source: "live",
      preset: null,
      adapter,
      closed: false,
    });
  let placement;
  let loading = false,
    frameKey = null,
    fitPending = true,
    serial = 0,
    sourceSerial = 0;

  const navigation = document.createElement("details");
  navigation.className = "preview-navigation";
  navigation.open = !matchMedia("(max-width:700px), (pointer:coarse)").matches;
  navigation.innerHTML = `<summary>Camera</summary><label>Projection<select id="preview-projection" aria-label="Camera projection"><option value="perspective">Perspective</option><option value="orthographic">Orthographic</option></select></label><div class="camera-modes"><button data-camera-mode="orbit" aria-pressed="true">Orbit</button><button data-camera-mode="pan" aria-pressed="false">Pan</button></div><div class="camera-pad"><button data-camera="up" aria-label="Camera up">▲</button><button data-camera="left" aria-label="Camera left">◀</button><button data-camera="fit" aria-label="Fit scene">Fit</button><button data-camera="right" aria-label="Camera right">▶</button><button data-camera="down" aria-label="Camera down">▼</button></div><div class="camera-zoom"><button data-camera="out" aria-label="Zoom camera out">−</button><button data-camera="in" aria-label="Zoom camera in">+</button><button data-camera="focus" aria-label="Frame selected fixture">Frame</button></div><div class="camera-presets"><button data-camera-preset="top">Top</button><button data-camera-preset="front">Front</button><button data-camera-preset="right">Right</button><button data-camera-preset="iso">Iso</button></div><output id="preview-camera-status" aria-label="Camera pose">Orbit</output><p>Drag: orbit · Pan mode: drag to move<br>Pinch / wheel: zoom · Two fingers: pan<br>Canvas keys: arrows, WASD, +/−, F</p>`;
  dialog.querySelector(".preview-viewport").append(navigation);
  const cameraKey =
    "schema-camera-v1:" + (params.get("panel") || location.pathname);
  let storedCamera = null,
    cameraTimer,
    holdTimer;
  try {
    const saved = JSON.parse(localStorage.getItem(cameraKey));
    validateCamera(saved.pose);
    if (["live", "carpushit"].includes(saved.source)) {
      storedCamera = saved;
      if (!params.get("scene")) $("preview-source").value = saved.source;
    }
  } catch {}
  function saveCamera() {
    if (!state.view || loading) return;
    const record = {
      source: state.source,
      mode: state.view.navigationMode,
      pose: state.view.cameraState(),
    };
    try {
      localStorage.setItem(cameraKey, JSON.stringify(record));
    } catch {}
  }
  function cameraChanged(pose) {
    placement?.update();
    const distance = Math.hypot(
      ...pose.position.map((v, i) => v - pose.target[i]),
    );
    $("preview-camera-status").textContent =
      `${pose.mode === "pan" ? "Pan" : "Orbit"} · ${distance.toFixed(1)} m`;
    $("preview-camera-status").dataset.pose = JSON.stringify(pose);
    $("preview-projection").value = pose.projection || "perspective";
    for (const b of navigation.querySelectorAll("[data-camera-mode]"))
      b.setAttribute(
        "aria-pressed",
        String(b.dataset.cameraMode === pose.mode),
      );
    if (!loading) {
      clearTimeout(cameraTimer);
      cameraTimer = setTimeout(saveCamera, 150);
    }
  }
  function cameraAction(key) {
    const view = state.view;
    if (!view) return;
    if (key === "fit") {
      view.fit();
      return;
    }
    if (key === "focus") {
      view.focusSelected();
      return;
    }
    if (key === "in" || key === "out") {
      view.navigate({ zoom: key === "in" ? 0.88 : 1 / 0.88 });
      return;
    }
    const x = key === "left" ? -1 : key === "right" ? 1 : 0,
      y = key === "up" ? 1 : key === "down" ? -1 : 0;
    view.navigate(
      view.navigationMode === "pan"
        ? { pan: [x, y] }
        : { orbit: [x * 0.12, y * 0.12] },
    );
  }
  const clearHold = () => {
    clearInterval(holdTimer);
    holdTimer = null;
  };
  for (const button of navigation.querySelectorAll("[data-camera]")) {
    const key = button.dataset.camera;
    button.onclick = (event) => {
      if (event.detail === 0) cameraAction(key);
    };
    button.onpointerdown = (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      button.setPointerCapture(event.pointerId);
      cameraAction(key);
      if (!["fit", "focus"].includes(key)) {
        clearHold();
        holdTimer = setInterval(() => cameraAction(key), 110);
      }
    };
    button.onpointerup =
      button.onpointercancel =
      button.onlostpointercapture =
        clearHold;
  }
  $("preview-iso").onclick = () => state.view?.viewPreset("iso");
  $("preview-rotate-left").onclick = () =>
    state.view?.navigate({ orbit: [-Math.PI / 4, 0] });
  $("preview-rotate-right").onclick = () =>
    state.view?.navigate({ orbit: [Math.PI / 4, 0] });
  $("preview-projection").onchange = () =>
    state.view?.projectionSet($("preview-projection").value);
  for (const button of navigation.querySelectorAll("[data-camera-preset]"))
    button.onclick = () => state.view?.viewPreset(button.dataset.cameraPreset);
  for (const button of navigation.querySelectorAll("[data-camera-mode]"))
    button.onclick = () =>
      state.view?.navigationModeSet(button.dataset.cameraMode);
  dialog.addEventListener("close", () => {
    clearHold();
    saveCamera();
  });
  addEventListener(
    "pagehide",
    () => {
      clearHold();
      clearTimeout(cameraTimer);
      saveCamera();
    },
    { once: true },
  );

  const error = (message) => {
    $("preview-loading").textContent = message;
    $("preview-loading").hidden = false;
  };

  placement = window.SchemaPlacement?.mount({
    view: () => $("preview-canvas"),
    host: dialog.querySelector(".preview-viewport"),
    key: "preview:" + (params.get("panel") || location.pathname),
    data: () => state.frame,
    selected: () =>
      state.view?.selected ||
      adapter.selected?.() ||
      state.frame?.project.fixtures[0]?.id,
    enabled: () =>
      state.source === "live" &&
      !!state.view &&
      !loading &&
      dialog.open &&
      state.panelActive !== false,
    frame: () => state.view?.focusSelected(true),
    project: (p) => state.view?.projectPoint(p),
    active: (on) => state.view?.placementActive(on),
    status: adapter.status,
    commit: async (command, revision) => {
      const ok = adapter.command
        ? await adapter.command(command, revision)
        : await window.SchemaEngine.request("/api/command", {
            command,
            expected_revision: revision,
          });
      if (ok === false) return false;
      state.frame =
        adapter.getFrame?.() ||
        (await window.SchemaEngine.request("/api/state"));
      update(state.frame);
      return true;
    },
  });
  state.placement = placement;
  placement?.toolbar(dialog.querySelector(".preview-controls"));
  const options = () => ({
    backend: $("preview-backend").value,
    quality: $("preview-quality").value,
    onCameraChange: cameraChanged,
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
    const chosen = state.source === "live" ? adapter.selected?.() : null;
    if (chosen && frame.project.fixtures.some((f) => f.id === chosen))
      state.view.selected = chosen;
    else if (!frame.project.fixtures.some((f) => f.id === state.view.selected))
      state.view.selected = frame.project.fixtures[0]?.id || null;
    state.view.update(
      frameToScene(frame, useOverrides ? state.preset.overrides : {}),
    );
    placement?.update();
    if (fitPending) {
      state.view.fit();
      fitPending = false;
    }
  }

  async function initialize() {
    const request = ++serial;
    const cameraBefore = state.view
      ? {
          source: state.source,
          mode: state.view.navigationMode,
          pose: state.view.cameraState(),
        }
      : storedCamera;
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
      if (cameraBefore && cameraBefore.source === state.source) {
        view.restoreCamera(cameraBefore.pose);
        view.navigationModeSet(cameraBefore.mode);
      }
      storedCamera = null;
      if (dialog.open && state.panelActive !== false) view.start();
      $("preview-loading").hidden = true;
    } catch (err) {
      if (request === serial) {
        state.failed = true;
        error("Graphics unavailable: " + err.message);
      }
    } finally {
      if (request === serial) {
        loading = false;
        saveCamera();
      }
    }
  }

  async function setSource() {
    const epoch = ++sourceSerial,
      source = $("preview-source").value;
    state.source = source;
    placement?.update();
    $("preview-review").hidden = true;
    $("preview-load-preset").hidden = source !== "carpushit";
    $("preview-mode").textContent =
      source === "carpushit"
        ? "Carpushit · preview only"
        : "Live · orbit / pinch / pan";
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

  $("preview-close").onclick = () => {
    if (!adapter.page) dialog.close();
    else if (!adapter.embedded)
      location.assign(window.SchemaEngine?.kind === "browser" ? "/app?view=studio" : "/?view=studio");
  };
  dialog.addEventListener("close", () => {
    serial++;
    loading = false;
    state.view?.stop();
    placement?.update();
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
    if (
      !dialog.open ||
      document.hidden ||
      loading ||
      state.panelActive === false
    )
      return;
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
      placement?.destroy();
    },
    { once: true },
  );
}

export function setActive(active) {
  if (!current) return;
  current.panelActive = active;
  current.placement?.update();
  if (!active) current.view?.stop();
  else if (current.dialog.open) current.view?.start();
}
