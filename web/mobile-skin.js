/* Local appearance only: no frame, camera data or sensor reading leaves this module. */
(function () {
  'use strict';
  const M = SchemaSkinCore,
    shell = document.querySelector('.pocket');
  const motion = matchMedia('(prefers-reduced-motion: reduce)');
  let scene = M.sceneLight(),
    environment = null,
    direction = { x: 0, y: 0 };
  let tiltEnabled = false,
    stream = null,
    timer = null,
    request = 0,
    pending = false;
  let tiltRequest = 0;
  let lastPaint = 0,
    lastScene = 0,
    tiltMessage = 'Off',
    cameraMessage = 'Off';
  const video = document.createElement('video');
  video.muted = true;
  video.playsInline = true;
  const canvas = document.createElement('canvas');
  canvas.width = 16;
  canvas.height = 12;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  const faces = document.createElement('div');
  faces.className = 'skin-facets';
  faces.setAttribute('aria-hidden', 'true');
  for (const side of ['top', 'left', 'right', 'bottom']) {
    const face = document.createElement('i');
    face.className = 'skin-facet-' + side;
    faces.append(face);
  }
  shell.prepend(faces);
  shell.classList.add('faceted-skin');
  const dialog = document.createElement('dialog');
  dialog.id = 'skin-dialog';
  dialog.setAttribute('aria-labelledby', 'skin-title');
  dialog.innerHTML =
    '<div class="dialog-head"><h2 id="skin-title">Shell lighting</h2><button type="button" aria-label="Close appearance">×</button></div><p>Faceted shell · live scene light. Optional phone sensors add a local impression of the room.</p><div class="skin-settings"><button type="button" id="skin-tilt">Enable phone tilt</button><output id="skin-tilt-status"></output><button type="button" id="skin-camera">Enable camera lighting</button><output id="skin-camera-status"></output></div><p class="skin-privacy">Camera: 16 × 12 local samples, twice a second. No recording or upload. Stops when you leave this page. Room lighting is approximate, not a physical light measurement.</p>';
  document.body.append(dialog);
  const tiltButton = dialog.querySelector('#skin-tilt'),
    cameraButton = dialog.querySelector('#skin-camera');
  dialog.querySelector('[aria-label="Close appearance"]').onclick = () => dialog.close();
  document.getElementById('skin-options').onclick = () => {
    document.querySelector('.app-menu').open = false;
    controls();
    dialog.showModal();
  };
  function controls() {
    tiltButton.textContent = tiltEnabled ? 'Disable phone tilt' : 'Enable phone tilt';
    tiltButton.setAttribute('aria-pressed', String(tiltEnabled));
    cameraButton.textContent =
      stream || pending ? 'Disable camera lighting' : 'Enable camera lighting';
    cameraButton.setAttribute('aria-pressed', String(!!stream));
    dialog.querySelector('#skin-tilt-status').textContent = tiltMessage;
    dialog.querySelector('#skin-camera-status').textContent = cameraMessage;
  }
  function paint() {
    if (document.hidden) return;
    const target = environment || scene;
    const color = target.rgb.map((v, i) =>
      Math.round(v * (environment ? 0.8 : 1) + (environment ? scene.rgb[i] * 0.2 : 0))
    );
    const light = M.facets({
      x: (environment?.x || -0.25) - direction.x,
      y: (environment?.y || -0.4) - direction.y
    });
    shell.style.setProperty('--skin-rgb', color.join(' '));
    shell.style.setProperty('--skin-energy', String(0.04 + Math.min(1, target.energy) * 0.12));
    for (const [side, value] of Object.entries(light))
      shell.style.setProperty('--skin-' + side, String(value));
  }
  function orientation(event) {
    if (document.hidden || motion.matches || performance.now() - lastPaint < 80) return;
    lastPaint = performance.now();
    const target = M.tilt(event, screen.orientation?.angle || 0);
    direction = {
      x: direction.x * 0.75 + target.x * 0.25,
      y: direction.y * 0.75 + target.y * 0.25
    };
    tiltMessage = 'On · relative phone tilt';
    controls();
    paint();
  }
  function stopTilt(message = 'Off') {
    tiltRequest++;
    tiltEnabled = false;
    removeEventListener('deviceorientation', orientation);
    direction = { x: 0, y: 0 };
    tiltMessage = message;
    controls();
    paint();
  }
  tiltButton.onclick = async () => {
    if (tiltEnabled) return stopTilt();
    if (!isSecureContext) {
      tiltMessage = 'Needs HTTPS or localhost';
      return controls();
    }
    if (motion.matches) {
      tiltMessage = 'Reduced motion is enabled';
      return controls();
    }
    if (!window.DeviceOrientationEvent) {
      tiltMessage = 'Tilt sensor unavailable';
      return controls();
    }
    // Permission call stays in the click handler for Safari transient activation.
    try {
      const generation = ++tiltRequest;
      const permission =
        typeof DeviceOrientationEvent.requestPermission === 'function'
          ? await DeviceOrientationEvent.requestPermission()
          : 'granted';
      if (generation !== tiltRequest || document.hidden || motion.matches) return;
      if (permission !== 'granted') {
        tiltMessage = 'Permission not granted';
        return controls();
      }
      tiltEnabled = true;
      tiltMessage = 'Enabled · waiting for sensor';
      addEventListener('deviceorientation', orientation);
      controls();
    } catch {
      tiltMessage = 'Tilt permission unavailable';
      controls();
    }
  };
  function stopCamera(message = 'Off') {
    request++;
    pending = false;
    clearInterval(timer);
    timer = null;
    stream?.getTracks().forEach((track) => track.stop());
    stream = null;
    video.pause();
    video.srcObject = null;
    context?.clearRect(0, 0, 16, 12);
    environment = null;
    cameraMessage = message;
    controls();
    paint();
  }
  function sample() {
    if (document.hidden || !stream || video.readyState < 2 || !context) return;
    try {
      context.drawImage(video, 0, 0, 16, 12);
      environment = M.cameraLight(context.getImageData(0, 0, 16, 12).data, 16, 12);
      paint();
    } catch {
      stopCamera('Camera sampling unavailable');
    }
  }
  cameraButton.onclick = async () => {
    if (stream || pending) return stopCamera();
    if (!isSecureContext) {
      cameraMessage = 'Needs HTTPS or localhost';
      return controls();
    }
    if (!navigator.mediaDevices?.getUserMedia || !context) {
      cameraMessage = 'Camera unavailable';
      return controls();
    }
    pending = true;
    cameraMessage = 'Awaiting camera permission';
    controls();
    const generation = ++request;
    let acquired;
    try {
      acquired = await navigator.mediaDevices.getUserMedia({
        audio: false,
        video: {
          facingMode: { ideal: 'environment' },
          width: { ideal: 160 },
          height: { ideal: 120 },
          frameRate: { ideal: 2, max: 5 }
        }
      });
      if (generation !== request || document.hidden) {
        acquired.getTracks().forEach((track) => track.stop());
        return;
      }
      stream = acquired;
      video.srcObject = stream;
      await video.play();
      if (generation !== request || document.hidden) return;
      pending = false;
      cameraMessage = 'On · local room tint / direction';
      controls();
      sample();
      timer = setInterval(sample, 500);
      stream.getTracks().forEach((track) =>
        track.addEventListener?.(
          'ended',
          () => {
            if (stream === acquired) stopCamera('Camera ended');
          },
          { once: true }
        )
      );
    } catch {
      if (generation === request) stopCamera('Camera permission denied or unavailable');
    }
  };
  function suspend() {
    stopCamera('Off · enable again to resume');
    stopTilt('Off · enable again to resume');
  }
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) suspend();
    else paint();
  });
  addEventListener('pagehide', suspend);
  motion.addEventListener('change', () => {
    if (motion.matches) stopTilt('Reduced motion is enabled');
  });
  window.SchemaSkin = {
    update(snapshot) {
      if (document.hidden || performance.now() - lastScene < 200) return;
      lastScene = performance.now();
      scene = M.sceneLight(snapshot);
      paint();
    }
  };
  controls();
  paint();
})();
