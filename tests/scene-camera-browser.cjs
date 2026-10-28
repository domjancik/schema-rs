"use strict";
const assert = require("node:assert/strict");
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
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
    const publication = await fetch(base + "/api/state").then((r) => r.json());
    const run = async (mobile) => {
      const context = await browser.newContext(
        mobile
          ? {
              viewport: { width: 390, height: 844 },
              isMobile: true,
              hasTouch: true,
            }
          : { viewport: { width: 1200, height: 900 } },
      );
      const page = await context.newPage(),
        errors = [],
        commands = [];
      page.on("pageerror", (e) => errors.push(e.message));
      await page.route("**/api/state", (r) => r.fulfill({ json: publication }));
      await page.route("**/api/command", (r) => {
        commands.push(r.request().postData());
        return r.abort();
      });
      await page.goto(base + "/preview?backend=webgl&quality=low");
      await page.waitForFunction(
        () =>
          Number(document.querySelector("#preview-stats")?.dataset.frames) > 1,
        { timeout: 30000 },
      );
      const pose = () =>
        page
          .locator("#preview-camera-status")
          .evaluate((e) => JSON.parse(e.dataset.pose));
      const details = page.locator(".preview-navigation");
      if (!(await details.evaluate((e) => e.open)))
        await details.locator("summary").click();
      let before = await pose();
      await page
        .getByRole("button", { name: "Camera right", exact: true })
        .click();
      let after = await pose();
      assert.notDeepEqual(after.position, before.position);
      await page.getByRole("button", { name: "Pan", exact: true }).click();
      before = await pose();
      await page
        .getByRole("button", { name: "Camera up", exact: true })
        .click();
      after = await pose();
      assert.notDeepEqual(after.target, before.target);
      const distance = (p) =>
        Math.hypot(...p.position.map((v, i) => v - p.target[i]));
      before = await pose();
      await page
        .getByRole("button", { name: "Zoom camera in", exact: true })
        .click();
      after = await pose();
      assert.ok(distance(after) < distance(before));
      await page.getByRole("button", { name: "Top", exact: true }).click();
      after = await pose();
      assert.ok(after.position[1] > after.target[1]);
      await page.getByRole("button", { name: "Orbit", exact: true }).click();
      if (mobile) {
        await details.locator("summary").click();
        const canvas = page.locator("#preview-canvas"),
          r = await canvas.boundingBox(),
          session = await context.newCDPSession(page),
          x = r.x + r.width * 0.4,
          y = r.y + r.height * 0.5;
        before = await pose();
        const touch = (type, points) =>
          session.send("Input.dispatchTouchEvent", {
            type,
            touchPoints: points,
          });
        await touch("touchStart", [
          { x: x - 25, y, id: 1 },
          { x: x + 25, y, id: 2 },
        ]);
        await touch("touchMove", [
          { x: x - 50, y: y + 12, id: 1 },
          { x: x + 50, y: y + 12, id: 2 },
        ]);
        await touch("touchEnd", []);
        await page.waitForTimeout(250);
        after = await pose();
        assert.notDeepEqual(after.position, before.position);
        assert.equal(await page.locator("#preview-selection").innerText(), "");
      } else {
        await page.locator("#preview-canvas").focus();
        before = await pose();
        await page.keyboard.press("w");
        after = await pose();
        assert.notDeepEqual(after.target, before.target);
        const rect = await page.locator("#preview-canvas").boundingBox();
        before = await pose();
        await page.mouse.move(
          rect.x + rect.width * 0.25,
          rect.y + rect.height * 0.5,
        );
        await page.mouse.down();
        await page.mouse.move(
          rect.x + rect.width * 0.25 + 70,
          rect.y + rect.height * 0.5 + 20,
          { steps: 10 },
        );
        await page.mouse.up();
        await page.waitForTimeout(150);
        after = await pose();
        assert.notDeepEqual(
          after.position,
          before.position,
          "Mouse drag should orbit",
        );
        assert.equal(await page.locator("#preview-selection").innerText(), "");
        await page.getByRole("button", { name: "Front", exact: true }).click();
        before = await pose();
        await page.waitForTimeout(250);
        await page.reload();
        await page.waitForFunction(
          () =>
            Number(document.querySelector("#preview-stats")?.dataset.frames) >
            1,
        );
        after = await pose();
        assert.ok(
          Math.hypot(...after.position.map((v, i) => v - before.position[i])) <
            0.01,
          "Camera pose should survive reload",
        );
      }
      await page.screenshot({
        path: `target/camera-navigation-${mobile ? "touch" : "desktop"}.png`,
        fullPage: true,
      });
      assert.deepEqual(commands, []);
      assert.deepEqual(errors, []);
      await context.close();
      console.log(
        (mobile ? "Touch" : "Desktop") +
          " camera controls, presets and read-only gestures passed.",
      );
    };
    await run(false);
    await run(true);
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
