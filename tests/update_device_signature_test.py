"""Linux signed-ingestion tests; temporary artifacts only, no service activation."""
import json,os,pathlib,subprocess,sys,tempfile,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'scripts/update_server'))
import apply as installer
from protocol import FILES,CHECKS,canonical,digest
class SignedInput(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.root=pathlib.Path(self.temp.name);home=self.root/'stage';base=self.root/'opt';(base/'releases').mkdir(parents=True);home.mkdir();self.patches=[]
  for name,value in {'HOME':home,'BASE':base,'PUBLIC':self.root/'public.pem','TRUST':self.root/'trust.json'}.items():p=patch.object(installer,name,value);p.start();self.patches.append(p)
  key=self.root/'key.pem';subprocess.run(['openssl','genpkey','-algorithm','ED25519','-out',str(key)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);subprocess.run(['openssl','pkey','-in',str(key),'-pubout','-out',str(installer.PUBLIC)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  scratch=self.root/'scratch';scratch.mkdir();elf=bytearray(128);elf[:6]=b'\x7fELF\x02\x01';elf[18:20]=(183).to_bytes(2,'little');(scratch/'schema-rs').write_bytes(elf);(scratch/FILES[1]).write_bytes(b'PK-test');(scratch/'latest.json').write_bytes(canonical({'commit':'a'*12,'sha256':digest(scratch/FILES[1]),'bytes':7}));(scratch/'SHA256SUMS.txt').write_bytes(b'test')
  self.release='a'*12+'-'+digest(scratch/'schema-rs')[:12]+'-1';self.stage=home/self.release;scratch.rename(self.stage)
  manifest={'schema':1,'target':'aarch64-unknown-linux-musl','release_id':self.release,'sequence':1,'source_commit':'a'*40,'tree':'b'*64,'checks':dict.fromkeys(CHECKS,True),'files':{name:{'size':(self.stage/name).stat().st_size,'sha256':digest(self.stage/name)}for name in FILES}}
  (self.stage/'manifest.json').write_bytes(canonical(manifest));subprocess.run(['openssl','pkeyutl','-sign','-inkey',str(key),'-rawin','-in',str(self.stage/'manifest.json'),'-out',str(self.stage/'manifest.sig')],check=True)
 def tearDown(self):
  for p in reversed(self.patches):p.stop()
  self.temp.cleanup()
 def test_valid_signature_stages_without_changing_current(self):
  folder,manifest=installer.incoming(self.release);self.assertEqual(folder.name,self.release);self.assertTrue((folder/'downloads/latest.json').is_file());self.assertFalse((installer.BASE/'current').exists())
 def test_tampered_artifact_is_rejected(self):
  (self.stage/FILES[1]).write_bytes(b'tampered')
  with self.assertRaisesRegex(ValueError,'hash mismatch'):installer.incoming(self.release)
 def test_bad_signature_is_rejected(self):
  (self.stage/'manifest.sig').write_bytes(b'x'*64)
  with self.assertRaises(subprocess.CalledProcessError):installer.incoming(self.release)
 def test_stale_sequence_is_rejected(self):
  installer.TRUST.write_bytes(canonical({'highest_sequence':2,'releases':{}}))
  with self.assertRaisesRegex(ValueError,'Stale release'):installer.incoming(self.release)
 def test_fifo_artifact_is_rejected_without_blocking(self):
  path=self.stage/FILES[1];path.unlink();os.mkfifo(path)
  with self.assertRaisesRegex(ValueError,'regular file'):installer.incoming(self.release)
if __name__=='__main__':unittest.main()
