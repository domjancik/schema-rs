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
   root=pathlib.Path(t);downloads=root/'downloads';downloads.mkdir();(downloads/'latest.json').write_text('{}');(downloads/'private.json').write_text('must never publish')
   release.export_site(root/'site',downloads)
   html=(root/'site/index.html').read_text(encoding='utf8')
   self.assertIn('/connect?view=studio',html);self.assertNotIn('href="/?view=studio"',html)
   self.assertFalse((root/'site/downloads/private.json').exists());self.assertFalse((root/'site/app.js').exists());self.assertFalse((root/'site/schema-project').exists())
if __name__=='__main__':unittest.main()
