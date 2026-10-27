#!/usr/bin/env python3
"""Root-owned Pi update transaction. Only signed fixed-name artifacts are accepted."""
import argparse,stat,hashlib,json,os,pathlib,re,shutil,signal,subprocess,time,urllib.request,uuid
from protocol import FILES,ID,canonical,digest,validate_files
HOME=pathlib.Path('/home/orangepi/.cache/schema-updates')
BASE=pathlib.Path('/opt/schema');DATA=pathlib.Path('/var/lib/schema');STATE=pathlib.Path('/var/lib/schema-updates')
PUBLIC=pathlib.Path('/etc/schema/update-pubkey.pem');BARRIER=DATA/'.update-in-progress';SESSION=DATA/'.session/session.json'
JOURNAL=STATE/'transaction.json';STATUS=STATE/'status.json';TRUST=STATE/'trusted.json'

def atomic(path,data,mode=0o600,owner=None):
 path=pathlib.Path(path);path.parent.mkdir(parents=True,exist_ok=True);tmp=path.with_name('.'+path.name+'-'+uuid.uuid4().hex)
 with open(tmp,'xb')as out:os.chmod(tmp,mode);out.write(data);out.flush();os.fsync(out.fileno())
 if owner:os.chown(tmp,*owner)
 os.replace(tmp,path);fd=os.open(path.parent,os.O_RDONLY|os.O_DIRECTORY)
 try:os.fsync(fd)
 finally:os.close(fd)
def sync_dir(path):
 fd=os.open(path,os.O_RDONLY|os.O_DIRECTORY)
 try:os.fsync(fd)
 finally:os.close(fd)

def read(path,default=None):
 try:return json.loads(path.read_bytes())
 except FileNotFoundError:return default

def status(operation,release,phase,error=None,**extra):
 value={'operation_id':operation,'release_id':release,'phase':phase,'error':error,'updated_ns':time.monotonic_ns(),'highest_sequence':read(TRUST,{}).get('highest_sequence',0),**extra};atomic(STATUS,canonical(value),0o644);return value

def system(*args,check=True):return subprocess.run(['systemctl',*args],check=check,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
def point(path,target):
 tmp=path.with_name('.'+path.name+'-'+uuid.uuid4().hex);os.symlink(str(target),tmp);os.replace(tmp,path);sync_dir(path.parent)
def api(path,body=None):
 token=read(pathlib.Path('/etc/schema/access.json'))['schema_token'];request=urllib.request.Request('http://127.0.0.1:8787'+path,headers={'x-schema-token':token,'Content-Type':'application/json'},data=None if body is None else json.dumps(body).encode())
 with urllib.request.urlopen(request,timeout=8)as response:return json.load(response)
def ready(timeout=15):
 deadline=time.monotonic()+timeout
 while True:
  try:return api('/api/session')
  except(OSError,ValueError):
   if time.monotonic()>deadline:raise RuntimeError('Engine did not become healthy')
   time.sleep(.2)
def record(folder):
 name=folder.name;binary=folder/'schema-rs';downloads=folder/'downloads'
 return {'release_id':name,'binary_sha256':digest(binary),'downloads':str(downloads)if downloads.is_dir()else None}
def operation_id(release):
 try:
  value=read(HOME/release/'request.json',{})['operation_id']
  return value if re.fullmatch('[a-f0-9]{32}',value)else uuid.uuid4().hex
 except(KeyError,OSError,ValueError):return uuid.uuid4().hex

def incoming(release):
 if not ID.fullmatch(release):raise ValueError('Invalid release ID')
 stage=HOME/release;fd=os.open(stage,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW);temporary=BASE/'releases'/('.incoming-'+uuid.uuid4().hex);temporary.mkdir(mode=0o700,parents=True)
 try:
  for name in ('manifest.json','manifest.sig',*FILES):
   source=os.open(name,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK,dir_fd=fd)
   with os.fdopen(source,'rb')as source_file:
    metadata=os.fstat(source_file.fileno())
    if not stat.S_ISREG(metadata.st_mode):raise ValueError('Artifact is not a regular file')
    size=metadata.st_size
    if size<=0 or size>(8192 if name=='manifest.json'else 1024 if name=='manifest.sig'else 128*1024*1024):raise ValueError('Artifact exceeds limits')
    with open(temporary/name,'xb')as output:shutil.copyfileobj(source_file,output);output.flush();os.fsync(output.fileno())
  subprocess.run(['openssl','pkeyutl','-verify','-pubin','-inkey',str(PUBLIC),'-rawin','-in',str(temporary/'manifest.json'),'-sigfile',str(temporary/'manifest.sig')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  manifest=json.loads((temporary/'manifest.json').read_bytes());validate_files(temporary,manifest)
  if manifest['release_id']!=release:raise ValueError('Manifest identity mismatch')
  if subprocess.check_output(['uname','-m'],text=True).strip()!='aarch64':raise ValueError('Device is not ARM64')
  trust=read(TRUST,{'highest_sequence':0,'releases':{}})
  if manifest['sequence']<trust['highest_sequence']:raise ValueError('Stale release sequence rejected')
  desktop=read(temporary/'latest.json',{})
  if desktop.get('commit')!=manifest['source_commit'][:12] or desktop.get('sha256')!=manifest['files']['Schema-s-Windows-x64.zip']['sha256']:raise ValueError('Desktop release metadata mismatch')
  destination=BASE/'releases'/release
  if destination.exists():
   if digest(destination/'schema-rs')!=manifest['files']['schema-rs']['sha256']:raise ValueError('Existing immutable release differs')
   return destination,manifest
  (temporary/'downloads').mkdir()
  for name in FILES[1:]:shutil.move(temporary/name,temporary/'downloads'/name)
  os.chmod(temporary/'schema-rs',0o755);os.chmod(temporary,0o755);os.chmod(temporary/'downloads',0o755)
  for path in (temporary/'downloads').iterdir():os.chmod(path,0o644)
  atomic(temporary/'BUILD.json',canonical({'release_id':release,'source_commit':manifest['source_commit'],'sha256':manifest['files']['schema-rs']['sha256'],'target':manifest['target'],'sequence':manifest['sequence']}),0o644)
  sync_dir(temporary/'downloads');sync_dir(temporary);os.replace(temporary,destination);sync_dir(destination.parent);return destination,manifest
 finally:
  os.close(fd)
  if temporary.exists():shutil.rmtree(temporary)

def recover(live=False):
 journal=read(JOURNAL)
 if not journal or journal.get('phase')=='complete':
  if BARRIER.exists():BARRIER.unlink()
  return
 if live:system('stop','schema.service',check=False)
 checkpoint=journal.get('checkpoint')
 if checkpoint and pathlib.Path(checkpoint).exists():atomic(SESSION,pathlib.Path(checkpoint).read_bytes(),owner=tuple(journal['session_owner']))
 point(BASE/'current',journal['previous'])
 if journal.get('previous_download'):point(DATA/'downloads',journal['previous_download'])
 elif(DATA/'downloads').is_symlink():(DATA/'downloads').unlink()
 if live:system('start','schema.service')
 if BARRIER.exists():BARRIER.unlink()
 journal['phase']='complete';journal['result']='rolled_back';atomic(JOURNAL,canonical(journal))
 status(journal['operation_id'],journal['release_id'],'rolled_back',journal.get('error','Interrupted deployment recovered'),installed_release=pathlib.Path(journal['previous']).name,previous_release=journal.get('previous_release'))

def activate(folder,manifest,operation,rollback=False):
 release=folder.name;previous=os.readlink(BASE/'current');current=(BASE/'current').resolve();trust=read(TRUST,{'highest_sequence':0,'releases':{}});trust['releases'][current.name]=record(current)
 if current==folder.resolve():status(operation,release,'succeeded',installed_release=release,previous_release=trust.get('previous_release'),unchanged=True);return
 old_download=os.readlink(DATA/'downloads')if(DATA/'downloads').is_symlink()else None
 if(DATA/'downloads').exists()and old_download is None:raise ValueError('Unexpected downloads directory; refusing overwrite')
 ola=system('show','olad.service','-p','MainPID','--value').stdout.strip()
 status(operation,release,'checkpointing');api('/api/session/save',{})
 journal={'operation_id':operation,'release_id':release,'previous':previous,'previous_download':old_download,'previous_release':current.name,'phase':'preparing','checkpoint':None,'rollback':rollback}
 atomic(JOURNAL,canonical(journal));atomic(BARRIER,canonical({'release_id':release}));committed=False
 try:
  status(operation,release,'stopping');system('stop','schema.service');session_bytes=SESSION.read_bytes();expected=json.loads(session_bytes);backups=STATE/'backups';backups.mkdir(mode=0o700,exist_ok=True);checkpoint=backups/(operation+'.json');atomic(checkpoint,session_bytes);metadata=SESSION.stat();journal.update(checkpoint=str(checkpoint),session_owner=[metadata.st_uid,metadata.st_gid],phase='stopped');atomic(JOURNAL,canonical(journal))
  point(BASE/'current','releases/'+release)
  if(folder/'downloads').is_dir():point(DATA/'downloads',folder/'downloads')
  journal['phase']='switched';atomic(JOURNAL,canonical(journal));status(operation,release,'verifying');system('start','schema.service');restored=ready()
  for key in ('project','undo','redo'):
   if restored[key]!=expected[key]:raise ValueError('Restored '+key+' differs')
  if restored['snapshot']['revision']!=expected['snapshot']['revision']:raise ValueError('Restored revision differs')
  if manifest.get('schema')==1 and not rollback:
   for key in ('inputs','states','plugins'):
    if restored.get(key)!=expected.get(key):raise ValueError('Restored '+key+' differs')
   for key in ('frame','time','paused','blackout'):
    if restored['snapshot'].get(key)!=expected['snapshot'].get(key):raise ValueError('Restored clock/control '+key+' differs')
  if system('show','olad.service','-p','MainPID','--value').stdout.strip()!=ola:raise ValueError('OLA changed during update')
  if system('is-active','schema.service').stdout.strip()!='active':raise ValueError('Engine service unhealthy')
  trust['releases'][release]=record(folder);trust['highest_sequence']=max(trust['highest_sequence'],manifest.get('sequence',0));trust['previous_release']=current.name;atomic(TRUST,canonical(trust))
  journal.update(phase='complete',result='succeeded');atomic(JOURNAL,canonical(journal));committed=True
  BARRIER.unlink(missing_ok=True)
  status(operation,release,'succeeded',installed_release=release,previous_release=current.name,highest_sequence=trust['highest_sequence'],revision=restored['snapshot']['revision'],output_enabled=restored['project']['output']['enabled'],history_preserved=True,ola_unchanged=True)
 except BaseException as error:
  if not committed:
   journal['error']=str(error);atomic(JOURNAL,canonical(journal));recover(live=True)
  else:BARRIER.unlink(missing_ok=True)
  raise

def execute(action,release):
 if not re.fullmatch(r'[a-f0-9]{12}(?:-[a-f0-9]{12}-[1-9][0-9]{0,9})?',release):raise ValueError('Invalid release name')
 operation=operation_id(release);status(operation,release,'validating')
 try:
  if action=='apply':folder,manifest=incoming(release)
  else:
   trust=read(TRUST,{'releases':{}});entry=trust['releases'].get(release)
   if not entry:raise ValueError('Rollback target is not a previously trusted release')
   folder=BASE/'releases'/release
   if digest(folder/'schema-rs')!=entry['binary_sha256']:raise ValueError('Rollback binary changed')
   manifest=read(folder/'manifest.json',{'sequence':0})
  activate(folder,manifest,operation,rollback=action=='rollback')
 except BaseException as error:
  latest=read(STATUS,{})
  if latest.get('phase')!='rolled_back':status(operation,release,'failed',str(error))
  raise

def main():
 import fcntl
 if os.geteuid()!=0:raise SystemExit('Root is required')
 STATE.mkdir(mode=0o755,exist_ok=True);parser=argparse.ArgumentParser();parser.add_argument('action',choices=['apply','rollback','recover']);parser.add_argument('release',nargs='?');parser.add_argument('--live',action='store_true');args=parser.parse_args()
 with open(STATE/'lock','a')as lock:
  try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
  except BlockingIOError:
   if args.action=='recover':return
   raise SystemExit('Another device update is running')
  if args.action=='recover':recover(args.live)
  else:execute(args.action,args.release or '')
if __name__=='__main__':
 try:main()
 except Exception as error:print('Update failed: '+str(error),flush=True);raise SystemExit(1)
