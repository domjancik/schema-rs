'use strict';
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
async function waitForState(predicate) {
  const deadline = Date.now() + 6000;
  while (Date.now() < deadline) {
    const state = await fetch(base + '/api/state', { cache: 'no-store' }).then((r) => r.json());
    if (predicate(state)) return state;
    await new Promise((resolve) => setTimeout(resolve, 30));
  }
  throw Error('Expected authored publication was not received');
}

(async () => {
  if (['8787', '8788', ''].includes(new URL(base).port)) throw Error('Use an isolated test engine');
  const state = await fetch(base + '/api/state').then((r) => r.json());
  assert.equal(state.project.output.enabled, false);
  const fixture = 'fixture-1',
    layer = 0,
    stack = {
      name: 'Chrome test',
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
            drivers: {
              Minimum: {
                id: 5,
                behavior: 'Create.Value.LFO',
                properties: { Period: 4, Type: 2 },
                children: [],
                drivers: {}
              }
            },
            children: [
              {
                id: 4,
                behavior: 'Create.Value.Constant',
                properties: { Value: 0.2 },
                drivers: {},
                children: []
              }
            ]
          },
          {
            id: 3,
            behavior: 'Create.Value.Constant',
            properties: { Value: 0.3 },
            drivers: {},
            children: []
          }
        ]
      }
    };
  const response = await fetch(base + '/api/command', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      expected_revision: state.snapshot.revision,
      command: { type: 'replace_layer', fixture, layer, stack }
    })
  });
  assert.equal(response.ok, true, await response.text());
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await browser.newPage({
        viewport: { width: 1450, height: 950 }
      }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto(base + '/?view=studio');
    const root = page.locator('#world .block[data-node="1"]');
    await root.locator('.node-tools .node-delete').waitFor({ timeout: 6000 });
    const { r, grab, remove, fold } = await page.evaluate(() => {
      const n = document.querySelector('#world .block[data-node="1"]');
      return {
        r: n.getBoundingClientRect().toJSON(),
        grab: n.querySelector('.node-grip').getBoundingClientRect().toJSON(),
        remove: n.querySelector('.node-delete').getBoundingClientRect().toJSON(),
        fold: n.querySelector('.fold').getBoundingClientRect().toJSON()
      };
    });
    assert.equal(await root.locator('.tree-icon').evaluate(e=>parseFloat(getComputedStyle(e).width)),14,'Category icons must not overlap the title/chrome');
    assert.ok(grab.x > r.x + r.width - 65);
    assert.ok(remove.x > grab.x);
    assert.ok(fold.x < grab.x);
    assert.equal(await page.locator('#world .block[data-node="3"] .fold').count(), 0);
    const foldRevision = (await waitForState(() => true)).snapshot.revision;
    await root.locator('.fold').click();
    assert.equal(await root.locator('.fold').getAttribute('aria-expanded'), 'false');
    assert.equal(await page.locator('#world .block').count(), 1);
    await root.locator('.fold').click();
    assert.equal(await root.locator('.fold').getAttribute('aria-expanded'), 'true');
    await page.locator('#world [data-node="4"]').waitFor();
    assert.equal(
      (await waitForState(() => true)).snapshot.revision,
      foldRevision,
      'Fold is a local view preference'
    );
    await page.locator('#world [data-node="2"] .fold').click();
    assert.equal(
      await page.locator('#world [data-node="2"] .fold').getAttribute('aria-expanded'),
      'false'
    );
    assert.equal(await page.locator('#world [data-node="4"]').count(), 0);
    assert.equal(
      await page
        .locator('#world [data-node="2"]')
        .getAttribute('class')
        .then((v) => v.includes('selected')),
      true
    );
    await page.locator('#world [data-node="2"] .fold').click();
    await page.locator('#world [data-node="4"]').waitFor();
    const before = await fetch(base + '/api/state').then((r) => r.json());
    await root.locator('.node-delete').click();
    const dialog = page.getByRole('dialog', { name: /Delete/ });
    await dialog.waitFor();
    assert.equal(
      await dialog.getByRole('button', { name: 'Delete block only', exact: true }).isDisabled(),
      true
    );
    assert.equal(
      (await fetch(base + '/api/state').then((r) => r.json())).snapshot.revision,
      before.snapshot.revision
    );
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.locator('#world .block[data-node="2"] .node-delete').click();
    await dialog.getByRole('button', { name: 'Delete block only', exact: true }).click();
    const publication = await waitForState(
      (s) =>
        JSON.stringify(s.project.content['fixture-1'][0].root.children.map((n) => n.id)) === '[4,3]'
    );
    const promoted = publication.project.content['fixture-1'][0].root;
    const nodes = [];
    const visit = (n) => {
      nodes.push(n.id);
      n.children.forEach(visit);
      Object.values(n.drivers).forEach(visit);
    };
    visit(promoted);
    assert.deepEqual(
      nodes.sort((a, b) => a - b),
      [1, 3, 4]
    );
    await page.locator('#undo').click();
    await waitForState((s) => s.project.content['fixture-1'][0].root.children[0]?.id === 2);
    await page.locator('#world .block[data-node="2"] .node-delete').click();
    await page.getByRole('button', { name: 'Delete full branch', exact: true }).click();
    await waitForState((s) => s.project.content['fixture-1'][0].root.children.length === 1);
    await page.locator('#undo').click();
    await waitForState((s) => s.project.content['fixture-1'][0].root.children.length === 2);
    await root.click({ position: { x: 35, y: 40 } });
    await page.keyboard.press('Backspace');
    await dialog.waitFor();
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    // Keep the surviving parent selected, as in the original prototype.
    await page.locator('#world [data-node="4"] .node-delete').click();
    await dialog.getByRole('button', { name: 'Delete full branch', exact: true }).click();
    await waitForState(
      (s) => s.project.content[fixture][layer].root.children[0].children.length === 0
    );
    await page.waitForFunction(
      () => document.querySelector('#world .block.selected')?.dataset.node === '2',
      null,
      { timeout: 2500 }
    );
    await page.locator('#undo').click();
    await waitForState(
      (s) => s.project.content[fixture][layer].root.children[0].children[0]?.id === 4
    );
    // The grip and block body share the original drag planner and 5px threshold.
    const dragRevision = (await waitForState(() => true)).snapshot.revision;
    await settleGrip();
    let gripBox = await page.evaluate(() =>
      document.querySelector('#world [data-node="2"] .node-grip').getBoundingClientRect().toJSON()
    );
    await page.mouse.move(gripBox.x + gripBox.width / 2, gripBox.y + gripBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(gripBox.x + gripBox.width / 2 + 3, gripBox.y + gripBox.height / 2);
    assert.equal(await page.locator('.branch-drag-ghost').count(), 0);
    await page.mouse.up();
    assert.equal((await waitForState(() => true)).snapshot.revision, dragRevision);
    async function settleGrip() {
      const published = await waitForState(() => true);
      await page.waitForFunction(
        (revision) => !ui.pending && ui.data?.snapshot.revision === revision,
        published.snapshot.revision
      );
      await page.locator('#world [data-node="2"] .node-grip').hover();
    }
    async function reorderFromGrip(shift = false) {
      await settleGrip();
      const { grip, destination } = await page.evaluate(() => ({
        grip: document
          .querySelector('#world [data-node="2"] .node-grip')
          .getBoundingClientRect()
          .toJSON(),
        destination: document
          .querySelector('#world [data-node="1"] .input-slot[data-index="2"]')
          .getBoundingClientRect()
          .toJSON()
      }));
      if (shift) await page.keyboard.down('Shift');
      await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2);
      await page.mouse.down();
      await page.mouse.move(grip.x + grip.width / 2 + 9, grip.y + grip.height / 2 + 6, {
        steps: 3
      });
      await page.locator('.branch-drag-ghost').waitFor();
      await page.mouse.move(
        destination.x + destination.width / 2,
        destination.y + destination.height / 2,
        { steps: 8 }
      );
      await page.mouse.up();
      if (shift) await page.keyboard.up('Shift');
    }
    await reorderFromGrip();
    let moved = await waitForState(
      (s) =>
        JSON.stringify(s.project.content[fixture][layer].root.children.map((n) => n.id)) === '[3,2]'
    );
    assert.deepEqual(
      moved.project.content[fixture][layer].root.children[1].children.map((n) => n.id),
      [4]
    );
    await page.locator('#undo').click();
    await waitForState(
      (s) =>
        JSON.stringify(s.project.content[fixture][layer].root.children.map((n) => n.id)) === '[2,3]'
    );
    await reorderFromGrip(true);
    moved = await waitForState(
      (s) =>
        JSON.stringify(s.project.content[fixture][layer].root.children.map((n) => n.id)) ===
        '[4,3,2]'
    );
    const movedBlock = moved.project.content[fixture][layer].root.children[2];
    assert.deepEqual(movedBlock.children, []);
    assert.equal(movedBlock.drivers.Minimum.id, 5);
    await page.locator('#undo').click();
    await waitForState(
      (s) =>
        JSON.stringify(s.project.content[fixture][layer].root.children.map((n) => n.id)) === '[2,3]'
    );
    await settleGrip();
    const cancelledRevision = (await waitForState(() => true)).snapshot.revision;
    gripBox = await page.evaluate(() =>
      document.querySelector('#world [data-node="2"] .node-grip').getBoundingClientRect().toJSON()
    );
    await page.mouse.move(gripBox.x + gripBox.width / 2, gripBox.y + gripBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(gripBox.x + gripBox.width / 2 + 12, gripBox.y + gripBox.height / 2 + 8);
    await page.locator('.branch-drag-ghost').waitFor();
    await page.keyboard.press('Escape');
    await page.mouse.up();
    assert.equal(await page.locator('.branch-drag-ghost').count(), 0);
    assert.equal((await waitForState(() => true)).snapshot.revision, cancelledRevision);
    // Another client can advance the revision while this chooser is open.
    await page.locator('#world [data-node="2"] .node-delete').click();
    await dialog.waitFor();
    const staleBefore = await waitForState(() => true),
      other = staleBefore.project.fixtures.find((f) => f.id !== fixture),
      oldName = other.name;
    const concurrent = await fetch(base + '/api/command', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        expected_revision: staleBefore.snapshot.revision,
        command: { type: 'set_fixture', fixture: { ...other, name: other.name + ' concurrent' } }
      })
    });
    assert.equal(concurrent.status, 200);
    await waitForState((s) => s.snapshot.revision > staleBefore.snapshot.revision);
    const rejected = page.waitForResponse(
      (r) => r.url().endsWith('/api/command') && r.status() === 409
    );
    await dialog.getByRole('button', { name: 'Delete full branch', exact: true }).click();
    await rejected;
    await page.waitForFunction(() =>
      document
        .querySelector('.node-delete-dialog [role=alert]')
        ?.textContent.includes('not committed')
    );
    const staleAfter = await waitForState(() => true);
    assert.deepEqual(staleAfter.project.content[fixture], staleBefore.project.content[fixture]);
    assert.equal(staleAfter.snapshot.revision, staleBefore.snapshot.revision + 1);
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await page.locator('#undo').click();
    await waitForState((s) => s.project.fixtures.find((f) => f.id === other.id).name === oldName);
    const preferenceRevision = (await fetch(base + '/api/state').then((r) => r.json())).snapshot
      .revision;
    await page.getByRole('button', { name: 'Drag as block only', exact: true }).click();
    assert.equal(
      await page
        .getByRole('button', { name: 'Drag as block only', exact: true })
        .getAttribute('aria-pressed'),
      'true'
    );
    await page.reload();
    await page.locator('#world .node-tools').first().waitFor();
    assert.equal(
      await page
        .getByRole('button', { name: 'Drag as block only', exact: true })
        .getAttribute('aria-pressed'),
      'true'
    );
    assert.equal(
      (await fetch(base + '/api/state').then((r) => r.json())).snapshot.revision,
      preferenceRevision
    );
    const touch = await browser.newContext({
      viewport: { width: 1000, height: 850 },
      hasTouch: true
    });
    const phone = await touch.newPage();
    await phone.goto(base + '/?view=studio');
    const touchDelete = phone.locator('#world .block[data-node="1"] .node-delete');
    await touchDelete.waitFor();
    assert.ok((await touchDelete.evaluate((e) => parseFloat(getComputedStyle(e).width))) >= 32);
    await touchDelete.tap();
    await phone.getByRole('dialog', { name: /Delete/ }).waitFor();
    await phone.getByRole('button', { name: 'Cancel', exact: true }).tap();
    await touch.close();
    // Inactive driver panes must render and delete the driver, not its layer root.
    await page.locator('#world [data-node="2"] strong').click();
    await page.locator('#inspector [title="Edit driver"]').click();
    await page.locator('#world [data-node="5"] .node-tools').waitFor();
    await page.locator('.workspace-panel:has(#world) .panel-menu summary').click();
    await page.getByRole('button', { name: 'Split side by side', exact: true }).click();
    await page.locator('.layer-world [data-node="5"] .node-tools').waitFor();
    assert.equal(await page.locator('.layer-world [data-node="1"]').count(), 0);
    await page.locator('.layer-world [data-node="5"] .node-delete').click();
    await dialog.getByRole('button', { name: 'Delete full branch', exact: true }).click();
    await waitForState((s) => !s.project.content[fixture][layer].root.children[0].drivers.Minimum);
    await page.locator('#world [data-node="1"]').waitFor();
    await page.locator('#undo').click();
    await waitForState(
      (s) => s.project.content[fixture][layer].root.children[0].drivers.Minimum?.id === 5
    );
    const beforePromotion = await waitForState(() => true),
      driverStack = structuredClone(beforePromotion.project.content[fixture][layer]);
    driverStack.root.children[0].drivers.Minimum = {
      id: 5,
      behavior: 'Modify.Value.MapTo',
      properties: { Minimum: 0, Maximum: 1 },
      children: [
        {
          id: 6,
          behavior: 'Create.Value.Constant',
          properties: { Value: 0.6 },
          children: [],
          drivers: {}
        }
      ],
      drivers: {
        Minimum: {
          id: 7,
          behavior: 'Create.Value.LFO',
          properties: { Period: 4, Type: 2 },
          children: [],
          drivers: {}
        }
      }
    };
    const replaced = await fetch(base + '/api/command', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        expected_revision: beforePromotion.snapshot.revision,
        command: { type: 'replace_layer', fixture, layer, stack: driverStack }
      })
    });
    assert.equal(replaced.status, 200);
    await waitForState(
      (s) =>
        s.project.content[fixture][layer].root.children[0].drivers.Minimum.children[0]?.id === 6
    );
    await page.locator('#world [data-node="2"] strong').click();
    await page.locator('#inspector [title="Edit driver"]').click();
    await page.locator('#world [data-node="5"] .fold').waitFor();
    await page.locator('#world [data-node="5"] .node-delete').click();
    await dialog.getByRole('button', { name: 'Delete block only', exact: true }).click();
    const promotedDriver = await waitForState(
      (s) => s.project.content[fixture][layer].root.children[0].drivers.Minimum.id === 6
    );
    assert.equal(
      promotedDriver.project.content[fixture][layer].root.children[0].drivers.Minimum.behavior,
      'Create.Value.Constant'
    );
    await page.locator('#world [data-node="6"].selected').waitFor();
    await page.locator('#undo').click();
    const undoDriver = await waitForState(
      (s) => s.project.content[fixture][layer].root.children[0].drivers.Minimum.id === 5
    );
    assert.deepEqual(undoDriver.project.content[fixture][layer], driverStack);
    assert.deepEqual(errors, []);
    await page.screenshot({ path: 'target/block-chrome.png', fullPage: true });
    console.log(
      'Top-right chrome, leaf folding, safe delete chooser, undo and keyboard deletion passed.'
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
