"use strict";
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path"),
  http = require("node:http");
const { chromium } = require(process.env.PLAYWRIGHT_MODULE),
  root = path.resolve("target/browser-site");
let version = 1,
  apiRequests = 0;
const mime = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".webmanifest": "application/manifest+json",
};
const policy = JSON.parse(
  fs.readFileSync("scripts/publication/vercel.json", "utf8"),
);
const server = http.createServer((req, res) => {
  const url = new URL(req.url, "http://localhost");
  if (url.pathname.startsWith("/api/")) apiRequests++;
  const name = url.pathname === "/mobile" ? "/mobile.html" : url.pathname;
  const file = path.resolve(root, "." + name);
  const headers =
    policy.headers.find((group) => group.source === url.pathname) ||
    policy.headers[0];
  for (const header of headers.headers) res.setHeader(header.key, header.value);
  res.setHeader("Cache-Control", "no-store");
  if (
    !file.startsWith(root + path.sep) ||
    !fs.existsSync(file) ||
    !fs.statSync(file).isFile()
  ) {
    res.writeHead(404);
    res.end();
    return;
  }
  res.setHeader(
    "Content-Type",
    mime[path.extname(file)] || "application/octet-stream",
  );
  if (url.pathname === "/mobile-worker.js") {
    res.end(
      fs
        .readFileSync(file, "utf8")
        .replace(
          "schema-mobile-offline-v3",
          "schema-mobile-offline-v" + (version === 1 ? "3" : "4"),
        ),
    );
    return;
  }
  if (version === 2 && url.pathname === "/servers.css") {
    setTimeout(() => res.end(fs.readFileSync(file)), 350);
    return;
  }
  fs.createReadStream(file).pipe(res);
});
(async () => {
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = "http://127.0.0.1:" + server.address().port;
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const page = await browser.newPage(),
      errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(base + "/mobile");
    await page.waitForFunction(
      () => SchemaEngine?.kind === "browser" && document.querySelector("#save"),
    );
    await page.waitForFunction(() => !!navigator.serviceWorker.controller);
    await page.evaluate(async () => {
      SchemaServersCore.repository(localStorage).save({
        origin: "http://stage.local:8787",
        name: "Stage",
        view: "mobile",
      });
      const s = await SchemaEngine.request("/api/state");
      await SchemaEngine.request("/api/command", {
        expected_revision: s.snapshot.revision,
        command: { type: "set_dimmer", fixture: "fixture-3", value: 0.42 },
      });
      await caches.open("other-app-cache");
    });
    const before = await page.evaluate(() =>
      SchemaEngine.request("/api/state"),
    );
    version = 2;
    await page.locator(".app-menu summary").click();
    await page.getByRole("button", { name: "Servers", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Servers", exact: true });
    await dialog
      .getByRole("button", { name: "Check for app updates", exact: true })
      .click();
    await dialog
      .getByRole("button", { name: "Update app now", exact: true })
      .waitFor({ timeout: 10000 });
    await Promise.all([
      page.waitForEvent("domcontentloaded"),
      dialog
        .getByRole("button", { name: "Update app now", exact: true })
        .click(),
    ]);
    await page.waitForFunction(
      () => SchemaEngine?.kind === "browser" && document.querySelector("#save"),
    );
    assert.deepEqual(
      (await page.evaluate(() => SchemaEngine.request("/api/state"))).project,
      before.project,
    );
    assert.equal(
      await page.evaluate(
        () => SchemaServersCore.repository(localStorage).list()[0].name,
      ),
      "Stage",
    );
    await page.waitForFunction(async () =>
      (await caches.keys()).includes("schema-mobile-offline-v4"),
    );
    const keys = await page.evaluate(() => caches.keys());
    assert(keys.includes("other-app-cache"));
    assert(!keys.includes("schema-mobile-offline-v3"));
    await page.evaluate(async () => {
      const s = await SchemaEngine.request("/api/state");
      await SchemaEngine.request("/api/command", {
        expected_revision: s.snapshot.revision,
        command: { type: "undo" },
      });
    });
    assert.notEqual(
      (
        await page.evaluate(() => SchemaEngine.request("/api/state"))
      ).project.fixtures.find((f) => f.id === "fixture-3").dimmer,
      0.42,
    );
    assert.equal(apiRequests, 0);
    assert.deepEqual(errors, []);
    console.log(
      "Real waiting-worker update activates explicitly, reloads, retains project/history/servers, and preserves unrelated caches.",
    );
  } finally {
    await browser.close();
    server.close();
  }
})().catch((e) => {
  console.error(e);
  server.close();
  process.exitCode = 1;
});
