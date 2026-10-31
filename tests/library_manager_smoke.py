"""Real PC/Pi library UI verification on two disposable output-disabled loopback servers."""
import argparse,json,os,pathlib,shutil,socket,subprocess,tempfile,time,urllib.request
p=argparse.ArgumentParser();p.add_argument('--binary',type=pathlib.Path,required=True);args=p.parse_args()
for port in [6990,6991]:
 with socket.socket() as sock:
  if sock.connect_ex(('127.0.0.1',port))==0:raise SystemExit('Refusing to touch occupied test port '+str(port))
with tempfile.TemporaryDirectory(prefix='schema-library-test-') as folder:
 root=pathlib.Path(folder);exe=root/args.binary.name;shutil.copy2(args.binary,exe);procs=[]
 try:
  for port,name,token in [(6990,'pc',None),(6991,'pi','1357')]:
   env=os.environ.copy();env.pop('SCHEMA_TOKEN',None)
   if token:env['SCHEMA_TOKEN']=token
   stop=root/(name+'.stop');log=(root/(name+'.log')).open('w');proc=subprocess.Popen([str(exe),'serve','--bind','127.0.0.1:'+str(port),'--project',str(root/name),'--shutdown-file',str(stop)],env=env,stdout=log,stderr=log,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0));procs.append((proc,stop,log))
   for i in range(100):
    try:
     req=urllib.request.Request('http://127.0.0.1:'+str(port)+'/api/state',headers={'x-schema-token':token} if token else {});data=json.load(urllib.request.urlopen(req,timeout=1));assert not data['project']['output']['enabled'];break
    except Exception:
     if proc.poll() is not None:raise RuntimeError('Disposable engine exited')
     time.sleep(.1)
   else:raise RuntimeError('Disposable engine did not become healthy')
  env=os.environ.copy();env['SCHEMA_URL']='http://127.0.0.1:6990';env['SCHEMA_PEER_URL']='http://127.0.0.1:6991';subprocess.run(['node','tests/library-manager-browser.cjs'],env=env,check=True)
 finally:
  for proc,stop,log in procs:stop.write_text('stop');proc.wait(timeout=20);log.close()
