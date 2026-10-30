const assert = require('node:assert/strict'),
  { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.PUBLIC_SITE_URL || 'http://127.0.0.1:8828';
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } }),
      errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(base);
    for (const id of ['windows-download', 'linux-x64-download', 'linux-arm64-download']) {
      await page.waitForFunction((id) => document.getElementById(id)?.hasAttribute('href'), id);
      assert.equal(await page.locator('#' + id).getAttribute('aria-disabled'), null);
    }
    assert.equal(await page.locator('#macos-universal-download').count(), 1, 'Mac release status must be visible');
    const manifest = await page.request.get(base + '/downloads/latest.json').then(r => r.json());
    const mac = manifest.platforms?.['macos-universal'];
    if (mac?.notarized === true) {
      assert.equal(await page.locator('#macos-universal-download').getAttribute('href'), '/downloads/Schema-s-macos-universal.zip');
    } else if (mac?.distribution === 'unsigned-preview' && mac?.signing === 'ad-hoc') {
      assert.equal(await page.locator('#macos-universal-download').getAttribute('href'), '/downloads/Schema-s-macos-universal-unsigned.zip');
      assert.match(await page.locator('#macos-release-status').textContent(), /unsigned.*not notarized/i);
    } else {
      assert.equal(await page.locator('#macos-universal-download').getAttribute('href'), null);
      assert.match(await page.locator('#macos-release-status').textContent(), /pending.*native.*validation/i);
    }
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.getByRole('link', { name: 'Open Studio ↗', exact: true }).first().click();
    await page.getByRole('heading',{name:'Servers',exact:true}).waitFor();
    const address=page.getByLabel('Server address',{exact:true}),save=page.getByRole('button',{name:'Save server',exact:true});
    await address.fill('javascript:alert(1)');await save.click();
    assert.equal(await page.locator('.saved-server').count(),0);
    await address.fill('http://schema.local:8787/?token=secret');await save.click();
    assert.match(await page.locator('.server-status').textContent(),/without/);
    await page.context().route('http://schema.local:8787/**',r=>r.fulfill({contentType:'text/html',body:'<h1>Host version UI</h1>'}));
    await address.fill('schema.local:8787');await save.click();
    const row=page.locator('.saved-server');
    assert.equal(await row.getByRole('link',{name:'Open Studio',exact:true}).getAttribute('href'),'http://schema.local:8787/?view=studio');
    const [host]=await Promise.all([page.waitForEvent('popup'),row.getByRole('link',{name:'Open Studio',exact:true}).click()]);
    await host.getByRole('heading',{name:'Host version UI'}).waitFor();await host.close();
    assert.deepEqual(errors, []);
    console.log(
      'Public mobile site, real download availability, secure device-address validation and saved server and direct host UI opening passed.'
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
