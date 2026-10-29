"""Build the browser host from public web inputs; never includes a user project."""
import argparse
import hashlib
import json
import io
import zipfile
import os
from pathlib import Path
import shutil
import subprocess

ROOT=Path(__file__).resolve().parents[1]
PUBLIC_EXTENSIONS={'.html','.js','.mjs','.css','.json','.svg','.png','.ico','.glb','.webmanifest','.txt'}

def export_browser(destination, wasm, notices=None, source_root=ROOT):
    destination,wasm,source_root=Path(destination),Path(wasm),Path(source_root)
    required=['schema_web.js','schema_web_bg.wasm']
    if not all((wasm/name).is_file() for name in required):
        raise ValueError('Compile the real WASM browser bindings before exporting')
    if notices is None or not has_notices(notices):raise ValueError('Dependency notices required for WASM distribution')
    destination.mkdir(parents=True,exist_ok=True)
    files=set()
    for source in (source_root/'web').rglob('*'):
        relative=source.relative_to(source_root/'web')
        if any(part.startswith('.') for part in relative.parts) or source.is_symlink() or 'wasm' in relative.parts:continue
        if source.is_file() and source.suffix in PUBLIC_EXTENSIONS:
            target=destination/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target);files.add(relative.as_posix())
    for name in ['index.html','mobile.html','preview.html']:
        source=source_root/'web'/name
        html=source.read_text(encoding='utf8').replace('<html','<html data-engine="browser"',1)
        if name=='index.html':
            html=html.replace('href="/?view=studio"','href="/play?view=studio"')
            (destination/'studio.html').write_text(html,encoding='utf8');files.add('studio.html')
        (destination/name).write_text(html,encoding='utf8');files.add(name)
    target=destination/'wasm';target.mkdir(exist_ok=True)
    for name in required:shutil.copyfile(wasm/name,target/name);files.add('wasm/'+name)
    shutil.copyfile(source_root/'LICENSE',destination/'ENGINE-LICENSE.txt');files.add('ENGINE-LICENSE.txt')
    shutil.copyfile(source_root/'docs/browser.md',destination/'browser-guide.txt')
    files.add('browser-guide.txt')
    for source in Path(notices).rglob('*'):
        if source.is_file():
            relative=Path('THIRD_PARTY_NOTICES')/source.relative_to(notices);target=destination/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target);files.add(relative.as_posix())
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    metadata={'version':1,'source_commit':commit,'files':{name:hashlib.sha256((destination/name).read_bytes()).hexdigest() for name in sorted(files)}}
    (destination/'browser-build.json').write_text(json.dumps(metadata,indent=2)+'\n',encoding='utf8')
    return destination

def has_notices(path):
    path=Path(path)
    return path.is_dir() and any(path.iterdir())

def committed_sources(ref):
    commit=subprocess.check_output(['git','rev-parse',ref],cwd=ROOT,text=True).strip()
    target=ROOT/'target/browser-sources'/commit;target.mkdir(parents=True,exist_ok=True)
    archive=subprocess.check_output(['git','archive','--format=zip',commit,'web','docs/browser.md','LICENSE'],cwd=ROOT)
    with zipfile.ZipFile(io.BytesIO(archive)) as source:
        for entry in source.infolist():
            path=target/entry.filename
            if not path.resolve().is_relative_to(target.resolve()):raise ValueError('Invalid source archive path')
            if entry.is_dir():path.mkdir(parents=True,exist_ok=True)
            else:path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(source.read(entry))
    return target

def main():
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,default=ROOT/'target/browser-site');p.add_argument('--bindgen',default='wasm-bindgen');p.add_argument('--skip-build',action='store_true');p.add_argument('--source-ref');a=p.parse_args()
    cache=Path(os.environ.get('CARGO_TARGET_DIR',ROOT/'target')).resolve()
    binary=cache/'wasm32-unknown-unknown/release/schema_web.wasm'
    if not a.skip_build:subprocess.run(['cargo','build','--locked','--release','-p','schema-web','--target','wasm32-unknown-unknown'],cwd=ROOT,check=True)
    bindings=ROOT/'web/wasm';bindings.mkdir(exist_ok=True)
    subprocess.run([a.bindgen,'--target','web','--out-dir',str(bindings),str(binary)],cwd=ROOT,check=True)
    subprocess.run(['cargo','fetch','--locked'],cwd=ROOT,check=True)
    from build_desktop import collect_notices
    notices=ROOT/'target/browser-notices';collect_notices(notices)
    print(export_browser(a.output,bindings,notices,committed_sources(a.source_ref) if a.source_ref else ROOT))

if __name__=='__main__':main()
