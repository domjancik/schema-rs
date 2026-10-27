"""Pinned SSH transport. Credentials stay in Windows Credential Manager."""
import json,pathlib,shlex,socket,sys,time,uuid
import paramiko
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'deploy'))
import windows_vault
from protocol import FILES,ID
class Device:
 def __init__(self,config):self.config=config;self.client=None;self.address=None
 def close(self):
  if self.client:self.client.close();self.client=None
 def connect(self):
  self.close();errors=[]
  for address in self.config['addresses']:
   client=paramiko.SSHClient();client.load_host_keys(str(pathlib.Path(self.config['known_hosts']).expanduser()));client.set_missing_host_key_policy(paramiko.RejectPolicy())
   try:
    wire=socket.create_connection((address,22),timeout=2)
    client.connect(self.config['host_key_alias'],sock=wire,username=self.config['user'],key_filename=str(pathlib.Path(self.config['ssh_key']).expanduser()),look_for_keys=False,allow_agent=False,timeout=3,banner_timeout=4,auth_timeout=4)
    client.get_transport().set_keepalive(15);self.client=client;self.address=address;return
   except Exception as error:
    client.close();errors.append(type(error).__name__)
  raise ConnectionError('Device unavailable or SSH identity rejected ('+', '.join(errors)+')')
 def command(self,command,root=False,timeout=30):
  if self.client is None:self.connect()
  if root:command="sudo -k -S -p '' -- /bin/bash -c "+shlex.quote(command)
  stdin,stdout,stderr=self.client.exec_command(command,timeout=timeout)
  if root:
   secret=windows_vault.read(self.config.get('sudo_credential','schema-rs/orangepi/sudo'))
   if secret is None:raise RuntimeError('Stored sudo credential unavailable')
   stdin.write(secret+'\n');stdin.flush();secret=None
  stdin.channel.shutdown_write();channel=stdout.channel;channel.set_combine_stderr(True);chunks=[];deadline=time.monotonic()+timeout;size=0
  while True:
   while channel.recv_ready():
    block=channel.recv(65536);size+=len(block)
    if size>2*1024*1024:channel.close();raise ValueError('Device response exceeds limit')
    chunks.append(block)
   if channel.exit_status_ready()and not channel.recv_ready():break
   if time.monotonic()>deadline:channel.close();raise TimeoutError('Device command timed out; status will be reconciled on reconnect')
   time.sleep(.05)
  text=b''.join(chunks).decode(errors='replace')
  if channel.recv_exit_status()!=0:raise RuntimeError(text.strip()[-2000:]or'Device command failed')
  return text
 def probe(self):
  code="""import pathlib,json,subprocess,hashlib\ndef load(path):\n try:return json.loads(pathlib.Path(path).read_text())\n except (OSError,ValueError):return None\nprint(json.dumps({'build':load('/opt/schema/current/BUILD.json'),'update':load('/var/lib/schema-updates/status.json'),'helper':pathlib.Path('/usr/local/libexec/schema-update/apply.py').is_file(),'public_key_sha256':hashlib.sha256(pathlib.Path('/var/lib/schema-updates/update-pubkey.pem').read_bytes()).hexdigest() if pathlib.Path('/var/lib/schema-updates/update-pubkey.pem').exists() else None,'service':subprocess.run(['systemctl','is-active','schema.service'],capture_output=True,text=True).stdout.strip()}))"""
  result=json.loads(self.command('python3 -c '+shlex.quote(code)));result.update(online=True,address=self.address);return result
 def bootstrap(self,signer):
  root=pathlib.Path(__file__).parent;stage='/home/'+self.config['user']+'/.cache/schema-update-enroll';self.command('install -d -m700 '+shlex.quote(stage));sftp=self.client.open_sftp()
  try:
   for name in ['apply.py','protocol.py','schema-update@.service','schema-rollback@.service','schema-update-recover.service']:sftp.put(str(root/name),stage+'/'+name)
   with sftp.open(stage+'/update-pubkey.pem','w')as out:out.write(signer.public_pem().decode())
  finally:sftp.close()
  program="""import pathlib,shutil,subprocess,os\nstage=pathlib.Path(STAGE);pub=pathlib.Path('/etc/schema/update-pubkey.pem')\nif pub.exists() and pub.read_bytes()!=(stage/'update-pubkey.pem').read_bytes():raise RuntimeError('Existing update signing key differs; explicit key rotation is required')\nfolder=pathlib.Path('/usr/local/libexec/schema-update');folder.mkdir(parents=True,exist_ok=True)\nfor name in ['apply.py','protocol.py']:\n shutil.copyfile(stage/name,folder/name);os.chmod(folder/name,0o755 if name=='apply.py' else 0o644)\nshutil.copyfile(stage/'update-pubkey.pem',pub);os.chmod(pub,0o644)\nfor name in ['schema-update@.service','schema-rollback@.service','schema-update-recover.service']:shutil.copyfile(stage/name,pathlib.Path('/etc/systemd/system')/name)\nstate=pathlib.Path('/var/lib/schema-updates');state.mkdir(mode=0o755,exist_ok=True)\nshutil.copyfile(pub,state/'update-pubkey.pem');os.chmod(state/'update-pubkey.pem',0o644)\ndrop=pathlib.Path('/etc/systemd/system/schema.service.d');drop.mkdir(exist_ok=True)\n(drop/'updates.conf').write_text('[Unit]\\nRequires=schema-update-recover.service\\nAfter=schema-update-recover.service\\n')\nsubprocess.run(['systemd-analyze','verify','schema-update@verify.service','schema-rollback@verify.service','schema-update-recover.service'],check=True)\nsubprocess.run(['systemctl','daemon-reload'],check=True)\nsubprocess.run(['systemctl','enable','schema-update-recover.service'],check=True)\nsubprocess.run(['systemctl','start','schema-update-recover.service'],check=True)\nprint('Updater enrolled; existing engine/OLA/AP/RTC unchanged.')""".replace('STAGE',repr(stage))
  return self.command('python3 -c '+shlex.quote(program),root=True,timeout=45)
 def push(self,folder,release_id,rollback=False,operation=None):
  if not(ID.fullmatch(release_id)or rollback and len(release_id)==12 and all(c in'0123456789abcdef'for c in release_id)):raise ValueError('Invalid device release ID')
  operation=operation or uuid.uuid4().hex
  if len(operation)!=32 or any(c not in'0123456789abcdef'for c in operation):raise ValueError('Invalid operation ID')
  stage='/home/'+self.config['user']+'/.cache/schema-updates/'+release_id;self.command('install -d -m700 '+shlex.quote(stage));sftp=self.client.open_sftp()
  try:
   if not rollback:
    for name in ['manifest.json','manifest.sig',*FILES]:sftp.put(str(folder/name),stage+'/'+name)
   with sftp.open(stage+'/request.json','w')as out:out.write(json.dumps({'operation_id':operation}))
  finally:sftp.close()
  unit=('schema-rollback@'if rollback else'schema-update@')+release_id+'.service'
  self.command('systemctl reset-failed '+shlex.quote(unit)+'; systemctl start --no-block '+shlex.quote(unit),root=True)
  return operation
