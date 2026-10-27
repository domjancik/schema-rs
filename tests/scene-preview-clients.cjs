const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
(async () => {
  const b = await chromium.launch({
    channel: "chrome",
    headless: true,
    args: [
      "--enable-unsafe-webgpu",
      "--ignore-gpu-blocklist",
      "--use-angle=d3d11",
    ],
  });
  try {
    for (const [route, width, height] of [
      ["/?view=studio", 1440, 1000],
      ["/mobile", 390, 844],
      ["/mobile", 844, 390],
    ]) {
      const p = await b.newPage({ viewport: { width, height } }),
        errors = [];
      p.on("pageerror", (e) => errors.push(e.message));
      await p.route("**/api/command", (r) => r.abort());
      await p.goto("http://127.0.0.1:8788" + route);
      await p.locator("#scene-preview").click();
      await p.waitForFunction(
        () =>
          Number(document.querySelector("#preview-stats")?.dataset.frames) > 2,
        null,
        { timeout: 60000 },
      );
      const box = await p.locator(".preview-viewport").boundingBox();
      assert(
        box.height >= 100 &&
          box.width > 200 &&
          box.x >= 0 &&
          box.x + box.width <= width,
      );
      await p.locator("#preview-close").click();
      assert.deepEqual(errors, []);
      await p.close();
    }
    console.log(
      "Studio and schemaboy GPU entry points render without losing their viewport or authoring controls.",
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
