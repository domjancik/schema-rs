const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!base.endsWith(':8823')) throw Error('Use isolated frontend');
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await browser.newPage({
        viewport: { width: 1600, height: 1050 },
        reducedMotion: 'reduce'
      }),
      data = await fetch(base + '/api/state').then((r) => r.json()),
      errors = [];
    assert.equal(data.project.output.enabled, false);
    const leaf = (id) => ({
      id,
      behavior: 'Create.Value.Constant',
      properties: { Value: 0.5 },
      drivers: {},
      children: []
    });
    data.project.content['fixture-1'][0] = {
      name: 'Fractal test',
      root: {
        ...leaf(1),
        behavior: 'Combine.Add',
        children: [
          { ...leaf(2), behavior: 'Combine.Multiply', children: [leaf(4), leaf(5)] },
          leaf(3)
        ]
      }
    };
    let commands = 0;
    page.on('pageerror', (e) => errors.push(e.message));
    await page.route('**/api/state', (r) => r.fulfill({ json: data }));
    await page.route('**/api/command', (r) => {
      commands++;
      return r.abort();
    });
    await page.goto(base + '/?view=studio');
    await page.locator('#world .block[data-node="1"]').waitFor();
    const active = () => page.locator('.workspace-panel.active');
    async function choose(label, value, panel = active()) {
      await panel.locator('.stack-layout-menu>summary').click();
      await panel.getByLabel(label, { exact: true }).selectOption(String(value));
      await panel
        .locator(`[data-${label === 'Sibling layout' ? 'fractal' : 'square'}="${value}"]`)
        .waitFor({ state: 'attached' });
    }
    const rects = () =>
      page.locator('#world .block[data-node]').evaluateAll((nodes) =>
        Object.fromEntries(
          nodes.map((n) => [
            n.dataset.node,
            (() => {
              const r = n.getBoundingClientRect();
              return { x: r.x, y: r.y, w: r.width, h: r.height };
            })()
          ])
        )
      );
    const near = (a, b) => assert(Math.abs(a - b) < 1, `${a} != ${b}`);
    await choose('Sibling layout', true);
    let r = await rects();
    near(r[2].w, r[3].w);
    near(r[2].x, r[1].x);
    near(r[3].x + r[3].w, r[1].x + r[1].w);
    near(r[4].w, r[5].w);
    assert(r[4].w < r[2].w);
    near(r[2].y + r[2].h, r[1].y);
    await choose('Block shape', true);
    r = await rects();
    for (const a of Object.values(r)) near(a.w, a.h);
    // A scaled grip projects a ghost at the exact indexed seam, then cancels without a command.
    const drag = await page.evaluate(() => {
      const box = (q) => {
        const r = document.querySelector(q).getBoundingClientRect();
        return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
      };
      return {
        from: box('#world .block[data-node="3"] .node-grip'),
        to: box('#world .block[data-node="2"] .input-slot[data-index="1"]')
      };
    });
    await page.mouse.move(drag.from.x, drag.from.y);
    await page.mouse.down();
    await page.mouse.move(drag.to.x, drag.to.y, { steps: 8 });
    await page.locator('.branch-drag-ghost[data-valid="true"]').waitFor();
    assert(await page.locator('#world .drop-ghost[data-node="3"]').count());
    await page.keyboard.press('Escape');
    await page.mouse.up();
    assert.equal(await page.locator('.branch-drag-ghost').count(), 0);
    await page.screenshot({ path: 'target/fractal-square.png' });
    // Indexed add targets still open the picker on the selected block.
    await page.locator('#world .block[data-node="2"] .input-slot[data-index="1"]').click();
    await page.locator('dialog[open]').waitFor();
    await page.keyboard.press('Escape');
    // Fold remains keyboard usable and preserves recursive geometry.
    await page.locator('#world .block[data-node="2"] .fold').focus();
    await page.keyboard.press('Enter');
    assert.equal(await page.locator('#world .block[data-node]').count(), 3);
    await page.keyboard.press('Enter');
    assert.equal(await page.locator('#world .block[data-node]').count(), 5);
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    const fold = page.locator('#world .block[data-node="2"] .fold');
    const pin = await fold.boundingBox();
    await fold.click();
    await page.locator('#world[data-fold-animating]').waitFor({ state: 'attached' });
    const pinned = await fold.boundingBox();
    near(pin.x, pinned.x);
    near(pin.y, pinned.y);
    await page.locator('#world[data-fold-animating]').waitFor({ state: 'detached' });
    await fold.click();
    await page.locator('#world[data-fold-animating]').waitFor({ state: 'detached' });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.reload();
    await page
      .locator('#world[data-fractal="true"][data-square="true"]')
      .waitFor({ state: 'attached' });
    r = await rects();
    near(r[1].w, r[1].h);
    await active().locator('.panel-menu>summary').click();
    await active().getByRole('button', { name: 'Split side by side', exact: true }).click();
    assert.equal(await page.locator('.workspace-panel[data-view="stack"]').count(), 2);
    const inactive = page.locator('.workspace-panel[data-view="stack"]:not(.active)');
    await choose('Sibling layout', false, inactive);
    assert.equal(await page.locator('#world').getAttribute('data-fractal'), 'true');
    assert.equal(await page.locator('.layer-world').getAttribute('data-fractal'), 'false');
    await choose('Sibling layout', false);
    r = await rects();
    for (const a of Object.values(r)) near(a.w, a.h);
    assert(r[1].w > r[3].w);
    // A short viewport must fit the entire expanded square tree, even below25% zoom.
    await page.setViewportSize({ width: 1600, height: 660 });
    await page.locator('#fit').click();
    const fitBounds = await page.evaluate(() => {
      const canvas = document.querySelector('#canvas'),
        c = canvas.getBoundingClientRect();
      return {
        canvas: { x: c.x, y: c.y, right: c.right, bottom: c.bottom },
        cards: [...document.querySelectorAll('#world .block[data-node]')].map((n) => {
          const r = n.getBoundingClientRect();
          return { x: r.x, y: r.y, right: r.right, bottom: r.bottom };
        }),
        scale: ui.scale,
        scroll: canvas.scrollTop
      };
    });
    assert(fitBounds.scale < 0.25);
    assert.equal(fitBounds.scroll, 0);
    for (const r of fitBounds.cards) {
      assert(r.x >= fitBounds.canvas.x && r.right <= fitBounds.canvas.right);
      assert(r.y >= fitBounds.canvas.y && r.bottom <= fitBounds.canvas.bottom);
    }
    await page.reload();
    await page.locator('#world .block[data-node]').first().waitFor();
    assert.equal(await page.locator('#world').getAttribute('data-square'), 'true');
    await choose('Block shape', false);
    r = await rects();
    assert(r[1].w > r[1].h);
    assert.equal(commands, 0);
    assert.deepEqual(errors, []);
    console.log(
      'Fractal/square geometry, indexed picker, keyboard fold, saved independent panels and zero authored commands passed'
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
