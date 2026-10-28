'use strict';
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1550, height: 950 } });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8822';
    if (['8787', '8788', ''].includes(new URL(base).port))
      throw Error('Use an isolated test engine');
    const initial = await fetch(base + '/api/state').then((r) => r.json());
    assert.equal(initial.project.output.enabled, false);
    const project = structuredClone(initial.project);
    const originalRoot = structuredClone(project.content['fixture-1'][0].root);
    for (const layers of Object.values(project.content)) layers.splice(3);
    project.director.splice(3);
    if (project.content['fixture-1'][0]) delete project.content['fixture-1'][0].enabled;
    await fetch(base + '/api/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        expected_revision: initial.snapshot.revision,
        command: { type: 'replace_project', project }
      })
    }).then((r) => {
      assert.equal(r.ok, true);
    });
    await page.goto(base + '/?view=studio');
    await page
      .getByRole('button', { name: 'Split layers', exact: true })
      .waitFor({ timeout: 7000 });
    await page.getByRole('button', { name: 'Split layers', exact: true }).click();
    await page.locator('.layer-viewport').nth(2).waitFor();
    assert.equal(await page.locator('.layer-viewport').count(), 3);
    const revision = await page.evaluate(() =>
      fetch('/api/state')
        .then((r) => r.json())
        .then((s) => s.snapshot.revision)
    );
    const committed=page.waitForResponse(response=>response.url().endsWith('/api/command')&&response.request().method()==='POST'&&JSON.parse(response.request().postData()).command.type==='set_layer_enabled');
    await page.getByRole('checkbox', { name: 'Enable layer 1', exact: true }).first().uncheck();
    const muteResponse=await committed;assert.equal(muteResponse.ok(),true,await muteResponse.text());
    await page.waitForFunction(()=>fetch('/api/state').then(r=>r.json()).then(s=>s.project.content['fixture-1'][0].enabled===false));
    let state = await page.evaluate(() => fetch('/api/state').then((r) => r.json()));
    assert.equal(state.project.content['fixture-1'][0].enabled, false);
    assert.deepEqual(state.project.content['fixture-1'][0].root, originalRoot);
    await page.locator('#undo').click();
    await page.waitForFunction(() =>
      fetch('/api/state')
        .then((r) => r.json())
        .then((s) => s.project.content['fixture-1'][0].enabled !== false)
    );
    await page.getByRole('button', { name: 'Add layer', exact: true }).click();
    await page.locator('.layer-viewport').nth(3).waitFor();
    assert.equal(await page.locator('.layer-viewport').count(), 4);
    const fourth = page.locator('.layer-viewport').nth(3);
    await fourth.getByRole('button', { name: 'Create stack', exact: false }).click();
    await page.locator('dialog[open]').waitFor();
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Single layer', exact: true }).click();
    assert.equal(await page.locator('.layer-viewport').count(), 1);
    await page.getByRole('button', { name: 'Split layers', exact: true }).click();
    await page.locator('.layer-viewport').nth(3).waitFor();
    assert.deepEqual(errors, []);
    await page.screenshot({ path: 'target/layer-workspace-test/split.png', fullPage: true });
    console.log('Split view, mute/undo, add/focus/create, view persistence passed.');
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
