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
      window.cameraSide = 1;
      Object.defineProperty(HTMLMediaElement.prototype, 'readyState', { get: () => 2 });
      const draw = CanvasRenderingContext2D.prototype.drawImage;
      CanvasRenderingContext2D.prototype.drawImage = function(source, ...args) {
        if (!(source instanceof HTMLVideoElement)) return draw.call(this, source, ...args);
        const image = this.createImageData(16,12);
        for (let y=0;y<12;y++) for (let x=0;x<16;x++) {
          const i=(y*16+x)*4, bright=window.cameraSide > 0 ? x>=8 : x<8;
          image.data[i]=image.data[i+1]=image.data[i+2]=bright?255:0;
          image.data[i+3]=255;
        }
        this.putImageData(image,0,0);
      };
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
    assert.equal(await page.locator('.skin-facets i').count(), 10, 'Broad facets, not edge strips');
    await page.locator('#skin-options').waitFor({ state: 'attached', timeout: 5000 });
    assert.deepEqual(await page.evaluate(() => sensorTest), { camera: 0, tilt: 0, stops: 0 });
    await page.screenshot({path:'target/skin-surface-neutral.png'});
    const controlColor = await page.locator('.encoder').evaluate(el => getComputedStyle(el).backgroundColor);
    await page.getByLabel('More options', { exact: true }).click();
    await page.locator('#skin-options').click();
    await page.getByRole('button', { name: 'Enable phone tilt', exact: true }).click();
    await page.evaluate(() => dispatchEvent(new DeviceOrientationEvent('deviceorientation', { beta: 75, gamma: 0 })));
    await page.waitForTimeout(90);
    const beforeTilt = await page
      .locator('.skin-facets i').first()
      .evaluate((el) => el.style.getPropertyValue('--facet-light'));
    await page.evaluate(() =>
      dispatchEvent(new DeviceOrientationEvent('deviceorientation', { beta: 85, gamma: 30 }))
    );
    assert.notEqual(
      await page.locator('.skin-facets i').first().evaluate((el) => el.style.getPropertyValue('--facet-light')),
      beforeTilt
    );
    for (let i=0;i<10;i++) {
      await page.waitForTimeout(85);
      await page.evaluate(() => dispatchEvent(new DeviceOrientationEvent('deviceorientation', {beta:85,gamma:35})));
    }
    await page.getByRole('button', { name: 'Close appearance', exact: true }).click();
    await page.screenshot({path:'target/skin-surface-tilted.png'});
    await page.getByLabel('More options', {exact:true}).click();
    await page.locator('#skin-options').click();
    const beforeCamera = await page.locator('.skin-facets').evaluate(el => [...el.children].map(face => face.style.backgroundColor));
    await page.getByRole('button', { name: 'Enable camera lighting', exact: true }).click();
    const afterCamera = await page.locator('.skin-facets').evaluate(el => [...el.children].map(face => face.style.backgroundColor));
    assert.notDeepEqual(afterCamera,beforeCamera,'Camera samples must change visible face paint');
    await page.evaluate(() => { window.cameraSide=-1; });
    await page.waitForTimeout(600);
    assert.notDeepEqual(await page.locator('.skin-facets').evaluate(el => [...el.children].map(face => face.style.backgroundColor)),afterCamera,'Light direction must change facet shading');
    assert.equal(await page.locator('.encoder').evaluate(el => getComputedStyle(el).backgroundColor),controlColor,'Shading must not alter control contrast');
    assert.equal(await page.locator('.screen-bezel').evaluate(el => getComputedStyle(el).clipPath),'none','No cut-out bevels on the screen');
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
