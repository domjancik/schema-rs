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
    await page.goto(
      "http://127.0.0.1:" + server.address().port + "/studio.html",
    );
    await page.waitForFunction(
      () => document.querySelector("#command-palette")?.disabled === false,
    );
    const reserved = await page.evaluate(() =>
      [
        { key: "p", ctrlKey: true },
        { key: "P", ctrlKey: true, shiftKey: true },
        { key: "P", metaKey: true, shiftKey: true },
      ].map((options) => {
        const event = new KeyboardEvent("keydown", {
          ...options,
          bubbles: true,
          cancelable: true,
        });
        document.dispatchEvent(event);
        const result = {
          prevented: event.defaultPrevented,
          opened: !!document.querySelector("dialog[open]"),
        };
        document.querySelector("dialog[open]")?.close();
        return result;
      }),
    );
    assert(
      reserved.every((r) => !r.prevented && !r.opened),
      "Browser print/private-window shortcuts must not be intercepted",
    );
    await page.keyboard.press("F2");
    let dialog = page.getByRole("dialog", { name: "Commands", exact: true });
    await dialog.waitFor();
    await page.keyboard.press("Escape");
    await page.locator("#command-palette").click();
    await dialog.waitFor();
    await dialog
      .getByLabel("Search actions", { exact: true })
      .fill("state shortcut");
    await dialog
      .getByRole("button", { name: "State and shortcut help", exact: true })
      .click();
    const help = page.getByRole("dialog", {
      name: "State and shortcut help",
      exact: true,
    });
    await help.getByText("F2", { exact: true }).waitFor();
    assert(!(await help.textContent()).includes("Ctrl/Cmd+Shift+P"));
    await page.keyboard.press("Escape");
    await page.evaluate(() => {
      const input = document.createElement("input");
      input.id = "shortcut-typing-test";
      document.body.append(input);
    });
    const input = page.locator("#shortcut-typing-test");
    await input.focus();
    await page.keyboard.press("Shift+P");
    assert.equal(await input.inputValue(), "P");
    assert.equal(await page.locator("dialog[open]").count(), 0);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(
      "http://127.0.0.1:" + server.address().port + "/mobile.html",
    );
    await page.waitForFunction(
      () => document.querySelector("#command-palette")?.disabled === false,
    );
    await page.keyboard.press("F2");
    await page.getByRole("dialog", { name: "Commands", exact: true }).waitFor();
    console.log(
      "F2 and menu open Commands in Studio/mobile; print/private-window shortcuts remain untouched and help matches.",
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
