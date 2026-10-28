const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const base = process.env.SCHEMA_URL || 'http://127.0.0.1:8823';
if (!/^http:\/\/127\.0\.0\.1:8823$/.test(base)) throw Error('Use isolated frontend 8823');
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  try {
    const page = await browser.newPage({
      viewport: { width: 390, height: 844 },
      isMobile: true,
      hasTouch: true
    });
    const frame = await fetch(base + '/api/state').then((r) => r.json());
    let commands = 0;
    await page.route('**/api/state', (r) => r.fulfill({ json: frame }));
    await page.route('**/api/command', (r) => {
      commands++;
      return r.abort();
    });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.addInitScript(() => {
      window.sensorTest = { camera: 0, tilt: 0, stops: 0 };
      window.DeviceOrientationEvent.requestPermission = async () => {
        sensorTest.tilt++;
        return 'granted';
      };
      navigator.mediaDevices.getUserMedia = async () => {
        sensorTest.camera++;
        return { getTracks: () => [{ stop: () => sensorTest.stops++ }] };
      };
      HTMLMediaElement.prototype.play = async () => {};
      Object.defineProperty(HTMLMediaElement.prototype, 'srcObject', {
        set(v) {
          this._stream = v;
        },
        get() {
          return this._stream;
        }
      });
    });
    await page.goto(base + '/mobile');
    await page.locator('#skin-options').waitFor({ state: 'attached', timeout: 5000 });
    assert.deepEqual(await page.evaluate(() => sensorTest), { camera: 0, tilt: 0, stops: 0 });
    await page.getByLabel('More options', { exact: true }).click();
    await page.locator('#skin-options').click();
    await page.getByRole('button', { name: 'Enable phone tilt', exact: true }).click();
    const beforeTilt = await page
      .locator('.pocket')
      .evaluate((el) => el.style.getPropertyValue('--skin-left'));
    await page.evaluate(() =>
      dispatchEvent(new DeviceOrientationEvent('deviceorientation', { beta: 20, gamma: 30 }))
    );
    assert.notEqual(
      await page.locator('.pocket').evaluate((el) => el.style.getPropertyValue('--skin-left')),
      beforeTilt
    );
    await page.getByRole('button', { name: 'Enable camera lighting', exact: true }).click();
    assert.deepEqual(await page.evaluate(() => sensorTest), { camera: 1, tilt: 1, stops: 0 });
    await page.getByRole('button', { name: 'Disable camera lighting', exact: true }).click();
    assert.equal(await page.evaluate(() => sensorTest.stops), 1);
    await page.getByRole('button', { name: 'Enable camera lighting', exact: true }).click();
    await page.evaluate(() => {
      Object.defineProperty(document, 'hidden', { configurable: true, value: true });
      document.dispatchEvent(new Event('visibilitychange'));
    });
    assert.equal(await page.evaluate(() => sensorTest.stops), 2);
    await page.evaluate(() => {
      Object.defineProperty(document, 'hidden', { configurable: true, value: false });
      document.dispatchEvent(new Event('visibilitychange'));
    });
    assert.equal(await page.evaluate(() => sensorTest.camera), 2, 'No automatic camera restart');
    await page.getByRole('button', { name: 'Close appearance', exact: true }).click();
    for (const size of [
      { width: 320, height: 740 },
      { width: 390, height: 844 },
      { width: 844, height: 390 }
    ]) {
      await page.setViewportSize(size);
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      const b = await page.locator('.pocket').boundingBox();
      assert(b.y + b.height <= size.height + 1);
    }
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByLabel('More options', { exact: true }).click();
    await page.locator('#skin-options').click();
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.getByRole('button', { name: 'Enable phone tilt', exact: true }).click();
    assert.equal(
      await page.evaluate(() => sensorTest.tilt),
      1,
      'Reduced motion avoids sensor permission'
    );
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await page.evaluate(() => {
      navigator.mediaDevices.getUserMedia = () =>
        new Promise((resolve) => (window.resolveCamera = resolve));
    });
    await page.getByRole('button', { name: 'Enable camera lighting', exact: true }).click();
    await page.getByRole('button', { name: 'Disable camera lighting', exact: true }).click();
    await page.evaluate(() =>
      resolveCamera({ getTracks: () => [{ stop: () => sensorTest.stops++ }] })
    );
    await page.waitForFunction(() => sensorTest.stops === 3);
    await page.evaluate(() => {
      navigator.mediaDevices.getUserMedia = async () => {
        throw new DOMException('Denied', 'NotAllowedError');
      };
    });
    await page.getByRole('button', { name: 'Enable camera lighting', exact: true }).click();
    await page.getByText('Camera permission denied or unavailable', { exact: true }).waitFor();
    await page.evaluate(() =>
      Object.defineProperty(window, 'isSecureContext', { value: false, configurable: true })
    );
    await page.getByRole('button', { name: 'Enable camera lighting', exact: true }).click();
    await page.getByText('Needs HTTPS or localhost', { exact: true }).waitFor();
    assert.equal(commands, 0);
    assert.deepEqual(errors, []);
    console.log('mobile skin permission, cleanup and viewport tests passed');
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
