const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = 'http://127.0.0.1:8823';
(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    for (const mobile of [false, true]) {
      const c = await b.newContext(
          mobile
            ? { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true }
            : { viewport: { width: 1550, height: 1000 } }
        ),
        p = await c.newPage(),
        data = await fetch(base + '/api/state').then((r) => r.json()),
        events = [],
        errors = [];
      assert.equal(data.project.output.enabled, false);
      p.on('pageerror', (e) => errors.push(e.message));
      await p.route('**/api/state', (r) => r.fulfill({ json: data }));
      await p.route('**/api/inputs', (r) => {
        const e = JSON.parse(r.request().postData());
        if (e.type === 'channel_test') {
          events.push(e);
          data.snapshot.channel_test = Object.keys(e.channels).length
            ? { universe: e.universe, channels: e.channels }
            : null;
        }
        return r.fulfill({ json: { ok: true } });
      });
      await p.route('**/api/command', () => {
        throw Error('Channel testing must not author commands');
      });
      await p.goto(base + (mobile?'/mobile':'/?view=studio'));
      await p.locator('#output-design').click();
      const d = p.locator('#output-design-dialog');
      await d.getByRole('button', { name: 'Channel test', exact: true }).click();
      assert(await d.getByLabel('Test channel 1', { exact: true }).isDisabled());
      await d.getByRole('button', { name: 'Arm channel test', exact: true }).click();
      const slider = d.getByLabel('Test channel 1', { exact: true });
      if(mobile){await slider.scrollIntoViewIfNeeded();const r=await slider.boundingBox();await p.touchscreen.tap(r.x+r.width*.7,r.y+r.height/2);await p.waitForTimeout(120);assert(events.at(-1).channels['1']>0);}

      await slider.evaluate((n) => {
        n.value = '128';
        n.dispatchEvent(new Event('input', { bubbles: true }));
        n.dispatchEvent(new Event('change', { bubbles: true }));
      });
      await p.waitForTimeout(120);
      assert.equal(events.at(-1).channels['1'], 128);
      assert.equal(events.at(-1).universe, 0);
      await d.getByLabel('Test master', { exact: true }).evaluate((n) => {
        n.value = '50';
        n.dispatchEvent(new Event('input', { bubbles: true }));
        n.dispatchEvent(new Event('change', { bubbles: true }));
      });
      await p.waitForTimeout(100);
      assert.equal(events.at(-1).channels['1'], 64);
      await d.getByRole('button', { name: 'Zero tested channels', exact: true }).click();
      await p.waitForTimeout(80);
      assert.equal(events.at(-1).channels['1'], 0);
      await d.getByRole('button', { name: 'Release channel 1', exact: true }).click();
      await p.waitForTimeout(80);
      assert.deepEqual(events.at(-1).channels, {});
      await d.getByRole('button', { name: 'Release test', exact: true }).click();
      assert(await slider.isDisabled());
      await d.getByLabel('Test universe', { exact: true }).fill('7');
      await d.getByLabel('Test universe', { exact: true }).press('Tab');
      await d.getByLabel('First test channel', { exact: true }).fill('497');
      await d.getByLabel('First test channel', { exact: true }).press('Tab');
      await d.getByRole('button', { name: 'Arm channel test', exact: true }).click();
      await d.getByLabel('Test channel 512', { exact: true }).evaluate((n) => {
        n.value = '255';
        n.dispatchEvent(new Event('change', { bubbles: true }));
        n.dispatchEvent(new Event('input', { bubbles: true }));
      });
      await p.waitForTimeout(120);
      assert.equal(events.at(-1).universe, 7);
      assert.equal(events.at(-1).channels['512'], 128);
      await p.keyboard.press('Escape');
      await p.waitForTimeout(100);
      assert.deepEqual(events.at(-1).channels, {});
      if(!mobile){const paneId=await p.locator('.workspace-panel[data-view="inspector"]').getAttribute('data-panel');const panel=p.locator(`[data-panel="${paneId}"]`);await panel.getByLabel('View type',{exact:true}).selectOption('dmxtest');await panel.getByRole('button',{name:'Arm channel test',exact:true}).click();await panel.getByLabel('Test channel 2',{exact:true}).evaluate(n=>{n.value='90';n.dispatchEvent(new Event('input',{bubbles:true}))});await p.waitForTimeout(120);assert.equal(events.at(-1).channels['2'],90);await panel.getByLabel('View type',{exact:true}).selectOption('inspector');await p.waitForTimeout(120);assert.deepEqual(events.at(-1).channels,{});}
      assert.deepEqual(errors, []);
      console.log(
        (mobile ? 'Mobile' : 'Desktop') +
          ' arm/sliders/master/zero/release/universe/end-channel/close passed'
      );
      await c.close();
    }
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
