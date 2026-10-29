const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!base.endsWith(':8823')) throw Error('Use isolated frontend');
(async () => {
  const b = await chromium.launch({
    channel: 'chrome',
    headless: true,
    args: ['--enable-unsafe-webgpu', '--ignore-gpu-blocklist', '--use-angle=d3d11']
  });
  try {
    const state = await fetch(base + '/api/state').then((r) => r.json());
    assert.equal(state.project.output.enabled, false);
    for (const mode of ['studio', 'webgl', 'auto', 'mobile', 'webgl-touch']) {
      const context = await b.newContext(
          (mode === 'mobile' || mode === 'webgl-touch')
            ? { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true }
            : { viewport: { width: 1500, height: 1000 } }
        ),
        p = await context.newPage(),
        errors = [];
      let commands = 0;
      p.on('pageerror', (e) => errors.push(e.message));
      p.on('console', (m) => {
        if (m.type() === 'error') errors.push(m.text());
      });
      await p.route('**/api/state', (r) => r.fulfill({ json: state }));
      await p.route('**/api/command', (r) => {
        commands++;
        return r.abort();
      });
      if (mode === 'studio') {
        await p.goto(base + '/?view=studio');
        await p
          .locator('.workspace-panel[data-view="setup"] [aria-label="View type"]')
          .selectOption('scene');
        const panel = p.locator('.workspace-panel[data-view="scene"]').first();
        await panel.getByRole('button', { name: 'Isometric view', exact: true }).click();
        const read = () =>
          panel.locator('svg[data-camera]').evaluate((e) => JSON.parse(e.dataset.camera));
        let c = await read();
        assert(Math.abs(c.yaw - Math.PI / 4) < 1e-9);
        assert(Math.abs(c.pitch - Math.asin(1 / Math.sqrt(3))) < 1e-9);
        await panel.getByRole('button', { name: 'Rotate view right 45°', exact: true }).click();
        let n = await read();
        assert.notEqual(n.yaw, c.yaw);
        assert.equal(n.pitch, c.pitch);
        assert.deepEqual(n.target, c.target);
        await panel.getByRole('button', { name: 'Isometric view', exact: true }).focus();
        await p.keyboard.press('Space');
        assert(Math.abs((await read()).yaw - Math.PI / 4) < 1e-9);
        await panel.locator('.panel-menu>summary').click();
        await panel.getByRole('button', { name: 'Split side by side', exact: true }).click();
        const scenes = p.locator('.workspace-panel[data-view="scene"] svg[data-camera]');
        await scenes.nth(1).waitFor();
        const first = await scenes.nth(0).getAttribute('data-camera');
        await p
          .locator('.workspace-panel[data-view="scene"]')
          .nth(1)
          .getByRole('button', { name: 'Rotate view left 45°', exact: true })
          .click();
        assert.equal(await scenes.nth(0).getAttribute('data-camera'), first);
        const poses = await scenes.evaluateAll((a) => a.map((e) => e.dataset.camera));
        await p.reload();
        await p.waitForFunction(
          (expected) =>
            JSON.stringify(
              [
                ...document.querySelectorAll('.workspace-panel[data-view="scene"] svg[data-camera]')
              ].map((e) => e.dataset.camera)
            ) === JSON.stringify(expected),
          poses
        );
      } else if (mode === 'mobile') {
        await p.goto(base + '/mobile');
        const svg = p.locator('#scene[data-camera]');
        await svg.waitFor();
        await p.getByRole('button', { name: 'Isometric view', exact: true }).click();
        const old = await svg.getAttribute('data-camera');
        await p.getByRole('button', { name: 'Rotate view right 45°', exact: true }).click();
        assert.notEqual(await svg.getAttribute('data-camera'), old);
      } else {
        await p.goto(base + `/preview?backend=${mode==='webgl-touch'?'webgl':mode}&quality=low`);
        await p.waitForFunction(
          () => Number(document.querySelector('#preview-stats')?.dataset.frames) > 2
        );
        await p.getByRole('button', { name: 'Isometric view', exact: true }).click();
        const read = () =>
          p.locator('#preview-camera-status').evaluate((e) => JSON.parse(e.dataset.pose));
        let c = await read();
        assert.equal(c.projection, 'orthographic');
        const delta = c.position.map((v, i) => Math.abs(v - c.target[i]));
        assert(Math.max(...delta) - Math.min(...delta) < 1e-7);
        if(mode==='webgl-touch'){
          const canvas=p.locator('#preview-canvas');await canvas.scrollIntoViewIfNeeded();const box=await canvas.boundingBox(),session=await context.newCDPSession(p),x=box.x+box.width/2,y=Math.min(700,box.y+box.height/2);
          const touch=(type,points)=>session.send('Input.dispatchTouchEvent',{type,touchPoints:points});
          await touch('touchStart',[{x:x-20,y,id:1},{x:x+20,y,id:2}]);await touch('touchMove',[{x:x-40,y:y+10,id:1},{x:x+40,y:y+10,id:2}]);await touch('touchEnd',[]);
          await p.waitForTimeout(200);const touched=await read();assert(touched.span<c.span,'Orthographic pinch zoom should reduce visible span');assert.notDeepEqual(touched.target,c.target);
        }
        const frames = await p.locator('#preview-stats').getAttribute('data-frames');
        await p.waitForFunction(
          (f) => Number(document.querySelector('#preview-stats').dataset.frames) > Number(f) + 2,
          frames
        );
        await p.getByRole('button', { name: 'Rotate view right 45°', exact: true }).click();
        assert.notDeepEqual((await read()).position, c.position);
        await p.screenshot({ path: `target/isometric-${mode}.png` });
        if (!(await p.locator('.preview-navigation').evaluate((e) => e.open)))
          await p.locator('.preview-navigation>summary').click();
        await p.getByLabel('Camera projection', { exact: true }).selectOption('perspective');
        assert.equal((await read()).projection, 'perspective');
        await p.getByRole('button', { name: 'Isometric view', exact: true }).click();
        await p.waitForTimeout(250);
        await p.reload();
        await p.waitForFunction(
          () => Number(document.querySelector('#preview-stats')?.dataset.frames) > 2
        );
        assert.equal((await read()).projection, 'orthographic');
      }
      assert.equal(commands, 0);
      assert.deepEqual(errors, []);
      console.log(mode + ' isometric/rotation/persistence passed');
      await context.close();
    }
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
