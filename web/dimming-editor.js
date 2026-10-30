/* Cyclic schedule editor. Preview is local; applying time to output is explicit and leased. */
(function (global) {
  "use strict";
  const O = SchemaOutputCore,
    DAY = 86400;
  const el = (tag, text, cls) => {
    const n = document.createElement(tag);
    if (text !== undefined) n.textContent = text;
    if (cls) n.className = cls;
    return n;
  };
  const svg = (tag, attrs = {}) => {
    const n = document.createElementNS("http://www.w3.org/2000/svg", tag);
    for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
    return n;
  };
  const wrap = (n) => ((n % DAY) + DAY) % DAY;
  const time = (n) => {
    n = Math.floor(wrap(n));
    return [Math.floor(n / 3600), Math.floor(n / 60) % 60, n % 60]
      .map((v) => String(v).padStart(2, "0"))
      .join(":");
  };
  function mount(host, options) {
    const t = structuredClone(options.timeline),
      owner =
        "time-" +
        Array.from(crypto.getRandomValues(new Uint8Array(16)), (v) =>
          v.toString(16).padStart(2, "0"),
        ).join("");
    let preview = null,
      playing = false,
      applied = false,
      disposed = false,
      pending = false,
      previous = performance.now(),
      renewed = 0,
      changed = 0,
      owned = false,
      operations = Promise.resolve(),
      sendTimer = null,
      latestSetting = null,
      lastSent = -Infinity;
    const root = el("section", undefined, "dimming-editor"),
      form = el("div", undefined, "output-form"),
      notice = el(
        "p",
        "Preview only · output time is unchanged.",
        "dimming-notice",
      );
    const field = (title, input) => {
      const label = el("label", title);
      input.setAttribute("aria-label", title);
      label.append(input);
      form.append(label);
      return input;
    };
    const input = (type, value) => {
      const n = el("input");
      n.type = type;
      if (value !== undefined) n.value = value;
      return n;
    };
    const button = (title, fn) => {
      const n = el("button", title);
      n.type = "button";
      n.onclick = fn;
      return n;
    };
    const enabled = field("Daily curve enabled", input("checkbox"));
    enabled.checked = t.enabled;
    enabled.onchange = () => {
      t.enabled = enabled.checked;
      draw();
    };
    const master = field("Global master", input("range", t.master));
    master.min = 0;
    master.max = 1;
    master.step = 0.01;
    const masterValue = el("output", Math.round(t.master * 100) + "%");
    master.after(masterValue);
    master.oninput = () => {
      t.master = Number(master.value);
      masterValue.textContent = Math.round(t.master * 100) + "%";
      draw();
    };
    const offset = field(
      "UTC offset (minutes)",
      input("number", t.utc_offset_minutes),
    );
    offset.min = -840;
    offset.max = 840;
    offset.step = 15;
    offset.onchange = () => {
      t.utc_offset_minutes = Number(offset.value);
      draw();
      if (applied) send();
    };
    form.append(
      button("Use device time zone", () => {
        offset.value = -new Date().getTimezoneOffset();
        offset.onchange();
      }),
    );
    const live = el("output", undefined, "dimming-current-time"),
      level = el("output", undefined, "dimming-current-level");
    live.setAttribute("aria-label", "Current schedule time");
    level.setAttribute("aria-label", "Current output dimming");
    root.append(
      form,
      live,
      level,
      el(
        "p",
        "Repeats daily, including the segment across midnight. Levels multiply fixture dimmers; control/program slots stay unchanged.",
      ),
    );
    const graph = svg("svg", {
        viewBox: "0 0 720 220",
        class: "dimming-graph",
        role: "img",
        "aria-label": "Daily dimming timeline",
      }),
      rows = el("div", undefined, "timeline-points");
    root.append(
      graph,
      el("p", "━ Current time   ┄ Simulated time", "dimming-legend"),
    );
    let liveLine, previewLine;
    function marker(cls, label) {
      const g = svg("g", { class: cls, "aria-label": label }),
        line = svg("path", { d: "M0 10V190" }),
        dot = svg("circle", { r: 5 });
      g.append(line, dot);
      graph.append(g);
      return { g, line, dot };
    }
    function move(m, seconds) {
      const x = 20 + (wrap(seconds) / DAY) * 680;
      m.g.setAttribute("transform", `translate(${x} 0)`);
      m.dot.setAttribute("cy", 190 - O.level(t, wrap(seconds) / 60) * 175);
    }
    function draw() {
      graph.replaceChildren();
      for (let h = 0; h <= 24; h += 3) {
        const x = 20 + (h / 24) * 680;
        graph.append(
          svg("path", { d: `M${x} 10V190`, class: "timeline-grid" }),
        );
        const label = svg("text", { x, y: 210, "text-anchor": "middle" });
        label.textContent = String(h).padStart(2, "0") + ":00";
        graph.append(label);
      }
      graph.append(
        svg("path", {
          class: "timeline-curve",
          d: Array.from(
            { length: 289 },
            (_, i) =>
              `${i ? "L" : "M"}${20 + (i / 288) * 680} ${190 - O.level(t, i * 5) * 175}`,
          ).join(" "),
        }),
      );
      for (const point of t.points) {
        const dot = svg("circle", {
          cx: 20 + (point.minute / 1440) * 680,
          cy: 190 - point.level * t.master * 175,
          r: 7,
          class: "timeline-handle",
          tabindex: 0,
          role: "slider",
          "aria-label": O.time(point.minute) + " dimming point",
          "aria-valuemin": 0,
          "aria-valuemax": 1439,
          "aria-valuenow": point.minute,
        });
        dot.onpointerdown = (e) => {
          dot.setPointerCapture(e.pointerId);
          e.preventDefault();
        };
        dot.onpointermove = (e) => {
          if (!dot.hasPointerCapture(e.pointerId) || !graph.getScreenCTM())
            return;
          const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(
              graph.getScreenCTM().inverse(),
            ),
            minute = Math.max(
              0,
              Math.min(1439, Math.round((((p.x - 20) / 680) * 1440) / 5) * 5),
            );
          if (!t.points.some((v) => v !== point && v.minute === minute))
            point.minute = minute;
          point.level = Math.max(
            0,
            Math.min(1, (190 - p.y) / 175 / (t.master || 1)),
          );
          dot.setAttribute("cx", 20 + (point.minute / 1440) * 680);
          dot.setAttribute("cy", 190 - point.level * t.master * 175);
        };
        dot.onpointerup = dot.onpointercancel = () => {
          draw();
          drawRows();
        };
        dot.onkeydown = (e) => {
          if (
            !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)
          )
            return;
          e.preventDefault();
          e.stopPropagation();
          if (e.key === "ArrowUp" || e.key === "ArrowDown")
            point.level = Math.max(
              0,
              Math.min(1, point.level + (e.key === "ArrowUp" ? 0.01 : -0.01)),
            );
          else {
            const m =
              (point.minute + (e.key === "ArrowRight" ? 5 : 1435)) % 1440;
            if (!t.points.some((p) => p !== point && p.minute === m))
              point.minute = m;
          }
          draw();
          drawRows();
        };
        graph.append(dot);
      }
      liveLine = marker("timeline-now", "Current time");
      previewLine = marker("timeline-preview", "Simulated time");
      update(false);
    }
    function drawRows() {
      rows.replaceChildren();
      t.points.sort((a, b) => a.minute - b.minute);
      for (const point of t.points) {
        const row = el("div", undefined, "timeline-row"),
          at = input("time", O.time(point.minute)),
          amount = input("number", Math.round(point.level * 100));
        at.setAttribute("aria-label", "Point time");
        amount.setAttribute("aria-label", "Point level percent");
        amount.min = 0;
        amount.max = 100;
        at.onchange = () => {
          point.minute = O.minute(at.value);
          draw();
        };
        amount.onchange = () => {
          point.level = Number(amount.value) / 100;
          draw();
        };
        row.append(
          at,
          amount,
          el("span", "%"),
          button("Remove", () => {
            t.points.splice(t.points.indexOf(point), 1);
            draw();
            drawRows();
          }),
        );
        rows.append(row);
      }
    }
    const controls = el("div", undefined, "dimming-time-controls"),
      simTime = input("time"),
      scrub = input("range"),
      play = button("Play time simulation", () => {
        preview ??= liveLocal();
        playing = !playing;
        previous = performance.now();
        if (applied) send();
        update(false);
      }),
      speed = el("select"),
      apply = input("checkbox"),
      previewLevel = el("output");
    simTime.step = 1;
    simTime.setAttribute("aria-label", "Simulated schedule time");
    scrub.min = 0;
    scrub.max = 86399;
    scrub.step = 1;
    scrub.setAttribute("aria-label", "Scrub schedule time");
    for (const [v, label] of [
      [1, "1×"],
      [60, "60× · one minute per second"],
      [3600, "3600× · one hour per second"],
    ])
      speed.add(new Option(label, v));
    speed.value = "60";
    speed.setAttribute("aria-label", "Time simulation speed");
    function seek(seconds) {
      if (!Number.isFinite(seconds)) return;
      preview = wrap(seconds);
      playing = false;
      previous = performance.now();
      if (applied) send();
      update(false);
    }
    simTime.oninput = () => {
      if (simTime.value) {
        const [h, m, s = 0] = simTime.value.split(":").map(Number);
        seek(h * 3600 + m * 60 + s);
      }
    };
    scrub.oninput = () => seek(Number(scrub.value));
    speed.onchange = () => {
      previous = performance.now();
      if (applied) send();
    };
    const applyLabel = el("label", "Apply simulated time to output");
    apply.setAttribute("aria-label", "Apply simulated time to output");
    applyLabel.prepend(apply);
    apply.onchange = () => {
      applied = apply.checked;
      owned = false;
      preview ??= liveLocal();
      if (applied) send();
      else {
        notice.textContent = "Returning output to live time…";
        control({ action: "live", owner }).then((result) => {
          if (result && !disposed)
            notice.textContent = result.simulating
              ? "Preview only · another view controls output time."
              : "Preview only · output follows live time.";
        });
      }
      update(false);
    };
    const reset = button("Use live time", () => {
      preview = null;
      playing = false;
      applied = false;
      apply.checked = false;
      notice.textContent = "Returning output to live time…";
      control({ action: "live" }).then((result) => {
        if (result && !disposed)
          notice.textContent = "Output follows live time.";
      });
      update(false);
    });
    controls.append(
      el("strong", "Time simulation"),
      simTime,
      play,
      speed,
      reset,
      applyLabel,
      previewLevel,
      scrub,
      notice,
    );
    previewLevel.setAttribute("aria-label", "Preview dimming");
    root.append(controls, rows);
    root.append(
      button("+ Add time point", () => {
        if (t.points.length >= 96) {
          notice.textContent = "Maximum 96 time points";
          return;
        }
        let minute = 720;
        while (t.points.some((p) => p.minute === minute))
          minute = (minute + 15) % 1440;
        t.points.push({ minute, level: 1 });
        draw();
        drawRows();
      }),
      button("Save dimming timeline", () => options.save(t)),
    );
    host.append(root);
    function status() {
      return options.data?.()?.dimming_clock;
    }
    function liveUTC() {
      return (
        status()?.live_seconds ??
        (() => {
          const d = new Date();
          return (
            d.getUTCHours() * 3600 + d.getUTCMinutes() * 60 + d.getUTCSeconds()
          );
        })()
      );
    }
    function liveLocal() {
      return wrap(liveUTC() + t.utc_offset_minutes * 60);
    }
    async function control(body) {
      try {
        operations = operations
          .catch(() => {})
          .then(() => SchemaEngine.request("/api/dimming/clock", body));
        return await operations;
      } catch (error) {
        if (!disposed) notice.textContent = error.message;
        return null;
      }
    }
    function send() {
      changed = performance.now();
      const actualOffset =
        options.data?.()?.project?.dimming?.utc_offset_minutes ??
        t.utc_offset_minutes;
      latestSetting = {
        action: "simulate",
        owner,
        seconds: wrap((preview ?? liveLocal()) - actualOffset * 60),
        speed: playing ? Number(speed.value) : 0,
      };
      if (!pending && !sendTimer)
        sendTimer = setTimeout(
          flush,
          Math.max(0, 100 - (performance.now() - lastSent)),
        );
    }
    async function flush() {
      sendTimer = null;
      if (disposed || !applied) {
        latestSetting = null;
        return;
      }
      pending = true;
      const settings = latestSetting;
      latestSetting = null;
      lastSent = performance.now();
      const result = await control(settings);
      pending = false;
      if (disposed || !applied) {
        if (result) control({ action: "live", owner });
        return;
      }
      if (result) {
        owned = true;
        renewed = performance.now();
        notice.textContent =
          "Output uses simulated time. Closing this view returns it to live.";
      } else {
        applied = false;
        apply.checked = false;
      }
      if (latestSetting && !sendTimer)
        sendTimer = setTimeout(
          flush,
          Math.max(0, 100 - (performance.now() - lastSent)),
        );
    }
    function update(advance = true) {
      if (disposed) return;
      const now = performance.now(),
        clock = status();
      if (advance && preview !== null && playing)
        preview = wrap(
          preview + ((now - previous) / 1000) * Number(speed.value),
        );
      previous = now;
      if (
        applied &&
        clock?.simulating &&
        clock.owner === owner &&
        !pending &&
        !latestSetting
      )
        preview = wrap(clock.seconds + t.utc_offset_minutes * 60);
      if (
        applied &&
        owned &&
        !pending &&
        now - changed > 1000 &&
        clock &&
        clock.owner !== owner
      ) {
        applied = false;
        apply.checked = false;
        notice.textContent = clock.simulating
          ? "Output time is controlled from another view."
          : "Output has returned to live time.";
      }
      if (applied && !document.hidden && !pending && now - renewed >= 3000) {
        renewed = now;
        control({ action: "renew", owner }).then((result) => {
          if (!result && !disposed) {
            applied = false;
            apply.checked = false;
          }
        });
      }
      const minutes = Math.abs(t.utc_offset_minutes),
        zone = `${t.utc_offset_minutes >= 0 ? "+" : "-"}${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
      live.textContent = `Current time ${time(liveLocal())} · UTC${zone}`;
      level.textContent = `Output ${Math.round((clock?.level ?? O.level(t, liveLocal() / 60)) * 100)}%${clock?.simulating ? " · simulated time" : " · live time"}`;
      const seconds = preview ?? liveLocal();
      if (document.activeElement !== simTime) simTime.value = time(seconds);
      if (document.activeElement !== scrub) scrub.value = seconds;
      play.textContent = playing
        ? "Pause time simulation"
        : "Play time simulation";
      previewLevel.textContent = `${preview === null ? "Live curve" : "Preview"} ${time(seconds)} · ${Math.round(O.level(t, seconds / 60) * 100)}%`;
      if (liveLine) {
        move(liveLine, liveLocal());
        previewLine.g.style.display = preview === null ? "none" : "";
        if (preview !== null) move(previewLine, preview);
        liveLine.g.setAttribute(
          "aria-label",
          "Current time " + time(liveLocal()),
        );
        previewLine.g.setAttribute(
          "aria-label",
          "Simulated time " + time(seconds),
        );
      }
    }
    draw();
    drawRows();
    const timer = setInterval(() => update(true), 250);
    return {
      destroy() {
        disposed = true;
        clearInterval(timer);
        clearTimeout(sendTimer);
        latestSetting = null;
        if (applied) control({ action: "live", owner });
        root.remove();
      },
    };
  }
  global.SchemaDimmingEditor = { mount };
})(window);
