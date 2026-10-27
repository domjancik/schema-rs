#!/bin/bash
set -euo pipefail
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
stage=${1:?Pass staging directory}
release=${2:?Pass source commit}
[[ "$release" =~ ^[0-9a-f]{7,40}$ ]]
[ "$(id -u)" = 0 ]
python3 - "$stage" "$release" <<'PY'
import hashlib,json,pathlib,sys
stage=pathlib.Path(sys.argv[1]);m=json.loads((stage/'arm-build.json').read_text());binary=(stage/'schema-rs').read_bytes()
assert m['source_commit']==sys.argv[2]
assert hashlib.sha256(binary).hexdigest()==m['sha256']
assert binary[:4]==bytes([127,69,76,70]) and binary[18:20]==bytes([183,0])
print('Verified ARM64 release SHA256:',m['sha256'])
PY
if ! getent passwd schema >/dev/null; then useradd --system --home-dir /var/lib/schema --create-home --shell /usr/sbin/nologin schema; fi
install -d -m755 /opt/schema/releases/"$release"
if [ -f /opt/schema/releases/"$release"/schema-rs ]; then
 cmp -s "$stage/schema-rs" /opt/schema/releases/"$release"/schema-rs || { echo 'Immutable release already exists with different bytes' >&2; exit 1; }
else
 install -m755 "$stage/schema-rs" /opt/schema/releases/"$release"/schema-rs
fi
install -m644 "$stage/arm-build.json" /opt/schema/releases/"$release"/BUILD.json
if [ -e /opt/schema/current ] && [ ! -L /opt/schema/current ]; then echo '/opt/schema/current is not a symlink; refusing to replace' >&2; exit 1; fi
ln -sfn "releases/$release" /opt/schema/current
install -d -o schema -g schema -m750 /var/lib/schema
install -d -m750 /etc/schema
python3 - "$stage" <<'PY'
import json,os,pathlib,re,sys
stage=pathlib.Path(sys.argv[1]);credential_file=stage/'credentials.json'
if not credential_file.exists():
 import secrets
 c={'ap_ssid':'Schema-OrangePi','ap_password':'schema-'+secrets.token_hex(8),'schema_token':secrets.token_urlsafe(24)}
 descriptor=os.open(credential_file,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(descriptor,'w') as target:json.dump(c,target)
c=json.loads(credential_file.read_text())
assert re.fullmatch(r'[A-Za-z0-9_-]{24,128}',c['schema_token'])
config=pathlib.Path('/etc/schema');(config/'schema.env').write_text('SCHEMA_TOKEN='+c['schema_token']+chr(10));(config/'access.json').write_text(json.dumps(c,indent=2)+chr(10))
for name in ['schema.env','access.json']:os.chmod(config/name,0o600)
root=pathlib.Path('/var/lib/schema');layout=root/'registry/layouts/init.json'
if not layout.exists():
 # A new device-local dark scene; no existing Windows project is transferred.
 slot=lambda name,source,constant=0:{'name':name,'source':source,'constant':constant,'channel':'','minimum':0,'maximum':255,'fine':False}
 p={'version':1,'name':'UV COB studio','fixtures':[{'id':'uv-cob-1','name':'UV COB','position':[0,0,0],'rotation':[0,0,0],'pixel_positions':[[0,0,0]],'pixel_reverse':False,'setup_channel':0,'groups':[],'model':'dimmer','dmx_model':'uv-cob','universe':0,'address':1,'dimmer':1}],
 'dmx_models':{'uv-cob':{'name':'UV COB 2 channel','slots':[slot('Dimmer','value'),slot('Program','constant')]}},
 'dimming':{'enabled':False,'master':1,'utc_offset_minutes':120,'points':[{'minute':0,'level':1}]},
 'content':{'uv-cob-1':[{'name':'UV dimmer','root':{'id':1,'behavior':'Create.Value.Constant','properties':{'Value':0},'children':[],'drivers':{}}},None,None]},'director':[None,None,None],
 'output':{'enabled':False,'target':'127.0.0.1:6454','fps':30}}
 for folder in ['registry/layouts','registry/content','registry/config','registry/stacks','resources','plugins']:(root/folder).mkdir(parents=True,exist_ok=True)
 layout.write_text(json.dumps({k:p[k] for k in ['version','name','fixtures','dmx_models','dimming']},indent=2))
 entries=[{'fixture':fixture,'layer':layer,'stack':stack} for fixture,layers in p['content'].items() for layer,stack in enumerate(layers)]
 entries += [{'fixture':'__director','layer':layer,'stack':stack} for layer,stack in enumerate(p['director'])]
 (root/'registry/content/init.json').write_text(json.dumps(entries,indent=2));(root/'registry/config/output.json').write_text(json.dumps(p['output'],indent=2))
PY
chown -R schema:schema /var/lib/schema
install -m755 "$stage/schema-start.sh" /usr/local/sbin/schema-start
install -m644 "$stage/schema.service" /etc/systemd/system/schema.service
install -m644 "$stage/schema-clock.service" /etc/systemd/system/schema-clock.service
systemctl daemon-reload
systemd-analyze verify /etc/systemd/system/schema-clock.service /etc/systemd/system/schema.service
systemctl enable --now schema-clock.service
printf 'CORE_INSTALLED\n'
