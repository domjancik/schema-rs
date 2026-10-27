import sys,pathlib,tempfile,json,unittest,hashlib,threading,urllib.request,urllib.error
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'scripts/update_server'))
from core import ReleaseStore,Signer,choose_release
class Releases(unittest.TestCase):
 def artifacts(self,root):
  source=root/'inputs';source.mkdir();elf=bytearray(128);elf[:6]=b'\x7fELF\x02\x01';elf[18:20]=(183).to_bytes(2,'little');(source/'schema-rs').write_bytes(elf)
  (source/'Schema-s-Windows-x64.zip').write_bytes(b'PK-test');(source/'latest.json').write_text(json.dumps({'commit':'a'*12,'sha256':hashlib.sha256(b'PK-test').hexdigest(),'bytes':7}));(source/'SHA256SUMS.txt').write_text('test');return source
 def test_only_complete_verified_releases_become_ready_and_tampering_fails(self):
  with tempfile.TemporaryDirectory() as work, ReleaseStore(pathlib.Path(work)/'state') as store:
   root=pathlib.Path(work);signer=Signer.generate();source=self.artifacts(root)
   with self.assertRaises(ValueError):store.publish(source,'a'*40,'b'*64,{},signer)
   self.assertIsNone(store.latest())
   release=store.publish(source,'a'*40,'b'*64,dict.fromkeys(['fmt','tests','clippy','syntax','smoke'],True),signer);self.assertEqual(store.latest()['release_id'],release['release_id']);store.verify(release['release_id'],signer)
   (store.path(release['release_id'])/'schema-rs').write_bytes(b'tampered')
   with self.assertRaises(ValueError):store.verify(release['release_id'],signer)
 def test_failed_release_and_manual_rollback_pin_do_not_auto_redeploy(self):
  releases=[{'release_id':'one','sequence':1},{'release_id':'two','sequence':2}]
  self.assertIsNone(choose_release(releases,'one',paused=True));self.assertIsNone(choose_release(releases,'one',pinned='one'));self.assertIsNone(choose_release(releases,'one',blocked={'two'}));self.assertEqual(choose_release(releases,'one')['release_id'],'two');self.assertIsNone(choose_release(releases,'two'))
class Dashboard(unittest.TestCase):
 def setUp(self):
  from http_ui import Server
  class Fake:
   def __init__(self):self.actions=[]
   def state(self):return {'device':{'online':False}}
   def action(self,value):
    if value.get('action')!='pause':raise ValueError('Unknown action')
    self.actions.append(value);return {'automatic':False}
  self.manager=Fake();self.server=Server(('127.0.0.1',0),self.manager);self.thread=threading.Thread(target=self.server.serve_forever,daemon=True);self.thread.start();self.base='http://127.0.0.1:'+str(self.server.server_port)
 def tearDown(self):self.server.shutdown();self.server.server_close();self.thread.join()
 def request(self,path='/',data=None,headers=None):return urllib.request.urlopen(urllib.request.Request(self.base+path,data=data,headers=headers or{}),timeout=3)
 def test_gui_requires_session_same_origin_and_csrf(self):
  with self.request()as response:cookie=response.headers['Set-Cookie'].split(';')[0];self.assertIn(b'Sch',response.read())
  with self.request('/api/state',headers={'Cookie':cookie})as response:csrf=json.load(response)['csrf']
  valid={'Cookie':cookie,'Origin':self.base,'X-Update-CSRF':csrf,'Content-Type':'application/json'}
  for override in [{'Origin':'https://untrusted.example'},{'X-Update-CSRF':'bad'},{'Host':'evil.example'}]:
   with self.assertRaises(urllib.error.HTTPError)as error:self.request('/api/action',b'{"action":"pause"}',{**valid,**override})
   self.assertEqual(error.exception.code,403)
  self.assertFalse(self.manager.actions)
  with self.request('/api/action',b'{"action":"pause"}',valid)as response:self.assertEqual(response.status,202)
  self.assertEqual(len(self.manager.actions),1)
  with self.assertRaises(urllib.error.HTTPError)as error:self.request('/api/state')
  self.assertEqual(error.exception.code,401)

class Recovery(unittest.TestCase):
 def test_queued_push_survives_restart_running_job_is_interrupted(self):
  with tempfile.TemporaryDirectory()as root,ReleaseStore(root)as store:
   queued=store.db.create('push','ready');running=store.db.create('build');store.db.update(running,status='running');store.db.recover();jobs={j['id']:j for j in store.db.jobs()};self.assertEqual(jobs[queued]['status'],'queued');self.assertEqual(jobs[running]['status'],'interrupted')
   store.db.ensure_sequence(22);self.assertEqual(store.db.sequence(),23);store.db.ensure_sequence(2);self.assertEqual(store.db.sequence(),24)
if __name__=='__main__':unittest.main()
