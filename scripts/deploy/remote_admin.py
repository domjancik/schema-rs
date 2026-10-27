"""Device-specific Windows SSH controller. Password entry is a separate local-only collector."""
import json,pathlib,shlex,socket,sys,time
import paramiko
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import windows_vault
TARGET='schema-rs/orangepi/sudo'
client=None
transport=None
key=paramiko.RSAKey.from_private_key_file(str(pathlib.Path.home()/'.ssh/id_rsa_nopass'))
def connect(force=False):
    global client,transport
    if not force and transport is not None and transport.is_active():return
    if client is not None:client.close()
    client=paramiko.SSHClient();client.load_host_keys(str(pathlib.Path.home()/'.ssh/known_hosts'));client.set_missing_host_key_policy(paramiko.RejectPolicy())
    wire=socket.create_connection(('fe80::a0:73ff:fec4:678b%16',22),timeout=10)
    client.connect('192.168.1.125',sock=wire,username='orangepi',pkey=key,allow_agent=False,look_for_keys=False,timeout=10)
    transport=client.get_transport();transport.set_keepalive(15)
def execute(command,privileged=False,timeout=120):
    connect();password=None
    if privileged:
        password=windows_vault.read(TARGET)
        if password is None:raise RuntimeError('No stored sudo credential; use the separate local masked collector')
        # Force authentication so the secret is consumed by sudo, never passed to child stdin.
        command="sudo -k -S -p '' -- /bin/bash -c "+shlex.quote(command)
    stdin,stdout,stderr=client.exec_command(command,timeout=timeout);channel=stdout.channel;channel.set_combine_stderr(True)
    if privileged:stdin.write(password+chr(10));stdin.flush();password=None
    stdin.channel.shutdown_write();deadline=time.monotonic()+timeout
    while True:
        while channel.recv_ready():sys.stdout.write(channel.recv(65536).decode(errors='replace'));sys.stdout.flush()
        if channel.exit_status_ready() and not channel.recv_ready():break
        if time.monotonic()>deadline:channel.close();raise TimeoutError('Command exceeded timeout; inspect remote outcome before retrying')
        time.sleep(.1)
    return channel.recv_exit_status()
def main():
    connect()
    if windows_vault.read(TARGET) is None:raise RuntimeError('Credential collector has not stored the sudo credential yet')
    if execute('true',True)!=0:raise RuntimeError('Stored sudo credential was rejected; no privileged deployment changes applied')
    print('READY: verified host key and sudo credential; no download proxy or password dialog opened.',flush=True)
    if '--verify-only' in sys.argv:return
    for line in sys.stdin:
        job=json.loads(line)
        if job.get('stop'):break
        try:
            connect(force=job.get('reconnect',False))
            if job.get('reconnect'):status=0
            elif 'upload' in job:
                sftp=client.open_sftp()
                for local,remote in job['upload']:
                    if remote.endswith('credentials.json'):
                        import stat
                        if stat.S_IMODE(sftp.stat(str(pathlib.PurePosixPath(remote).parent)).st_mode)!=0o700:raise RuntimeError('Credential staging directory must be mode 0700')
                        with sftp.open(remote,'wb') as target:
                            target.chmod(0o600);target.write(pathlib.Path(local).read_bytes())
                    else:sftp.put(local,remote)
                sftp.close();status=0
            else:status=execute(job['command'],job.get('sudo',False),job.get('timeout',120))
            print(chr(10)+'JOB_DONE '+json.dumps({'id':job.get('id'),'status':status}),flush=True)
        except Exception as error:print(chr(10)+'JOB_ERROR '+str(error),flush=True)
if __name__=='__main__':
    sys.stdout.reconfigure(encoding='utf-8');sys.stderr.reconfigure(encoding='utf-8')
    try:main()
    finally:
        if client is not None:client.close()
