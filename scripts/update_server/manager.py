"""Serial device transactions and an independent committed-source build queue."""
import hashlib,pathlib,queue,threading,time,paramiko
from builder import build,source_identity
from core import choose_release,utc
from device import Device
TERMINAL={'succeeded','failed','rolled_back'}
class Manager:
 def __init__(self,config,store,signer):
  self.config=config;self.store=store;self.signer=signer;self.stop=threading.Event();self.lock=threading.RLock();self.build_queue=queue.Queue();self.push_queue=queue.Queue();self.building=False;self.pushing=False;self.device={'online':False,'service':'unknown','checked_at':None};self.threads=[];store.db.recover()
  for job in reversed(store.db.jobs()):
   if job['status']=='queued':
    if job['kind']=='build':self.building=True;self.build_queue.put(job['id'])
    elif job['kind']in('push','rollback'):self.pushing=True;self.push_queue.put(job)
 def start(self):
  for target in [self.build_loop,self.device_loop,self.watch_loop]:
   thread=threading.Thread(target=target,daemon=True);thread.start();self.threads.append(thread)
 def close(self):
  self.stop.set()
  for thread in self.threads:thread.join(timeout=8)
 def enqueue_build(self):
  with self.lock:
   if self.building:raise ValueError('A build is already queued or running')
   job=self.store.db.create('build');self.building=True;self.build_queue.put(job);return job
 def enqueue_push(self,release_id,rollback=False):
  with self.lock:
   if self.pushing:raise ValueError('A device push is already queued or running')
   if rollback:
    if release_id!=self.device.get('previous_release'):raise ValueError('Only the previous trusted release can be rolled back')
   else:self.store.verify(release_id,self.signer)
   self.store.db.set('pinned',release_id if rollback else None);job_id=self.store.db.create('rollback'if rollback else'spush',release_id);self.pushing=True;self.push_queue.put({'id':job_id,'kind':'rollback'if rollback else'spush','release_id':release_id});return job_id
 def action(self,value):
  action=value.get('action');unknown=set(value)-{'action','release_id'}
  if unknown:raise ValueError('Unknown action fields')
  if action=='build':return {'job':self.enqueue_build()}
  if action in('push','retry'):
   release_id=value.get('release_id')or(self.store.latest()or{}).get('release_id')
   if not release_id:raise ValueError('No ready release')
   if action=='retry':self.store.db.set('blocked',[r for r in self.store.db.get('blocked',[])if r!=release_id])
   return{'job':self.enqueue_push(release_id)}
  if action=='rollback':return{'job':self.enqueue_push(self.device.get('previous_release',''),True)}
  if action in('pause','resume'):
   self.store.db.set('auto_deploy',action=='resume')
   if action=='resume':self.store.db.set('pinned',None)
   return{'automatic':action=='resume'}
  if action in('watch_on','watch_off'):self.store.db.set('auto_build',action=='watch_on');return{'watch':action=='watch_on'}
  raise ValueError('Unknown action')
 def state(self):
  with self.lock:device=dict(self.device);building=self.building;pushing=self.pushing
  try:commit,tree=source_identity(self.config['repo'])
  except Exception:commit,tree=None,None
  return{'service':'schema-update-server','device':device,'releases':self.store.list(),'jobs':self.store.db.jobs(),'events':self.store.db.events(),'auto_deploy':self.store.db.get('auto_deploy',True),'auto_build':self.store.db.get('auto_build',True),'pinned':self.store.db.get('pinned'),'blocked':self.store.db.get('blocked',[]),'building':building,'pushing':pushing,'source_commit':commit,'source_tree':tree,'public_key_sha256':hashlib.sha256(self.signer.public_pem()).hexdigest(),'at':utc()}
 def build_loop(self):
  while not self.stop.is_set():
   try:job=self.build_queue.get(timeout=1)
   except queue.Empty:continue
   try:build(self.config['repo'],self.store,self.signer,self.config,job,self.stop)
   except Exception:pass
   finally:
    with self.lock:self.building=False
 def watch_loop(self):
  while not self.stop.wait(5):
   if not self.store.db.get('auto_build',True):continue
   try:
    _,tree=source_identity(self.config['repo'])
    if not self.building and tree!=self.store.db.get('last_attempt_tree')and tree!=self.store.db.get('last_ready_tree'):self.enqueue_build()
   except(Exception):pass
 def refresh_device(self,device):
  device.connect();result=device.probe();build_info=result.get('build')or{};update=result.get('update')or{};result['installed_release']=build_info.get('release_id')or build_info.get('source_commit');result['previous_release']=update.get('previous_release');result['checked_at']=utc()
  if result.get('public_key_sha256')not in(None,hashlib.sha256(self.signer.public_pem()).hexdigest()):raise RuntimeError('Device signing identity differs; explicit key rotation required')
  self.store.db.ensure_sequence(int(update.get('highest_sequence',0)))
  if not result['helper']:
   device.bootstrap(self.signer);result['helper']=True
  with self.lock:self.device=result
  return result
 def reconcile(self,info):
  update=info.get('update')or{};operation=update.get('operation_id');phase=update.get('phase')
  if operation and phase in TERMINAL:
   for job in self.store.db.jobs():
    if job['id']==operation and job['status']not in('succeeded','failed'):
     ok=phase=='succeeded';self.store.db.update(operation,status='succeeded'if ok else'failed',phase=phase,detail=update.get('error')or'');self.store.db.event(operation,'Device transaction reconciled: '+phase)
 def device_loop(self):
  device=Device(self.config['device'])
  while not self.stop.is_set():
   try:
    info=self.refresh_device(device);self.reconcile(info)
    if not self.pushing:
     release=choose_release(self.store.list(),info.get('installed_release'),paused=not self.store.db.get('auto_deploy',True),pinned=self.store.db.get('pinned'),blocked=self.store.db.get('blocked',[]))
     if release:self.enqueue_push(release['release_id'])
    if not self.push_queue.empty():
     job=self.push_queue.get_nowait();self.deploy(device,job)
   except Exception as error:
    with self.lock:self.device={**self.device,'online':False,'error':str(error),'checked_at':utc()}
   finally:device.close()
   self.stop.wait(self.config.get('poll_seconds',10))
 def deploy(self,device,job):
  db=self.store.db;identity=job['id'];release=job['release_id'];rollback=job['kind']=='rollback';db.update(identity,status='running',phase='uploading');db.event(identity,'Pinned SSH connection verified; uploading immutable artifacts')
  try:
   folder=None if rollback else self.store.path(release)
   if not rollback:self.store.verify(release,self.signer)
   operation=device.push(folder,release,rollback=rollback,operation=identity);deadline=time.monotonic()+220;last=None
   while not self.stop.is_set():
    try:
     result=device.probe();update=result.get('update')or{};phase=update.get('phase')
     if update.get('operation_id')==operation:
      if phase!=last:db.update(identity,phase=phase);db.event(identity,'Device: '+phase);last=phase
      with self.lock:self.device={**self.device,**result,'checked_at':utc()}
      if phase in TERMINAL:
       if phase!='succeeded':raise RuntimeError(update.get('error')or'Device rolled back')
       db.update(identity,status='succeeded',phase='succeeded',detail='Scene/history preserved; OLA unchanged');db.event(identity,'Healthy release committed; engine barrier released');return
    except(ConnectionError,OSError,TimeoutError,paramiko.SSHException):
     db.update(identity,phase='waiting for reconnect');device.close()
     try:device.connect()
     except Exception:pass
    if time.monotonic()>deadline:raise TimeoutError('Waiting for device completion timed out; device recovery continues independently')
    self.stop.wait(1)
   db.update(identity,status='interrupted',phase='server stopping; device continues')
  except Exception as error:
   db.update(identity,status='failed',phase='failed',detail=str(error));db.event(identity,str(error));blocked=self.store.db.get('blocked',[])
   if release not in blocked:db.set('blocked',blocked+[release])
  finally:
   with self.lock:self.pushing=False
