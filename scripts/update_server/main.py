"""Schéma update host: committed builds, pinned SSH pushes and a loopback dashboard."""
import argparse,json,os,pathlib,sys,threading
from core import ReleaseStore,Signer,atomic
from device import windows_vault
from manager import Manager
from http_ui import Server

TARGET='schema-rs/updates/signing-key'
def defaults():
 home=pathlib.Path.home()
 return {'python':sys.executable,'repo':str(pathlib.Path(__file__).resolve().parents[2]),'port':8890,'poll_seconds':10,'build_jobs':2,'gpu_smoke':True,'browser_channel':'chrome','playwright_module':str(home/'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'),'device':{'addresses':['10.0.0.2','fe80::a0:73ff:fec4:678b%16','10.42.0.1'],'host_key_alias':'192.168.1.125','user':'orangepi','ssh_key':str(home/'.ssh/id_rsa_nopass'),'known_hosts':str(home/'.ssh/known_hosts'),'sudo_credential':'schema-rs/orangepi/sudo'}}
def load_signer():
 secret=windows_vault.read(TARGET)
 if secret:return Signer.from_secret(secret)
 signer=Signer.generate();windows_vault.write(TARGET,signer.secret(),username='schema-update-signing');return signer

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('command',choices=['init','serve']);parser.add_argument('--state',type=pathlib.Path,default=pathlib.Path(os.environ.get('LOCALAPPDATA',pathlib.Path.home()))/'Schema-s/updates');args=parser.parse_args();root=args.state.resolve();root.mkdir(parents=True,exist_ok=True);config_path=root/'config.json'
 if not config_path.exists():atomic(config_path,(json.dumps(defaults(),indent=2)+'\n').encode())
 config=json.loads(config_path.read_text());signer=load_signer();atomic(root/'update-pubkey.pem',signer.public_pem())
 if args.command=='init':print('Update server initialized. Configuration: '+str(config_path));return
 import msvcrt
 with open(root/'server.lock','a+b')as lock:
  lock.seek(0);lock.write(b'0');lock.flush();lock.seek(0)
  try:msvcrt.locking(lock.fileno(),msvcrt.LK_NBLCK,1)
  except OSError:raise SystemExit('An update server already owns this state directory')
  with ReleaseStore(root)as store:
   manager=Manager(config,store,signer);server=Server(('127.0.0.1',int(config.get('port',8890))),manager);manager.restart_callback=server.shutdown;manager.start();print('Schéma updates: http://127.0.0.1:'+str(server.server_port)+'/',flush=True)
   try:server.serve_forever(poll_interval=.5)
   except KeyboardInterrupt:pass
   finally:server.server_close();manager.close()
   if manager.restart_requested:raise SystemExit(75)
if __name__=='__main__':main()
