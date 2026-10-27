import importlib.util,pathlib,tempfile,unittest,zipfile,json,hashlib
class PackageTest(unittest.TestCase):
 def test_release_archive_is_allowlisted_and_hash_matches(self):
  spec=importlib.util.spec_from_file_location('build_desktop',pathlib.Path('scripts/build_desktop.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  with tempfile.TemporaryDirectory() as work:
   root=pathlib.Path(work);binary=root/'schema-rs.exe';binary.write_bytes(b'MZ-test-only');(root/'secret.env').write_text('not for distribution');notices=root/'notices';notices.mkdir();(notices/'MIT.txt').write_text('test license')
   archive=module.package(binary,root/'out','0.1.0-preview','abc123',notices)
   release=json.loads((root/'out/latest.json').read_text());self.assertEqual(release['sha256'],hashlib.sha256(archive.read_bytes()).hexdigest());self.assertEqual(release['bytes'],archive.stat().st_size)
   with zipfile.ZipFile(archive) as zipped:
    names=zipped.namelist();self.assertIn('Schema-s/Start Schema-s.cmd',names);self.assertIn('Schema-s/schema-rs.exe',names);self.assertFalse(any('secret' in n or '.session' in n for n in names));self.assertIn('desktop',zipped.read('Schema-s/Start Schema-s.cmd').decode())
if __name__=='__main__':unittest.main()
