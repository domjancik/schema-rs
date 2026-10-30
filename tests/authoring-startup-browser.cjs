"use strict";
const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE),
  base = process.env.SCHEMA_URL || "http://127.0.0.1:8833";
if (["", "8787", "8788"].includes(new URL(base).port))
  throw Error("Use isolated engine");
(async () => {
  const b = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const p = await b.newPage({ viewport: { width: 320, height: 568 } }),
      errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    let release;
    const gate = new Promise((r) => (release = r));
    await p.route("**/api/state/compact", async (route) => {
      await gate;
      await route.continue();
    });
    await p.goto(base + "/?view=studio");
    const commands = p.locator("#command-palette");
    await commands.waitFor({ state: "attached" });
    assert(
      await commands.isDisabled(),
      "Commands must wait for a scene publication",
    );
    await p.keyboard.press("F2");
    assert.equal(await p.locator("dialog[open]").count(), 0);
    release();
    await p.waitForFunction(
      () => !document.querySelector("#command-palette").disabled,
    );
    await p.keyboard.press("F2");
    await p.getByRole("dialog", { name: "Commands", exact: true }).waitFor();
    await p.keyboard.press("Escape");
    await p.waitForFunction(() => !document.querySelector("dialog[open]"));
    await p.route("**/api/command", (route) =>
      route.fulfill({
        status: 409,
        json: { error: "Visible revision conflict during authoring" },
      }),
    );
    await p.locator("#undo").click();
    await p.waitForFunction(() =>
      document
        .querySelector("#status")
        .textContent.includes("Visible revision conflict"),
    );
    assert(
      await p
        .locator("#status")
        .evaluate((e) => parseFloat(getComputedStyle(e).fontSize) > 0),
      "Phone errors remain readable",
    );
    assert.equal(await p.locator("#status").getAttribute("data-error"), "true");
    assert.deepEqual(errors, []);
    console.log(
      "Delayed startup blocks premature Commands; phone errors remain visible; no authored command reached the engine.",
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
