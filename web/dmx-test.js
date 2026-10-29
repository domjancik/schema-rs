/* Raw channel testing uses short runtime leases, never project edits. */
(function (g) {
  'use strict';
  function mount(host, { data }) {
    const el = (tag, text) => {
        const n = document.createElement(tag);
        if (text !== undefined) n.textContent = text;
        return n;
      },
      owner = Array.from(crypto.getRandomValues(new Uint8Array(16)),b=>b.toString(16).padStart(2,'0')).join(''),
      levels = new Map();
    let universe = 0,
      start = 1,
      master = 1,
      armed = false,
      disposed = false,
      timer = null,
      sending = false,
      pending = null;
    const box = el('section');
    box.className = 'dmx-test-panel';
    const heading = el('h3', 'Channel test'),
      status = el('p'),
      form = el('div');
    form.className = 'dmx-test-controls';
    const field = (title, input) => {
      const label = el('label', title);
      input.setAttribute('aria-label', title);
      label.append(input);
      form.append(label);
      return input;
    };
    const number = (value, min, max) => {
      const n = el('input');
      n.type = 'number';
      n.min = min;
      n.max = max;
      n.value = value;
      return n;
    };
    const u = field('Test universe', number(0, 0, 32767)),
      first = field('First test channel', number(1, 1, 497)),
      m = el('input');
    m.type = 'range';
    m.min = 0;
    m.max = 100;
    m.value = 100;
    field('Test master', m);
    const actions = el('div');
    actions.className = 'dmx-test-actions';
    const arm = el('button', 'Arm channel test'),
      zero = el('button', 'Zero tested channels'),
      release = el('button', 'Release test');
    for (const b of [arm, zero, release]) {
      b.type = 'button';
      actions.append(b);
    }
    const grid = el('div');
    grid.className = 'dmx-test-grid';
    box.append(
      heading,
      el(
        'p',
        'Only touched channels are overridden. Release restores scene levels. Closing or disconnecting releases the test within 5 seconds.'
      ),
      form,
      actions,
      status,
      grid
    );
    host.append(box);
    function event(channels) {
      return { type: 'channel_test', owner, universe, channels };
    }
    function transmit(channels) {
      pending=event(channels);
      if(sending)return;
      sending=true;
      (async()=>{
        while(pending){const payload=pending;pending=null;
          try{await g.SchemaEngine.request('/api/inputs',payload);}
          catch(error){armed=false;if(pending&&Object.keys(pending.channels).length)pending=null;
            if(!disposed){status.dataset.error='true';status.textContent=error.message;update();}
          }
        }
        sending=false;
      })();
    }
    function send() {
      clearTimeout(timer);
      if (armed)
        transmit(Object.fromEntries([...levels].map(([c, v]) => [c, Math.round(v * master)])));
    }
    function stop() {
      status.dataset.error = '';
      const had = armed || levels.size;
      armed = false;
      levels.clear();
      clearTimeout(timer);
      if (had) transmit({});
      draw();
      update();
    }
    function label(channel) {
      const p = data()?.project;
      if (!p) return '';
      const f = p.fixtures.find((f) => {
        const width = g.SchemaOutputCore?.footprint(p, f) || 1;
        return f.universe === universe && channel >= f.address && channel < f.address + width;
      });
      return f ? f.name + ' · slot ' + (channel - f.address + 1) : 'Unpatched';
    }
    function draw() {
      grid.replaceChildren();
      for (let channel = start; channel < start + 16 && channel <= 512; channel++) {
        const row = el('div');
        row.className = 'dmx-test-channel';
        row.dataset.channel = channel;
        const title = el('strong', 'CH ' + channel),
          patch = el('small', label(channel)),
          slider = el('input'),
          value = el('output'),
          free = el('button', 'Release');
        slider.type = 'range';
        slider.min = 0;
        slider.max = 255;
        slider.step = 1;
        slider.setAttribute('aria-label', 'Test channel ' + channel);
        free.type = 'button';
        free.setAttribute('aria-label', 'Release channel ' + channel);
        slider.oninput = () => {
          if (!armed) return;
          if (!levels.has(channel) && levels.size >= 32) {
            status.textContent = 'At most 32 test channels. Release a channel first.';
            return;
          }
          levels.set(channel, Number(slider.value));
          update();
          clearTimeout(timer);
          timer = setTimeout(send, 40);
        };
        slider.onchange = () => {
          if (armed) send();
        };
        free.onclick = () => {
          levels.delete(channel);
          send();
          update();
        };
        row.append(title, patch, slider, value, free);
        grid.append(row);
      }
    }
    function update() {
      if (disposed) return;
      const frame = data(),
        capture = !frame?.project.output.enabled;
      arm.textContent = armed ? 'Test armed' : 'Arm channel test';
      arm.disabled = armed;
      zero.disabled = !armed || !levels.size;
      release.disabled = !armed && !levels.size;
      m.disabled = !armed;
      if (!status.textContent || !status.dataset.error)
        status.textContent =
          (!armed && frame?.snapshot.channel_test
            ? 'Channel test active · U' + frame.snapshot.channel_test.universe + ' · '
            : '') +
          (capture ? 'Capture only · ' : 'Live output · ') +
          (armed
            ? levels.size + ' test channels · ' + Math.round(master * 100) + '% master'
            : 'Test released') +
          (frame?.snapshot.blackout ? ' · BLACKOUT' : '');
      for (const row of grid.children) {
        const channel = Number(row.dataset.channel),
          slider = row.querySelector('input'),
          value = row.querySelector('output'),
          own = levels.has(channel),
          raw = levels.get(channel) ?? frame?.snapshot.universes?.[universe]?.[channel - 1] ?? 0;
        if (document.activeElement !== slider) slider.value = raw;
        slider.disabled = !armed;
        value.textContent = Math.round((raw / 255) * 100) + '% · ' + raw;
        row.classList.toggle('overridden', own);
        row.querySelector('button').disabled = !own;
      }
    }
    arm.onclick = () => {
      armed = true;
      status.dataset.error = '';
      update();
    };
    release.onclick = stop;
    zero.onclick = () => {
      for (const key of levels.keys()) levels.set(key, 0);
      send();
      update();
    };
    m.oninput = () => {
      master = Number(m.value) / 100;
      clearTimeout(timer);
      timer = setTimeout(send, 40);
      update();
    };
    m.onchange = send;
    u.onchange = () => {
      const next = Number(u.value);
      if (!Number.isInteger(next) || next < 0 || next > 32767) {
        u.value = universe;
        return;
      }
      stop();
      universe = next;
      draw();
      update();
    };
    first.onchange = () => {
      start = Math.max(1, Math.min(497, Math.round(Number(first.value) || 1)));
      first.value = start;
      draw();
      update();
    };
    function hidden() {
      if (document.hidden) stop();
    }
    function unload() {
      if (armed || levels.size) {
        if(window.SchemaEngine?.kind==='browser'){window.SchemaEngine.request('/api/inputs',event({})).catch(()=>{});return;}
        const token = sessionStorage.getItem('schema-token');
        fetch('/api/inputs', {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            ...(token ? { 'x-schema-token': token } : {})
          },
          body: JSON.stringify(event({})),
          keepalive: true
        }).catch(() => {});
      }
    }
    const pulse = setInterval(() => {
      if (!box.isConnected) {
        destroy();
        return;
      }
      if (armed && levels.size) send();
      update();
    }, 1000);
    document.addEventListener('visibilitychange', hidden);
    addEventListener('pagehide', unload);
    function destroy() {
      if (disposed) return;
      stop();
      disposed = true;
      clearInterval(pulse);
      clearTimeout(timer);
      document.removeEventListener('visibilitychange', hidden);
      removeEventListener('pagehide', unload);
    }
    draw();
    update();
    return { update, destroy };
  }
  g.SchemaDmxTest = { mount };
})(globalThis);
