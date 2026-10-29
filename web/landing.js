'use strict';
(async () => {
  const button = document.getElementById('windows-download'),
    detail = document.getElementById('release-detail');
  try {
    const response = await fetch('/downloads/latest.json', { cache: 'no-store' });
    if (!response.ok) throw Error('unavailable');
    const release = await response.json();
    if (!release.available) throw Error('unavailable');
    button.href = '/downloads/Schema-s-Windows-x64.zip';
    button.removeAttribute('aria-disabled');
    button.textContent = 'Download for Windows ↓';
    detail.textContent = `${release.version} · ${(release.bytes / 1048576).toFixed(1)} MB Windows · ${release.commit.slice(0, 12)}`;
    for (const [platform, label] of [
      ['linux-x64', 'Linux x64 ↓'],
      ['linux-arm64', 'Linux ARM64 / Raspberry Pi ↓'],
      ['macos-universal', 'macOS · Apple Silicon + Intel ↓']
    ]) {
      const file = release.platforms?.[platform],
        link = document.getElementById(platform + '-download');
      const expected = `Schema-s-${platform}.${platform === 'macos-universal' ? 'zip' : 'tar.gz'}`;
      if (file?.filename === expected && (platform !== 'macos-universal' || file.notarized === true)) {
        link.href = '/downloads/' + file.filename;
        link.textContent = label;
        link.removeAttribute('aria-disabled');
        if (platform === 'macos-universal') document.getElementById('macos-release-status').textContent = 'Universal macOS download · Apple Silicon + Intel · signed and notarized.';
      }
    }
    if (release.platforms) {
      const installer = document.getElementById('linux-installer');
      installer.href = '/downloads/install.sh';
      installer.textContent = 'Download boot-service installer ↧';
    }
    document.getElementById('download-checksum').hidden = false;
  } catch {
    button.textContent = 'Windows download not installed on this host';
    detail.textContent =
      'Open Studio here, or use a Schéma host that has the desktop package available.';
  }
})();

(async()=>{try{const response=await fetch('/browser-build.json',{cache:'no-store'});if(!response.ok)return;const build=await response.json();if(build.version!==1||!build.files?.['wasm/schema_web_bg.wasm'])return;const link=document.getElementById('browser-editor');if(link){link.href='/play?view=studio';link.removeAttribute('aria-disabled');}}catch{}})();
