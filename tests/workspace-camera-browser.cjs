"use strict";
const assert = require("node:assert/strict");
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
(async () => {
  const browser = await chromium.launch({
    channel: "chrome",
    headless: true,
    args: [
      "--enable-unsafe-webgpu",
      "--ignore-gpu-blocklist",
      "--use-angle=d3d11",
    ],
  });
  try {
    const page = await browser.newPage({
        viewport: { width: 1500, height: 950 },
      }),
      publication = await fetch(base + "/api/state").then((r) => r.json()),
      errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.route("**/api/state", (r) => r.fulfill({ json: publication }));
    await page.route("**/api/command", () => {
      throw Error("Camera workspace must remain read-only");
    });
    await page.goto(base + "/?view=studio");
    await page
      .locator(".workspace-panel[data-view=setup]")
      .getByRole("combobox", { name: "View type", exact: true })
      .selectOption("gpu");
    await page
      .locator(".workspace-panel[data-view=stack]")
      .getByRole("combobox", { name: "View type", exact: true })
      .selectOption("gpu");
    await page.locator("iframe.panel-preview").nth(1).waitFor();
    const frames = await Promise.all(
      (await page.locator("iframe.panel-preview").elementHandles()).map((h) =>
        h.contentFrame(),
      ),
    );
    for(const frame of frames){try{await frame.waitForFunction(()=>Number(document.querySelector('#preview-stats')?.dataset.frames)>1,{timeout:30000});}catch(error){console.log('Frame startup failure',frame.url(),await frame.locator('body').innerText());throw error;}}
    for (const frame of frames) {
      assert.equal(await frame.locator('#scene-preview').count(), 0, 'Embedded scene has no intermediate opening screen');
      assert.equal(await frame.locator('a[href]').count(), 0, 'Embedded scene must not navigate Studio inside its iframe');
      assert.equal(await frame.locator('#preview-close').isVisible(), false, 'A workspace view is changed through the panel controls');
      assert.equal(await frame.locator('#scene-preview-dialog').evaluate(e => e.matches(':modal')), false, 'Embedded scene is a persistent panel, not a modal');
      await frame.locator('#preview-canvas').focus();
      await frame.press('#preview-canvas', 'Escape');
      assert.equal(await frame.locator('#scene-preview-dialog').isVisible(), true, 'Escape cannot expose an opening screen');
    }
    const pose = (frame) =>
      frame
        .locator("#preview-camera-status")
        .evaluate((e) => JSON.parse(e.dataset.pose));
    const second = await pose(frames[1]);
    const menu = frames[0].locator(".preview-navigation");
    if (!(await menu.evaluate((e) => e.open)))
      await menu.locator("summary").click();
    await frames[0]
      .getByRole("button", { name: "Camera right", exact: true })
      .click();
    assert.deepEqual(
      await pose(frames[1]),
      second,
      "Duplicate 3D views must retain independent cameras",
    );
    const original = await page
      .locator("iframe.panel-preview")
      .first()
      .elementHandle();
    await frames[0].locator("#preview-canvas").focus();
    const before = await frames[1]
      .locator("#preview-stats")
      .getAttribute("data-frames");
    await frames[0].press("#preview-canvas", "Control+Space");
    await page.locator(".workspace-panel.maximized").waitFor();
    assert.equal(
      await original.evaluate((e) => e.isConnected),
      true,
      "Maximize must not recreate GPU view",
    );
    await page.waitForTimeout(300);
    const paused = await frames[1]
      .locator("#preview-stats")
      .getAttribute("data-frames");
    assert.ok(
      Number(paused) <= Number(before) + 3,
      "Hidden preview rendering should pause",
    );
    await frames[0].press("#preview-canvas", "Escape");
    await page.waitForFunction(
      () => !document.querySelector(".workspace-panel.maximized"),
    );
    assert.equal(await original.evaluate((e) => e.isConnected), true);
    await page.goto(base + '/preview?quality=low&backend=webgl');
    await page.getByRole('button', { name: 'Return to Studio', exact: true }).waitFor();
    assert.equal(await page.locator('#scene-preview-dialog').evaluate(e => e.matches(':modal')), false);
    await page.getByRole('button', { name: 'Return to Studio', exact: true }).click();
    await page.waitForURL(base + '/?view=studio');
    await page.locator('.workspace-panel').first().waitFor();
    assert.deepEqual(errors, []);
    console.log(
      "Independent GPU cameras, embedded maximize/restore, renderer preservation and hidden-preview pause passed.",
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
