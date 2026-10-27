const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
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
      viewport: { width: 850, height: 650 },
    });
    await page.addInitScript(() => {
      const original = WebGL2RenderingContext.prototype.getExtension;
      WebGL2RenderingContext.prototype.getExtension = function (name) {
        return name === "EXT_color_buffer_float"
          ? null
          : original.call(this, name);
      };
    });
    await page.route("**/api/command", (r) => r.abort());
    await page.goto(
      (process.env.SCHEMA_URL || "http://127.0.0.1:8788") +
        "/preview?backend=webgl&quality=low",
    );
    await page.waitForFunction(
      () =>
        Number(document.querySelector("#preview-stats")?.dataset.frames) > 3,
      null,
      { timeout: 30000 },
    );
    assert(await page.locator("#preview-gi").isDisabled());
    assert(await page.locator("#preview-volumes").isDisabled());
    assert.match(
      await page.locator("#preview-stats").textContent(),
      /basic lighting/,
    );
    await page
      .locator("#preview-canvas")
      .evaluate((canvas) =>
        canvas
          .getContext("webgl2")
          .getExtension("WEBGL_lose_context")
          .loseContext(),
      );
    await page.waitForFunction(() =>
      document
        .querySelector("#preview-loading")
        ?.textContent.includes("GPU context lost"),
    );
    await page.locator("#preview-close").click();
    await page.locator("#scene-preview").click();
    await page.waitForFunction(
      () =>
        Number(document.querySelector("#preview-stats")?.dataset.frames) > 3,
      null,
      { timeout: 30000 },
    );
    console.log(
      "Missing float-target support keeps basic lighting; lost WebGL context recovers by reopening.",
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
