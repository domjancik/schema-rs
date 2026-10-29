"""Build a small, explicit public site and verified native release archives."""
import argparse, hashlib, json, pathlib, shutil, struct, tarfile, tomllib
ROOT = pathlib.Path(__file__).resolve().parents[1]
PLATFORMS = {'linux-x64': ('x86_64', 62), 'linux-arm64': ('ARM64', 183)}

def digest(path): return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()

def validate_elf(binary, platform):
    header = pathlib.Path(binary).read_bytes()[:64]
    if platform not in PLATFORMS or len(header) < 64 or header[:6] != b'\x7fELF\x02\x01' or struct.unpack('<H', header[18:20])[0] != PLATFORMS[platform][1]:
        raise ValueError('Expected a Linux 64-bit little-endian ELF for ' + platform)

def package_linux(binary, output, platform, commit, notices):
    binary, output, notices = map(pathlib.Path, (binary, output, notices))
    validate_elf(binary, platform)
    if not notices.is_dir() or not any(notices.iterdir()): raise ValueError('Dependency notices required')
    output.mkdir(parents=True, exist_ok=True)
    archive = output / ('Schema-s-' + platform + '.tar.gz')
    with tarfile.open(archive.with_suffix('.tmp'), 'w:gz') as tar:
        def add(source, name, executable=False):
            info = tar.gettarinfo(str(source), 'Schema-s/' + name)
            info.uid = info.gid = 0; info.uname = info.gname = ''; info.mode = 0o755 if executable else 0o644
            with open(source, 'rb') as content: tar.addfile(info, content)
        add(binary, 'schema-rs', True)
        add(ROOT/'scripts/publication/install.sh', 'install.sh', True)
        add(ROOT/'scripts/publication/install_linux.py', 'install_linux.py')
        add(ROOT/'docs/linux.md', 'README.md')
        add(ROOT/'LICENSE', 'LICENSE.txt')
        add(ROOT/'web/vendor/THREE-LICENSE.txt', 'THIRD_PARTY_NOTICES/three/LICENSE.txt')
        add(ROOT/'web/vendor/scene-three.mjs.LEGAL.txt', 'THIRD_PARTY_NOTICES/three/BUNDLE_NOTICES.txt')
        add(ROOT/'web/scene-assets/carpushit-provenance.json', 'SCENE_ASSET_PROVENANCE.json')
        for path in sorted(notices.rglob('*')):
            if path.is_file(): add(path, 'THIRD_PARTY_NOTICES/' + path.relative_to(notices).as_posix())
        metadata = json.dumps({'commit':commit,'platform':platform,'sha256':digest(binary)},indent=2).encode()
        import io
        info=tarfile.TarInfo('Schema-s/BUILD.json');info.size=len(metadata);info.mode=0o644;tar.addfile(info,io.BytesIO(metadata))
    archive.with_suffix('.tmp').replace(archive)
    return archive

def verify_macos_release(archive, commit):
    """Import only artifacts attested by the native signing/notarization job.

    The job and its artifact transport are trusted; this sidecar is not a substitute
    for codesign/notarytool. Do not accept arbitrary uploads as release artifacts.
    """
    import zipfile, plistlib
    archive=pathlib.Path(archive)
    if archive.name != 'Schema-s-macos-universal.zip': raise ValueError('Expected universal Mac release')
    try:
        proof=json.loads(archive.with_name(archive.name+'.release.json').read_text(encoding='utf8'))
        if (proof.get('notarized') is not True or proof.get('commit') != commit or
            proof.get('platform') != 'macos-universal' or proof.get('filename') != archive.name or
            proof.get('sha256') != digest(archive) or proof.get('bytes') != archive.stat().st_size):
            raise ValueError('Mac release attestation mismatch')
        with zipfile.ZipFile(archive) as z:
            metadata=json.loads(z.read('Schema-s.app/Contents/Resources/BUILD.json'))
            info=plistlib.loads(z.read('Schema-s.app/Contents/Info.plist'))
            if metadata.get('commit') != commit or metadata.get('platform') != 'macos-universal' or info.get('CFBundleIdentifier') != 'tools.scenic.schema-s':
                raise ValueError('Mac bundle metadata mismatch')
    except (OSError, KeyError, json.JSONDecodeError, zipfile.BadZipFile, plistlib.InvalidFileException) as error:
        raise ValueError('Native notarized Mac release and attestation required') from error
    return {key:proof[key] for key in ['filename','sha256','bytes','notarized']}

def export_site(destination, downloads):
    destination, downloads = map(pathlib.Path, (destination, downloads))
    destination.mkdir(parents=True, exist_ok=True)
    html=(ROOT/'web/welcome.html').read_text(encoding='utf8')
    html=html.replace('href="/?view=studio"','href="/connect?view=studio"').replace('href="/mobile"','href="/connect?view=mobile"')
    html=html.replace('<script defer src="/install-ui.js"></script>','')
    html=html.replace('<link rel="manifest" href="/app.webmanifest">','')
    html=html.replace('<button class="button" data-install>Install on your phone</button>','<a class="button" href="/connect?view=mobile">Install on your phone</a>')
    html=html.replace('/downloads/SHA256SUMS.txt','/downloads/SHA256SUMS.txt')
    (destination/'index.html').write_text(html,encoding='utf8')
    for name in ['landing.css','landing.js','favicon.ico','app-icon-192.png','app-icon-512.png']:
        shutil.copyfile(ROOT/'web'/name,destination/name)
    for name in ['connect.html','connect.js']:
        shutil.copyfile(ROOT/'scripts/publication'/name,destination/name)
    target=destination/'downloads';target.mkdir(exist_ok=True)
    manifest=json.loads((downloads/'latest.json').read_text(encoding='utf8')) if (downloads/'latest.json').is_file() else {}
    mac=manifest.get('platforms',{}).get('macos-universal',{})
    for path in downloads.iterdir():
        if path.name == 'Schema-s-macos-universal.zip' and (mac.get('notarized') is not True or mac.get('sha256') != digest(path)): continue
        if path.is_file() and (path.name in {'latest.json','SHA256SUMS.txt','install.sh','Schema-s-Windows-x64.zip','Schema-s-linux-x64.tar.gz','Schema-s-linux-arm64.tar.gz','Schema-s-macos-universal.zip'}): shutil.copyfile(path,target/path.name)
    shutil.copyfile(ROOT/'scripts/publication/vercel.json',destination/'vercel.json')
    shutil.copyfile(ROOT/'docs/linux.md',destination/'linux.txt')
    shutil.copyfile(ROOT/'docs/macos.md',destination/'macos.txt')

def main():
    p=argparse.ArgumentParser();p.add_argument('--linux-x64',type=pathlib.Path,required=True);p.add_argument('--linux-arm64',type=pathlib.Path,required=True);p.add_argument('--macos',type=pathlib.Path);p.add_argument('--windows',type=pathlib.Path,required=True);p.add_argument('--notices',type=pathlib.Path,required=True);p.add_argument('--commit',required=True);p.add_argument('--output',type=pathlib.Path,default=ROOT/'target/publication/downloads');p.add_argument('--site',type=pathlib.Path,default=ROOT/'target/publication/site');a=p.parse_args()
    if not __import__('re').fullmatch('[0-9a-f]{7,40}',a.commit): raise ValueError('Source commit required')
    a.output.mkdir(parents=True,exist_ok=True)
    version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']+'-preview'
    artifacts={}
    for platform,binary in [('linux-x64',a.linux_x64),('linux-arm64',a.linux_arm64)]:
        file=package_linux(binary,a.output,platform,a.commit,a.notices)
        artifacts[platform]={'filename':file.name,'sha256':digest(file),'bytes':file.stat().st_size}
    with __import__('zipfile').ZipFile(a.windows) as z:
        if 'Schema-s/schema-rs.exe' not in z.namelist(): raise ValueError('Invalid Windows package')
    windows=a.output/'Schema-s-Windows-x64.zip';shutil.copyfile(a.windows,windows)
    artifacts['windows-x64']={'filename':windows.name,'sha256':digest(windows),'bytes':windows.stat().st_size}
    if a.macos:
        artifacts['macos-universal']=verify_macos_release(a.macos,a.commit)
        shutil.copyfile(a.macos,a.output/a.macos.name)
    shutil.copyfile(ROOT/'scripts/publication/bootstrap.sh',a.output/'install.sh')
    manifest={'available':True,'version':version,'commit':a.commit,**artifacts['windows-x64'],'platforms':artifacts}
    (a.output/'latest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf8')
    (a.output/'SHA256SUMS.txt').write_text(''.join(f"{v['sha256']}  {v['filename']}\n" for v in artifacts.values())+digest(a.output/'install.sh')+'  install.sh\n',encoding='utf8')
    export_site(a.site,a.output)
    print(json.dumps(manifest,indent=2))
if __name__=='__main__':main()
