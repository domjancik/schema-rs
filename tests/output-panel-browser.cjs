"use strict";
const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
(async () => {
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const page = await browser.newPage({
        viewport: { width: 1500, height: 1000 },
      }),
      errors = [],
      commands = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const original = await fetch(base + "/api/state").then((r) => r.json());
    let publication = structuredClone(original);
    await page.route("**/api/state", (r) => r.fulfill({ json: publication }));
    await page.route("**/api/command", (r) => {
      const value = r.request().postDataJSON();
      commands.push(value);
      if (value.expected_revision !== publication.snapshot.revision)
        return r.fulfill({ status: 409, json: { error: "Revision conflict" } });
      assert.equal(value.command.type, "set_output");
      assert.equal(
        value.command.config.enabled,
        false,
        "No live output enabled during UI test",
      );
      publication.project.output = value.command.config;
      publication.snapshot.revision++;
      return r.fulfill({ json: { revision: publication.snapshot.revision } });
    });
    await page.goto(base + "/?view=studio");
    await page.locator(".studio-output").waitFor();
    const first = page.locator(".workspace-panel[data-view=output]").first();
    await first.locator(".panel-menu summary").click();
    await first
      .getByRole("button", { name: "Split side by side", exact: true })
      .click();
    const panels = page.locator(".workspace-panel[data-view=output]");
    assert.equal(await panels.count(), 2);
    const a = panels.nth(0).getByLabel("Art-Net destination", { exact: true }),
      b = panels.nth(1).getByLabel("Art-Net destination", { exact: true });
    await a.fill("127.0.0.1:6455");
    assert.equal(
      await b.inputValue(),
      original.project.output.target,
      "Duplicate views keep independent drafts",
    );
    publication = structuredClone(publication);
    publication.project.output.target = "127.0.0.1:6456";
    publication.snapshot.revision++;
    await page.waitForFunction(
      () =>
        document.querySelectorAll(".studio-output input[type=text]")[1]
          ?.value === "127.0.0.1:6456",
    );
    assert.equal(
      await a.inputValue(),
      "127.0.0.1:6455",
      "Published snapshots cannot overwrite a draft",
    );
    await panels
      .nth(0)
      .getByRole("button", { name: "Apply", exact: true })
      .click();
    await panels
      .nth(0)
      .getByText(/Settings were not applied/)
      .waitFor();
    assert.equal(publication.project.output.target, "127.0.0.1:6456");
    await panels
      .nth(0)
      .getByRole("button", { name: "Reload settings", exact: true })
      .click();
    assert.equal(await a.inputValue(), "127.0.0.1:6456");
    await a.fill("127.0.0.1:6457");
    await panels
      .nth(0)
      .getByRole("button", { name: "Apply", exact: true })
      .click();
    await page.waitForFunction(
      () =>
        document.querySelectorAll(".studio-output input[type=text]")[1]
          ?.value === "127.0.0.1:6457",
    );
    assert.equal(commands.length, 2);
    const grid = panels
      .nth(0)
      .getByRole("grid", { name: "DMX channel levels" });
    await grid.focus();
    await page.keyboard.press("Home");
    await page.keyboard.press("ArrowRight");
    assert.match(
      await panels.nth(0).locator(".output-readout").textContent(),
      /Channel 2/,
    );
    assert.deepEqual(errors, []);
    const after = await fetch(base + "/api/state").then((r) => r.json());
    assert.deepEqual(after.project, original.project);
    assert.equal(after.snapshot.revision, original.snapshot.revision);
    console.log(
      "Output panes: separate drafts, published telemetry, revision-conflict protection, explicit reload/apply and keyboard channel browsing passed.",
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
