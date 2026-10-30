"""Automatic allowlisted local activation and verified public artifact publication."""
import hashlib,importlib.util,json,os,pathlib,re,shutil,subprocess,tarfile,time,uuid
from core import atomic,utc
from local_host import LocalHost

class Pending(Exception):pass

def workflow_runs(runs,commit):
 selected={}
 for name in ['Browser engine','macOS']:
  run=next((r for r in runs if r['workflowName']==name and r['headSha']==commit),None)
  if not run or run['status']!='completed':raise Pending('Waiting for '+name+' CI')
  if run['conclusion']!='success':raise ValueError(name+' CI failed; publication withheld')
  selected[name]=run['databaseId']
 return selected

def vercel_runner(node,config):
 if config.get('vercel_cli'):
  entry=pathlib.Path(config['vercel_cli'])
  if not entry.is_file():raise ValueError('Enrolled Vercel CLI entry point is missing')
  return [node,entry]
 npx=node.parent/'node_modules/npm/bin/npx-cli.js'
 if not npx.is_file():raise ValueError('The enrolled Node installation must include npm/npx')
 return [node,npx,'--yes','vercel@latest']

def seal_public(work,commit,platforms):
 files={p.relative_to(work).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()for p in (work/'site').rglob('*')if p.is_file()}
 atomic(work/'prepared.json',json.dumps({'commit':commit,'platforms':platforms,'files':files}).encode())
def prepared_public(work,commit):
 marker=work/'prepared.json'
 if not marker.exists():return None
 value=json.loads(marker.read_bytes())
 if value['commit']!=commit:raise ValueError('Prepared publication source differs')
 for name,digest in value['files'].items():
  path=work/name
  if path.is_symlink()or not path.resolve().is_relative_to(work.resolve())or hashlib.sha256(path.read_bytes()).hexdigest()!=digest:raise ValueError('Prepared publication checksum differs')
 return value['platforms']

class Delivery:
 def __init__(self,config,store,signer,stop):self.config=config;self.store=store;self.signer=signer;self.stop=stop;self.locals={}
 def state(self):return self.store.db.get('delivery_targets',{})
 def status(self,target,value):
  with self.store.db.lock:
   all_targets=self.state();old=all_targets.get(target,{})
   if {k:v for k,v in old.items()if k!='at'}!={k:v for k,v in value.items()if k!='at'}:
    all_targets[target]={**value,'at':utc()};self.store.db.set('delivery_targets',all_targets)
    if old.get('status')!=value.get('status')or old.get('detail')!=value.get('detail'):self.store.db.event(None,target+': '+value.get('detail',value.get('status','')))
 def ready(self):
  release=self.store.latest()
  if release:self.store.verify(release['release_id'],self.signer)
  return release
 def local_once(self):
  release=self.ready()
  if not release:return
  for config in self.config.get('local_hosts',[]):
   target='local:'+config['id'];previous=self.state().get(target,{})
   if previous.get('status')=='failed'and previous.get('release_id')==release['release_id']and self.store.db.get('delivery_retry_at',0)<=previous.get('attempt_at',0):continue
   try:
    if config['id']not in self.locals:self.locals[config['id']]=LocalHost(self.store.root,config)
    result=self.locals[config['id']].activate(self.store.path(release['release_id']),release);self.status(target,{**result,'name':config.get('name',config['id'])})
   except Exception as error:self.status(target,{'status':'failed','release_id':release['release_id'],'name':config.get('name',config['id']),'detail':str(error),'attempt_at':time.time()})
 def command(self,args,cwd=None,timeout=90):
  command=[str(v)for v in args];environment={**os.environ,'CI':'1'}
  for key in ['VERCEL_TOKEN','VERCEL_OIDC_TOKEN','VERCEL_AUTH_TOKEN','VERCEL_ORG_ID','VERCEL_PROJECT_ID','VERCEL_GLOBAL_CONFIG']:environment.pop(key,None)
  with subprocess.Popen(command,cwd=cwd,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',errors='replace',env=environment,start_new_session=os.name!='nt',creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))as process:
   try:stdout,stderr=process.communicate(timeout=timeout)
   except subprocess.TimeoutExpired:
    if os.name=='nt':subprocess.run(['taskkill','/PID',str(process.pid),'/T','/F'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    else:
     import signal
     os.killpg(process.pid,signal.SIGKILL)
    process.communicate();raise RuntimeError('Publication command timed out: '+command[0])
   if process.returncode:raise RuntimeError((stderr or stdout or command[0]+' failed')[-2000:])
   return stdout
 def ensure_source(self,commit):
  if not re.fullmatch('[a-f0-9]{40}',commit):raise ValueError('Invalid source commit')
  repo=self.config['repo'];remote=self.command(['git','remote','get-url','origin'],repo).strip();expected='https://github.com/'+self.config['publication']['github_repo']+'.git'
  if remote.rstrip('/')!=expected:raise ValueError('Publication remote differs from enrolled repository')
  head=self.command(['git','ls-remote','origin','refs/heads/main'],repo).split()[0]
  if head==commit:return
  ancestor=subprocess.run(['git','merge-base','--is-ancestor',head,commit],cwd=repo,capture_output=True).returncode
  if ancestor:
   if subprocess.run(['git','merge-base','--is-ancestor',commit,head],cwd=repo,capture_output=True).returncode==0:return
   raise ValueError('main advanced independently; refusing non-fast-forward publication')
  self.command(['git','-c','credential.helper=!gh auth git-credential','-c','push.followTags=false','push','origin',commit+':refs/heads/main'],repo)
 def artifact(self,work,run,name):
  target=work/name
  if (target/'.download-complete').exists():return target
  temporary=work/(name+'-'+uuid.uuid4().hex);temporary.mkdir()
  try:
   self.command(['gh','run','download',run,'--repo',self.config['publication']['github_repo'],'--name',name,'--dir',temporary],timeout=180)
   atomic(temporary/'.download-complete',b'complete');temporary.replace(target)
  except Exception:
   if temporary.resolve().is_relative_to(work.resolve())and not temporary.is_symlink():shutil.rmtree(temporary)
   raise
  return target
 def source(self,release):
  job=next((j for j in self.store.db.jobs()if j['kind']=='build'and j['release_id']==release['release_id']),None)
  if not job or not re.fullmatch('[a-f0-9]{32}',job['id']):raise ValueError('Ready source snapshot unavailable')
  root=self.store.root/'builds'/job['id'];return root/'source',root/'notices'
 def public_once(self):
  config=self.config.get('publication',{})
  if not config.get('enabled'):return
  from urllib.parse import urlsplit
  origin=urlsplit(config.get('url',''))
  if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+',config.get('github_repo',''))or not re.fullmatch(r'[A-Za-z0-9_-]+',config.get('scope',''))or not re.fullmatch(r'prj_[A-Za-z0-9]+',config.get('project_id',''))or not re.fullmatch(r'team_[A-Za-z0-9]+',config.get('org_id',''))or origin.scheme!='https'or not origin.hostname or origin.username or origin.password or origin.path not in ('','/')or origin.query or origin.fragment:raise ValueError('Invalid enrolled publication target')
  release=self.ready()
  if not release:return
  commit=release['source_commit'];previous=self.state().get('public',{})
  if previous.get('release_id')==release['release_id']:
   if previous.get('status')=='current':return
   if previous.get('next_retry',0)>time.time()and self.store.db.get('delivery_retry_at',0)<=previous.get('attempt_at',0):return
  info={'release_id':release['release_id'],'source_commit':commit,'name':'Public app and downloads'}
  try:
   self.ensure_source(commit)
   runs=json.loads(self.command(['gh','run','list','--repo',config['github_repo'],'--branch','main','--commit',commit,'--limit','20','--json','databaseId,workflowName,status,headSha,conclusion']))
   gates=workflow_runs(runs,commit);work=self.store.root/'publications'/release['release_id'];work.mkdir(parents=True,exist_ok=True)
   self.status('public',{**info,'status':'publishing','detail':'Downloading verified browser/Linux/macOS artifacts'})
   browser=self.artifact(work,gates['Browser engine'],'schema-browser-site');linux=self.artifact(work,gates['Browser engine'],'linux-x64-release');mac=self.artifact(work,gates['macOS'],'macos-universal-unsigned')
   browser_manifest=json.loads((browser/'browser-build.json').read_bytes())
   if browser_manifest.get('source_commit')!=commit:raise ValueError('Browser artifact source differs')
   source,notices=self.source(release);spec=importlib.util.spec_from_file_location('ready_public_packager',source/'scripts/build_public_release.py');packager=importlib.util.module_from_spec(spec);spec.loader.exec_module(packager)
   platforms=prepared_public(work,commit);site=work/'site'
   if platforms is None:
    downloads=work/'downloads';downloads.mkdir(exist_ok=True);folder=self.store.path(release['release_id'])
    linux_archive=linux/'Schema-s-linux-x64.tar.gz'
    with tarfile.open(linux_archive)as tar:
     metadata=json.load(tar.extractfile('Schema-s/BUILD.json'))
     if metadata.get('commit')!=commit:raise ValueError('Linux artifact source differs')
     binary=tar.extractfile('Schema-s/schema-rs').read()
     if hashlib.sha256(binary).hexdigest()!=metadata['sha256']:raise ValueError('Linux artifact digest differs')
    x64=downloads/linux_archive.name;shutil.copyfile(linux_archive,x64)
    arm=packager.package_linux(folder/'schema-rs',downloads,'linux-arm64',commit,notices)
    windows=downloads/'Schema-s-Windows-x64.zip';shutil.copyfile(folder/windows.name,windows)
    mac_archive=mac/'Schema-s-macos-universal-unsigned.zip';mac_info=packager.verify_macos_release(mac_archive,commit);shutil.copyfile(mac_archive,downloads/mac_archive.name)
    platforms={name:{'filename':file.name,'sha256':packager.digest(file),'bytes':file.stat().st_size,'commit':commit}for name,file in [('linux-x64',x64),('linux-arm64',arm),('windows-x64',windows)]};platforms['macos-universal']=mac_info
    shutil.copyfile(source/'scripts/publication/bootstrap.sh',downloads/'install.sh')
    native=json.loads((folder/'latest.json').read_bytes());manifest={'available':True,'version':native['version'],'commit':commit,**platforms['windows-x64'],'platforms':platforms}
    atomic(downloads/'latest.json',(json.dumps(manifest,indent=2)+'\n').encode());atomic(downloads/'SHA256SUMS.txt',(''.join(v['sha256']+'  '+v['filename']+'\n'for v in platforms.values())+packager.digest(downloads/'install.sh')+'  install.sh\n').encode())
    site=work/'site';packager.export_site(site,downloads,browser)
    link=site/'.vercel';link.mkdir(exist_ok=True);atomic(link/'project.json',json.dumps({'projectId':config['project_id'],'orgId':config['org_id'],'projectName':'schema-s'}).encode())
    seal_public(work,commit,platforms)
   if self.store.latest()['release_id']!=release['release_id']:raise Pending('A newer ready build supersedes this publication')
   self.status('public',{**info,'status':'publishing','detail':'Deploying verified app and four platform downloads'})
   node=pathlib.Path(self.command(['node','-p','process.execPath']).strip());runner=vercel_runner(node,config)
   auth_config=['--global-config',config['vercel_config']]if config.get('vercel_config')else[]
   self.command([*runner,'deploy','--cwd',site,'--scope',config['scope'],'--prod','--yes',*auth_config],cwd=source,timeout=600)
   url=config['url'].rstrip('/')
   expected=work/'public-expectation.json';atomic(expected,json.dumps({'commit':commit,'platforms':platforms}).encode())
   self.command([node,pathlib.Path(__file__).with_name('verify_public.mjs'),url,expected],timeout=300)
   self.status('public',{**info,'status':'current' ,'url':url+'/app','detail':'App and Windows/Linux/macOS downloads verified','platforms':{k:v['commit']for k,v in platforms.items()}})
  except Pending as error:self.status('public',{**info,'status':'waiting','detail':str(error)})
  except Exception as error:self.status('public',{**info,'status':'failed','detail':str(error),'attempt_at':time.time(),'next_retry':time.time()+300})
