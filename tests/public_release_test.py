"""Public release boundaries: only verified binaries and explicit public inputs."""
import importlib.util,json,pathlib,tarfile,tempfile,unittest,sys
ROOT=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('public_release',ROOT/'scripts/build_public_release.py')
release=importlib.util.module_from_spec(spec);sys.modules[spec.name]=release;spec.loader.exec_module(release)
class PublicRelease(unittest.TestCase):
 def test_wrong_architecture_is_rejected_before_packaging(self):
  with tempfile.TemporaryDirectory() as t:
   p=pathlib.Path(t)/'binary';p.write_bytes(b'MZ'+bytes(100))
   with self.assertRaises(ValueError):release.validate_elf(p,'linux-arm64')
 def test_linux_archive_and_manifest_are_complete_and_public_only(self):
  with tempfile.TemporaryDirectory() as t:
   root=pathlib.Path(t);binary=root/'schema-rs';data=bytearray(64);data[:6]=b'\x7fELF\x02\x01';data[18:20]=(183).to_bytes(2,'little');binary.write_bytes(data)
   notices=root/'notices';notices.mkdir();(notices/'LICENSE').write_text('test notice')
   archive=release.package_linux(binary,root/'out','linux-arm64','abc1234',notices)
   with tarfile.open(archive) as tar:
    names=tar.getnames();self.assertIn('Schema-s/schema-rs',names);self.assertIn('Schema-s/install.sh',names)
    self.assertFalse(any('.session' in n or 'access.json' in n or 'schema-project' in n for n in names))
    self.assertEqual(tar.getmember('Schema-s/schema-rs').mode,0o755)
 def test_site_export_does_not_publish_a_nonexistent_engine(self):
  with tempfile.TemporaryDirectory() as t:
   root=pathlib.Path(t);downloads=root/'downloads';downloads.mkdir();(downloads/'latest.json').write_text('{}');(downloads/'private.json').write_text('must never publish');(downloads/'Schema-s-macos-universal.zip').write_bytes(b'unsigned stale build')
   release.export_site(root/'site',downloads)
   html=(root/'site/index.html').read_text(encoding='utf8')
   self.assertIn('/connect?view=studio',html);self.assertNotIn('href="/?view=studio"',html)
   self.assertFalse((root/'site/downloads/Schema-s-macos-universal.zip').exists());self.assertFalse((root/'site/downloads/private.json').exists());self.assertFalse((root/'site/app.js').exists());self.assertFalse((root/'site/schema-project').exists())
 def test_mac_public_import_requires_notarized_matching_commit_and_hash(self):
  import zipfile,hashlib,plistlib
  with tempfile.TemporaryDirectory() as t:
   root=pathlib.Path(t);archive=root/'Schema-s-macos-universal.zip'
   with zipfile.ZipFile(archive,'w') as z:
    z.writestr('Schema-s.app/Contents/Resources/BUILD.json',json.dumps({'commit':'abc1234','platform':'macos-universal'}))
    z.writestr('Schema-s.app/Contents/Info.plist',plistlib.dumps({'CFBundleIdentifier':'tools.scenic.schema-s'}))
   with self.assertRaises(ValueError):release.verify_macos_release(archive,'abc1234')
   proof={'commit':'abc1234','platform':'macos-universal','notarized':True,'sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),'bytes':archive.stat().st_size,'filename':archive.name}
   sidecar=root/(archive.name+'.release.json');sidecar.write_text(json.dumps(proof))
   self.assertEqual(release.verify_macos_release(archive,'abc1234')['filename'],archive.name)
   with self.assertRaises(ValueError):release.verify_macos_release(archive,'def5678')
   proof['notarized']=False;sidecar.write_text(json.dumps(proof))
   with self.assertRaises(ValueError):release.verify_macos_release(archive,'abc1234')
   proof['notarized']=True;proof['sha256']='0'*64;sidecar.write_text(json.dumps(proof))
   with self.assertRaises(ValueError):release.verify_macos_release(archive,'abc1234')
 def test_browser_publication_verifies_artifacts_before_copying_and_preserves_landing(self):
  import hashlib
  with tempfile.TemporaryDirectory() as t:
   p=pathlib.Path(t);browser=p/'browser';browser.mkdir();site=p/'site';site.mkdir();(site/'index.html').write_text('landing')
   (browser/'studio.html').write_text('studio');(browser/'index.html').write_text('editor')
   files={name:hashlib.sha256((browser/name).read_bytes()).hexdigest() for name in ['studio.html','index.html']}
   (browser/'browser-build.json').write_text(json.dumps({'version':1,'files':files}))
   release.install_browser(site,browser);self.assertEqual((site/'index.html').read_text(),'landing');self.assertEqual((site/'studio.html').read_text(),'studio');self.assertNotIn('index.html',json.loads((site/'browser-build.json').read_text())['files'])
   (browser/'studio.html').write_text('tampered')
   with self.assertRaises(ValueError):release.install_browser(site,browser)
if __name__=='__main__':unittest.main()
