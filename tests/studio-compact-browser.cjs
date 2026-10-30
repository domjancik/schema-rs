"use strict";
const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
(async () => {
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
      }),
      errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const initial = await fetch(base + "/api/state").then((r) => r.json());
    await page.goto(base + "/?view=studio");
    await page.locator(".schema-workspace").waitFor();
    assert(
      (await page.locator("body>header").boundingBox()).height <= 64,
      "Phone Studio header must remain one compact row",
    );
    const picker = page.getByRole('navigation',{name:'Studio panels',exact:true});
    await picker.waitFor();
    assert.equal(await page.locator('.workspace-panel:visible').count(),1,'A phone shows one usable full-width pane');
    await picker.getByRole('button',{name:'Output',exact:true}).click();
    const output = page.locator(".workspace-panel[data-view=output]:visible");
    await output.waitFor();
    assert.equal(
      await output.locator(".output-levels [data-channel]").count(),
      512,
    );
    assert.equal(
      await page
        .locator(
          ".workspace-panel[data-view=stack] .telemetry,.scene-panel .output-card",
        )
        .count(),
      0,
      "Output does not live in another editor",
    );
    await page
      .getByRole("button", { name: "More Studio actions", exact: true })
      .click();
    for (const name of ["Servers", "Components", "Connection"])
      assert.equal(
        await page
          .locator(".studio-more-items")
          .getByRole("button", { name, exact: true })
          .isVisible(),
        true,
      );
    await page.keyboard.press("Escape");
    await page.locator(".workspace-menu summary").click();
    await page
      .getByRole("button", { name: "Show split layout", exact: true })
      .click();
    assert.equal(await page.locator(".workspace-panel:visible").count(), 4);
    await page.locator(".workspace-menu summary").click();
    await page
      .getByRole("button", { name: "Focus one panel", exact: true })
      .click();
    assert.equal(await page.locator(".workspace-panel:visible").count(), 1);

    for (const [width, height] of [
      [320, 568],
      [390, 844],
      [768, 1024],
      [844, 390],
    ]) {
      await page.setViewportSize({ width, height });
      const pane = await page.locator(".workspace-panel:visible").boundingBox();
      assert(
        pane.width >= width - 20 && pane.height >= height * 0.45,
        "Compact pane retains useful editing space",
      );
      assert(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
        "No document overflow",
      );
      assert(
        await page
          .locator("body>footer")
          .evaluate((e) => e.getBoundingClientRect().bottom <= innerHeight + 1),
        "Footer remains in viewport",
      );
    }
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.waitForFunction(
      () =>
        !document
          .querySelector(".schema-workspace")
          .classList.contains("workspace-compact"),
    );
    assert.equal(await page.locator(".workspace-panel").count(), 4);
    assert.equal(await page.locator(".workspace-panel:visible").count(), 4);
    assert.deepEqual(errors, []);
    const after = await fetch(base + "/api/state").then((r) => r.json());
    assert.deepEqual(after.project, initial.project);
    assert.equal(after.snapshot.revision, initial.snapshot.revision);
    console.log(
      "Compact Studio header, all-pane navigation, independent output, responsive bounds and unchanged project passed.",
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
