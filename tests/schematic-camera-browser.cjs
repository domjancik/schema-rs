const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!base.endsWith(':8823')) throw Error('Use isolated frontend');
(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const frame = await fetch(base + '/api/state').then((r) => r.json());
    const page = await b.newPage({
        viewport: { width: 390, height: 844 },
        isMobile: true,
        hasTouch: true
      }),
      errors = [];
    let commands = 0;
    page.on('pageerror', (e) => errors.push(e.message));
    await page.route('**/api/state', (r) => r.fulfill({ json: frame }));
    await page.route('**/api/command', (r) => {
      commands++;
      return r.abort();
    });
    await page.goto(base + '/mobile');
    await page.locator('#scene[data-camera]').waitFor({ timeout: 5000 });
    const pose = () => page.locator('#scene').getAttribute('data-camera');
    let old = await pose();
    const box = await page.locator('#scene').boundingBox();
    await page.mouse.move(box.x + box.width * 0.35, box.y + box.height * 0.45);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.65, box.y + box.height * 0.55, { steps: 8 });
    await page.mouse.up();
    assert.notEqual(await pose(), old);
    const saved = await pose();
    await page.reload();
    await page.waitForFunction(
      (v) => document.querySelector('#scene')?.dataset.camera === v,
      saved
    );
    await page.locator('.schematic-camera summary').click();
    await page.getByRole('button', { name: 'Pan camera', exact: true }).click();
    await page.getByRole('button', { name: 'Close camera controls', exact: true }).click();
    old = await pose();
    const cdp = await page.context().newCDPSession(page),
      x = box.x + box.width * 0.5,
      y = box.y + box.height * 0.5;
    await cdp.send('Input.dispatchTouchEvent', {
      type: 'touchStart',
      touchPoints: [
        { x: x - 30, y },
        { x: x + 30, y }
      ]
    });
    await cdp.send('Input.dispatchTouchEvent', {
      type: 'touchMove',
      touchPoints: [
        { x: x - 50, y: y + 15 },
        { x: x + 50, y: y + 15 }
      ]
    });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    assert.notEqual(await pose(), old);
    const twoFinger = JSON.parse(await pose()),
      beforeTwo = JSON.parse(old);
    assert.notEqual(twoFinger.zoom, beforeTwo.zoom, 'Pinch must change zoom');
    assert.notDeepEqual(twoFinger.target, beforeTwo.target, 'Two fingers must pan');
    await page.locator('.schematic-camera summary').click();
    await page.getByRole('button', { name: 'Fit scene camera', exact: true }).click();
    await page.getByRole('button', { name: 'Close camera controls', exact: true }).click();
    const target = frame.project.fixtures[1];
    await page.locator(`[data-scene-fixture="${target.id}"] .schematic-hit`).tap();
    await page.waitForFunction(
      (name) => document.querySelector('#scene-title').textContent.includes(name),
      target.name
    );
    const selectedTitle = await page.locator('#scene-title').textContent(),
      hit = await page
        .locator(`[data-scene-fixture="${frame.project.fixtures[0].id}"] .schematic-hit`)
        .boundingBox();
    await cdp.send('Input.dispatchTouchEvent', {
      type: 'touchStart',
      touchPoints: [{ x: hit.x + 18, y: hit.y + 18, id: 1 }]
    });
    await cdp.send('Input.dispatchTouchEvent', {
      type: 'touchMove',
      touchPoints: [{ x: hit.x + 60, y: hit.y + 38, id: 1 }]
    });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    assert.equal(
      await page.locator('#scene-title').textContent(),
      selectedTitle,
      'Dragging a fixture marker navigates without selecting it'
    );
    const cancelled = await pose();
    await cdp.send('Input.dispatchTouchEvent', {
      type: 'touchStart',
      touchPoints: [{ x, y, id: 1 }]
    });
    await cdp.send('Input.dispatchTouchEvent', {
      type: 'touchMove',
      touchPoints: [{ x: x + 45, y: y + 20, id: 1 }]
    });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
    assert.equal(await pose(), cancelled, 'Cancellation restores the previous camera');
    await page.setViewportSize({ width: 320, height: 568 });
    await page.locator('.schematic-camera summary').click();
    await page.getByRole('button', { name: 'Fit scene camera', exact: true }).click();
    await page.getByRole('button', { name: 'Close camera controls', exact: true }).click();
    await page.waitForFunction(() =>
      [...document.querySelectorAll('#scene .schematic-hit')].every((e) => {
        const r = e.getBoundingClientRect();
        return document
          .elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
          ?.closest('[data-scene-fixture]');
      })
    );
    assert.equal(commands, 0);
    assert.deepEqual(errors, []);
    console.log(
      'Mobile schematic direct drag, two-finger pan/pinch, pose persistence and read-only navigation passed'
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
