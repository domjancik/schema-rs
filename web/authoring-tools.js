"use strict";
window.SchemaAuthoring = (() => {
  let adapter,
    latest,
    bar,
    paletteButton,
    signature = "",
    pickerDialog;
  const el = (tag, text, cls) => {
    const n = document.createElement(tag);
    if (text !== undefined) n.textContent = text;
    if (cls) n.className = cls;
    return n;
  };
  const button = (text, action) => {
    const n = el("button", text);
    n.type = "button";
    n.onclick = action;
    return n;
  };
  const notify = (text, error = false) => adapter?.onStatus?.(text, error);
  function sheet(title) {
    const d = el("dialog", undefined, "authoring-dialog");
    d.setAttribute("aria-label", title);
    const h = el("header"),
      heading = el("h2", title),
      close = button("×", () => d.close());
    close.setAttribute("aria-label", "Close " + title);
    h.append(heading, close);
    d.append(h);
    document.body.append(d);
    d.onclose = () => d.remove();
    return d;
  }
  function field(host, label, input) {
    const l = el("label", label);
    input.setAttribute("aria-label", label);
    l.append(input);
    host.append(l);
    return input;
  }
  async function run(command, revision = adapter.getRevision()) {
    try {
      return await adapter.commit(command, revision);
    } catch (e) {
      notify(e.message, true);
      return false;
    }
  }
  function selected() {
    const s = adapter.getSelection();
    return { fixture: s.fixture, layer: s.layer, node: s.node };
  }
  function selectedNode() {
    const s = selected(),
      p = adapter.getProject();
    return SchemaBlocks.find(
      (s.fixture === "__director" ? p.director : p.content[s.fixture])?.[
        s.layer
      ]?.root,
      s.node,
    );
  }
  function blockControls(host, node, target) {
    if (!node || !adapter) return;
    const revision = adapter.getRevision();
    const wrap = el("div", undefined, "block-state-controls");
    wrap.dataset.node = node.id;
    wrap.dataset.fixture = target.fixture;
    wrap.dataset.layer = target.layer;
    const bypass = button(node.bypassed ? "Enable block" : "Bypass block", () =>
      run(
        { type: "set_node_bypassed", ...target, bypassed: !node.bypassed },
        revision,
      ),
    );
    bypass.setAttribute("aria-pressed", String(!!node.bypassed));
    bypass.className = "bypass-control";
    bypass.title =
      "Pass inputs through without evaluating this block or its property drivers. Saved and undoable.";
    wrap.append(bypass);
    if (target.fixture !== "__director") {
      const focus = button("Focus output", () => {
        const current = latest?.snapshot.debug_focus;
        const active =
          current &&
          current.fixture === target.fixture &&
          current.layer === target.layer &&
          current.node === node.id;
        return run({ type: "set_debug_focus", target: active ? null : target });
      });
      focus.className = "debug-control";
      focus.title =
        "Temporarily send this block’s evaluated output to the fixture. The program continues running.";
      wrap.append(focus);
    }
    host.append(wrap);
    syncControls();
  }
  function decorateBlock(element, node, target) {
    element.classList.toggle("is-bypassed", !!node.bypassed);
    element.dataset.bypassed = String(!!node.bypassed);
    element.dataset.fixture = target.fixture;
    element.dataset.layer = target.layer;
    const focus = latest?.snapshot.debug_focus;
    const debug =
      !!focus &&
      focus.fixture === target.fixture &&
      focus.layer === target.layer &&
      focus.node === node.id;
    element.classList.toggle("is-debug-output", debug);
    if (node.bypassed || debug) {
      if (element.namespaceURI === "http://www.w3.org/2000/svg") {
        const label = document.createElementNS(element.namespaceURI, "title");
        label.textContent = node.bypassed
          ? "Bypassed · inputs pass through"
          : "Debug output focus";
        element.append(label);
      } else {
        const badge = el(
          "span",
          node.bypassed ? "BYPASSED" : "DEBUG OUTPUT",
          "block-state-badge",
        );
        element.append(badge);
      }
    }
  }
  function syncControls() {
    for (const box of document.querySelectorAll(".block-state-controls")) {
      const focus = latest?.snapshot.debug_focus;
      const active =
        !!focus &&
        focus.fixture === box.dataset.fixture &&
        focus.layer === Number(box.dataset.layer) &&
        focus.node === Number(box.dataset.node);
      const control = box.querySelector(".debug-control");
      if (control) {
        control.textContent = active ? "Stop output focus" : "Focus output";
        control.setAttribute("aria-pressed", String(active));
      }
    }
    for (const block of document.querySelectorAll(
      ".block[data-node],.node[data-node]",
    )) {
      const focus = latest?.snapshot.debug_focus;
      const active =
        !!focus &&
        focus.fixture === block.dataset.fixture &&
        focus.layer === Number(block.dataset.layer) &&
        focus.node === Number(block.dataset.node);
      block.classList.toggle("is-debug-output", active);
    }
  }
  function update(data) {
    latest = data;
    if (!data || !adapter) return;
    if (paletteButton) paletteButton.disabled = false;
    syncControls();
    const s = data.snapshot,
      p = data.project,
      key = JSON.stringify([
        p.output.enabled,
        p.output.transport,
        s.paused,
        s.blackout,
        s.debug_focus,
        s.diagnostics,
      ]);
    if (key === signature) return;
    signature = key;
    if (!bar) return;
    bar.replaceChildren();
    const state = (text, kind, title) => {
      const chip = el("span", text, "state-chip state-" + kind);
      chip.title = title || text;
      bar.append(chip);
    };
    const browser = SchemaEngine.kind === "browser";
    state(
      s.blackout
        ? "Blackout"
        : browser
          ? "Browser preview"
          : p.output.enabled
            ? "Live output"
            : "Capture only",
      s.blackout ? "danger" : p.output.enabled ? "live" : "preview",
      s.blackout
        ? "All physical output is blacked out"
        : browser
          ? "Browser computes scene values; physical output is unavailable"
          : p.output.enabled
            ? "Physical output is enabled"
            : "Physical output is disabled; computed channel levels remain visible",
    );
    if (s.paused) state("Paused", "paused");
    if (s.debug_focus) {
      const f = p.fixtures.find((f) => f.id === s.debug_focus.fixture);
      const stop = button(
        (document.querySelector(".screen-caption")
          ? "Debug output"
          : "Debug · " + (f?.name || s.debug_focus.fixture)) + " · Stop",
        () => run({ type: "set_debug_focus", target: null }),
      );
      stop.className = "state-chip state-debug";
      stop.title =
        "Temporary block output focus is active. Stop to restore normal fixture output.";
      bar.append(stop);
    }
    if (s.diagnostics?.length)
      state(
        s.diagnostics.length +
          " warning" +
          (s.diagnostics.length === 1 ? "" : "s"),
        "warning",
        s.diagnostics.join("\n"),
      );
  }
  function choose(candidates, onSelect, event) {
    const unique = [...new Map(candidates.map((f) => [f.id, f])).values()];
    if (!unique.length) return;
    if (unique.length === 1) {
      onSelect(unique[0].id, !!event?.shiftKey);
      return;
    }
    if (event?.altKey) {
      const index = unique.findIndex(
        (f) => f.id === adapter?.getSelection().fixture,
      );
      onSelect(unique[(index + 1) % unique.length].id, !!event?.shiftKey);
      return;
    }
    pickerDialog?.close();
    const d = (pickerDialog = sheet("Choose fixture")),
      input = field(d, "Find overlapping fixture", el("input")),
      list = el("div", undefined, "authoring-results");
    input.type = "search";
    const hint = el(
      "p",
      unique.length + " fixtures here · choose by name, group or DMX address.",
    );
    d.append(hint, list);
    function draw() {
      list.replaceChildren();
      for (const f of SchemaAuthoringCore.search(
        unique.map((f) => ({
          ...f,
          label: f.name || f.id,
          keywords: [
            ...(f.groups || []),
            f.id,
            `${f.universe ?? 0}:${f.address ?? 1}`,
          ].join(" "),
        })),
        input.value,
      )) {
        const b = button(f.label, () => {
          d.close();
          onSelect(f.id, !!event?.shiftKey);
        });
        b.append(
          el(
            "small",
            [...(f.groups || []), `${f.universe ?? 0}:${f.address ?? 1}`].join(
              " · ",
            ),
          ),
        );
        list.append(b);
      }
      if (!list.children.length) list.append(el("p", "No matching fixtures"));
    }
    input.oninput = draw;
    d.onkeydown = (e) => {
      if (e.key === "Enter" && document.activeElement === input) {
        e.preventDefault();
        list.querySelector("button")?.click();
      } else if (e.key === "ArrowDown" && document.activeElement === input) {
        e.preventDefault();
        list.querySelector("button")?.focus();
      }
    };
    draw();
    d.showModal();
    input.focus();
  }
  async function arrange() {
    if (!adapter) {
      latest = await SchemaEngine.request("/api/state");
      adapter = {
        getProject: () => latest.project,
        getRevision: () => latest.snapshot.revision,
        getSelection: () => ({
          fixture: latest.project.fixtures[0]?.id,
          layer: 0,
          node: 0,
        }),
        commit: async (command, expected_revision) => {
          await SchemaEngine.request("/api/command", {
            command,
            expected_revision,
          });
          latest = await SchemaEngine.request("/api/state");
          return true;
        },
      };
    }
    const p = adapter.getProject(),
      selection = selected(),
      ids = new Set(adapter.getSelectedFixtures?.() || [selection.fixture]),
      revision = adapter.getRevision(),
      d = sheet("Arrange fixtures");
    d.append(
      el(
        "p",
        "Move layout positions. Align and distribute only affect the chosen axis; Snap affects X, Y and Z. Each Apply is one undoable edit.",
      ),
    );
    const list = el("div", undefined, "arrange-fixtures");
    const checks = [];
    for (const f of p.fixtures) {
      const check = el("input");
      check.type = "checkbox";
      check.checked = ids.has(f.id);
      check.value = f.id;
      field(list, f.name, check);
      checks.push(check);
    }
    const selectAll = button("Select all", () =>
      checks.forEach((c) => (c.checked = true)),
    );
    d.append(selectAll, list);
    const mode = field(d, "Operation", el("select"));
    for (const [value, text] of [
      ["snap", "Snap to grid"],
      ["align", "Align"],
      ["distribute", "Distribute evenly"],
    ])
      mode.append(new Option(text, value));
    const axis = field(d, "Axis", el("select"));
    ["X", "Y", "Z"].forEach((v, i) => axis.append(new Option(v, i)));
    const anchor = field(d, "Align to", el("select"));
    for (const [value, text] of [
      ["active", "Active fixture"],
      ["min", "Minimum"],
      ["center", "Center"],
      ["max", "Maximum"],
    ])
      anchor.append(new Option(text, value));
    const step = field(d, "Grid spacing (m)", el("input"));
    step.type = "number";
    step.min = ".001";
    step.step = ".1";
    step.value = localStorage.getItem("schema-fixture-grid") || ".5";
    const message = el("p");
    message.setAttribute("role", "alert");
    const apply = button("Apply arrangement", async () => {
      try {
        const plan = SchemaAuthoringCore.arrange(
          p,
          checks.filter((c) => c.checked).map((c) => c.value),
          {
            mode: mode.value,
            axis: Number(axis.value),
            anchor: anchor.value,
            active: selection.fixture,
            step: Number(step.value),
          },
        );
        if (!plan.commands.length) {
          message.textContent = "Fixtures already match this arrangement.";
          return;
        }
        apply.disabled = true;
        if (await run(plan, revision)) {
          localStorage.setItem("schema-fixture-grid", step.value);
          d.close();
        } else
          message.textContent =
            "Scene changed. Close and reopen Arrange to use the latest positions.";
      } catch (e) {
        message.textContent = e.message;
      } finally {
        apply.disabled = false;
      }
    });
    mode.onchange = () => {
      axis.parentElement.hidden = mode.value === "snap";
      anchor.parentElement.hidden = mode.value !== "align";
      step.parentElement.hidden = mode.value !== "snap";
    };
    mode.onchange();
    d.append(message, apply);
    d.showModal();
  }
  async function history() {
    const d = sheet("History and checkpoints"),
      message = el("p"),
      list = el("div", undefined, "authoring-results"),
      checkpoints = el("div", undefined, "authoring-results");
    message.setAttribute("role", "status");
    const name = field(d, "Checkpoint name", el("input"));
    name.maxLength = 80;
    const save = button("Create checkpoint", async () => {
      save.disabled = true;
      if (
        await run(
          { type: "save_checkpoint", name: name.value.trim() },
          revision,
        )
      ) {
        name.value = "";
        await refresh();
      } else
        message.textContent =
          "Could not create checkpoint. Use a unique name and refresh if the scene changed.";
      save.disabled = false;
    });
    d.append(
      save,
      el("h3", "Timeline"),
      list,
      el("h3", "Named checkpoints"),
      checkpoints,
      message,
    );
    let revision;
    async function refresh() {
      try {
        const state = await SchemaEngine.request("/api/history");
        revision = state.revision;
        list.replaceChildren();
        for (const entry of state.entries) {
          const b = button(
            (entry.current ? "● Current · " : entry.offset < 0 ? "← " : "→ ") +
              entry.label,
            async () => {
              b.disabled = true;
              if (
                await run(
                  { type: "history_jump", offset: entry.offset },
                  revision,
                )
              )
                await refresh();
              else
                message.textContent =
                  "History changed. Refresh before choosing another entry.";
            },
          );
          b.disabled = entry.current;
          b.setAttribute("aria-current", String(entry.current));
          list.append(b);
        }
        checkpoints.replaceChildren();
        for (const checkpoint of state.checkpoints) {
          const row = el("div", undefined, "checkpoint-row"),
            restore = button(checkpoint.name, async () => {
              if (
                await run(
                  { type: "restore_checkpoint", name: checkpoint.name },
                  revision,
                )
              )
                await refresh();
            }),
            remove = button("Remove", async () => {
              if (
                await run(
                  { type: "remove_checkpoint", name: checkpoint.name },
                  revision,
                )
              )
                await refresh();
            });
          restore.title =
            "Restore " +
            checkpoint.project +
            " · " +
            checkpoint.fixtures +
            " fixtures. Physical output stays disabled. Undo returns to the current scene.";
          remove.setAttribute(
            "aria-label",
            "Remove checkpoint " + checkpoint.name,
          );
          row.append(restore, remove);
          checkpoints.append(row);
        }
        if (!state.checkpoints.length)
          checkpoints.append(el("p", "No named checkpoints yet."));
        message.textContent =
          "Timeline navigation preserves redo. Restoring a checkpoint is one undoable edit.";
      } catch (e) {
        message.textContent = e.message;
      }
    }
    d.append(button("Refresh history", refresh));
    d.showModal();
    await refresh();
  }
  async function exportBundle() {
    try {
      await window.SchemaAppNavigation?.flush();
      const data = await SchemaEngine.request("/api/projects/bundle/raw"),
        url = URL.createObjectURL(
          new Blob([data], { type: "application/json" }),
        ),
        a = el("a");
      a.href = url;
      a.download =
        (adapter.getProject().name.replace(/[^a-z0-9_-]/gi, "-") || "project") +
        ".schema-bundle.json";
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      notify("Portable bundle exported");
    } catch (e) {
      notify(e.message, true);
    }
  }
  function importBundle() {
    const input = el("input");
    input.type = "file";
    input.accept = ".json,application/json";
    input.onchange = async () => {
      try {
        const file = input.files[0];
        if (!file) return;
        if (file.size > 64 * 1024 * 1024) throw Error("Bundle exceeds 64 MiB");
        const text = await file.text();
        const { document: bundle } = await SchemaEngine.request(
            "/api/storage/migrate",
            { format: "bundle", text },
          ),
          revision = adapter.getRevision(),
          d = sheet("Import portable project");
        d.append(
          el("p", bundle.project?.name || "Unnamed project"),
          el(
            "p",
            `${bundle.project?.fixtures?.length || 0} fixtures · ${Object.keys(bundle.plugins?.packages || {}).length} component packages · ${Object.keys(bundle.resources || {}).length} resource files`,
          ),
          el(
            "p",
            "Import replaces the running scene and keeps Undo available. Physical output starts disabled. Existing resource and component names must match or be renamed.",
          ),
        );
        const message = el("p"),
          apply = button("Import project", async () => {
            apply.disabled = true;
            try {
              await SchemaEngine.request("/api/projects/bundle", {
                text,
                expected_revision: revision,
              });
              await adapter.afterBundle?.();
              await SchemaEngine.request("/api/state");
              d.close();
              notify("Portable project imported · output disabled");
              window.dispatchEvent(new Event("schema-project-imported"));
            } catch (e) {
              message.textContent = e.message;
            } finally {
              apply.disabled = false;
            }
          });
        message.setAttribute("role", "alert");
        d.append(message, apply);
        d.showModal();
      } catch (e) {
        notify(e.message, true);
      }
    };
    input.click();
  }
  function stateHelp() {
    const d = sheet("State and shortcut help");
    for (const [label, kind, description] of [
      [
        "Selected",
        "selected",
        "Blue outline: the current editing target. This does not isolate its output.",
      ],
      [
        "Enabled",
        "live",
        "Mint check: evaluated by the engine. Layer selection is independent of this switch.",
      ],
      [
        "Bypassed",
        "bypassed",
        "Dashed block: its inputs pass through; its operation and property drivers are skipped. Saved and undoable.",
      ],
      [
        "Debug output",
        "debug",
        "Amber outline and banner: a specific block drives fixture output temporarily. The program still evaluates. Stop restores the normal root output.",
      ],
      [
        "Capture only",
        "preview",
        "Output is computed and visible, but no physical transport is enabled.",
      ],
      [
        "Blackout / error",
        "danger",
        "Red signals blackout or an error. It never denotes selection.",
      ],
    ]) {
      const row = el("p");
      row.append(
        el("strong", label, "state-chip state-" + kind),
        document.createTextNode(" " + description),
      );
      d.append(row);
    }
    const table = el("dl");
    for (const [k, v] of [
      ["F2", "Search actions"],
      ["Ctrl/Cmd+K", "Find fixture"],
      ["Ctrl/Cmd+S", "Projects"],
      ["Ctrl/Cmd+Z / Shift+Z", "Undo / Redo"],
      ["Ctrl/Cmd+D", "Duplicate block or branch"],
      ["Ctrl+Space", "Maximize focused panel"],
      ["G", "Fixture placement handles in a scene view"],
      ["Alt+click", "Cycle crowded fixtures in schematic views"],
      ["Space", "Pause / resume clock (outside fields)"],
      ["Escape", "Close the active dialog or menu"],
    ])
      table.append(el("dt", k), el("dd", v));
    d.append(table);
    d.showModal();
  }
  function actions() {
    const click = (id) => () => document.getElementById(id)?.click(),
      result = [];
    for (const [label, id, shortcut, keywords] of [
      ["Projects / Save", "save", "Ctrl+S", "load store"],
      ["Find fixture", "find-fixture", "Ctrl+K", "search name group address"],
      ["Undo", "undo", "Ctrl+Z", "history"],
      ["Redo", "redo", "Ctrl+Shift+Z", "history"],
      [
        "Pause / resume clock",
        document.getElementById("pause") ? "pause" : "play",
        "Space",
        "animation",
      ],
      ["Output panel", "output-design", "", "artnet dmx"],
      ["Connection", "connect", "", "server token"],
      ["Components", "components", "", "plugins reload"],
      ["Saved servers", "servers-button", "", "pair network"],
      ["Toggle light / dark theme", "theme-toggle", "", "appearance"],
      ["Fullscreen", "fullscreen-toggle", "", "display"],
      ["Import ScenicScript", "import", "", "yaml"],
      ["Export ScenicScript", "export", "", "yaml"],
    ]) {
      const n = document.getElementById(id);
      if (n && !n.disabled)
        result.push({ label, shortcut, keywords, run: click(id) });
    }
    result.push(
      {
        label: "Find fixture",
        shortcut: "Ctrl+K",
        keywords: "search names groups addresses",
        run: () => SchemaBlockUI.findFixture(),
      },
      {
        label: "Arrange fixtures",
        keywords: "snap align distribute layout grid",
        run: arrange,
      },
      {
        label: "History and checkpoints",
        keywords: "undo redo restore named snapshot timeline",
        run: history,
      },
      {
        label: "Export portable project bundle",
        keywords: "download resources plugins",
        run: exportBundle,
      },
      {
        label: "Import portable project bundle",
        keywords: "upload resources plugins",
        run: importBundle,
      },
      {
        label: "State and shortcut help",
        keywords: "colors selected enabled bypass debug keyboard",
        run: stateHelp,
      },
    );
    result.push(...(adapter.getActions?.() || []));
    result.push(
      {
        label: "Copy selected branch",
        shortcut: "Ctrl+C",
        run: () => SchemaBlockUI.copy("branch"),
      },
      {
        label: "Cut selected branch",
        shortcut: "Ctrl+X",
        run: () => SchemaBlockUI.copy("branch", true),
      },
      {
        label: "Paste branch",
        shortcut: "Ctrl+V",
        run: () => SchemaBlockUI.paste(),
      },
      {
        label: "Duplicate selected branch",
        shortcut: "Ctrl+D",
        run: () => SchemaBlockUI.duplicate(),
      },
    );
    const n = selectedNode(),
      target = selected(),
      revision = adapter.getRevision();
    if (n) {
      result.push({
        label: n.bypassed ? "Enable selected block" : "Bypass selected block",
        keywords: "compare effect disable",
        run: () =>
          run(
            {
              type: "set_node_bypassed",
              ...target,
              bypassed: !n.bypassed,
            },
            revision,
          ),
      });
      if (selected().fixture !== "__director")
        result.push({
          label: "Focus selected block output",
          keywords: "debug isolate solo preview",
          run: () =>
            run({
              type: "set_debug_focus",
              target: {
                ...target,
              },
            }),
        });
    }
    if (latest?.snapshot.debug_focus)
      result.push({
        label: "Stop debug output focus",
        keywords: "restore normal output",
        run: () => run({ type: "set_debug_focus", target: null }),
      });
    return result.filter(
      (a, i, all) => all.findIndex((b) => b.label === a.label) === i,
    );
  }
  async function open() {
    if (!adapter) return;
    if (!latest?.project) {
      notify("Wait for the scene to connect");
      return;
    }
    try {
      await window.SchemaAppNavigation?.flush();
      for (let i = 0; adapter.pending?.() && i < 100; i++)
        await new Promise((resolve) => setTimeout(resolve, 30));
      if (adapter.pending?.())
        throw Error("Wait for the current edit to finish");
    } catch (error) {
      notify(error.message, true);
      return;
    }
    if (document.querySelector("dialog[open]")) return;
    const d = sheet("Commands"),
      input = field(d, "Search actions", el("input")),
      list = el("div", undefined, "authoring-results");
    input.type = "search";
    input.autocomplete = "off";
    d.append(list);
    let index = 0,
      results = [];
    function draw() {
      results = SchemaAuthoringCore.search(actions(), input.value);
      index = Math.min(index, results.length - 1);
      list.replaceChildren();
      results.forEach((action, i) => {
        const b = button(action.label, () => {
          d.close();
          queueMicrotask(action.run);
        });
        b.dataset.active = String(i === index);
        if (action.shortcut) b.append(el("kbd", action.shortcut));
        list.append(b);
      });
      if (!results.length) list.append(el("p", "No matching actions"));
    }
    input.oninput = () => {
      index = 0;
      draw();
    };
    d.onkeydown = (e) => {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        index =
          (index + (e.key === "ArrowDown" ? 1 : -1) + results.length) %
          Math.max(1, results.length);
        draw();
        list.children[index]?.scrollIntoView({ block: "nearest" });
      } else if (e.key === "Enter" && document.activeElement === input) {
        e.preventDefault();
        list.children[index]?.click();
      }
    };
    draw();
    d.showModal();
    input.focus();
  }
  function configure(value) {
    adapter = value;
    if (paletteButton) return;
    const actions = document.querySelector(".header-actions,.app-menu-panel");
    if (actions) {
      paletteButton = button("Commands", open);
      paletteButton.id = "command-palette";
      paletteButton.disabled = true;
      paletteButton.title = "Search actions (F2)";
      paletteButton.setAttribute("aria-keyshortcuts", "F2");
      actions.append(paletteButton);
    }
    const footer = document.querySelector("body>footer"),
      caption = document.querySelector(".screen-caption");
    if (footer || caption) {
      bar = el("span", undefined, "authoring-state");
      bar.setAttribute("aria-label", "Engine state");
      if (caption) caption.replaceChildren(bar);
      else footer.prepend(bar);
    }
    document.addEventListener(
      "keydown",
      (event) => {
        if (
          event.key === "F2" &&
          !event.ctrlKey &&
          !event.metaKey &&
          !event.altKey &&
          !event.shiftKey &&
          !event.repeat &&
          !event.isComposing
        ) {
          event.preventDefault();
          event.stopImmediatePropagation();
          if (!document.querySelector("dialog[open]")) open();
        }
      },
      true,
    );
  }
  return {
    configure,
    update,
    blockControls,
    decorateBlock,
    choose,
    arrange,
    history,
    open,
    exportBundle,
    importBundle,
    stateHelp,
  };
})();
