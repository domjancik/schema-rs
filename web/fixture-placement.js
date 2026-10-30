/* Layout editing shared by orthographic schematics and perspective previews. */
(function (global) {
  "use strict";
  const LIMIT = 1e7,
    probe = 0.01,
    names = ["X", "Y", "Z"];
  const validPosition = (p) =>
    Array.isArray(p) &&
    p.length === 3 &&
    p.every((v) => Number.isFinite(v) && Math.abs(v) <= LIMIT);
  const round = (v) => Math.round(v * 1e6) / 1e6;
  function move(position, axes, delta, project, snap = 0) {
    if (!validPosition(position)) throw Error("Invalid fixture position");
    if (!Number.isFinite(snap) || snap < 0)
      throw Error("Invalid snap distance");
    if (
      !axes.length ||
      axes.length > 2 ||
      new Set(axes).size !== axes.length ||
      axes.some((a) => ![0, 1, 2].includes(a))
    )
      throw Error("Invalid move axes");
    if (!Number.isFinite(delta.x) || !Number.isFinite(delta.y)) return null;
    const a = project(position);
    if (!a || a.visible === false || !(a.w > 0)) return null;
    const ends = axes.map((axis) => {
      const p = [...position];
      p[axis] += probe;
      return project(p);
    });
    if (ends.some((p) => !p || p.visible === false || !(p.w > 0))) return null;
    let target = { x: a.x + delta.x, y: a.y + delta.y };
    if (axes.length === 1) {
      const v = { x: ends[0].x - a.x, y: ends[0].y - a.y },
        length = v.x * v.x + v.y * v.y;
      if (length < 1e-12) return null;
      const t = (delta.x * v.x + delta.y * v.y) / length;
      target = { x: a.x + v.x * t, y: a.y + v.y * t };
    }
    // Solve in homogeneous screen coordinates, so depth changes do not distort a drag.
    const vectors = ends.map((p) => ({
        x: p.x * p.w - a.x * a.w - target.x * (p.w - a.w),
        y: p.y * p.w - a.y * a.w - target.y * (p.w - a.w),
      })),
      c = { x: (target.x - a.x) * a.w, y: (target.y - a.y) * a.w };
    let amounts;
    if (axes.length === 1) {
      const v = vectors[0],
        length = v.x * v.x + v.y * v.y;
      if (length < 1e-16) return null;
      amounts = [(c.x * v.x + c.y * v.y) / length];
    } else {
      const [u, v] = vectors,
        det = u.x * v.y - u.y * v.x;
      if (Math.abs(det) < 1e-12) return null;
      amounts = [(c.x * v.y - c.y * v.x) / det, (u.x * c.y - u.y * c.x) / det];
    }
    if (
      a.w + amounts.reduce((sum, t, i) => sum + t * (ends[i].w - a.w), 0) <=
      1e-6
    )
      return null;
    const result = [...position];
    axes.forEach((axis, i) => {
      const v = position[axis] + amounts[i] * probe;
      result[axis] = round(snap ? Math.round(v / snap) * snap : v);
    });
    return validPosition(result) ? result : null;
  }
  let adapter = {};
  const preferences = new Map();
  function mount(options) {
    const view = options.view(),
      host = options.host,
      key = options.key || view.id || "scene",
      prefs = preferences.get(key) || { enabled: false, snap: 0, plane: "0,2" };
    preferences.set(key, prefs);
    const ns = "http://www.w3.org/2000/svg",
      el = (tag, text, cls) => {
        const n = document.createElement(tag);
        if (text !== undefined) n.textContent = text;
        if (cls) n.className = cls;
        return n;
      },
      svg = (tag, attrs = {}) => {
        const n = document.createElementNS(ns, tag);
        for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
        return n;
      };
    const overlay = svg("svg", {
      class: "fixture-placement-overlay",
      "aria-label": "Fixture move gizmo",
    });
    overlay.style.display = "none";
    host.append(overlay);
    const launch = el("button", "Place", "fixture-placement-launch");
    launch.type = "button";
    launch.setAttribute("aria-label", "Place selected fixture");
    launch.title = "Fixture position and move gizmo (G)";
    const dialog = el("dialog", undefined, "fixture-placement-dialog");
    dialog.setAttribute("aria-label", "Fixture position");
    const head = el("div", undefined, "fixture-placement-heading"),
      title = el("h2", "Fixture position"),
      close = el("button", "×");
    close.type = "button";
    close.setAttribute("aria-label", "Close fixture position");
    close.onclick = () => dialog.close();
    head.append(title, close);
    const description = el("p"),
      grid = el("div", undefined, "fixture-position-fields"),
      fields = names.map((name, axis) => {
        const label = el("label", name),
          input = el("input");
        input.type = "number";
        input.step = "any";
        input.min = -LIMIT;
        input.max = LIMIT;
        input.setAttribute("aria-label", name + " position (m)");
        label.append(input);
        grid.append(label);
        const row = el("div", undefined, "fixture-position-nudge");
        for (const direction of [-1, 1]) {
          const button = el("button", direction < 0 ? "−" : "+");
          button.type = "button";
          button.setAttribute(
            "aria-label",
            `Nudge ${name} ${direction < 0 ? "down" : "up"}`,
          );
          button.onclick = () => {
            const value = Number(input.value);
            if (input.value.trim() && Number.isFinite(value))
              input.value = String(
                round(value + direction * (prefs.snap || 0.1)),
              );
            validateDraft();
          };
          row.append(button);
        }
        label.append(row);
        return input;
      });
    const settings = el("div", undefined, "fixture-position-settings"),
      snapLabel = el("label", "Snap"),
      snap = el("select");
    snap.setAttribute("aria-label", "Fixture snap distance");
    for (const [value, label] of [
      [0, "Free"],
      [0.01, "1 cm"],
      [0.1, "10 cm"],
      [0.5, "50 cm"],
      [1, "1 m"],
    ])
      snap.add(new Option(label, value));
    snap.value = prefs.snap;
    snap.onchange = () => (prefs.snap = Number(snap.value));
    snapLabel.append(snap);
    const planeLabel = el("label", "Drag plane"),
      plane = el("select");
    plane.setAttribute("aria-label", "Fixture drag plane");
    for (const [value, label] of [
      ["0,2", "XZ · horizontal"],
      ["0,1", "XY"],
      ["1,2", "YZ"],
    ])
      plane.add(new Option(label, value));
    plane.value = prefs.plane;
    plane.onchange = () => {
      prefs.plane = plane.value;
      update();
    };
    planeLabel.append(plane);
    settings.append(snapLabel, planeLabel);
    const actions = el("div", undefined, "fixture-position-actions"),
      apply = el("button", "Apply position"),
      gizmo = el("button"),
      ground = el("button", "Set Y to 0");
    for (const b of [apply, gizmo, ground]) b.type = "button";
    apply.className = "primary";
    actions.append(apply, gizmo, ground);
    ground.onclick = () => {
      fields[1].value = "0";
      validateDraft();
    };
    const message = el("p");
    message.setAttribute("role", "status");
    dialog.append(head, description, grid, settings, actions, message);
    const arrange=el("button","Arrange fixtures…");arrange.type="button";arrange.onclick=()=>{dialog.close();global.SchemaAuthoring?.arrange();};dialog.append(arrange);
    document.body.append(dialog);
    let lastDraw = null;
    let gesture = null,
      writing = false,
      disposed = false,
      opened = null,
      launchHost = null;
    const data = () => options.data(),
      selected = () =>
        data()?.project.fixtures.find((f) => f.id === options.selected()),
      canEdit = () =>
        !disposed &&
        !!selected() &&
        (options.enabled?.() ?? true) &&
        !!(options.commit || adapter.command);
    function notice(text) {
      message.textContent = text;
    }
    function validateDraft() {
      const p = fields.map((f) =>
        f.value.trim() === "" ? NaN : Number(f.value),
      );
      apply.disabled = writing || !canEdit() || !validPosition(p);
      return p;
    }
    function show() {
      const f = selected();
      if (!canEdit()) return;
      opened = { fixture: f.id, revision: data().snapshot.revision };
      description.textContent = f.name + " · layout position in metres";
      fields.forEach((field, i) => (field.value = String(f.position[i])));
      snap.value = prefs.snap;
      plane.value = prefs.plane;
      gizmo.textContent = prefs.enabled ? "Hide move gizmo" : "Show move gizmo";
      notice(
        "Drag an axis or plane. Arrow keys on a handle nudge; Shift is finer.",
      );
      validateDraft();
      dialog.showModal();
    }
    const expandable = host.classList.contains("screen") && view.id === "scene";
    function expand(on) {
      prefs.expanded = on;
      host.classList.toggle("fixture-placement-expanded", on);
      if (on)
        host.style.setProperty(
          "--placement-header-bottom",
          (document.querySelector(".site-header")?.getBoundingClientRect()
            .bottom || 0) +
            8 +
            "px",
        );
      launch.textContent = on ? "Done" : "Place";
      launch.setAttribute(
        "aria-label",
        on ? "Finish fixture placement" : "Place selected fixture",
      );
      options.resize?.();
      if (on) options.frame?.();
      else options.fit?.();
      update();
    }
    if (expandable) {
      const expandButton = el("button", "Expand 3D view");
      expandButton.type = "button";
      expandButton.onclick = () => {
        prefs.enabled = true;
        dialog.close();
        expand(true);
      };
      actions.append(expandButton);
    }
    if (options.frame) {
      const frameButton = el("button", "Frame layout");
      frameButton.type = "button";
      frameButton.onclick = () => {
        prefs.enabled = true;
        dialog.close();
        options.frame();
        update();
      };
      actions.append(frameButton);
    }
    launch.onclick = () => {
      if (prefs.expanded) {
        prefs.enabled = false;
        expand(false);
      } else show();
    };
    fields.forEach((field) => (field.oninput = validateDraft));
    gizmo.onclick = () => {
      prefs.enabled = !prefs.enabled;
      dialog.close();
      update();
    };
    apply.onclick = async () => {
      if (!opened) return;
      const p = validateDraft();
      if (!validPosition(p)) return;
      const target = { ...opened };
      if ((await commit(p, target.fixture, target.revision)) && dialog.open)
        dialog.close();
    };
    async function commit(position, fixture, revision) {
      if (writing || disposed || !canEdit() || !validPosition(position))
        return false;
      const original = data()?.project.fixtures.find((f) => f.id === fixture);
      if (!original) return false;
      if (position.every((v, i) => v === original.position[i])) return true;
      writing = true;
      options.active?.(true);
      launch.disabled = true;
      apply.disabled = true;
      try {
        const ok = await (options.commit || adapter.command)(
          { type: "move_fixture", fixture, position },
          revision,
        );
        if (ok === false)
          throw Error("Move rejected; refresh the scene and try again");
        return true;
      } catch (error) {
        notice(error.message);
        (options.status || adapter.status)?.(error.message, true);
        return false;
      } finally {
        writing = false;
        options.active?.(false);
        if (!disposed) update();
      }
    }
    function cancel() {
      if (!gesture) return;
      const pointer = gesture.pointer;
      gesture = null;
      options.active?.(false);
      if (overlay.hasPointerCapture(pointer))
        overlay.releasePointerCapture(pointer);
      update();
    }
    function update() {
      if (disposed) return;
      const f = selected();
      launch.disabled = !canEdit() || writing || !!adapter.pending?.();
      if (dialog.open) validateDraft();
      launch.setAttribute("aria-pressed", String(prefs.enabled));
      if (
        gesture &&
        (!canEdit() ||
          !f ||
          f.id !== gesture.fixture ||
          data().snapshot.revision !== gesture.revision)
      ) {
        cancel();
        notice("Scene changed; move canceled");
        return;
      }
      if (!prefs.enabled || !f || !canEdit()) {
        overlay.style.display = "none";
        return;
      }
      const element = options.view();
      if (!element) return;
      const rect = element.getBoundingClientRect(),
        parent = host.getBoundingClientRect();
      overlay.style.display = "block";
      Object.assign(overlay.style, {
        left: rect.left - parent.left + host.scrollLeft + "px",
        top: rect.top - parent.top + host.scrollTop + "px",
        width: rect.width + "px",
        height: rect.height + "px",
      });
      overlay.setAttribute("viewBox", `0 0 ${rect.width} ${rect.height}`);
      const position = gesture?.position || f.position,
        origin = options.project(position);
      overlay.dataset.position = JSON.stringify(position);
      overlay.dataset.fixture = f.id;
      const focus = overlay.contains(document.activeElement)
        ? document.activeElement.closest("[data-move-axes]")?.dataset.moveAxes
        : prefs.focusAxis;
      if (!origin || origin.visible === false || !(origin.w > 0)) {
        overlay.replaceChildren();
        lastDraw = null;
        return;
      }
      const rawVectors = names.map((name, axis) => {
        const p = [...position];
        p[axis] += probe;
        const q = options.project(p);
        if (!q || q.visible === false || !(q.w > 0)) return null;
        const x = q.x - origin.x,
          y = q.y - origin.y,
          length = Math.hypot(x, y);
        return length < 1e-6
          ? null
          : { x: (x / length) * 68, y: (y / length) * 68, length };
      });
      // Ignore engine frame counters: unchanged handles keep their focus and pointer identity.
      const signature = JSON.stringify([
        f.id,
        position,
        origin,
        rawVectors,
        rect.width,
        rect.height,
        prefs.plane,
        !!gesture,
      ]);
      if (signature === lastDraw) return;
      lastDraw = signature;
      overlay.replaceChildren();
      const maxLength = Math.max(...rawVectors.map((v) => v?.length || 0));
      const vectors = rawVectors.map((v) =>
        v && v.length >= maxLength * 0.04 ? v : null,
      );
      const axes = prefs.plane.split(",").map(Number),
        a = vectors[axes[0]],
        b = vectors[axes[1]];
      if (a && b && Math.abs(a.x * b.y - a.y * b.x) > 250) {
        const points = [
          [0.2, 0.2],
          [0.48, 0.2],
          [0.48, 0.48],
          [0.2, 0.48],
        ].map(
          ([u, v]) =>
            `${origin.x + a.x * u + b.x * v},${origin.y + a.y * u + b.y * v}`,
        );
        const handle = svg("g", {
          "data-move-axes": axes.join(","),
          role: "button",
          tabindex: 0,
          "aria-label":
            "Move fixture in " + axes.map((a) => names[a]).join("") + " plane",
          class: "fixture-plane-handle",
        });
        handle.append(
          svg("circle", {
            cx: origin.x + (a.x + b.x) * 0.34,
            cy: origin.y + (a.y + b.y) * 0.34,
            r: 22,
            class: "fixture-plane-hit",
          }),
          svg("polygon", { points: points.join(" ") }),
        );
        overlay.append(handle);
      }
      vectors.forEach((v, axis) => {
        if (!v) return;
        const handle = svg("g", {
          "data-move-axes": String(axis),
          role: "slider",
          tabindex: 0,
          "aria-label": "Move fixture " + names[axis],
          "aria-valuemin": -LIMIT,
          "aria-valuemax": LIMIT,
          "aria-valuenow": position[axis],
          "aria-valuetext": position[axis] + " metres",
          class: "fixture-axis-handle",
          "data-axis": names[axis],
        });
        handle.append(
          svg("line", {
            x1: origin.x,
            y1: origin.y,
            x2: origin.x + v.x,
            y2: origin.y + v.y,
            class: "fixture-axis-hit",
          }),
          svg("line", {
            x1: origin.x,
            y1: origin.y,
            x2: origin.x + v.x,
            y2: origin.y + v.y,
          }),
          svg("circle", {
            cx: origin.x + v.x,
            cy: origin.y + v.y,
            r: 22,
            class: "fixture-axis-hit",
          }),
          svg("circle", { cx: origin.x + v.x, cy: origin.y + v.y, r: 12 }),
        );
        const text = svg("text", { x: origin.x + v.x, y: origin.y + v.y + 4 });
        text.textContent = names[axis];
        handle.append(text);
        overlay.append(handle);
      });
      overlay.append(
        svg("circle", {
          cx: origin.x,
          cy: origin.y,
          r: 4,
          class: "fixture-move-origin",
        }),
      );
      if (gesture) {
        const label = svg("text", {
          x: origin.x + 10,
          y: origin.y + 22,
          class: "fixture-move-readout",
        });
        label.textContent = position.map(round).join(" / ") + " m";
        overlay.append(label);
      }
      if (focus) {
        overlay
          .querySelector(`[data-move-axes="${focus}"]`)
          ?.focus({ preventScroll: true });
        prefs.focusAxis = null;
      }
    }
    overlay.onpointerdown = (event) => {
      const handle = event.target.closest("[data-move-axes]"),
        f = selected();
      if (
        !handle ||
        event.button !== 0 ||
        !canEdit() ||
        writing ||
        adapter.pending?.()
      )
        return;
      event.preventDefault();
      event.stopPropagation();
      handle.focus({ preventScroll: true });
      gesture = {
        pointer: event.pointerId,
        fixture: f.id,
        revision: data().snapshot.revision,
        axes: handle.dataset.moveAxes.split(",").map(Number),
        original: [...f.position],
        position: [...f.position],
        start: { x: event.clientX, y: event.clientY },
        moved: false,
      };
      options.active?.(true);
      overlay.setPointerCapture(event.pointerId);
    };
    overlay.onpointermove = (event) => {
      const g = gesture;
      if (!g || event.pointerId !== g.pointer) return;
      event.preventDefault();
      event.stopPropagation();
      const delta = {
        x: event.clientX - g.start.x,
        y: event.clientY - g.start.y,
      };
      if (!g.moved && Math.hypot(delta.x, delta.y) < 3) return;
      g.moved = true;
      const p = move(
        g.original,
        g.axes,
        delta,
        options.project,
        prefs.snap / (event.shiftKey ? 10 : 1),
      );
      if (p) g.position = p;
      update();
    };
    overlay.onpointerup = (event) => {
      if (!gesture || event.pointerId !== gesture.pointer) return;
      event.preventDefault();
      event.stopPropagation();
      const g = gesture;
      gesture = null;
      if (overlay.hasPointerCapture(g.pointer))
        overlay.releasePointerCapture(g.pointer);
      if (g.moved) commit(g.position, g.fixture, g.revision);
      else {
        options.active?.(false);
        update();
      }
    };
    overlay.onpointercancel = overlay.onlostpointercapture = cancel;
    overlay.onwheel = (event) => {
      event.preventDefault();
      event.stopPropagation();
    };
    overlay.onkeydown = (event) => {
      if (
        (event.key === "Enter" || event.key === " ") &&
        event.target.closest("[data-move-axes]")
      ) {
        event.preventDefault();
        event.stopPropagation();
        show();
        return;
      }
      const handle = event.target.closest("[data-move-axes]"),
        f = selected();
      if (
        !handle ||
        !f ||
        !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)
      )
        return;
      event.preventDefault();
      event.stopPropagation();
      const axes = handle.dataset.moveAxes.split(",").map(Number),
        axis =
          axes.length === 1
            ? axes[0]
            : axes[["ArrowUp", "ArrowDown"].includes(event.key) ? 1 : 0],
        direction = ["ArrowUp", "ArrowRight"].includes(event.key) ? 1 : -1,
        p = [...f.position];
      p[axis] = round(
        p[axis] + (direction * (prefs.snap || 0.1)) / (event.shiftKey ? 10 : 1),
      );
      prefs.focusAxis = handle.dataset.moveAxes;
      commit(p, f.id, data().snapshot.revision);
    };
    function keys(event) {
      if (event.key === "Escape" && gesture) {
        event.preventDefault();
        event.stopPropagation();
        cancel();
      } else if (
        canEdit() &&
        event.key.toLowerCase() === "g" &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey &&
        !event.repeat &&
        (event.target === options.view() || overlay.contains(event.target)) &&
        !document.querySelector("dialog[open]")
      ) {
        event.preventDefault();
        event.stopPropagation();
        prefs.enabled = !prefs.enabled;
        update();
        if (!prefs.enabled) options.view()?.focus({ preventScroll: true });
      }
    }
    function pointer(event) {
      if (gesture && event.pointerId !== gesture.pointer) {
        cancel();
        event.preventDefault();
        event.stopPropagation();
      }
    }
    function visibility() {
      if (document.hidden) cancel();
    }
    document.addEventListener("visibilitychange", visibility);
    global.addEventListener("blur", cancel);
    document.addEventListener("keydown", keys, true);
    host.addEventListener("pointerdown", pointer, true);
    return {
      update,
      get active() {
        return !!gesture || writing;
      },
      toolbar(parent) {
        if (launchHost !== parent) {
          parent.append(launch);
          launchHost = parent;
        }
        update();
      },
      destroy() {
        disposed = true;
        cancel();
        if (expandable) {
          prefs.expanded = false;
          host.classList.remove("fixture-placement-expanded");
        }
        overlay.remove();
        launch.remove();
        dialog.remove();
        document.removeEventListener("visibilitychange", visibility);
        global.removeEventListener("blur", cancel);
        document.removeEventListener("keydown", keys, true);
        host.removeEventListener("pointerdown", pointer, true);
      },
    };
  }
  const api = { move, mount, configure: (value) => (adapter = value) };
  global.SchemaPlacement = api;
  if (typeof module !== "undefined") module.exports = api;
})(globalThis);
