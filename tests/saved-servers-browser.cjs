"use strict";
const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
(async () => {
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  try {
    const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
      }),
      errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    page.on("console", (m) => {
      if (
        m.type() === "error" &&
        /Content Security Policy|Refused.*security policy/i.test(m.text())
      )
        errors.push(m.text().slice(-400));
    });
    await page.goto(base + "/mobile");
    await page.locator(".app-menu summary").click();
    await page
      .getByRole("button", { name: "Servers", exact: true })
      .click({ timeout: 1500 });
    const dialog = page.getByRole("dialog", { name: "Servers", exact: true });
    await dialog
      .getByLabel("Server address", { exact: true })
      .fill("schema.local:8787");
    await dialog.getByLabel("Server name", { exact: true }).fill("Stage");
    await dialog
      .getByLabel("Preferred view", { exact: true })
      .selectOption("studio");
    await dialog
      .getByRole("button", { name: "Save server", exact: true })
      .click();
    const row = dialog.locator(".saved-server").filter({ hasText: "Stage" });
    await row.waitFor();
    assert.equal(
      await row
        .getByRole("link", { name: "Open Studio", exact: true })
        .getAttribute("href"),
      "http://schema.local:8787/?view=studio",
    );
    await page.reload();
    await page.locator(".app-menu summary").click();
    await page.getByRole("button", { name: "Servers", exact: true }).click();
    await dialog
      .locator(".saved-server")
      .filter({ hasText: "Stage" })
      .waitFor();

    const saved = dialog.locator(".saved-server").filter({ hasText: "Stage" });
    await saved.getByRole("button", { name: "QR", exact: true }).click();
    await dialog
      .getByRole("img", { name: "Pairing QR code", exact: true })
      .waitFor();
    const png = await dialog
      .getByRole("img", { name: "Pairing QR code", exact: true })
      .evaluate((c) => c.toDataURL("image/png"));
    await saved.getByRole("button", { name: "Remove", exact: true }).click();
    await dialog.getByLabel("Choose QR image", { exact: true }).setInputFiles({
      name: "server.png",
      mimeType: "image/png",
      buffer: Buffer.from(png.split(",")[1], "base64"),
    });
    await page.waitForFunction(
      () =>
        document.querySelector('#servers-dialog [aria-label="Server address"]')
          .value === "http://schema.local:8787",
    );
    assert.equal(
      await dialog.locator(".saved-server").count(),
      0,
      "Scanning only reviews; it does not save or open a host",
    );
    await dialog
      .getByLabel("Server name", { exact: true })
      .fill("<img src=x onerror=alert(1)>");
    await dialog
      .getByRole("button", { name: "Save server", exact: true })
      .click();
    assert.equal(await dialog.locator(".saved-server img").count(), 0);
    assert.equal(
      await dialog.locator(".saved-server strong").textContent(),
      "<img src=x onerror=alert(1)>",
    );
    await page.evaluate(() => {
      navigator.mediaDevices.getUserMedia = async () => {
        throw new DOMException("Denied", "NotAllowedError");
      };
    });
    await dialog.getByRole("button", { name: "Scan QR", exact: true }).click();
    await page.waitForFunction(() =>
      document
        .querySelector(".server-status")
        .textContent.includes("not granted"),
    );
    await page.evaluate(() => {
      navigator.mediaDevices.getUserMedia = async () => {
        const c = document.createElement("canvas");
        c.width = c.height = 100;
        window.testStream = c.captureStream(1);
        return window.testStream;
      };
    });
    await dialog.getByRole("button", { name: "Scan QR", exact: true }).click();
    await page.waitForFunction(
      () => document.querySelector(".server-scanner video")?.srcObject !== null,
    );
    await dialog
      .getByRole("button", { name: "Close servers", exact: true })
      .click();
    assert.equal(
      await page.evaluate(() =>
        testStream.getTracks().every((t) => t.readyState === "ended"),
      ),
      true,
      "Closing stops every camera track",
    );
    if (!(await page.locator(".app-menu").evaluate((e) => e.open)))
      await page.locator(".app-menu summary").click();
    await page.getByRole("button", { name: "Servers", exact: true }).click();
    await page.evaluate(() => {
      navigator.mediaDevices.getUserMedia = () =>
        new Promise((resolve) => {
          window.releaseCamera = () => {
            const c = document.createElement("canvas");
            c.width = c.height = 100;
            window.lateStream = c.captureStream(1);
            resolve(window.lateStream);
          };
        });
    });
    await dialog.getByRole("button", { name: "Scan QR", exact: true }).click();
    await page.waitForFunction(() => !!window.releaseCamera);
    await dialog
      .getByRole("button", { name: "Close servers", exact: true })
      .click();
    await page.evaluate(() => releaseCamera());
    await page.waitForFunction(() =>
      lateStream.getTracks().every((t) => t.readyState === "ended"),
    );
    if (!(await page.locator(".app-menu").evaluate((e) => e.open)))
      await page.locator(".app-menu summary").click();
    await page.getByRole("button", { name: "Servers", exact: true }).click();
    await dialog
      .getByRole("button", { name: "Check for app updates", exact: true })
      .click();
    await page.waitForFunction(() =>
      document
        .querySelector(".server-updates p")
        .textContent.match(/host|latest app|update/i),
    );
    for (const width of [320, 390, 844]) {
      await page.setViewportSize({ width, height: 568 });
      const box = await dialog.boundingBox();
      assert(box.x >= 0 && box.x + box.width <= width);
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
        true,
      );
    }
    assert.deepEqual(errors, []);
    console.log(
      "Saved servers: direct host links, reload persistence, QR image review, safe labels, denied/late camera cleanup and phone bounds passed.",
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
