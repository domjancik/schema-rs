"use strict";
const assert = require("node:assert/strict"),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || "http://127.0.0.1:8823";
if (["8787", "8788", ""].includes(new URL(base).port))
  throw Error("Use isolated output-disabled engine");
(async () => {
  const state = () => fetch(base + "/api/state").then((r) => r.json());
  async function edit(command) {
    const s = await state();
    const r = await fetch(base + "/api/command", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ expected_revision: s.snapshot.revision, command }),
    });
    const text = await r.text();
    assert.equal(r.status, 200, text);
    const revision = JSON.parse(text).revision;
    for (let i = 0; i < 100; i++) {
      if ((await state()).snapshot.revision >= revision) return;
      await new Promise((resolve) => setTimeout(resolve, 20));
    }
    throw Error("Publication did not catch up");
  }
  async function waitState(predicate) {
    for (let i = 0; i < 200; i++) {
      const s = await state();
      if (predicate(s)) return s;
      await new Promise((resolve) => setTimeout(resolve, 20));
    }
    throw Error("Expected publication not received");
  }
  assert.equal((await state()).project.output.enabled, false);
  await edit({
    type: "replace_layer",
    fixture: "fixture-1",
    layer: 0,
    stack: {
      name: "Animated position",
      root: {
        id: 1,
        behavior: "Create.Position.XYZ",
        properties: { X: 5, Y: 6, Z: 7 },
        children: [],
        drivers: {},
      },
    },
  });
  await edit({
    type: "move_fixture",
    fixture: "fixture-1",
    position: [0, 0, 0],
  });
  const original = (await state()).project,
    b = await chromium.launch({
      channel: "chrome",
      headless: true,
      args: [
        "--enable-unsafe-webgpu",
        "--ignore-gpu-blocklist",
        "--use-angle=d3d11",
      ],
    });
  try {
    for (const mode of ["studio", "mobile", "webgl", "gpu"]) {
      const context = await b.newContext(
          mode === "mobile"
            ? {
                viewport: { width: 390, height: 844 },
                isMobile: true,
                hasTouch: true,
              }
            : { viewport: { width: 1500, height: 1000 } },
        ),
        p = await context.newPage();
      p.setDefaultTimeout(10000);
      const errors = [];
      p.on("pageerror", (e) => errors.push(e.message));
      if (mode === "studio") {
        await p.goto(base + "/?view=studio");
        await p
          .locator(
            '.workspace-panel[data-view="setup"] [aria-label="View type"]',
          )
          .selectOption("scene");
        await p
          .locator('.workspace-panel[data-view="scene"]')
          .getByRole("button", { name: "3D", exact: true })
          .click();
      } else if (mode === "mobile") await p.goto(base + "/mobile");
      else {
        await p.goto(
          base +
            "/preview?quality=low&backend=" +
            (mode === "webgl" ? "webgl" : "auto"),
        );
        await p.waitForFunction(
          () =>
            Number(document.querySelector("#preview-stats")?.dataset.frames) >
            2,
          null,
          { timeout: 60000 },
        );
      }
      await p
        .getByRole("button", { name: "Place selected fixture", exact: true })
        .click();
      const dialog = p.getByRole("dialog", {
        name: "Fixture position",
        exact: true,
      });
      await dialog.waitFor();
      assert.equal(
        await dialog.getByLabel("Y position (m)", { exact: true }).inputValue(),
        String(original.fixtures[0].position[1]),
      );
      await dialog.getByLabel("X position (m)", { exact: true }).fill("1.25");
      await dialog.getByLabel("Y position (m)", { exact: true }).fill("2.5");
      await dialog.getByLabel("Z position (m)", { exact: true }).fill("-.75");
      const rev = (await state()).snapshot.revision;
      await dialog
        .getByRole("button", { name: "Apply position", exact: true })
        .click();
      await waitState(
        (s) =>
          JSON.stringify(s.project.fixtures[0].position) ===
            "[1.25,2.5,-0.75]" && s.snapshot.revision === rev + 1,
      );
      await edit({ type: "undo" });
      assert.deepEqual((await state()).project, original);
      await p
        .getByRole("button", { name: "Place selected fixture", exact: true })
        .click();
      await dialog
        .getByRole("button", { name: "Show move gizmo", exact: true })
        .click();
      const layer = p.locator(".fixture-placement-overlay").first();
      await layer.locator('[data-move-axes="1"]').waitFor();
      await p.waitForFunction(
        (expected) =>
          document.querySelector(".fixture-placement-overlay")?.dataset
            .position === JSON.stringify(expected),
        original.fixtures[0].position,
      );
      const point = await layer
          .locator('[data-move-axes="1"] circle')
          .last()
          .boundingBox(),
        before = await state(),
        camera = await p
          .locator("[data-camera]")
          .first()
          .getAttribute("data-camera")
          .catch(() => null);
      const x = point.x + point.width / 2,
        y = point.y + point.height / 2;
      await p.mouse.move(x, y);
      await p.mouse.down();
      await p.mouse.move(x, y - 35, { steps: 5 });
      assert.deepEqual(
        (await state()).project,
        before.project,
        "Drag previews do not mutate authored state",
      );
      if (camera)
        assert.equal(
          await p.locator("[data-camera]").first().getAttribute("data-camera"),
          camera,
          "Axis drag does not orbit the camera",
        );
      await p.mouse.up();
      await waitState(
        (s) =>
          s.project.fixtures[0].position[1] !==
            before.project.fixtures[0].position[1] &&
          s.snapshot.revision === before.snapshot.revision + 1,
      );
      const checked = await state();
      const moved = checked.project.fixtures[0].position;
      assert.equal(moved[0], original.fixtures[0].position[0]);
      assert.equal(moved[2], original.fixtures[0].position[2]);
      assert.notEqual(
        moved[1],
        original.fixtures[0].position[1],
        JSON.stringify({
          mode,
          moved,
          before: before.project.fixtures[0].position,
        }),
      );
      await edit({ type: "undo" });
      assert.deepEqual((await state()).project, original);
      await layer.locator('[data-move-axes="0"]').waitFor();
      await p.waitForFunction(
        (expected) =>
          document.querySelector(".fixture-placement-overlay")?.dataset
            .position === JSON.stringify(expected),
        original.fixtures[0].position,
      );
      const pos = await layer
        .locator('[data-move-axes="0"] circle')
        .last()
        .boundingBox();
      await p.mouse.move(pos.x + pos.width / 2, pos.y + pos.height / 2);
      await p.mouse.down();
      await p.mouse.move(pos.x + pos.width / 2 + 30, pos.y + pos.height / 2);
      await p.keyboard.press("Escape");
      await p.mouse.up();
      assert.deepEqual(
        (await state()).project,
        original,
        "Escape cancels placement",
      );
      if (mode === "studio") {
        await p
          .getByRole("button", { name: "Place selected fixture", exact: true })
          .click();
        await dialog
          .getByLabel("Fixture snap distance", { exact: true })
          .selectOption("0.5");
        await dialog
          .getByLabel("Fixture drag plane", { exact: true })
          .selectOption("0,2");
        await dialog
          .getByRole("button", { name: "Close fixture position", exact: true })
          .click();
        await layer.locator('[data-move-axes="0,2"]').waitFor();
        const box = await layer.locator('[data-move-axes="0,2"]').boundingBox();
        await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await p.mouse.down();
        await p.mouse.move(
          box.x + box.width / 2 + 45,
          box.y + box.height / 2 + 25,
          { steps: 5 },
        );
        await p.mouse.up();
        const planeMove = await waitState(
          (s) =>
            JSON.stringify(s.project.fixtures[0].position) !==
            JSON.stringify(original.fixtures[0].position),
        );
        const v = planeMove.project.fixtures[0].position;
        assert.equal(v[1], 0);
        assert.equal(v[0] / 0.5, Math.round(v[0] / 0.5));
        assert.equal(v[2] / 0.5, Math.round(v[2] / 0.5));
        await edit({ type: "undo" });
        assert.deepEqual((await state()).project, original);
        await p.waitForFunction(
          (expected) =>
            document.querySelector(".fixture-placement-overlay")?.dataset
              .position === JSON.stringify(expected),
          original.fixtures[0].position,
        );
        await layer.locator('[data-move-axes="1"]').focus();
        await p.keyboard.press("ArrowUp");
        await waitState((s) => s.project.fixtures[0].position[1] === 0.5);
        await edit({ type: "undo" });
        assert.deepEqual((await state()).project, original);
        await p
          .getByRole("button", { name: "Place selected fixture", exact: true })
          .click();
        await dialog.getByLabel("X position (m)", { exact: true }).fill("1e20");
        assert.equal(
          await dialog
            .getByRole("button", { name: "Apply position", exact: true })
            .isDisabled(),
          true,
        );
        await dialog
          .getByRole("button", { name: "Close fixture position", exact: true })
          .click();
      }
      if (mode === "studio") {
        await p.waitForFunction(
          (expected) =>
            document.querySelector(".fixture-placement-overlay")?.dataset
              .position === JSON.stringify(expected),
          original.fixtures[0].position,
        );
        const handle = await layer
          .locator('[data-move-axes="1"] circle')
          .last()
          .boundingBox();
        await p.mouse.move(
          handle.x + handle.width / 2,
          handle.y + handle.height / 2,
        );
        await p.mouse.down();
        await p.mouse.move(
          handle.x + handle.width / 2,
          handle.y + handle.height / 2 - 20,
        );
        const other = [...original.fixtures[1].position];
        other[0] += 0.125;
        const start = (await state()).snapshot.revision;
        await edit({
          type: "move_fixture",
          fixture: original.fixtures[1].id,
          position: other,
        });
        await p.waitForTimeout(150);
        await p.mouse.up();
        assert.equal((await state()).snapshot.revision, start + 1);
        assert.deepEqual(
          (await state()).project.fixtures[0].position,
          original.fixtures[0].position,
        );
        await edit({ type: "undo" });
        assert.deepEqual((await state()).project, original);
      }
      if (mode === "studio") {
        await layer.locator('[data-move-axes="1"]').focus();
        await p.keyboard.press("g");
        assert.equal(await layer.isVisible(), false);
        await p.keyboard.press("g");
        await layer.locator('[data-move-axes="1"]').waitFor();
        await layer.locator('[data-move-axes="1"]').focus();
        await p.keyboard.press("Enter");
        await dialog.waitFor();
        await dialog
          .getByRole("button", { name: "Close fixture position", exact: true })
          .click();
      }
      if (mode === "mobile") {
        await p.waitForFunction(
          (expected) =>
            document.querySelector(".fixture-placement-overlay")?.dataset
              .position === JSON.stringify(expected),
          original.fixtures[0].position,
        );
        const cdp = await context.newCDPSession(p),
          hit = await layer
            .locator('[data-move-axes="1"] circle')
            .last()
            .boundingBox(),
          touch = {
            id: 31,
            x: hit.x + hit.width / 2,
            y: hit.y + hit.height / 2,
          };
        await cdp.send("Input.dispatchTouchEvent", {
          type: "touchStart",
          touchPoints: [touch],
        });
        await cdp.send("Input.dispatchTouchEvent", {
          type: "touchMove",
          touchPoints: [{ ...touch, y: touch.y - 25 }],
        });
        await cdp.send("Input.dispatchTouchEvent", {
          type: "touchCancel",
          touchPoints: [],
        });
        assert.deepEqual((await state()).project, original);
        await cdp.send("Input.dispatchTouchEvent", {
          type: "touchStart",
          touchPoints: [touch],
        });
        await cdp.send("Input.dispatchTouchEvent", {
          type: "touchMove",
          touchPoints: [{ ...touch, y: touch.y - 25 }],
        });
        await cdp.send("Input.dispatchTouchEvent", {
          type: "touchEnd",
          touchPoints: [],
        });
        await waitState((s) => s.project.fixtures[0].position[1] !== 0);
        await edit({ type: "undo" });
        assert.deepEqual((await state()).project, original);
      }
      if (mode === "mobile") {
        await p.setViewportSize({ width: 320, height: 568 });
        await p
          .getByRole("button", { name: "Place selected fixture", exact: true })
          .click();
        await dialog
          .getByRole("button", { name: "Expand 3D view", exact: true })
          .click();
        const size = await p.locator("#scene").boundingBox();
        assert(size.height > 200);
        assert(size.y + size.height < 568);
        await p
          .getByRole("button", {
            name: "Finish fixture placement",
            exact: true,
          })
          .click();
        assert.equal(await p.locator(".fixture-placement-expanded").count(), 0);
        assert.deepEqual((await state()).project, original);
      }
      if (mode === "webgl" || mode === "gpu") {
        const revision = (await state()).snapshot.revision;
        await p
          .getByRole("button", { name: "Place selected fixture", exact: true })
          .click();
        await dialog
          .getByRole("button", { name: "Frame layout", exact: true })
          .click();
        await p.waitForFunction(
          (expected) =>
            JSON.stringify(
              JSON.parse(
                document.querySelector("#preview-camera-status").dataset.pose,
              ).target,
            ) === JSON.stringify(expected),
          original.fixtures[0].position,
        );
        assert.equal((await state()).snapshot.revision, revision);
      }
      if (mode === "webgl") {
        await p.locator("#preview-source").selectOption("carpushit");
        await p.waitForFunction(() =>
          document
            .querySelector("#preview-mode")
            ?.textContent.includes("preview only"),
        );
        assert.equal(
          await p
            .getByRole("button", {
              name: "Place selected fixture",
              exact: true,
            })
            .isDisabled(),
          true,
        );
        assert.deepEqual((await state()).project, original);
      }
      assert.deepEqual(errors, []);
      await context.close();
    }
    console.log(
      "Fixture placement: Studio/mobile/GPU/WebGL numeric XYZ, layout positions, single-commit axis drag, undo and cancellation passed.",
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
