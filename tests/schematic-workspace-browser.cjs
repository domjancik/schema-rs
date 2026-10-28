const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!base.endsWith(':8823')) throw Error('Use isolated frontend');
(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await b.newPage({ viewport: { width: 1500, height: 950 } }),
      publication = await fetch(base + '/api/state').then((r) => r.json()),
      errors = [];
    let commands = 0;
    page.on('pageerror', (e) => errors.push(e.message));
    await page.route('**/api/state', (r) => r.fulfill({ json: publication }));
    await page.route('**/api/command', (r) => {
      commands++;
      return r.abort();
    });
    await page.route('**/preview?panel=*', (r) =>
      r.fulfill({ contentType: 'text/html', body: '<h1>Preview instance</h1>' })
    );
    await page.goto(base + '/?view=studio');
    await page
      .locator('.workspace-panel[data-view=setup]')
      .getByRole('combobox', { name: 'View type', exact: true })
      .selectOption('scene');
    const pane = () => page.locator('.workspace-panel[data-view=scene]').first();
    await pane().getByRole('button', { name: '3D', exact: true }).click();
    await pane().locator('svg[data-camera]').waitFor();
    await page
      .locator('.workspace-panel[data-view=inspector]')
      .getByRole('combobox', { name: 'View type', exact: true })
      .selectOption('gpu');
    await page.locator('iframe.panel-preview').waitFor();
    await page.evaluate(
      () => (window.stablePreview = document.querySelector('iframe.panel-preview'))
    );
    const id = await pane().getAttribute('data-panel'),
      view = () => page.locator(`[data-panel="${id}"] svg[data-camera]`);
    let old = await view().getAttribute('data-camera');
    let box = await view().boundingBox();
    await page.mouse.move(box.x + box.width * 0.2, box.y + box.height * 0.55);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.6, box.y + box.height * 0.65, { steps: 10 });
    await page.mouse.up();
    assert.notEqual(await view().getAttribute('data-camera'), old);
    assert(
      await page.evaluate(
        () => window.stablePreview === document.querySelector('iframe.panel-preview')
      ),
      'Camera navigation must preserve neighboring GPU instances'
    );
    await page.locator(`[data-panel="${id}"] .panel-menu summary`).click();
    await page.getByRole('button', { name: 'Split side by side', exact: true }).click();
    let views = page.locator('.workspace-panel[data-view=scene] svg[data-camera]');
    await views.nth(1).waitFor();
    const first = await views.nth(0).getAttribute('data-camera');
    await views.nth(1).focus();
    await page.keyboard.press('ArrowRight');
    views = page.locator('.workspace-panel[data-view=scene] svg[data-camera]');
    assert.equal(await views.nth(0).getAttribute('data-camera'), first);
    assert.notEqual(await views.nth(1).getAttribute('data-camera'), first);
    const oneStep = await views.nth(1).getAttribute('data-camera');
    await page.keyboard.press('ArrowRight');
    assert.notEqual(
      await views.nth(1).getAttribute('data-camera'),
      oneStep,
      'Repeated keyboard navigation retains camera focus'
    );
    await page.keyboard.press('Delete');
    await page.keyboard.press('Backspace');
    const poses = await views.evaluateAll((a) => a.map((e) => e.dataset.camera));
    await page.reload();
    await page.waitForFunction(
      (values) =>
        JSON.stringify(
          [...document.querySelectorAll('.workspace-panel[data-view=scene] svg[data-camera]')].map(
            (e) => e.dataset.camera
          )
        ) === JSON.stringify(values),
      poses
    );
    assert.equal(commands, 0);
    assert.deepEqual(errors, []);
    console.log(
      'Desktop 3D drag/keyboard, independent duplicate Scene cameras, saved workspaces and zero authored mutations passed'
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
