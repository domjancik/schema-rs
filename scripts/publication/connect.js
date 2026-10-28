'use strict';
const address = document.getElementById('engine-address'),
  form = document.getElementById('connect-form'),
  error = document.getElementById('connect-error');
const mobile = new URLSearchParams(location.search).get('view') === 'mobile';
form.querySelector('button').textContent = mobile ? 'Open schémaboy ↗' : 'Open Studio ↗';
try {
  if (!address.value) address.value = localStorage.getItem('schema-public-engine') || '';
} catch {}
form.addEventListener('submit', (event) => {
  event.preventDefault();
  try {
    const text = address.value.trim(),
      url = new URL(
        /^[a-z][a-z0-9+.-]*:/i.test(text) && !/^[-\w.]+:\d/.test(text) ? text : 'http://' + text
      );
    if (
      !['http:', 'https:'].includes(url.protocol) ||
      url.username ||
      url.password ||
      url.search ||
      url.hash ||
      !url.hostname ||
      url.pathname !== '/'
    )
      throw Error(
        'Enter only an HTTP(S) engine address and port, without passwords, paths or query parameters.'
      );
    if (url.origin === location.origin)
      throw Error('Enter the address of your running engine, not this download site.');
    try {
      localStorage.setItem('schema-public-engine', url.origin);
    } catch {}
    url.pathname = mobile ? '/mobile' : '/';
    if (!mobile) url.search = 'view=studio';
    location.assign(url.href);
  } catch (e) {
    error.textContent =
      e.message === 'Invalid URL'
        ? 'Enter a valid engine address, such as http://192.168.1.20:8787.'
        : e.message;
  }
});

address.disabled = false;
form.querySelector('button').disabled = false;
