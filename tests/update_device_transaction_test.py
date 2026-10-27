"""Linux-only transaction fault tests. Every path/service/API is isolated or mocked."""
import json,pathlib,sys,tempfile,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'scripts/update_server'))
import apply as installer
from protocol import canonical
class Transactions(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.root=pathlib.Path(self.temp.name);base=self.root/'opt';data=self.root/'data';state=self.root/'state'
  self.patches=[]
  for name,value in {'BASE':base,'DATA':data,'STATE':state,'BARRIER':data/'.update-in-progress','SESSION':data/'.session/session.json','JOURNAL':state/'transaction.json','STATUS':state/'status.json','TRUST':state/'trusted.json'}.items():
   p=patch.object(installer,name,value);p.start();self.patches.append(p)
  for folder in [base/'releases',data/'.session',state]:folder.mkdir(parents=True,exist_ok=True)
  self.old=base/'releases'/('a'*12);self.new=base/'releases'/('b'*12+'-'+'c'*12+'-1')
  for folder in [self.old,self.new]:folder.mkdir();(folder/'schema-rs').write_bytes(b'executable');(folder/'downloads').mkdir()
  (base/'current').symlink_to('releases/'+self.old.name);(data/'downloads').symlink_to(self.old/'downloads')
  self.session={'project':{'output':{'enabled':False},'name':'Keep me'},'undo':[{'name':'Previous'}],'redo':[],'snapshot':{'revision':19,'time':42}}
  installer.SESSION.write_bytes(canonical(self.session));self.calls=[]
  def system(*args,**kwargs):
   self.calls.append(args)
   class Result:stdout='999'if args[0]=='show'else'active'
   return Result()
  for name,fn in [('system',system),('api',lambda *args:self.session)]:p=patch.object(installer,name,fn);p.start();self.patches.append(p)
 def tearDown(self):
  for p in reversed(self.patches):p.stop()
  self.temp.cleanup()
 def test_success_retains_checkpoint_and_records_trusted_previous(self):
  with patch.object(installer,'ready',return_value=self.session):installer.activate(self.new,{'schema':1,'sequence':1},'d'*32)
  self.assertEqual((installer.BASE/'current').resolve(),self.new);self.assertFalse(installer.BARRIER.exists());self.assertEqual(json.loads(installer.SESSION.read_bytes()),self.session)
  status=installer.read(installer.STATUS);self.assertEqual(status['phase'],'succeeded');self.assertTrue(status['history_preserved']);self.assertTrue(status['ola_unchanged']);self.assertEqual(status['previous_release'],self.old.name)
  self.assertEqual(installer.read(installer.TRUST)['highest_sequence'],1)
  with patch.object(installer,'ready',return_value=self.session):installer.activate(self.new,{'schema':1,'sequence':1},'f'*32)
  self.assertEqual(installer.read(installer.STATUS)['previous_release'],self.old.name)
 def test_failed_health_check_restores_links_and_exact_scene(self):
  broken={**self.session,'undo':[]}
  with patch.object(installer,'ready',return_value=broken),self.assertRaisesRegex(ValueError,'undo differs'):installer.activate(self.new,{'schema':1,'sequence':1},'e'*32)
  self.assertEqual((installer.BASE/'current').resolve(),self.old);self.assertEqual((installer.DATA/'downloads').resolve(),self.old/'downloads');self.assertFalse(installer.BARRIER.exists());self.assertEqual(json.loads(installer.SESSION.read_bytes()),self.session);self.assertEqual(installer.read(installer.STATUS)['phase'],'rolled_back')
 def test_running_clock_during_validation_triggers_recovery(self):
  changed={**self.session,'snapshot':{**self.session['snapshot'],'time':43}}
  with patch.object(installer,'ready',return_value=changed),self.assertRaisesRegex(ValueError,'clock/control time'):installer.activate(self.new,{'schema':1,'sequence':1},'e'*32)
  self.assertEqual((installer.BASE/'current').resolve(),self.old);self.assertFalse(installer.BARRIER.exists())
 def test_boot_recovers_interrupted_switch_without_starting_services(self):
  owner=installer.SESSION.stat();backup=installer.STATE/'backup.json';backup.write_bytes(canonical(self.session));installer.atomic(installer.JOURNAL,canonical({'phase':'switched','checkpoint':str(backup),'session_owner':[owner.st_uid,owner.st_gid],'previous':'releases/'+self.old.name,'previous_download':str(self.old/'downloads'),'operation_id':'f'*32,'release_id':self.new.name}));installer.point(installer.BASE/'current','releases/'+self.new.name);installer.BARRIER.write_bytes(b'held');installer.recover()
  self.assertEqual((installer.BASE/'current').resolve(),self.old);self.assertFalse(self.calls);self.assertFalse(installer.BARRIER.exists())
if __name__=='__main__':unittest.main()
