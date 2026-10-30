"""Allowlisted Windows engine activation with durable checkpoints and a startup barrier."""
import ctypes,json,os,pathlib,re,subprocess,time,urllib.request,uuid,zipfile
from core import atomic
SESSION_KEYS=('version','project','undo','redo','inputs','states','plugins','resources','checkpoints')
SNAPSHOT_KEYS=('revision','frame','time','paused','blackout')

def same_session(expected,actual):
 for key in SESSION_KEYS:
  if expected.get(key)!=actual.get(key):raise ValueError('Restored '+key+' differs')
 for key in SNAPSHOT_KEYS:
  if expected['snapshot'].get(key)!=actual['snapshot'].get(key):raise ValueError('Restored '+key+' differs')

def pristine(session):
 return session['snapshot']['revision']==0 and not session.get('undo') and not session.get('redo') and not session['project']['output']['enabled']

def listener(port):
 if type(port)is not int or not 1<=port<=65535:raise ValueError('Invalid local port')
 code="[Console]::OutputEncoding=[Text.UTF8Encoding]::new();$c=@(Get-NetTCPConnection -State Listen -ErrorAction Stop | Where-Object LocalPort -eq "+str(port)+"); if($c.Count){Get-CimInstance Win32_Process -ErrorAction Stop -Filter ('ProcessId='+$c[0].OwningProcess)|Select-Object ProcessId,ExecutablePath|ConvertTo-Json -Compress};exit 0"
 result=subprocess.run(['powershell.exe','-NoProfile','-NonInteractive','-Command',code],capture_output=True,text=True,encoding='utf-8',check=True,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
 return json.loads(result.stdout)if result.stdout.strip()else None

class ProcessHandle:
 def __init__(self,pid,expected):
  from ctypes import wintypes as w
  self.api=ctypes.WinDLL('kernel32',use_last_error=True);self.api.OpenProcess.argtypes=[w.DWORD,w.BOOL,w.DWORD];self.api.OpenProcess.restype=w.HANDLE
  self.api.QueryFullProcessImageNameW.argtypes=[w.HANDLE,w.DWORD,w.LPWSTR,ctypes.POINTER(w.DWORD)];self.api.TerminateProcess.argtypes=[w.HANDLE,w.UINT];self.api.WaitForSingleObject.argtypes=[w.HANDLE,w.DWORD];self.api.CloseHandle.argtypes=[w.HANDLE]
  self.handle=self.api.OpenProcess(0x1000|0x100000|1,False,int(pid))
  if not self.handle:raise OSError('Cannot open local engine process')
  buffer=ctypes.create_unicode_buffer(32768);size=w.DWORD(len(buffer))
  if not self.api.QueryFullProcessImageNameW(self.handle,0,buffer,ctypes.byref(size))or pathlib.Path(buffer.value).resolve()!=pathlib.Path(expected).resolve():self.close();raise ValueError('Local process identity differs; refusing to stop it')
 def wait(self,seconds=15):
  if self.api.WaitForSingleObject(self.handle,int(seconds*1000))!=0:raise TimeoutError('Local engine did not stop gracefully')
 def stop_pristine_demo(self):
  if not self.api.TerminateProcess(self.handle,0):raise OSError('Cannot adopt idle demo engine')
  self.wait()
 def close(self):
  if self.handle:self.api.CloseHandle(self.handle);self.handle=None

def api(config,path,body=None):
 headers={'Content-Type':'application/json'}
 if config.get('token_credential'):
  from device import windows_vault
  token=windows_vault.read(config['token_credential'])
  if not token:raise ValueError('Local API credential unavailable')
  headers['x-schema-token']=token
 request=urllib.request.Request('http://127.0.0.1:'+str(config['port'])+path,headers=headers,data=None if body is None else json.dumps(body).encode())
 with urllib.request.urlopen(request,timeout=8)as response:return json.load(response)

class LocalHost:
 def __init__(self,root,config):
  if not re.fullmatch(r'[a-zA-Z0-9_-]{1,32}',config['id']):raise ValueError('Invalid local target ID')
  self.config=config;self.root=pathlib.Path(root)/'hosts'/config['id'];self.root.mkdir(parents=True,exist_ok=True);self.state_path=self.root/'current.json';self.journal=self.root/'transaction.json';self.project=pathlib.Path(config['project']).resolve();self.session=self.project/'.session/session.json';self.barrier=self.project/'.update-in-progress'
  if not self.project.is_dir():raise ValueError('Local project directory does not exist')
 def read(self,path):
  try:return json.loads(path.read_bytes())
  except FileNotFoundError:return None
 def process_config(self,executable,stop_file,barrier):
  if pathlib.Path(barrier).resolve()!=self.barrier:raise ValueError('Local startup barrier must use the engine project path')
  args=[str(executable),'serve','--bind','127.0.0.1:'+str(self.config['port']),'--project',str(self.project),'--shutdown-file',str(stop_file)]
  if self.session.exists():args+=['--resume-session',str(self.session)]
  return args
 def start(self,executable,stop_file,barrier):
  env=os.environ.copy();env.pop('SCHEMA_TOKEN',None)
  if self.config.get('token_credential'):
   from device import windows_vault
   env['SCHEMA_TOKEN']=windows_vault.read(self.config['token_credential'])or''
  with open(self.root/'engine.log','ab',buffering=0)as log:
   return subprocess.Popen(self.process_config(executable,stop_file,barrier),stdout=log,stderr=subprocess.STDOUT,env=env,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
 def ready(self,process):
  deadline=time.monotonic()+20
  while time.monotonic()<deadline:
   if process.poll()is not None:raise RuntimeError('Local candidate exited before becoming healthy')
   try:return api(self.config,'/api/session')
   except (OSError,ValueError):time.sleep(.15)
  raise TimeoutError('Local candidate did not become healthy')
 def recover(self):
  journal=self.read(self.journal)
  if not journal or journal.get('phase')=='complete':return
  owner=listener(self.config['port'])
  if owner:
   handle=ProcessHandle(owner['ProcessId'],journal['candidate'])
   try:atomic(journal['candidate_stop'],b'stop');handle.wait()
   finally:handle.close()
  if journal.get('backup'):atomic(self.session,pathlib.Path(journal['backup']).read_bytes())
  previous=journal.get('previous')
  if previous:
   barrier=self.barrier;atomic(barrier,b'hold');stop=self.root/('shutdown-'+uuid.uuid4().hex)
   process=self.start(previous['executable'],stop,barrier);self.ready(process);barrier.unlink(missing_ok=True)
   previous={**previous,'pid':process.pid,'shutdown_file':str(stop)};atomic(self.state_path,json.dumps(previous).encode())
  journal['phase']='complete';journal['result']='rolled_back';atomic(self.journal,json.dumps(journal).encode())
 def activate(self,folder,manifest):
  self.recover();release=manifest['release_id'];current=self.read(self.state_path);owner=listener(self.config['port'])
  if current and current.get('release_id')==release and owner:
   if pathlib.Path(owner['ExecutablePath']).resolve()!=pathlib.Path(current['executable']).resolve():raise ValueError('Local port belongs to another engine')
   return {'status':'current','release_id':release,'url':'http://127.0.0.1:'+str(self.config['port'])}
  destination=self.root/'releases'/release;destination.mkdir(parents=True,exist_ok=True);candidate=destination/'schema-rs.exe'
  if not candidate.exists():
   with zipfile.ZipFile(pathlib.Path(folder)/'Schema-s-Windows-x64.zip')as archive:data=archive.read('Schema-s/schema-rs.exe')
   if not data.startswith(b'MZ'):raise ValueError('Invalid Windows engine')
   atomic(candidate,data)
  previous=current
  if owner:
   allowed=pathlib.Path(current['executable'])if current else pathlib.Path(self.config.get('adopt_executable','__not_enrolled__'))
   handle=ProcessHandle(owner['ProcessId'],allowed)
   try:
    api(self.config,'/api/session/save',{});running=api(self.config,'/api/session')
    if not current or current.get('supports_shutdown') is False:
     if not pristine(running):return {'status':'waiting','detail':'Close the older desktop once to enroll it without risking active work'}
     previous_dir=self.root/'previous-install';previous_dir.mkdir(exist_ok=True);previous_exe=previous_dir/'schema-rs.exe'
     if not previous_exe.exists():atomic(previous_exe,allowed.read_bytes())
     previous={'executable':str(previous_exe),'release_id':'adopted-install','supports_shutdown':False};handle.stop_pristine_demo()
    else:
     stop=pathlib.Path(current['shutdown_file']).resolve()
     if not stop.is_relative_to(self.root.resolve()):raise ValueError('Invalid local shutdown path')
     atomic(stop,b'stop');handle.wait()
   finally:handle.close()
  backup=self.root/('checkpoint-'+uuid.uuid4().hex+'.json');expected=None
  if self.session.exists():atomic(backup,self.session.read_bytes())
  stop=self.root/('shutdown-'+uuid.uuid4().hex);barrier=self.barrier;atomic(barrier,b'hold')
  journal={'phase':'stopped','backup':str(backup)if backup.exists()else None,'previous':previous,'candidate':str(candidate),'candidate_stop':str(stop)};atomic(self.journal,json.dumps(journal).encode())
  process=None
  try:
   if backup.exists():
    normalized=backup.with_suffix('.migrated.json')
    subprocess.run([str(candidate),'migrate','--format','session','--input',str(backup),'--output',str(normalized)],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=60);expected=json.loads(normalized.read_bytes())
   process=self.start(candidate,stop,barrier);actual=self.ready(process)
   if expected:same_session(expected,actual)
   installed={'release_id':release,'executable':str(candidate),'pid':process.pid,'shutdown_file':str(stop),'source_commit':manifest['source_commit'],'supports_shutdown':True}
   atomic(self.state_path,json.dumps(installed).encode());journal['phase']='complete';journal['result']='succeeded';atomic(self.journal,json.dumps(journal).encode());barrier.unlink(missing_ok=True)
   return {'status':'current','release_id':release,'source_commit':manifest['source_commit'],'history_preserved':True,'revision':actual['snapshot']['revision'],'url':'http://127.0.0.1:'+str(self.config['port'])}
  except Exception as error:
   try:self.recover()
   except Exception as recovery_error:raise RuntimeError('Local activation failed: '+str(error)+'; rollback needs attention: '+str(recovery_error))from error
   raise
