"use strict";
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path"),
  http = require("node:http"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const root = path.resolve("target/browser-site"),
  types = {
    ".html": "text/html",
    ".js": "text/javascript",
    ".mjs": "text/javascript",
    ".wasm": "application/wasm",
    ".json": "application/json",
    ".css": "text/css",
    ".svg": "image/svg+xml",
    ".woff2": "font/woff2",
    ".png": "image/png",
  };
const server = http.createServer((req, res) => {
  const url = new URL(req.url, "http://localhost");
  if (url.pathname === "/test") {
    res.setHeader("Content-Type", "text/html");
    res.end("<!doctype html><title>Migration test</title>");
    return;
  }
  const file = path.resolve(root, "." + url.pathname);
  if (!file.startsWith(root + path.sep) || !fs.existsSync(file)) {
    res.writeHead(404);
    res.end();
    return;
  }
  const config = JSON.parse(
    fs.readFileSync("scripts/publication/vercel.json", "utf8"),
  );
  for (const header of (
    config.headers.find((h) => h.source === url.pathname) || config.headers[0]
  ).headers)
    res.setHeader(header.key, header.value);
  res.setHeader(
    "Content-Type",
    types[path.extname(file)] || "application/octet-stream",
  );
  fs.createReadStream(file).pipe(res);
});

(async () => {
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const page = await browser.newPage();
    page.setDefaultTimeout(6000);
    await page.goto(
      "http://127.0.0.1:" + server.address().port + "/studio.html",
    );
    await page.waitForFunction(
      () => document.querySelector("#command-palette")?.disabled === false,
    );
    const revision = await page.evaluate(async () => {
      const state = await SchemaEngine.request("/api/state");
      return (
        await SchemaEngine.request("/api/command", {
          expected_revision: state.snapshot.revision,
          command: {
            type: "set_dimming",
            timeline: {
              enabled: true,
              master: 0.8,
              utc_offset_minutes: 180,
              points: [
                { minute: 0, level: 0 },
                { minute: 720, level: 1 },
              ],
            },
          },
        })
      ).revision;
    });
    await page.waitForFunction(
      (rev) =>
        document.getElementById("frame")?.textContent.includes("REV " + rev),
      revision,
    );
    await page.evaluate(() => SchemaOutput.open("dimming"));
    await page.getByLabel("Current schedule time", { exact: true }).waitFor();
    await page.locator(".timeline-now").waitFor();
    const before = await page.evaluate(() =>
      SchemaEngine.request("/api/session"),
    );
    const dialog = page.getByRole("dialog", {
      name: "Output design",
      exact: true,
    });
    await dialog
      .getByLabel("Simulated schedule time", { exact: true })
      .fill("12:00:00");
    await dialog
      .getByLabel("Simulated schedule time", { exact: true })
      .dispatchEvent("input");
    await page.locator(".timeline-preview").waitFor();
    assert.equal(
      await page.evaluate(
        async () =>
          (await SchemaEngine.request("/api/dimming/clock")).simulating,
      ),
      false,
      "Scrubbing is preview-only until explicitly applied",
    );
    await dialog
      .getByLabel("Apply simulated time to output", { exact: true })
      .check();
    await page.waitForFunction(
      async () => (await SchemaEngine.request("/api/dimming/clock")).simulating,
    );
    const fixed = await page.evaluate(() =>
      SchemaEngine.request("/api/dimming/clock"),
    );
    assert.equal(fixed.speed, 0);
    assert.equal(
      fixed.seconds,
      32400,
      "Simulation uses the saved schedule UTC offset",
    );
    assert.equal(
      fixed.level,
      0.8,
      "The native engine applies the simulated schedule level",
    );
    await page
      .getByLabel("Current schedule time", { exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({ path: "target/dimming-simulation.png" });
    await dialog
      .getByLabel("Simulated schedule time", { exact: true })
      .fill("23:59:55");
    await dialog
      .getByLabel("Simulated schedule time", { exact: true })
      .dispatchEvent("input");
    await page.waitForFunction(
      async () =>
        (await SchemaEngine.request("/api/dimming/clock")).seconds === 75595,
    );
    await dialog
      .getByLabel("Time simulation speed", { exact: true })
      .selectOption("60");
    await dialog
      .getByRole("button", { name: "Play time simulation", exact: true })
      .click();
    await page.waitForFunction(
      async () =>
        (await SchemaEngine.request("/api/dimming/clock")).speed === 60,
    );
    await page.waitForTimeout(250);
    const running = await page.evaluate(() =>
      SchemaEngine.request("/api/dimming/clock"),
    );
    assert(
      (running.seconds + 180 * 60) % 86400 < 180,
      "Playback crosses midnight in schedule-local time",
    );
    await dialog
      .getByRole("button", { name: "Use live time", exact: true })
      .click();
    await page.waitForFunction(
      async () =>
        !(await SchemaEngine.request("/api/dimming/clock")).simulating,
    );
    await dialog
      .getByLabel("Apply simulated time to output", { exact: true })
      .check();
    await page.waitForFunction(
      async () => (await SchemaEngine.request("/api/dimming/clock")).simulating,
    );
    await dialog
      .getByRole("button", { name: "Close output", exact: true })
      .click();
    await page.waitForFunction(
      async () =>
        !(await SchemaEngine.request("/api/dimming/clock")).simulating,
    );
    const after = await page.evaluate(() =>
      SchemaEngine.request("/api/session"),
    );
    for (const key of ["project", "undo", "redo", "checkpoints"])
      assert.deepEqual(
        after[key],
        before[key],
        key + " remains unchanged by simulation",
      );
    await page.setViewportSize({ width: 320, height: 568 });
    await page.goto(
      "http://127.0.0.1:" + server.address().port + "/mobile.html",
    );
    await page.waitForFunction(
      () => document.querySelector("#command-palette")?.disabled === false,
    );
    await page.evaluate(() => SchemaOutput.open("dimming"));
    await page.getByLabel("Current schedule time", { exact: true }).waitFor();
    const mobileBounds = await page
      .locator("#output-design-dialog")
      .evaluate((e) => ({ client: e.clientWidth, scroll: e.scrollWidth }));
    assert(
      mobileBounds.scroll <= mobileBounds.client + 1,
      "Phone dimming controls must fit horizontally",
    );
    await page
      .getByRole("button", { name: "Close output", exact: true })
      .click();
    console.log(
      "Current-time marker, preview-only scrubbing, explicit output simulation, playback, live return, close cleanup and unchanged history passed.",
    );
  } finally {
    await browser.close();
    server.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
  server.close();
});
