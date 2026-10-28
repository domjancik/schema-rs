import importlib.util,json,pathlib,tempfile,unittest,hashlib,sys
from unittest.mock import patch
from types import SimpleNamespace
ROOT=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('installer',ROOT/'scripts/publication/install_linux.py');m=importlib.util.module_from_spec(spec);sys.modules[spec.name]=m;spec.loader.exec_module(m)
class Installer(unittest.TestCase):
 def test_preserves_existing_managed_data_and_credentials(self):
  with tempfile.TemporaryDirectory() as t:
   root=pathlib.Path(t);package=root/'package';package.mkdir();binary=package/'schema-rs';binary.write_bytes(b'test')
   (package/'BUILD.json').write_text(json.dumps({'commit':'abc1234','platform':'linux-x64','sha256':hashlib.sha256(b'test').hexdigest()}))
   (root/'etc/schema').mkdir(parents=True);env=root/'etc/schema/schema.env';env.write_text('SCHEMA_TOKEN=existing-token-do-not-change\n');env.chmod(0o600)
   (root/'var/lib/schema/.session').mkdir(parents=True);session=root/'var/lib/schema/.session/session.json';session.write_text('history')
   m.install(package,root,8787,'0.0.0.0',staging=True)
   self.assertEqual(env.read_text(),'SCHEMA_TOKEN=existing-token-do-not-change\n');self.assertEqual(session.read_text(),'history')
   self.assertNotIn('rtc', (root/'etc/systemd/system/schema.service').read_text().lower())
   self.assertTrue((root/'opt/schema/current').is_symlink())
 def test_refuses_unmanaged_service_before_writing(self):
  with tempfile.TemporaryDirectory() as t:
   root=pathlib.Path(t);service=root/'etc/systemd/system/schema.service';service.parent.mkdir(parents=True);service.write_text('existing Orange Pi service')
   with self.assertRaises(ValueError):m.install(root/'package',root,8787,'0.0.0.0',staging=True)
   self.assertFalse((root/'opt/schema').exists())
 def test_checksum_failure_does_not_install(self):
  with tempfile.TemporaryDirectory() as t:
   root=pathlib.Path(t);package=root/'package';package.mkdir();(package/'schema-rs').write_bytes(b'bad');(package/'BUILD.json').write_text(json.dumps({'commit':'abc1234','sha256':'0'*64}))
   with self.assertRaises(ValueError):m.install(package,root,8787,'0.0.0.0',staging=True)
   self.assertFalse((root/'opt/schema').exists())
 def test_failed_health_check_restores_previous_service_and_binary(self):
  with tempfile.TemporaryDirectory() as t:
   root=pathlib.Path(t);package=root/'package';package.mkdir();(package/'schema-rs').write_bytes(b'new')
   (package/'BUILD.json').write_text(json.dumps({'commit':'abc1234','platform':'linux-x64','sha256':hashlib.sha256(b'new').hexdigest()}))
   unit=root/'etc/systemd/system/schema.service';unit.parent.mkdir(parents=True);unit.write_text(m.MARKER+'\nold unit')
   launcher=root/'usr/local/lib/schema/start';launcher.parent.mkdir(parents=True);launcher.write_text('old launcher')
   env=root/'etc/schema/schema.env';env.parent.mkdir(parents=True);env.write_text('SCHEMA_TOKEN=existing-token-do-not-change\n')
   old=root/'opt/schema/releases/old';old.mkdir(parents=True);(old/'schema-rs').write_bytes(b'old');current=root/'opt/schema/current';current.symlink_to('releases/old')
   with patch.object(m.os,'geteuid',return_value=0),patch.object(m.platform,'machine',return_value='x86_64'),patch.object(m.shutil,'which',return_value='/usr/bin/systemctl'),patch.object(m.subprocess,'run',return_value=SimpleNamespace(returncode=0)),patch.object(m.urllib.request,'urlopen',side_effect=OSError('offline')),patch.object(m.time,'sleep'):
    with self.assertRaises(RuntimeError):m.install(package,root,8787,'0.0.0.0')
   self.assertEqual(current.readlink(),pathlib.Path('releases/old'));self.assertEqual(unit.read_text(),m.MARKER+'\nold unit');self.assertEqual(launcher.read_text(),'old launcher')
   self.assertEqual(env.read_text(),'SCHEMA_TOKEN=existing-token-do-not-change\n')
if __name__=='__main__':unittest.main()
