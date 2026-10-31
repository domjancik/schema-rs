import hashlib,importlib.util,pathlib,tempfile,unittest,zipfile
spec=importlib.util.spec_from_file_location('cross_toolchain',pathlib.Path(__file__).resolve().parents[1]/'scripts/cross_toolchain.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ArchiveTests(unittest.TestCase):
 def test_mismatched_and_escaping_archives_never_extract(self):
  with tempfile.TemporaryDirectory() as folder:
   root=pathlib.Path(folder);archive=root/'tool.zip'
   with zipfile.ZipFile(archive,'w') as z:z.writestr('../outside.txt','bad')
   with self.assertRaisesRegex(ValueError,'checksum'):m.extract_verified(archive,root/'out','0'*64)
   with self.assertRaisesRegex(ValueError,'Unsafe'):m.extract_verified(archive,root/'out',hashlib.sha256(archive.read_bytes()).hexdigest())
   self.assertFalse((root/'outside.txt').exists())
 def test_verified_contained_archive_extracts(self):
  with tempfile.TemporaryDirectory() as folder:
   root=pathlib.Path(folder);archive=root/'tool.zip'
   with zipfile.ZipFile(archive,'w') as z:z.writestr('compiler/readme.txt','verified')
   m.extract_verified(archive,root/'out',hashlib.sha256(archive.read_bytes()).hexdigest());self.assertEqual((root/'out/compiler/readme.txt').read_text(),'verified')
if __name__=='__main__':unittest.main()
