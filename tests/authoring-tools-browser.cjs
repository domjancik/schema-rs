"use strict";
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8833";
if (["", "8787", "8788"].includes(new URL(base).port))
  throw Error("Use an isolated output-disabled engine");
const state = () => fetch(base + "/api/state").then((r) => r.json());
async function edit(command) {
  const before = await state();
  const r = await fetch(base + "/api/command", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      expected_revision: before.snapshot.revision,
      command,
    }),
  });
  assert(r.ok, await r.text());
}
async function wait(check) {
  for (let i = 0; i < 120; i++) {
    const s = await state();
    if (check(s)) return s;
    await new Promise((r) => setTimeout(r, 30));
  }
  throw Error("Engine publication did not match");
}
(async () => {
  assert.equal((await state()).project.output.enabled, false);
  const existing = await fetch(base + "/api/history").then((r) => r.json());
  if (existing.checkpoints.some((c) => c.name === "Before arrange"))
    await edit({ type: "remove_checkpoint", name: "Before arrange" });
  const initial = await state();
  await edit({
    type: "replace_layer",
    fixture: "fixture-1",
    layer: 0,
    stack: {
      name: "Comparison",
      root: {
        id: 1,
        behavior: "Modify.Value.Invert",
        properties: {},
        children: [
          {
            id: 2,
            behavior: "Create.Value.Constant",
            properties: { Value: 0.25 },
            children: [],
            drivers: {},
          },
        ],
        drivers: {},
      },
    },
  });
  await edit({
    type: "move_fixture",
    fixture: "fixture-2",
    position: initial.project.fixtures[0].position,
  });
  const b = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const p = await b.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
      }),
      errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(base + "/?view=studio");
    await p.locator(".workspace-panel:visible").waitFor();
    async function action(query, label) {
      await p.waitForFunction(() => !document.querySelector("dialog[open]"));
      await p.waitForFunction(
        () => !document.querySelector("#command-palette")?.disabled,
      );
      await p.keyboard.press("Control+Shift+P");
      const d = p.getByRole("dialog", { name: "Commands", exact: true });
      await d.getByLabel("Search actions", { exact: true }).fill(query);
      await d.getByRole("button", { name: label, exact: true }).click();
    }
    await action("byp sel", "Bypass selected block");
    await wait((s) => s.project.content["fixture-1"][0].root.bypassed === true);
    await p.locator('.block[data-node="1"].is-bypassed').waitFor();
    assert.equal((await state()).snapshot.fixtures[0].result.value, 0.25);
    await action("enable sel", "Enable selected block");
    await wait((s) => !s.project.content["fixture-1"][0].root.bypassed);
    await action("focus sel", "Focus selected block output");
    await wait((s) => s.snapshot.debug_focus?.node === 1);
    await p.locator(".authoring-state .state-debug").waitFor();
    assert.equal(
      (await state()).project.content["fixture-1"][0].root.bypassed,
      undefined,
    );
    await action("stop debug", "Stop debug output focus");
    await wait((s) => !s.snapshot.debug_focus);
    await action("checkpoint", "History and checkpoints");
    let h = p.getByRole("dialog", {
      name: "History and checkpoints",
      exact: true,
    });
    await h
      .getByLabel("Checkpoint name", { exact: true })
      .fill("Before arrange");
    await h
      .getByRole("button", { name: "Create checkpoint", exact: true })
      .click();
    await h
      .getByRole("button", { name: "Before arrange", exact: true })
      .waitFor();
    await h
      .getByRole("button", {
        name: "Close History and checkpoints",
        exact: true,
      })
      .click();
    const beforeArrange = await state();
    await action("arrange", "Arrange fixtures");
    const arrange = p.getByRole("dialog", {
      name: "Arrange fixtures",
      exact: true,
    });
    await arrange
      .getByRole("button", { name: "Select all", exact: true })
      .click();
    await arrange
      .getByLabel("Operation", { exact: true })
      .selectOption("align");
    await arrange.getByLabel("Axis", { exact: true }).selectOption("0");
    await arrange
      .getByLabel("Align to", { exact: true })
      .selectOption("center");
    await arrange
      .getByRole("button", { name: "Apply arrangement", exact: true })
      .click();
    await wait(
      (s) => s.snapshot.revision === beforeArrange.snapshot.revision + 1,
    );
    assert.equal(
      new Set((await state()).project.fixtures.map((f) => f.position[0])).size,
      1,
    );
    await edit({ type: "undo" });
    await wait(
      (s) =>
        s.project.fixtures[0].position[0] ===
        beforeArrange.project.fixtures[0].position[0],
    );
    const picker = p.getByRole("combobox", {
      name: "Studio panel",
      exact: true,
    });
    await picker.selectOption(
      await picker
        .locator("option")
        .filter({ hasText: "Fixture setup" })
        .first()
        .getAttribute("value"),
    );
    await p.locator('#stage [data-fixture="fixture-1"]').click({ force: true });
    const choose = p.getByRole("dialog", {
      name: "Choose fixture",
      exact: true,
    });
    await choose
      .getByLabel("Find overlapping fixture", { exact: true })
      .fill("Wash 02");
    await choose.getByRole("button", { name: /Wash 02/ }).click();
    await picker.selectOption(
      await picker
        .locator("option")
        .filter({ hasText: "Stack editor" })
        .first()
        .getAttribute("value"),
    );
    await action("state shortcut", "State and shortcut help");
    assert(
      await p
        .getByRole("dialog", { name: "State and shortcut help", exact: true })
        .getByText("Ctrl/Cmd+Shift+P", { exact: true })
        .isVisible(),
    );
    await p.keyboard.press("Escape");
    const bundle = await fetch(base + "/api/projects/bundle").then((r) =>
      r.json(),
    );
    bundle.project.name = "Portable authored scene";
    const manifest = JSON.parse(
      fs.readFileSync("examples/plugins/gain/plugin.json", "utf8"),
    );
    bundle.plugins.packages.demo = { folder: "gain", enabled: true, manifest };
    bundle.resources["example.bin"] = [1, 2, 3];
    const chooser = p.waitForEvent("filechooser");
    await action("import portable", "Import portable project bundle");
    await (
      await chooser
    ).setFiles({
      name: "example.schema-bundle.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify(bundle)),
    });
    await p
      .getByRole("dialog", { name: "Import portable project", exact: true })
      .getByRole("button", { name: "Import project", exact: true })
      .click();
    await wait((s) => s.project.name === "Portable authored scene");
    assert.equal((await state()).project.output.enabled, false);
    assert(
      (await fetch(base + "/api/catalog").then((r) => r.json())).some((row) =>
        row.key.includes("demo.Gain"),
      ),
    );
    for (const size of [
      { width: 320, height: 568 },
      { width: 844, height: 390 },
    ]) {
      await p.setViewportSize(size);
      await action("history", "History and checkpoints");
      h = p.getByRole("dialog", {
        name: "History and checkpoints",
        exact: true,
      });
      const box = await h.boundingBox();
      assert(
        box.x >= 0 &&
          box.y >= 0 &&
          box.x + box.width <= size.width + 1 &&
          box.y + box.height <= size.height + 1,
      );
      await p.keyboard.press("Escape");
      await h.waitFor({ state: "detached" });
      assert(
        await p.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      );
    }
    assert.deepEqual(errors, []);
    console.log(
      "Studio Commands, bypass/debug, arrange/undo, crowded picking, history/checkpoints, bundle resources/plugins and phone bounds passed.",
    );
    const mobile = await b.newPage({
      viewport: { width: 390, height: 844 },
      hasTouch: true,
    });
    await mobile.goto(base + "/mobile");
    await mobile
      .locator("#command-palette:not([disabled])")
      .waitFor({ state: "attached" });
    await mobile.keyboard.press("Control+Shift+P");
    await mobile
      .getByRole("dialog", { name: "Commands", exact: true })
      .waitFor();
    await mobile.keyboard.press("Escape");
    await mobile.locator("#selection-meta").click();
    await mobile.locator(".block-state-controls").waitFor();
    console.log(
      "Shared schémaboy command palette and block state controls passed.",
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
