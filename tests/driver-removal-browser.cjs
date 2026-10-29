"use strict";
const assert = require("node:assert/strict");
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
if (["8787", "8788", ""].includes(new URL(base).port)) throw Error("Use an isolated engine");
const leaf = (id, value) => ({ id, behavior: "Create.Value.Constant", properties: { Value: value }, children: [], drivers: {} });
const stack = { name: "Driver removal", root: {
  id: 1, behavior: "Modify.Value.MapTo", properties: { Minimum: 0.2, Maximum: 1 },
  children: [leaf(2, 0.5)], drivers: { Minimum: {
    id: 3, behavior: "Modify.Value.MapTo", properties: { Minimum: 0.1, Maximum: 1 },
    children: [leaf(4, 0.6)], drivers: { Minimum: leaf(5, 0.3) }
  } }
} };
(async () => {
  const state = () => fetch(base + "/api/state").then(r => r.json());
  async function edit(command) {
    const current = await state();
    const response = await fetch(base + "/api/command", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ expected_revision: current.snapshot.revision, command }) });
    assert.equal(response.status, 200, await response.text());
  }
  assert.equal((await state()).project.output.enabled, false);
  await edit({ type: "replace_layer", fixture: "fixture-1", layer: 0, stack });
  const before = (await state()).project;
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  const errors = [];
  async function open(route, mobile = false) {
    const context = await browser.newContext({ viewport: mobile ? { width: 390, height: 900 } : { width: 1550, height: 1000 }, hasTouch: mobile, reducedMotion: "reduce" });
    const page = await context.newPage();
    page.setDefaultTimeout(6000);
    page.on("pageerror", e => errors.push(e.message));
    await page.goto(base + route);
    return page;
  }
  async function undo(page) {
    await page.locator("#undo").click();
    await page.waitForFunction(async expected => JSON.stringify((await fetch("/api/state").then(r => r.json())).project) === JSON.stringify(expected), before);
    assert.deepEqual((await state()).project, before);
  }
  async function removed(page) {
    await page.waitForFunction(async () => !(await fetch("/api/state").then(r => r.json())).project.content["fixture-1"][0].root.drivers.Minimum);
    const root = (await state()).project.content["fixture-1"][0].root;
    assert.equal(root.properties.Minimum, 0.2);
    assert.deepEqual(root.children, stack.root.children);
  }
  try {
    let page = await open("/?view=studio");
    await page.locator('#world [data-node="1"]').click();
    await page.getByRole("button", { name: "Remove Minimum driver", exact: true }).click();
    await removed(page); await undo(page);
    await page.locator('#inspector [title="Edit driver"]').click();
    await page.getByRole("button", { name: "Remove current driver", exact: true }).click();
    await removed(page); await undo(page);
    // Embedded scene stacks use the same inspector actions on their selected owner.
    await page.locator('.workspace-panel[data-view=setup] [aria-label="View type"]').selectOption("scene");
    const panel = page.locator('.workspace-panel[data-view=scene]').first();
    await panel.getByLabel("Stacks in scene", { exact: true }).selectOption("selected");
    await panel.locator('[data-scene-stack-fixture="fixture-1"] [data-node="1"] strong').click();
    await page.getByRole("button", { name: "Remove Minimum driver", exact: true }).click();
    await removed(page); await undo(page);
    await page.context().close();
    for (const view of ["3d", "linear", "stack"]) {
      page = await open("/mobile", true);
      await page.locator(`[data-view="${view}"]`).click();
      await page.getByRole("button", { name: "Open selected block inspector" }).click();
      await page.locator("#menu").getByRole("button", { name: "Minimum", exact: true }).click();
      await page.locator("#menu").getByRole("button", { name: "Remove driver", exact: true }).click();
      await removed(page); await undo(page);
      await page.context().close();
    }
    page = await open("/mobile", true);
    await page.getByRole("button", { name: "Open selected block inspector" }).click();
    await page.locator("#menu").getByRole("button", { name: "Minimum", exact: true }).click();
    await page.locator("#menu").getByRole("button", { name: "Edit driver", exact: true }).click();
    await page.getByRole("button", { name: "Open selected block inspector" }).click();
    await page.locator("#menu").getByRole("button", { name: "Remove current driver", exact: true }).click();
    await removed(page); await undo(page);
    // Deleting a driver root removes its attachment, never the owner's layer.
    await page.getByRole("button", { name: "Open selected block inspector" }).click();
    await page.locator("#menu").getByRole("button", { name: "Minimum", exact: true }).click();
    await page.locator("#menu").getByRole("button", { name: "Edit driver", exact: true }).click();
    await page.locator("#action-b").click({ modifiers: ["Shift"] });
    await page.locator("#menu").getByRole("button", { name: "Delete full branch", exact: true }).click();
    await removed(page); await undo(page);
    // Removing a nested driver returns to its surviving parent driver.
    async function enterMobileDriver() {
      await page.getByRole("button", { name: "Open selected block inspector" }).click();
      await page.locator("#menu").getByRole("button", { name: "Minimum", exact: true }).click();
      await page.locator("#menu").getByRole("button", { name: "Edit driver", exact: true }).click();
    }
    await enterMobileDriver();
    await enterMobileDriver();
    await page.getByRole("button", { name: "Open selected block inspector" }).click();
    await page.locator("#menu").getByRole("button", { name: "Remove current driver", exact: true }).click();
    await page.waitForFunction(async () => !(await fetch("/api/state").then(r => r.json())).project.content["fixture-1"][0].root.drivers.Minimum.drivers.Minimum);
    assert.equal((await state()).project.content["fixture-1"][0].root.drivers.Minimum.id, 3);
    assert.equal(await page.locator("#selection-meta").textContent(), "DRIVER / Minimum");
    await undo(page);
    // Driver-root block-only deletion promotes its input; the owner survives.
    await page.getByRole("button", { name: "Open selected block inspector" }).click();
    await page.locator("#menu").getByRole("button", { name: "Delete…", exact: true }).click();
    await page.locator("#menu").getByRole("button", { name: "Delete block only", exact: true }).click();
    await page.waitForFunction(async () => (await fetch("/api/state").then(r => r.json())).project.content["fixture-1"][0].root.drivers.Minimum.id === 4);
    assert.equal((await state()).project.content["fixture-1"][0].root.id, 1);
    await undo(page);
    await page.locator('button[data-view="stack"]').click();
    await page.locator('#scene [data-node="4"]').click();
    await page.locator("#action-b").click({ modifiers: ["Shift"] });
    await page.locator("#menu").getByRole("button", { name: "Delete full branch", exact: true }).click();
    await page.waitForFunction(async () => (await fetch("/api/state").then(r => r.json())).project.content["fixture-1"][0].root.drivers.Minimum.children.length === 0);
    assert.equal((await state()).project.content["fixture-1"][0].root.drivers.Minimum.id, 3);
    await undo(page);
    assert.deepEqual(errors, []);
    console.log("Driver removal: Studio, driver editor, embedded stacks, all Schemaboy views, stored values and one-step undo passed.");
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
