"use strict";
window.SchemaOutputPanel = (() => {
  const el = (tag, text, cls) => {
    const n = document.createElement(tag);
    if (text !== undefined) n.textContent = text;
    if (cls) n.className = cls;
    return n;
  };
  const button = (text, fn) => {
    const b = el("button", text);
    b.type = "button";
    b.onclick = fn;
    return b;
  };
  let serial = 0;
  function mount(host, options) {
    const root = el("section", undefined, "studio-output"),
      heading = el("div", undefined, "output-heading"),
      title = el("strong"),
      status = el("span");
    status.setAttribute("role", "status");
    heading.append(title, status);
    const form = el("form", undefined, "output-connection"),
      target = el("input"),
      enabled = el("input"),
      apply = el("button", "Apply");
    target.type = "text";
    target.setAttribute("aria-label", "Art-Net destination");
    enabled.type = "checkbox";
    enabled.setAttribute("aria-label", "Enable live DMX output");
    apply.type = "submit";
    const targetLabel = el("label", "Destination");
    targetLabel.append(target);
    const enabledLabel = el("label", "Live output", "output-enable");
    enabledLabel.prepend(enabled);
    form.append(targetLabel, enabledLabel, apply);
    const actions = el("div", undefined, "output-actions");
    for (const [tab, label] of [
      ["connection", "Connection"],
      ["patch", "Patch"],
      ["models", "Models"],
      ["dimming", "Dimming"],
      ["test", "Channel test"],
    ])
      actions.append(button(label, () => SchemaOutput.open(tab)));
    const blackout = button("Blackout", () => {
      const data = options.data();
      options.command(
        { type: "blackout", enabled: !data.snapshot.blackout },
        data.snapshot.revision,
      );
    });
    blackout.className = "output-blackout";
    actions.append(blackout);
    const line = el("div", undefined, "output-universe"),
      universe = el("select"),
      usage = el("span");
    universe.setAttribute("aria-label", "DMX universe");
    line.append(universe, usage);
    const grid = el("div", undefined, "output-levels"),
      readout = el("span", undefined, "output-readout"),
      message = el("p", undefined, "output-message");
    grid.setAttribute("role", "grid");
    grid.setAttribute("aria-label", "DMX channel levels");
    grid.tabIndex = 0;
    message.setAttribute("role", "status");
    const cells = [],
      id = "output-" + ++serial;
    let selected = 0,
      dirty = false,
      revision = 0,
      signature = "",
      lastPaint = -Infinity,
      disposed = false,
      busy = false,
      bytes = [];
    for (let i = 0; i < 512; i++) {
      const cell = el("span");
      cell.id = id + "-channel-" + i;
      cell.dataset.channel = i + 1;
      cell.setAttribute("role", "gridcell");
      cell.onpointerenter = () => {
        selected = i;
        read();
      };
      cell.onclick = () => {
        selected = i;
        read();
      };
      grid.append(cell);
      cells.push(cell);
    }
    function read() {
      readout.textContent = `Universe ${universe.value || 0} · Channel ${selected + 1} · Value ${bytes[selected] || 0}`;
      grid.setAttribute("aria-activedescendant", cells[selected].id);
      for (const cell of grid.querySelectorAll(".channel-selected"))
        cell.classList.remove("channel-selected");
      cells[selected].classList.add("channel-selected");
    }
    grid.onkeydown = (e) => {
      const delta = {
        ArrowLeft: -1,
        ArrowRight: 1,
        ArrowUp: -32,
        ArrowDown: 32,
        Home: -512,
        End: 512,
      }[e.key];
      if (delta === undefined) return;
      e.preventDefault();
      selected = Math.max(0, Math.min(511, selected + delta));
      read();
    };
    universe.onchange = () => update(true);
    const reload = button("Reload settings", () => {
      dirty = false;
      message.textContent = "";
      update(true);
    });
    for (const input of [target, enabled])
      input.addEventListener("input", () => {
        dirty = true;
      });
    form.onsubmit = async (e) => {
      e.preventDefault();
      if (busy) return;
      busy = true;
      apply.disabled = true;
      const data = options.data();
      try {
        const ok = await options.command(
          {
            type: "set_output",
            config: {
              ...data.project.output,
              target: target.value.trim(),
              enabled: enabled.checked,
            },
          },
          revision,
        );
        if (ok) {
          dirty = false;
          message.textContent = "Output settings applied.";
        } else
          message.textContent =
            "Settings were not applied. Reload settings to review the latest values.";
      } finally {
        busy = false;
        apply.disabled = false;
        update(true);
      }
    };
    root.append(heading, form, actions, line, grid, readout, message, reload);
    host.replaceChildren(root);
    function update(force = false) {
      if (disposed) return;
      const now = performance.now();
      if (!force && now - lastPaint < 100) return;
      lastPaint = now;
      const data = options.data();
      if (!data) return;
      const config = data.project.output,
        isUsb = config.transport === "open_dmx";
      title.textContent = isUsb ? "Open DMX USB output" : "Art-Net output";
      status.textContent =
        (data.snapshot.blackout ? "Blackout · " : "") +
        (data.snapshot.debug_focus ? "Debug output · " : "") +
        data.snapshot.output_status;
      status.className =
        "state-" +
        (data.snapshot.blackout
          ? "danger"
          : data.snapshot.debug_focus
            ? "debug"
            : config.enabled
              ? "live"
              : "preview");
      if (!dirty && !busy) {
        revision = data.snapshot.revision;
        target.value = config.target;
        enabled.checked = config.enabled;
      }
      targetLabel.hidden = isUsb;
      enabled.disabled = window.SchemaEngine?.kind === "browser" || busy;
      apply.disabled = busy;
      blackout.textContent = data.snapshot.blackout
        ? "Release blackout"
        : "Blackout";
      blackout.setAttribute("aria-pressed", String(!!data.snapshot.blackout));
      const values = [
        ...new Set([
          "0",
          ...Object.keys(data.snapshot.universes || {}),
          ...data.project.fixtures.map((f) => String(f.universe)),
        ]),
      ].sort((a, b) => Number(a) - Number(b));
      const key = values.join(",");
      if (key !== signature) {
        const previous = universe.value;
        universe.replaceChildren(
          ...values.map((v) => new Option("Universe " + v, v)),
        );
        universe.value = values.includes(previous) ? previous : values[0];
        signature = key;
      }
      const current = Number(universe.value);
      bytes = data.snapshot.universes?.[universe.value] || [];
      const used = SchemaOutputCore.usage(data.project, current).filter(
        Boolean,
      ).length;
      usage.textContent = used + " / 512 slots patched";
      for (let i = 0; i < 512; i++) {
        const value = bytes[i] || 0,
          cell = cells[i];
        if (cell.dataset.value !== String(value)) {
          cell.dataset.value = value;
          cell.style.setProperty("--dmx-level", Math.max(0.1, value / 255));
          cell.title = "Channel " + (i + 1) + ": " + value;
          cell.setAttribute("aria-label", cell.title);
        }
      }
      read();
    }
    update(true);
    return {
      update,
      destroy() {
        disposed = true;
        host.replaceChildren();
      },
    };
  }
  return {
    render(host, options) {
      if (!host._outputPanel) host._outputPanel = mount(host, options);
      else host._outputPanel.update(true);
      host._panelTick = () => host._outputPanel?.update();
    },
  };
})();
