const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!base.endsWith(':8823')) throw Error('Use isolated frontend');
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await browser.newPage();
    const release = {
      available: true, version: 'test', bytes: 1024, commit: 'abc1234',
      platforms: { 'macos-universal': { filename: 'Schema-s-macos-universal.zip', notarized: false } }
    };
    await page.route('**/downloads/latest.json', route => route.fulfill({ json: release }));
    await page.goto(base + '/welcome');
    const link = page.locator('#macos-universal-download');
    await page.waitForFunction(() => document.getElementById('windows-download').hasAttribute('href'));
    assert.equal(await link.getAttribute('href'), null);
    Object.assign(release.platforms['macos-universal'],{filename:'Schema-s-macos-universal-unsigned.zip',distribution:'unsigned-preview',signing:'ad-hoc'});
    await page.reload();
    await page.waitForFunction(() => document.getElementById('macos-universal-download').hasAttribute('href'));
    assert.equal(await link.getAttribute('href'), '/downloads/Schema-s-macos-universal-unsigned.zip');
    assert.match(await page.locator('#macos-release-status').textContent(),/unsigned preview.*not notarized/i);
    release.platforms['macos-universal'].filename = 'Schema-s-macos-universal.zip';
    release.platforms['macos-universal'].notarized = true;
    await page.reload();
    await page.waitForFunction(() => document.getElementById('macos-universal-download').hasAttribute('href'));
    assert.equal(await link.getAttribute('href'), '/downloads/Schema-s-macos-universal.zip');
    assert.equal(await link.getAttribute('aria-disabled'), null);
    release.platforms['macos-universal'].filename = 'https://other.invalid/app.zip';
    await page.reload();
    await page.waitForFunction(() => document.getElementById('windows-download').hasAttribute('href'));
    assert.equal(await link.getAttribute('href'), null);
    console.log('Mac download gating: pending, unsigned preview, notarized, and invalid filename passed');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
