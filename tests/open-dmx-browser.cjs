const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!base.endsWith(':8823')) throw Error('Use isolated frontend');
(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const p = await b.newPage(),
      data = await fetch(base + '/api/state').then((r) => r.json()),
      errors = [];
    assert.equal(data.project.output.enabled, false);
    let payload,
      linux = false;
    p.on('pageerror', (e) => errors.push(e.message));
    await p.route('**/api/state', (r) => r.fulfill({ json: data }));
    await p.route('**/api/output/devices', (r) =>
      r.fulfill({
        json: linux
          ? { open_dmx: false, devices: [], error: 'Use Art-Net/OLA on Linux' }
          : {
              open_dmx: true,
              devices: [{ serial: 'DMX123', description: 'Open DMX USB' }],
              error: null
            }
      })
    );
    await p.route('**/api/command', (r) => {
      payload = JSON.parse(r.request().postData());
      assert.equal(payload.command.type, 'set_output');
      assert.equal(payload.command.config.enabled, false);
      data.project.output = payload.command.config;
      data.snapshot.revision++;
      return r.fulfill({ json: { revision: data.snapshot.revision } });
    });
    await p.goto(base + '/?view=studio');
    await p.locator('#world .block[data-node]').first().waitFor();
    await p.getByRole('button', { name: 'Output design', exact: true }).click();
    await p
      .locator('#output-design-dialog')
      .getByRole('button', { name: 'Connection', exact: true })
      .click();
    const d = p.locator('#output-design-dialog');
    await d
      .getByLabel('USB adapter', { exact: true })
      .locator('option[value="DMX123"]')
      .waitFor({ state: 'attached' });
    await d.getByLabel('Output transport', { exact: true }).selectOption('open_dmx');
    await d.getByLabel('USB adapter', { exact: true }).selectOption('DMX123');
    await d.getByLabel('USB universe (zero based)', { exact: true }).fill('7');
    await d.getByRole('button', { name: 'Save output connection' }).click();
    await p.waitForFunction(() => ui.data.project.output.transport === 'open_dmx');
    assert.equal(payload.command.config.device, 'DMX123');
    assert.equal(payload.command.config.universe, 7);
    assert.equal(payload.command.config.fps, 30);
    assert.equal(payload.expected_revision, data.snapshot.revision - 1);
    linux = true;
    await d.getByRole('button', { name: 'Refresh USB adapters' }).click();
    await d.getByText('Use Art-Net/OLA on Linux', { exact: true }).waitFor();
    await p.waitForFunction(
      () =>
        document.querySelector(
          '#output-design-dialog [aria-label="Output transport"] option[value="open_dmx"]'
        ).disabled
    );
    assert.equal(await d.getByLabel('Output transport', { exact: true }).inputValue(), 'open_dmx');
    await d.getByLabel('Output transport', { exact: true }).selectOption('art_net');
    await d.getByRole('button', { name: 'Save output connection' }).click();
    await p.waitForFunction(
      () => !ui.data.project.output.transport || ui.data.project.output.transport === 'art_net'
    );
    assert.equal(payload.command.config.target, '127.0.0.1:6454');
    assert.deepEqual(errors, []);
    console.log(
      'USB discovery/serial/universe/FPS, expected-revision disabled save, Linux capability and Art-Net switch passed'
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
