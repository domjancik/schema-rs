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
    page.setDefaultTimeout(15000);
    page.on("pageerror", (e) => console.log("UI error: " + e.message));
    await page.goto("http://127.0.0.1:" + server.address().port + "/test");
    await page.evaluate(async () => {
      const { SchemaPlayer } = await import("/schema-player.mjs");
      window.player = await SchemaPlayer.create({ persist: false });
      window.SchemaEngine = player;
      const state = await player.request("/api/state");
      const p = state.project;
      p.content["fixture-1"][0] = {
        name: "Inspection",
        enabled: true,
        root: {
          id: 20,
          behavior: "Modify.Color.Dim",
          properties: { Amount: 0.5 },
          drivers: {},
          children: [
            {
              id: 21,
              behavior: "Create.Color.Color",
              properties: { Color: "#ff0000" },
              drivers: {},
              children: [],
            },
          ],
        },
      };
      await player.request("/api/command", {
        expected_revision: state.snapshot.revision,
        command: { type: "replace_project", project: p },
      });
      window.calls = 0;
      const request = player.request.bind(player);
      player.request = (path, body) => {
        if (path === "/api/inspection") calls++;
        return request(path, body);
      };
      document.body.innerHTML =
        '<div class="block" data-fixture="fixture-1" data-layer="0" data-node="20" style="width:240px;height:130px;position:relative"><span>Dim</span><span class="node-value"></span><div class="node-meter"><i></i></div></div><aside id="inspector"></aside>';
    });
    await page.addStyleTag({ url: "/block-inspection.css" });
    await page.addScriptTag({ url: "/block-inspection.js" });
    await page.evaluate(() => {
      SchemaInspection.configure({
        selection: () => ({ fixture: "fixture-1", layer: 0, node: 20 }),
        revision: () => 1,
      });
      SchemaInspection.mount(document.getElementById("inspector"), {
        fixture: "fixture-1",
        layer: 0,
        node: 20,
      });
    });
    await page.waitForSelector(".block-attributes [data-color]");
    const outgoing = page.locator('[data-attributes="outgoing"]'),
      incoming = page.locator('[data-attributes="incoming"]');
    await outgoing.getByText("Colors", { exact: true }).waitFor();
    await incoming.getByText("Colors", { exact: true }).waitFor();
    assert.equal(
      await incoming.locator("[data-color]").first().getAttribute("data-color"),
      "1,0,0,1",
    );
    assert.equal(
      await outgoing.locator("[data-color]").first().getAttribute("data-color"),
      "0.5,0,0,1",
    );
    const compact = await page.evaluate(() =>
      player.request("/api/state/compact"),
    );
    assert(
      compact.snapshot.fixtures.every((f) => Object.keys(f.nodes).length === 0),
    );
    await page.evaluate(() => {
      window.inspectionFlood = setInterval(() => SchemaInspection.tick(), 10);
    });
    await page.waitForTimeout(1100);
    assert(
      (await page.evaluate(() => calls)) < 12,
      "Demand polling is throttled",
    );
    await page.evaluate(() => clearInterval(window.inspectionFlood));
    await page.evaluate(() => {
      document.body.replaceChildren();
    });
    await page.waitForTimeout(350);
    const stopped = await page.evaluate(() => calls);
    await page.waitForTimeout(600);
    assert.equal(
      await page.evaluate(() => calls),
      stopped,
      "No inspected views means no requests",
    );
    console.log("Starting full Studio check");
    await page.goto(
      "http://127.0.0.1:" + server.address().port + "/studio.html",
      { waitUntil: "domcontentloaded" },
    );
    await page.waitForSelector(".block.has-attributes");
    await page.waitForSelector('[data-attributes="outgoing"]');
    await page.screenshot({ path: "target/inspection-studio.png" });
    console.log("Starting phone check");
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(
      "http://127.0.0.1:" + server.address().port + "/mobile.html",
    );
    await page.waitForSelector('#inspect-attributes[role="button"]');
    await page.locator("#inspect-attributes").click();
    await page.waitForSelector(".attribute-dialog");
    await page.waitForSelector(
      '.attribute-dialog [data-attributes="outgoing"]',
    );
    await page.screenshot({ path: "target/inspection-phone.png" });
    console.log(
      "Real WASM block colors, incoming/outgoing attributes, compact state and on-demand idle behavior passed",
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
