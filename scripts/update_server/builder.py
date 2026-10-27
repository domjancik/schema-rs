"""Build a committed source snapshot, validate it, and atomically sign/publish artifacts."""
import hashlib,importlib.util,json,os,pathlib,queue,re,shutil,subprocess,threading,time,tomllib,urllib.request,zipfile
from core import utc
INPUTS=['src','web','Cargo.toml','Cargo.lock','.cargo','LICENSE','docs/desktop.md','scripts/build_desktop.py']
def source_identity(repo):
 commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip();tree=subprocess.check_output(['git','ls-tree','-r',commit,'--',*INPUTS],cwd=repo);return commit,hashlib.sha256(tree).hexdigest()
def run(args,cwd,env,log,stop,timeout=900):
 process=subprocess.Popen([str(v)for v in args],cwd=cwd,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf8',errors='replace',creationflags=getattr(subprocess,'CREATE_NEW_PROCESS_GROUP',0));lines=queue.Queue()
 def reader():
  for line in process.stdout:lines.put(line.rstrip())
  lines.put(None)
 thread=threading.Thread(target=reader,daemon=True);thread.start();deadline=time.monotonic()+timeout
 while process.poll()is None or not lines.empty():
  if stop.is_set()or time.monotonic()>deadline:
   if os.name=='nt':subprocess.run(['taskkill','/PID',str(process.pid),'/T','/F'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
   else:process.terminate()
   process.wait(timeout=10);raise RuntimeError('Build cancelled or exceeded time limit')
  try:
   line=lines.get(timeout=.2)
   if line is not None:log(line)
  except queue.Empty:pass
 thread.join(timeout=2)
 while not lines.empty():
  line=lines.get()
  if line is not None:log(line)
 if process.returncode!=0:raise RuntimeError('Validation/build command failed: '+str(args[0]))
def smoke(binary,source,work,config,log,stop):
 folder=work/'smoke';folder.mkdir();copy=folder/'schema-rs.exe';shutil.copy2(binary,copy);env=os.environ.copy();env.pop('SCHEMA_TOKEN',None);process=subprocess.Popen([str(copy),'desktop','--no-open','--project',str(folder/'project'),'--bind','127.0.0.1:0'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf8',errors='replace',env=env)
 try:
  line_queue=queue.Queue()
  def reader():
   for line in process.stdout:line_queue.put(line)
  threading.Thread(target=reader,daemon=True).start();deadline=time.monotonic()+20;base=None
  while time.monotonic()<deadline and process.poll()is None:
   try:line=line_queue.get(timeout=.2)
   except queue.Empty:continue
   match=re.search(r'Sch.ma engine: (http://127\.0\.0\.1:\d+)',line)
   if match:base=match.group(1);break
  if not base:raise RuntimeError('Smoke engine failed to start')
  with urllib.request.urlopen(base+'/api/state',timeout=5)as response:state=json.load(response)
  if state['project']['output']['enabled']:raise RuntimeError('Smoke engine output is unexpectedly enabled')
  for path in ['/preview','/app.webmanifest','/mobile','/scene-assets/carpushit.glb']:
   with urllib.request.urlopen(base+path,timeout=5)as response:
    if response.status!=200:raise RuntimeError('Embedded UI smoke failed')
  if config.get('gpu_smoke'):
   env.update(SCHEMA_URL=base,SCHEMA_BROWSER_CHANNEL=config.get('browser_channel','chrome'),PLAYWRIGHT_MODULE=config['playwright_module'],SCHEMA_EXPECT_BACKEND='WebGPU');run(['node','tests/scene-preview-browser.cjs'],source,env,log,stop,120)
  log('Isolated native smoke passed; output disabled.')
 finally:process.terminate();process.wait(timeout=10)
def build(repo,store,signer,config,job,stop):
 repo=pathlib.Path(repo);commit,tree=source_identity(repo);db=store.db;log=lambda text:db.event(job,text)
 def phase(value):db.update(job,status='running',phase=value);log(value)
 work=store.root/'builds'/job;work.mkdir(parents=True);archive=work/'source.zip';source=work/'source';source.mkdir();env=os.environ.copy();cache=store.root/'build-cache';env['CARGO_TARGET_DIR']=str(cache);env['CARGO_BUILD_JOBS']=str(config.get('build_jobs',2));env['CARGO_PROFILE_DEV_DEBUG']='0';env['CARGO_PROFILE_TEST_DEBUG']='0';env['CARGO_INCREMENTAL']='0';env.pop('SCHEMA_TOKEN',None)
 try:
  if shutil.disk_usage(store.root).free<768*1024*1024:raise RuntimeError('At least 768 MiB free disk space is required for an update build; clean regenerable compiler artifacts and retry')
  phase('snapshot');subprocess.run(['git','archive','--format=zip','-o',str(archive),commit],cwd=repo,check=True)
  with zipfile.ZipFile(archive)as zipped:zipped.extractall(source)
  sysroot=subprocess.check_output(['rustc','--print','sysroot'],text=True).strip();linker=pathlib.Path(sysroot)/'lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe'
  if not linker.is_file():raise RuntimeError('Windows Rust cross linker is not installed')
  env['CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER']=str(linker);env['CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS']='-C linker-flavor=ld.lld -C link-self-contained=yes'
  checks={}
  for name,commands in [('fmt',[['cargo','fmt','--check']]),('tests',[['cargo','test','--locked','--quiet']]),('clippy',[['cargo','clippy','--locked','--all-targets','--','-D','warnings']]),('syntax',[['node','--check',str(path.relative_to(source))]for path in sorted((source/'web').glob('*.js'))]+[['node','--check',str(path.relative_to(source))]for path in sorted((source/'web').glob('*.mjs'))])]:
   phase(name)
   for command in commands:run(command,source,env,log,stop)
   checks[name]=True
  phase('build ARM64');run(['cargo','build','--locked','--release','--target','aarch64-unknown-linux-musl'],source,env,log,stop)
  phase('build Windows');run(['cargo','build','--locked','--release'],source,env,log,stop)
  phase('smoke');smoke(cache/'release/schema-rs.exe',source,work,config,log,stop);checks['smoke']=True;checks['gpu_smoke']=bool(config.get('gpu_smoke'))
  phase('package');output=work/'artifacts';output.mkdir();shutil.copyfile(cache/'aarch64-unknown-linux-musl/release/schema-rs',output/'schema-rs')
  spec=importlib.util.spec_from_file_location('schema_release_packager',source/'scripts/build_desktop.py');packager=importlib.util.module_from_spec(spec);spec.loader.exec_module(packager)
  original=os.getcwd();saved=os.environ.get('CARGO_TARGET_DIR');os.environ['CARGO_TARGET_DIR']=str(cache)
  try:
   notices=work/'notices';packager.collect_notices(notices);version=tomllib.loads((source/'Cargo.toml').read_text())['package']['version']+'-preview';packager.package(cache/'release/schema-rs.exe',output,version,commit[:12],notices)
  finally:
   if saved is None:os.environ.pop('CARGO_TARGET_DIR',None)
   else:os.environ['CARGO_TARGET_DIR']=saved
  phase('sign and publish');release=store.publish(output,commit,tree,checks,signer);db.update(job,status='succeeded',phase='ready',release_id=release['release_id']);log('Signed ready release '+release['release_id']);db.set('last_attempt_tree',tree);return release
 except Exception as error:
  db.update(job,status='failed',phase='failed',detail=str(error));db.event(job,str(error));db.set('last_attempt_tree',tree);raise
