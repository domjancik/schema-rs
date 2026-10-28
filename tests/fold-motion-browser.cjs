const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!base.endsWith(':8823')) throw Error('Use isolated output-disabled frontend');
(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const p = await b.newPage({ viewport: { width: 1500, height: 1000 } }),
      data = await fetch(base + '/api/state').then((r) => r.json()),
      errors = [];
    assert.equal(data.project.output.enabled, false);
    let commands = 0;
    const leaf = (id) => ({
      id,
      behavior: 'Create.Value.Constant',
      properties: { Value: 0.5 },
      children: [],
      drivers: {}
    });
    data.project.content['fixture-1'][0] = {
      name: 'Fold motion test',
      root: {
        id: 1,
        behavior: 'Combine.Add',
        properties: {},
        drivers: {},
        children: [
          {
            id: 2,
            behavior: 'Modify.Value.MapTo',
            properties: { Minimum: 0, Maximum: 1 },
            drivers: {},
            children: [leaf(4)]
          },
          {
            id: 3,
            behavior: 'Combine.Multiply',
            properties: {},
            drivers: {},
            children: [leaf(5), leaf(6)]
          }
        ]
      }
    };
    p.on('pageerror', (e) => errors.push(e.message));
    await p.route('**/api/state', (r) => r.fulfill({ json: data }));
    await p.route('**/api/command', (r) => {
      commands++;
      return r.abort();
    });
    await p.goto(base + '/?view=studio');
    const fold = () => p.locator('#world .block[data-node="1"] .fold');
    await fold().waitFor();
    await fold().hover();
    const pin = await fold().boundingBox();
    await fold().click();
    await p.locator('#world[data-fold-animating]').waitFor({ state: 'attached', timeout: 2500 });
    assert.equal(await p.locator('#world .block[data-node]').count(), 1);
    assert((await p.locator('.fold-exit-ghost').count()) > 0);
    assert.equal(await p.locator('.fold-exit-ghost[data-node]').count(), 0);
    assert(
      await p
        .locator('.fold-exit-ghost')
        .evaluateAll((a) => a.every((e) => e.inert && e.getAttribute('aria-hidden') === 'true'))
    );
    const after = await fold().boundingBox();
    assert(
      Math.abs(after.x + after.width / 2 - pin.x - pin.width / 2) < 2 &&
        Math.abs(after.y + after.height / 2 - pin.y - pin.height / 2) < 2,
      'Fold control stays pinned'
    );
    for (let i = 0; i < 3; i++) {
      await p.mouse.click(pin.x + pin.width / 2, pin.y + pin.height / 2);
      await p.waitForTimeout(25);
    }
    assert.equal(await fold().getAttribute('aria-expanded'), 'true');
    await p.waitForFunction(
      () => !document.querySelector('#world')?.hasAttribute('data-fold-animating')
    );
    assert.equal(await p.locator('.fold-exit-ghost,.fold-motion-layer').count(), 0);
    assert.equal(await p.locator('#world .block[data-node]').count(), 6);
    assert.equal(await p.locator('#world .block[inert]').count(), 0);
    await fold().focus();
    await p.keyboard.press('Space');
    assert.equal(await fold().getAttribute('aria-expanded'), 'false');
    await p.keyboard.press('Enter');
    assert.equal(await fold().getAttribute('aria-expanded'), 'true');
    await p.waitForFunction(
      () => !document.querySelector('#world')?.hasAttribute('data-fold-animating')
    );
    await fold().click();
    await p.getByRole('button', { name: 'Edit layer 2', exact: true }).click();
    assert.equal(await p.locator('.fold-motion-layer').count(), 0);
    assert.equal(await p.locator('#world[data-fold-animating]').count(), 0);
    await p.getByRole('button', { name: 'Edit layer 1', exact: true }).click();
    await fold().click();
    assert.equal(await fold().getAttribute('aria-expanded'), 'true');
    await p.emulateMedia({ reducedMotion: 'reduce' });
    assert.equal(await p.locator('.fold-motion-layer').count(), 0);
    await fold().click();
    assert.equal(await p.locator('.fold-motion-layer').count(), 0);
    assert.equal(await p.locator('#world[data-fold-animating]').count(), 0);
    assert.equal(await p.locator('#world .block[data-node]').count(), 1);
    assert.equal(commands, 0);
    assert.deepEqual(errors, []);
    console.log(
      'Fold exits/reveal, pinned controls, rapid reversals, cleanup, reduced motion and zero authored commands passed'
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
