"""Build an allowlisted macOS bundle; signing/notarization runs on a Mac only."""
import argparse, hashlib, json, pathlib, plistlib, shutil, struct, subprocess, sys, tomllib
ROOT=pathlib.Path(__file__).resolve().parents[1]
CPUS={'macos-arm64':{0x100000c},'macos-x64':{0x1000007},'macos-universal':{0x100000c,0x1000007}}
def validate_macho(path,platform,kind=2):
 data=pathlib.Path(path).read_bytes()
 if platform not in CPUS:raise ValueError('Unknown macOS architecture')
 def thin(offset,size):
  if size<32 or offset<0 or offset+size>len(data) or data[offset:offset+4]!=b'\xcf\xfa\xed\xfe':raise ValueError('Expected64-bit Mach-O')
  cpu,_,filetype=struct.unpack_from('<III',data,offset+4)
  if filetype!=kind:raise ValueError('Unexpected Mach-O file type')
  return cpu
 if data[:4]==b'\xca\xfe\xba\xbe':
  if len(data)<8:raise ValueError('Truncated universal header')
  count=struct.unpack_from('>I',data,4)[0]
  if not 1<=count<=8 or len(data)<8+20*count:raise ValueError('Invalid universal header')
  cpus=set()
  for i in range(count):
   cpu,_,offset,size,_=struct.unpack_from('>IIIII',data,8+i*20)
   if thin(offset,size)!=cpu or cpu in cpus:raise ValueError('Invalid universal slice')
   cpus.add(cpu)
 else:cpus={thin(0,len(data))}
 if cpus!=CPUS[platform]:raise ValueError('Wrong macOS architecture')
 return cpus

def bundle(binary,launcher,output,commit,notices,platform,ftdi=None,ftdi_license=None,sparkle=None,sparkle_license=None,update_key=None,update_feed=None,build_number=1):
 import re
 if not re.fullmatch('[0-9a-f]{7,40}',commit):raise ValueError('Source commit required')
 if not isinstance(build_number,int) or build_number<1:raise ValueError('Positive build number required')
 if any([sparkle,update_key,update_feed,sparkle_license]):
  import base64,urllib.parse
  if not all([sparkle,update_key,update_feed,sparkle_license]):raise ValueError('Sparkle framework/license, update public key and HTTPS feed are all required')
  try:key=base64.b64decode(update_key,validate=True)
  except Exception as error:raise ValueError('Invalid Sparkle public key') from error
  url=urllib.parse.urlparse(update_feed)
  if len(key)!=32 or url.scheme!='https' or not url.hostname or url.username or url.password:raise ValueError('Invalid signed update configuration')
  if pathlib.Path(sparkle).name!='Sparkle.framework' or not pathlib.Path(sparkle).is_dir() or not pathlib.Path(sparkle_license).is_file():raise ValueError('Sparkle SDK/framework license unavailable')
 validate_macho(binary,platform);validate_macho(launcher,platform)
 notices=pathlib.Path(notices)
 if not notices.is_dir() or not any(notices.iterdir()):raise ValueError('Dependency notices required')
 app=pathlib.Path(output)/'Schema-s.app'
 if app.exists():raise ValueError('Refusing to overwrite an existing app bundle')
 if ftdi:
  validate_macho(ftdi,platform,6)
  if not ftdi_license or not pathlib.Path(ftdi_license).is_file():raise ValueError('FTDI redistribution license required')
 macos=app/'Contents/MacOS';resources=app/'Contents/Resources';macos.mkdir(parents=True);resources.mkdir()
 for source,name in [(binary,'schema-rs'),(launcher,'Schema-s')]:shutil.copyfile(source,macos/name);(macos/name).chmod(0o755)
 info={'CFBundleIdentifier':'tools.scenic.schema-s','CFBundleName':'Schema-s','CFBundleDisplayName':'Schema-s','CFBundleExecutable':'Schema-s','CFBundlePackageType':'APPL','CFBundleShortVersionString':tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf8'))['package']['version'],'CFBundleVersion':str(build_number),'LSMinimumSystemVersion':'13.0','LSUIElement':True,'NSLocalNetworkUsageDescription':'Schema-s sends Art-Net lighting data to devices on your local network and hosts controls for your phone.','NSHighResolutionCapable':True}
 if sparkle:info.update({'SUFeedURL':update_feed,'SUPublicEDKey':update_key,'SUAllowsAutomaticUpdates':True})
 (app/'Contents/Info.plist').write_bytes(plistlib.dumps(info));(resources/'BUILD.json').write_text(json.dumps({'commit':commit,'platform':platform},indent=2)+'\n',encoding='utf8')
 shutil.copytree(notices,resources/'THIRD_PARTY_NOTICES');shutil.copyfile(ROOT/'LICENSE',resources/'LICENSE.txt');shutil.copyfile(ROOT/'web/scene-assets/carpushit-provenance.json',resources/'SCENE_ASSET_PROVENANCE.json');shutil.copyfile(ROOT/'docs/macos.md',resources/'README.md')
 for source,name in [(ROOT/'web/vendor/THREE-LICENSE.txt','THREE-LICENSE.txt'),(ROOT/'web/vendor/scene-three.mjs.LEGAL.txt','THREE-BUNDLE-NOTICES.txt')]:shutil.copyfile(source,resources/'THIRD_PARTY_NOTICES'/name)
 if sparkle:
  frameworks=app/'Contents/Frameworks';frameworks.mkdir(exist_ok=True);shutil.copytree(sparkle,frameworks/'Sparkle.framework',symlinks=True);shutil.copyfile(sparkle_license,resources/'THIRD_PARTY_NOTICES/SPARKLE-LICENSE.txt')
 if ftdi:
  framework=app/'Contents/Frameworks';framework.mkdir(exist_ok=True);shutil.copyfile(ftdi,framework/'libftd2xx.dylib');shutil.copyfile(ftdi_license,resources/'THIRD_PARTY_NOTICES/FTDI-LICENSE.txt')
 return app

def sign_and_archive(app,output,platform,identity=None,profile=None):
 if sys.platform!='darwin':raise ValueError('Signing and distributable archives require macOS')
 if not profile:raise ValueError('Notary Keychain profile required for distribution')
 if not identity:raise ValueError('Developer ID identity required for distribution; do not ship unsigned bundles')
 app=pathlib.Path(app);output=pathlib.Path(output);output.mkdir(parents=True,exist_ok=True)
 def is_macho(file):
  if not file.is_file() or file.is_symlink():return False
  with file.open('rb') as content:return content.read(4) in [b'\xcf\xfa\xed\xfe',b'\xca\xfe\xba\xbe',b'\xbe\xba\xfe\xca']
 candidates=[file for file in app.rglob('*') if is_macho(file)]
 nested=[folder for folder in app.rglob('*') if folder.is_dir() and not folder.is_symlink() and folder.suffix in ['.xpc','.app','.framework','.bundle']]
 for file in candidates+sorted(nested,key=lambda p:len(p.parts),reverse=True)+[app]:subprocess.run(['codesign','--force','--options','runtime','--timestamp','--sign',identity,str(file)],check=True)
 subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
 archive=output/('Schema-s-'+platform+'.zip')
 def compress():
  if archive.exists():archive.unlink()
  subprocess.run(['ditto','-c','-k','--keepParent',str(app),str(archive)],check=True)
 compress()
 subprocess.run(['xcrun','notarytool','submit',str(archive),'--keychain-profile',profile,'--wait'],check=True)
 subprocess.run(['xcrun','stapler','staple',str(app)],check=True);subprocess.run(['xcrun','stapler','validate',str(app)],check=True);subprocess.run(['spctl','--assess','--type','execute','--verbose',str(app)],check=True);compress()
 digest=hashlib.sha256(archive.read_bytes()).hexdigest();metadata=json.loads((app/'Contents/Resources/BUILD.json').read_text());metadata.update({'notarized':True,'sha256':digest,'filename':archive.name,'bytes':archive.stat().st_size});(output/(archive.name+'.release.json')).write_text(json.dumps(metadata,indent=2)+'\n',encoding='utf8');(output/(archive.name+'.sha256')).write_text(digest+'  '+archive.name+'\n',encoding='utf8');return archive

def main():
 p=argparse.ArgumentParser();p.add_argument('--binary',required=True,type=pathlib.Path);p.add_argument('--launcher',required=True,type=pathlib.Path);p.add_argument('--notices',required=True,type=pathlib.Path);p.add_argument('--platform',choices=CPUS,default='macos-universal');p.add_argument('--commit',required=True);p.add_argument('--output',type=pathlib.Path,default=ROOT/'target/macos');p.add_argument('--ftdi',type=pathlib.Path);p.add_argument('--ftdi-license',type=pathlib.Path);p.add_argument('--identity');p.add_argument('--notary-profile');p.add_argument('--sparkle',type=pathlib.Path);p.add_argument('--sparkle-license',type=pathlib.Path);p.add_argument('--update-key');p.add_argument('--update-feed');p.add_argument('--build-number',type=int,default=1);p.add_argument('--bundle-only',action='store_true');a=p.parse_args()
 app=bundle(a.binary,a.launcher,a.output,a.commit,a.notices,a.platform,a.ftdi,a.ftdi_license,a.sparkle,a.sparkle_license,a.update_key,a.update_feed,a.build_number)
 if a.bundle_only:print('Development bundle only (not a signed release): '+str(app));return
 print(sign_and_archive(app,a.output,a.platform,a.identity,a.notary_profile))
if __name__=='__main__':main()
