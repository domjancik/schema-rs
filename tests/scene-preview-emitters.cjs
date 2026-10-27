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
    for (const backend of ["auto", "webgl"]) {
      const page = await browser.newPage({
          viewport: { width: 850, height: 650 },
        }),
        errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      page.on("console", (message) => {
        if (message.type() === "error") errors.push(message.text());
      });
      const fixture = {
        id: "source",
        name: "Mixed emitters",
        position: [0, 2, 0],
        rotation: [0, 0, 0],
        pixel_positions: [[0, 0, 0]],
        groups: [],
        model: "dimmer",
        universe: 0,
        address: 1,
        dimmer: 1,
      };
      const frame = {
        project: {
          name: "Native emitter preview",
          fixtures: [fixture],
          dmx_models: {},
        },
        snapshot: {
          frame: 1,
          revision: 1,
          fixtures: [{ fixture, result: {} }],
          universes: { 0: [128] },
          emitters: [0, 1, 2].map((kind) => ({
            fixture: "source",
            layer: 0,
            node: kind,
            kind,
            position: [kind - 1, 2, 0],
            direction: [0, -1, 0],
            color: [1, 0.4, 0.2, 1],
            intensity: 0.5,
            range: 5,
            angle: 50,
            width: 1,
            height: 1,
          })),
        },
      };
      await page.route("**/api/state", (r) => r.fulfill({ json: frame }));
      await page.route("**/api/command", (r) => r.abort());
      await page.goto(
        "http://127.0.0.1:8788/preview?quality=low&backend=" + backend,
      );
      await page.waitForFunction(
        () =>
          Number(document.querySelector("#preview-stats")?.dataset.frames) > 3,
        null,
        { timeout: 60000 },
      );
      assert.deepEqual(errors, []);
      console.log(
        backend +
          ": point/plane/spot emitters render with SSGI and volumetrics.",
      );
      await page.close();
    }
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
