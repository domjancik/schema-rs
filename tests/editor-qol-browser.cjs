"use strict";
const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
if (["8787", "8788", ""].includes(new URL(base).port))
  throw Error("Use isolated engine");
(async () => {
  const state = () => fetch(base + "/api/state").then((r) => r.json());
  async function edit(command) {
    const s = await state();
    const r = await fetch(base + "/api/command", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ expected_revision: s.snapshot.revision, command }),
    });
    assert.equal(r.status, 200, await r.text());
  }
  assert.equal((await state()).project.output.enabled, false);
  if ((await state()).project.fixtures.some((f) => f.id === "qol-added"))
    await edit({ type: "remove_fixture", fixture: "qol-added" });
  const stack = {
    name: "QoL test",
    root: {
      id: 1,
      behavior: "Combine.Add",
      properties: {},
      drivers: {},
      children: [
        {
          id: 2,
          behavior: "Create.Value.Constant",
          properties: { Value: 0.37 },
          children: [],
          drivers: {},
        },
      ],
    },
  };
  await edit({ type: "replace_layer", fixture: "fixture-1", layer: 0, stack });
  const before = (await state()).project;
  const b = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const p = await b.newPage({
      viewport: { width: 1500, height: 1000 },
      reducedMotion: "reduce",
    });
    p.setDefaultTimeout(6000);
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(base + "/?view=studio");
    async function undo() {
      await p.locator("#undo").click();
      await p.waitForFunction(
        async (expected) =>
          JSON.stringify(
            (await fetch("/api/state").then((r) => r.json())).project,
          ) === JSON.stringify(expected),
        before,
      );
    }
    await p.locator('#world [data-node="2"] strong').click();
    await p
      .getByRole("button", { name: "Duplicate branch", exact: true })
      .click();
    await p.waitForFunction(
      () => ui.data.project.content["fixture-1"][0].root.children.length === 2,
    );
    await undo();
    await p.locator('#world [data-node="2"] strong').click();
    await p
      .getByRole("button", { name: "Reset Value to default", exact: true })
      .click();
    await p.waitForFunction(
      () =>
        ui.data.project.content["fixture-1"][0].root.children[0].properties
          .Value !== 0.37,
    );
    assert.equal(
      (await state()).project.content["fixture-1"][0].root.children[0]
        .properties.Value,
      await p.evaluate(
        () =>
          ui.catalog.find((b) => b.key === "Create.Value.Constant").defaults
            .Value,
      ),
    );
    await undo();
    await p.locator('#world [data-node="2"] strong').click();
    await p.keyboard.press("Control+d");
    await p.waitForFunction(
      () => ui.data.project.content["fixture-1"][0].root.children.length === 2,
    );
    await undo();
    // Duplicating a root creates its own layer instead of changing its output.
    await p.locator('#world [data-node="1"] strong').click();
    await p
      .getByRole("button", { name: "Duplicate branch", exact: true })
      .click();
    await p.waitForFunction(
      () => ui.layer === 3 && ui.data.project.content["fixture-1"].length === 4,
    );
    await undo();
    await p.waitForFunction(() => ui.layer === 0);
    await p.getByRole("button", { name: "Find fixture", exact: true }).click();
    const finder = p.getByRole("dialog", { name: "Find fixture", exact: true });
    await finder.getByRole("searchbox").fill("fixture-2");
    await finder
      .getByRole("button", { name: /fixture-2/ })
      .first()
      .click();
    await p.waitForFunction(() => ui.fixture === "fixture-2");
    assert.deepEqual((await state()).project, before);
    await p.keyboard.press("Control+k");
    await finder.waitFor();
    await p.keyboard.press("Escape");
    await p.keyboard.press("Control+s");
    await p.locator("#projects-dialog[open]").waitFor();
    await p.waitForFunction(
      () =>
        !document.querySelector(
          '#projects-dialog [aria-label="Close projects"]',
        ).disabled,
    );
    await p.keyboard.press("Escape");
    await p.locator("#projects-dialog").waitFor({ state: "hidden" });
    const added = {
      ...before.fixtures[0],
      id: "qol-added",
      name: "Newly added wash",
      address: 450,
    };
    await edit({ type: "add_fixture", fixture: added });
    await p
      .locator('select[aria-label="Panel fixture"] option[value="qol-added"]')
      .first()
      .waitFor({ state: "attached" });
    await undo();
    await p.waitForFunction(
      () =>
        !document.querySelector(
          'select[aria-label="Panel fixture"] option[value="qol-added"]',
        ),
    );
    const renamed = structuredClone(before);
    renamed.fixtures[0].name = "Renamed project fixture";
    await edit({ type: "replace_project", project: renamed });
    await p.waitForFunction(() =>
      [
        ...document.querySelector('select[aria-label="Panel fixture"]').options,
      ].some((o) => o.textContent === "Renamed project fixture"),
    );
    await undo();
    await p.waitForFunction(
      () =>
        ![
          ...document.querySelector('select[aria-label="Panel fixture"]')
            .options,
        ].some((o) => o.textContent === "Renamed project fixture"),
    );
    await p.keyboard.press("Control+k");
    await finder.getByRole("searchbox").fill("fixture-1");
    await finder
      .getByRole("button", { name: /fixture-1/ })
      .first()
      .click();
    // Editing text must not duplicate a branch.
    await p.locator('#world [data-node="1"] strong').click();
    await p.locator('#world [data-node="2"] strong').click();
    const input = p.locator('#inspector input[aria-label="Value"]');
    await input.focus();
    await p.waitForTimeout(250);
    assert.equal(
      await input.evaluate((el) => document.activeElement === el),
      true,
    );
    const revision = (await state()).snapshot.revision;
    await p.keyboard.press("Control+d");
    await p.waitForTimeout(200);
    assert.equal((await state()).snapshot.revision, revision);
    await input.fill(".61");
    await p.keyboard.press("Control+s");
    await p.locator("#projects-dialog[open]").waitFor();
    await p.waitForFunction(
      () =>
        !document.querySelector(
          '#projects-dialog [aria-label="Close projects"]',
        ).disabled,
    );
    assert.equal(
      (await state()).project.content["fixture-1"][0].root.children[0]
        .properties.Value,
      0.61,
    );
    await p.keyboard.press("Escape");
    await p.locator("#projects-dialog").waitFor({ state: "hidden" });
    await undo();
    await p.context().close();
    const phone = await b.newPage({
      viewport: { width: 390, height: 900 },
      hasTouch: true,
    });
    phone.setDefaultTimeout(6000);
    phone.on("pageerror", (e) => errors.push(e.message));
    await phone.goto(base + "/mobile");
    await phone
      .getByRole("button", { name: "Open selected block inspector" })
      .click();
    await phone
      .locator("#menu")
      .getByRole("button", { name: "Duplicate branch", exact: true })
      .click();
    await phone.waitForFunction(
      async () =>
        (await fetch("/api/state").then((r) => r.json())).project.content[
          "fixture-1"
        ].length === 4,
    );
    await phone.locator("#undo").click();
    await phone.waitForFunction(
      async (expected) =>
        JSON.stringify(
          (await fetch("/api/state").then((r) => r.json())).project,
        ) === JSON.stringify(expected),
      before,
    );
    await phone.locator('button[data-view="stack"]').click();
    await phone.locator('#scene [data-node="2"]').click();
    await phone
      .getByRole("button", { name: "Open selected block inspector" })
      .click();
    await phone
      .locator("#menu")
      .getByRole("button", { name: "Value", exact: true })
      .click();
    await phone
      .locator("#menu")
      .getByRole("button", { name: "Reset to default", exact: true })
      .click();
    await phone.waitForFunction(
      async () =>
        (await fetch("/api/state").then((r) => r.json())).project.content[
          "fixture-1"
        ][0].root.children[0].properties.Value !== 0.37,
    );
    await phone.locator("#undo").click();
    await phone.waitForFunction(
      async (expected) =>
        JSON.stringify(
          (await fetch("/api/state").then((r) => r.json())).project,
        ) === JSON.stringify(expected),
      before,
    );
    await phone.getByLabel("More options", { exact: true }).click();
    await phone
      .getByRole("button", { name: "Find fixture", exact: true })
      .click();
    await phone
      .getByRole("dialog", { name: "Find fixture" })
      .getByRole("searchbox")
      .fill("fixture-2");
    await phone
      .getByRole("dialog", { name: "Find fixture" })
      .getByRole("button", { name: /fixture-2/ })
      .first()
      .click();
    assert.deepEqual((await state()).project, before);
    assert.deepEqual(errors, []);
    console.log(
      "QoL: duplicate branches/root layers, reset values, fixture finder, project shortcut, refreshed panel options and undo passed.",
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
