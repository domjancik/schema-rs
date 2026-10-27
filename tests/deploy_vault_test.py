import importlib.util,pathlib,unittest,uuid
ROOT=pathlib.Path(__file__).resolve().parents[1]
class VaultTests(unittest.TestCase):
 def test_native_credential_roundtrip_and_removal(self):
  path=ROOT/'scripts/deploy/windows_vault.py';self.assertTrue(path.exists(),'Native Windows credential helper exists')
  spec=importlib.util.spec_from_file_location('vault',path);vault=importlib.util.module_from_spec(spec);spec.loader.exec_module(vault)
  target='schema-rs/test/'+uuid.uuid4().hex
  try:
   self.assertIsNone(vault.read(target));vault.write(target,'vault-check-ångström');self.assertEqual(vault.read(target),'vault-check-ångström')
  finally:vault.delete(target)
  self.assertIsNone(vault.read(target))
if __name__=='__main__':unittest.main()
