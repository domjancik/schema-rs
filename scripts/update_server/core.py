"""Durable signed-release catalog and scheduler policy; no network or credentials in manifests."""
import base64,datetime,hashlib,json,os,pathlib,shutil,sqlite3,threading,uuid
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization
from protocol import FILES,CHECKS,ID,canonical,digest,validate_files

def utc():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def atomic(path,data):
 path=pathlib.Path(path);path.parent.mkdir(parents=True,exist_ok=True);temporary=path.with_name(path.name+'.'+uuid.uuid4().hex+'.tmp')
 with open(temporary,'xb')as out:out.write(data);out.flush();os.fsync(out.fileno())
 os.replace(temporary,path)
class Signer:
 def __init__(self,key):self.key=key
 @classmethod
 def generate(cls):return cls(Ed25519PrivateKey.generate())
 @classmethod
 def from_secret(cls,value):return cls(Ed25519PrivateKey.from_private_bytes(base64.b64decode(value,validate=True)))
 def secret(self):return base64.b64encode(self.key.private_bytes(serialization.Encoding.Raw,serialization.PrivateFormat.Raw,serialization.NoEncryption())).decode()
 def public_pem(self):return self.key.public_key().public_bytes(serialization.Encoding.PEM,serialization.PublicFormat.SubjectPublicKeyInfo)
 def sign(self,data):return self.key.sign(data)
 def verify(self,data,signature):
  try:self.key.public_key().verify(signature,data)
  except Exception as error:raise ValueError('Invalid release signature')from error
class Database:
 def __init__(self,path):
  self.lock=threading.RLock();self.connection=sqlite3.connect(path,check_same_thread=False);self.connection.row_factory=sqlite3.Row
  with self.connection:
   self.connection.executescript('PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY,kind TEXT,status TEXT,phase TEXT,created TEXT,updated TEXT,release_id TEXT,detail TEXT); CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY AUTOINCREMENT,job TEXT,at TEXT,message TEXT);')
   self.connection.execute("UPDATE jobs SET kind='push' WHERE kind='spush'")
 def close(self):
  with self.lock:self.connection.close()
 def get(self,key,default=None):
  with self.lock:
   row=self.connection.execute('SELECT value FROM kv WHERE key=?',(key,)).fetchone();return json.loads(row[0])if row else default
 def set(self,key,value):
  with self.lock,self.connection:self.connection.execute('INSERT OR REPLACE INTO kv VALUES(?,?)',(key,json.dumps(value)))
 def sequence(self):
  with self.lock,self.connection:
   row=self.connection.execute("SELECT value FROM kv WHERE key='sequence'").fetchone();value=(int(json.loads(row[0]))if row else 0)+1;self.connection.execute("INSERT OR REPLACE INTO kv VALUES('sequence',?)",(json.dumps(value),));return value
 def ensure_sequence(self,value):
  with self.lock:
   if value>self.get('sequence',0):self.set('sequence',value)
 def create(self,kind,release_id=''):
  job=uuid.uuid4().hex
  with self.lock,self.connection:self.connection.execute('INSERT INTO jobs VALUES(?,?,?,?,?,?,?,?)',(job,kind,'queued','queued',utc(),utc(),release_id,''))
  return job
 def update(self,job,status=None,phase=None,detail=None,release_id=None):
  values={key:value for key,value in [('status',status),('phase',phase),('detail',detail),('release_id',release_id)]if value is not None};values['updated']=utc()
  with self.lock,self.connection:self.connection.execute('UPDATE jobs SET '+','.join(key+'=?'for key in values)+' WHERE id=?',(*values.values(),job))
 def event(self,job,message):
  with self.lock,self.connection:self.connection.execute('INSERT INTO events(job,at,message) VALUES(?,?,?)',(job,utc(),str(message)[-3000:]));self.connection.execute('DELETE FROM events WHERE id<(SELECT COALESCE(MAX(id),0)-3000 FROM events)')
 def jobs(self):
  with self.lock:return[dict(row)for row in self.connection.execute('SELECT * FROM jobs ORDER BY created DESC LIMIT 60')]
 def events(self,job=None):
  with self.lock:
   rows=self.connection.execute('SELECT * FROM events WHERE job=? ORDER BY id DESC LIMIT 200',(job,))if job else self.connection.execute('SELECT * FROM events ORDER BY id DESC LIMIT 150');return list(reversed([dict(row)for row in rows]))
 def recover(self):
  with self.lock,self.connection:self.connection.execute("UPDATE jobs SET status='interrupted',phase='server restarted',updated=? WHERE status='running'",(utc(),))
class ReleaseStore:
 def __init__(self,root):
  self.root=pathlib.Path(root).resolve();self.root.mkdir(parents=True,exist_ok=True);self.releases=self.root/'releases';self.releases.mkdir(exist_ok=True);self.db=Database(self.root/'updates.sqlite3')
 def __enter__(self):return self
 def __exit__(self,*args):self.db.close()
 def path(self,release_id):
  if not ID.fullmatch(release_id):raise ValueError('Invalid release ID')
  return self.releases/release_id
 def publish(self,source,commit,tree,checks,signer):
  if not all(checks.get(name)is True for name in CHECKS):raise ValueError('Required checks missing')
  source=pathlib.Path(source);sequence=self.db.sequence();release_id=commit[:12]+'-'+digest(source/'schema-rs')[:12]+'-'+str(sequence);target=self.path(release_id)
  if target.exists():return self.verify(release_id,signer)
  staging=self.releases/('.staging-'+uuid.uuid4().hex);staging.mkdir()
  try:
   for name in FILES:shutil.copyfile(source/name,staging/name)
   manifest={'schema':1,'release_id':release_id,'sequence':sequence,'source_commit':commit,'tree':tree,'target':'aarch64-unknown-linux-musl','created_at':utc(),'checks':checks,'files':{name:{'size':(staging/name).stat().st_size,'sha256':digest(staging/name)}for name in FILES}}
   validate_files(staging,manifest);data=canonical(manifest);atomic(staging/'manifest.json',data);atomic(staging/'manifest.sig',signer.sign(data));os.replace(staging,target);atomic(self.root/'ready.json',canonical({'release_id':release_id}));self.db.set('last_ready_tree',tree);return manifest
  finally:
   if staging.exists():shutil.rmtree(staging)
 def verify(self,release_id,signer):
  folder=self.path(release_id);data=(folder/'manifest.json').read_bytes();signer.verify(data,(folder/'manifest.sig').read_bytes());return validate_files(folder,json.loads(data))
 def list(self):
  result=[]
  for folder in self.releases.iterdir():
   if ID.fullmatch(folder.name):
    try:result.append(json.loads((folder/'manifest.json').read_text()))
    except(OSError,ValueError):continue
  return sorted(result,key=lambda item:item['sequence'],reverse=True)
 def latest(self):
  releases=self.list();return releases[0]if releases else None

def choose_release(releases,installed,paused=False,pinned=None,blocked=()):
 if paused or pinned:return None
 newest=max(releases,key=lambda item:item['sequence'],default=None)
 if not newest or newest['release_id']==installed or newest['release_id']in blocked:return None
 return newest
