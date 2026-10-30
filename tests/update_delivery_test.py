import pathlib,sys,tempfile,unittest,json,zipfile,subprocess,socket,os
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'scripts/update_server'))
from core import ReleaseStore,Signer
from delivery import Delivery,seal_public,prepared_public
from delivery import workflow_runs,Pending
from local_host import same_session,pristine,LocalHost
class DeliveryPolicy(unittest.TestCase):
 def test_commit_watcher_covers_browser_crate_and_public_packaging(self):
  from builder import source_identity
  with tempfile.TemporaryDirectory()as directory:
   root=pathlib.Path(directory)
   def git(*args):subprocess.run(['git',*args],cwd=root,check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   git('init');git('config','user.name','Test');git('config','user.email','test@example.invalid')
   for name in ['src/main.rs','crates/schema-web/src/lib.rs','scripts/publication/install.sh','tests/managed_shutdown.rs']:
    p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text('first')
   git('add','.');git('commit','-m','initial');previous=source_identity(root)[1]
   for name in ['crates/schema-web/src/lib.rs','scripts/publication/install.sh','tests/managed_shutdown.rs']:
    (root/name).write_text('changed');git('add',name);git('commit','-m','changed');current=source_identity(root)[1];self.assertNotEqual(previous,current);previous=current

 def test_publication_waits_for_both_exact_commit_workflows_and_rejects_failures(self):
  commit='a'*40
  with self.assertRaises(Pending):workflow_runs([],commit)
  runs=[{'workflowName':name,'headSha':commit,'status':'completed','conclusion':'success','databaseId':i}for i,name in enumerate(['Browser engine','macOS'])]
  self.assertEqual(set(workflow_runs(runs,commit)),{'Browser engine','macOS'})
  runs[1]['headSha']='b'*40
  with self.assertRaises(Pending):workflow_runs(runs,commit)
  runs[1]['headSha']=commit;runs[1]['conclusion']='failure'
  with self.assertRaises(ValueError):workflow_runs(runs,commit)
 def test_restore_checks_authored_state_history_inputs_and_clock(self):
  expected={'project':{'output':{'enabled':False}},'undo':[1],'redo':[],'snapshot':{'revision':3,'frame':10,'time':2,'paused':False,'blackout':False}}
  same_session(expected,json.loads(json.dumps(expected)))
  for key in ['project','undo','inputs']:
   actual=json.loads(json.dumps(expected));actual[key]='changed'
   with self.assertRaises(ValueError):same_session(expected,actual)
  self.assertFalse(pristine(expected));expected['snapshot']['revision']=0;expected['undo']=[];self.assertTrue(pristine(expected))
 def test_target_ids_cannot_escape_private_state(self):
  with tempfile.TemporaryDirectory()as root:
   with self.assertRaises(ValueError):LocalHost(root,{'id':'../../outside','project':root,'port':8797})
class LocalRecovery(unittest.TestCase):
 def test_startup_barrier_uses_the_engine_project_path(self):
  with tempfile.TemporaryDirectory()as directory:
   host=LocalHost(directory,{'id':'test','project':directory,'port':8788})
   self.assertEqual(host.barrier,pathlib.Path(directory).resolve()/'.update-in-progress')
   with self.assertRaises(ValueError):host.process_config('engine','stop',pathlib.Path(directory)/'wrong-barrier')
   self.assertNotIn('--update-barrier',host.process_config('engine','stop',host.barrier))

 @unittest.skipUnless(os.name=='nt','Windows listener query')
 def test_actual_windows_listener_handles_unused_and_owned_ports(self):
  from local_host import listener,ProcessHandle
  with socket.socket()as sock:
   sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
   self.assertIsNone(listener(port))
   sock.listen();owner=listener(port);self.assertEqual(owner['ProcessId'],os.getpid())
   handle=ProcessHandle(owner['ProcessId'],owner['ExecutablePath']);handle.close()

 def test_failed_candidate_restores_original_checkpoint(self):
  with tempfile.TemporaryDirectory()as directory:
   root=pathlib.Path(directory);project=root/'project';(project/'.session').mkdir(parents=True)
   checkpoint=b'{"version":2,"project":{"name":"original"},"undo":[1],"redo":[],"snapshot":{"revision":3}}';(project/'.session/session.json').write_bytes(checkpoint)
   host=LocalHost(root,{'id':'test','project':str(project),'port':8788});folder=root/'ready';folder.mkdir()
   with zipfile.ZipFile(folder/'Schema-s-Windows-x64.zip','w')as archive:archive.writestr('Schema-s/schema-rs.exe',b'MZtest')
   def migrate(args,**kwargs):pathlib.Path(args[args.index('--output')+1]).write_bytes(checkpoint)
   with patch('local_host.listener',return_value=None),patch('local_host.subprocess.run',side_effect=migrate),patch.object(host,'start',return_value=object()),patch.object(host,'ready',side_effect=RuntimeError('health failed')):
    with self.assertRaisesRegex(RuntimeError,'health failed'):host.activate(folder,{'release_id':'test-release','source_commit':'a'*40})
   self.assertEqual((project/'.session/session.json').read_bytes(),checkpoint)
   self.assertEqual(json.loads(host.journal.read_bytes())['result'],'rolled_back')
 def test_untrusted_windows_payload_never_starts(self):
  with tempfile.TemporaryDirectory()as directory:
   root=pathlib.Path(directory);host=LocalHost(root,{'id':'test','project':directory,'port':8788});folder=root/'ready';folder.mkdir()
   with zipfile.ZipFile(folder/'Schema-s-Windows-x64.zip','w')as archive:archive.writestr('Schema-s/schema-rs.exe',b'not executable')
   with patch('local_host.listener',return_value=None),patch.object(host,'start')as start:
    with self.assertRaises(ValueError):host.activate(folder,{'release_id':'test-release','source_commit':'a'*40})
    start.assert_not_called()
class PublicSafety(unittest.TestCase):
 def test_prepared_publication_is_immutable_and_source_bound(self):
  with tempfile.TemporaryDirectory()as directory:
   root=pathlib.Path(directory);(root/'site').mkdir();file=root/'site/test';file.write_bytes(b'original');self.assertIsNone(prepared_public(root,'a'));seal_public(root,'a',{'platform':{}});self.assertEqual(prepared_public(root,'a'),{'platform':{}})
   with self.assertRaises(ValueError):prepared_public(root,'b')
   file.write_bytes(b'tampered')
   with self.assertRaises(ValueError):prepared_public(root,'a')
 def test_unattended_command_has_bounded_lifetime(self):
  import threading
  with tempfile.TemporaryDirectory()as directory,ReleaseStore(directory)as store:
   delivery=Delivery({},store,Signer.generate(),threading.Event());self.assertEqual(delivery.command([sys.executable,'-c','print("ok")']).strip(),'ok')
   with self.assertRaisesRegex(RuntimeError,'timed out'):delivery.command([sys.executable,'-c','import time;time.sleep(10)'],timeout=.1)

 def test_invalid_target_configuration_cannot_run_commands(self):
  import threading
  with tempfile.TemporaryDirectory()as directory,ReleaseStore(directory)as store:
   delivery=Delivery({'publication':{'enabled':True,'scope':'team & bad'}},store,Signer.generate(),threading.Event())
   with patch.object(delivery,'command')as command:
    with self.assertRaises(ValueError):delivery.public_once()
    command.assert_not_called()
if __name__=='__main__':unittest.main()
