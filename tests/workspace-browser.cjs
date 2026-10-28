'use strict';
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8822';
    await page.goto(base + '/?view=studio');
    await page.locator('.workspace-panel').nth(2).waitFor();
    assert.equal(await page.locator('.workspace-panel').count(), 3);
    const revision = await page.evaluate(() =>
      fetch('/api/state')
        .then((r) => r.json())
        .then((s) => s.snapshot.revision)
    );
    const stack = () => page.locator('.workspace-panel[data-view=stack]').first();
    await stack().locator('.panel-menu summary').click();
    await stack().getByRole('button', { name: 'Split side by side', exact: true }).click();
    assert.equal(await page.locator('.workspace-panel[data-view=stack]').count(), 2);
    const stacks = page.locator('.workspace-panel[data-view=stack]');
    await stacks
      .nth(1)
      .getByRole('combobox', { name: 'Panel layer', exact: true })
      .selectOption('1');
    await stacks
      .nth(0)
      .getByRole('combobox', { name: 'Panel layer', exact: true })
      .selectOption('0');
    assert.equal(
      await stacks.nth(1).getByRole('combobox', { name: 'Panel layer', exact: true }).inputValue(),
      '1'
    );
    const divider = page.locator('.panel-divider').first(),
      bounds = await divider.boundingBox();
    await page.mouse.move(bounds.x + 2, bounds.y + bounds.height / 2);
    await page.mouse.down();
    await page.mouse.move(bounds.x + 80, bounds.y + bounds.height / 2, { steps: 8 });
    await page.mouse.up();
    const source = page.locator('.workspace-panel[data-view=stack]').first(),
      destination = page.locator('.workspace-panel[data-view=inspector]').first(),
      sourceId = await source.getAttribute('data-panel'),
      targetId = await destination.getAttribute('data-panel'),
      grip = await source.getByRole('button', { name: 'Move panel', exact: true }).boundingBox(),
      targetBounds = await destination.boundingBox();
    await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2);
    await page.mouse.down();
    await page.mouse.move(
      targetBounds.x + targetBounds.width / 2,
      targetBounds.y + targetBounds.height / 2,
      { steps: 15 }
    );
    await page.mouse.up();
    assert.equal(
      await page.locator(`[data-panel="${sourceId}"]`).getAttribute('data-view'),
      'inspector'
    );
    assert.equal(
      await page.locator(`[data-panel="${targetId}"]`).getAttribute('data-view'),
      'stack'
    );
    await page.reload();
    await page.locator('.workspace-panel[data-view=stack]').nth(1).waitFor();
    assert.deepEqual(
      (
        await page
          .locator('.workspace-panel[data-view=stack] select[aria-label="Panel layer"]')
          .evaluateAll((nodes) => nodes.map((n) => n.value))
      ).sort(),
      ['0', '1']
    );
    await page
      .locator('.workspace-panel[data-view=stack]')
      .nth(1)
      .getByRole('combobox', { name: 'View type', exact: true })
      .selectOption('scene');
    await page
      .locator('.workspace-panel[data-view=setup]')
      .getByRole('combobox', { name: 'View type', exact: true })
      .selectOption('scene');
    assert.equal(await page.locator('.workspace-panel[data-view=scene]').count(), 2);
    page.once('dialog', (d) => d.accept('Lighting'));
    await page.getByRole('button', { name: 'New workspace', exact: true }).click();
    await page.getByRole('button', { name: 'Lighting', exact: true }).waitFor();
    const scenes = page.locator('.workspace-panel[data-view=scene]');
    await scenes.nth(0).getByRole('button', { name: 'XY', exact: true }).click();
    assert.equal(await page.locator('.panel-scene[aria-label="XY scene view"]').count(), 1);
    assert.equal(await page.locator('.panel-scene[aria-label="XZ scene view"]').count(), 1);
    const beforeMax = await page.locator('.workspace-panel').count();
    await page.locator('.workspace-panel .panel-grip').first().focus();
    await page.keyboard.press('Control+Space');
    assert.equal(await page.locator('.workspace-panel').count(), 1);
    await page.keyboard.press('Control+Space');
    assert.equal(await page.locator('.workspace-panel').count(), beforeMax);
    const closePane = page.locator('.workspace-panel[data-view=scene]').last();
    await closePane.locator('.panel-menu summary').click();
    await closePane.getByRole('button', { name: 'Close panel', exact: true }).click();
    assert.equal(await page.locator('.workspace-panel').count(), beforeMax - 1);
    await page.getByRole('button', { name: 'Authoring', exact: true }).click();
    assert.equal(await page.locator('.workspace-panel').count(), beforeMax);
    await page.getByRole('button', { name: 'Lighting', exact: true }).click();
    assert.equal(await page.locator('.workspace-panel').count(), beforeMax - 1);
    const dockSource = page.locator('.workspace-panel[data-view=inspector]').first(),
      dockTarget = page.locator('.workspace-panel[data-view=stack]').first(),
      dockSourceId = await dockSource.getAttribute('data-panel'),
      dockTargetId = await dockTarget.getAttribute('data-panel'),
      dockGrip = await dockSource
        .getByRole('button', { name: 'Move panel', exact: true })
        .boundingBox(),
      dockRect = await dockTarget.boundingBox();
    await page.mouse.move(dockGrip.x + dockGrip.width / 2, dockGrip.y + dockGrip.height / 2);
    await page.mouse.down();
    await page.mouse.move(dockRect.x + 10, dockRect.y + dockRect.height / 2, { steps: 12 });
    await page.mouse.up();
    assert.ok(
      await page.evaluate(
        ([source, target]) => {
          const state = JSON.parse(localStorage.getItem('schema-workspaces-v1'));
          function paired(n) {
            return (
              n.kind === 'split' &&
              ((n.a.id === source && n.b.id === target) ||
                (n.a.id === target && n.b.id === source) ||
                paired(n.a) ||
                paired(n.b))
            );
          }
          return paired(SchemaWorkspace.active(state).root);
        },
        [dockSourceId, dockTargetId]
      ),
      'Edge docking should create the intended neighboring split'
    );
    assert.deepEqual(errors, []);
    const after = await page.evaluate(() =>
      fetch('/api/state')
        .then((r) => r.json())
        .then((s) => s.snapshot.revision)
    );
    assert.equal(after, revision, 'layout edits must not modify the scene');
    assert.ok(
      (await page
        .locator('.workspace-panel[data-view=inspector] select[aria-label="View type"]')
        .evaluate((e) => e.clientWidth)) >= 70,
      'View selector should remain readable beside pin toggle'
    );
    await page.screenshot({ path: 'target/layer-workspace-test/workspace.png', fullPage: true });
    console.log(
      'Independent duplicate panels, view switching, resizing, persistence and unchanged engine revision passed.'
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
