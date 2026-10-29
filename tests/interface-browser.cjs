"use strict";
const assert = require("node:assert/strict");
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
if (["8787", "8788", ""].includes(new URL(base).port))
  throw Error("Use an isolated engine");
function contrast(a, b) {
  const luminance = (color) =>
    color
      .match(/[\d.]+/g)
      .slice(0, 3)
      .map(Number)
      .map((v) => {
        v /= 255;
        return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
      })
      .reduce((sum, v, i) => sum + v * [0.2126, 0.7152, 0.0722][i], 0);
  const x = luminance(a),
    y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
(async () => {
  const before = await fetch(base + "/api/state").then((r) => r.json());
  assert.equal(before.project.output.enabled, false);
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  const errors = [];
  let commands = 0;
  try {
    const page = await browser.newPage({
      viewport: { width: 1500, height: 1000 },
    });
    page.on("pageerror", (e) => errors.push(e.message));
    await page.route("**/api/command", (r) => {
      commands++;
      return r.abort();
    });
    await page.goto(base + "/?view=studio");
    await page.locator("#world .block").first().waitFor();
    for (const theme of ["light", "dark"]) {
      await page.evaluate((theme) => {
        document.documentElement.dataset.theme = theme;
      }, theme);
      const toolbarColors = await page
        .locator("#output-design")
        .evaluate((el) => {
          const c = getComputedStyle(el);
          return [c.color, c.backgroundColor];
        });
      assert(
        contrast(...toolbarColors) >= 4.5,
        "Theme changes keep toolbar text readable immediately",
      );
      const blockColors = await page
        .locator("#world .block")
        .evaluateAll((blocks) =>
          blocks
            .filter((block) =>
              block.querySelector(".node-properties")?.textContent.trim(),
            )
            .map((block) => [
              getComputedStyle(block.querySelector(".node-properties")).color,
              getComputedStyle(block.querySelector(".block-shell path")).fill,
            ]),
        );
      for (const colors of blockColors)
        assert(
          contrast(...colors) >= 4.5,
          "Block property labels remain readable on their family color",
        );
      await page.locator("#save").click();
      await page.waitForFunction(
        () => !document.querySelector("#projects-dialog input").disabled,
      );
      for (const selector of [
        "#projects-dialog input",
        "#projects-dialog .project-actions button:first-child",
      ]) {
        const colors = await page
          .locator(selector)
          .first()
          .evaluate((el) => {
            const c = getComputedStyle(el);
            return [c.color, c.backgroundColor];
          });
        assert(
          contrast(...colors) >= 4.5,
          `${theme} ${selector}: text contrast`,
        );
      }
      await page.getByLabel("Close projects", { exact: true }).click();
      await page.locator("#output-design").click();
      const output = page.getByRole("dialog", { name: "Output design" });
      await output
        .getByRole("button", { name: "Daily dimming", exact: true })
        .waitFor();
      assert.equal(
        await output
          .getByRole("button", { name: "Daily dimming", exact: true })
          .getAttribute("aria-pressed"),
        "true",
      );
      await output
        .getByRole("button", { name: "DMX models", exact: true })
        .click();
      assert.equal(
        await output
          .getByRole("button", { name: "DMX models", exact: true })
          .getAttribute("aria-pressed"),
        "true",
      );
      assert.equal(
        await output
          .getByRole("button", { name: "Daily dimming", exact: true })
          .getAttribute("aria-pressed"),
        "false",
      );
      await page.keyboard.press("Escape");
      const select = page.locator('select[aria-label="View type"]').first();
      await select.focus();
      assert.notEqual(
        await select.evaluate((el) => getComputedStyle(el).outlineStyle),
        "none",
        "Keyboard focus stays visible",
      );
    }
    for (const width of [1500, 1100, 1024, 850]) {
      await page.setViewportSize({ width, height: 900 });
      const layout = await page.evaluate(() => {
        const header = document
            .querySelector("body>header")
            .getBoundingClientRect(),
          save = document.querySelector("#save").getBoundingClientRect(),
          footer = document
            .querySelector("body>footer")
            .getBoundingClientRect();
        return {
          headerBottom: header.bottom,
          saveBottom: save.bottom,
          footerBottom: footer.bottom,
          scrollWidth: document.documentElement.scrollWidth,
          viewportWidth: innerWidth,
        };
      });
      assert(
        layout.saveBottom <= layout.headerBottom + 1,
        "Toolbar stays inside its header",
      );
      assert(layout.footerBottom <= 901, "Workspace footer remains in view");
      assert(
        layout.scrollWidth <= layout.viewportWidth,
        "Studio avoids horizontal page overflow",
      );
    }
    const mobile = await browser.newPage({
      viewport: { width: 390, height: 844 },
      isMobile: true,
      hasTouch: true,
    });
    mobile.on("pageerror", (e) => errors.push(e.message));
    await mobile.route("**/api/command", (r) => {
      commands++;
      return r.abort();
    });
    await mobile.goto(base + "/mobile");
    await mobile.locator("#save").click();
    await mobile.waitForFunction(
      () => !document.querySelector("#projects-dialog input").disabled,
    );
    assert(
      parseFloat(
        await mobile
          .locator("#projects-dialog input")
          .first()
          .evaluate((el) => getComputedStyle(el).fontSize),
      ) >= 16,
      "Phone fields avoid focus zoom",
    );
    for (const size of [
      { width: 320, height: 740 },
      { width: 390, height: 844 },
      { width: 844, height: 390 },
    ]) {
      await mobile.setViewportSize(size);
      const box = await mobile.locator("#projects-dialog").boundingBox();
      assert(
        box.x >= 0 &&
          box.x + box.width <= size.width + 1 &&
          box.height <= size.height + 1,
        "Dialog stays in viewport",
      );
    }
    assert.equal(commands, 0);
    assert.deepEqual(errors, []);
    const after = await fetch(base + "/api/state").then((r) => r.json());
    assert.deepEqual(after.project, before.project);
    assert.equal(after.snapshot.revision, before.snapshot.revision);
    console.log(
      "Interface: light/dark contrast, selected output tabs, keyboard focus, mobile fields/viewport and read-only dialogs passed.",
    );
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
