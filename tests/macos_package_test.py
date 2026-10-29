"""macOS bundle construction is inspectable/testable without executing Mac binaries."""
import importlib.util,pathlib,plistlib,tempfile,unittest,struct,sys
ROOT=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('macpack',ROOT/'scripts/build_macos.py');mac=importlib.util.module_from_spec(spec);sys.modules[spec.name]=mac;spec.loader.exec_module(mac)
class MacBundle(unittest.TestCase):
 def test_bundle_uses_public_inputs_local_network_description_and_native_launcher(self):
  with tempfile.TemporaryDirectory() as t:
   p=pathlib.Path(t);binary=p/'engine';binary.write_bytes(struct.pack('<IIIIIIII',0xfeedfacf,0x100000c,0,2,0,0,0,0));launcher=p/'launcher';launcher.write_bytes(binary.read_bytes());notices=p/'notices';notices.mkdir();(notices/'LICENSE.txt').write_text('test notice');app=mac.bundle(binary,launcher,p/'out','abc1234',notices,'macos-arm64');info=plistlib.loads((app/'Contents/Info.plist').read_bytes());self.assertEqual(info['CFBundleIdentifier'],'tools.scenic.schema-s');self.assertIn('Art-Net',info['NSLocalNetworkUsageDescription']);self.assertTrue((app/'Contents/MacOS/Schema-s').is_file());self.assertTrue((app/'Contents/MacOS/schema-rs').is_file());self.assertFalse(any('schema-project' in str(x) or '.session' in str(x) for x in app.rglob('*')))
 def test_complete_signed_update_configuration_and_overwrite_guard(self):
  import base64
  with tempfile.TemporaryDirectory() as t:
   p=pathlib.Path(t);binary=p/'engine';binary.write_bytes(struct.pack('<IIIIIIII',0xfeedfacf,0x100000c,0,2,0,0,0,0));notices=p/'notices';notices.mkdir();(notices/'LICENSE').write_text('notice');sdk=p/'Sparkle.framework';sdk.mkdir();license=p/'LICENSE';license.write_text('Sparkle license');key=base64.b64encode(bytes(32)).decode()
   with self.assertRaises(ValueError):mac.bundle(binary,binary,p/'bad','abc1234',notices,'macos-arm64',update_key=key)
   with self.assertRaises(ValueError):mac.bundle(binary,binary,p/'bad','abc1234',notices,'macos-arm64',sparkle=sdk,sparkle_license=license,update_key=key,update_feed='http://example.com/appcast.xml')
   app=mac.bundle(binary,binary,p/'out','abc1234',notices,'macos-arm64',sparkle=sdk,sparkle_license=license,update_key=key,update_feed='https://example.com/appcast.xml',build_number=42)
   info=plistlib.loads((app/'Contents/Info.plist').read_bytes());self.assertEqual(info['CFBundleVersion'],'42');self.assertEqual(info['SUPublicEDKey'],key);self.assertNotIn('SUEnableAutomaticChecks',info)
   self.assertTrue((app/'Contents/Frameworks/Sparkle.framework').is_dir())
   with self.assertRaises(ValueError):mac.bundle(binary,binary,p/'out','abc1234',notices,'macos-arm64')
 def test_universal_slices_and_optional_ftdi_licensing(self):
  with tempfile.TemporaryDirectory() as t:
   p=pathlib.Path(t);arm=struct.pack('<IIIIIIII',0xfeedfacf,0x100000c,0,2,0,0,0,0);intel=struct.pack('<IIIIIIII',0xfeedfacf,0x1000007,0,2,0,0,0,0)
   fat=struct.pack('>II',0xcafebabe,2)+struct.pack('>IIIII',0x100000c,0,48,32,0)+struct.pack('>IIIII',0x1000007,0,80,32,0)+arm+intel
   binary=p/'engine';binary.write_bytes(fat);self.assertEqual(mac.validate_macho(binary,'macos-universal'),{0x100000c,0x1000007})
   with self.assertRaises(ValueError):mac.validate_macho(binary,'macos-arm64')
   notices=p/'notices';notices.mkdir();(notices/'LICENSE').write_text('notice');dylib=p/'ftdi';dylib.write_bytes(struct.pack('<IIIIIIII',0xfeedfacf,0x100000c,0,6,0,0,0,0));thin=p/'thin';thin.write_bytes(arm)
   with self.assertRaises(ValueError):mac.bundle(thin,thin,p/'out','abc1234',notices,'macos-arm64',ftdi=dylib)
 def test_wrong_architecture_is_rejected(self):
  with tempfile.TemporaryDirectory() as t:
   p=pathlib.Path(t)/'binary';p.write_bytes(b'MZ'+bytes(100));
   with self.assertRaises(ValueError):mac.validate_macho(p,'macos-arm64')
if __name__=='__main__':unittest.main()
