#!/usr/bin/env python3
"""Opt-in systemd install. Does not configure AP, firewall, OLA or clocks."""
import argparse, hashlib, ipaddress, json, os, pathlib, platform, re, secrets, shutil, subprocess, sys, time, urllib.request
MARKER='# Managed by Schema-s public installer'

def atomic(path,data,mode=0o644):
    path.parent.mkdir(parents=True,exist_ok=True)
    temp=path.with_name(path.name+'.install-tmp')
    fd=os.open(temp,os.O_WRONLY|os.O_CREAT|os.O_EXCL,mode)
    with os.fdopen(fd,'wb') as out:out.write(data.encode() if isinstance(data,str) else data)
    os.replace(temp,path);path.chmod(mode)

def install(package,root,port,bind,staging=False,start=True):
    package,root=pathlib.Path(package).resolve(),pathlib.Path(root).resolve()
    if not 1024<=port<=65535:raise ValueError('Port must be between 1024 and 65535')
    ipaddress.ip_address(bind)
    unit=root/'etc/systemd/system/schema.service'
    if unit.exists() and MARKER not in unit.read_text():
        raise ValueError('Existing custom schema.service retained. Use its configured updater; this installer will not replace it.')
    if not staging and (sys.platform!='linux' or os.geteuid()!=0):raise ValueError('System installation requires sudo on Linux')
    metadata=json.loads((package/'BUILD.json').read_text());binary=package/'schema-rs'
    binary_data=binary.read_bytes()
    if hashlib.sha256(binary_data).hexdigest()!=metadata['sha256']:raise ValueError('Binary checksum mismatch')
    commit=metadata['commit']
    if not re.fullmatch('[0-9a-f]{7,40}',commit):raise ValueError('Invalid release identity')
    expected={'x86_64':'linux-x64','amd64':'linux-x64','aarch64':'linux-arm64','arm64':'linux-arm64'}.get(platform.machine().lower())
    if not staging and metadata.get('platform')!=expected:raise ValueError('Release architecture does not match this OS (64-bit ARM or x64 required)')
    if not staging and not shutil.which('systemctl'):raise ValueError('systemd is required; run the portable binary directly on other systems')
    current=root/'opt/schema/current';release=root/('opt/schema/releases/public-'+commit)
    if current.exists() and not current.is_symlink():raise ValueError('Refusing to replace non-symlink /opt/schema/current')
    if release.exists() and (release/'schema-rs').read_bytes()!=binary_data:raise ValueError('Immutable release differs')
    env=root/'etc/schema/schema.env';project=root/'var/lib/schema';launcher=root/'usr/local/lib/schema/start'
    if env.exists():
        match=re.search(r'^SCHEMA_TOKEN=([A-Za-z0-9_-]{16,256})$',env.read_text(),re.M)
        if not match:raise ValueError('Existing schema.env has no supported access token; retained unchanged')
        token=match.group(1)
    else:token=secrets.token_urlsafe(32)
    previous={p:p.read_bytes() if p.exists() else None for p in [unit,launcher]}
    was_enabled = not staging and subprocess.run(['systemctl','is-enabled','--quiet','schema.service'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode == 0
    previous_link=os.readlink(current) if current.is_symlink() else None
    if not staging:
        if subprocess.run(['getent','passwd','schema'],stdout=subprocess.DEVNULL).returncode:
            subprocess.run(['useradd','--system','--home-dir','/var/lib/schema','--create-home','--shell','/usr/sbin/nologin','schema'],check=True)
    release.mkdir(parents=True,exist_ok=True)
    if not (release/'schema-rs').exists():atomic(release/'schema-rs',binary_data,0o755)
    atomic(release/'BUILD.json',json.dumps(metadata,indent=2)+'\n')
    if not env.exists():atomic(env,'SCHEMA_TOKEN='+token+'\n',0o600)
    project.mkdir(parents=True,exist_ok=True)
    # The native server creates a dark/output-disabled project when no saved one exists.
    host='['+bind+']' if ':' in bind else bind
    script=f'#!/bin/sh\n{MARKER}\nset -eu\nexec /opt/schema/current/schema-rs serve --project /var/lib/schema --bind {host}:{port}\n'
    atomic(launcher,script,0o755)
    service=f'''{MARKER}
[Unit]
Description=Schema-s lighting engine and web controls
After=network.target
[Service]
Type=simple
User=schema
Group=schema
WorkingDirectory=/var/lib/schema
EnvironmentFile=/etc/schema/schema.env
ExecStart=/usr/local/lib/schema/start
Restart=on-failure
RestartSec=3
KillSignal=SIGINT
TimeoutStopSec=30
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/lib/schema
UMask=0077
[Install]
WantedBy=multi-user.target
'''
    atomic(unit,service)
    temp=current.with_name('current.install-tmp');temp.symlink_to('releases/public-'+commit);os.replace(temp,current)
    if not staging:
        subprocess.run(['chown','-R','schema:schema',str(project)],check=True)
        subprocess.run(['systemctl','daemon-reload'],check=True)
        subprocess.run(['systemctl','enable','schema.service'],check=True)
        if start:
            try:
                subprocess.run(['systemctl','restart','schema.service'],check=True)
                address=bind if bind not in ['0.0.0.0','::'] else ('::1' if bind=='::' else '127.0.0.1')
                address='['+address+']' if ':' in address else address
                request=urllib.request.Request(f'http://{address}:{port}/api/state',headers={'X-Schema-Token':token})
                for attempt in range(30):
                    try:
                        with urllib.request.urlopen(request,timeout=2) as response:
                            if response.status==200:break
                    except Exception:
                        if attempt==29:raise RuntimeError('Engine did not become healthy')
                        time.sleep(.5)
            except Exception:
                subprocess.run(['systemctl','stop','schema.service'],check=False)
                if not was_enabled:subprocess.run(['systemctl','disable','schema.service'],check=False)
                for path,data in previous.items():
                    if data is None:path.unlink(missing_ok=True)
                    else:atomic(path,data,0o755 if path==launcher else 0o644)
                if previous_link is None:current.unlink(missing_ok=True)
                else:
                    temp.symlink_to(previous_link);os.replace(temp,current)
                subprocess.run(['systemctl','daemon-reload'],check=False)
                if previous_link:subprocess.run(['systemctl','restart','schema.service'],check=False)
                raise
    return {'commit':commit,'port':port,'staging':staging}

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--port',type=int,default=8787);p.add_argument('--bind',default='0.0.0.0');p.add_argument('--no-start',action='store_true');p.add_argument('--dry-run',action='store_true');p.add_argument('--prefix',type=pathlib.Path,help='Stage files for inspection/tests only; no services/users are changed');a=p.parse_args()
    if a.dry_run:
        print('Install verified binary under /opt/schema, preserve /var/lib/schema and /etc/schema/schema.env, enable schema.service. No AP/OLA/RTC/firewall changes.');return
    result=install(pathlib.Path(__file__).parent,a.prefix or pathlib.Path('/'),a.port,a.bind,staging=a.prefix is not None,start=not a.no_start)
    print(json.dumps(result));print('Access token: sudo cat /etc/schema/schema.env (keep private). Open http://DEVICE-IP:'+str(a.port)+'/welcome')
if __name__=='__main__':
    try:main()
    except Exception as error:print('Install stopped: '+str(error),file=sys.stderr);sys.exit(1)
