const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = 'http://127.0.0.1:8823';
(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const p = await b.newPage({ viewport: { width: 1600, height: 1100 }, reducedMotion: 'reduce' }),
      data = await fetch(base + '/api/state').then((r) => r.json()),
      errors = [];
    assert.equal(data.project.output.enabled, false);
    let commands = [];
    const leaf = (id) => ({
      id,
      behavior: 'Create.Value.Constant',
      properties: { Value: 0.5 },
      children: [],
      drivers: {}
    });
    data.project.fixtures = data.project.fixtures.slice(0, 2);data.snapshot.fixtures=data.snapshot.fixtures.filter(f=>data.project.fixtures.some(p=>p.id===f.fixture.id));
    data.project.content['fixture-1'][0] = {
      name: 'Embed A',
      root: { ...leaf(1), behavior: 'Combine.Add', children: [leaf(2)] }
    };
    data.project.content['fixture-2'][0] = {
      name: 'Embed B',
      root: { ...leaf(1), behavior: 'Combine.Add', children: [leaf(2)] }
    };
    p.on('pageerror', (e) => errors.push(e.message));
    await p.route('**/api/state', (r) => r.fulfill({ json: data }));
    await p.route('**/api/command', (r) => {
      commands.push(JSON.parse(r.request().postData()));
      return r.abort();
    });
    await p.goto(base + '/?view=studio');
    await p
      .locator('.workspace-panel[data-view=setup] [aria-label="View type"]')
      .selectOption('scene');
    const panel = p.locator('.workspace-panel[data-view=scene]').first();
    await panel.getByLabel('Stacks in scene', { exact: true }).selectOption('all');
    await p.waitForFunction(() => document.querySelectorAll('.scene-stack').length === 2);
    let group = panel.locator('[data-scene-stack-fixture="fixture-2"]');
    await group.locator('.block[data-node="2"]').click();
    assert.equal(await p.evaluate(() => ui.fixture), 'fixture-2');
    assert.equal(await p.evaluate(() => ui.node), 2);
    await p.locator('#inspector input[aria-label="Value"]').fill('.7');
    await p.locator('#inspector input[aria-label="Value"]').press('Tab');
    await p.waitForFunction(() => !ui.pending);
    assert.equal(commands.length, 1);
    assert.equal(commands[0].command.fixture, 'fixture-2');
    assert.equal(commands[0].command.node, 2);
    assert.equal(commands[0].command.type, 'set_property');
    assert.equal(commands[0].expected_revision, data.snapshot.revision);
    commands = [];
    await group.locator('.input-slot').first().click();
    await p.locator('#block-picker-dialog[open]').waitFor();
    await p.keyboard.press('Escape');
    await group.locator('.fold').click();
    assert.equal(await group.locator('.block').count(), 1);
    await group.locator('.fold').click();
    assert.equal(await group.locator('.block').count(), 2);
    await panel.getByRole('button', { name: 'Isometric view', exact: true }).click();
    group = panel.locator('[data-scene-stack-fixture="fixture-2"]');
    await group.waitFor();
    const before = await group.locator('.layer-world').getAttribute('style');
    await panel.getByRole('button', { name: 'Rotate view right 45°', exact: true }).click();
    await p.waitForFunction(
      (old) =>
        document
          .querySelector('[data-scene-stack-fixture="fixture-2"] .layer-world')
          .getAttribute('style') !== old,
      before
    );
    await p.screenshot({ path: 'target/scene-stacks-iso.png' });
    await panel.getByLabel('Stacks in scene', { exact: true }).selectOption('selected');
    await p.waitForFunction(() => document.querySelectorAll('.scene-stack').length === 1);
    assert.equal(
      await panel.locator('.scene-stack').getAttribute('data-scene-stack-fixture'),
      'fixture-2'
    );
    await p.reload();
    await p.locator('.scene-stack').waitFor();
    assert.equal(await p.getByLabel('Stacks in scene', { exact: true }).inputValue(), 'selected');
    await p.getByLabel('Stacks in scene', { exact: true }).selectOption('off');
    assert.equal(await p.locator('.scene-stack').count(), 0);
    assert.deepEqual(errors, []);
    assert.equal(commands.length, 0);
    console.log(
      'Scene-stack all/selected/off, fixture-specific selection, add/fold, camera tracking and reload passed'
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
