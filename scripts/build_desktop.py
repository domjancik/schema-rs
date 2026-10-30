"""Build the Windows portable engine + browser UI; package only explicit public inputs."""
import argparse,hashlib,json,pathlib,subprocess,sys,tomllib,zipfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
ARCHIVE='Schema-s-Windows-x64.zip'
LAUNCHER='@echo off\r\ntitle Schema-s Engine\r\n"%~dp0schema-rs.exe" desktop %*\r\nif errorlevel 1 pause\r\n'

def package(binary,output,version,commit,notices):
    binary,output,notices=map(pathlib.Path,(binary,output,notices))
    if not binary.is_file() or binary.read_bytes()[:2]!=b'MZ':raise ValueError('A Windows PE executable is required')
    if not notices.is_dir() or not any(notices.iterdir()):raise ValueError('Dependency notices are required')
    output.mkdir(parents=True,exist_ok=True)
    archive=output/ARCHIVE;temporary=output/(ARCHIVE+'.tmp')
    with zipfile.ZipFile(temporary,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=6) as zipped:
        zipped.write(binary,'Schema-s/schema-rs.exe')
        zipped.writestr('Schema-s/Start Schema-s.cmd',LAUNCHER)
        zipped.write(ROOT/'docs/desktop.md','Schema-s/README.txt')
        zipped.write(ROOT/'LICENSE','Schema-s/LICENSE.txt')
        zipped.write(ROOT/'web/vendor/THREE-LICENSE.txt','Schema-s/THIRD_PARTY_NOTICES/three/LICENSE.txt')
        zipped.write(ROOT/'web/vendor/QR-LICENSES.txt','Schema-s/THIRD_PARTY_NOTICES/QR-LICENSES.txt')
        zipped.write(ROOT/'web/vendor/scene-three.mjs.LEGAL.txt','Schema-s/THIRD_PARTY_NOTICES/three/BUNDLE_NOTICES.txt')
        zipped.write(ROOT/'web/scene-assets/carpushit-provenance.json','Schema-s/SCENE_ASSET_PROVENANCE.json')
        for notice in sorted(notices.rglob('*')):
            if notice.is_file():zipped.write(notice,'Schema-s/THIRD_PARTY_NOTICES/'+notice.relative_to(notices).as_posix())
    temporary.replace(archive)
    digest=hashlib.sha256(archive.read_bytes()).hexdigest()
    (output/'SHA256SUMS.txt').write_text(digest+'  '+ARCHIVE+'\n',encoding='utf8')
    metadata={'available':True,'version':version,'commit':commit,'sha256':digest,'bytes':archive.stat().st_size,'filename':ARCHIVE}
    pointer=output/'latest.json.tmp';pointer.write_text(json.dumps(metadata,indent=2)+'\n',encoding='utf8');pointer.replace(output/'latest.json')
    return archive

def collect_notices(destination):
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--format-version','1'],cwd=ROOT,text=True))
    destination.mkdir(parents=True,exist_ok=True);index=[]
    for crate in metadata['packages']:
        if crate['name']=='schema-rs':continue
        label=crate['name']+'-'+crate['version'];folder=pathlib.Path(crate['manifest_path']).parent;files=[]
        for item in folder.iterdir():
            if item.is_file() and any(item.name.lower().startswith(prefix) for prefix in ('license','copying','notice','copyright')):files.append(item)
        if crate.get('license_file'):
            explicit=folder/crate['license_file']
            if explicit.is_file() and explicit not in files:files.append(explicit)
        target=destination/label;target.mkdir(exist_ok=True)
        for item in files:(target/item.name).write_bytes(item.read_bytes())
        (target/'PACKAGE.txt').write_text(label+'\nLicense: '+str(crate.get('license'))+'\nRepository: '+str(crate.get('repository'))+'\n',encoding='utf8')
        index.append(label+' '+str(crate.get('license')))
    (destination/'DEPENDENCIES.txt').write_text('\n'.join(sorted(index))+'\n',encoding='utf8')

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--skip-build',action='store_true');parser.add_argument('--binary',type=pathlib.Path,default=ROOT/'target/release/schema-rs.exe');parser.add_argument('--output',type=pathlib.Path,default=ROOT/'target/distribution/current');args=parser.parse_args()
    if sys.platform!='win32':raise SystemExit('Build the Windows x64 distribution on Windows')
    if not args.skip_build:subprocess.run(['cargo','build','--locked','--release'],cwd=ROOT,check=True)
    notices=ROOT/'target/distribution/notices';collect_notices(notices)
    version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']+'-preview'
    commit=subprocess.check_output(['git','rev-parse','--short=12','HEAD'],cwd=ROOT,text=True).strip()
    print(package(args.binary,args.output,version,commit,notices))
if __name__=='__main__':main()
