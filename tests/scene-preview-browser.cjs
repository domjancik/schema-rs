"use strict";
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8788";
(async () => {
  const channel = process.env.SCHEMA_BROWSER_CHANNEL,
    launch = channel
      ? {
          channel,
          args: [
            "--enable-unsafe-webgpu",
            "--ignore-gpu-blocklist",
            "--use-angle=d3d11",
          ],
        }
      : {
          executablePath: process.env.CHROMIUM_EXECUTABLE,
        };
  const browser = await chromium.launch({ headless: true, ...launch });
  try {
    const page = await browser.newPage({
        viewport: { width: 1200, height: 900 },
      }),
      errors = [],
      commands = [];
    page.on("pageerror", (e) => errors.push(e.message));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    const publication = await page.request
      .get(base + "/api/state")
      .then((r) => r.json());
    await page.route("**/api/state", (route) =>
      route.fulfill({ json: publication }),
    );
    let allowCommand = false;
    await page.route("**/api/command", (route) => {
      assert(allowCommand, "Read-only preview must not issue engine commands");
      commands.push(route.request().postDataJSON());
      return route.fulfill({
        json: { revision: publication.snapshot.revision + 1 },
      });
    });
    await page.goto(base + "/preview?quality=low");
    await page.waitForFunction(
      () =>
        Number(document.querySelector("#preview-stats")?.dataset.frames) > 3,
      null,
      { timeout: 60000 },
    );
    const firstBackend = await page
      .locator("#preview-stats")
      .getAttribute("data-backend");
    if (process.env.SCHEMA_EXPECT_BACKEND)
      assert.equal(firstBackend, process.env.SCHEMA_EXPECT_BACKEND);
    await page.locator("#preview-source").selectOption("carpushit");
    await page.waitForFunction(
      () =>
        document
          .querySelector("#preview-stats")
          .textContent.includes("38 fixtures"),
      null,
      { timeout: 60000 },
    );
    assert.equal(commands.length, 0);
    assert(
      await page
        .locator("#preview-mode")
        .textContent()
        .then((text) => text.includes("preview only")),
    );
    await page.locator("#preview-load-preset").click();
    await page.locator("#preview-review").waitFor({ state: "visible" });
    await page.locator("#preview-cancel-load").click();
    assert.equal(commands.length, 0);
    for (const id of ["preview-gi", "preview-volumes"]) {
      await page.locator("#" + id).uncheck();
      await page.waitForTimeout(350);
      await page.locator("#" + id).check();
    }
    await page.waitForTimeout(1800);
    await page.screenshot({
      path: `target/gpu-verified-${firstBackend.toLowerCase()}.png`,
    });
    await page.locator("#preview-model").setInputFiles({
      name: "invalid.glb",
      mimeType: "model/gltf-binary",
      buffer: Buffer.from("invalid"),
    });
    await page.locator("#preview-loading").waitFor({ state: "visible" });
    await page.locator("#preview-reset-model").click();
    await page
      .locator("#preview-model")
      .setInputFiles("web/scene-assets/carpushit.glb");
    await page
      .locator("#preview-loading")
      .waitFor({ state: "hidden", timeout: 30000 });
    await page.locator("#preview-backend").selectOption("webgl");
    await page.waitForFunction(
      () =>
        document.querySelector("#preview-stats")?.dataset.backend ===
          "WebGL2" &&
        Number(document.querySelector("#preview-stats")?.dataset.frames) > 3,
      null,
      { timeout: 60000 },
    );
    assert.equal(commands.length, 0);
    await page.locator("#preview-load-preset").click();
    allowCommand = true;
    await page.locator("#preview-confirm-load").click();
    await page.waitForFunction(
      () => document.querySelector("#preview-source").value === "live",
    );
    assert.equal(commands.length, 1);
    assert.equal(commands[0].expected_revision, publication.snapshot.revision);
    assert.equal(commands[0].command.type, "replace_project");
    assert.equal(commands[0].command.project.fixtures.length, 38);
    assert.equal(commands[0].command.project.output.enabled, false);
    // The standalone page stays open; exercise modal close/reopen in Studio.
    await page.getByRole('button', { name: 'Return to Studio', exact: true }).click();
    await page.waitForURL(base + '/?view=studio');
    await page.evaluate(() => SchemaScenePreview.open());
    await page.waitForFunction(() => Number(document.querySelector('#preview-stats')?.dataset.frames) > 1);
    await page.locator("#preview-close").click();
    const frames = await page
      .locator("#preview-stats")
      .getAttribute("data-frames");
    await page.waitForTimeout(500);
    assert.equal(
      await page.locator("#preview-stats").getAttribute("data-frames"),
      frames,
      "Closed preview stops rendering/publishing",
    );
    await page.evaluate(() => SchemaScenePreview.open());
    await page.locator("#scene-preview-dialog").waitFor({ state: "visible" });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForTimeout(300);
    const box = await page.locator(".preview-viewport").boundingBox();
    assert(
      box.height >= 100 && box.x >= 0 && box.x + box.width <= 390,
      "Phone preview stays within viewport",
    );
    assert.deepEqual(errors, []);
    fs.writeFileSync(
      "target/gpu-browser-verification-" + firstBackend + ".json",
      JSON.stringify(
        {
          firstBackend,
          forcedFallback: "WebGL2",
          fixtures: 38,
          readOnlyPreset: true,
          explicitLoadCommands: commands.length,
          outputDisabled: true,
          errors,
        },
        null,
        2,
      ),
    );
    console.log(
      firstBackend +
        ": Carpushit, WebGL2 switch, effects, GLB import/recovery, explicit revisioned load and resource lifecycle passed.",
    );
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
