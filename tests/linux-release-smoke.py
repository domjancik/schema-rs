import pathlib,subprocess,tempfile,urllib.request,json,signal,tarfile,sys,queue,threading,time,re
with tempfile.TemporaryDirectory(prefix='schema-release-') as folder:
 archive=pathlib.Path(sys.argv[1]);stage=pathlib.Path(folder)/'package';stage.mkdir()
 with tarfile.open(archive) as tar:
  assert all(m.name.startswith('Schema-s/') and '..' not in pathlib.PurePosixPath(m.name).parts and (m.isfile() or m.isdir()) for m in tar.getmembers())
  tar.extractall(stage,filter='data')
 binary=str(stage/'Schema-s/schema-rs')
 folder=str(pathlib.Path(folder)/'project')
 def start():
  p=subprocess.Popen([binary,'desktop','--no-open','--project',folder,'--bind','127.0.0.1:0'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  lines=queue.Queue();seen=[]
  def reader():
   for line in p.stdout:lines.put(line)
  threading.Thread(target=reader,daemon=True).start();deadline=time.monotonic()+25
  while time.monotonic()<deadline and p.poll() is None:
   try:line=lines.get(timeout=.2)
   except queue.Empty:continue
   seen.append(line)
   match=re.search(r'engine: (http://127\.0\.0\.1:\d+)',line)
   if match:return p,match.group(1)
  p.send_signal(signal.SIGINT)
  try:p.wait(timeout=5)
  except subprocess.TimeoutExpired:p.kill();p.wait()
  raise RuntimeError('No startup URL: '+repr(seen))
 def get(base,path):return json.load(urllib.request.urlopen(base+path))
 p,base=start()
 try:
  state=get(base,'/api/state');assert not state['project']['output']['enabled'];f=state['project']['fixtures'][0];f['dimmer']=.42
  req=urllib.request.Request(base+'/api/command',data=json.dumps({'expected_revision':state['snapshot']['revision'],'command':{'type':'set_fixture','fixture':f}}).encode(),headers={'Content-Type':'application/json'})
  urllib.request.urlopen(req).close()
  assert get(base,'/api/state')['snapshot']['can_undo']
  for path in ['/mobile','/welcome','/mobile-skin.js','/preview']:
   assert urllib.request.urlopen(base+path).status==200
 finally:p.send_signal(signal.SIGINT);p.wait(timeout=15)
 p,base=start()
 try:
  state=get(base,'/api/state');assert state['project']['fixtures'][0]['dimmer']==.42;assert state['snapshot']['can_undo'];assert not state['project']['output']['enabled']
 finally:p.send_signal(signal.SIGINT);p.wait(timeout=15)
 print('Linux x64 extracted binary: web assets, output-disabled startup, edit/undo and durable restart passed.')
