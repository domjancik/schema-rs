/* Live evaluated attributes. Demand is collected from visible views, never from authored state. */
(function (global) {
  "use strict";
  const cache = new Map(),
    labels = {
      colors: "Colors",
      value: "Value",
      position: "Position",
      rotation: "Rotation",
      speed: "Speed",
      duration: "Duration",
      progress: "Progress",
      pick: "Pick",
      pixel_positions: "Pixels",
      fixture_reference: "Fixture",
      audio: "Audio",
      texture: "Texture",
      actions: "Actions",
      channels: "Channels",
      emitters: "Light emitters",
    };
  let adapter,
    timer,
    busy = false,
    lastRevision,
    lastRequest = -Infinity;
  const key = (t) => `${t.fixture}/${t.layer}/${t.node}`;
  const element = (tag, text, cls) => {
    const e = document.createElement(tag);
    if (text !== undefined) e.textContent = text;
    if (cls) e.className = cls;
    return e;
  };
  const target = (e) => ({
    fixture:
      e.dataset.inspectFixture ||
      e.dataset.fixture ||
      adapter?.selection()?.fixture,
    layer: Number(
      e.dataset.inspectLayer ??
        e.dataset.layer ??
        adapter?.selection()?.layer ??
        0,
    ),
    node: Number(e.dataset.inspectNode ?? e.dataset.node),
  });
  const visible = (e) => {
    if (!e.isConnected || e.closest("[hidden]")) return false;
    const r = e.getBoundingClientRect();
    return (
      r.width > 0 &&
      r.height > 0 &&
      r.bottom > 0 &&
      r.right > 0 &&
      r.top < innerHeight &&
      r.left < innerWidth
    );
  };
  const number = (n) =>
    Number.isFinite(n) ? String(Math.round(n * 1000) / 1000) : "—";
  function format(value) {
    if (typeof value === "number") return number(value);
    if (Array.isArray(value))
      return value
        .map((v) =>
          Array.isArray(v)
            ? "[" + v.map(number).join(", ") + "]"
            : typeof v === "object"
              ? JSON.stringify(v)
              : typeof v === "number"
                ? number(v)
                : String(v),
        )
        .join(", ");
    if (value && typeof value === "object") return JSON.stringify(value);
    return String(value);
  }
  function swatch(color) {
    const c = element("i", undefined, "attribute-swatch");
    c.dataset.color = color.join(",");
    const rgba = color.map((v, i) =>
      i < 3
        ? Math.round(Math.max(0, Math.min(1, v)) * 255)
        : Math.max(0, Math.min(1, v)),
    );
    c.style.setProperty("--sample-color", `rgba(${rgba.join(",")})`);
    c.title = `RGBA ${color.map(number).join(", ")}`;
    c.setAttribute("role", "img");
    c.setAttribute("aria-label", c.title);
    return c;
  }
  function values(parent, name, value, count) {
    if (name === "colors") {
      const row = element("span", undefined, "attribute-colors");
      for (const color of value.slice(0, 8)) row.append(swatch(color));
      if (count > value.length)
        row.append(element("span", `+${count - value.length}`));
      parent.append(row, element("span", format(value), "attribute-numbers"));
    } else parent.append(element("span", format(value)));
    if (count !== undefined)
      parent.append(
        element(
          "small",
          `${count} ${count === 1 ? "item" : "items"}${count > 8 ? " · first 8 shown" : ""}`,
        ),
      );
  }
  function mount(parent, t) {
    const section = element("section", undefined, "block-inspection");
    section.dataset.inspectFixture = t.fixture;
    section.dataset.inspectLayer = t.layer;
    section.dataset.inspectNode = t.node;
    section.setAttribute("aria-label", "Evaluated block attributes");
    section.append(
      element("h3", "Live attributes"),
      element("p", "Waiting for evaluation…", "attribute-status"),
    );
    parent.append(section);
    return section;
  }
  function inspector(section, sample) {
    const signature = sample
      ? JSON.stringify([sample.incoming, sample.outgoing, sample.bypassed])
      : null;
    if (section._sample === signature) return;
    section._sample = signature;
    section.replaceChildren(element("h3", "Live attributes"));
    if (!sample) {
      section.append(
        element(
          "p",
          "Waiting for this block to evaluate. Disabled layers have no sample.",
          "attribute-status",
        ),
      );
      return;
    }
    if (sample.bypassed)
      section.append(
        element(
          "p",
          "Bypassed · incoming attributes pass through",
          "attribute-status",
        ),
      );
    for (const direction of ["incoming", "outgoing"]) {
      const bank = sample[direction],
        box = element("section", undefined, "attribute-direction");
      box.dataset.attributes = direction;
      box.append(
        element("h4", direction === "incoming" ? "Incoming" : "Outgoing"),
      );
      const entries = Object.entries(bank.attributes);
      if (!entries.length)
        box.append(element("p", "No attributes", "attribute-empty"));
      else {
        const dl = element("dl");
        for (const [name, value] of entries) {
          const dt = element("dt", labels[name] || name),
            dd = element("dd");
          values(dd, name, value, bank.counts[name]);
          dl.append(dt, dd);
        }
        box.append(dl);
      }
      section.append(box);
    }
  }
  function block(node, sample) {
    if (!sample) return;
    const signature = JSON.stringify(sample.outgoing);
    if (node._attributeSample === signature) return;
    node._attributeSample = signature;
    const attrs = sample.outgoing.attributes;
    const output =
      attrs.value ??
      attrs.colors?.[0]?.slice(0, 3).reduce((a, b) => a + b, 0) / 3;
    const readout = node.querySelector(".node-value");
    if (readout) {
      readout.textContent = Number.isFinite(output) ? output.toFixed(2) : "—";
      readout.title = `${attrs.value !== undefined ? "Output value" : "Brightness · mean RGB of first pixel"}: ${readout.textContent}`;
    }
    const meter = node.querySelector(".node-meter i");
    if (meter)
      meter.style.width = `${Math.max(0, Math.min(1, output || 0)) * 100}%`;
    if (node.namespaceURI === "http://www.w3.org/2000/svg") {
      let group = node.querySelector(".block-attributes");
      if (group) group.remove();
      group = document.createElementNS(node.namespaceURI, "g");
      group.classList.add("block-attributes");
      group.style.pointerEvents = "none";
      const body = node.querySelector(".node-body");
      if (!body) return;
      let x = Number(body.getAttribute("x")) + 6,
        y =
          Number(body.getAttribute("y")) +
          Number(body.getAttribute("height")) -
          10;
      for (const color of (attrs.colors || []).slice(0, 4)) {
        const rect = document.createElementNS(node.namespaceURI, "rect");
        for (const [k, v] of Object.entries({
          x,
          y,
          width: 9,
          height: 4,
          rx: 1,
          fill: `rgb(${color
            .slice(0, 3)
            .map((v) => Math.round(Math.max(0, Math.min(1, v)) * 255))
            .join(",")})`,
          "fill-opacity": color[3],
        }))
          rect.setAttribute(k, v);
        rect.dataset.color = color.join(",");
        group.append(rect);
        x += 11;
      }
      const text = node.querySelector(".node-output");
      if (text)
        text.textContent = Number.isFinite(output)
          ? output.toFixed(2)
          : attrs.position
            ? "Position"
            : "—";
      node.append(group);
      return;
    }
    let footer = node.querySelector(".block-readout");
    if (!footer) {
      footer = element("div", undefined, "block-readout");
      (node.querySelector(".block-face") || node).append(footer);
    }
    if (readout && readout.parentNode !== footer) footer.append(readout);
    const meterElement = node.querySelector(".node-meter");
    if (meterElement && meterElement.parentNode !== footer)
      footer.insertBefore(meterElement, readout?.parentNode === footer ? readout : null);
    let row = node.querySelector(".block-attributes");
    if (!row) row = element("div", undefined, "block-attributes");
    if (row.parentNode !== footer) footer.prepend(row);
    node.classList.add("has-attributes");
    row.replaceChildren();
    row.title = Object.entries(attrs)
      .map(([k, v]) => `${labels[k] || k}: ${format(v)}`)
      .join("\n");
    if (attrs.colors) {
      const colors = element("span", undefined, "attribute-colors");
      for (const c of attrs.colors.slice(0, 4)) colors.append(swatch(c));
      row.append(colors);
      if (sample.outgoing.counts.colors > 1)
        row.append(
          element(
            "span",
            `${sample.outgoing.counts.colors} px`,
            "attribute-count",
          ),
        );
    }
    const entries = Object.entries(attrs).filter(([k]) => k !== "colors");
    for (const [k, v] of entries.slice(0, 2))
      row.append(
        element("span", `${labels[k] || k} ${format(v)}`, "attribute-brief"),
      );
    if (entries.length > 2)
      row.append(element("span", `+${entries.length - 2}`));
    if (!Object.keys(attrs).length)
      row.append(element("span", "No output attributes", "attribute-empty"));
  }
  async function tick() {
    if (
      busy ||
      !adapter ||
      document.hidden ||
      performance.now() - lastRequest < 250
    )
      return;
    const revision = adapter.revision();
    if (revision !== lastRevision) {
      cache.clear();
      lastRevision = revision;
      document
        .querySelectorAll(".has-attributes,svg .node[data-node]")
        .forEach((n) => {
          n._attributeSample = undefined;
          n.classList.remove("has-attributes");
          n.querySelector(".block-attributes")?.remove();
        });
      document.querySelectorAll(".block-inspection").forEach((n) => {
        n._sample = undefined;
      });
    }
    const sections = [...document.querySelectorAll(".block-inspection")].filter(
        visible,
      ),
      nodes = [
        ...document.querySelectorAll(".block[data-node],svg .node[data-node]"),
      ]
        .filter(visible)
        .slice(0, 96),
      targets = new Map();
    for (const e of [...sections, ...nodes]) {
      const t = target(e);
      if (t.fixture && t.node) targets.set(key(t), t);
    }
    const mobile = adapter.mobile?.();
    if (mobile && visible(mobile)) {
      const t = adapter.selection();
      if (t.node) targets.set(key(t), t);
    }
    if (!targets.size) return;
    lastRequest = performance.now();
    busy = true;
    try {
      const frame = await SchemaEngine.request("/api/inspection", {
        targets: [...targets.values()],
      });
      if (frame.revision !== adapter.revision()) return;
      for (const b of frame.blocks)
        if (b.revision === adapter.revision()) cache.set(key(b.target), b);
      for (const k of cache.keys()) if (!targets.has(k)) cache.delete(k);
      paint();
    } catch (error) {
      for (const section of sections) {
        const status = section.querySelector(".attribute-status");
        if (status) status.textContent = error.message;
      }
    } finally {
      busy = false;
    }
  }
  function paint() {
    if (!adapter || document.hidden) return;
    const nodes = [
        ...document.querySelectorAll(".block[data-node],svg .node[data-node]"),
      ]
        .filter(visible)
        .slice(0, 96),
      sections = [...document.querySelectorAll(".block-inspection")].filter(
        visible,
      ),
      mobile = adapter.mobile?.();
    for (const n of nodes) block(n, cache.get(key(target(n))));
    for (const section of sections)
      inspector(section, cache.get(key(target(section))));
    if (mobile) {
      mobile.disabled = !adapter.selection()?.node;
      const sample = cache.get(key(adapter.selection()));
      const signature = JSON.stringify(sample?.outgoing.attributes ?? null);
      if (mobile._attributeSample === signature) return;
      mobile._attributeSample = signature;
      mobile.replaceChildren();
      if (sample?.outgoing.attributes.colors) {
        const colors = element("span", undefined, "attribute-colors");
        for (const c of sample.outgoing.attributes.colors.slice(0, 1))
          colors.append(swatch(c));
        mobile.append(colors);
      } else
        mobile.textContent =
          sample?.outgoing.attributes.value !== undefined
            ? number(sample.outgoing.attributes.value)
            : "◧";
    }
  }
  function configure(options) {
    adapter = options;
    clearInterval(timer);
    timer = setInterval(tick, 250);
    tick();
    const mobile = adapter.mobile?.();
    if (mobile) {
      mobile.disabled = !adapter.selection()?.node;
      mobile.setAttribute("role", "button");
      mobile.tabIndex = 0;
      mobile.setAttribute(
        "aria-label",
        "Inspect incoming and outgoing block attributes",
      );
      const open = () => {
        const t = adapter.selection();
        if (!t.node) return;
        const dialog = element("dialog", undefined, "attribute-dialog"),
          close = element("button", "Close");
        dialog.setAttribute("aria-label", "Live block attributes");
        close.onclick = () => dialog.close();
        dialog.append(close);
        const section = mount(dialog, t);
        inspector(section, cache.get(key(t)));
        dialog.addEventListener("close", () => dialog.remove(), { once: true });
        document.body.append(dialog);
        dialog.showModal();
        tick();
      };
      mobile.onclick = open;
      mobile.onkeydown = (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          open();
        }
      };
    }
  }
  global.SchemaInspection = {
    configure,
    mount,
    tick,
    paint,
    format,
    bank(t) {
      const sample = cache.get(key(t));
      return sample && sample.revision === adapter?.revision()
        ? sample.outgoing.attributes
        : undefined;
    },
  };
})(window);
