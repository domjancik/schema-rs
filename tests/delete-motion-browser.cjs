const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = 'http://127.0.0.1:8823';
(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const p = await b.newPage({ viewport: { width: 1500, height: 1000 } }),
      data = await fetch(base + '/api/state').then((r) => r.json()),
      errors = [];
    assert.equal(data.project.output.enabled, false);
    const leaf = (id) => ({
        id,
        behavior: 'Create.Value.Constant',
        properties: { Value: 0.5 },
        children: [],
        drivers: {}
      }),
      initial = {
        name: 'Deletion motion',
        root: {
          ...leaf(1),
          behavior: 'Combine.Add',
          children: [{ ...leaf(2), behavior: 'Modify.Value.MapTo', children: [leaf(4)] }, leaf(3)]
        }
      };
    data.project.content['fixture-1'][0] = structuredClone(initial);
    let commands = [],
      fail = false;
    p.on('pageerror', (e) => errors.push(e.message));
    await p.route('**/api/state', (r) => r.fulfill({ json: data }));
    await p.route('**/api/command', (r) => {
      const body = JSON.parse(r.request().postData());
      commands.push(body);
      if (fail) return r.fulfill({ status: 409, json: { error: 'Revision conflict' } });
      assert.equal(body.expected_revision, data.snapshot.revision);
      const c = body.command;
      if (c.type === 'undo') data.project.content['fixture-1'][0] = structuredClone(initial);
      else {
        assert.equal(c.type, 'delete_node');
        if (c.node === 1) data.project.content['fixture-1'][0] = null;
        else
          data.project.content['fixture-1'][0].root.children = c.branch
            ? [leaf(3)]
            : [leaf(4), leaf(3)];
      }
      data.snapshot.revision++;
      return r.fulfill({ json: { revision: data.snapshot.revision } });
    });
    await p.goto(base + '/?view=studio');
    await p.locator('#world [data-node="2"]').waitFor();
    await p
      .locator('.workspace-panel[data-view="setup"] [aria-label="View type"]')
      .selectOption('scene');
    await p.getByLabel('Stacks in scene', { exact: true }).selectOption('all');
    await p.locator('[data-scene-stack-fixture="fixture-1"] .block[data-node="2"]').waitFor();

    async function remove(id, branch) {
      await p.locator(`#world [data-node="${id}"] .node-delete`).click();
      await p
        .getByRole('button', {
          name: branch ? 'Delete full branch' : 'Delete block only',
          exact: true
        })
        .click();
    }
    async function undo() {
      await p.waitForFunction(() => !ui.pending);
      await p.evaluate(() => command({ type: 'undo' }));
      await p.locator('#world [data-node="2"]').waitFor();
      await p.waitForFunction(() => !ui.pending);
    }
    await p.evaluate(() => {
      window.pauseDelete = true;
      const play = SchemaFoldMotion.play;
      SchemaFoldMotion.play = (world, before, options) => {
        const result = play(world, before, options);
        if (options.deleting && window.pauseDelete)
          for (const animation of world.getAnimations({ subtree: true })) animation.pause();
        return result;
      };
    });
    await remove(2, false);
    await p.locator('#world[data-delete-animating]').waitFor({ state: 'attached' });
    const simultaneous = await p.evaluate(() => ({
      main: document.querySelectorAll('#world .fold-exit-ghost').length,
      scene: document.querySelectorAll('[data-scene-stack-fixture="fixture-1"] .fold-exit-ghost')
        .length
    }));
    assert.equal(simultaneous.main, 1);
    assert.equal(simultaneous.scene, 1);
    assert.equal(await p.locator('#world .fold-exit-ghost[data-node]').count(), 0);
    assert(
      await p
        .locator('#world .fold-exit-ghost')
        .evaluate((n) => n.inert && n.getAttribute('aria-hidden') === 'true')
    );
    assert.equal(await p.locator('#world .block[data-node="2"]').count(), 0);
    assert(
      await p
        .locator('#world .block[data-node="1"]')
        .evaluate((n) => n.getAnimations().some((a) => a.effect.getTiming().delay >= 160))
    );
    await p.evaluate(() => {
      window.pauseDelete = false;
      for (const animation of document.getAnimations()) animation.play();
    });
    await p.locator('#world[data-delete-animating]').waitFor({ state: 'detached' });
    assert.equal(await p.locator('.fold-motion-layer').count(), 0);
    assert.equal(commands.length, 1);
    await undo();
    await remove(2, true);
    await p.locator('#world[data-delete-animating]').waitFor({ state: 'attached' });
    assert.equal(await p.locator('#world .fold-exit-ghost').count(), 2);
    await p.emulateMedia({ reducedMotion: 'reduce' });
    await p.waitForFunction(() => document.querySelectorAll('.fold-motion-layer').length === 0);
    await undo();
    await remove(2, false);
    assert.equal(await p.locator('#world[data-delete-animating]').count(), 0);
    await undo();
    fail = true;
    await remove(2, true);
    await p
      .locator('.node-delete-dialog [role=alert]')
      .filter({ hasText: 'not committed' })
      .waitFor();
    assert.equal(await p.locator('#world [data-node="2"]').count(), 1);
    assert.equal(await p.locator('#world[data-delete-animating]').count(), 0);
    await p.getByRole('button', { name: 'Cancel', exact: true }).click();
    fail = false;
    await p.emulateMedia({ reducedMotion: 'no-preference' });
    await remove(1, true);
    await p.locator('#world[data-delete-animating]').waitFor({ state: 'attached' });
    assert.equal(await p.locator('#world .fold-exit-ghost').count(), 4);
    await p.locator('#world[data-delete-animating]').waitFor({ state: 'detached' });
    assert.equal(await p.locator('#world .block[data-node]').count(), 0);
    assert.deepEqual(errors, []);
    console.log(
      'Committed deletion/block promotion/branch/root exits, delayed restack, undo, conflict safety, cleanup and reduced motion passed'
    );
  } finally {
    await b.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
